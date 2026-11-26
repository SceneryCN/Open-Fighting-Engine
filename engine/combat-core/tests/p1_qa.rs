// SPDX-License-Identifier: MPL-2.0
//! Black-box acceptance cases for the ADR-004 Rust API.
use ofe_combat_core::Position;
use ofe_combat_core::duel::*;

fn fighter(x: i32, z: i32) -> FighterConfig {
    FighterConfig {
        spawn: Position { x, z, h: 0 },
        max_health: 100,
        walk_speed: 5,
        jump_speed: 20,
        gravity: 4,
        hurtbox: Rect {
            x: -2,
            h: 0,
            width: 4,
            height: 10,
        },
        attack: AttackConfig {
            startup: 0,
            active: 3,
            recovery: 2,
            damage: 25,
            hitstun: 2,
            blockstun: 2,
            pushback: 3,
            hitbox: Rect {
                x: 0,
                h: 0,
                width: 10,
                height: 10,
            },
            lane_rule: LaneRule::SameLane,
        },
    }
}

fn config(distance: i32) -> DuelConfig {
    DuelConfig {
        fighters: [fighter(0, 0), fighter(distance, 0)],
        lane: LaneConfig {
            startup: 1,
            travel: 2,
            recovery: 1,
            cooldown: 2,
            interruptible: true,
        },
    }
}

fn create(config: DuelConfig) -> Duel {
    match Duel::new(config) {
        Ok(duel) => duel,
        Err(_) => panic!("valid test configuration rejected"),
    }
}

fn neutral() -> Controls {
    Controls::default()
}
fn attack() -> Controls {
    Controls {
        attack: true,
        ..neutral()
    }
}
fn switch() -> Controls {
    Controls {
        switch_lane: true,
        ..neutral()
    }
}
fn tick(duel: &mut Duel, a: Controls, b: Controls) {
    assert!(duel.step(duel.frame() + 1, [a, b]).is_ok());
}
fn hits(duel: &Duel) -> Vec<(u32, u32, i32)> {
    duel.events()
        .iter()
        .filter_map(|event| match event {
            Event::Hit {
                attacker,
                target,
                damage,
            } => Some((*attacker, *target, *damage)),
            _ => None,
        })
        .collect()
}

#[test]
fn active_hitbox_is_half_open_and_mirrored_for_both_sides() {
    for (distance, expected) in [(12, 100), (11, 75), (-12, 100), (-11, 75)] {
        let mut game = create(config(distance));
        tick(&mut game, attack(), neutral());
        assert_eq!(
            game.fighters()[1].health,
            expected,
            "displacement {distance}"
        );
    }
}

#[test]
fn startup_active_and_recovery_have_one_contact_per_instance() {
    let mut cfg = config(8);
    cfg.fighters[0].attack.startup = 2;
    cfg.fighters[0].attack.pushback = 0;
    let mut game = create(cfg);
    tick(&mut game, attack(), neutral());
    assert!(hits(&game).is_empty());
    tick(&mut game, neutral(), neutral());
    assert!(hits(&game).is_empty());
    tick(&mut game, neutral(), neutral());
    assert_eq!(hits(&game), [(1, 2, 25)]);
    assert!(matches!(game.fighters()[1].state, State::Hitstun));
    for _ in 0..4 {
        tick(&mut game, neutral(), neutral());
        assert!(hits(&game).is_empty());
    }
    assert_eq!(game.fighters()[1].health, 75);
}

#[test]
fn simultaneous_knockouts_apply_both_queued_attacks() {
    let mut cfg = config(8);
    for actor in &mut cfg.fighters {
        actor.max_health = 20;
    }
    let mut game = create(cfg);
    tick(&mut game, attack(), attack());
    let mut pairs: Vec<_> = hits(&game).into_iter().map(|(a, b, _)| (a, b)).collect();
    pairs.sort();
    assert_eq!(pairs, [(1, 2), (2, 1)]);
    assert!(
        game.fighters()
            .iter()
            .all(|f| f.health == 0 && matches!(f.state, State::Ko))
    );
    let mut ko: Vec<_> = game
        .events()
        .iter()
        .filter_map(|e| match e {
            Event::Knockout { id } => Some(*id),
            _ => None,
        })
        .collect();
    ko.sort();
    assert_eq!(ko, [1, 2]);
    tick(&mut game, attack(), attack());
    assert!(hits(&game).is_empty());
}

