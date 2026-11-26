"""Provenance shared by OFE release artifacts."""
from io import BytesIO
import json
from pathlib import Path
import platform
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parents[2]
TARGETS = ("windows-x64", "linux-x64", "macos-x64", "macos-arm64")


def native_target():
    system = {"Windows": "windows", "Linux": "linux", "Darwin": "macos"}[platform.system()]
    cpu = {"amd64": "x64", "x86_64": "x64", "arm64": "arm64", "aarch64": "arm64"}[
        platform.machine().lower()]
    target = f"{system}-{cpu}"
    if target not in TARGETS:
        raise ValueError(f"unsupported release platform: {target}")
    return target


def metadata(kind, target):
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    return {"product": "Open Fighting Engine", "package": kind, "target": target,
            "version": (ROOT / "VERSION").read_text(encoding="utf-8").strip(),
            "revision": revision, "source": f"https://github.com/SceneryCN/Open-Fighting-Engine/tree/{revision}"}


def add_metadata(archive, kind, target):
    info = metadata(kind, target)
    data = (json.dumps(info, indent=2) + "\n").encode()
    entry = tarfile.TarInfo("RELEASE.json")
    entry.size = len(data)
    archive.addfile(entry, BytesIO(data))
    return info


def add_stdlib_notice(archive):
    sysroot = subprocess.check_output(["rustc", "--print", "sysroot"], cwd=ROOT, text=True).strip()
    path = Path(sysroot) / "share/doc/rust/COPYRIGHT-library.html"
    archive.add(path, arcname="LICENSES/Rust/COPYRIGHT-library.html")
