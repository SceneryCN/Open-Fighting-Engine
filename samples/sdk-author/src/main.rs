// SPDX-License-Identifier: MIT
use ofe_animation_sdk::{Animation, Frame, Frames};
use ofe_character_sdk::{BodyBox, BoxKind, Character, NormalMove};
use std::{collections::BTreeMap, error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let target = args
        .next()
        .map(PathBuf::from)
        .ok_or("expected a new output directory")?;
    if args.next().is_some() {
        return Err("usage: rook-author NEW_DIRECTORY".into());
    }
    let idle = Animation {
        id: "idle".into(),
        looping: true,
        frames: Frames::Sequence(vec![
            Frame::new("assets/idle.png".into(), 14, 116, 247),
            Frame::new("assets/idle.png".into(), 14, 116, 249),
        ]),
    };
    let mut contact = Frame::new(1, 3, 116, 247);
    contact.events.push("contact".into());
    let attack = Animation {
        id: "attack".into(),
        looping: false,
        frames: Frames::Sheet {
            image: "assets/rook.png".into(),
            columns: 2,
            rows: 1,
            frames: vec![
                Frame::new(0, 2, 116, 247),
                contact,
                Frame::new(0, 5, 116, 247),
            ],
        },
    };
    let character = Character {
        id: "rook".into(),
        name: "Rook".into(),
        description: "A jade training fighter with a straight punch.".into(),
        author: "OFE contributors".into(),
        license: "CC0-1.0".into(),
        notice: include_str!("../art/NOTICE.txt").into(),
        max_health: 96,
        max_energy: 0,
        normal: NormalMove {
            id: "straight".into(),
            startup: 2,
            active: 3,
            recovery: 5,
            damage: 12,
            hitstun: 6,
            blockstun: 3,
            hitstop: 1,
            pushback: 1,
        },
        boxes: vec![
            BodyBox {
                kind: BoxKind::Hurt,
                x: -1,
                h: 0,
                width: 2,
                height: 5,
            },
            BodyBox {
                kind: BoxKind::Hit,
                x: 1,
                h: 1,
                width: 4,
                height: 3,
            },
            BodyBox {
                kind: BoxKind::Push,
                x: -1,
                h: 0,
                width: 2,
                height: 3,
            },
        ],
        animations: vec![idle.document()?, attack.document()?],
        assets: BTreeMap::from([
            (
                "rook.png".into(),
                include_bytes!("../art/rook.png").to_vec(),
            ),
            (
                "idle.png".into(),
                include_bytes!("../art/idle.png").to_vec(),
            ),
        ]),
    };
    character.write_new(&target)?;
    Ok(())
}
