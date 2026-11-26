// SPDX-License-Identifier: MPL-2.0
use super::*;
use ofe_character_runtime::Character;
use ofe_combat_core::arena::{ArenaConfig, CombatantConfig};
use std::path::Path;

const FILES: Files = fixture!(
    "arena",
    "manifest.json",
    "attributes.json",
    "hitboxes/body.json",
    "moves/jab.json",
    "moves/flurry.json",
    "moves/grab.json",
    "moves/bolt.json",
    "moves/nova.json",
    "moves/brace.json",
    "moves/slip.json",
    "move_profile.json"
);

/// Roster to load from the four-slot offline scene.
#[derive(Clone, Copy)]
pub enum ArenaMode {
    /// Slots one and two play against slots three and four.
    Teams,
    /// Each fighter belongs to its own team.
    FreeForAll,
}

#[derive(Deserialize)]
struct Slot {
    #[serde(deserialize_with = "json_integer::read")]
    id: u32,
    #[serde(deserialize_with = "json_integer::read")]
    team: u32,
    #[serde(deserialize_with = "json_integer::read")]
    ffa_team: u32,
    spawn: Spawn,
}

#[derive(Deserialize)]
struct ArenaScene {
    manifest: String,
    profile: String,
    fighters: [Slot; 4],
    lane: Lane,
    #[serde(deserialize_with = "json_integer::read")]
    initial_energy: i32,
    #[serde(deserialize_with = "json_integer::read")]
    walk_speed: i32,
    #[serde(deserialize_with = "json_integer::read")]
    jump_speed: i32,
    #[serde(deserialize_with = "json_integer::read")]
    gravity: i32,
    #[serde(deserialize_with = "json_integer::read")]
    dash_speed: i32,
    #[serde(deserialize_with = "json_integer::read")]
    dash_ticks: u32,
}

/// Read the bundled OFC and move profile, resolving move IDs to manifest order.
pub fn sample_arena_config(mode: ArenaMode) -> Result<ArenaConfig, String> {
    let scene: ArenaScene = serde_json::from_str(include_str!("../fixtures/arena_scene.json"))
        .map_err(|e| format!("arena_scene.json: {e}"))?;
    let resolve = |path: &str| {
        path.strip_prefix("arena/")
            .map(str::to_owned)
            .ok_or_else(|| format!("unavailable fixture: {path}"))
    };
    let manifest_path = resolve(&scene.manifest)?;
    if manifest_path != "manifest.json" {
        return Err(format!("unsupported embedded manifest {manifest_path}"));
    }
    let profile = resolve(&scene.profile)?;
    let data = CombatData::load_from(|p| embedded(FILES, p), Some(&profile))?;
    let moves = data.combat_moves()?;
    let hurtbox = data.rectangle("hurt")?;
    let attributes = data.attributes;
    let fighters = scene
        .fighters
        .into_iter()
        .map(|s| {
            let team = match mode {
                ArenaMode::Teams => s.team,
                ArenaMode::FreeForAll => s.ffa_team,
            };
            let base = FighterConfig {
                spawn: Position {
                    x: s.spawn.x,
                    z: s.spawn.z,
                    h: s.spawn.h,
                },
                max_health: attributes.max_health,
                walk_speed: scene.walk_speed,
                jump_speed: scene.jump_speed,
                gravity: scene.gravity,
                hurtbox,
                attack: moves[0].base,
            };
            CombatantConfig {
                id: s.id,
                team,
                base,
                moves: moves.clone(),
                dash_speed: scene.dash_speed,
                dash_ticks: scene.dash_ticks,
                max_energy: attributes.max_energy,
                initial_energy: scene.initial_energy,
            }
        })
        .collect();
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

/// Build a mirror match from external OFC data and the bundled duel's motion settings.
pub fn external_arena_config(root: &Path) -> Result<ArenaConfig, String> {
    let character = Character::load(root)?;
    let moves = character.combat.combat_moves()?;
    let hurtbox = character.combat.rectangle("hurt")?;
    let scene: Scene =
        serde_json::from_str(include_str!("../fixtures/scene.json")).map_err(|e| e.to_string())?;
    let advanced: ArenaScene = serde_json::from_str(include_str!("../fixtures/arena_scene.json"))
        .map_err(|e| e.to_string())?;
    let attributes = character.combat.attributes;
    let fighters = scene
        .fighters
        .into_iter()
        .enumerate()
        .map(|(i, m)| {
            let id = i as u32 + 1;
            CombatantConfig {
                id,
                team: id,
                base: FighterConfig {
                    spawn: Position {
                        x: m.spawn.x,
                        z: m.spawn.z,
                        h: m.spawn.h,
                    },
                    max_health: attributes.max_health,
                    walk_speed: m.walk_speed,
                    jump_speed: m.jump_speed,
                    gravity: m.gravity,
                    hurtbox,
                    attack: moves[0].base,
                },
                moves: moves.clone(),
                dash_speed: advanced.dash_speed,
                dash_ticks: advanced.dash_ticks,
                max_energy: attributes.max_energy,
                initial_energy: advanced.initial_energy.min(attributes.max_energy),
            }
        })
        .collect();
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
