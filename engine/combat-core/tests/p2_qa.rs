// SPDX-License-Identifier: MPL-2.0
//! Independent black-box checks for the P2 combat contract in ADR-005.
use ofe_combat_core::Position;
use ofe_combat_core::arena::*;
use ofe_combat_core::duel::*;

fn strike() -> MoveConfig {
    MoveConfig {
        base: AttackConfig {
            startup: 0,
            active: 1,
            recovery: 4,
            damage: 7,
            hitstun: 2,
            blockstun: 1,
            pushback: 0,
            hitbox: Rect {
                x: -16,
                h: 0,
                width: 32,
                height: 8,
            },
            lane_rule: LaneRule::SameLane,
        },
        kind: MoveKind::Strike,
        hitstop: 0,
        windows: vec![],
        cancels: vec![],
        armor: 0,
        invulnerability: 0,
        energy_cost: 0,
    }
}

fn actor(id: u32, team: u32, x: i32, moves: Vec<MoveConfig>) -> CombatantConfig {
    CombatantConfig {
        id,
        team,
        base: FighterConfig {
            spawn: Position { x, z: 0, h: 0 },
            max_health: 60,
            walk_speed: 2,
            jump_speed: 9,
            gravity: 2,
            hurtbox: Rect {
                x: -1,
                h: 0,
                width: 2,
                height: 5,
            },
            attack: moves[0].base,
        },
        moves,
        dash_speed: 5,
        dash_ticks: 3,
        max_energy: 10,
        initial_energy: 10,
    }
}

fn cfg(fighters: Vec<CombatantConfig>) -> ArenaConfig {
    ArenaConfig {
        fighters,
        lane: LaneConfig {
            startup: 0,
            travel: 2,
            recovery: 0,
            cooldown: 2,
            interruptible: true,
        },
    }
}
fn arena(fighters: Vec<CombatantConfig>) -> Arena {
    Arena::new(cfg(fighters)).unwrap()
}
fn cmd(id: u32) -> Command {
    Command {
        id,
        controls: Controls::default(),
        dash: false,
        move_index: None,
    }
}
fn attack(id: u32, index: usize) -> Command {
    Command {
        move_index: Some(index),
        ..cmd(id)
    }
}
fn guard(id: u32) -> Command {
    Command {
        controls: Controls {
            guard: true,
            ..Controls::default()
        },
        ..cmd(id)
    }
}
fn tick(game: &mut Arena, commands: &[Command]) {
    game.step(game.frame() + 1, commands).unwrap();
}
fn who(game: &Arena, id: u32) -> &ArenaFighter {
    game.fighters().iter().find(|f| f.fighter.id == id).unwrap()
}
fn count_hits(game: &Arena) -> usize {
    game.events()
        .iter()
        .filter(|e| matches!(e, ArenaEvent::Base(Event::Hit { .. })))
        .count()
}

#[test]
fn four_player_contacts_are_order_independent_and_team_filtered() {
    for (teams, each_damage, contacts) in [([1, 1, 2, 2], 14, 8), ([1, 2, 3, 4], 21, 12)] {
        let ids = [41, 7, 23, 12];
        let source = || {
            (0..4)
                .map(|i| actor(ids[i], teams[i], i as i32, vec![strike()]))
                .collect::<Vec<_>>()
        };
        let mut normal = arena(source());
        let mut reversed = source();
        reversed.reverse();
        let mut permuted = arena(reversed);
        let cmds = || ids.map(|id| attack(id, 0));
        tick(&mut normal, &cmds());
        let mut input = cmds();
        input.reverse();
        tick(&mut permuted, &input);
        assert_eq!(
            normal
                .fighters()
                .iter()
                .map(|f| f.fighter.id)
                .collect::<Vec<_>>(),
            [7, 12, 23, 41]
        );
        assert_eq!(normal.fighters(), permuted.fighters());
        assert_eq!(normal.events(), permuted.events());
        assert_eq!(count_hits(&normal), contacts);
        for f in normal.fighters() {
            assert_eq!(f.fighter.health, 60 - each_damage);
        }
    }
}

#[test]
fn four_lethal_strikes_all_settle_before_ko() {
    let mut game = arena(
        (1..=4)
            .map(|id| {
                let mut f = actor(id, id, 0, vec![strike()]);
                f.base.max_health = 1;
                f
            })
            .collect(),
    );
    tick(
        &mut game,
        &[attack(4, 0), attack(2, 0), attack(1, 0), attack(3, 0)],
    );
    assert_eq!(count_hits(&game), 12);
    assert_eq!(
        game.events()
            .iter()
            .filter(|e| matches!(e, ArenaEvent::Base(Event::Knockout { .. })))
            .count(),
        4
    );
    assert!(
        game.fighters()
            .iter()
            .all(|f| f.fighter.state == State::Ko && f.fighter.health == 0)
    );
}

