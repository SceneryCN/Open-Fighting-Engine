// SPDX-License-Identifier: MPL-2.0
//! Deterministic two-to-four-entity combat. One successful step commits one 60 Hz tick.
//! Rules are immutable; mutable frame data is staged until every checked operation succeeds.
mod action;
mod geometry;
mod resolve;
mod types;
use crate::duel::{Controls, DuelError, Event, Fighter, LaneConfig, State};
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
        self.moves.iter().enumerate().all(|(i, m)| {
            let start = u64::from(m.base.startup);
            let end = start + u64::from(m.base.active);
            m.base.valid() && m.energy_cost >= 0 &&
                !matches!(m.kind, MoveKind::Projectile {speed, lifetime} if speed <= 0 || lifetime == 0) &&
                m.windows.iter().all(|w| w.start <= w.end && u64::from(w.start) >= start && u64::from(w.end) < end)
                && m.windows.windows(2).all(|p| p[0].end < p[1].start)
                && m.cancels.iter().all(|c| c.into != i && c.into < self.moves.len() && c.window.start <= c.window.end && u64::from(c.window.end) < m.duration())
        })
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

type Spawn = (EntityId, usize, u64, Position, i32);

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
}

fn facing(actors: &[ArenaFighter], i: usize) -> i32 {
    let actor = actors[i];
    let opponent = actors
        .iter()
        .filter(|v| v.team != actor.team && v.fighter.state != State::Ko)
        .min_by_key(|v| {
            (
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
        Ok(Self {
            config,
            world,
            frame: 0,
            events: Vec::new(),
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

        let mut world = self.world.clone();
        let mut effects = FrameEffects {
            events: Vec::new(),
            spawn: Vec::new(),
            last_id: world.last_id,
        };
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
                &mut effects,
            )?;
        }
        world.advance_projectiles(&self.config, &mut effects)?;
        let mut contacts = world.contacts(&self.config, active);
        let armor = world.timers.iter().map(Timers::armor).collect::<Vec<_>>();
        let immune = world
            .timers
            .iter()
            .zip(&self.config.fighters)
            .map(|(t, c)| t.protected(c))
            .collect::<Vec<_>>();
        let resolution = resolve::resolve(&world.fighters, &armor, &immune, &mut contacts)?;
        world.apply(&self.config, &contacts, resolution, &mut effects.events)?;
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
        self.world = world;
        self.events = effects.events;
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
}

impl World {
    fn advance_projectiles(
        &mut self,
        config: &ArenaConfig,
        effects: &mut FrameEffects,
    ) -> Result<(), DuelError> {
        let mut kept = 0;
        for i in 0..self.flights.len() {
            let mut flight = self.flights[i];
            if flight.remaining == 0 {
                continue;
            }
            let MoveKind::Projectile { speed, .. } =
                config.fighters[flight.owner].moves[flight.move_index].kind
            else {
                unreachable!("validated projectile")
            };
            let mut projectile = self.projectiles[i];
            shift(&mut projectile.position.x, i64::from(speed * flight.facing))?;
            flight.remaining -= 1;
            self.flights[kept] = flight;
            self.projectiles[kept] = projectile;
            kept += 1;
        }
        self.flights.truncate(kept);
        self.projectiles.truncate(kept);
        for (id, move_index, cast, position, facing) in effects.spawn.drain(..) {
            let owner = self
                .fighters
                .binary_search_by_key(&id, |f| f.fighter.id)
                .expect("known caster");
            let MoveKind::Projectile { lifetime, .. } =
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

    fn contacts(&self, config: &ArenaConfig, active: [Option<usize>; 4]) -> Vec<Contact> {
        let mut contacts = Vec::new();
        let mut collect = |s: Source| {
            let rules = &config.fighters[s.owner].moves[s.move_index];
            for (j, defender) in self.fighters.iter().enumerate() {
                if s.spent & (1 << j) == 0
                    && self.fighters[s.owner].team != defender.team
                    && geometry::intersects(
                        s.body,
                        defender.fighter,
                        rules.base,
                        config.fighters[j].base.hurtbox,
                    )
                {
                    contacts.push(Contact {
                        attacker: s.owner,
                        target: j,
                        instance: s.instance,
                        window: s.window,
                        facing: s.body.facing,
                        attack: rules.base,
                        hitstop: rules.hitstop,
                        throw: rules.kind == MoveKind::Throw,
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
                });
            }
        }
        for (p, f) in self.projectiles.iter().zip(&self.flights) {
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
            });
        }
        contacts
    }

    fn apply(
        &mut self,
        config: &ArenaConfig,
        contacts: &[Contact],
        result: resolve::ContactResult,
        events: &mut Vec<ArenaEvent>,
    ) -> Result<(), DuelError> {
        // An opponent who escaped hitstun before settlement starts a fresh combo.
        for (j, target) in self.fighters.iter().enumerate() {
            if target.fighter.state != State::Hitstun {
                for row in &mut self.combos {
                    row[j] = 0;
                }
            }
        }
        for (i, blocked) in result.resolved {
            let c = contacts[i];
            let mut cast = c.instance;
            if let Ok(p) = self.projectiles.binary_search_by_key(&c.instance, |p| p.id) {
                self.flights[p].spent |= 1 << c.target;
                cast = self.flights[p].cast;
            }
            if let Action::Attack(ref mut a) = self.timers[c.attacker].action
                && a.instance == cast
            {
                if c.instance == cast {
                    a.spent |= 1 << c.target;
                }
                a.confirmed |= !blocked;
            }
            if !blocked {
                let combo = &mut self.combos[c.attacker][c.target];
                *combo = combo.checked_add(1).ok_or(DuelError::Overflow)?;
            }
        }
        events.extend(result.events);
        for (i, impact) in result.impacts.into_iter().enumerate() {
            let view = &mut self.fighters[i];
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
