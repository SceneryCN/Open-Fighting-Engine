// SPDX-License-Identifier: MPL-2.0
use ofe_combat_core::arena::{Arena, Command};
use ofe_replay_runtime::{Replay, ReplayError};
use serde_json::{Value, json};

#[test]
fn sparse_recording_can_start_inside_an_attack() {
    let mut original =
        Arena::with_seed(ofe_sim::sample_duel_arena_config().unwrap(), u64::MAX).unwrap();
    let open = Command {
        id: 1,
        move_index: Some(0),
        ..Default::default()
    };
    original.step(1, &[open]).unwrap();
    let mut tape = Replay::new(&original);
    for frame in 2..=20 {
        original.step(frame, &[]).unwrap();
        tape.record(&original, &[]).unwrap();
    }
    assert!(tape.record(&original, &[]).is_err());
    let bytes = tape.to_bytes().unwrap();
    let decoded = Replay::from_bytes(&bytes).unwrap();
    assert_eq!(decoded.len(), 19);
    assert_eq!(decoded.initial_arena().unwrap().frame(), 1);
    assert_eq!(decoded.commands(0), Some([].as_slice()));
    assert_eq!(
        decoded.verify().unwrap().state_hash(),
        original.state_hash()
    );

    let mut changed: Value = serde_json::from_slice(&bytes).unwrap();
    changed["frames"][2]["hash"] = json!("0000000000000000");
    let decoded = Replay::from_bytes(&serde_json::to_vec(&changed).unwrap()).unwrap();
    assert!(matches!(
        decoded.verify(),
        Err(ReplayError::Mismatch {
            frame: 4,
            expected: 0,
            ..
        })
    ));
}

#[test]
fn unsupported_version_malformed_hash_and_discontinuous_frames_fail_to_load() {
    let mut arena = Arena::new(ofe_sim::sample_duel_arena_config().unwrap()).unwrap();
    let mut tape = Replay::new(&arena);
    arena.step(1, &[]).unwrap();
    tape.record(&arena, &[]).unwrap();
    let bytes = tape.to_bytes().unwrap();
    for (pointer, value) in [
        ("/version", json!(2)),
        ("/initial_hash", json!("0")),
        ("/frames/0/hash", json!("FEDCBA9876543210")),
        ("/frames/0/frame", json!(2)),
    ] {
        let mut doc: Value = serde_json::from_slice(&bytes).unwrap();
        *doc.pointer_mut(pointer).unwrap() = value;
        assert!(
            Replay::from_bytes(&serde_json::to_vec(&doc).unwrap()).is_err(),
            "{pointer}"
        );
    }
    assert!(Replay::from_bytes(&bytes[..bytes.len() - 1]).is_err());
}
