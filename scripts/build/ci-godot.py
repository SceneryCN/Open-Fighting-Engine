"""Fetch checksum-pinned official CI editor and the host release export template."""
from hashlib import sha256
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
from urllib.request import urlopen
from zipfile import ZipFile

VERSION = "4.7.2"
TAG = f"{VERSION}-stable"
PREFIX = f"Godot_v{TAG}_"
URL = f"https://github.com/godotengine/godot-builds/releases/download/{TAG}/{PREFIX}"
DIGESTS = {
    "linux.x86_64.zip": "cadd3204e728a35d3f13adb7fd0d7902636b79f6b95c40c265eb73b6c35329e4",
    "macos.universal.zip": "c58a24e31d720be9d62f60cb5627c4e695fb72f21b0cfe1bc9ccaa9a3b3ba63e",
    "win64.exe.zip": "731980f9608d61333e5baf54a2ef17210acc7a538446c0cb9969f002aca1e953",
    "export_templates.tpz": "f298490b8d44d934be425a5a65a51bf15f422428b229a06a6e11d9ffea248011",
}


def fetch(suffix, cache):
    path = cache / (PREFIX + suffix)
    if not path.is_file():
        tmp = path.with_suffix(".part")
        with urlopen(URL + suffix, timeout=180) as response, tmp.open("wb") as out:
            shutil.copyfileobj(response, out)
        tmp.replace(path)
    digest = sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1 << 20), b""):
            digest.update(chunk)
    if digest.hexdigest() != DIGESTS[suffix]:
        sys.exit(f"SHA-256 mismatch for {path}")
    return path


def install(cache):
    host = platform.system()
    if host == "Darwin":
        suffix = "macos.universal.zip"
        exe = cache / "Godot.app/Contents/MacOS/Godot"
        template_root = Path.home() / "Library/Application Support/Godot"
        template = "macos.zip"
    elif host == "Linux":
        suffix = "linux.x86_64.zip"
        exe = cache / (PREFIX + "linux.x86_64")
        template_root = Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local/share")) / "godot"
        template = "linux_release.x86_64"
    elif host == "Windows":
        suffix = "win64.exe.zip"
        exe = cache / (PREFIX + "win64_console.exe")
        template_root = Path(os.environ["APPDATA"]) / "Godot"
        template = "windows_release_x86_64.exe"
    else:
        sys.exit(f"Unsupported CI host {host}")
    archive = fetch(suffix, cache)
    if host == "Darwin":
        subprocess.run(["ditto", "-xk", archive, cache], check=True)
    else:
        with ZipFile(archive) as bundle:
            bundle.extractall(cache)
        exe.chmod(0o755)
    out = template_root / "export_templates" / (VERSION + ".stable")
    out.mkdir(parents=True, exist_ok=True)
    with ZipFile(fetch("export_templates.tpz", cache)) as bundle:
        items = ["version.txt", template]
        if host == "Windows":
            items.append("windows_release_x86_64_console.exe")
        for item in items:
            with bundle.open("templates/" + item) as src, (out / item).open("wb") as dest:
                shutil.copyfileobj(src, dest)
            (out / item).chmod(0o755)
    return exe


if __name__ == "__main__":
    cache = Path(sys.argv[1]).resolve()
    cache.mkdir(parents=True, exist_ok=True)
    print(install(cache))
