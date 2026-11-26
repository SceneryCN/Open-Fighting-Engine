// SPDX-License-Identifier: MPL-2.0
use std::{
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};

fn from_stdin(input: &[u8]) -> Output {
    let mut process = Command::new(env!("CARGO_BIN_EXE_ofe-sim"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    process.stdin.take().unwrap().write_all(input).unwrap();
    process.wait_with_output().unwrap()
}

#[test]
fn frame_script_matches_console_and_exercises_resolved_combat() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let script = root.join("fixtures/duel.frames");
    let run = Command::new(env!("CARGO_BIN_EXE_ofe-sim"))
        .current_dir(std::env::temp_dir())
        .arg("--script")
        .arg(script)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdin = from_stdin(include_bytes!("../fixtures/duel.frames"));
    assert!(stdin.status.success());
    assert_eq!(run.stdout, stdin.stdout);
    let text = String::from_utf8(run.stdout).unwrap();
    assert!(text.lines().eq(include_str!("duel.golden").lines()));
    assert!(text.contains("frame=0 p1 hp=60"));
    assert!(text.contains("p2 hp=75"));
    assert!(text.contains("frame=2 p1 hp=48"));
    assert!(text.contains("p2 hp=60"));
    assert!(text.contains("Blocked { attacker: 1, target: 2 }"));
    assert!(text.contains("state=LaneSwitch"));
    assert!(text.contains("state=Jump"));
    assert!(text.contains("z=1000"));
    let last = text
        .lines()
        .find(|line| line.starts_with("frame=26 "))
        .unwrap();
    assert!(last.contains("p1 hp=36") && last.contains("p2 hp=60"));
}

#[test]
fn mixed_controls_and_comments_consume_one_frame() {
    let run = from_stdin(b"# ready\n\nRJ L # frame 1\n");
    assert!(run.status.success());
    let text = String::from_utf8(run.stdout).unwrap();
    assert_eq!(
        text.lines()
            .filter(|line| line.starts_with("frame="))
            .count(),
        2
    );
    assert!(text.contains("state=Jump"));
}

#[test]
fn reports_bad_input_location_before_stepping() {
    for (line, expected) in [
        ("LR G", "P1"),
        ("A GG", "P2"),
        ("? G", "P1"),
        ("A", "expected P1 and P2"),
        ("A G S", "exactly two players"),
    ] {
        let run = from_stdin(format!("# comment\n{line}\n").as_bytes());
        assert!(!run.status.success());
        let error = String::from_utf8(run.stderr).unwrap();
        assert!(error.contains("line 2"));
        assert!(error.contains(expected));
        assert!(!String::from_utf8(run.stdout).unwrap().contains("frame=1"));
    }
}

#[test]
fn absent_script_and_unknown_options_fail() {
    for arguments in [
        vec!["--script", "no-such-file/duel.frames"],
        vec!["--unknown"],
    ] {
        let run = Command::new(env!("CARGO_BIN_EXE_ofe-sim"))
            .args(arguments)
            .output()
            .unwrap();
        assert!(!run.status.success());
        assert!(!run.stderr.is_empty());
    }
}
