use criterion::{Criterion, criterion_group, criterion_main};
use ofe_combat_core::Battle;
use std::hint::black_box;

fn foundation(c: &mut Criterion) {
    let mut battle = Battle::new();
    c.bench_function("p0/submit_step_event", |b| {
        b.iter(|| {
            let frame = battle.snapshot().frame + 1;
            battle
                .submit(frame, if frame % 2 == 0 { 1 } else { -1 })
                .unwrap();
            battle.step().unwrap();
            black_box(battle.movement_event());
        })
    });
    c.bench_function("p0/snapshot_hash", |b| {
        b.iter(|| black_box(battle.snapshot()).state_hash())
    });
}
criterion_group!(benches, foundation);
criterion_main!(benches);
