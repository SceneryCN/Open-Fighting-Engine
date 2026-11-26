// SPDX-License-Identifier: MPL-2.0
//! Measure public Duel ticks with the same OFC inputs as the standalone simulator.
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use ofe_combat_core::duel::{Controls, Duel, Event};
use ofe_sim::sample_config;
use std::hint::black_box;

fn duel_ticks(c: &mut Criterion) {
    let neutral = [Controls::default(); 2];
    let attacks = [Controls {
        attack: true,
        ..Controls::default()
    }; 2];
    let config = sample_config().expect("valid sample OFCs");
    let new_duel = || Duel::new(config).expect("valid duel");
    let start_attacks = || {
        let mut duel = new_duel();
        duel.step(1, attacks).unwrap();
        duel
    };

    let mut probe = start_attacks();
    probe.step(2, neutral).unwrap();
    assert_eq!(
        probe
            .events()
            .iter()
            .filter(|e| matches!(e, Event::Hit { .. }))
            .count(),
        2
    );

    let mut duel = new_duel();
    c.bench_function("duel/idle_tick", |b| {
        b.iter(|| {
            duel.step(duel.frame() + 1, black_box(neutral)).unwrap();
            black_box(duel.fighters());
        })
    });
    c.bench_function("duel/trade_tick", |b| {
        b.iter_batched_ref(
            start_attacks,
            |duel| {
                duel.step(2, black_box(neutral)).unwrap();
                black_box(duel.events());
            },
            BatchSize::SmallInput,
        )
    });
}
criterion_group!(benches, duel_ticks);
criterion_main!(benches);
