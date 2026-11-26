// SPDX-License-Identifier: MPL-2.0
//! Configuration adapter for the simulator's two schema-validated OFC fixtures.
use ofe_character_runtime::CombatData;
use ofe_combat_core::Position;
use ofe_combat_core::duel::{DuelConfig, FighterConfig, LaneConfig};
use ofe_resource_runtime::json_integer;
use serde::Deserialize;

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

fn embedded(files: Files, path: &str) -> Result<&'static str, String> {
    files
        .iter()
        .find(|(name, _)| *name == path)
        .map(|(_, text)| *text)
        .ok_or_else(|| format!("reference {path:?} is missing from the embedded OFC"))
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
    let combat = CombatData::load_from(|p| embedded(files, p), None)?;
    let moves = combat.combat_moves()?;
    let [normal] = moves.as_slice() else {
        return Err("P1 requires one normal move".into());
    };
    if normal.hitstop != 0
        || normal.armor != 0
        || normal.energy_cost != 0
        || normal.invulnerability != 0
        || !normal.cancels.is_empty()
    {
        return Err("P1 requires one basic move without advanced mechanics".into());
    }
    Ok(FighterConfig {
        spawn: Position {
            x: movement.spawn.x,
            z: movement.spawn.z,
            h: movement.spawn.h,
        },
        max_health: combat.attributes.max_health,
        walk_speed: movement.walk_speed,
        jump_speed: movement.jump_speed,
        gravity: movement.gravity,
        hurtbox: combat.rectangle("hurt")?,
        attack: normal.base,
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

mod arena_data;
pub use arena_data::{ArenaMode, external_arena_config, sample_arena_config};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_references_and_relative_escapes_fail() {
        assert!(embedded(AMBER, "../indigo/manifest.json").is_err());
    }
}
