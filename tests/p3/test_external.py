"""Black-box release CLI checks. Point OFE_SIM at an already built executable."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from zipfile import ZIP_STORED, ZipFile, ZipInfo

ROOT = Path(__file__).resolve().parents[2]
CLI = ROOT / "tools/ofe-validator/ofe.py"
SIM = Path(os.environ.get("OFE_SIM", ROOT / "target/release" / ("ofe-sim.exe" if os.name == "nt" else "ofe-sim"))).resolve()
GRID = Path(__file__).with_name("fixtures") / "grid.png"


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value), encoding="utf-8")


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def fixture(root):
    manifest = dict(format="OFC", format_version="1.0", id="qa_external",
                    author="OFE QA", license="CC0-1.0", character="info.json",
                    attributes="stats.json", moves=["moves/a.json", "moves/b.json"],
                    animations=["clips/wait.json"], hitboxes=["boxes.json"],
                    sounds=[], assets=["clips/grid.png"], licenses=["notice.txt"], dependencies=[])
    write(root / "manifest.json", manifest)
    write(root / "info.json", dict(name="QA", description="Independent P3 runtime fixture"))
    write(root / "stats.json", dict(max_health=127, max_energy=0))
    attack = dict(startup=0, active=1, recovery=0, damage=13, hitstun=0, blockstun=0,
                  hitstop=0, pushback=0, cancel_windows=[], invulnerability=0, armor=0,
                  energy_cost=0, lane_rule="BOTH_LANES")
    write(root / "moves/a.json", dict(attack, id="first"))
    write(root / "moves/b.json", dict(attack, id="second", damage=7))
    box = dict(x=-32, h=0, width=64, height=5)
    write(root / "boxes.json", dict(id="body", boxes=[dict(box, kind="hurt"), dict(box, kind="hit")]))
    write(root / "clips" / "wait.json", dict(format="OFA", format_version="1.0",
          id="wait", loop=True, source=dict(kind="sheet", image="grid.png", columns=3, rows=2),
          frames=[dict(index=4, duration=2, origin=dict(x=-2, y=1), events=["enter"])]))
    (root / "clips/grid.png").write_bytes(GRID.read_bytes())
    (root / "notice.txt").write_text("Original QA fixture, CC0-1.0.", encoding="utf-8")
    (root / "unreferenced.txt").write_text("should stay outside the package", encoding="utf-8")


class ExternalPackage(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.temp = Path(tmp.name)
        self.source = self.temp / "external actor"
        fixture(self.source)
        self.package = self.temp / "external actor.ofc"
        self.script = self.temp / "frames.txt"
        self.script.write_text("A -\n", encoding="utf-8")

    def invoke(self, *args, status=0):
        result = subprocess.run([str(arg) for arg in args], cwd=self.temp,
                                capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, status, result.stdout + result.stderr)
        return result

    def cli(self, *args, status=0):
        return self.invoke(sys.executable, CLI, *args, status=status)

    def pack(self):
        self.cli("package", self.source, self.package)

    def test_reproducible_archive_and_metadata(self):
        self.pack()
        original = self.package.read_bytes()
        self.pack()
        self.assertEqual(self.package.read_bytes(), original)
        with ZipFile(self.package) as archive:
            files = archive.namelist()
            self.assertEqual(files, sorted(files))
            self.assertEqual(set(files), {"manifest.json", "info.json", "stats.json", "moves/a.json",
                "moves/b.json", "boxes.json", "clips/wait.json", "clips/grid.png", "notice.txt"})
        self.cli("validate", self.source)
        self.cli("validate", self.package)
        metadata = json.loads(self.cli("inspect", self.source).stdout)
        self.assertEqual(metadata, json.loads(self.cli("inspect", self.package).stdout))
        serialized = json.dumps(metadata)
        for field in ("qa_external", "OFE QA", "CC0-1.0", "first", "second", "wait"):
            self.assertIn(field, serialized)

    def test_external_changes_affect_fights_without_rebuild(self):
        self.assertTrue(SIM.is_file(), f"build the release simulator or set OFE_SIM: {SIM}")
        self.pack()
        direct = self.invoke(SIM, "--character", self.source, "--script", self.script).stdout
        for source in (self.source, self.package):
            self.assertEqual(self.cli("run", source, "--simulator", SIM, "--script", self.script).stdout, direct)
        self.assertIn("hp=127", direct)
        self.assertIn("hp=114", direct)
        self.assertIn("damage: 13", direct)
        write(self.source / "stats.json", dict(max_health=200, max_energy=0))
        path = self.source / "moves/a.json"
        write(path, dict(read(path), damage=19))
        updated = self.invoke(SIM, "--character", self.source, "--script", self.script).stdout
        self.assertIn("hp=181", updated)
        self.assertIn("damage: 19", updated)
        self.assertEqual(self.cli("run", self.package, "--simulator", SIM, "--script", self.script).stdout, direct)
        manifest = read(self.source / "manifest.json")
        manifest["moves"].reverse()
        write(self.source / "manifest.json", manifest)
        changed_order = self.invoke(SIM, "--character", self.source, "--script", self.script).stdout
        self.assertIn("damage: 7", changed_order)
        self.assertIn("hp=193", changed_order)

    def test_stored_archive_and_failed_replacement(self):
        self.pack()
        stored = self.temp / "stored.ofc"
        with ZipFile(self.package) as source, ZipFile(stored, "w", ZIP_STORED) as target:
            for name in source.namelist():
                target.writestr(name, source.read(name))
        self.cli("validate", stored)
        previous = self.package.read_bytes()
        (self.source / "clips/grid.png").unlink()
        self.cli("package", self.source, self.package, status=1)
        self.assertEqual(previous, self.package.read_bytes())
        self.cli("validate", self.package)

    def test_zip_rejects_ambiguous_names_and_links(self):
        self.pack()
        original = self.package.read_bytes()
        for name, mode in [("../outside.txt", 0), ("/outside.txt", 0), ("STATS.json", 0),
                           ("ref-link", 0o120777 << 16)]:
            with self.subTest(name=name):
                self.package.write_bytes(original)
                with ZipFile(self.package, "a") as archive:
                    entry = ZipInfo(name)
                    entry.create_system = 3
                    entry.external_attr = mode
                    archive.writestr(entry, "stats.json")
                self.cli("validate", self.package, status=1)
                self.assertFalse((self.temp / "outside.txt").exists())

    def test_invalid_profile_and_missing_local_references(self):
        path = self.source / "moves/a.json"
        write(path, dict(read(path), cancel_windows=[{"start":0,"end":0}]))
        self.cli("validate", self.source, status=1)
        self.invoke(SIM, "--character", self.source, "--script", self.script, status=1)
        write(path, dict(read(path), cancel_windows=[]))
        profile = dict(format="OFE_MOVE_PROFILE", format_version="1.0", moves=[
            dict(id=id, kind="strike", windows=[], cancel_into=[]) for id in ("first", "unknown")])
        write(self.source / "move_profile.json", profile)
        self.cli("validate", self.source, status=1)
        self.invoke(SIM, "--character", self.source, "--script", self.script, status=1)
        (self.source / "move_profile.json").unlink()
        (self.source / "clips/grid.png").unlink()
        self.cli("validate", self.source, status=1)
        self.invoke(SIM, "--character", self.source, "--script", self.script, status=1)


if __name__ == "__main__":
    unittest.main()
