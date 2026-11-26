"""Prepare a thin release template from Godot 4.7.2's official universal macOS template."""
import argparse
from pathlib import Path
import platform
import subprocess
from tempfile import TemporaryDirectory
from zipfile import ZipFile

PROJECT = Path(__file__).resolve().parents[2] / "integrations/godot/runtime"
TEMPLATE = Path.home() / "Library/Application Support/Godot/export_templates/4.7.2.stable/macos.zip"


def prepare(source, project, arch):
    directory = project / ".godot/ofe-templates"
    directory.mkdir(parents=True, exist_ok=True)
    output = directory / f"macos-{arch}.zip"
    with TemporaryDirectory(dir=directory) as temp:
        temp = Path(temp)
        result = temp / output.name
        with ZipFile(source) as original, ZipFile(result, "w") as archive:
            release = "macos_template.app/Contents/MacOS/godot_macos_release.universal"
            if release not in original.namelist():
                raise ValueError(f"{source} lacks {release}")
            for item in original.infolist():
                if item.filename == release:
                    universal = temp / "universal"
                    thin = temp / "release"
                    universal.write_bytes(original.read(item))
                    subprocess.run(["lipo", universal, "-thin", arch, "-output", thin], check=True)
                    item.filename = release.removesuffix("universal") + arch
                    archive.writestr(item, thin.read_bytes())
                elif not item.filename.endswith("/godot_macos_debug.universal"):
                    archive.writestr(item, original.read(item))
        result.replace(output)
    print(output)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--template", type=Path, default=TEMPLATE)
    parser.add_argument("--project", type=Path, default=PROJECT)
    parser.add_argument("--arch", choices=["arm64", "x86_64"], default=platform.machine())
    options = parser.parse_args()
    prepare(options.template, options.project, options.arch)
