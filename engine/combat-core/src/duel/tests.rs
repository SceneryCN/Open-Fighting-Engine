// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::Position;

fn config() -> DuelConfig {
    let rect = Rect {
        x: -1,
        h: 0,
        width: 2,
        height: 2,
    };
    let mut fighters = [FighterConfig {
        spawn: Position { x: 0, h: 0, z: 0 },
        max_health: 5,
        walk_speed: 1,
        jump_speed: 2,
        gravity: 1,
        hurtbox: rect,
        attack: AttackConfig {
            startup: 0,
            active: 2,
            recovery: 1,
            damage: 3,
            hitstun: 1,
            blockstun: 1,
            pushback: 1,
            hitbox: Rect {
                x: 0,
                width: 3,
                ..rect
            },
            lane_rule: LaneRule::SameLane,
        },
    }; 2];
    fighters[1].spawn.x = 2;
    DuelConfig {
        fighters,
        lane: LaneConfig {
            startup: 1,
            travel: 2,
            recovery: 1,
            cooldown: 1,
            interruptible: true,
        },
    }
}

fn tick(duel: &mut Duel, a: Controls, b: Controls) {
    duel.step(duel.frame() + 1, [a, b]).unwrap();
}

#[test]
fn validation_rejects_malformed_rules() {
    type Mutator = fn(&mut DuelConfig);
    for change in [
        (|c| c.lane.travel = 0) as Mutator,
        |c| c.fighters[1].max_health = 0,
        |c| c.fighters[0].gravity = -1,
        |c| c.fighters[0].jump_speed = 0,
        |c| c.fighters[1].spawn.z = 500,
        |c| c.fighters[1].spawn.h = -1,
        |c| c.fighters[1].walk_speed = -1,
        |c| c.fighters[1].attack.damage = -1,
        |c| c.fighters[1].hurtbox.height = 0,
        |c| c.fighters[1].attack.hitbox.width = -1,
        |c| c.fighters[1].attack.active = 0,
        |c| c.fighters[1].attack.lane_rule = LaneRule::DepthRange(-1),
    ] {
        let mut config = config();
        change(&mut config);
        assert_eq!(Duel::new(config).unwrap_err(), DuelError::InvalidConfig);
    }
}

#[test]
fn second_actor_overflow_keeps_first_actor_action_and_previous_events() {
    let mut config = config();
    config.fighters[1].spawn.x = i32::MAX;
    let mut duel = Duel::new(config).unwrap();
    let idle = Controls::default();
    let walk = Controls { axis: 1, ..idle };
    let attack = Controls {
        attack: true,
        ..idle
    };
    tick(&mut duel, walk, idle);
    let before = duel.clone();
    assert!(!duel.events().is_empty());
    assert_eq!(duel.step(2, [attack, walk]), Err(DuelError::Overflow));
    assert_eq!(duel.frame(), 1);
    assert_eq!(duel.fighters(), before.fighters());
    assert_eq!(duel.events(), before.events());
    tick(&mut duel, walk, idle);
    assert_eq!(duel.fighters()[0].position.x, 2);
}

#[test]
fn overflow_in_second_simultaneous_hit_preserves_both_attacks() {
    let mut config = config();
    config.fighters[1].spawn.x = i32::MIN + 1;
    config.fighters[0].spawn.x = i32::MIN;
    config.fighters[0].attack.pushback = 1;
    let mut duel = Duel::new(config).unwrap();
    let idle = Controls::default();
    let attack = Controls {
        attack: true,
        ..idle
    };
    assert_eq!(duel.step(1, [attack; 2]), Err(DuelError::Overflow));
    assert_eq!(duel.frame(), 0);
    assert!(duel.events().is_empty());
    assert_eq!(duel.fighters()[1].health, 5);
    tick(&mut duel, attack, idle);
    assert_eq!(duel.fighters()[1].health, 2);
}

#[test]
fn simultaneous_ko_contacts_survive_settlement() {
    let mut config = config();
    for f in &mut config.fighters {
        f.max_health = f.attack.damage;
    }
    let mut duel = Duel::new(config).unwrap();
    let attack = Controls {
        attack: true,
        ..Default::default()
    };
    tick(&mut duel, attack, attack);
    assert!(
        duel.fighters()
            .iter()
            .all(|f| f.health == 0 && f.state == State::Ko)
    );
    assert_eq!(
        duel.events()
            .iter()
            .filter(|e| matches!(e, Event::Hit { .. }))
            .count(),
        2
    );
    assert_eq!(
        duel.events()
            .iter()
            .filter(|e| matches!(e, Event::Knockout { .. }))
            .count(),
        2
    );
    tick(&mut duel, attack, attack);
    assert!(duel.events().is_empty());
}

#[test]
fn maximum_legal_phase_lengths_do_not_wrap() {
    let mut config = config();
    let attack = &mut config.fighters[0].attack;
    attack.startup = u32::MAX;
    attack.active = u32::MAX;
    attack.recovery = u32::MAX;
    config.lane.startup = u32::MAX;
    config.lane.travel = u32::MAX;
    config.lane.recovery = u32::MAX;
    let mut duel = Duel::new(config).unwrap();
    let idle = Controls::default();
    tick(
        &mut duel,
        Controls {
            attack: true,
            ..idle
        },
        Controls {
            switch_lane: true,
            ..idle
        },
    );
    assert_eq!(duel.fighters()[0].state, State::Attack);
    assert_eq!(duel.fighters()[1].state, State::LaneSwitch);
    assert_eq!(duel.fighters()[1].health, 5);
}

#[test]
fn extreme_mirrored_pull_checks_the_final_coordinate() {
    let mut config = config();
    config.fighters[0].spawn.x = -2;
    config.fighters[1].spawn.x = 0;
    config.fighters[1].attack.pushback = i32::MIN;
    let mut duel = Duel::new(config).unwrap();
    let idle = Controls::default();
    let attack = Controls {
        attack: true,
        ..idle
    };
    tick(&mut duel, idle, attack);
    assert_eq!(duel.fighters()[0].position.x, i32::MAX - 1);

    config.fighters[0].spawn.x = 0;
    config.fighters[1].spawn.x = 2;
    let mut duel = Duel::new(config).unwrap();
    assert_eq!(duel.step(1, [idle, attack]), Err(DuelError::Overflow));
    assert_eq!(duel.frame(), 0);
}

#[test]
fn knockout_ends_both_kinds_of_switch_on_a_lane() {
    for interruptible in [true, false] {
        let mut config = config();
        config.lane.interruptible = interruptible;
        config.fighters[0].attack.lane_rule = LaneRule::BothLanes;
        config.fighters[1].max_health = 1;
        let mut duel = Duel::new(config).unwrap();
        let idle = Controls::default();
        tick(
            &mut duel,
            idle,
            Controls {
                switch_lane: true,
                ..idle
            },
        );
        tick(
            &mut duel,
            Controls {
                attack: true,
                ..idle
            },
            idle,
        );
        assert_eq!(duel.fighters()[1].state, State::Ko);
        assert_eq!(duel.fighters()[1].position.z, FAR);
    }
}
