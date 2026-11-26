"""Package the release CLI, example inputs and licenses from the locked build."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
from io import BytesIO

ROOT = Path(__file__).resolve().parents[2]


def cargo_packages():
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--format-version", "1"], cwd=ROOT
    ))
    packages = {p["id"]: p for p in metadata["packages"]}
    nodes = {n["id"]: n for n in metadata["resolve"]["nodes"]}
    pending = [next(p["id"] for p in packages.values() if p["name"] == "ofe-sim")]
    visited = set()
    while pending:
        current = pending.pop()
        if current in visited:
            continue
        visited.add(current)
        pending.extend(dep["pkg"] for dep in nodes[current]["deps"]
                       if any(k["kind"] != "dev" for k in dep["dep_kinds"]))
        if packages[current]["source"]:
            yield packages[current]


def package(output):
    executable = "ofe-sim.exe" if os.name == "nt" else "ofe-sim"
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    output.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(output, "w:gz") as archive:
        for source, name in [
            (ROOT / "target/release" / executable, executable),
            (ROOT / "tools/ofe-sim/fixtures/duel.frames", "duel.frames"),
            (ROOT / "tools/ofe-sim/fixtures/arena.frames", "arena.frames"),
            (ROOT / "tools/ofe-sim/fixtures/amber", "characters/amber"),
            (ROOT / "tools/ofe-validator/ofe.py", "tools/ofe-validator/ofe.py"),
            (ROOT / "tools/ofe-validator/ofc_archive.py", "tools/ofe-validator/ofc_archive.py"),
            (ROOT / "tools/ofe-validator/requirements.txt", "tools/ofe-validator/requirements.txt"),
            (ROOT / "formats", "formats"),
            (ROOT / "LICENSE", "LICENSE"),
            (ROOT / "LICENSES", "LICENSES"),
            (ROOT / "tools/ofe-sim/fixtures/amber/licenses/notice.txt", "LICENSES/samples.txt"),
        ]:
            archive.add(source, arcname=name)
        for crate in cargo_packages():
            directory = Path(crate["manifest_path"]).parent
            notices = sorted(p for p in directory.iterdir()
                             if p.name.upper().startswith(("LICENSE", "COPYING", "NOTICE")))
            if not notices:
                raise ValueError(f"No packaged license found for {crate['name']}")
            for notice in notices:
                archive.add(notice, arcname=f"LICENSES/{crate['name']}-{crate['version']}/{notice.name}")
        readme = (f"OFE combat simulator\nSource: https://github.com/SceneryCN/Open-Fighting-Engine/tree/{revision}\n"
                  f"Run ./{executable} --script duel.frames to replay the example, or ./{executable} for interactive controls.\n"
                  f"Run ./{executable} --mode 2v2 --script arena.frames or use --mode ffa for four independent teams.\n"
                  f"Run ./{executable} --character characters/amber --script duel.frames to use the external character.\n"
                  "Install the authoring CLI dependencies using Python 3.12: python -m pip install --require-hashes -r tools/ofe-validator/requirements.txt\n"
                  "Build a character: python tools/ofe-validator/ofe.py package characters/amber amber.ofc\n"
                  f"Play the packed character: python tools/ofe-validator/ofe.py run amber.ofc --simulator ./{executable} --script duel.frames\n"
                  "Built with Rust 1.100.0. Standard library notices: https://doc.rust-lang.org/1.100.0/copyright.html\n").encode()
        info = tarfile.TarInfo("README.txt")
        info.size = len(readme)
        archive.addfile(info, BytesIO(readme))


if __name__ == "__main__":
    package(Path(sys.argv[1]))