#[test]
fn exact_dash_ticks_then_normal_walk_and_input_validation_is_atomic() {
    let mut game = arena(vec![
        actor(1, 1, 0, vec![strike()]),
        actor(2, 2, 100, vec![strike()]),
    ]);
    tick(
        &mut game,
        &[Command {
            dash: true,
            ..cmd(1)
        }],
    );
    assert!(who(&game, 1).dashing);
    assert!(
        game.events()
            .iter()
            .any(|e| matches!(e, ArenaEvent::Dashed { id: 1 }))
    );
    for invalid in [
        vec![cmd(9)],
        vec![cmd(2), cmd(2)],
        vec![attack(1, 99)],
        vec![Command {
            controls: Controls {
                axis: 2,
                ..Default::default()
            },
            ..cmd(1)
        }],
    ] {
        let state = game.fighters().to_vec();
        let events = game.events().to_vec();
        assert!(game.step(2, &invalid).is_err());
        assert_eq!(game.frame(), 1);
        assert_eq!(game.fighters(), state);
        assert_eq!(game.events(), events);
    }
    for x in [10, 15] {
        tick(&mut game, &[]);
        assert_eq!(who(&game, 1).fighter.position.x, x);
    }
    tick(
        &mut game,
        &[Command {
            controls: Controls {
                axis: -1,
                ..Default::default()
            },
            ..cmd(1)
        }],
    );
    assert!(!who(&game, 1).dashing);
    assert_eq!(who(&game, 1).fighter.position.x, 13);
}

#[test]
fn freeze_uses_maximum_and_resumes_hit_and_block_stun() {
    for blocking in [true, false] {
        let mut light = strike();
        light.hitstop = 1;
        let mut heavy = strike();
        heavy.hitstop = 3;
        heavy.base.hitstun = 4;
        heavy.base.blockstun = 3;
        let mut game = arena(vec![
            actor(4, 1, 0, vec![light]),
            actor(8, 1, 1, vec![heavy]),
            actor(9, 2, 2, vec![strike()]),
        ]);
        let response = if blocking { guard(9) } else { cmd(9) };
        tick(&mut game, &[attack(4, 0), attack(8, 0), response]);
        let state = if blocking {
            State::Blockstun
        } else {
            State::Hitstun
        };
        let target = who(&game, 9);
        assert_eq!(target.fighter.state, state);
        assert_eq!(target.hitstop, 3);
        assert_eq!(target.fighter.health, if blocking { 60 } else { 46 });
        assert_eq!(who(&game, 4).combo, if blocking { 0 } else { 1 });
        for remaining in [2, 1, 0] {
            tick(
                &mut game,
                &[Command {
                    dash: true,
                    ..cmd(9)
                }],
            );
            assert_eq!(who(&game, 9).hitstop, remaining);
            assert_eq!(who(&game, 9).fighter.state, state);
        }
        for _ in 0..if blocking { 3 } else { 4 } {
            tick(&mut game, &[attack(9, 0)]);
            assert_eq!(who(&game, 9).fighter.state, state);
        }
        tick(&mut game, &[attack(9, 0)]);
        assert_eq!(who(&game, 9).move_index, Some(0));
    }
}

#[test]
fn hit_confirm_cancel_first_and_last_tick_and_super_cost_once() {
    for (attempt, distance, energy, succeeds) in [
        (1, 3, 10, false),
        (2, 3, 10, true),
        (3, 3, 10, true),
        (4, 3, 10, false),
        (2, 70, 10, false),
        (2, 3, 9, false),
    ] {
        let mut normal = strike();
        normal.base.recovery = 5;
        normal.cancels = vec![Cancel {
            window: Window { start: 2, end: 3 },
            into: 1,
        }];
        let mut super_move = strike();
        super_move.kind = MoveKind::Super;
        super_move.energy_cost = 10;
        let mut hero = actor(1, 1, 0, vec![normal, super_move]);
        hero.initial_energy = energy;
        let mut game = arena(vec![hero, actor(2, 2, distance, vec![strike()])]);
        tick(&mut game, &[attack(1, 0)]);
        for _ in 1..attempt {
            tick(&mut game, &[]);
        }
        tick(&mut game, &[attack(1, 1)]);
        assert_eq!(who(&game, 1).move_index, Some(if succeeds { 1 } else { 0 }));
        assert_eq!(who(&game, 1).energy, if succeeds { 0 } else { energy });
        assert_eq!(
            game.events().iter().any(|e| matches!(
                e,
                ArenaEvent::Cancelled {
                    id: 1,
                    from: 0,
                    into: 1
                }
            )),
            succeeds
        );
        tick(&mut game, &[attack(1, 1)]);
        assert!(who(&game, 1).energy >= 0);
    }
}

