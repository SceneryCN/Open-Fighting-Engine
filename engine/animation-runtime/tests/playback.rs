// SPDX-License-Identifier: MPL-2.0
use ofe_animation_runtime::{Clip, Origin, Player, SourceRect};
use ofe_resource_runtime::ResourceStore;
use std::fs;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

const PNG: &[u8] = include_bytes!("data/sheet.png");
const SEQUENCE: &str = r#"{
"format":"OFA","format_version":"1.0","id":"walk","loop":false,
"source":{"kind":"sequence"},
"frames":[
{"duration":20e-1,"origin":{"x":-1.000,"y":4e0},"events":["foot","sound"],"image":"sheet.png"},
{"duration":3,"origin":{"x":0,"y":0},"events":["end"],"image":"sheet.png"}
]}"#;
const SHEET: &str = r#"{
"format":"OFA","format_version":"1.0","id":"sheet","loop":true,
"source":{"kind":"sheet","image":"sheet.png","columns":3.0,"rows":2e0},
"frames":[{"duration":2,"origin":{"x":1,"y":-2},"events":["one"],"index":4e0},
{"duration":1,"origin":{"x":3,"y":2},"events":["two"],"index":5}]
}"#;

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Package(PathBuf);
impl Package {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "ofe-animation-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::create_dir(path.join("anim")).unwrap();
        fs::write(path.join("anim/sheet.png"), PNG).unwrap();
        Self(path)
    }
    fn load(&self, data: &str) -> Result<Clip, String> {
        fs::write(self.0.join("anim/clip.json"), data).unwrap();
        let mut store = ResourceStore::open(&self.0).unwrap();
        Clip::load(&mut store, "anim/clip.json")
    }
}
impl Drop for Package {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn sequence_durations_and_events_match_the_displayed_frame() {
    let clip = Package::new().load(SEQUENCE).unwrap();
    assert_eq!(clip.id(), "walk");
    assert!(!clip.is_looping());
    let first = &clip.frames()[0];
    assert_eq!(first.origin, Origin { x: -1, y: 4 });
    assert_eq!(
        first.source_rect,
        SourceRect {
            x: 0,
            y: 0,
            width: 6,
            height: 4
        }
    );
    assert_eq!(first.image_path, "sheet.png");
    assert!(Arc::ptr_eq(&first.image, &clip.frames()[1].image));
    let mut p = Player::new(&clip);
    assert_eq!(p.frame_index(), 0);
    assert!(!p.finished());
    for (index, events, finished) in [
        (0, &["foot", "sound"][..], false),
        (0, &[][..], false),
        (1, &["end"][..], false),
        (1, &[][..], false),
        (1, &[][..], true),
        (1, &[][..], true),
    ] {
        assert_eq!(p.tick(), events);
        assert_eq!(p.frame_index(), index);
        assert_eq!(p.finished(), finished);
    }
}

#[test]
fn sheet_uses_row_major_cells_and_loops_with_fresh_events() {
    let clip = Package::new().load(SHEET).unwrap();
    let cells: Vec<_> = clip.frames().iter().map(|f| f.source_rect).collect();
    assert_eq!(
        cells,
        [
            SourceRect {
                x: 2,
                y: 2,
                width: 2,
                height: 2
            },
            SourceRect {
                x: 4,
                y: 2,
                width: 2,
                height: 2
            }
        ]
    );
    let mut p = Player::new(&clip);
    for _ in 0..3 {
        assert_eq!(p.tick(), ["one"]);
        assert_eq!(p.frame_index(), 0);
        assert!(p.tick().is_empty());
        assert_eq!(p.frame_index(), 0);
        assert_eq!(p.tick(), ["two"]);
        assert_eq!(p.frame_index(), 1);
        assert!(!p.finished());
    }
}

#[test]
fn single_frame_reenters_only_when_looping() {
    let input = r#"{"format":"OFA","format_version":"1.0","id":"a","loop":false,"source":{"kind":"sequence"},"frames":[{"duration":1,"image":"sheet.png","events":["start"],"origin":{"x":0,"y":0}}]}"#;
    let package = Package::new();
    for looping in [true, false] {
        let clip = package
            .load(&input.replace("false", &looping.to_string()))
            .unwrap();
        let mut p = Player::new(&clip);
        assert_eq!(p.tick(), ["start"]);
        assert_eq!(p.finished(), !looping);
        let events = p.tick();
        if looping {
            assert_eq!(events, ["start"]);
        } else {
            assert!(events.is_empty());
        }
        assert_eq!(p.frame_index(), 0);
    }
}

#[test]
fn malformed_structure_numbers_grid_and_missing_files_report_paths() {
    let package = Package::new();
    for invalid in [
        SHEET.replace("3.0", "4"),
        SHEET.replace("4e0", "6"),
        SHEET.replace("4e0", "-1"),
        SHEET.replace("3.0", "0"),
        SHEET.replace("3.0", "2147483648"),
        SHEET.replace("4e0", "4.0000000000000001"),
        SHEET.replace("4e0", "null"),
        SHEET.replace("4e0", "4,\"image\":null"),
        SHEET.replace("\"1.0\"", "\"2.0\""),
        SHEET.replace("sheet.png", "missing.png"),
        SHEET.replace("sheet.png", "../sheet.png"),
        SEQUENCE.replace("20e-1", "0"),
        SEQUENCE.replace("20e-1", "2.00000000000000000001"),
        SEQUENCE.replace("20e-1", "1e-999"),
        SEQUENCE.replace("foot", "Invalid!"),
        SEQUENCE.replace("\"walk\"", "\"X\""),
        SEQUENCE.replace("\"sequence\"", "\"sequence\",\"columns\":null"),
        SEQUENCE.replace("\"sequence\"", "\"sequence\",\"rows\":2"),
        SEQUENCE.replace("\"duration\":3", "\"duration\":3,\"index\":0"),
    ] {
        let error = package.load(&invalid).unwrap_err();
        assert!(error.contains("anim/clip.json"), "{error}");
    }
    for bad in [
        Vec::new(),
        PNG[..8].to_vec(),
        PNG[..32].to_vec(),
        {
            let mut p = PNG.to_vec();
            p[16..20].fill(0);
            p
        },
        {
            let mut p = PNG.to_vec();
            p[25] = 255;
            p
        },
    ] {
        fs::write(package.0.join("anim/sheet.png"), bad).unwrap();
        assert!(package.load(SEQUENCE).unwrap_err().contains("sheet.png"));
    }
}
