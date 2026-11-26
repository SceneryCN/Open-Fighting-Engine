// SPDX-License-Identifier: MPL-2.0
use std::{
    io::Write,
    process::{Command, Output, Stdio},
};

fn console(mode: &str, input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ofe-sim"))
        .args(["--mode", mode])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn both_modes_run_the_same_script_from_stdin_or_file() {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/arena.frames");
    for mode in ["2v2", "ffa"] {
        let scripted = Command::new(env!("CARGO_BIN_EXE_ofe-sim"))
            .args(["--mode", mode, "--script"])
            .arg(&script)
            .current_dir(std::env::temp_dir())
            .output()
            .unwrap();
        assert!(
            scripted.status.success(),
            "{}",
            String::from_utf8_lossy(&scripted.stderr)
        );
        let interactive = console(mode, include_bytes!("../fixtures/arena.frames"));
        assert!(interactive.status.success());
        assert_eq!(interactive.stdout, scripted.stdout);
        let text = String::from_utf8(scripted.stdout).unwrap();
        assert!(text.contains("p1 team=1 hp=150 energy=40"));
        assert!(text.contains(if mode == "2v2" {
            "p4 team=2"
        } else {
            "p4 team=4"
        }));
        assert!(text.contains("Cancelled { id: 1, from: 1, into: 4 }"));
        assert!(text.contains("ProjectileSpawned"));
        assert!(text.contains("Dashed"));
    }
}

#[test]
fn bad_arity_or_ambiguous_command_fails_at_the_correct_player_and_line() {
    for (line, expected) in [
        ("- - -", "P1 P2 P3 P4"),
        ("- 12 - -", "P2"),
        ("- - D- -", "P3"),
        ("- - - A4", "P4"),
    ] {
        let run = console("ffa", format!("# test\n{line}\n").as_bytes());
        assert!(!run.status.success());
        let error = String::from_utf8(run.stderr).unwrap();
        assert!(
            error.contains("line 2") && error.contains(expected),
            "{error}"
        );
        assert!(!String::from_utf8(run.stdout).unwrap().contains("frame=1\n"));
    }
}
