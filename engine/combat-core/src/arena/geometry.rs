// SPDX-License-Identifier: MPL-2.0
use crate::duel::*;
pub(super) const FAR: i32 = 1000;

pub(super) fn nearest(z: i32) -> i32 {
    if z >= FAR / 2 { FAR } else { 0 }
}

impl Rect {
    fn valid(self) -> bool {
        self.width > 0 && self.height > 0
    }

    pub(super) fn bounds(self, owner: Fighter) -> [i64; 4] {
        // Intermediate geometry can extend past the representable center coordinates.
        let origin = i64::from(owner.position.x);
        let x = i64::from(self.x);
        let width = i64::from(self.width);
        let left = if owner.facing == 1 {
            origin + x
        } else {
            origin - x - width
        };
        let bottom = i64::from(owner.position.h) + i64::from(self.h);
        [left, left + width, bottom, bottom + i64::from(self.height)]
    }
}

impl LaneRule {
    pub(super) fn allows(self, a: i32, b: i32) -> bool {
        match self {
            Self::SameLane => nearest(a) == nearest(b),
            Self::CrossLane => nearest(a) != nearest(b),
            Self::BothLanes => true,
            Self::DepthRange(range) => a.abs_diff(b) <= range as u32,
        }
    }
}

impl FighterConfig {
    pub(super) fn valid(self) -> bool {
        let attack = self.attack;
        self.max_health > 0
            && self.walk_speed >= 0
            && self.jump_speed > 0
            && self.gravity > 0
            && self.spawn.h >= 0
            && matches!(self.spawn.z, 0 | FAR)
            && self.hurtbox.valid()
            && attack.valid()
    }
}

impl AttackConfig {
    pub(super) fn valid(self) -> bool {
        self.hitbox.valid()
            && self.active > 0
            && self.damage >= 0
            && !matches!(self.lane_rule, LaneRule::DepthRange(n) if n < 0)
    }
}
pub(super) fn overlaps(a: [i64; 4], b: [i64; 4]) -> bool {
    a[0] < b[1] && b[0] < a[1] && a[2] < b[3] && b[2] < a[3]
}
