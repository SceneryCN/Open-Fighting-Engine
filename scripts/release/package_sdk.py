"""Assemble the portable author kit from versioned source and data."""
from pathlib import Path
import subprocess
import sys
import tarfile

ROOT = Path(__file__).resolve().parents[2]


def package(destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    paths = ["sdk", "samples", "formats", "tools/ofe-validator", "docs/p7/AUTHORING.md",
             "docs/p7/SDK.md", "rust-toolchain.toml", "LICENSE", "LICENSES"]
    files = subprocess.check_output(["git", "ls-files", "-z", "--", *paths], cwd=ROOT)
    with tarfile.open(destination, "w:gz") as archive:
        for name in files.decode().split("\0"):
            if name:
                archive.add(ROOT / name, arcname=name)
        archive.add(ROOT / "docs/p7/AUTHORING.md", arcname="README.md")


if __name__ == "__main__":
    package(Path(sys.argv[1]))
