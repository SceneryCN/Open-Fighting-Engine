// SPDX-License-Identifier: MPL-2.0
use super::valid_id;
use ofe_combat_core::{
    arena::{Cancel, MoveConfig, MoveKind, Window},
    duel::{AttackConfig, LaneRule, Rect},
};
use ofe_resource_runtime::json_integer;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// One OFC action and its resolved profile, in manifest order.
pub struct ResolvedMove {
    definition: Move,
    kind: MoveKind,
    lane_rule: LaneRule,
    windows: Vec<Window>,
    cancels: Vec<Cancel>,
}
impl ResolvedMove {
    /// OFC move identifier.
    pub fn id(&self) -> &str {
        &self.definition.id
    }
    /// Bind this action to the chosen local attack rectangle for an Arena instance.
    pub fn combat_move(&self, hitbox: Rect) -> MoveConfig {
        let m = &self.definition;
        MoveConfig {
            base: m.base(hitbox, self.lane_rule),
            kind: self.kind,
            hitstop: m.hitstop,
            windows: self.windows.clone(),
            cancels: self.cancels.clone(),
            armor: m.armor,
            invulnerability: m.invulnerability,
            energy_cost: m.energy_cost as i32,
        }
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Move {
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
    fn lane(&self) -> Result<LaneRule, String> {
        let lane_rule = match (self.lane_rule.as_str(), self.depth_range) {
            ("SAME_LANE", None) => LaneRule::SameLane,
            ("CROSS_LANE", None) => LaneRule::CrossLane,
            ("BOTH_LANES", None) => LaneRule::BothLanes,
            ("DEPTH_RANGE", Some(range)) if range >= 0 => LaneRule::DepthRange(range),
            _ => return Err("invalid combination of lane_rule and depth_range".into()),
        };
        Ok(lane_rule)
    }
    fn base(&self, hitbox: Rect, lane_rule: LaneRule) -> AttackConfig {
        AttackConfig {
            startup: self.startup,
            active: self.active,
            recovery: self.recovery,
            damage: self.damage,
            hitstun: self.hitstun,
            blockstun: self.blockstun,
            pushback: self.pushback,
            hitbox,
            lane_rule,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Profile {
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

pub(crate) fn moves(
    moves: Vec<Move>,
    profile: Option<Profile>,
) -> Result<Vec<ResolvedMove>, String> {
    let profile = profile.unwrap_or_else(|| Profile {
        format: "OFE_MOVE_PROFILE".into(),
        format_version: "1.0".into(),
        moves: moves
            .iter()
            .map(|m| ProfileMove {
                id: m.id.clone(),
                kind: "strike".into(),
                windows: vec![],
                cancel_into: vec![],
                speed: None,
                lifetime: None,
            })
            .collect(),
    });
    if profile.format != "OFE_MOVE_PROFILE" || profile.format_version != "1.0" {
        return Err("expected OFE_MOVE_PROFILE 1.0".into());
    }
    let ids: BTreeMap<_, _> = moves
        .iter()
        .enumerate()
        .map(|(i, m)| (m.id.clone(), i))
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
        .into_iter()
        .map(|m| {
            let p = profiles
                .get(m.id.as_str())
                .ok_or_else(|| format!("missing profile for {}", m.id))?;
            let durations = [
                m.startup,
                m.active,
                m.recovery,
                m.hitstun,
                m.blockstun,
                m.hitstop,
                m.armor,
                m.invulnerability,
                m.energy_cost,
            ];
            if !valid_id(&m.id)
                || m.damage < 0
                || m.active == 0
                || durations.iter().any(|&v| v > i32::MAX as u32)
            {
                return Err(format!("{}: invalid move ID or numerical range", m.id));
            }
            let lane_rule = m.lane()?;
            let end_active = u64::from(m.startup) + u64::from(m.active);
            let total = end_active + u64::from(m.recovery);
            let mut previous = None;
            for w in &p.windows {
                if w.start < m.startup
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
                if w.start > w.end || w.end > i32::MAX as u32 || u64::from(w.end) >= total {
                    return Err(format!("{}: cancel window outside move", m.id));
                }
                for &into in &targets {
                    cancels.push(Cancel {
                        window: w.into(),
                        into,
                    });
                }
            }
            Ok(ResolvedMove {
                definition: m,
                kind: p.kind()?,
                lane_rule,
                windows: p.windows.iter().copied().map(Window::from).collect(),
                cancels,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::Bounds;
    use crate::Attributes;

    #[test]
    fn typed_documents_accept_exact_integers_in_all_json_number_forms() {
        for (token, expected) in [
            ("60.0", 60),
            ("6e1", 60),
            ("0.060e3", 60),
            ("-0e9999999999999999999999999999", 0),
            ("2147483647.00000000000", i32::MAX),
            ("-214748364800e-2", i32::MIN),
        ] {
            let input = format!(r#"{{"max_health":{token},"max_energy":0}}"#);
            let attributes: Attributes = serde_json::from_str(&input).unwrap();
            assert_eq!(attributes.max_health, expected, "{token}");
        }
        let window: Bounds =
            serde_json::from_str(r#"{"start":0.0,"end":429496729500e-2}"#).unwrap();
        assert_eq!(window.end, u32::MAX);
    }

    #[test]
    fn fractions_overflow_and_non_numbers_cannot_silently_round() {
        for token in [
            "2.0000000000000000001",
            "1e-400",
            "2147483648.0",
            "-2147483649e0",
            "1e1000000",
            "null",
            "\"4\"",
            "{}",
        ] {
            let input = format!(r#"{{"max_health":{token},"max_energy":0}}"#);
            assert!(
                serde_json::from_str::<Attributes>(&input).is_err(),
                "{token}"
            );
        }
        assert!(serde_json::from_str::<Bounds>(r#"{"start":-1e0,"end":0}"#).is_err());
    }
}
