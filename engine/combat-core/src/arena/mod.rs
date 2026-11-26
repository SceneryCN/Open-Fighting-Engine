// SPDX-License-Identifier: MPL-2.0
//! Deterministic two-to-four-entity combat. One successful step commits one 60 Hz tick.
//! Rules are immutable; mutable frame data is staged until every checked operation succeeds.
mod action;
mod geometry;
mod resolve;
mod types;
use crate::duel::{DuelError, Event, Fighter, LaneConfig, State};
use crate::{EntityId, Position};
use action::{Action, Timers, next_id, shift};
use resolve::Contact;
pub use types::*;

impl CombatantConfig {
    fn valid(&self) -> bool {
        if self.id == 0
            || self.dash_speed <= 0
            || self.dash_ticks == 0
            || self.initial_energy < 0
            || self.initial_energy > self.max_energy
            || !self.base.valid()
            || self
                .moves
                .first()
                .is_none_or(|m| m.base != self.base.attack)
        {
            return false;
        }
        self.moves
            .iter()
            .enumerate()
            .all(|(i, m)| m.valid(i, self.moves.len()))
    }
}

impl MoveConfig {
    fn valid(&self, index: usize, move_count: usize) -> bool {
        if !self.base.valid() || self.energy_cost < 0 {
            return false;
        }
        if let MoveKind::Projectile { speed, lifetime } = self.kind
            && (speed <= 0 || lifetime == 0)
        {
            return false;
        }

        let first_active = u64::from(self.base.startup);
        let end_active = first_active + u64::from(self.base.active);
        let valid_windows = self.windows.iter().all(|w| {
            w.start <= w.end && u64::from(w.start) >= first_active && u64::from(w.end) < end_active
        });
        let disjoint = self
            .windows
            .windows(2)
            .all(|pair| pair[0].end < pair[1].start);
        let valid_cancels = self.cancels.iter().all(|c| {
            c.into != index
                && c.into < move_count
                && c.window.start <= c.window.end
                && u64::from(c.window.end) < self.duration()
        });
        valid_windows && disjoint && valid_cancels
    }
}

#[derive(Debug, Clone, Copy)]
struct Flight {
    owner: usize,
    move_index: usize,
    cast: u64,
    facing: i32,
    spent: u8,
    remaining: u32,
    speed: i32,
}

#[derive(Debug, Clone)]
struct World {
    fighters: Vec<ArenaFighter>,
    timers: Vec<Timers>,
    projectiles: Vec<Projectile>,
    flights: Vec<Flight>,
    combos: [[u32; 4]; 4],
    last_id: u64,
}

#[derive(Debug, Clone)]
struct Spawn {
    id: EntityId,
    move_index: usize,
    cast: u64,
    position: Position,
    facing: i32,
}

#[derive(Debug, Clone)]
struct FrameEffects {
    events: Vec<ArenaEvent>,
    spawn: Vec<Spawn>,
    last_id: u64,
}

/// Match simulation with ID-sorted observation order. All participants advance together.
#[derive(Debug, Clone)]
pub struct Arena {
    config: ArenaConfig,
    world: World,
    frame: u64,
    events: Vec<ArenaEvent>,
    // Scratch is reset from committed data before every attempt, including retries.
    staging: World,
    effects: FrameEffects,
    contacts: Vec<Contact>,
    idle: bool,
}

fn facing(actors: &[ArenaFighter], i: usize) -> i32 {
    let actor = actors[i];
    let opponent = actors
        .iter()
        .filter(|v| v.team != actor.team)
        .min_by_key(|v| {
            (
                v.fighter.state == State::Ko,
                v.fighter.position.x.abs_diff(actor.fighter.position.x),
                v.fighter.id,
            )
        });
    match opponent {
        None => actor.fighter.facing,
        Some(v) => {
            if (actor.fighter.position.x, actor.fighter.id) < (v.fighter.position.x, v.fighter.id) {
                1
            } else {
                -1
            }
        }
    }
}

