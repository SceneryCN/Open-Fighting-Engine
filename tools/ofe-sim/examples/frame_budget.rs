// SPDX-License-Identifier: MPL-2.0
//! Release-mode stress and differential trace driver for the public Arena API.
use ofe_combat_core::arena::{Arena, ArenaConfig, ArenaEvent, Command, MoveKind};
use ofe_combat_core::duel::{DuelError, Event, Rect};
use ofe_sim::{ArenaMode, sample_arena_config};
use std::fmt::{self, Write};
use std::hint::black_box;
use std::time::Instant;

const HZ: u128 = 60;
const EMITTERS: usize = 4;
const DEFAULT_FRAMES: usize = 6_000;

#[derive(Clone, Copy)]
enum Load {
    Idle,
    Trade,
    Shots { lifetime: u32, burst: bool },
}

fn configured(load: Load) -> (ArenaConfig, Vec<Command>) {
    let mut config = sample_arena_config(ArenaMode::FreeForAll).unwrap();
    let mut commands = Vec::new();
    for (i, fighter) in config.fighters.iter_mut().enumerate() {
        fighter.base.max_health = i32::MAX;
        fighter.base.spawn.x = match load {
            Load::Shots { burst: false, .. } => [0, 128, 32000, 32128][i],
            _ => 0,
        };
        fighter.base.spawn.z = 0;
        fighter.base.spawn.h = 0;
        let normal = &mut fighter.moves[0];
        normal.base.startup = 0;
        normal.base.active = 1;
        normal.base.recovery = 0;
        normal.base.damage = 1;
        normal.base.pushback = 0;
        normal.base.hitstun = 0;
        normal.base.hitbox = Rect {
            x: -8,
            h: 0,
            width: 16,
            height: 8,
        };
        normal.hitstop = 0;
        normal.windows.clear();
        normal.cancels.clear();
        normal.armor = 0;
        normal.invulnerability = 0;
        if let Load::Shots { lifetime, burst } = load {
            normal.kind = MoveKind::Projectile { speed: 1, lifetime };
            if burst {
                normal.base.hitbox.x = -4096;
                normal.base.hitbox.width = 8192;
                normal.invulnerability = 1;
            }
        }
        fighter.base.attack = normal.base;
        if !matches!(load, Load::Idle) {
            commands.push(Command {
                id: fighter.id,
                move_index: Some(0),
                ..Default::default()
            });
        }
    }
    (config, commands)
}

#[derive(Clone, Copy)]
struct Digest(u64);

impl Digest {
    fn new() -> Self {
        Self(0xcbf29ce484222325)
    }
    fn absorb(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }
    fn observe(&mut self, arena: &Arena) {
        self.absorb(&arena.frame().to_le_bytes());
        // Debug records include all public fighter and event fields in order.
        write!(self, "{:?}|{:?}|", arena.fighters(), arena.events()).unwrap();
        self.absorb(&(arena.projectiles().len() as u64).to_le_bytes());
        for p in arena.projectiles() {
            self.absorb(&p.id.to_le_bytes());
            self.absorb(&p.owner.to_le_bytes());
            for axis in [p.position.x, p.position.z, p.position.h] {
                self.absorb(&axis.to_le_bytes());
            }
        }
    }
}
impl Write for Digest {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.absorb(s.as_bytes());
        Ok(())
    }
}

#[derive(Default)]
struct Stats {
    samples: Vec<u128>,
    hits: u64,
    events: u64,
    spawned: u64,
    max_live: usize,
    min_live: Option<usize>,
}
impl Stats {
    fn step(&mut self, arena: &mut Arena, commands: &[Command]) {
        let frame = arena.frame() + 1;
        let start = Instant::now();
        let result = arena.step(frame, black_box(commands));
        let elapsed = start.elapsed().as_nanos();
        result.unwrap();
        assert_eq!(arena.frame(), frame);
        self.samples.push(elapsed);
        let live = arena.projectiles().len();
        self.max_live = self.max_live.max(live);
        self.min_live = Some(self.min_live.map_or(live, |n| n.min(live)));
        self.events += arena.events().len() as u64;
        for e in arena.events() {
            match e {
                ArenaEvent::Base(Event::Hit { .. }) => self.hits += 1,
                ArenaEvent::ProjectileSpawned { .. } => self.spawned += 1,
                _ => {}
            }
        }
    }
    fn report(&mut self, name: &str) {
        self.samples.sort_unstable();
        let count = self.samples.len();
        let percentile = |percent: usize| self.samples[(count * percent).div_ceil(100) - 1];
        let misses = self
            .samples
            .iter()
            .filter(|&&ns| ns * HZ > 1_000_000_000)
            .count();
        println!(
            "{name},{count},{},{},{},{},{},{},{},{},{},{}",
            self.min_live.unwrap(),
            self.max_live,
            self.hits,
            self.events,
            self.spawned,
            percentile(50),
            percentile(95),
            percentile(99),
            self.samples[count - 1],
            misses
        );
    }
}
fn warm(arena: &mut Arena, commands: &[Command], n: usize) {
    for _ in 0..n {
        arena.step(arena.frame() + 1, commands).unwrap();
    }
}
fn sustain(name: &str, load: Load, frames: usize, trace: bool) {
    let (config, cmd) = configured(load);
    let mut arena = Arena::new(config).unwrap();
    let warmup = match load {
        Load::Shots { lifetime, .. } => lifetime as usize,
        _ => 256,
    };
    warm(&mut arena, &cmd, warmup);
    let mut stats = Stats::default();
    let mut digest = Digest::new();
    for _ in 0..frames {
        if trace {
            arena.step(arena.frame() + 1, &cmd).unwrap();
            digest.observe(&arena);
        } else {
            stats.step(&mut arena, &cmd);
        }
    }
    if trace {
        println!("trace {name} frames={frames} hash={:016x}", digest.0);
        return;
    }
    match load {
        Load::Idle => assert_eq!(stats.events, 0),
        Load::Trade => assert_eq!(stats.hits, frames as u64 * 12),
        Load::Shots { lifetime, .. } => {
            let live = lifetime as usize * EMITTERS;
            assert_eq!((stats.min_live, stats.max_live), (Some(live), live));
            assert_eq!(stats.spawned, frames as u64 * 4);
            assert!(stats.hits > 0);
        }
    }
    stats.report(name);
}

