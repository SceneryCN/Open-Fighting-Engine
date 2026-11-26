"""Verify the distributable archives on their native OS and assemble SHA256SUMS."""
import argparse
import hashlib
import json
from pathlib import Path
import plistlib
import struct
import subprocess
import tarfile
import tempfile

from package_sim import crate_notices
from release_info import TARGETS, metadata, native_target


def require_files(files, required):
    missing = sorted(set(required) - files.keys())
    if missing:
        raise ValueError(f"missing package files: {', '.join(missing)}")


def machine(stream):
    header = stream.read(64)
    if header.startswith(b"MZ"):
        stream.seek(struct.unpack_from("<I", header, 60)[0])
        if stream.read(6) == b"PE\0\0\x64\x86":
            return "windows-x64"
    if header.startswith(b"\x7fELF\x02\x01") and header[18:20] == b"\x3e\0":
        return "linux-x64"
    if header.startswith(b"\xcf\xfa\xed\xfe"):
        cpu, = struct.unpack_from("<I", header, 4)
        return {0x01000007: "macos-x64", 0x0100000C: "macos-arm64"}.get(cpu)
    return None


def verify(archive, kind, target):
    files = {m.name: m for m in archive if m.isfile()}
    require_files(files, ["RELEASE.json", "LICENSE", "LICENSES/MPL-2.0.txt",
                          "LICENSES/MIT.txt", "LICENSES/CC0-1.0.txt"])
    if json.load(archive.extractfile(files["RELEASE.json"])) != metadata(kind, target):
        raise ValueError(f"{kind}: release metadata differs from checkout or target")
    if kind == "author-kit":
        require_files(files, ["README.md", "samples/sdk-author/Cargo.lock", "docs/p7/SDK.md",
                              "samples/characters/rook/manifest.json", "samples/projects/starter/project.json",
                              "sdk/animation-sdk/LICENSE", "sdk/character-sdk/LICENSE", "samples/sdk-author/LICENSE",
                              "tools/ofe-validator/ofe.py", "formats/ogp/schema.json"])
        return
    package = "ofe-bridge" if kind == "runtime" else "ofe-sim"
    require_files(files, [path for _, path in crate_notices(package)])
    require_files(files, ["README.txt", "LICENSES/Rust/COPYRIGHT-library.html"])
    if kind == "sim":
        binaries = ["ofe-sim.exe" if target.startswith("windows") else "ofe-sim"]
        require_files(files, ["duel.frames", "arena.frames", "characters/amber/manifest.json",
                              "LICENSES/samples.txt"])
    else:
        require_files(files, ["LICENSES/Godot/Godot-COPYRIGHT.txt", "LICENSES/Godot/Godot-LICENSE.txt",
                              "LICENSES/Godot/godot-cpp-LICENSE.md"])
        if target.startswith("macos"):
            prefix = "OFE.app/Contents/"
            plist = plistlib.load(archive.extractfile(files[prefix + "Info.plist"]))
            binaries = [prefix + "MacOS/" + plist["CFBundleExecutable"],
                        prefix + "Frameworks/libofe_godot.dylib"]
            if not any(name.startswith(prefix + "Resources/") and name.endswith(".pck") for name in files):
                raise ValueError("missing macOS game data")
        else:
            binaries = (["OFE", "libofe_godot.so"] if target == "linux-x64"
                        else ["OFE.exe", "OFE.console.exe", "ofe_godot.dll"])
            require_files(files, ["OFE.pck"])
    require_files(files, binaries)
    for name in binaries:
        with archive.extractfile(files[name]) as stream:
            if machine(stream) != target:
                raise ValueError(f"{name}: target differs from {target}")
    return binaries[0]


def desktop(target, runtime, sim):
    if target != native_target():
        raise ValueError(f"native {target} host required")
    with tempfile.TemporaryDirectory(prefix="ofe release ") as directory:
        root = Path(directory)
        commands = []
        for kind, path in [("runtime", runtime), ("sim", sim)]:
            with tarfile.open(path, "r:gz") as archive:
                binary = verify(archive, kind, target)
                location = root / kind
                archive.extractall(location, filter="data")
            if kind == "runtime":
                executable = "OFE.console.exe" if target == "windows-x64" else binary
                commands.append(([location / executable, "--headless", "--", "--p5-test"], "OFE_P5_PASS"))
            else:
                commands.append(([location / binary, "--script", location / "duel.frames"], "frame="))
        for command, marker in commands:
            proc = subprocess.run(command, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                  text=True, errors="replace", timeout=120, check=True)
            if marker not in proc.stdout:
                raise ValueError(f"missing {marker} in {proc.stdout}")
        print(f"Verified licenses, {target} binaries, extracted match and simulator.")


def manifest(directory):
    packages = [(f"ofe-{kind}-{target}.tar.gz", kind, target)
                for target in TARGETS for kind in ("sim", "runtime")]
    packages.append(("ofe-author-kit.tar.gz", "author-kit", "portable"))
    expected = {name for name, _, _ in packages}
    available = {p.name for p in directory.glob("*.tar.gz")}
    if available != expected:
        raise ValueError(f"archive set differs: missing {expected - available}, extra {available - expected}")
    lines = []
    for name, kind, target in sorted(packages):
        path = directory / name
        with tarfile.open(path, "r:gz") as archive:
            verify(archive, kind, target)
        with path.open("rb") as stream:
            digest = hashlib.file_digest(stream, "sha256").hexdigest()
        lines.append(f"{digest}  {name}\n")
    (directory / "SHA256SUMS").write_bytes("".join(lines).encode("ascii"))
    print(f"Verified {len(lines)} release assets; wrote SHA256SUMS")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    native = sub.add_parser("desktop")
    native.add_argument("target", choices=TARGETS)
    native.add_argument("runtime", type=Path)
    native.add_argument("sim", type=Path)
    sub.add_parser("manifest").add_argument("directory", type=Path)
    args = parser.parse_args()
    if args.action == "desktop":
        desktop(args.target, args.runtime, args.sim)
    else:
        manifest(args.directory)
