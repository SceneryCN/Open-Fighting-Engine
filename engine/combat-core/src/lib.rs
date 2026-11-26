// SPDX-License-Identifier: MPL-2.0
//! Deterministic fixed-tick simulations: the original bridge proof and two-fighter duels.
pub mod duel;
use std::fmt;

/// Fixed simulation frequency. The host owns pacing.
pub const TICKS_PER_SECOND: u32 = 60;
/// Stable identity within a battle.
pub type EntityId = u32;

/// Numbered horizontal input. Axis values are -1, 0 and 1.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Input {
    /// Simulation tick at which this input takes effect.
    pub frame: u64,
    /// Signed horizontal movement in logical units.
    pub axis: i32,
}

/// Integer world coordinates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Position {
    /// Rightward displacement.
    pub x: i32,
    /// Depth away from the viewer.
    pub z: i32,
    /// Height above ground.
    pub h: i32,
}

/// Complete P0 authoritative state, including queued input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Snapshot {
    /// Last completed tick.
    pub frame: u64,
    /// Stable combat entity identifier.
    pub entity_id: EntityId,
    /// Horizontal position.
    pub x: i32,
    /// Depth position.
    pub z: i32,
    /// Height position.
    pub h: i32,
    /// Input waiting for the next tick.
    pub pending: Option<Input>,
}

impl Snapshot {
    /// Stable FNV-1a over versioned, little-endian fields, without padding.
    pub fn state_hash(&self) -> u64 {
        let mut hash = 0xcbf29ce484222325_u64;
        let mut add = |bytes: &[u8]| {
            for &byte in bytes {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x100000001b3);
            }
        };
        add(b"OFE-P0-v1");
        add(&self.frame.to_le_bytes());
        add(&self.entity_id.to_le_bytes());
        for coordinate in [self.x, self.z, self.h] {
            add(&coordinate.to_le_bytes());
        }
        add(&[u8::from(self.pending.is_some())]);
        if let Some(input) = self.pending {
            add(&input.frame.to_le_bytes());
            add(&input.axis.to_le_bytes());
        }
        hash
    }
}

/// Tick-local observation, valid after a successful move until the next step.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MovementEvent {
    /// Tick that produced this event.
    pub frame: u64,
    /// Moved entity.
    pub entity_id: EntityId,
    /// Resulting location.
    pub position: Position,
}

/// Recoverable simulation errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// Axis outside the supported range.
    InvalidAxis,
    /// Submission targets a different tick.
    WrongFrame,
    /// A submission already exists for the next tick.
    DuplicateInput,
    /// Tick or position exceeds its numeric representation.
    Overflow,
    /// Unsupported entity identity in a P0 snapshot.
    InvalidEntity,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

/// One authoritative battle instance, independent of rendering and wall time.
pub struct Battle {
    state: Snapshot,
    movement: Option<MovementEvent>,
}

impl Default for Battle {
    fn default() -> Self {
        Self::new()
    }
}

impl Battle {
    /// Create a battle at tick zero with entity one at the origin.
    pub fn new() -> Self {
        Self {
            state: Snapshot {
                frame: 0,
                entity_id: 1,
                x: 0,
                z: 0,
                h: 0,
                pending: None,
            },
            movement: None,
        }
    }

    /// Restore all future-affecting state. Events from earlier ticks are discarded.
    pub fn restore(snapshot: Snapshot) -> Result<Self, Error> {
        if snapshot.entity_id != 1 {
            return Err(Error::InvalidEntity);
        }
        let mut battle = Self {
            state: Snapshot {
                pending: None,
                ..snapshot
            },
            movement: None,
        };
        if let Some(input) = snapshot.pending {
            battle.submit(input.frame, input.axis)?;
        }
        Ok(battle)
    }

    /// Queue exactly one input for the upcoming tick.
    pub fn submit(&mut self, frame: u64, axis: i32) -> Result<(), Error> {
        if !(-1..=1).contains(&axis) {
            return Err(Error::InvalidAxis);
        }
        if self.state.frame.checked_add(1).ok_or(Error::Overflow)? != frame {
            return Err(Error::WrongFrame);
        }
        if self.state.pending.is_some() {
            return Err(Error::DuplicateInput);
        }
        self.state.pending = Some(Input { frame, axis });
        Ok(())
    }

    /// Advance one simulation tick atomically. Missing input defaults to neutral.
    pub fn step(&mut self) -> Result<(), Error> {
        let frame = self.state.frame.checked_add(1).ok_or(Error::Overflow)?;
        let axis = self.state.pending.map_or(0, |input| input.axis);
        let x = self.state.x.checked_add(axis).ok_or(Error::Overflow)?;
        self.state = Snapshot {
            frame,
            x,
            pending: None,
            ..self.state
        };
        self.movement = (axis != 0).then_some(MovementEvent {
            frame,
            entity_id: self.state.entity_id,
            position: Position {
                x,
                z: self.state.z,
                h: self.state.h,
            },
        });
        Ok(())
    }

    /// Copy the complete current state into caller-owned storage.
    pub fn snapshot(&self) -> Snapshot {
        self.state
    }

    /// Observe the last tick's movement, if any.
    pub fn movement_event(&self) -> Option<MovementEvent> {
        self.movement
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_errors_preserve_pending_input() {
        let mut battle = Battle::new();
        battle.submit(1, -1).unwrap();
        let queued = battle.snapshot();
        assert_eq!(battle.submit(1, 0), Err(Error::DuplicateInput));
        assert_eq!(battle.submit(2, 0), Err(Error::WrongFrame));
        assert_eq!(battle.submit(1, 2), Err(Error::InvalidAxis));
        assert_eq!(battle.snapshot(), queued);
        battle.step().unwrap();
        assert_eq!(battle.movement_event().unwrap().position.x, -1);
        battle.step().unwrap();
        assert_eq!(battle.snapshot().x, -1);
        assert_eq!(battle.movement_event(), None);
    }

    #[test]
    fn overflow_is_atomic() {
        for (x, axis) in [(i32::MAX, 1), (i32::MIN, -1)] {
            let mut battle = Battle::restore(Snapshot {
                x,
                ..Battle::new().snapshot()
            })
            .unwrap();
            battle.submit(1, axis).unwrap();
            let queued = battle.snapshot();
            assert_eq!(battle.step(), Err(Error::Overflow));
            assert_eq!(battle.snapshot(), queued);
        }
        let mut battle = Battle::restore(Snapshot {
            frame: u64::MAX,
            ..Battle::new().snapshot()
        })
        .unwrap();
        assert_eq!(battle.step(), Err(Error::Overflow));
        assert_eq!(battle.submit(0, 0), Err(Error::Overflow));
    }
}
