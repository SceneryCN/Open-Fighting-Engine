// SPDX-License-Identifier: MPL-2.0
//! Bind two authored characters to an external duel scene and its dash/energy settings.
use super::CombatData;
use ofe_combat_core::{
    Position,
    arena::{ArenaConfig, CombatantConfig},
    duel::{FighterConfig, LaneConfig},
};
use ofe_resource_runtime::json_integer;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Spawn {
    #[serde(deserialize_with = "json_integer::read")]
    pub x: i32,
    #[serde(deserialize_with = "json_integer::read")]
    pub z: i32,
    #[serde(deserialize_with = "json_integer::read")]
    pub h: i32,
}

#[derive(Deserialize)]
pub struct Movement {
    pub spawn: Spawn,
    #[serde(deserialize_with = "json_integer::read")]
    pub walk_speed: i32,
    #[serde(deserialize_with = "json_integer::read")]
    pub jump_speed: i32,
    #[serde(deserialize_with = "json_integer::read")]
    pub gravity: i32,
}

#[derive(Deserialize)]
pub struct Lane {
    #[serde(deserialize_with = "json_integer::read")]
    pub startup: u32,
    #[serde(deserialize_with = "json_integer::read")]
    pub travel: u32,
    #[serde(deserialize_with = "json_integer::read")]
    pub recovery: u32,
    #[serde(deserialize_with = "json_integer::read")]
    pub cooldown: u32,
    pub interruptible: bool,
}

#[derive(Deserialize)]
pub struct Scene {
    pub fighters: [Movement; 2],
    pub lane: Lane,
}

#[derive(Deserialize)]
struct Advanced {
    #[serde(deserialize_with = "json_integer::read")]
    dash_speed: i32,
    #[serde(deserialize_with = "json_integer::read")]
    dash_ticks: u32,
    #[serde(deserialize_with = "json_integer::read")]
    initial_energy: i32,
}

/// Build a P3-compatible two-player match; each slot may use a different character.
pub fn bind(
    combat: [&CombatData; 2],
    scene_json: &str,
    advanced_json: &str,
) -> Result<ArenaConfig, String> {
    let scene: Scene = serde_json::from_str(scene_json).map_err(|e| e.to_string())?;
    let advanced: Advanced = serde_json::from_str(advanced_json).map_err(|e| e.to_string())?;
    let mut fighters = Vec::with_capacity(2);
    for (i, (movement, data)) in scene.fighters.into_iter().zip(combat).enumerate() {
        let moves = data.combat_moves()?;
        let id = i as u32 + 1;
        fighters.push(CombatantConfig {
            id,
            team: id,
            base: FighterConfig {
                spawn: Position {
                    x: movement.spawn.x,
                    z: movement.spawn.z,
                    h: movement.spawn.h,
                },
                max_health: data.attributes.max_health,
                walk_speed: movement.walk_speed,
                jump_speed: movement.jump_speed,
                gravity: movement.gravity,
                hurtbox: data.rectangle("hurt")?,
                attack: moves[0].base,
            },
            moves,
            dash_speed: advanced.dash_speed,
            dash_ticks: advanced.dash_ticks,
            max_energy: data.attributes.max_energy,
            initial_energy: advanced.initial_energy.min(data.attributes.max_energy),
        });
    }
    let l = scene.lane;
    Ok(ArenaConfig {
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
