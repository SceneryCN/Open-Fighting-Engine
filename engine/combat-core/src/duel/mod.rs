// SPDX-License-Identifier: MPL-2.0
//! Two-fighter compatibility interface over the unified [crate::arena::Arena] simulation.
//! Commands use stable slot order. The normal move, lane switch, jump, guard and walk retain
//! their original priority and tick semantics.
mod types;
use crate::arena::{
    Arena, ArenaConfig, ArenaEvent, CombatantConfig, Command, MoveConfig, MoveKind,
};
pub use types::*;

/// Two opposing fighters using their configured normal attacks.
#[derive(Debug, Clone)]
pub struct Duel {
    arena: Arena,
    fighters: [Fighter; 2],
    events: Vec<Event>,
}

impl Duel {
    /// Validate both fighters and initialize tick zero.
    pub fn new(config: DuelConfig) -> Result<Self, DuelError> {
        let fighters = config
            .fighters
            .into_iter()
            .enumerate()
            .map(|(i, base)| {
                let id = i as u32 + 1;
                CombatantConfig {
                    id,
                    team: id,
                    base,
                    moves: vec![MoveConfig {
                        base: base.attack,
                        kind: MoveKind::Strike,
                        hitstop: 0,
                        windows: Vec::new(),
                        cancels: Vec::new(),
                        armor: 0,
                        invulnerability: 0,
                        energy_cost: 0,
                    }],
                    dash_speed: 1,
                    dash_ticks: 1,
                    max_energy: 0,
                    initial_energy: 0,
                }
            })
            .collect();
        let arena = Arena::new(ArenaConfig {
            fighters,
            lane: config.lane,
        })?;
        Ok(Self {
            fighters: std::array::from_fn(|i| arena.fighters()[i].fighter),
            arena,
            events: Vec::with_capacity(8),
        })
    }

    /// Last successfully committed tick.
    pub fn frame(&self) -> u64 {
        self.arena.frame()
    }
    /// P1 views in slot order.
    pub fn fighters(&self) -> &[Fighter; 2] {
        &self.fighters
    }
    /// Contacts, knockouts and final changes from the last committed tick.
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// Consume both inputs atomically via the shared arena resolver.
    pub fn step(&mut self, frame: u64, inputs: [Controls; 2]) -> Result<(), DuelError> {
        let commands: [Command; 2] = std::array::from_fn(|i| Command {
            id: i as u32 + 1,
            controls: inputs[i],
            ..Command::default()
        });
        self.arena.step(frame, &commands)?;
        self.fighters = std::array::from_fn(|i| self.arena.fighters()[i].fighter);
        self.events.clear();
        self.events
            .extend(self.arena.events().iter().filter_map(|e| match e {
                ArenaEvent::Base(e) => Some(*e),
                _ => None,
            }));
        Ok(())
    }
}

#[cfg(test)]
mod tests;
