// SPDX-License-Identifier: MPL-2.0
//! Resolve eligible contacts against one post-movement, pre-damage world.
use super::{ArenaEvent, ArenaFighter};
use crate::duel::{AttackConfig, DuelError, Event, State};

#[derive(Clone, Copy, Debug)]
pub(super) struct Contact {
    pub attacker: usize,
    pub target: usize,
    pub instance: u64,
    pub window: usize,
    pub facing: i32,
    pub attack: AttackConfig,
    pub hitstop: u32,
    pub throw: bool,
}

#[derive(Debug, Default)]
pub(super) struct Impact {
    pub damage: i64,
    pub push: i64,
    pub hitstun: u32,
    pub blockstun: u32,
    pub hitstop: u32,
    pub armor_used: u32,
    pub took_hit: bool,
    pub blocked: bool,
}

pub(super) struct ContactResult {
    pub impacts: Vec<Impact>,
    pub events: Vec<ArenaEvent>,
    // Indices refer to the canonically sorted contact list.
    pub resolved: Vec<(usize, bool)>,
}

pub(super) fn resolve(
    fighters: &[ArenaFighter],
    armor: &[u32],
    invulnerable: &[bool],
    contacts: &mut [Contact],
) -> Result<ContactResult, DuelError> {
    contacts.sort_unstable_by_key(|c| (c.attacker, c.target, c.instance, c.window));
    let mut result = ContactResult {
        impacts: (0..fighters.len()).map(|_| Impact::default()).collect(),
        events: Vec::new(),
        resolved: Vec::new(),
    };
    for (index, c) in contacts.iter().enumerate() {
        let source = &fighters[c.attacker];
        let defender = &fighters[c.target];
        if invulnerable[c.target]
            || defender.fighter.state == State::Ko
            || source.team == defender.team
        {
            continue;
        }
        let attacker = source.fighter.id;
        let target = defender.fighter.id;
        let blocked = !c.throw && matches!(defender.fighter.state, State::Guard | State::Blockstun);
        result.resolved.push((index, blocked));
        let source_effect = &mut result.impacts[c.attacker];
        source_effect.hitstop = source_effect.hitstop.max(c.hitstop);
        let effect = &mut result.impacts[c.target];
        effect.hitstop = effect.hitstop.max(c.hitstop);
        if blocked {
            effect.blocked = true;
            effect.blockstun = effect.blockstun.max(c.attack.blockstun);
            result
                .events
                .push(ArenaEvent::Base(Event::Blocked { attacker, target }));
            continue;
        }
        let damage = c.attack.damage;
        effect.damage = effect
            .damage
            .checked_add(i64::from(damage))
            .ok_or(DuelError::Overflow)?;
        result.events.push(ArenaEvent::Base(Event::Hit {
            attacker,
            target,
            damage,
        }));
        if !c.throw && effect.armor_used < armor[c.target] {
            effect.armor_used += 1;
            result.events.push(ArenaEvent::Armored { attacker, target });
        } else {
            effect.took_hit = true;
            effect.hitstun = effect.hitstun.max(c.attack.hitstun);
            let offset = i64::from(c.facing) * i64::from(c.attack.pushback);
            effect.push = effect.push.checked_add(offset).ok_or(DuelError::Overflow)?;
        }
    }
    Ok(result)
}
