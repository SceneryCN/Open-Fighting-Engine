// SPDX-License-Identifier: MPL-2.0
//! Portable, versioned committed state. Engine-owned scratch and caches are rebuilt.
use super::*;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

const VERSION: u32 = 1;
const GAMMA: u64 = 0x9e37_79b9_7f4a_7c15;

/// Complete authority at a committed tick. Deserialize and validate with
/// [Arena::from_checkpoint] before attempting simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArenaSnapshot {
    version: u32,
    config: ArenaConfig,
    frame: u64,
    rng_state: u64,
    infinite_health: bool,
    world: World,
    events: Vec<ArenaEvent>,
}

impl ArenaSnapshot {
    /// Last successful tick in this checkpoint.
    pub fn frame(&self) -> u64 {
        self.frame
    }
}

struct Fnv64(u64);

impl Write for Fnv64 {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for &byte in buf {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn rules_hash(config: &ArenaConfig) -> u64 {
    let mut hash = Fnv64(0xcbf29ce484222325);
    hash.write_all(b"OFE-ARENA-v1").expect("infallible sink");
    serde_json::to_writer(&mut hash, config).expect("integer combat rules");
    hash.0
}

impl Arena {
    pub(super) fn commit_clock(&mut self, frame: u64) {
        self.frame = frame;
        self.rng_state = self.rng_state.wrapping_add(GAMMA);
    }

    /// Original random stream seed, including after restoring a mid-match checkpoint.
    pub fn seed(&self) -> u64 {
        self.rng_state.wrapping_sub(self.frame.wrapping_mul(GAMMA))
    }

    /// SplitMix64 sample at the current tick. Reading a sample is side-effect free.
    pub fn random_sample(&self) -> u64 {
        let x = self.rng_state;
        let x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        let x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        x ^ (x >> 31)
    }

    /// Immutable compiled combat rules, in canonical fighter ID order.
    pub fn config(&self) -> &ArenaConfig {
        &self.config
    }

    /// Whether this match currently suppresses health loss on contact.
    pub fn infinite_health(&self) -> bool {
        self.infinite_health
    }

    /// Hash of authoritative rules, committed state and ordered combat events.
    pub fn state_hash(&self) -> u64 {
        let mut hash = Fnv64(self.rules_hash);
        serde_json::to_writer(
            &mut hash,
            &(
                self.frame,
                self.rng_state,
                self.infinite_health,
                &self.world,
                &self.events,
            ),
        )
        .expect("integer combat state");
        hash.0
    }

    /// Capture all state needed to reproduce subsequent committed ticks.
    pub fn checkpoint(&self) -> ArenaSnapshot {
        ArenaSnapshot {
            version: VERSION,
            config: self.config.clone(),
            frame: self.frame,
            rng_state: self.rng_state,
            infinite_health: self.infinite_health,
            world: self.world.clone(),
            events: self.events.clone(),
        }
    }

    /// Construct a fresh independent instance from a validated checkpoint.
    pub fn from_checkpoint(snapshot: &ArenaSnapshot) -> Result<Self, DuelError> {
        if snapshot.version != VERSION {
            return Err(DuelError::InvalidConfig);
        }
        let mut arena = Self::new(snapshot.config.clone())?;
        if arena.config != snapshot.config
            || !snapshot.world.valid(&arena.config)
            || !snapshot
                .events
                .iter()
                .all(|event| valid_event(*event, &arena.config, snapshot.world.last_id))
        {
            return Err(DuelError::InvalidConfig);
        }
        arena.frame = snapshot.frame;
        arena.rng_state = snapshot.rng_state;
        arena.infinite_health = snapshot.infinite_health;
        arena.events.clone_from(&snapshot.events);
        arena.world.clone_from(&snapshot.world);
        arena.idle = arena.world.is_idle();
        Ok(arena)
    }

