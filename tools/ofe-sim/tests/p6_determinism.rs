// SPDX-License-Identifier: MPL-2.0
//! Independent round trips using the shipped four-player rules and public Arena API.
use ofe_combat_core::arena::{Arena, ArenaEvent, ArenaSnapshot, Command};
use ofe_combat_core::duel::{Controls, Event, State};
use ofe_sim::{ArenaMode, sample_arena_config};

fn action(id: u32, index: usize) -> Command {
    Command {
        id,
        move_index: Some(index),
        ..Default::default()
    }
}

fn commands(frame: u64) -> Vec<Command> {
    match frame {
        1 => vec![action(1, 1), action(2, 1), action(3, 3)],
        24 => vec![action(1, 1)],
        26 => vec![action(1, 4)],
        40 => vec![Command {
            id: 1,
            controls: Controls {
                switch_lane: true,
                ..Default::default()
            },
            ..Default::default()
        }],
        66 => vec![Command {
            id: 4,
            controls: Controls {
                jump: true,
                ..Default::default()
            },
            ..Default::default()
        }],
        80..=96 => vec![Command {
            id: 4,
            controls: Controls {
                axis: 1,
                ..Default::default()
            },
            ..Default::default()
        }],
        _ => Vec::new(),
    }
}

#[test]
fn checkpoints_resume_trades_projectiles_cancels_switches_and_movement() {
    let rules = sample_arena_config(ArenaMode::FreeForAll).unwrap();
    let mut live = Arena::with_seed(rules.clone(), u64::MAX).unwrap();
    let mut scratch = Arena::with_seed(rules, 17).unwrap();
    assert_ne!(live.state_hash(), scratch.state_hash());
    live.set_infinite_health(true);
    let mut expected = Vec::new();
    let mut checkpoints = Vec::new();
    let mut observations = [false; 6];
    for frame in 1..=100 {
        live.step(frame, &commands(frame)).unwrap();
        let hits = live
            .events()
            .iter()
            .filter_map(|event| match event {
                ArenaEvent::Base(Event::Hit {
                    attacker, target, ..
                }) => Some((*attacker, *target)),
                _ => None,
            })
            .collect::<Vec<_>>();
        observations[0] |= hits.iter().any(|(a, b)| hits.contains(&(*b, *a)));
        observations[1] |= !live.projectiles().is_empty();
        observations[2] |= live
            .events()
            .iter()
            .any(|e| matches!(e, ArenaEvent::Cancelled { .. }));
        observations[3] |= live.fighters().iter().any(|f| f.combo > 1);
        observations[4] |= live.fighters()[0].fighter.state == State::LaneSwitch;
        observations[5] |= live.fighters()[3].fighter.position.x > 6;
        expected.push((
            live.state_hash(),
            live.fighters().to_vec(),
            live.events().to_vec(),
            live.projectiles().to_vec(),
        ));
        if matches!(frame, 1 | 3 | 24 | 26 | 41 | 46 | 67 | 90) {
            checkpoints.push((frame, serde_json::to_vec(&live.checkpoint()).unwrap()));
        }
    }
    assert_eq!(
        observations, [true; 6],
        "scenario must actually exercise each feature"
    );
    for (frame, bytes) in checkpoints {
        let decoded: ArenaSnapshot = serde_json::from_slice(&bytes).unwrap();
        scratch.restore(&decoded).unwrap();
        let fresh = Arena::from_checkpoint(&decoded).unwrap();
        assert_eq!(scratch.state_hash(), fresh.state_hash());
        for tick in frame..=100 {
            let (hash, fighters, events, projectiles) = &expected[tick as usize - 1];
            assert_eq!(
                (
                    scratch.state_hash(),
                    scratch.fighters(),
                    scratch.events(),
                    scratch.projectiles()
                ),
                (
                    *hash,
                    fighters.as_slice(),
                    events.as_slice(),
                    projectiles.as_slice()
                ),
                "restored from {frame}, at frame {tick}"
            );
            let before = scratch.state_hash();
            assert!(scratch.step(tick + 2, &[]).is_err());
            assert_eq!(scratch.state_hash(), before);
            scratch.step(tick + 1, &commands(tick + 1)).unwrap();
        }
    }
}

