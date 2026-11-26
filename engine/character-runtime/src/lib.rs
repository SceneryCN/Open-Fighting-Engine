// SPDX-License-Identifier: MPL-2.0
//! Character package loading, action profile resolution and native combat-data binding.
pub mod duel_scene;
mod moves;
pub use moves::ResolvedMove;
use ofe_animation_runtime::Clip;
use ofe_combat_core::{arena::MoveConfig, duel::Rect};
use ofe_resource_runtime::{ResourceStore, json_integer};
use serde::{Deserialize, de::DeserializeOwned};
use std::{collections::BTreeSet, io::ErrorKind, path::Path};

fn valid_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id.as_bytes()[0].is_ascii_lowercase()
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-'))
}

fn parse<T: DeserializeOwned>(bytes: &[u8], path: &str) -> Result<T, String> {
    serde_json::from_slice(bytes).map_err(|e| format!("{path}: {e}"))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Dependency {
    id: String,
    version: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format: String,
    format_version: String,
    id: String,
    author: String,
    license: String,
    character: String,
    attributes: String,
    moves: Vec<String>,
    animations: Vec<String>,
    hitboxes: Vec<String>,
    sounds: Vec<String>,
    assets: Vec<String>,
    licenses: Vec<String>,
    dependencies: Vec<Dependency>,
}

impl Manifest {
    fn validate(&self) -> Result<(), String> {
        if self.format != "OFC" || self.format_version != "1.0" {
            return Err("manifest.json: expected OFC 1.0".into());
        }
        if !valid_id(&self.id) || self.author.is_empty() || self.license.is_empty() {
            return Err("manifest.json: invalid id, author or license".into());
        }
        for (list, required) in [
            (&self.moves, true),
            (&self.animations, true),
            (&self.hitboxes, true),
            (&self.sounds, false),
            (&self.assets, false),
            (&self.licenses, true),
        ] {
            if (required && list.is_empty())
                || list.iter().collect::<BTreeSet<_>>().len() != list.len()
            {
                return Err(
                    "manifest.json: empty required list or duplicate resource reference".into(),
                );
            }
        }
        let mut dependencies = BTreeSet::new();
        for d in &self.dependencies {
            if !valid_id(&d.id) || d.version.is_empty() || !dependencies.insert((&d.id, &d.version))
            {
                return Err("manifest.json: invalid or duplicate dependency".into());
            }
        }
        Ok(())
    }
}

/// Validated health and energy limits from attributes.json.
#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub struct Attributes {
    /// Initial and maximum health.
    #[serde(deserialize_with = "json_integer::read")]
    pub max_health: i32,
    /// Upper bound on energy.
    #[serde(deserialize_with = "json_integer::read")]
    pub max_energy: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CharacterInfo {
    name: String,
    description: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BoxSet {
    id: String,
    boxes: Vec<LocalBox>,
}

/// Local collision geometry as declared by the character, independent of the scene.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct LocalBox {
    /// Box purpose: hit, hurt or push.
    pub kind: String,
    /// Horizontal offset facing right.
    #[serde(deserialize_with = "json_integer::read")]
    pub x: i32,
    /// Height offset.
    #[serde(deserialize_with = "json_integer::read")]
    pub h: i32,
    /// Horizontal extent.
    #[serde(deserialize_with = "json_integer::read")]
    pub width: i32,
    /// Vertical extent.
    #[serde(deserialize_with = "json_integer::read")]
    pub height: i32,
}

/// Parsed combat assets shared by bundled fixtures and external packages.
pub struct CombatData {
    /// Character limits.
    pub attributes: Attributes,
    /// Moves in stable manifest order.
    pub moves: Vec<ResolvedMove>,
    /// Local geometry across named box sets.
    pub boxes: Vec<(String, Vec<LocalBox>)>,
}

impl CombatData {
    /// Load just the combat projection from a trusted resource provider (e.g. embedded assets).
    /// The caller supplies the profile path when one is present.
    pub fn load_from<B: AsRef<[u8]>>(
        mut read: impl FnMut(&str) -> Result<B, String>,
        profile: Option<&str>,
    ) -> Result<Self, String> {
        let manifest: Manifest = parse(read("manifest.json")?.as_ref(), "manifest.json")?;
        manifest.validate()?;
        Self::from_manifest(&manifest, &mut read, profile)
    }

    fn from_manifest<B: AsRef<[u8]>>(
        manifest: &Manifest,
        read: &mut impl FnMut(&str) -> Result<B, String>,
        profile: Option<&str>,
    ) -> Result<Self, String> {
        let attr = &manifest.attributes;
        let attributes: Attributes = parse(read(attr)?.as_ref(), attr)?;
        if attributes.max_health <= 0 || attributes.max_energy < 0 {
            return Err(format!("{attr}: invalid health or energy"));
        }
        let mut boxes = Vec::new();
        let mut ids = BTreeSet::new();
        for path in &manifest.hitboxes {
            let set: BoxSet = parse(read(path)?.as_ref(), path)?;
            if !valid_id(&set.id)
                || !ids.insert(set.id.clone())
                || set.boxes.is_empty()
                || set.boxes.iter().any(|b| {
                    !matches!(b.kind.as_str(), "hit" | "hurt" | "push")
                        || b.width <= 0
                        || b.height <= 0
                })
            {
                return Err(format!(
                    "{path}: invalid box set, rectangle or duplicate ID"
                ));
            }
            boxes.push((set.id, set.boxes));
        }
        let mut definitions = Vec::new();
        for path in &manifest.moves {
            definitions.push(parse(read(path)?.as_ref(), path)?);
        }
        let profile = profile.map(|p| parse(read(p)?.as_ref(), p)).transpose()?;
        Ok(Self {
            attributes,
            boxes,
            moves: moves::moves(definitions, profile)?,
        })
    }

    /// Resolve unambiguous geometry for the current Arena representation.
    pub fn rectangle(&self, kind: &str) -> Result<Rect, String> {
        let mut found = self
            .boxes
            .iter()
            .flat_map(|(_, v)| v)
            .filter(|b| b.kind == kind);
        match (found.next(), found.next()) {
            (Some(b), None) => Ok(Rect {
                x: b.x,
                h: b.h,
                width: b.width,
                height: b.height,
            }),
            _ => Err(format!(
                "combat adapter requires exactly one {kind} rectangle"
            )),
        }
    }

    /// Compile the moves in manifest order using the package's combat hit rectangle.
    pub fn combat_moves(&self) -> Result<Vec<MoveConfig>, String> {
        let hit = self.rectangle("hit")?;
        Ok(self.moves.iter().map(|m| m.combat_move(hit)).collect())
    }
}

/// A complete loaded OFC development directory; creation publishes only validated resources.
pub struct Character {
    /// Manifest ID.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Authored description.
    pub description: String,
    /// Author attribution.
    pub author: String,
    /// Package license declaration.
    pub license: String,
    /// Combat attributes, moves and box sets.
    pub combat: CombatData,
    /// Authored animation clips in manifest order.
    pub animations: Vec<Clip>,
    /// Bytes read during this load, shared with clips.
    pub resources: ResourceStore,
}

impl Character {
    /// Load an OFC directory rooted at manifest.json and the optional P2 move profile.
    pub fn load(root: &Path) -> Result<Self, String> {
        let mut resources = ResourceStore::open(root)?;
        let profile = match root.join("move_profile.json").symlink_metadata() {
            Ok(_) => Some("move_profile.json"),
            Err(e) if e.kind() == ErrorKind::NotFound => None,
            Err(e) => return Err(format!("move_profile.json: {e}")),
        };
        let mut read = |p: &str| resources.read("manifest.json", p);
        let manifest: Manifest = parse(&read("manifest.json")?, "manifest.json")?;
        manifest.validate()?;
        let info: CharacterInfo = parse(&read(&manifest.character)?, &manifest.character)?;
        if info.name.is_empty() {
            return Err(format!("{}: empty character name", manifest.character));
        }
        let combat = CombatData::from_manifest(&manifest, &mut read, profile)?;
        for p in manifest
            .sounds
            .iter()
            .chain(&manifest.assets)
            .chain(&manifest.licenses)
        {
            read(p)?;
        }
        let mut animations = Vec::new();
        let mut ids = BTreeSet::new();
        for p in &manifest.animations {
            let clip = Clip::load(&mut resources, p)?;
            if !ids.insert(clip.id().to_owned()) {
                return Err(format!("{p}: duplicate animation ID {}", clip.id()));
            }
            animations.push(clip);
        }
        Ok(Self {
            id: manifest.id,
            name: info.name,
            description: info.description,
            author: manifest.author,
            license: manifest.license,
            combat,
            animations,
            resources,
        })
    }
}
