// SPDX-License-Identifier: MPL-2.0
//! Two-fighter simulation at [crate::TICKS_PER_SECOND]. Each call commits a whole tick.
//! On free grounded ticks the input priority is attack, eligible switch, jump, guard, walk.
//! Facing is sampled from the previous committed positions before starting an action.
//! Airborne fighters can steer horizontally. Locked actions preserve their facing.
mod types;
pub use types::*;

use crate::{EntityId, Position};

const FAR: i32 = 1000;

fn nearest(z: i32) -> i32 {
    if z >= FAR / 2 { FAR } else { 0 }
}

impl Rect {
    fn valid(self) -> bool {
        self.width > 0 && self.height > 0
    }

    fn bounds(self, owner: Fighter) -> [i64; 4] {
        // Intermediate geometry can extend past the representable center coordinates.
        let origin = i64::from(owner.position.x);
        let x = i64::from(self.x);
        let width = i64::from(self.width);
        let left = if owner.facing == 1 {
            origin + x
        } else {
            origin - x - width
        };
        let bottom = i64::from(owner.position.h) + i64::from(self.h);
        [left, left + width, bottom, bottom + i64::from(self.height)]
    }
}

impl LaneRule {
    fn allows(self, a: i32, b: i32) -> bool {
        match self {
            Self::SameLane => nearest(a) == nearest(b),
            Self::CrossLane => nearest(a) != nearest(b),
            Self::BothLanes => true,
            Self::DepthRange(range) => a.abs_diff(b) <= range as u32,
        }
    }
}

impl FighterConfig {
    fn valid(self) -> bool {
        let attack = self.attack;
        self.max_health > 0
            && self.walk_speed >= 0
            && self.jump_speed > 0
            && self.gravity > 0
            && self.spawn.h >= 0
            && matches!(self.spawn.z, 0 | FAR)
            && self.hurtbox.valid()
            && attack.hitbox.valid()
            && attack.active > 0
            && attack.damage >= 0
            && attack.pushback >= 0
            && !matches!(attack.lane_rule, LaneRule::DepthRange(n) if n < 0)
    }
}

