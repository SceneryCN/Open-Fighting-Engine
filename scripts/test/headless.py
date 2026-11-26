"""Import and run the P0 scene (or test its exported executable) with a timeout."""
import argparse
from pathlib import Path
import plistlib
import subprocess
import sys

PROJECT = Path(__file__).resolve().parents[2] / "integrations/godot/runtime"


def run(command, expect_marker=False):
    print("+", *command, flush=True)
    proc = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                          timeout=90, text=True, errors="replace")
    print(proc.stdout, end="")
    if proc.returncode or (expect_marker and "OFE_P0_PASS" not in proc.stdout.splitlines()):
        sys.exit(proc.returncode or 1)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("godot", help="Godot 4.7.2 executable")
    parser.add_argument("--exported", action="store_true", help="test an exported executable")
    args = parser.parse_args()
    executable = Path(args.godot)
    if args.exported and executable.suffix == ".app":
        contents = executable / "Contents"
        with (contents / "Info.plist").open("rb") as plist:
            executable = contents / "MacOS" / plistlib.load(plist)["CFBundleExecutable"]
        args.godot = str(executable.resolve())
    base = [args.godot, "--headless"]
    if not args.exported:
        version = subprocess.check_output([args.godot, "--version"], text=True).strip()
        if not version.startswith("4.7.2.stable."):
            sys.exit(f"Godot 4.7.2 stable required, got {version}")
        base += ["--path", str(PROJECT)]
        run(base + ["--editor", "--import"])
    run(base + ["--", "--p0-test"], expect_marker=True)