fn burst(frames: usize, trace: bool) {
    let lifetime = 1024;
    let (config, cmd) = configured(Load::Shots {
        lifetime,
        burst: true,
    });
    let mut ready = Arena::new(config).unwrap();
    warm(&mut ready, &cmd, lifetime as usize);
    assert_eq!(ready.projectiles().len(), 4096);
    let mut stats = Stats::default();
    let repeats = if trace { 1 } else { frames.min(200) };
    for _ in 0..repeats {
        // Every sample begins at the same immune frame. Snapshot setup is outside the timer.
        let mut arena = ready.clone();
        if trace {
            arena.step(arena.frame() + 1, &[]).unwrap();
            let mut digest = Digest::new();
            digest.observe(&arena);
            println!("trace burst frames=1 hash={:016x}", digest.0);
        } else {
            stats.step(&mut arena, &[]);
        }
    }
    if !trace {
        assert_eq!(stats.hits, repeats as u64 * 4092 * 3);
        stats.report("burst_4092x3");
    }
}

// Stage successful movement and attempted projectile emission before the last actor overflows.
// A valid retry must equal an independently initialized control at every subsequent tick.
fn rollback_trace() {
    let (mut cfg, cmd) = configured(Load::Shots {
        lifetime: 256,
        burst: false,
    });
    cfg.fighters[3].base.spawn.x = i32::MAX;
    let mut test = Arena::new(cfg.clone()).unwrap();
    let mut reference = Arena::new(cfg).unwrap();
    let mut rejected = cmd.clone();
    rejected[3].move_index = None;
    rejected[3].dash = true;
    rejected[3].controls.axis = 1;
    let mut original = Digest::new();
    original.observe(&test);
    assert_eq!(test.step(1, &rejected), Err(DuelError::Overflow));
    let mut rolled = Digest::new();
    rolled.observe(&test);
    assert_eq!(original.0, rolled.0);
    let mut transcript = Digest::new();
    for tick in 1..=64 {
        test.step(tick, &cmd).unwrap();
        reference.step(tick, &cmd).unwrap();
        assert_eq!(test.fighters(), reference.fighters());
        assert_eq!(test.projectiles(), reference.projectiles());
        assert_eq!(test.events(), reference.events());
        transcript.observe(&test);
    }
    println!("trace rollback_retry frames=64 hash={:016x}", transcript.0);
}

fn main() {
    let mut frames = DEFAULT_FRAMES;
    let mut trace = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--frames" => {
                frames = args
                    .next()
                    .expect("missing frame count")
                    .parse()
                    .expect("positive integer");
                assert!(frames > 0);
            }
            "--trace" => trace = true,
            _ => panic!("usage: frame_budget [--frames N] [--trace]"),
        }
    }
    if trace {
        rollback_trace();
    } else {
        println!(
            "workload,frames,min_live,max_live,hits,events,spawned,p50_ns,p95_ns,p99_ns,max_ns,missed_60hz"
        );
    }
    for (name, load) in [
        ("idle", Load::Idle),
        ("twelve_contacts", Load::Trade),
        (
            "live_1024",
            Load::Shots {
                lifetime: 256,
                burst: false,
            },
        ),
        (
            "live_4096",
            Load::Shots {
                lifetime: 1024,
                burst: false,
            },
        ),
    ] {
        sustain(name, load, frames, trace);
    }
    burst(frames, trace);
}
