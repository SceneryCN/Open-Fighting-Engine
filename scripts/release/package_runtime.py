"""Archive a tested Godot export plus its matching source and license notices."""
from io import BytesIO
from pathlib import Path
import sys
import tarfile

from package_sim import crate_notices
from release_info import ROOT, add_metadata, add_stdlib_notice, native_target


def package(folder, output):
    output.parent.mkdir(parents=True, exist_ok=True)
    target = native_target()
    with tarfile.open(output, "w:gz") as archive:
        release = add_metadata(archive, "runtime", target)
        add_stdlib_notice(archive)
        for entry in sorted(folder.iterdir()):
            archive.add(entry, arcname=entry.name)
        archive.add(ROOT / "LICENSE", arcname="LICENSE")
        archive.add(ROOT / "LICENSES", arcname="LICENSES")
        archive.add(ROOT / "integrations/godot/licenses", arcname="LICENSES/Godot")
        for path, name in crate_notices("ofe-bridge"):
            archive.add(path, arcname=name)
        command = {"windows-x64": "open OFE.exe", "linux-x64": "run ./OFE",
                   "macos-arm64": "open OFE.app", "macos-x64": "open OFE.app"}[target]
        readme = (f"Open Fighting Engine {release['version']} - {target}\n"
                  f"Matching source: {release['source']}\n"
                  "Rust standard library notices: LICENSES/Rust/COPYRIGHT-library.html\n"
                  f"Extract the whole archive into its own folder, then {command}. Keep its files together.\n"
                  "Choose a mode, select fighters, Continue, then Fight. Esc pauses. Controls lists all seats.\n"
                  "Player 1: A/D move, W jump, S change lane. J or O attack, K guard, L dash, U special, I super.\n"
                  "Built-in example character data, sprites and stage theme music: CC0-1.0.\n").encode()
        header = tarfile.TarInfo("README.txt")
        header.size = len(readme)
        archive.addfile(header, BytesIO(readme))


if __name__ == "__main__":
    package(Path(sys.argv[1]), Path(sys.argv[2]))
