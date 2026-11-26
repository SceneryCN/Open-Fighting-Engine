"""Validate the standalone P2 profile, including frame and ID diagnostics."""
import json
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/ofe-validator"))
import ofe


class MoveProfile(unittest.TestCase):
    def validate(self, moves):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "move_profile.json"
            path.write_text(json.dumps(dict(format="OFE_MOVE_PROFILE", format_version="1.0",
                                            moves=moves)), encoding="utf-8")
            ofe.validate(path)

    def test_projectile_requires_speed_and_lifetime(self):
        shot = dict(id="shot", kind="projectile", windows=[], cancel_into=[])
        with self.assertRaises(ofe.Invalid):
            self.validate([shot])
        self.validate([dict(shot, speed=2, lifetime=1)])

    def test_rejects_bad_windows_and_unresolvable_targets(self):
        move = dict(id="jab", kind="strike", windows=[], cancel_into=[])
        for moves in ([move, move], [dict(move, cancel_into=["unknown"])],
                      [dict(move, windows=[dict(start=2, end=3), dict(start=3, end=4)])],
                      [dict(move, speed=1)]):
            with self.subTest(moves=moves), self.assertRaises(ofe.Invalid):
                self.validate(moves)
