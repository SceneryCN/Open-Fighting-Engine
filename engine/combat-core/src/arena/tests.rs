// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::duel::*;

fn roster(kind: MoveKind) -> ArenaConfig {
    let body = Rect {
        x: -1,
        h: 0,
        width: 2,
        height: 2,
    };
    let strike = AttackConfig {
        startup: 0,
        active: 1,
        recovery: 0,
        damage: 10,
        hitstun: 1,
        blockstun: 1,
        pushback: 0,
        hitbox: Rect {
            x: 0,
            width: 3,
            ..body
        },
        lane_rule: LaneRule::SameLane,
    };
    let fighters = (1..=2)
        .map(|id| CombatantConfig {
            id,
            team: id,
            base: FighterConfig {
                spawn: Position {
                    x: if id == 1 { 0 } else { 2 },
                    z: 0,
                    h: 0,
                },
                max_health: 10,
                walk_speed: 3,
                jump_speed: 2,
                gravity: 1,
                hurtbox: body,
                attack: strike,
            },
            moves: vec![MoveConfig {
                base: strike,
                kind,
                hitstop: 0,
                windows: vec![],
                cancels: vec![],
                armor: 0,
                invulnerability: 0,
                energy_cost: 3,
            }],
            initial_energy: 6,
            max_energy: 6,
            dash_speed: 6,
            dash_ticks: 2,
        })
        .collect();
    ArenaConfig {
        fighters,
        lane: LaneConfig {
            startup: 1,
            travel: 2,
            recovery: 1,
            cooldown: 2,
            interruptible: true,
        },
    }
}

#[test]
fn duel_survivor_turns_after_walking_past_defeated_opponent() {
    let arena = roster(MoveKind::Strike);
    let mut duel = Duel::new(DuelConfig {
        fighters: [arena.fighters[0].base, arena.fighters[1].base],
        lane: arena.lane,
    })
    .unwrap();
    let idle = Controls::default();
    duel.step(
        1,
        [
            Controls {
                attack: true,
                ..idle
            },
            idle,
        ],
    )
    .unwrap();
    assert_eq!(duel.fighters()[1].state, State::Ko);
    duel.step(2, [Controls { axis: 1, ..idle }, idle]).unwrap();
    duel.step(3, [idle, idle]).unwrap();
    assert_eq!(duel.fighters()[0].facing, -1);
}

#[test]
fn extreme_displacements_fail_without_writing() {
    for (original, delta) in [(1, i64::MAX), (-1, i64::MIN), (i32::MAX, 1), (i32::MIN, -1)] {
        let mut x = original;
        assert_eq!(shift(&mut x, delta), Err(DuelError::Overflow));
        assert_eq!(x, original);
    }
}

#[test]
fn projectile_id_exhaustion_rolls_back_cast_cost_and_generated_events() {
    let mut arena = Arena::new(roster(MoveKind::Projectile {
        speed: 1,
        lifetime: 2,
    }))
    .unwrap();
    arena
        .step(
            1,
            &[Command {
                id: 1,
                controls: Controls {
                    axis: -1,
                    ..Default::default()
                },
                ..Default::default()
            }],
        )
        .unwrap();
    let previous = arena.events().to_vec();
    arena.world.last_id = u64::MAX - 1;
    let cast = Command {
        id: 1,
        move_index: Some(0),
        ..Default::default()
    };
    assert_eq!(arena.step(2, &[cast]), Err(DuelError::Overflow));
    assert_eq!(arena.frame(), 1);
    assert_eq!(arena.world.last_id, u64::MAX - 1);
    assert_eq!(arena.fighters()[0].energy, 6);
    assert_eq!(arena.events(), previous);
    assert!(arena.projectiles().is_empty());
    arena.step(2, &[]).unwrap();
}

#[test]
fn projectile_remains_dangerous_after_its_owner_is_knocked_out() {
    let mut config = roster(MoveKind::Strike);
    config.fighters[0].moves[0].kind = MoveKind::Projectile {
        speed: 3,
        lifetime: 2,
    };
    config.fighters[1].base.spawn.x = 5;
    let opponent = &mut config.fighters[1];
    opponent.base.attack.hitbox.width = 6;
    opponent.moves[0].base = opponent.base.attack;
    let mut arena = Arena::new(config).unwrap();
    let attack = |id| Command {
        id,
        move_index: Some(0),
        ..Default::default()
    };
    arena.step(1, &[attack(2), attack(1)]).unwrap();
    assert_eq!(arena.fighters()[0].fighter.state, State::Ko);
    assert_eq!(arena.fighters()[1].fighter.health, 10);
    arena.step(2, &[]).unwrap();
    assert_eq!(arena.fighters()[1].fighter.state, State::Ko);
    assert_eq!(arena.projectiles()[0].position.x, 3);
    arena.step(3, &[]).unwrap();
    assert!(arena.projectiles().is_empty());
}

#[test]
fn retry_discards_partially_rebuilt_projectiles_and_events() {
    let mut config = roster(MoveKind::Projectile {
        speed: 2,
        lifetime: 12,
    });
    for actor in &mut config.fighters {
        actor.moves[0].energy_cost = 0;
    }
    config.fighters[1].base.spawn.x = 300;
    let mut arena = Arena::new(config).unwrap();
    let cast = Command {
        id: 1,
        move_index: Some(0),
        ..Default::default()
    };
    arena.step(1, &[cast]).unwrap();
    arena.step(2, &[cast]).unwrap();
    // Allocating the cast ID succeeds, copying both live projectiles succeeds, and
    // the new projectile ID fails. A retry must rebuild exclusively from frame 2.
    arena.world.last_id = u64::MAX - 1;
    let mut uninterrupted = arena.clone();
    let failed = arena.step(3, &[cast]);
    assert_eq!(failed, Err(DuelError::Overflow));
    for frame in 3..=15 {
        assert_eq!(arena.frame(), uninterrupted.frame());
        assert_eq!(arena.events(), uninterrupted.events());
        assert_eq!(arena.fighters(), uninterrupted.fighters());
        assert_eq!(arena.projectiles(), uninterrupted.projectiles());
        arena.step(frame, &[]).unwrap();
        uninterrupted.step(frame, &[]).unwrap();
    }
    assert!(arena.projectiles().is_empty());
}

#[test]
fn idle_fast_path_validates_neutral_inputs_and_advances_hidden_cooldown() {
    let mut config = roster(MoveKind::Strike);
    config.lane = LaneConfig {
        startup: 0,
        travel: 1,
        recovery: 0,
        cooldown: 3,
        interruptible: true,
    };
    let mut arena = Arena::new(config).unwrap();
    let neutral = Command {
        id: 1,
        ..Default::default()
    };
    assert_eq!(
        arena.step(1, &[neutral, neutral]),
        Err(DuelError::InvalidInput)
    );
    assert_eq!(arena.step(2, &[]), Err(DuelError::WrongFrame));
    let switch = Command {
        controls: Controls {
            switch_lane: true,
            ..Default::default()
        },
        ..neutral
    };
    arena.step(1, &[switch]).unwrap();
    arena.step(2, &[]).unwrap();
    assert_eq!(arena.fighters()[0].fighter.state, State::Idle);
    arena.step(3, &[switch]).unwrap();
    arena.step(4, &[]).unwrap();
    assert_eq!(arena.fighters()[0].fighter.position.z, 1000);
    arena.step(5, &[switch]).unwrap();
    assert_eq!(arena.fighters()[0].fighter.position.z, 0);
}
