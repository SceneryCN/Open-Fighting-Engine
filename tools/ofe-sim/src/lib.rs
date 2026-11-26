// SPDX-License-Identifier: MPL-2.0
//! Configuration adapter for the simulator's two schema-validated OFC fixtures.
mod json_integer;
use ofe_combat_core::Position;
use ofe_combat_core::duel::{AttackConfig, DuelConfig, FighterConfig, LaneConfig, LaneRule, Rect};
use serde::{Deserialize, de::DeserializeOwned};

type Files = &'static [(&'static str, &'static str)];

macro_rules! fixture {
    ($dir:literal, $($path:literal),+ $(,)?) => {
        &[$(($path, include_str!(concat!("../fixtures/", $dir, "/", $path)))),+]
    };
}

const AMBER: Files = fixture!(
    "amber",
    "manifest.json",
    "attributes.json",
    "moves/tap.json",
    "hitboxes/body.json"
);
const INDIGO: Files = fixture!(
    "indigo",
    "manifest.json",
    "attributes.json",
    "moves/tap.json",
    "hitboxes/body.json"
);

fn document<T: DeserializeOwned>(files: Files, path: &str) -> Result<T, String> {
    let (_, text) = files
        .iter()
        .find(|(name, _)| *name == path)
        .ok_or_else(|| format!("reference {path:?} is missing from the embedded OFC"))?;
    serde_json::from_str(text).map_err(|error| format!("{path}: {error}"))
}

#[derive(Deserialize)]
struct Manifest {
    format: String,
    format_version: String,
    attributes: String,
    moves: Vec<String>,
    hitboxes: [String; 1],
}

#[derive(Deserialize)]
struct Attributes {
    #[serde(deserialize_with = "json_integer::read")]
    max_health: i32,
    #[serde(deserialize_with = "json_integer::read")]
    max_energy: i32,
}

#[derive(Deserialize)]
struct Move {
    id: String,
    #[serde(deserialize_with = "json_integer::read")]
    startup: u32,
    #[serde(deserialize_with = "json_integer::read")]
    active: u32,
    #[serde(deserialize_with = "json_integer::read")]
    recovery: u32,
    #[serde(deserialize_with = "json_integer::read")]
    damage: i32,
    #[serde(deserialize_with = "json_integer::read")]
    hitstun: u32,
    #[serde(deserialize_with = "json_integer::read")]
    blockstun: u32,
    #[serde(deserialize_with = "json_integer::read")]
    hitstop: u32,
    #[serde(deserialize_with = "json_integer::read")]
    armor: u32,
    #[serde(deserialize_with = "json_integer::read")]
    energy_cost: u32,
    #[serde(deserialize_with = "json_integer::read")]
    invulnerability: u32,
    #[serde(deserialize_with = "json_integer::read")]
    pushback: i32,
    cancel_windows: Vec<Bounds>,
    lane_rule: String,
    #[serde(default, deserialize_with = "json_integer::optional")]
    depth_range: Option<i32>,
}

impl Move {
    fn into_attack(self, hitbox: Rect) -> Result<AttackConfig, String> {
        if self.hitstop != 0
            || self.armor != 0
            || self.energy_cost != 0
            || self.invulnerability != 0
            || !self.cancel_windows.is_empty()
        {
            return Err("P1 requires zero hitstop, armor, energy_cost and invulnerability, and no cancel_windows".into());
        }
        self.base(hitbox)
    }

    fn base(&self, hitbox: Rect) -> Result<AttackConfig, String> {
        let lane_rule = match (self.lane_rule.as_str(), self.depth_range) {
            ("SAME_LANE", None) => LaneRule::SameLane,
            ("CROSS_LANE", None) => LaneRule::CrossLane,
            ("BOTH_LANES", None) => LaneRule::BothLanes,
            ("DEPTH_RANGE", Some(range)) if range >= 0 => LaneRule::DepthRange(range),
            _ => return Err("invalid combination of lane_rule and depth_range".into()),
        };
        Ok(AttackConfig {
            startup: self.startup,
            active: self.active,
            recovery: self.recovery,
            damage: self.damage,
            hitstun: self.hitstun,
            blockstun: self.blockstun,
            pushback: self.pushback,
            hitbox,
            lane_rule,
        })
    }
}

#[derive(Deserialize)]
struct BoxSet {
    boxes: Vec<LocalBox>,
}

#[derive(Deserialize)]
struct LocalBox {
    kind: String,
    #[serde(deserialize_with = "json_integer::read")]
    x: i32,
    #[serde(deserialize_with = "json_integer::read")]
    h: i32,
    #[serde(deserialize_with = "json_integer::read")]
    width: i32,
    #[serde(deserialize_with = "json_integer::read")]
    height: i32,
}

