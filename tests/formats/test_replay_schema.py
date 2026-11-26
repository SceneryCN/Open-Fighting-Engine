"""Check the published replay schema against the actual recording writer."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[2]


class ReplaySchema(unittest.TestCase):
    def test_live_four_fighter_recording(self):
        schema = json.loads((ROOT / "formats/replay/schema.json").read_text())
        Draft202012Validator.check_schema(schema)
        validator = Draft202012Validator(schema)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "match.json"
            subprocess.run(
                ["cargo", "run", "--quiet", "--locked", "-p", "ofe-sim", "--", "record",
                 "--mode", "ffa", "--seed", str(2**64-1), "--script",
                 "tools/ofe-sim/fixtures/arena.frames", "--out", str(path)],
                cwd=ROOT, check=True,
            )
            document = json.loads(path.read_text())
        validator.validate(document)
        self.assertEqual(document["initial"]["rng_state"], 2**64-1)
        document["frames"][0]["commands"][0]["controls"]["axis"] = 2
        self.assertFalse(validator.is_valid(document))