    /// Restore a checkpoint for the same rules. Rejection leaves the match intact.
    pub fn restore(&mut self, snapshot: &ArenaSnapshot) -> Result<(), DuelError> {
        if self.config != snapshot.config {
            return Err(DuelError::InvalidConfig);
        }
        *self = Self::from_checkpoint(snapshot)?;
        Ok(())
    }
}

fn valid_event(event: ArenaEvent, config: &ArenaConfig, last_id: u64) -> bool {
    let fighter = |id| config.fighters.iter().find(|f| f.id == id);
    let ids = |attacker, target| {
        attacker != target && fighter(attacker).is_some() && fighter(target).is_some()
    };
    match event {
        ArenaEvent::Base(Event::Moved { id, position }) => {
            fighter(id).is_some() && position.h >= 0 && (0..=geometry::FAR).contains(&position.z)
        }
        ArenaEvent::Base(Event::StateChanged { id, .. } | Event::Knockout { id })
        | ArenaEvent::Dashed { id } => fighter(id).is_some(),
        ArenaEvent::Base(Event::Hit {
            attacker,
            target,
            damage,
        }) => damage >= 0 && ids(attacker, target),
        ArenaEvent::Base(Event::Blocked { attacker, target })
        | ArenaEvent::Armored { attacker, target } => ids(attacker, target),
        ArenaEvent::Started { id, move_index } => {
            fighter(id).is_some_and(|f| move_index < f.moves.len())
        }
        ArenaEvent::Cancelled { id, from, into } => {
            fighter(id).is_some_and(|f| from < f.moves.len() && into < f.moves.len())
        }
        ArenaEvent::ProjectileSpawned { id, owner } => {
            (1..=last_id).contains(&id) && fighter(owner).is_some()
        }
    }
}

impl World {
    pub(super) fn is_idle(&self) -> bool {
        self.projectiles.is_empty()
            && self.fighters.iter().zip(&self.timers).all(|(f, t)| {
                f.fighter.state == State::Idle && f.hitstop == 0 && f.combo == 0 && t.is_idle()
            })
    }

    fn valid(&self, config: &ArenaConfig) -> bool {
        let n = config.fighters.len();
        if self.fighters.len() != n
            || self.timers.len() != n
            || self.projectiles.len() != self.flights.len()
            || !self
                .projectiles
                .windows(2)
                .all(|pair| pair[0].id < pair[1].id)
        {
            return false;
        }
        let mask = (1u8 << n) - 1;
        let valid_slots = self.fighters.iter().enumerate().all(|(i, slot)| {
            let f = slot.fighter;
            let c = &config.fighters[i];
            f.id == c.id
                && slot.team == c.team
                && matches!(f.facing, -1 | 1)
                && (0..=c.base.max_health).contains(&f.health)
                && (f.health == 0) == (f.state == State::Ko)
                && (0..=c.max_energy).contains(&slot.energy)
                && f.position.h >= 0
                && (0..=geometry::FAR).contains(&f.position.z)
                && slot.combo == *self.combos[i].iter().max().expect("fixed four targets")
                && self.timers[i].valid(slot, c, config.lane, self.last_id, mask)
        });
        valid_slots
            && self
                .projectiles
                .iter()
                .zip(&self.flights)
                .all(|(p, flight)| {
                    let Some(c) = config.fighters.get(flight.owner) else {
                        return false;
                    };
                    let Some(m) = c.moves.get(flight.move_index) else {
                        return false;
                    };
                    let MoveKind::Projectile { speed, lifetime } = m.kind else {
                        return false;
                    };
                    c.id == p.owner
                        && flight.cast > 0
                        && flight.cast < p.id
                        && p.id <= self.last_id
                        && matches!(flight.facing, -1 | 1)
                        && flight.speed == speed * flight.facing
                        && flight.remaining < lifetime
                        && flight.spent & !mask == 0
                        && p.position.h >= 0
                        && (0..=geometry::FAR).contains(&p.position.z)
                })
    }
}
