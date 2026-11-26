// SPDX-License-Identifier: MPL-2.0
use ofe_combat_core::arena::{Arena, Command};
use ofe_sim::{ArenaMode, sample_arena_config};
use std::io::BufRead;

pub(crate) const USAGE: &str = "ofe-sim --mode 2v2|ffa [--script FILE]\nEach frame: P1 P2 P3 P4. - neutral, L/R walk, J jump, S switch lane, G guard, D dash. A or 0 jab, 1 flurry, 2 throw, 3 projectile, 4 super, 5 armor, 6 invulnerable strike. Example: 1 G - -";

fn parse(id: u32, token: &str) -> Result<Command, String> {
    let mut dash = false;
    let mut move_index = None;
    let mut base = String::new();
    for c in token.chars() {
        match c {
            'D' if !dash => dash = true,
            '0'..='6' if move_index.is_none() => move_index = Some(usize::from(c as u8 - b'0')),
            _ => base.push(c),
        }
    }
    if token != "-" && token.contains('-') {
        return Err(format!("neutral must stand alone: {token}"));
    }
    let controls = super::controls(if base.is_empty() { "-" } else { &base })?;
    if controls.attack && move_index.is_some() {
        return Err(format!("choose a single move: {token}"));
    }
    Ok(Command {
        id,
        controls,
        dash,
        move_index,
    })
}

fn print_arena(arena: &Arena) {
    println!("frame={}", arena.frame());
    for slot in arena.fighters() {
        let f = &slot.fighter;
        println!(
            "  p{} team={} hp={} energy={} state={:?} pos=({},{},{}) facing={} move={:?} combo={} hitstop={} dash={}",
            f.id,
            slot.team,
            f.health,
            slot.energy,
            f.state,
            f.position.x,
            f.position.z,
            f.position.h,
            f.facing,
            slot.move_index,
            slot.combo,
            slot.hitstop,
            slot.dashing
        );
    }
    for p in arena.projectiles() {
        println!(
            "  projectile={} owner={} pos=({},{},{})",
            p.id, p.owner, p.position.x, p.position.z, p.position.h
        );
    }
    for event in arena.events() {
        println!("  {event:?}");
    }
}

pub(crate) fn run(input: impl BufRead, mode: ArenaMode) -> Result<(), String> {
    let mut arena =
        Arena::new(sample_arena_config(mode)?).map_err(|e| format!("initialization: {e:?}"))?;
    print_arena(&arena);
    for (i, line) in input.lines().enumerate() {
        let n = i + 1;
        let line = line.map_err(|e| format!("line {n}: {e}"))?;
        let words: Vec<_> = line
            .split('#')
            .next()
            .unwrap_or_default()
            .split_whitespace()
            .collect();
        if words.is_empty() {
            continue;
        }
        if words.len() != arena.fighters().len() {
            return Err(format!("line {n}: expected P1 P2 P3 P4"));
        }
        let commands = arena
            .fighters()
            .iter()
            .zip(words)
            .map(|(slot, word)| {
                let id = slot.fighter.id;
                parse(id, word).map_err(|e| format!("line {n} P{id}: {e}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let frame = arena.frame().checked_add(1).ok_or("frame overflow")?;
        arena
            .step(frame, &commands)
            .map_err(|e| format!("frame {frame}: {e:?}"))?;
        print_arena(&arena);
    }
    Ok(())
}