#[test]
fn guard_blocks_once_without_loss_and_unblocked_hit_pushes_away() {
    let mut game = create(config(8));
    tick(
        &mut game,
        attack(),
        Controls {
            guard: true,
            ..neutral()
        },
    );
    assert_eq!(game.fighters()[1].health, 100);
    assert!(matches!(game.fighters()[1].state, State::Blockstun));
    assert!(game.events().iter().any(|e| matches!(
        e,
        Event::Blocked {
            attacker: 1,
            target: 2
        }
    )));
    for _ in 0..2 {
        tick(&mut game, neutral(), neutral());
        assert_eq!(game.fighters()[1].health, 100);
    }
    let mut game = create(config(-8));
    tick(&mut game, attack(), neutral());
    assert_eq!(game.fighters()[1].position.x, -11);
}

#[test]
fn lane_threshold_and_closed_depth_range_use_post_movement_positions() {
    for (rule, health) in [
        (LaneRule::SameLane, 100),
        (LaneRule::CrossLane, 75),
        (LaneRule::BothLanes, 75),
        (LaneRule::DepthRange(499), 100),
        (LaneRule::DepthRange(500), 75),
    ] {
        let mut cfg = config(8);
        cfg.fighters[0].attack.lane_rule = rule;
        let mut game = create(cfg);
        tick(&mut game, neutral(), switch());
        assert_eq!(game.fighters()[1].position.z, 0);
        tick(&mut game, attack(), neutral());
        assert_eq!(game.fighters()[1].health, health);
        if health == 100 {
            assert_eq!(game.fighters()[1].position.z, 500);
        } else {
            assert_eq!(game.fighters()[1].position.z, 1000);
        }
    }
}

#[test]
fn uninterruptible_crossing_finishes_travel_and_cooldown() {
    let mut cfg = config(8);
    cfg.fighters[0].attack.lane_rule = LaneRule::BothLanes;
    cfg.lane.interruptible = false;
    let mut game = create(cfg);
    tick(&mut game, neutral(), switch());
    tick(&mut game, attack(), neutral());
    assert_eq!(game.fighters()[1].position.z, 500);
    assert_eq!(game.fighters()[1].health, 75);
    tick(&mut game, neutral(), neutral());
    assert_eq!(game.fighters()[1].position.z, 1000);
    for _ in 0..3 {
        tick(&mut game, neutral(), switch());
        assert_eq!(game.fighters()[1].position.z, 1000);
    }
    tick(&mut game, neutral(), switch());
    tick(&mut game, neutral(), neutral());
    assert_eq!(game.fighters()[1].position.z, 500);
}

#[test]
fn neutral_frames_land_after_airborne_height_prevents_hit() {
    let mut game = create(config(8));
    tick(
        &mut game,
        attack(),
        Controls {
            jump: true,
            ..neutral()
        },
    );
    assert!(game.fighters()[1].position.h >= 10);
    assert!(matches!(game.fighters()[1].state, State::Jump));
    assert!(hits(&game).is_empty());
    let mut falling = false;
    for _ in 0..20 {
        tick(&mut game, neutral(), neutral());
        falling |= matches!(game.fighters()[1].state, State::Fall);
        assert!(game.fighters()[1].position.h >= 0);
    }
    assert!(falling);
    assert!(matches!(game.fighters()[1].state, State::Idle));
    assert_eq!(game.fighters()[1].position.h, 0);
}

