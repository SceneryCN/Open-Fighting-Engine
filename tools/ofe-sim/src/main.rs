// SPDX-License-Identifier: MPL-2.0
//! Local two-player frame console.
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
            println!("{USAGE}");
            Ok(())
        }
        [flag, path] if flag == "--script" => {
            let file = File::open(path)
                .map_err(|e| format!("{}: {e}", std::path::Path::new(path).display()))?;
            run(BufReader::new(file))
        }
        _ => Err(USAGE.into()),
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
