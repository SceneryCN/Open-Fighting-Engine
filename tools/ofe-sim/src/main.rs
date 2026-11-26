// SPDX-License-Identifier: MPL-2.0
//! Local duel and four-player frame consoles.
mod arena_console;
use ofe_combat_core::duel::{Controls, Duel};
use std::{
    env,
    fs::File,
    io::{self, BufRead, BufReader, IsTerminal},
    process::ExitCode,
};

const USAGE: &str = "ofe-sim [--script FILE]\nEnter one frame per line: P1 P2. Controls: - neutral, L left, R right, J jump, S switch lane, A attack, G guard. Combine letters, e.g. RJ. EOF exits.";

fn controls(token: &str) -> Result<Controls, String> {
    let mut output = Controls::default();
    if token == "-" {
        return Ok(output);
    }
    for c in token.chars() {
        match c {
            'L' | 'R' if output.axis == 0 => output.axis = if c == 'L' { -1 } else { 1 },
            'J' if !output.jump => output.jump = true,
            'S' if !output.switch_lane => output.switch_lane = true,
            'A' if !output.attack => output.attack = true,
            'G' if !output.guard => output.guard = true,
            _ => return Err(format!("invalid or repeated control {c:?} in {token:?}")),
        }
    }
    Ok(output)
}

fn print_frame(duel: &Duel) {
    print!("frame={}", duel.frame());
    for f in duel.fighters() {
        print!(
            " p{} hp={} state={:?} x={} z={} h={} facing={}",
            f.id, f.health, f.state, f.position.x, f.position.z, f.position.h, f.facing
        );
    }
    println!();
    for event in duel.events() {
        println!("  {event:?}");
    }
}

fn run(input: impl BufRead) -> Result<(), String> {
    let mut duel = Duel::new(ofe_sim::sample_config()?)
        .map_err(|error| format!("initialization: {error:?}"))?;
    print_frame(&duel);
    for (index, line) in input.lines().enumerate() {
        let line = line.map_err(|e| format!("line {}: {e}", index + 1))?;
        let mut words = line
            .split('#')
            .next()
            .unwrap_or_default()
            .split_whitespace();
        let Some(first) = words.next() else { continue };
        let Some(second) = words.next() else {
            return Err(format!("line {}: expected P1 and P2", index + 1));
        };
        if words.next().is_some() {
            return Err(format!("line {}: expected exactly two players", index + 1));
        }
        let parse =
            |word, player| controls(word).map_err(|e| format!("line {} P{player}: {e}", index + 1));
        let frame = duel.frame().checked_add(1).ok_or("frame overflow")?;
        duel.step(frame, [parse(first, 1)?, parse(second, 2)?])
            .map_err(|error| format!("frame {frame}: {error:?}"))?;
        print_frame(&duel);
    }
    Ok(())
}

fn open_script(path: &std::ffi::OsStr) -> Result<File, String> {
    File::open(path).map_err(|e| format!("{}: {e}", std::path::Path::new(path).display()))
}

fn entry() -> Result<(), String> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    match args.as_slice() {
        [] => {
            if io::stdin().is_terminal() {
                println!("{USAGE}");
            }
            run(io::stdin().lock())
        }
        [help] if help == "--help" || help == "-h" => {
            println!(
                "{USAGE}\n{}\nofe-sim --character DIR [--script FILE]; @N selects a manifest move by index.",
                arena_console::USAGE
            );
            Ok(())
        }
        [flag, path] if flag == "--script" => {
            let file = open_script(path)?;
            run(BufReader::new(file))
        }
        [flag, root, rest @ ..] if flag == "--character" => {
            let cfg = ofe_sim::external_arena_config(std::path::Path::new(root))?;
            match rest {
                [] => arena_console::run_config(io::stdin().lock(), cfg),
                [flag, path] if flag == "--script" => {
                    arena_console::run_config(BufReader::new(open_script(path)?), cfg)
                }
                _ => Err("usage: ofe-sim --character DIR [--script FILE]".into()),
            }
        }
        [flag, mode, rest @ ..] if flag == "--mode" => {
            let mode = match mode.to_str() {
                Some("2v2") => ofe_sim::ArenaMode::Teams,
                Some("ffa") => ofe_sim::ArenaMode::FreeForAll,
                _ => return Err(arena_console::USAGE.into()),
            };
            match rest {
                [] => {
                    if io::stdin().is_terminal() {
                        println!("{}", arena_console::USAGE);
                    }
                    arena_console::run(io::stdin().lock(), mode)
                }
                [flag, path] if flag == "--script" => {
                    let file = open_script(path)?;
                    arena_console::run(BufReader::new(file), mode)
                }
                _ => Err(arena_console::USAGE.into()),
            }
        }
        _ => Err(format!("{USAGE}\n{}", arena_console::USAGE)),
    }
}

fn main() -> ExitCode {
    match entry() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
