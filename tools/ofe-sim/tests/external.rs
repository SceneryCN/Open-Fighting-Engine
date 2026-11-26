// SPDX-License-Identifier: MPL-2.0
use ofe_character_runtime::Character;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU32, Ordering},
};

struct Copy(PathBuf);
impl Copy {
    fn of(name: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let dst = std::env::temp_dir().join(format!(
            "ofe-char-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fn tree(from: &Path, to: &Path) {
            fs::create_dir(to).unwrap();
            for entry in fs::read_dir(from).unwrap() {
                let entry = entry.unwrap();
                let dest = to.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    tree(&entry.path(), &dest);
                } else {
                    fs::copy(entry.path(), dest).unwrap();
                }
            }
        }
        tree(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures")
                .join(name),
            &dst,
        );
        Self(dst)
    }
    fn edit(&self, path: &str, edit: impl FnOnce(&mut Value)) {
        let path = self.0.join(path);
        let mut value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        edit(&mut value);
        fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
    }
}
impl Drop for Copy {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn external_changes_reach_real_arena_without_recompilation() {
    let dir = Copy::of("amber");
    dir.edit("attributes.json", |d| d["max_health"] = json!(88));
    dir.edit("moves/tap.json", |d| d["damage"] = json!(23));
    let script = dir.0.join("frames.txt");
    fs::write(&script, "A -\n- -\n").unwrap();
    let run = Command::new(env!("CARGO_BIN_EXE_ofe-sim"))
        .arg("--character")
        .arg(&dir.0)
        .arg("--script")
        .arg(script)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let log = String::from_utf8(run.stdout).unwrap();
    assert!(
        log.contains("p2 team=2 hp=65") && log.contains("damage: 23"),
        "{log}"
    );
}

#[test]
fn invalid_profiles_fail_before_simulation() {
    let dir = Copy::of("arena");
    let profile = dir.0.join("move_profile.json");
    let original = fs::read(&profile).unwrap();
    for case in 0..6 {
        fs::write(&profile, &original).unwrap();
        dir.edit("move_profile.json", |p| match case {
            0 => {
                p["moves"].as_array_mut().unwrap().pop();
            }
            1 => p["moves"][1]["id"] = json!("unexpected"),
            2 => p["moves"][1]["id"] = json!("jab"),
            3 => p["moves"][1]["cancel_into"] = json!(["absent"]),
            4 => p["moves"][1]["windows"][1]["start"] = json!(0),
            _ => p["moves"][3]["speed"] = json!(0),
        });
        assert!(Character::load(&dir.0).is_err(), "case {case}");
    }
    fs::remove_file(profile).unwrap();
    assert!(
        Character::load(&dir.0).is_err(),
        "OFC cancel windows require a profile"
    );
}

#[test]
fn public_loader_checks_metadata_geometry_and_declared_resources() {
    let dir = Copy::of("arena");
    let c = Character::load(&dir.0).unwrap();
    assert_eq!(c.id, "arena_probe");
    assert_eq!(c.combat.moves[2].id(), "grab");
    assert_eq!(c.animations[0].id(), "idle");
    dir.edit("hitboxes/body.json", |d| d["boxes"][0]["width"] = json!(0));
    assert!(Character::load(&dir.0).is_err());

    let dir = Copy::of("amber");
    fs::remove_file(dir.0.join("licenses/notice.txt")).unwrap();
    assert!(Character::load(&dir.0).is_err());
}
