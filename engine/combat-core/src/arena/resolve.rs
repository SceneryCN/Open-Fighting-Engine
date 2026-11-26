// SPDX-License-Identifier: MPL-2.0
//! Resolve eligible contacts against one post-movement, pre-damage world.
use super::{ArenaConfig, ArenaEvent, ArenaFighter, MoveKind};
use crate::duel::{DuelError, Event, State};

#[derive(Clone, Copy, Debug)]
pub(super) struct Contact {
    pub attacker: usize,
    pub target: usize,
    pub instance: u64,
    pub window: usize,
    pub facing: i32,
    pub move_index: usize,
    pub projectile: Option<usize>,
    pub blocked: bool,
}

#[derive(Clone, Copy, Debug, Default)]
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

pub(super) fn resolve(
    fighters: &[ArenaFighter],
    config: &ArenaConfig,
    armor: &[u32; 4],
    contacts: &mut [Contact],
    events: &mut Vec<ArenaEvent>,
) -> Result<[Impact; 4], DuelError> {
    contacts.sort_unstable_by_key(|c| (c.attacker, c.target, c.instance, c.window));
    let mut impacts = [Impact::default(); 4];
    for c in contacts {
        let source = &fighters[c.attacker];
        let defender = &fighters[c.target];
        let rules = &config.fighters[c.attacker].moves[c.move_index];
        let attacker = source.fighter.id;
        let target = defender.fighter.id;
        let throw = rules.kind == MoveKind::Throw;
        c.blocked = !throw && matches!(defender.fighter.state, State::Guard | State::Blockstun);
        let source_effect = &mut impacts[c.attacker];
        source_effect.hitstop = source_effect.hitstop.max(rules.hitstop);
        let effect = &mut impacts[c.target];
        effect.hitstop = effect.hitstop.max(rules.hitstop);
        if c.blocked {
            effect.blocked = true;
            effect.blockstun = effect.blockstun.max(rules.base.blockstun);
            events.push(ArenaEvent::Base(Event::Blocked { attacker, target }));
            continue;
        }
        let damage = rules.base.damage;
        effect.damage = effect
            .damage
            .checked_add(i64::from(damage))
            .ok_or(DuelError::Overflow)?;
        events.push(ArenaEvent::Base(Event::Hit {
            attacker,
            target,
            damage,
        }));
        if !throw && effect.armor_used < armor[c.target] {
            effect.armor_used += 1;
            events.push(ArenaEvent::Armored { attacker, target });
        } else {
            effect.took_hit = true;
            effect.hitstun = effect.hitstun.max(rules.base.hitstun);
            let offset = i64::from(c.facing) * i64::from(rules.base.pushback);
            effect.push = effect.push.checked_add(offset).ok_or(DuelError::Overflow)?;
        }
    }
    Ok(impacts)
}