#[test]
fn rejected_step_preserves_observable_state_and_internal_attack_age() {
    let mut cfg = config(8);
    cfg.fighters[0].attack.startup = 1;
    let mut game = create(cfg);
    tick(&mut game, attack(), neutral());
    let previous_count = game.events().len();
    assert!(previous_count > 0);
    assert!(matches!(
        game.step(1, [neutral(), neutral()]),
        Err(DuelError::WrongFrame)
    ));
    assert!(matches!(
        game.step(
            2,
            [
                neutral(),
                Controls {
                    axis: 2,
                    ..neutral()
                }
            ]
        ),
        Err(DuelError::InvalidInput)
    ));
    assert_eq!(game.frame(), 1);
    assert_eq!(game.fighters()[1].health, 100);
    assert_eq!(game.events().len(), previous_count);
    tick(&mut game, neutral(), neutral());
    assert_eq!(hits(&game), [(1, 2, 25)]);
}

#[test]
fn invalid_geometry_lanes_and_durations_are_rejected() {
    for mutation in 0..7 {
        let mut cfg = config(8);
        match mutation {
            0 => cfg.fighters[1].hurtbox.height = 0,
            1 => cfg.fighters[0].attack.lane_rule = LaneRule::DepthRange(-1),
            2 => cfg.fighters[1].attack.active = 0,
            3 => cfg.fighters[0].spawn.z = 499,
            4 => cfg.fighters[0].gravity = 0,
            5 => cfg.fighters[0].max_health = -1,
            _ => cfg.lane.travel = 0,
        }
        assert!(matches!(Duel::new(cfg), Err(DuelError::InvalidConfig)));
    }
}

#[test]
fn two_player_walk_and_failed_overflow_roll_back_the_entire_tick() {
    let mut cfg = config(i32::MAX - 5);
    cfg.fighters[1].spawn.z = 1000;
    let mut game = create(cfg);
    let walk = |axis| Controls { axis, ..neutral() };
    tick(&mut game, walk(-1), walk(1));
    assert_eq!(game.fighters()[0].position.x, -5);
    assert_eq!(game.fighters()[1].position.x, i32::MAX);
    assert!(
        game.events().iter().any(
            |event| matches!(event, Event::Moved { id: 2, position } if position.x == i32::MAX)
        )
    );
    let before = *game.fighters();
    let previous_events = game.events().to_vec();
    assert_eq!(game.step(2, [walk(1), walk(1)]), Err(DuelError::Overflow));
    assert_eq!(game.frame(), 1);
    assert_eq!(*game.fighters(), before);
    assert_eq!(game.events(), previous_events);
    tick(&mut game, walk(1), walk(-1));
    assert_eq!(game.fighters()[0].position.x, 0);
    assert_eq!(game.fighters()[1].position.x, i32::MAX - 5);
}

#[test]
fn identical_input_repeats_states_and_ordered_events_each_frame() {
    let cfg = config(8);
    let mut first = create(cfg);
    let mut replay = create(cfg);
    let mut tick_count = 0;
    for index in 0..240 {
        let controls = |offset| {
            let tick = index + offset;
            Controls {
                axis: (tick % 3) - 1,
                jump: tick % 23 == 0,
                switch_lane: tick % 11 == 0,
                attack: tick % 5 == 0,
                guard: tick % 7 == 0,
            }
        };
        for game in [&mut first, &mut replay] {
            tick(game, controls(1), controls(3));
        }
        assert_eq!(first.fighters(), replay.fighters());
        assert_eq!(first.events(), replay.events());
        tick_count += 1;
    }
    assert_eq!(first.frame(), tick_count);
}

#[test]
fn walking_behind_a_locked_attack_avoids_its_active_frame() {
    let mut cfg = config(5);
    cfg.fighters[0].attack.startup = 1;
    cfg.fighters[1].walk_speed = 10;
    let mut game = create(cfg);
    tick(&mut game, attack(), neutral());
    tick(
        &mut game,
        neutral(),
        Controls {
            axis: -1,
            ..neutral()
        },
    );
    assert_eq!(game.fighters()[1].position.x, -5);
    assert_eq!(game.fighters()[1].health, 100);
    assert!(hits(&game).is_empty());
}