#[derive(Debug, Clone, Copy, Default)]
enum Action {
    #[default]
    Free,
    Attack {
        age: u64,
        spent: bool,
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

#[derive(Debug, Clone, Copy, Default)]
struct Timers {
    action: Action,
    velocity: i32,
    cooldown: u32,
}

fn move_x(fighter: &mut Fighter, offset: i32) -> Result<(), DuelError> {
    fighter.position.x = fighter
        .position
        .x
        .checked_add(offset)
        .ok_or(DuelError::Overflow)?;
    Ok(())
}

fn airborne(fighter: &mut Fighter, timers: &mut Timers, gravity: i32) -> Result<(), DuelError> {
    let height = fighter
        .position
        .h
        .checked_add(timers.velocity)
        .ok_or(DuelError::Overflow)?;
    fighter.position.h = height.max(0);
    timers.velocity = if height <= 0 {
        0
    } else {
        timers
            .velocity
            .checked_sub(gravity)
            .ok_or(DuelError::Overflow)?
    };
    Ok(())
}

/// Advance actions up to hit detection. Returns whether this attack instance can hit on this tick.
fn advance(
    fighter: &mut Fighter,
    timers: &mut Timers,
    rules: FighterConfig,
    lane: LaneConfig,
    input: Controls,
    facing: i32,
) -> Result<bool, DuelError> {
    if fighter.state == State::Ko {
        return Ok(false);
    }
    let switch_ready = timers.cooldown == 0;
    timers.cooldown = timers.cooldown.saturating_sub(1);
    // Actions store the age to process next. Keeping the last visible phase through
    // settlement allows a hit on the final switch tick to obey interruptibility.
    let attack_end = u64::from(rules.attack.startup) + u64::from(rules.attack.active);
    let travel_end = u64::from(lane.startup) + u64::from(lane.travel);
    timers.action = match timers.action {
        Action::Attack { age, .. } if age >= attack_end + u64::from(rules.attack.recovery) => {
            Action::Free
        }
        Action::Switch { age, .. } if age >= travel_end + u64::from(lane.recovery) => Action::Free,
        Action::Stun { remaining: 0, .. } => Action::Free,
        other => other,
    };
    let grounded = fighter.position.h == 0 && timers.velocity == 0;
    if matches!(timers.action, Action::Free) {
        fighter.facing = facing;
        if grounded && input.attack {
            timers.action = Action::Attack {
                age: 0,
                spent: false,
            };
        } else if grounded && input.switch_lane && switch_ready {
            timers.action = Action::Switch {
                age: 0,
                from: fighter.position.z,
            };
        } else if grounded && input.jump {
            timers.velocity = rules.jump_speed;
        }
    }
    match &mut timers.action {
        Action::Attack { age, spent } => {
            fighter.state = State::Attack;
            let active = !*spent && (u64::from(rules.attack.startup)..attack_end).contains(age);
            *age += 1;
            return Ok(active);
        }
        Action::Switch { age, from } => {
            fighter.state = State::LaneSwitch;
            let elapsed = (*age + 1)
                .saturating_sub(u64::from(lane.startup))
                .min(u64::from(lane.travel));
            let delta = i64::from(FAR - 2 * *from);
            fighter.position.z = *from + (delta * elapsed as i64 / i64::from(lane.travel)) as i32;
            *age += 1;
            if *age == travel_end + u64::from(lane.recovery) {
                timers.cooldown = lane.cooldown;
            }
        }
        Action::Stun { remaining, state } => {
            fighter.state = *state;
            *remaining -= 1;
            if !grounded {
                airborne(fighter, timers, rules.gravity)?;
            }
        }
        Action::Free => {
            if grounded && timers.velocity == 0 && input.guard {
                fighter.state = State::Guard;
            } else {
                move_x(fighter, input.axis * rules.walk_speed)?;
                let velocity = timers.velocity;
                airborne(fighter, timers, rules.gravity)?;
                fighter.state = if fighter.position.h > 0 {
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
    Ok(false)
}

fn intersects(striker: Fighter, target: Fighter, attack: AttackConfig, hurt: Rect) -> bool {
    if !attack
        .lane_rule
        .allows(striker.position.z, target.position.z)
    {
        return false;
    }
    let a = attack.hitbox.bounds(striker);
    let b = hurt.bounds(target);
    a[0] < b[1] && b[0] < a[1] && a[2] < b[3] && b[2] < a[3]
}

/// The P1 authoritative simulation. Input slots are stable and indexed by entity ID minus one.
#[derive(Debug, Clone)]
pub struct Duel {
    config: DuelConfig,
    frame: u64,
    fighters: [Fighter; 2],
    timers: [Timers; 2],
    events: Vec<Event>,
}

impl Duel {
    /// Validate both fighters and initialize tick zero, with no pending actions or events.
    pub fn new(config: DuelConfig) -> Result<Self, DuelError> {
        if config.lane.travel == 0 || !config.fighters.into_iter().all(FighterConfig::valid) {
            return Err(DuelError::InvalidConfig);
        }
        let left_first = config.fighters[0].spawn.x <= config.fighters[1].spawn.x;
        let fighters = std::array::from_fn(|i| Fighter {
            id: (i + 1) as EntityId,
            position: config.fighters[i].spawn,
            health: config.fighters[i].max_health,
            state: if config.fighters[i].spawn.h > 0 {
                State::Fall
            } else {
                State::Idle
            },
            facing: if (i == 0) == left_first { 1 } else { -1 },
        });
        Ok(Self {
            config,
            frame: 0,
            fighters,
            timers: [Timers::default(); 2],
            events: Vec::with_capacity(8),
        })
    }

    /// Last successfully committed tick.
    pub fn frame(&self) -> u64 {
        self.frame
    }

    /// Views of the two committed entities in slot order.
    pub fn fighters(&self) -> &[Fighter; 2] {
        &self.fighters
    }

    /// Contacts, knockouts and final movement/state changes from the last successful tick.
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// Consume both inputs atomically. A fixed-size staging copy preserves the
    /// previous tick (including events) if any coordinate operation fails.
    pub fn step(&mut self, frame: u64, inputs: [Controls; 2]) -> Result<(), DuelError> {
        if self.frame.checked_add(1).ok_or(DuelError::Overflow)? != frame {
            return Err(DuelError::WrongFrame);
        }
        if inputs.iter().any(|input| !(-1..=1).contains(&input.axis)) {
            return Err(DuelError::InvalidInput);
        }
        let mut fighters = self.fighters;
        let mut timers = self.timers;
        let mut active = [false; 2];
        let mut contacts = [None; 2];
        let left_first = fighters[0].position.x <= fighters[1].position.x;
        for i in 0..2 {
            let facing = if (i == 0) == left_first { 1 } else { -1 };
            active[i] = advance(
                &mut fighters[i],
                &mut timers[i],
                self.config.fighters[i],
                self.config.lane,
                inputs[i],
                facing,
            )?;
        }
        // Collect from the same frame for both participants before applying any hit.
        for i in 0..2 {
            let j = 1 - i;
            let attack = self.config.fighters[i].attack;
            let striker = fighters[i];
            let target = fighters[j];
            if active[i]
                && target.state != State::Ko
                && intersects(striker, target, attack, self.config.fighters[j].hurtbox)
            {
                if let Action::Attack { spent, .. } = &mut timers[i].action {
                    *spent = true;
                }
                let blocked = matches!(target.state, State::Guard | State::Blockstun);
                contacts[i] = Some((striker.facing, blocked));
            }
        }
        for (i, contact) in contacts.iter().enumerate() {
            let Some((facing, blocked)) = *contact else {
                continue;
            };
            let j = 1 - i;
            let attack = self.config.fighters[i].attack;
            let target = &mut fighters[j];
            let timer = &mut timers[j];
            if !blocked {
                target.health = (target.health - attack.damage).max(0);
                move_x(target, facing * attack.pushback)?;
            }
            if target.health == 0 {
                target.state = State::Ko;
                timer.action = Action::Free;
            } else if target.state == State::LaneSwitch && !self.config.lane.interruptible {
                continue;
            } else {
                if target.state == State::LaneSwitch {
                    target.position.z = nearest(target.position.z);
                    timer.cooldown = self.config.lane.cooldown;
                }
                target.state = if blocked {
                    State::Blockstun
                } else {
                    State::Hitstun
                };
                timer.action = Action::Stun {
                    state: target.state,
                    remaining: if blocked {
                        attack.blockstun
                    } else {
                        attack.hitstun
                    },
                };
            }
        }
        self.events.clear();
        for (i, contact) in contacts.into_iter().enumerate() {
            if let Some((_, blocked)) = contact {
                let attacker = fighters[i].id;
                let target = fighters[1 - i].id;
                self.events.push(if blocked {
                    Event::Blocked { attacker, target }
                } else {
                    Event::Hit {
                        attacker,
                        target,
                        damage: self.config.fighters[i].attack.damage,
                    }
                });
            }
        }
        for (old, new) in self.fighters.iter().zip(fighters) {
            if new.state == State::Ko && old.state != State::Ko {
                self.events.push(Event::Knockout { id: new.id });
            }
            if old.position != new.position {
                self.events.push(Event::Moved {
                    id: new.id,
                    position: new.position,
                });
            }
            if old.state != new.state {
                self.events.push(Event::StateChanged {
                    id: new.id,
                    state: new.state,
                });
            }
        }
        self.frame = frame;
        self.fighters = fighters;
        self.timers = timers;
        Ok(())
    }
}