impl BoxSet {
    fn one(&self, kind: &str) -> Result<Rect, String> {
        let mut matched = self.boxes.iter().filter(|entry| entry.kind == kind);
        match (matched.next(), matched.next()) {
            (Some(b), None) => Ok(Rect {
                x: b.x,
                h: b.h,
                width: b.width,
                height: b.height,
            }),
            _ => Err(format!("P1 sample requires exactly one {kind} rectangle")),
        }
    }
}

#[derive(Deserialize)]
struct Spawn {
    #[serde(deserialize_with = "json_integer::read")]
    x: i32,
    #[serde(deserialize_with = "json_integer::read")]
    z: i32,
    #[serde(deserialize_with = "json_integer::read")]
    h: i32,
}

#[derive(Deserialize)]
struct Movement {
    spawn: Spawn,
    #[serde(deserialize_with = "json_integer::read")]
    walk_speed: i32,
    #[serde(deserialize_with = "json_integer::read")]
    jump_speed: i32,
    #[serde(deserialize_with = "json_integer::read")]
    gravity: i32,
}

#[derive(Deserialize)]
struct Lane {
    #[serde(deserialize_with = "json_integer::read")]
    startup: u32,
    #[serde(deserialize_with = "json_integer::read")]
    travel: u32,
    #[serde(deserialize_with = "json_integer::read")]
    recovery: u32,
    #[serde(deserialize_with = "json_integer::read")]
    cooldown: u32,
    interruptible: bool,
}

#[derive(Deserialize)]
struct Scene {
    fighters: [Movement; 2],
    lane: Lane,
}

fn fighter(files: Files, movement: Movement) -> Result<FighterConfig, String> {
    let manifest: Manifest = document(files, "manifest.json")?;
    if manifest.format != "OFC" || manifest.format_version != "1.0" {
        return Err("P1 fixtures require OFC 1.0".into());
    }
    let attributes: Attributes = document(files, &manifest.attributes)?;
    let [path] = manifest.moves.as_slice() else {
        return Err("P1 requires one normal move".into());
    };
    let definition: Move = document(files, path)?;
    let boxes: BoxSet = document(files, &manifest.hitboxes[0])?;
    let attack = definition.into_attack(boxes.one("hit")?)?;
    Ok(FighterConfig {
        spawn: Position {
            x: movement.spawn.x,
            z: movement.spawn.z,
            h: movement.spawn.h,
        },
        max_health: attributes.max_health,
        walk_speed: movement.walk_speed,
        jump_speed: movement.jump_speed,
        gravity: movement.gravity,
        hurtbox: boxes.one("hurt")?,
        attack,
    })
}

/// Decode the two bundled OFC manifests and the scenario into native core configuration.
/// An unsupported move or missing referenced document returns a diagnostic.
pub fn sample_config() -> Result<DuelConfig, String> {
    let scene: Scene = serde_json::from_str(include_str!("../fixtures/scene.json"))
        .map_err(|error| format!("scene.json: {error}"))?;
    let [first, second] = scene.fighters;
    let fighters = [
        fighter(AMBER, first).map_err(|e| format!("amber: {e}"))?,
        fighter(INDIGO, second).map_err(|e| format!("indigo: {e}"))?,
    ];
    let l = scene.lane;
    Ok(DuelConfig {
        fighters,
        lane: LaneConfig {
            startup: l.startup,
            travel: l.travel,
            recovery: l.recovery,
            cooldown: l.cooldown,
            interruptible: l.interruptible,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_references_and_relative_escapes_fail() {
        assert!(document::<Manifest>(AMBER, "../indigo/manifest.json").is_err());
    }

    #[test]
    fn unsupported_move_features_fail() {
        let mut movement = document::<Move>(AMBER, "moves/tap.json").unwrap();
        movement.hitstop = 3;
        let hitbox = document::<BoxSet>(AMBER, "hitboxes/body.json")
            .unwrap()
            .one("hit")
            .unwrap();
        assert!(movement.into_attack(hitbox).is_err());
    }
}

#[derive(Deserialize, Clone, Copy)]
#[serde(deny_unknown_fields)]
struct Bounds {
    #[serde(deserialize_with = "json_integer::read")]
    start: u32,
    #[serde(deserialize_with = "json_integer::read")]
    end: u32,
}

mod arena_data;
pub use arena_data::{ArenaMode, sample_arena_config};
