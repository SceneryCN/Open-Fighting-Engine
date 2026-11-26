// SPDX-License-Identifier: MPL-2.0
//! Consumer-side tests for the versioned P3 runtime contract.
use ofe_animation_runtime::{Clip, Player};
use ofe_character_runtime::Character;
use ofe_resource_runtime::ResourceStore;
use serde_json::{Value, json};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

const GRID: &[u8] = include_bytes!("../fixtures/grid.png");
static ORDINAL: AtomicUsize = AtomicUsize::new(0);
struct Package(PathBuf);
impl Package {
    fn new() -> Self {
        loop {
            let path = std::env::temp_dir().join(format!(
                "ofe-consumer-{}-{}",
                std::process::id(),
                ORDINAL.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("could not create fixture: {e}"),
            }
        }
    }
    fn bytes(&self, name: &str, bytes: &[u8]) {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    fn json(&self, name: &str, data: Value) {
        self.bytes(name, &serde_json::to_vec(&data).unwrap());
    }
}
impl Drop for Package {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn with_clip(kind: &str, looping: bool) -> Package {
    let pkg = Package::new();
    pkg.bytes("clips/grid.png", GRID);
    let sheet = kind == "sheet";
    let source = if sheet {
        json!({"kind":"sheet","image":"grid.png","columns":3,"rows":2})
    } else {
        json!({"kind":"sequence"})
    };
    let mut frames = json!([
        {"duration":2,"origin":{"x":-3,"y":1},"events":["begin","accent"]},
        {"duration":1,"origin":{"x":4,"y":-1},"events":[]},
        {"duration":3,"origin":{"x":5,"y":7},"events":["land"]},
    ]);
    for (i, frame) in frames.as_array_mut().unwrap().iter_mut().enumerate() {
        if sheet {
            frame["index"] = json!([4, 2, 3][i]);
        } else {
            frame["image"] = json!("grid.png");
        }
    }
    pkg.json("clips/motion.json",json!({"format":"OFA","format_version":"1.0","id":"motion","loop":looping,"source":source,"frames":frames}));
    pkg
}

#[test]
fn cache_ownership_clear_and_disk_reload() {
    let pkg = Package::new();
    pkg.bytes("a/owner.json", b"{}");
    pkg.bytes("a/other.json", b"{}");
    pkg.bytes("a/common.bin", b"original");
    let mut store = ResourceStore::open(&pkg.0).unwrap();
    let first = store.read("a/owner.json", "common.bin").unwrap();
    assert_eq!(store.cached_count(), 1);
    pkg.bytes("a/common.bin", b"replacement");
    let second = store.read("a/other.json", "common.bin").unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(&*second, b"original");
    store.clear();
    assert_eq!(store.cached_count(), 0);
    let replacement = store.read("a/owner.json", "common.bin").unwrap();
    assert!(!Arc::ptr_eq(&first, &replacement));
    drop(store);
    assert_eq!(&*first, b"original");
    assert_eq!(&*replacement, b"replacement");
}

#[test]
fn relative_paths_stay_below_referencing_document() {
    let pkg = Package::new();
    pkg.bytes("clips/owner.json", b"{}");
    pkg.bytes("sibling/exists.bin", b"neighbor");
    let mut store = ResourceStore::open(&pkg.0).unwrap();
    for reference in [
        "../sibling/exists.bin",
        "unknown.bin",
        "/absolute",
        "C:/outside",
        "a\\b",
    ] {
        assert!(
            store.read("clips/owner.json", reference).is_err(),
            "{reference}"
        );
    }
    assert_eq!(store.cached_count(), 0);
}

#[test]
fn sequence_and_sheet_display_exact_frames_and_enter_events() {
    let frames = [0, 0, 1, 2, 2, 2];
    let events: &[&[&str]] = &[&["begin", "accent"], &[], &[], &["land"], &[], &[]];
    for kind in ["sheet", "sequence"] {
        for looping in [true, false] {
            let pkg = with_clip(kind, looping);
            let mut store = ResourceStore::open(&pkg.0).unwrap();
            let clip = Clip::load(&mut store, "clips/motion.json").unwrap();
            let first = &clip.frames()[0];
            assert_eq!((first.origin.x, first.origin.y), (-3, 1));
            assert_eq!(first.image_path, "grid.png");
            assert!(Arc::ptr_eq(&first.image, &clip.frames()[2].image));
            let bounds: Vec<_> = clip
                .frames()
                .iter()
                .map(|f| {
                    let r = f.source_rect;
                    (r.x, r.y, r.width, r.height)
                })
                .collect();
            if kind == "sheet" {
                assert_eq!(bounds, [(2, 2, 2, 2), (4, 0, 2, 2), (0, 2, 2, 2)]);
            } else {
                assert_eq!(bounds, [(0, 0, 6, 4); 3]);
            }
            let mut player = Player::new(&clip);
            assert_eq!(player.frame_index(), 0);
            assert!(!player.finished());
            for i in 0..18 {
                let expected_idx = if looping || i < 6 { frames[i % 6] } else { 2 };
                let expected_events = if looping || i < 6 { events[i % 6] } else { &[] };
                assert_eq!(player.tick(), expected_events);
                assert_eq!(player.frame_index(), expected_idx);
                assert_eq!(player.finished(), !looping && i >= 5);
            }
        }
    }
}

#[test]
fn clip_rejects_invalid_sheet_dimensions_bounds_and_duration() {
    for (which, value) in [("columns", 4), ("index", 6), ("duration", 0)] {
        let pkg = with_clip("sheet", false);
        let path = pkg.0.join("clips/motion.json");
        let mut doc: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        if which == "columns" {
            doc["source"]["columns"] = json!(value);
        } else {
            doc["frames"][0][which] = json!(value);
        }
        pkg.json("clips/motion.json", doc);
        let mut store = ResourceStore::open(&pkg.0).unwrap();
        assert!(
            Clip::load(&mut store, "clips/motion.json").is_err(),
            "{which}"
        );
    }
}

fn authored() -> Package {
    let pkg = with_clip("sheet", true);
    let original = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/ofe-sim/fixtures/amber");
    for rel in [
        "character.json",
        "attributes.json",
        "hitboxes/body.json",
        "licenses/notice.txt",
        "moves/tap.json",
    ] {
        pkg.bytes(rel, &fs::read(original.join(rel)).unwrap());
    }
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(original.join("manifest.json")).unwrap()).unwrap();
    manifest["id"] = json!("qa_order");
    manifest["animations"] = json!(["clips/motion.json"]);
    manifest["assets"] = json!(["clips/grid.png"]);
    manifest["moves"] = json!(["moves/second.json", "moves/tap.json"]);
    let mut second: Value =
        serde_json::from_slice(&fs::read(original.join("moves/tap.json")).unwrap()).unwrap();
    second["id"] = json!("second");
    pkg.json("moves/second.json", second);
    pkg.json("manifest.json", manifest);
    pkg
}

#[test]
fn loaded_character_owns_data_and_preserves_declared_move_order() {
    let pkg = authored();
    let first = Character::load(&pkg.0).unwrap();
    pkg.json("attributes.json", json!({"max_health":99,"max_energy":17}));
    let changed = Character::load(&pkg.0).unwrap();
    assert_eq!(first.id(), "qa_order");
    assert_eq!(first.max_health(), 60);
    assert_eq!(changed.max_health(), 99);
    assert_eq!(changed.max_energy(), 17);
    assert_eq!(first.move_ids().collect::<Vec<_>>(), ["second", "tap"]);
    assert_eq!(first.animations().len(), 1);
    fs::remove_file(pkg.0.join("clips/grid.png")).unwrap();
    assert!(Character::load(&pkg.0).is_err());
    drop(pkg);
    assert_eq!(
        Player::new(&first.animations()[0]).tick(),
        ["begin", "accent"]
    );
}