#[test]
fn each_explicit_window_can_hit_each_target_once_and_combo_expires() {
    let mut m = strike();
    m.base.active = 5;
    m.base.hitstun = 6;
    m.windows = vec![Window { start: 0, end: 1 }, Window { start: 3, end: 4 }];
    let mut game = arena(vec![
        actor(1, 1, 0, vec![m]),
        actor(2, 2, 1, vec![strike()]),
        actor(3, 2, 2, vec![strike()]),
    ]);
    tick(&mut game, &[attack(1, 0)]);
    assert_eq!(count_hits(&game), 2);
    for expected in [0, 0, 2, 0] {
        tick(&mut game, &[]);
        assert_eq!(count_hits(&game), expected);
    }
    assert_eq!(who(&game, 1).combo, 2);
    assert_eq!(who(&game, 2).fighter.health, 46);
    assert_eq!(who(&game, 3).fighter.health, 46);
    for _ in 0..8 {
        tick(&mut game, &[]);
    }
    assert_eq!(who(&game, 1).combo, 0);
}

#[test]
fn throw_bypasses_guard_and_armor_but_not_invulnerability() {
    for (protection, expected) in [("guard", 53), ("armor", 53), ("invulnerable", 60)] {
        let mut grab = strike();
        grab.kind = MoveKind::Throw;
        let mut startup = strike();
        startup.base.startup = 3;
        startup.armor = u32::from(protection == "armor");
        startup.invulnerability = u32::from(protection == "invulnerable");
        let mut game = arena(vec![
            actor(1, 1, 0, vec![grab]),
            actor(2, 2, 2, vec![startup]),
        ]);
        tick(
            &mut game,
            &[
                attack(1, 0),
                if protection == "guard" {
                    guard(2)
                } else {
                    attack(2, 0)
                },
            ],
        );
        assert_eq!(who(&game, 2).fighter.health, expected);
        if expected < 60 {
            assert_eq!(who(&game, 2).fighter.state, State::Hitstun);
        } else {
            assert_eq!(who(&game, 2).fighter.state, State::Attack);
        }
    }
}

#[test]
fn simultaneous_sources_spend_last_armor_in_stable_id_order() {
    let mut startup = strike();
    startup.base.startup = 4;
    startup.armor = 1;
    let mut game = arena(vec![
        actor(11, 1, 1, vec![strike()]),
        actor(3, 2, 2, vec![startup]),
        actor(6, 1, 0, vec![strike()]),
    ]);
    tick(&mut game, &[attack(11, 0), attack(3, 0), attack(6, 0)]);
    assert!(game.events().iter().any(|e| matches!(
        e,
        ArenaEvent::Armored {
            attacker: 6,
            target: 3
        }
    )));
    assert!(!game.events().iter().any(|e| matches!(
        e,
        ArenaEvent::Armored {
            attacker: 11,
            target: 3
        }
    )));
    assert_eq!(who(&game, 3).fighter.health, 46);
    assert_eq!(who(&game, 3).fighter.state, State::Hitstun);
}

#[test]
fn projectile_hits_after_caster_recovers_and_expires_at_lifetime() {
    let mut shot = strike();
    shot.kind = MoveKind::Projectile {
        speed: 5,
        lifetime: 4,
    };
    shot.base.recovery = 0;
    shot.base.hitbox = Rect {
        x: 0,
        h: 0,
        width: 3,
        height: 8,
    };
    let mut game = arena(vec![
        actor(1, 1, 0, vec![shot]),
        actor(2, 2, 11, vec![strike()]),
        actor(3, 2, 16, vec![strike()]),
    ]);
    tick(&mut game, &[attack(1, 0)]);
    assert_eq!(game.projectiles().len(), 1);
    assert_eq!(game.projectiles()[0].position.x, 0);
    assert!(
        game.events()
            .iter()
            .any(|e| matches!(e, ArenaEvent::ProjectileSpawned { owner: 1, .. }))
    );
    tick(&mut game, &[]);
    assert_eq!(who(&game, 1).move_index, None);
    assert_eq!(game.projectiles()[0].position.x, 5);
    assert_eq!(count_hits(&game), 0);
    tick(&mut game, &[]);
    assert_eq!(who(&game, 2).fighter.health, 53);
    tick(&mut game, &[]);
    assert_eq!(who(&game, 3).fighter.health, 53);
    tick(&mut game, &[]);
    assert!(game.projectiles().is_empty());
    assert_eq!(who(&game, 2).fighter.health, 53);
}

#[test]
fn rejects_bad_orders_energy_windows_and_profile_combinations() {
    for variant in 0..9 {
        let mut a = actor(1, 1, 0, vec![strike()]);
        let mut b = actor(2, 2, 10, vec![strike()]);
        match variant {
            0 => b.id = 1,
            1 => b.team = 1,
            2 => a.initial_energy = 11,
            3 => a.moves[0].energy_cost = -1,
            4 => {
                a.moves[0].kind = MoveKind::Projectile {
                    speed: 0,
                    lifetime: 2,
                }
            }
            5 => a.moves[0].windows = vec![Window { start: 0, end: 1 }],
            6 => {
                a.moves[0].cancels = vec![Cancel {
                    window: Window { start: 0, end: 0 },
                    into: 17,
                }]
            }
            7 => a.base.attack.damage += 1,
            _ => a.dash_ticks = 0,
        }
        assert!(
            matches!(Arena::new(cfg(vec![a, b])), Err(DuelError::InvalidConfig)),
            "variant {variant}"
        );
    }
}
