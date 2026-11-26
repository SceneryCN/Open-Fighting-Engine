// SPDX-License-Identifier: MPL-2.0
use super::*;
use geometry::{FAR, nearest};

#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) enum Action {
    #[default]
    Free,
    Attack(Attack),
    Dash {
        remaining: u32,
        direction: i32,
    },
    Switch {
        age: u64,
        from: i32,
    },
    Stun {
        remaining: u32,
        state: State,
    },
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Attack {
    pub(super) index: usize,
    pub(super) instance: u64,
    // The next age to execute. Current observable age is age - 1.
    pub(super) age: u64,
    pub(super) armor: u32,
    pub(super) confirmed: bool,
    #[serde(deserialize_with = "serde::Deserialize::deserialize")]
    pub(super) window: Option<usize>,
    pub(super) spent: u8,
}

#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Timers {
    pub(super) action: Action,
    velocity: i32,
    cooldown: u32,
}

impl Window {
    fn contains(self, age: u64) -> bool {
        (u64::from(self.start)..=u64::from(self.end)).contains(&age)
    }
}

impl MoveConfig {
    pub(super) fn duration(&self) -> u64 {
        u64::from(self.base.startup) + u64::from(self.base.active) + u64::from(self.base.recovery)
    }
    fn window(&self, age: u64) -> Option<usize> {
        if self.windows.is_empty() {
            let start = u64::from(self.base.startup);
            (start..start + u64::from(self.base.active))
                .contains(&age)
                .then_some(0)
        } else {
            self.windows.iter().position(|w| w.contains(age))
        }
    }
}

fn airborne(f: &mut Fighter, timers: &mut Timers, gravity: i32) -> Result<(), DuelError> {
    let h = f
        .position
        .h
        .checked_add(timers.velocity)
        .ok_or(DuelError::Overflow)?;
    f.position.h = h.max(0);
    timers.velocity = if h <= 0 {
        0
    } else {
        timers
            .velocity
            .checked_sub(gravity)
            .ok_or(DuelError::Overflow)?
    };
    Ok(())
}

pub(super) fn shift(x: &mut i32, delta: i64) -> Result<(), DuelError> {
    *x = i32::try_from(
        i64::from(*x)
            .checked_add(delta)
            .ok_or(DuelError::Overflow)?,
    )
    .map_err(|_| DuelError::Overflow)?;
    Ok(())
}

pub(super) fn next_id(last: &mut u64) -> Result<u64, DuelError> {
    *last = last.checked_add(1).ok_or(DuelError::Overflow)?;
    Ok(*last)
}

impl Timers {
    pub(super) fn valid(
        &self,
        slot: &ArenaFighter,
        config: &CombatantConfig,
        lane: LaneConfig,
        last_id: u64,
        mask: u8,
    ) -> bool {
        if self.cooldown > lane.cooldown
            || slot.dashing != matches!(self.action, Action::Dash { .. })
        {
            return false;
        }
        if let Action::Attack(a) = self.action {
            let Some(m) = config.moves.get(a.index) else {
                return false;
            };
            return slot.move_index == Some(a.index)
                && slot.fighter.state == State::Attack
                && (1..=last_id).contains(&a.instance)
                && (1..=m.duration()).contains(&a.age)
                && a.window == m.window(a.age - 1)
                && a.spent & !mask == 0
                && a.armor <= m.armor;
        }
        if slot.move_index.is_some() {
            return false;
        }
        match self.action {
            Action::Attack(_) => unreachable!(),
            Action::Free => matches!(
                slot.fighter.state,
                State::Idle | State::Walk | State::Jump | State::Fall | State::Guard | State::Ko
            ),
            Action::Dash {
                remaining,
                direction,
            } => {
                slot.fighter.state == State::Walk
                    && matches!(direction, -1 | 1)
                    && remaining < config.dash_ticks
            }
            Action::Switch { age, from } => {
                slot.fighter.state == State::LaneSwitch
                    && matches!(from, 0 | FAR)
                    && (1..=u64::from(lane.startup)
                        + u64::from(lane.travel)
                        + u64::from(lane.recovery))
                        .contains(&age)
            }
            Action::Stun { state, .. } => {
                matches!(state, State::Hitstun | State::Blockstun) && state == slot.fighter.state
            }
        }
    }

    pub(super) fn is_idle(&self) -> bool {
        matches!(self.action, Action::Free) && self.cooldown == 0 && self.velocity == 0
    }

    pub(super) fn protected(&self, config: &CombatantConfig) -> bool {
        matches!(self.action, Action::Attack(a) if a.age <= u64::from(config.moves[a.index].invulnerability))
    }

    pub(super) fn armor(&self) -> u32 {
        match self.action {
            Action::Attack(a) => a.armor,
            _ => 0,
        }
    }

    pub(super) fn react(
        &mut self,
        view: &mut ArenaFighter,
        state: State,
        duration: u32,
        lane: LaneConfig,
    ) {
        if view.fighter.state == State::LaneSwitch {
            if !lane.interruptible && state != State::Ko {
                return;
            }
            view.fighter.position.z = nearest(view.fighter.position.z);
            self.cooldown = lane.cooldown;
        }
        self.action = if state == State::Ko {
            Action::Free
        } else {
            Action::Stun {
                remaining: duration,
                state,
            }
        };
        view.fighter.state = state;
        view.move_index = None;
        view.dashing = false;
    }

