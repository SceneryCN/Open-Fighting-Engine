// SPDX-License-Identifier: MPL-2.0
//! Isolated allocation measurement for owned snapshots in a four-player projectile frame.
use ofe_combat_core::arena::{Arena, Command};
use ofe_sim::{ArenaMode, sample_arena_config};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, Ordering};

struct Meter;
static LIVE: AtomicIsize = AtomicIsize::new(0);

#[global_allocator]
static ALLOCATOR: Meter = Meter;

// SAFETY: all allocations use the original System allocator and layout.
unsafe impl GlobalAlloc for Meter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: caller supplies the valid layout required by GlobalAlloc.
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            LIVE.fetch_add(layout.size() as isize, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size() as isize, Ordering::Relaxed);
        // SAFETY: the caller returns the same valid allocation/layout pair.
        unsafe { System.dealloc(ptr, layout) };
    }
}

#[test]
#[ignore = "isolated memory measurement, run separately in release with --test-threads=1"]
fn four_players_snapshot_owned_bytes_return_to_baseline() {
    let mut arena = Arena::new(sample_arena_config(ArenaMode::FreeForAll).unwrap()).unwrap();
    let attack = Command {
        id: 1,
        move_index: Some(3),
        ..Default::default()
    };
    arena.step(1, &[attack]).unwrap();
    assert!(!arena.projectiles().is_empty());
    let mut bytes = 0;
    let baseline = LIVE.load(Ordering::Relaxed);
    for _ in 0..1000 {
        let snapshot = arena.checkpoint();
        let heap = LIVE.load(Ordering::Relaxed) - baseline;
        assert!(heap > 0);
        bytes = heap as usize + std::mem::size_of_val(&snapshot);
        std::hint::black_box(&snapshot);
        drop(snapshot);
        assert_eq!(LIVE.load(Ordering::Relaxed), baseline);
    }
    println!("four_player_projectile_checkpoint_owned_bytes={bytes} repeats=1000");
}
