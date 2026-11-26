"""Check runtime --p1-dir and --p2-dir against changed OFC data in a fresh directory."""
import json
from pathlib import Path
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory

ROOT = Path(__file__).resolve().parents[2]


def update(path, key, value):
    data = json.loads(path.read_text(encoding="utf-8"))
    data[key] = value
    path.write_text(json.dumps(data), encoding="utf-8")


def main(godot):
    fixtures = ROOT / "tools/ofe-sim/fixtures"
    with TemporaryDirectory() as scratch:
        scratch = Path(scratch)
        p1 = scratch / "external amber"
        p2 = scratch / "external indigo"
        shutil.copytree(fixtures / "amber", p1)
        shutil.copytree(fixtures / "indigo", p2)
        update(p1 / "attributes.json", "max_health", 137)
        update(p2 / "attributes.json", "max_health", 191)
        update(p1 / "moves/tap.json", "damage", 23)
        proc = subprocess.run([godot, "--headless", "--path", str(ROOT / "integrations/godot/runtime"),
            "--script", str(Path(__file__).with_suffix(".gd").resolve()), "--",
            f"--p1-dir={p1}", f"--p2-dir={p2}"], cwd=scratch, text=True, capture_output=True, timeout=60)
        print(proc.stdout, end="")
        print(proc.stderr, end="", file=sys.stderr)
        return proc.returncode or int("OFE_P4_EXTERNAL_PASS" not in proc.stdout.splitlines())


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