    /// Return the active window, if this actor can produce a new contact this tick.
    pub(super) fn advance(
        &mut self,
        view: &mut ArenaFighter,
        config: &CombatantConfig,
        lane: LaneConfig,
        command: Command,
        facing: i32,
        effects: &mut FrameEffects,
    ) -> Result<Option<usize>, DuelError> {
        if view.hitstop > 0 {
            view.hitstop -= 1;
            return Ok(None);
        }
        if view.fighter.state == State::Ko {
            return Ok(None);
        }
        let switch_ready = self.cooldown == 0;
        self.cooldown = self.cooldown.saturating_sub(1);
        let travel_end = u64::from(lane.startup) + u64::from(lane.travel);
        self.action = match self.action {
            Action::Attack(a) if a.age >= config.moves[a.index].duration() => Action::Free,
            Action::Switch { age, .. } if age >= travel_end + u64::from(lane.recovery) => {
                Action::Free
            }
            Action::Stun { remaining: 0, .. } | Action::Dash { remaining: 0, .. } => Action::Free,
            other => other,
        };
        let input = command.controls;
        let grounded = view.fighter.position.h == 0 && self.velocity == 0;
        let requested = command.move_index.or(input.attack.then_some(0));
        let free = matches!(self.action, Action::Free);
        if free {
            view.fighter.facing = facing;
        }
        let start = requested.filter(|&index| {
            if view.energy < config.moves[index].energy_cost {
                return false;
            }
            if free {
                return grounded;
            }
            let Action::Attack(a) = self.action else {
                return false;
            };
            a.confirmed
                && config.moves[a.index]
                    .cancels
                    .iter()
                    .any(|c| c.into == index && c.window.contains(a.age))
        });
        let id = view.fighter.id;
        if let Some(index) = start {
            let instance = next_id(&mut effects.last_id)?;
            if let Action::Attack(a) = self.action {
                effects.events.push(ArenaEvent::Cancelled {
                    id,
                    from: a.index,
                    into: index,
                });
            }
            self.action = Action::Attack(Attack {
                instance,
                index,
                age: 0,
                confirmed: false,
                window: None,
                spent: 0,
                armor: config.moves[index].armor,
            });
            if config.moves[index].launch_speed > 0 {
                self.velocity = config.moves[index].launch_speed;
            }
            view.energy -= config.moves[index].energy_cost;
            effects.events.push(ArenaEvent::Started {
                id,
                move_index: index,
            });
        } else if free && grounded {
            if command.dash {
                self.action = Action::Dash {
                    remaining: config.dash_ticks,
                    direction: if input.axis == 0 { facing } else { input.axis },
                };
                effects.events.push(ArenaEvent::Dashed { id });
            } else if input.switch_lane && switch_ready {
                self.action = Action::Switch {
                    age: 0,
                    from: view.fighter.position.z,
                };
            } else if input.jump {
                self.velocity = config.base.jump_speed;
            }
        }
        view.dashing = matches!(self.action, Action::Dash { .. });
        view.move_index = match self.action {
            Action::Attack(a) => Some(a.index),
            _ => None,
        };
        let f = &mut view.fighter;
        if matches!(self.action, Action::Attack(_)) && (self.velocity != 0 || f.position.h != 0) {
            airborne(f, self, config.base.gravity)?;
        }
        match &mut self.action {
            Action::Attack(a) => {
                f.state = State::Attack;
                let rules = &config.moves[a.index];
                let window = rules.window(a.age);
                if a.window != window {
                    a.spent = 0;
                }
                a.window = window;
                let first_active = rules
                    .windows
                    .first()
                    .map_or(rules.base.startup, |w| w.start);
                if matches!(rules.kind, MoveKind::Projectile { .. })
                    && a.age == u64::from(first_active)
                {
                    effects.spawn.push(Spawn {
                        id,
                        move_index: a.index,
                        cast: a.instance,
                        position: f.position,
                        facing: f.facing,
                    });
                }
                a.age += 1;
                if !matches!(rules.kind, MoveKind::Projectile { .. }) {
                    return Ok(window);
                }
            }
            Action::Switch { age, from } => {
                f.state = State::LaneSwitch;
                let elapsed = (*age + 1)
                    .saturating_sub(u64::from(lane.startup))
                    .min(u64::from(lane.travel));
                let delta = i64::from(FAR - 2 * *from);
                f.position.z = *from + (delta * elapsed as i64 / i64::from(lane.travel)) as i32;
                *age += 1;
                if *age == travel_end + u64::from(lane.recovery) {
                    self.cooldown = lane.cooldown;
                }
            }
            Action::Dash {
                remaining,
                direction,
            } => {
                f.state = State::Walk;
                shift(&mut f.position.x, i64::from(*direction * config.dash_speed))?;
                *remaining -= 1;
            }
            Action::Stun { remaining, state } => {
                f.state = *state;
                *remaining -= 1;
                if !grounded {
                    airborne(f, self, config.base.gravity)?;
                }
            }
            Action::Free => {
                if grounded && self.velocity == 0 && input.guard {
                    f.state = State::Guard;
                } else {
                    shift(
                        &mut f.position.x,
                        i64::from(input.axis * config.base.walk_speed),
                    )?;
                    let velocity = self.velocity;
                    airborne(f, self, config.base.gravity)?;
                    f.state = if f.position.h > 0 {
                        if velocity > 0 {
                            State::Jump
                        } else {
                            State::Fall
                        }
                    } else if input.axis != 0 {
                        State::Walk
                    } else {
                        State::Idle
                    };
                }
            }
        }
        Ok(None)
    }
}
