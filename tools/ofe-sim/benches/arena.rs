// SPDX-License-Identifier: MPL-2.0
//! Four fighters at one position give twelve directed strike contacts in FFA.
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use ofe_combat_core::arena::{Arena, ArenaEvent, Command};
use ofe_combat_core::duel::{Controls, Event, Rect};
use ofe_sim::{ArenaMode, sample_arena_config};
use std::hint::black_box;

fn benchmark(c: &mut Criterion) {
    let mut config = sample_arena_config(ArenaMode::FreeForAll).expect("valid sample roster");
    let area = Rect {
        x: -8,
        h: 0,
        width: 16,
        height: 8,
    };
    let mut commands = Vec::new();
    for fighter in &mut config.fighters {
        fighter.base.spawn.x = 0;
        fighter.base.spawn.z = 0;
        fighter.base.spawn.h = 0;
        fighter.base.hurtbox = area;
        let normal = &mut fighter.moves[0];
        normal.base.startup = 0;
        normal.base.hitbox = area;
        normal.windows.clear();
        fighter.base.attack = normal.base;
        commands.push(Command {
            id: fighter.id,
            controls: Controls::default(),
            dash: false,
            move_index: Some(0),
        });
    }
    let fresh = || Arena::new(config.clone()).unwrap();
    let mut probe = fresh();
    probe.step(1, &commands).unwrap();
    assert_eq!(
        probe
            .events()
            .iter()
            .filter(|e| matches!(e, ArenaEvent::Base(Event::Hit { .. })))
            .count(),
        12
    );
    let mut idle = fresh();
    c.bench_function("arena/four_idle", |b| {
        b.iter(|| {
            idle.step(idle.frame() + 1, &[]).unwrap();
            black_box(idle.fighters());
        })
    });
    c.bench_function("arena/twelve_contacts", |b| {
        b.iter_batched_ref(
            fresh,
            |arena| {
                arena.step(1, black_box(&commands)).unwrap();
                black_box(arena.events());
            },
            BatchSize::SmallInput,
        )
    });
}
criterion_group!(benches, benchmark);
criterion_main!(benches);