impl Arena {
    /// Validate a two-, three- or four-entity roster, then initialize tick zero.
    pub fn new(mut config: ArenaConfig) -> Result<Self, DuelError> {
        config.fighters.sort_unstable_by_key(|f| f.id);
        if !(2..=4).contains(&config.fighters.len())
            || config.lane.travel == 0
            || config.fighters.iter().any(|f| !f.valid())
            || config.fighters.windows(2).any(|p| p[0].id == p[1].id)
            || config
                .fighters
                .iter()
                .all(|f| f.team == config.fighters[0].team)
        {
            return Err(DuelError::InvalidConfig);
        }
        let mut fighters: Vec<_> = config
            .fighters
            .iter()
            .map(|c| ArenaFighter {
                fighter: Fighter {
                    id: c.id,
                    position: c.base.spawn,
                    health: c.base.max_health,
                    state: if c.base.spawn.h == 0 {
                        State::Idle
                    } else {
                        State::Fall
                    },
                    facing: 1,
                },
                team: c.team,
                combo: 0,
                energy: c.initial_energy,
                hitstop: 0,
                move_index: None,
                dashing: false,
            })
            .collect();
        for i in 0..fighters.len() {
            fighters[i].fighter.facing = facing(&fighters, i);
        }
        let world = World {
            timers: vec![Timers::default(); fighters.len()],
            fighters,
            combos: [[0; 4]; 4],
            flights: Vec::new(),
            projectiles: Vec::new(),
            last_id: 0,
        };
        let n = world.fighters.len();
        // Ordinary melee can generate one directed contact per opposing pair. Projectile-heavy
        // frames grow these reusable buffers on demand; capacity is never a gameplay limit.
        let contact_capacity = n * (n - 1);
        let event_capacity = 2 * contact_capacity + 6 * n;
        Ok(Self {
            config,
            idle: world
                .fighters
                .iter()
                .all(|v| v.fighter.state == State::Idle),
            staging: world.clone(),
            effects: FrameEffects {
                events: Vec::with_capacity(event_capacity),
                spawn: Vec::with_capacity(n),
                last_id: 0,
            },
            contacts: Vec::with_capacity(contact_capacity),
            world,
            frame: 0,
            events: Vec::with_capacity(event_capacity),
        })
    }

    /// Last committed tick.
    pub fn frame(&self) -> u64 {
        self.frame
    }
    /// Combatants sorted by ID.
    pub fn fighters(&self) -> &[ArenaFighter] {
        &self.world.fighters
    }
    /// Events from the last successful tick.
    pub fn events(&self) -> &[ArenaEvent] {
        &self.events
    }
    /// Live projectiles sorted by instance ID, including the final active tick.
    pub fn projectiles(&self) -> &[Projectile] {
        &self.world.projectiles
    }

