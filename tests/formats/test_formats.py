"""P0 format contracts, version rejection, resource graph and diagnostics."""

import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools" / "ofe-validator"))
import ofe

FIXTURES = ROOT / "tests" / "fixtures" / "formats"


class Validation(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        shutil.copytree(FIXTURES / "valid", self.root, dirs_exist_ok=True)
        self.project = self.root / "project.json"

    def rewrite(self, relative, change):
        path = self.root / relative
        data = json.loads(path.read_text(encoding="utf-8"))
        change(data)
        path.write_text(json.dumps(data), encoding="utf-8")
        return path

    def test_valid_graph_and_sheet(self):
        ofe.validate(self.project)
        ofe.validate(self.root / "character/animations/sheet.json")

    def test_invalid_examples(self):
        for kind, member in [("ofc", "format_version"), ("ofa", "frames"),
                             ("ogp", "modes"), ("ofl", "screens")]:
            with self.subTest(kind=kind), self.assertRaisesRegex(ofe.Invalid, "#/" + member):
                ofe.validate(FIXTURES / "invalid" / f"{kind}.json")

    def test_unsupported_version(self):
        for path in ["project.json", "layout.json", "character/manifest.json", "character/animations/idle.json"]:
            with self.subTest(path=path), self.assertRaisesRegex(ofe.Invalid, "#/format_version"):
                ofe.validate(self.rewrite(path, lambda doc: doc.update(format_version="1.1")))

    def test_referenced_wrong_format(self):
        self.rewrite("project.json", lambda doc: doc.update(layout="project.json"))
        with self.assertRaisesRegex(ofe.Invalid, "#/layout -> "):
            ofe.validate(self.project)

    def test_nested_missing_resource(self):
        (self.root / "character/animations/assets/pixel.png").unlink()
        with self.assertRaisesRegex(ofe.Invalid, r"#/characters/0.*#/animations/0.*#/frames/0/image"):
            ofe.validate(self.project)

    def test_path_rules(self):
        for path in ["../escape", "/absolute", "C:/windows", "C:relative", "file://server",
                     "assets\\alien", "stage.json/", "\x00"]:
            with self.subTest(path=path), self.assertRaisesRegex(ofe.Invalid, "#/theme"):
                ofe.validate(self.rewrite("project.json", lambda doc: doc.update(theme=path)))

    def test_symlink_escape(self):
        with tempfile.TemporaryDirectory() as external:
            asset = Path(external) / "outside"
            asset.write_text("outside", encoding="utf-8")
            (self.root / "theme.json").unlink()
            try:
                (self.root / "theme.json").symlink_to(asset)
            except OSError as exc:
                self.skipTest(f"file symlinks unavailable: {exc}")
            with self.assertRaisesRegex(ofe.Invalid, "#/theme.*inside"):
                ofe.validate(self.project)

    def test_sheet_index(self):
        def out_of_bounds(doc):
            doc["frames"][0]["index"] = 1
        with self.assertRaisesRegex(ofe.Invalid, "#/frames/0/index"):
            ofe.validate(self.rewrite("character/animations/sheet.json", out_of_bounds))

    def test_png_source_type(self):
        (self.root / "character/animations/assets/pixel.png").write_bytes(b"not a PNG")
        with self.assertRaisesRegex(ofe.Invalid, "#/frames/0/image.*PNG"):
            ofe.validate(self.project)

    def test_move_depth_and_windows(self):
        path = "character/moves/tap.json"
        self.rewrite(path, lambda doc: doc.update(lane_rule="DEPTH_RANGE"))
        with self.assertRaisesRegex(ofe.Invalid, "depth_range"):
            ofe.validate(self.project)
        self.rewrite(path, lambda doc: doc.update(depth_range=300))
        ofe.validate(self.project)
        self.rewrite(path, lambda doc: doc.update(cancel_windows=[{"start": 3, "end": 2}]))
        with self.assertRaisesRegex(ofe.Invalid, "#/cancel_windows/0/end"):
            ofe.validate(self.project)

    def test_screen_element_ids(self):
        self.rewrite("layout.json", lambda doc: doc["screens"]["pause"].extend(doc["screens"]["pause"][:]))
        with self.assertRaisesRegex(ofe.Invalid, "#/screens/pause/1/id"):
            ofe.validate(self.project)

    def test_integer_uses_exact_decimal_value(self):
        attributes = self.root / "character/attributes.json"
        for token in ("1", "1.0", "10e-1", "0.1e1", "2147483647.0000000"):
            with self.subTest(token=token):
                attributes.write_text('{"max_health":' + token + ',"max_energy":0}')
                ofe.validate(self.project)
        for token in ("1.00000000000000000001", "2147483647.00000001", "1e-400", "2147483648.0"):
            with self.subTest(token=token), self.assertRaisesRegex(ofe.Invalid, "#/max_health"):
                attributes.write_text('{"max_health":' + token + ',"max_energy":0}')
                ofe.validate(self.project)

    def test_strict_json_and_shape(self):
        for payload in ['{"format":"OGP","format":"OFA"}', '{"x":NaN}', '{"x":1e400}', '[]']:
            with self.subTest(payload=payload), self.assertRaises(ofe.Invalid):
                self.project.write_text(payload)
                ofe.validate(self.project)


if __name__ == "__main__":
    unittest.main()
