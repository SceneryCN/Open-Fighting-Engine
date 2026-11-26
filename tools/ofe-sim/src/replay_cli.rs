// SPDX-License-Identifier: MPL-2.0
use crate::{arena_console, open_script};
use ofe_combat_core::arena::Arena;
use ofe_replay_runtime::{MAX_BYTES, Replay};
use ofe_sim::{ArenaMode, sample_arena_config, sample_duel_arena_config};
use std::{
    ffi::OsString,
    fs::{self, File},
    io::{BufReader, Read, Write},
    path::Path,
};

pub(crate) const USAGE: &str = "ofe-sim record --mode duel|2v2|ffa --seed U64 --script INPUT --out RECORDING\nofe-sim replay RECORDING";

pub(crate) fn record(args: &[OsString]) -> Result<(), String> {
    let mut options = std::collections::BTreeMap::new();
    let (pairs, remainder) = args.as_chunks::<2>();
    for pair in pairs {
        let Some(key @ ("--mode" | "--seed" | "--script" | "--out")) = pair[0].to_str() else {
            return Err(USAGE.into());
        };
        if options.insert(key, &pair[1]).is_some() {
            return Err(format!("duplicate argument {key}\n{USAGE}"));
        }
    }
    if !remainder.is_empty() || options.len() != 4 {
        return Err(USAGE.into());
    }
    let seed = options["--seed"]
        .to_str()
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or("seed must be an unsigned 64-bit integer")?;
    let config = match options["--mode"].to_str() {
        Some("duel") => sample_duel_arena_config()?,
        Some("2v2") => sample_arena_config(ArenaMode::Teams)?,
        Some("ffa") => sample_arena_config(ArenaMode::FreeForAll)?,
        _ => return Err(USAGE.into()),
    };
    let script = Path::new(options["--script"]);
    let output = Path::new(options["--out"]);
    let input = open_script(script.as_os_str())?;
    if output
        .canonicalize()
        .is_ok_and(|p| Some(p) == script.canonicalize().ok())
    {
        return Err("output must be separate from the input script".into());
    }
    let mut arena = Arena::with_seed(config, seed).map_err(|e| format!("invalid arena: {e:?}"))?;
    let mut tape = Replay::new(&arena);
    arena_console::advance(BufReader::new(input), &mut arena, |after, commands| {
        tape.record(after, commands).map_err(|e| e.to_string())
    })?;
    let bytes = tape.to_bytes().map_err(|e| e.to_string())?;
    replace_file(output, &bytes).map_err(|e| format!("{}: {e}", output.display()))?;
    println!(
        "recorded {} frames to {} (final hash {:016x})",
        tape.len(),
        output.display(),
        arena.state_hash()
    );
    Ok(())
}

/// Rename an exclusively created sibling after all data has reached disk.
fn replace_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or(Path::new("."));
    for attempt in 0..100 {
        let temp = parent.join(format!(
            ".ofe-recording-{}-{attempt}.tmp",
            std::process::id()
        ));
        let mut file = match File::create_new(&temp) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        };
        let result = file.write_all(bytes).and_then(|()| file.sync_all());
        drop(file);
        let result = result.and_then(|()| fs::rename(&temp, path));
        if result.is_err() {
            let _ = fs::remove_file(temp);
        }
        return result;
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "cannot allocate a temporary output file",
    ))
}

pub(crate) fn verify(path: &Path) -> Result<(), String> {
    let file = open_script(path.as_os_str())?;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let tape = Replay::from_bytes(&bytes).map_err(|e| e.to_string())?;
    let arena = tape.verify().map_err(|e| e.to_string())?;
    println!(
        "verified {} frames, final frame {}, hash {:016x}",
        tape.len(),
        arena.frame(),
        arena.state_hash()
    );
    Ok(())
}
