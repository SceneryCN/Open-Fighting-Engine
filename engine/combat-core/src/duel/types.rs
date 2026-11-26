// SPDX-License-Identifier: MPL-2.0
use crate::{EntityId, Position};

/// Host-supplied rules and initial conditions, validated by Duel::new.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuelConfig {
    /// Configurations for entity IDs 1 and 2, in order.
    pub fighters: [FighterConfig; 2],
    /// Shared lane-switch timing.
    pub lane: LaneConfig,
}

/// Data for one fighter. Distances are integral logical units, time is in ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FighterConfig {
    /// Initial position on a lane at ground level or above.
    pub spawn: Position,
    /// Starting health, strictly positive.
    pub max_health: i32,
    /// Nonnegative movement per tick.
    pub walk_speed: i32,
    /// Initial upward velocity when jumping.
    pub jump_speed: i32,
    /// Positive downward acceleration per tick.
    pub gravity: i32,
    /// Local damageable rectangle.
    pub hurtbox: Rect,
    /// The P1 normal attack.
    pub attack: AttackConfig,
}

/// Lane change duration and hit response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaneConfig {
    /// Stationary ticks at the beginning.
    pub startup: u32,
    /// Interpolation ticks, at least one.
    pub travel: u32,
    /// Stationary ticks at the destination.
    pub recovery: u32,
    /// Ticks before another switch may start.
    pub cooldown: u32,
    /// Whether an ordinary hit cancels a switch and snaps to the nearest lane.
    pub interruptible: bool,
}

/// A half-open local X/H rectangle. X is relative to facing, H points up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    /// Left edge when facing right.
    pub x: i32,
    /// Bottom edge.
    pub h: i32,
    /// Positive width.
    pub width: i32,
    /// Positive height.
    pub height: i32,
}

/// Rules for a single normal attack. The active window can contact once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttackConfig {
    /// Initial non-hitting ticks.
    pub startup: u32,
    /// Potentially hitting ticks.
    pub active: u32,
    /// Final non-hitting ticks.
    pub recovery: u32,
    /// Nonnegative health lost on hit.
    pub damage: i32,
    /// Additional ticks during which a hit target cannot act.
    pub hitstun: u32,
    /// Additional ticks during which a blocking target cannot act.
    pub blockstun: u32,
    /// Distance in the attacker's facing direction on an unblocked hit.
    pub pushback: i32,
    /// Local attack rectangle.
    pub hitbox: Rect,
    /// Depth filter applied before hitbox overlap.
    pub lane_rule: LaneRule,
}

/// Depth eligibility. The nearest lane changes at Z=500.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneRule {
    /// Only the same nearest lane.
    SameLane,
    /// Only opposite nearest lanes.
    CrossLane,
    /// Either lane.
    BothLanes,
    /// Inclusive maximum absolute Z separation.
    DepthRange(i32),
}

/// Simultaneous requests for one fighter on one numbered tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Controls {
    /// -1 for left, 0 for neutral, 1 for right.
    pub axis: i32,
    /// Request a grounded jump.
    pub jump: bool,
    /// Request a change to the opposite lane.
    pub switch_lane: bool,
    /// Request a normal attack.
    pub attack: bool,
    /// Hold ground guard.
    pub guard: bool,
}

/// Public read-only observation of one fighter after settlement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fighter {
    /// Stable entity ID within this duel.
    pub id: EntityId,
    /// World X/Z/H.
    pub position: Position,
    /// Remaining health, bounded below by zero.
    pub health: i32,
    /// Current action or reaction.
    pub state: State,
    /// World direction of local +X.
    pub facing: i32,
}

/// Committed action or reaction in the duel state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum State {
    Idle,
    Walk,
    Jump,
    Fall,
    LaneSwitch,
    Attack,
    Guard,
    Hitstun,
    Blockstun,
    Ko,
}

/// Changes committed in a tick, with simultaneous contacts ordered by attacker ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Event {
    Moved {
        id: EntityId,
        position: Position,
    },
    StateChanged {
        id: EntityId,
        state: State,
    },
    Hit {
        attacker: EntityId,
        target: EntityId,
        damage: i32,
    },
    Blocked {
        attacker: EntityId,
        target: EntityId,
    },
    Knockout {
        id: EntityId,
    },
}

/// A rejected creation or tick. A rejected tick has no observable effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuelError {
    /// Invalid rule, geometry, or initial position.
    InvalidConfig,
    /// Invalid axis in either input.
    InvalidInput,
    /// Submitted tick is not the next tick.
    WrongFrame,
    /// An authoritative coordinate, velocity or frame cannot be represented.
    Overflow,
}

impl std::fmt::Display for DuelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for DuelError {}