#[test]
#[ignore = "release performance measurement, run with --ignored --nocapture"]
fn four_fighter_checkpoint_and_replay_cost() {
    use std::{hint::black_box, time::Instant};
    let mut arena = Arena::new(sample_arena_config(ArenaMode::FreeForAll).unwrap()).unwrap();
    arena.set_infinite_health(true);
    let mut step = Vec::new();
    let mut save = Vec::new();
    let mut restore = Vec::new();
    let mut verify = Vec::new();
    let mut max_bytes = 0;
    for frame in 1..=6000 {
        let inputs = commands((frame - 1) % 100 + 1);
        let start = Instant::now();
        let snapshot = arena.checkpoint();
        let bytes = serde_json::to_vec(&snapshot).unwrap();
        save.push(start.elapsed().as_nanos());
        max_bytes = max_bytes.max(bytes.len());
        let start = Instant::now();
        let saved = serde_json::from_slice(&bytes).unwrap();
        let mut replay = Arena::from_checkpoint(&saved).unwrap();
        restore.push(start.elapsed().as_nanos());
        let start = Instant::now();
        arena.step(frame, &inputs).unwrap();
        step.push(start.elapsed().as_nanos());
        let start = Instant::now();
        replay.step(frame, &inputs).unwrap();
        let hash = black_box(replay.state_hash());
        verify.push(start.elapsed().as_nanos());
        assert_eq!(hash, arena.state_hash());
    }
    println!("frames=6000 max_serialized_checkpoint_bytes={max_bytes}");
    for (name, mut samples) in [
        ("step", step),
        ("checkpoint_json", save),
        ("restore_json", restore),
        ("replay_step_and_hash", verify),
    ] {
        samples.sort_unstable();
        println!(
            "{name}_us median={:.3} p99={:.3} max={:.3}",
            samples[2999] as f64 / 1000.,
            samples[5939] as f64 / 1000.,
            samples[5999] as f64 / 1000.
        );
    }
}

#[test]
fn malformed_checkpoint_rejection_preserves_destination_and_future() {
    let rules = sample_arena_config(ArenaMode::FreeForAll).unwrap();
    let mut original = Arena::with_seed(rules, 42).unwrap();
    original.step(1, &commands(1)).unwrap();
    let mut destination = Arena::from_checkpoint(&original.checkpoint()).unwrap();
    let bytes = serde_json::to_value(original.checkpoint()).unwrap();
    for (pointer, value) in [
        ("/version", serde_json::json!(999)),
        ("/world/timers", serde_json::json!([])),
        ("/world/fighters/0/fighter/facing", serde_json::json!(0)),
        ("/world/flights/0/owner", serde_json::json!(100)),
        ("/config/fighters/0/base/max_health", serde_json::json!(700)),
    ] {
        let mut corrupt = bytes.clone();
        *corrupt.pointer_mut(pointer).expect(pointer) = value;
        let snapshot: ArenaSnapshot = serde_json::from_value(corrupt).unwrap();
        assert!(destination.restore(&snapshot).is_err(), "{pointer}");
        assert_eq!(destination.state_hash(), original.state_hash());
    }
    destination.step(2, &[]).unwrap();
    original.step(2, &[]).unwrap();
    assert_eq!(destination.state_hash(), original.state_hash());
}

#[test]
fn released_cli_vectors_and_earliest_divergence() {
    use serde_json::Value;
    use std::{fs, path::Path, process::Command as Process};

    let test_dir = std::env::temp_dir().join(format!("ofe-p6-vector-{}", std::process::id()));
    fs::create_dir(&test_dir).unwrap();
    struct Scratch(std::path::PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let scratch = Scratch(test_dir);
    let tape = scratch.0.join("demo recording.ofr");
    let binary = env!("CARGO_BIN_EXE_ofe-sim");
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/arena.frames");
    let recorded = Process::new(binary)
        .args([
            "record",
            "--mode",
            "ffa",
            "--seed",
            "18446744073709551615",
            "--script",
        ])
        .arg(script)
        .arg("--out")
        .arg(&tape)
        .current_dir(&scratch.0)
        .output()
        .unwrap();
    assert!(recorded.status.success(), "{:?}", recorded.stderr);
    let mut disk: Value = serde_json::from_slice(&fs::read(&tape).unwrap()).unwrap();
    assert_eq!(disk["initial_hash"], "b0fff557443b6004");
    for (frame, hash) in [
        (1, "d6010b61913fcb1b"),
        (3, "0e0ab11308986203"),
        (9, "68926f47c617e146"),
        (22, "78b3a389c0be7054"),
    ] {
        assert_eq!(disk["frames"][frame - 1]["hash"], hash, "frame {frame}");
    }
    // Whitespace and object member order in the file are independent of the canonical hash.
    fs::write(&tape, serde_json::to_vec_pretty(&disk).unwrap()).unwrap();
    let verify = || {
        Process::new(binary)
            .arg("replay")
            .arg(&tape)
            .current_dir(&scratch.0)
            .output()
            .unwrap()
    };
    assert!(verify().status.success());
    // Remove the frame-three cancel, then introduce a later independent mismatch.
    disk["frames"][2]["commands"][0]["move_index"] = Value::Null;
    disk["frames"][6]["hash"] = Value::String("0000000000000000".into());
    fs::write(&tape, serde_json::to_vec(&disk).unwrap()).unwrap();
    let mismatch = verify();
    assert!(!mismatch.status.success());
    let message = String::from_utf8(mismatch.stderr).unwrap();
    assert!(
        message.contains("frame 3")
            && message.contains("expected 0e0ab11308986203")
            && message.contains("actual "),
        "{message}"
    );
}