    /// Submit zero or one command per fighter. Invalid inputs and arithmetic errors roll back
    /// the entire tick, including spent contacts, emitted projectiles and previous events.
    pub fn step(&mut self, frame: u64, inputs: &[Command]) -> Result<(), DuelError> {
        if self.frame.checked_add(1).ok_or(DuelError::Overflow)? != frame {
            return Err(DuelError::WrongFrame);
        }
        if self.idle && inputs.is_empty() {
            self.events.clear();
            self.frame = frame;
            return Ok(());
        }
        let mut commands = [None; 4];
        for command in inputs {
            let i = self
                .config
                .fighters
                .binary_search_by_key(&command.id, |f| f.id)
                .map_err(|_| DuelError::InvalidInput)?;
            if commands[i].is_some()
                || !(-1..=1).contains(&command.controls.axis)
                || command
                    .move_index
                    .is_some_and(|m| m >= self.config.fighters[i].moves.len())
            {
                return Err(DuelError::InvalidInput);
            }
            commands[i] = Some(*command);
        }

        if self.idle
            && inputs
                .iter()
                .all(|c| c.controls == Default::default() && !c.dash && c.move_index.is_none())
        {
            self.events.clear();
            self.frame = frame;
            return Ok(());
        }
        let world = &mut self.staging;
        world.fighters.copy_from_slice(&self.world.fighters);
        world.timers.copy_from_slice(&self.world.timers);
        world.combos = self.world.combos;
        let effects = &mut self.effects;
        effects.events.clear();
        effects.spawn.clear();
        effects.last_id = self.world.last_id;
        let mut active = [None; 4];
        for (i, actor) in world.fighters.iter_mut().enumerate() {
            let command = commands[i].unwrap_or(Command {
                id: actor.fighter.id,
                ..Command::default()
            });
            active[i] = world.timers[i].advance(
                actor,
                &self.config.fighters[i],
                self.config.lane,
                command,
                facing(&self.world.fighters, i),
                effects,
            )?;
        }
        world.advance_projectiles(&self.world, &self.config, effects)?;
        self.contacts.clear();
        if !world.projectiles.is_empty() || active.iter().any(Option::is_some) {
            world.contacts(&self.config, active, &mut self.contacts);
        }
        world.clear_expired_combos();
        if !self.contacts.is_empty() {
            let mut armor = [0; 4];
            for (i, t) in world.timers.iter().enumerate() {
                armor[i] = t.armor();
            }
            let impacts = resolve::resolve(
                &world.fighters,
                &self.config,
                &armor,
                &mut self.contacts,
                &mut effects.events,
            )?;
            world.apply(&self.config, &self.contacts, &impacts)?;
        }
        for (old, new) in self.world.fighters.iter().zip(&world.fighters) {
            let id = new.fighter.id;
            if new.fighter.state == State::Ko && old.fighter.state != State::Ko {
                effects
                    .events
                    .push(ArenaEvent::Base(Event::Knockout { id }));
            }
            if new.fighter.position != old.fighter.position {
                effects.events.push(ArenaEvent::Base(Event::Moved {
                    id,
                    position: new.fighter.position,
                }));
            }
            if new.fighter.state != old.fighter.state {
                effects.events.push(ArenaEvent::Base(Event::StateChanged {
                    id,
                    state: new.fighter.state,
                }));
            }
        }
        world.last_id = effects.last_id;
        self.idle = world.projectiles.is_empty()
            && world.fighters.iter().zip(&world.timers).all(|(v, t)| {
                v.fighter.state == State::Idle && v.hitstop == 0 && v.combo == 0 && t.is_idle()
            });
        std::mem::swap(&mut self.world, world);
        std::mem::swap(&mut self.events, &mut effects.events);
        self.frame = frame;
        Ok(())
    }
}

struct Source {
    owner: usize,
    move_index: usize,
    instance: u64,
    window: usize,
    spent: u8,
    body: Fighter,
    projectile: Option<usize>,
}

impl World {
    fn advance_projectiles(
        &mut self,
        previous: &World,
        config: &ArenaConfig,
        effects: &mut FrameEffects,
    ) -> Result<(), DuelError> {
        self.flights.clear();
        self.projectiles.clear();
        // Write survivors directly into next-frame storage in stable ID order. Previous storage
        // remains untouched for rollback; each survivor is copied exactly once.
        for (mut flight, mut projectile) in previous
            .flights
            .iter()
            .copied()
            .zip(previous.projectiles.iter().copied())
        {
            if flight.remaining == 0 {
                continue;
            }
            shift(&mut projectile.position.x, i64::from(flight.speed))?;
            flight.remaining -= 1;
            self.flights.push(flight);
            self.projectiles.push(projectile);
        }
        for Spawn {
            id,
            move_index,
            cast,
            position,
            facing,
        } in effects.spawn.drain(..)
        {
            let owner = self
                .fighters
                .binary_search_by_key(&id, |f| f.fighter.id)
                .expect("known caster");
            let MoveKind::Projectile { lifetime, speed } =
                config.fighters[owner].moves[move_index].kind
            else {
                unreachable!("projectile source")
            };
            let projectile_id = next_id(&mut effects.last_id)?;
            self.flights.push(Flight {
                owner,
                move_index,
                cast,
                facing,
                spent: 0,
                remaining: lifetime - 1,
                speed: speed * facing,
            });
            self.projectiles.push(Projectile {
                id: projectile_id,
                owner: id,
                position,
            });
            effects.events.push(ArenaEvent::ProjectileSpawned {
                id: projectile_id,
                owner: id,
            });
        }
        Ok(())
    }

