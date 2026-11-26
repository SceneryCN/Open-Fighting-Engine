use ofe_combat_core::{Battle, Error, Input, Snapshot};

#[test]
fn replay_from_queued_snapshot_matches_each_tick() {
    let mut live = Battle::new();
    let initial_hash = live.snapshot().state_hash();
    live.submit(1, 1).unwrap();
    assert_ne!(live.snapshot().state_hash(), initial_hash);
    let mut restored = Battle::restore(live.snapshot()).unwrap();
    for frame in 1..=600 {
        for battle in [&mut live, &mut restored] {
            if frame > 1 {
                battle.submit(frame, (frame % 3) as i32 - 1).unwrap();
            }
            battle.step().unwrap();
        }
        assert_eq!(
            live.snapshot().state_hash(),
            restored.snapshot().state_hash(),
            "frame {frame}"
        );
    }
}

#[test]
fn restore_rejects_inconsistent_external_state() {
    let start = Battle::new().snapshot();
    for (state, error) in [
        (
            Snapshot {
                entity_id: 0,
                ..start
            },
            Error::InvalidEntity,
        ),
        (
            Snapshot {
                pending: Some(Input { frame: 5, axis: 1 }),
                ..start
            },
            Error::WrongFrame,
        ),
        (
            Snapshot {
                pending: Some(Input { frame: 1, axis: -3 }),
                ..start
            },
            Error::InvalidAxis,
        ),
    ] {
        assert_eq!(Battle::restore(state).err(), Some(error));
    }
}
