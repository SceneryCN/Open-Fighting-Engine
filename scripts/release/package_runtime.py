"""Archive a tested Godot export plus its matching source and license notices."""
from io import BytesIO
from pathlib import Path
import subprocess
import sys
import tarfile

from package_sim import ROOT, crate_notices


def package(folder, output):
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    with tarfile.open(output, "w:gz") as archive:
        for entry in sorted(folder.iterdir()):
            archive.add(entry, arcname=entry.name)
        archive.add(ROOT / "LICENSE", arcname="LICENSE")
        archive.add(ROOT / "LICENSES", arcname="LICENSES")
        archive.add(ROOT / "integrations/godot/licenses", arcname="LICENSES/Godot")
        for path, name in crate_notices("ofe-bridge"):
            archive.add(path, arcname=name)
        readme = (f"Open Fighting Engine - Godot runtime\n"
                  f"Matching source: https://github.com/SceneryCN/Open-Fighting-Engine/tree/{revision}\n"
                  "Rust standard library notices: https://doc.rust-lang.org/1.100.0/copyright.html\n"
                  "Run the included executable to play. R restarts the match. Controls appear on screen.\n"
                  "Built-in example character data and sprites: CC0-1.0.\n").encode()
        header = tarfile.TarInfo("README.txt")
        header.size = len(readme)
        archive.addfile(header, BytesIO(readme))


if __name__ == "__main__":
    package(Path(sys.argv[1]), Path(sys.argv[2]))
