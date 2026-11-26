// SPDX-License-Identifier: MPL-2.0
use super::*;
use ofe_combat_core::arena::{ArenaConfig, Cancel, CombatantConfig, MoveConfig, MoveKind, Window};
use std::collections::{BTreeMap, BTreeSet};

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
#[serde(deny_unknown_fields)]
struct Profile {
    format: String,
    format_version: String,
    moves: Vec<ProfileMove>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileMove {
    id: String,
    kind: String,
    windows: Vec<Bounds>,
    cancel_into: Vec<String>,
    #[serde(default, deserialize_with = "json_integer::optional")]
    speed: Option<i32>,
    #[serde(default, deserialize_with = "json_integer::optional")]
    lifetime: Option<u32>,
}

impl ProfileMove {
    fn kind(&self) -> Result<MoveKind, String> {
        match (self.kind.as_str(), self.speed, self.lifetime) {
            ("strike", None, None) => Ok(MoveKind::Strike),
            ("throw", None, None) => Ok(MoveKind::Throw),
            ("super", None, None) => Ok(MoveKind::Super),
            ("projectile", Some(speed), Some(lifetime)) if speed > 0 && lifetime > 0 => {
                Ok(MoveKind::Projectile { speed, lifetime })
            }
            _ => Err(format!(
                "{}: invalid kind or projectile parameters",
                self.id
            )),
        }
    }
}

impl From<Bounds> for Window {
    fn from(b: Bounds) -> Self {
        Self {
            start: b.start,
            end: b.end,
        }
    }
}

fn moves(manifest: &Manifest, profile: Profile, hitbox: Rect) -> Result<Vec<MoveConfig>, String> {
    if profile.format != "OFE_MOVE_PROFILE" || profile.format_version != "1.0" {
        return Err("expected OFE_MOVE_PROFILE 1.0".into());
    }
    let moves = manifest
        .moves
        .iter()
        .map(|path| document::<Move>(FILES, path))
        .collect::<Result<Vec<_>, _>>()?;
    let ids: BTreeMap<_, _> = moves
        .iter()
        .enumerate()
        .map(|(i, m)| (m.id.as_str(), i))
        .collect();
    if moves.is_empty() || ids.len() != moves.len() {
        return Err("expected nonempty OFC moves with unique IDs".into());
    }
    let mut profiles = BTreeMap::new();
    for p in &profile.moves {
        if !ids.contains_key(p.id.as_str()) || profiles.insert(p.id.as_str(), p).is_some() {
            return Err(format!("unknown or duplicate profile move {}", p.id));
        }
    }
    moves
        .iter()
        .map(|m| {
            let p = profiles
                .get(m.id.as_str())
                .ok_or_else(|| format!("missing profile for {}", m.id))?;
            let base = m.base(hitbox)?;
            let end_active = u64::from(base.startup) + u64::from(base.active);
            let total = end_active + u64::from(base.recovery);
            let mut previous = None;
            for w in &p.windows {
                if w.start < base.startup
                    || w.start > w.end
                    || u64::from(w.end) >= end_active
                    || previous.is_some_and(|end| end >= w.start)
                {
                    return Err(format!("{}: invalid or overlapping hit windows", m.id));
                }
                previous = Some(w.end);
            }
            if p.cancel_into.is_empty() != m.cancel_windows.is_empty() {
                return Err(format!("{}: cancel windows require matching targets", m.id));
            }
            let mut targets = BTreeSet::new();
            for name in &p.cancel_into {
                let index = ids
                    .get(name.as_str())
                    .ok_or_else(|| format!("{}: unknown cancel target {name}", m.id))?;
                if name == &m.id || !targets.insert(*index) {
                    return Err(format!("{}: duplicate or self cancel target {name}", m.id));
                }
            }
            let mut cancels = Vec::new();
            for &w in &m.cancel_windows {
                if w.start > w.end || u64::from(w.end) >= total {
                    return Err(format!("{}: cancel window outside move", m.id));
                }
                for &into in &targets {
                    cancels.push(Cancel {
                        window: w.into(),
                        into,
                    });
                }
            }
            Ok(MoveConfig {
                base,
                kind: p.kind()?,
                hitstop: m.hitstop,
                windows: p.windows.iter().copied().map(Window::from).collect(),
                cancels,
                armor: m.armor,
                invulnerability: m.invulnerability,
                energy_cost: i32::try_from(m.energy_cost).map_err(|_| "energy cost exceeds i32")?,
            })
        })
        .collect()
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
    let manifest: Manifest = document(FILES, &resolve(&scene.manifest)?)?;
    if manifest.format != "OFC" || manifest.format_version != "1.0" {
        return Err("expected OFC 1.0".into());
    }
    let attributes: Attributes = document(FILES, &manifest.attributes)?;
    let boxes: BoxSet = document(FILES, &manifest.hitboxes[0])?;
    let profile: Profile = document(FILES, &resolve(&scene.profile)?)?;
    let moves = moves(&manifest, profile, boxes.one("hit")?)?;
    let hurtbox = boxes.one("hurt")?;
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_profiles_fail_before_simulation() {
        for case in 0..6 {
            let mut profile: Profile = document(FILES, "move_profile.json").unwrap();
            match case {
                0 => {
                    profile.moves.pop();
                }
                1 => profile.moves[1].id = "unexpected".into(),
                2 => profile.moves[1].id = "jab".into(),
                3 => profile.moves[1].cancel_into = vec!["absent".into()],
                4 => profile.moves[1].windows[1].start = 0,
                _ => profile.moves[3].speed = Some(0),
            }
            let manifest = document(FILES, "manifest.json").unwrap();
            let boxes: BoxSet = document(FILES, "hitboxes/body.json").unwrap();
            assert!(
                moves(&manifest, profile, boxes.one("hit").unwrap()).is_err(),
                "case {case}"
            );
        }
    }
}
