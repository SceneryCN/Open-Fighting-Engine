// SPDX-License-Identifier: MIT
//! Build an editable OFC 1.0 directory containing a basic playable character.
use serde::Serialize;
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, io, path::Path};

/// Basic grounded attack in simulation frames and engine world units.
#[derive(Debug, Serialize)]
pub struct NormalMove {
    pub id: String,
    pub startup: i32,
    pub active: i32,
    pub recovery: i32,
    pub damage: i32,
    pub hitstun: i32,
    pub blockstun: i32,
    pub hitstop: i32,
    pub pushback: i32,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BoxKind {
    Hit,
    Hurt,
    Push,
}

#[derive(Serialize)]
pub struct BodyBox {
    pub kind: BoxKind,
    pub x: i32,
    pub h: i32,
    pub width: i32,
    pub height: i32,
}

/// OFA documents are embedded under `animations/<id>.json`. Asset names are relative
/// to animations/assets/. Each value contains the actual bytes that will ship.
pub struct Character {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub notice: String,
    pub max_health: i32,
    pub max_energy: i32,
    pub normal: NormalMove,
    pub boxes: Vec<BodyBox>,
    pub animations: Vec<Value>,
    pub assets: BTreeMap<String, Vec<u8>>,
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn id_valid(id: &str) -> bool {
    id.len() <= 64
        && id.starts_with(|c: char| c.is_ascii_lowercase())
        && id
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_-".contains(&c))
}

fn json_file(files: &mut BTreeMap<String, Vec<u8>>, path: String, value: Value) -> io::Result<()> {
    let mut bytes = serde_json::to_vec_pretty(&value)?;
    bytes.push(b'\n');
    if files.insert(path, bytes).is_some() {
        return Err(invalid("duplicate document path"));
    }
    Ok(())
}

impl Character {
    /// Write a new directory. An existing output path is always preserved.
    /// Run the OFE CLI validator on the result before packaging.
    pub fn write_new(&self, target: &Path) -> io::Result<()> {
        if !id_valid(&self.id) || !id_valid(&self.normal.id) {
            return Err(invalid("invalid character or move id"));
        }
        let mut files = BTreeMap::new();
        let mut animations = Vec::new();
        for clip in &self.animations {
            let id = clip["id"]
                .as_str()
                .filter(|id| id_valid(id))
                .ok_or_else(|| invalid("invalid OFA id"))?;
            let path = format!("animations/{id}.json");
            json_file(&mut files, path.clone(), clip.clone())?;
            animations.push(path);
        }
        let mut assets = Vec::new();
        let mut asset_keys = std::collections::BTreeSet::new();
        for (name, bytes) in &self.assets {
            if !name.split('/').all(|part| {
                !part.is_empty()
                    && !part.starts_with('.')
                    && !part.ends_with('.')
                    && part
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            }) {
                return Err(invalid("asset needs a portable relative path"));
            }
            if !asset_keys.insert(name.to_ascii_lowercase()) {
                return Err(invalid("duplicate asset path"));
            }
            let path = format!("animations/assets/{name}");
            files.insert(path.clone(), bytes.clone());
            assets.push(path);
        }
        let move_path = format!("moves/{}.json", self.normal.id);
        let mut attack = json!(&self.normal);
        attack["cancel_windows"] = json!([]);
        attack["invulnerability"] = json!(0);
        attack["armor"] = json!(0);
        attack["energy_cost"] = json!(0);
        attack["lane_rule"] = json!("SAME_LANE");
        json_file(&mut files, move_path.clone(), attack)?;
        json_file(
            &mut files,
            "hitboxes/body.json".into(),
            json!({"id":"body","boxes":self.boxes}),
        )?;
        json_file(
            &mut files,
            "character.json".into(),
            json!({"name":self.name,"description":self.description}),
        )?;
        json_file(
            &mut files,
            "attributes.json".into(),
            json!({"max_health":self.max_health,"max_energy":self.max_energy}),
        )?;
        files.insert(
            "licenses/notice.txt".into(),
            self.notice.as_bytes().to_vec(),
        );
        json_file(
            &mut files,
            "manifest.json".into(),
            json!({
                "format":"OFC", "format_version":"1.0", "id":self.id, "author":self.author, "license":self.license,
                "character":"character.json", "attributes":"attributes.json","moves":[move_path], "hitboxes":["hitboxes/body.json"],
                "animations":animations, "assets":assets, "licenses":["licenses/notice.txt"], "dependencies":[], "sounds":[]
            }),
        )?;
        fs::create_dir(target)?;
        let result = files.into_iter().try_for_each(|(relative, data)| {
            let path = target.join(relative);
            fs::create_dir_all(path.parent().expect("file in package"))?;
            fs::write(path, data)
        });
        if result.is_err() {
            fs::remove_dir_all(target)?;
        }
        result
    }
}