    fn contacts(
        &self,
        config: &ArenaConfig,
        active: [Option<usize>; 4],
        contacts: &mut Vec<Contact>,
    ) {
        let mut hurt = [[0; 4]; 4];
        let mut enemies = [0u8; 4];
        for (j, defender) in self.fighters.iter().enumerate() {
            if defender.fighter.state == State::Ko || self.timers[j].protected(&config.fighters[j])
            {
                continue;
            }
            hurt[j] = config.fighters[j].base.hurtbox.bounds(defender.fighter);
            for (i, attacker) in self.fighters.iter().enumerate() {
                if defender.team != attacker.team {
                    enemies[i] |= 1 << j;
                }
            }
        }
        let mut collect = |s: Source| {
            let mut candidates = enemies[s.owner] & !s.spent;
            if candidates == 0 {
                return;
            }
            let attack = config.fighters[s.owner].moves[s.move_index].base;
            let hit = attack.hitbox.bounds(s.body);
            while candidates != 0 {
                let j = candidates.trailing_zeros() as usize;
                candidates &= candidates - 1;
                if geometry::overlaps(hit, hurt[j])
                    && attack
                        .lane_rule
                        .allows(s.body.position.z, self.fighters[j].fighter.position.z)
                {
                    contacts.push(Contact {
                        attacker: s.owner,
                        target: j,
                        instance: s.instance,
                        window: s.window,
                        facing: s.body.facing,
                        move_index: s.move_index,
                        projectile: s.projectile,
                        blocked: false,
                    });
                }
            }
        };
        for (owner, slot) in active.into_iter().enumerate() {
            if let Some(window) = slot
                && let Action::Attack(a) = self.timers[owner].action
            {
                collect(Source {
                    owner,
                    move_index: a.index,
                    instance: a.instance,
                    window,
                    spent: a.spent,
                    body: self.fighters[owner].fighter,
                    projectile: None,
                });
            }
        }
        for (index, (p, f)) in self.projectiles.iter().zip(&self.flights).enumerate() {
            let body = Fighter {
                position: p.position,
                facing: f.facing,
                ..self.fighters[f.owner].fighter
            };
            collect(Source {
                owner: f.owner,
                move_index: f.move_index,
                instance: p.id,
                window: 0,
                spent: f.spent,
                body,
                projectile: Some(index),
            });
        }
    }

    fn clear_expired_combos(&mut self) {
        let mut expired = 0u8;
        for (j, target) in self.fighters.iter().enumerate() {
            if target.fighter.state != State::Hitstun {
                expired |= 1 << j;
            }
        }
        if expired == 0 {
            return;
        }
        for (v, row) in self.fighters.iter_mut().zip(&mut self.combos) {
            if v.combo == 0 {
                continue;
            }
            for (j, count) in row.iter_mut().enumerate() {
                if expired & (1 << j) != 0 {
                    *count = 0;
                }
            }
            v.combo = *row.iter().max().expect("fixed four slots");
        }
    }

    fn apply(
        &mut self,
        config: &ArenaConfig,
        contacts: &[Contact],
        impacts: &[resolve::Impact; 4],
    ) -> Result<(), DuelError> {
        for c in contacts {
            let mut cast = c.instance;
            if let Some(p) = c.projectile {
                self.flights[p].spent |= 1 << c.target;
                cast = self.flights[p].cast;
            }
            if let Action::Attack(ref mut a) = self.timers[c.attacker].action
                && a.instance == cast
            {
                if c.instance == cast {
                    a.spent |= 1 << c.target;
                }
                a.confirmed |= !c.blocked;
            }
            if !c.blocked {
                let combo = &mut self.combos[c.attacker][c.target];
                *combo = combo.checked_add(1).ok_or(DuelError::Overflow)?;
            }
        }
        for (i, (view, impact)) in self.fighters.iter_mut().zip(impacts).enumerate() {
            view.combo = *self.combos[i].iter().max().expect("fixed four slots");
            view.hitstop = view.hitstop.max(impact.hitstop);
            view.fighter.health = (i64::from(view.fighter.health) - impact.damage).max(0) as i32;
            shift(&mut view.fighter.position.x, impact.push)?;
            if let Action::Attack(ref mut a) = self.timers[i].action {
                a.armor -= impact.armor_used;
            }
            if view.fighter.health == 0 {
                self.timers[i].react(view, State::Ko, 0, config.lane);
            } else if impact.took_hit {
                self.timers[i].react(view, State::Hitstun, impact.hitstun, config.lane);
            } else if impact.blocked {
                self.timers[i].react(view, State::Blockstun, impact.blockstun, config.lane);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
