// SPDX-License-Identifier: MPL-2.0
//! Golden vectors derived from independent little-endian FNV-1a encoding.
use ofe_combat_core::Battle;

#[test]
fn snapshot_hash_matches_known_vectors() {
    let mut battle = Battle::new();
    assert_eq!(battle.snapshot().state_hash(), 0xbfb6_7b99_8e87_3d8f);
    battle.submit(1, -1).unwrap();
    assert_eq!(battle.snapshot().state_hash(), 0xbff3_f27c_bfcf_2729);
    battle.step().unwrap();
    assert_eq!(battle.snapshot().state_hash(), 0x9ec8_03c0_a923_b750);
}
