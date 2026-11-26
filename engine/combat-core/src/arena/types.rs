// SPDX-License-Identifier: MPL-2.0
use crate::duel::{AttackConfig, Controls, Event, Fighter, FighterConfig, LaneConfig};
use crate::{EntityId, Position};

/// A validated match has two to four unique entities on opposing teams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArenaConfig {
    /// Fighters in arbitrary input order.
    pub fighters: Vec<CombatantConfig>,
    /// Shared depth movement timings.
    pub lane: LaneConfig,
}

/// Complete configuration for one combatant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatantConfig {
    /// Stable nonzero identity.
    pub id: EntityId,
    /// Friendly hits are suppressed between equal team IDs.
    pub team: u32,
    /// Movement, health and geometry; attack must equal the first move.
    pub base: FighterConfig,
    /// Indexed command list. Index zero is the normal attack.
    pub moves: Vec<MoveConfig>,
    /// Positive dash distance per tick.
    pub dash_speed: i32,
    /// Duration of a dash.
    pub dash_ticks: u32,
    /// Maximum energy.
    pub max_energy: i32,
    /// Energy available on tick zero.
    pub initial_energy: i32,
}

/// Attack properties shared by strikes, throws, supers and projectile sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveConfig {
    /// Base frame data, damage and geometry.
    pub base: AttackConfig,
    /// Contact/emission behavior.
    pub kind: MoveKind,
    /// Subsequent frozen fighter ticks on contact.
    pub hitstop: u32,
    /// Explicit hit windows; empty uses the base active interval.
    pub windows: Vec<Window>,
    /// Allowed hit-confirmed transitions.
    pub cancels: Vec<Cancel>,
    /// Number of non-throw hits absorbed without interrupting this action.
    pub armor: u32,
    /// Initial action ticks immune to contact.
    pub invulnerability: u32,
    /// Energy spent at the beginning of the action.
    pub energy_cost: i32,
}

/// Action behavior. Throws bypass guard and armor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveKind {
    /// Ordinary close-range contact.
    Strike,
    /// Unguardable contact.
    Throw,
    /// Emit one independent hit source at the first effective age.
    Projectile {
        /// Forward speed per tick after spawning.
        speed: i32,
        /// Lifetime in ticks including the spawn tick.
        lifetime: u32,
    },
    /// Special action with the configured energy cost.
    Super,
}

/// Inclusive action ages.
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct Window {
    /// First age.
    pub start: u32,
    /// Last age.
    pub end: u32,
}

/// One permitted move transition after a hit confirm.
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct Cancel {
    /// Legal source ages.
    pub window: Window,
    /// Target move index.
    pub into: usize,
}

/// A command for one entity. Missing entities receive neutral input.
#[derive(Debug, Clone, PartialEq, Eq, Copy, Default)]
pub struct Command {
    /// Target entity.
    pub id: EntityId,
    /// Base actions.
    pub controls: Controls,
    /// Request dash.
    pub dash: bool,
    /// Explicit move request; takes precedence over the normal attack button.
    pub move_index: Option<usize>,
}

/// Committed observation of a fighter.
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct ArenaFighter {
    /// P1-compatible position, state and health.
    pub fighter: Fighter,
    /// Team ID.
    pub team: u32,
    /// Remaining energy.
    pub energy: i32,
    /// Largest active combo by this fighter against any opponent.
    pub combo: u32,
    /// Remaining frozen ticks.
    pub hitstop: u32,
    /// Current move, if attacking.
    pub move_index: Option<usize>,
    /// True during a dash.
    pub dashing: bool,
}

/// Committed observation of a live projectile.
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct Projectile {
    /// Unique attack-instance identity.
    pub id: u64,
    /// Source fighter.
    pub owner: EntityId,
    /// Current world position.
    pub position: Position,
}

/// Per-tick events. Base events preserve Duel semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArenaEvent {
    /// P1-compatible event.
    Base(Event),
    /// Beginning of an action.
    Started {
        /// Acting entity.
        id: EntityId,
        /// Index into the actor's configured moves.
        move_index: usize,
    },
    /// Hit-confirmed transition.
    Cancelled {
        /// Acting entity.
        id: EntityId,
        /// Old move.
        from: usize,
        /// New move.
        into: usize,
    },
    /// First tick of dash.
    Dashed {
        /// Acting entity.
        id: EntityId,
    },
    /// A defender spent armor to maintain the current action.
    Armored {
        /// Originating entity.
        attacker: EntityId,
        /// Defending entity.
        target: EntityId,
    },
    /// A projectile became active.
    ProjectileSpawned {
        /// Projectile instance.
        id: u64,
        /// Source fighter.
        owner: EntityId,
    },
}
