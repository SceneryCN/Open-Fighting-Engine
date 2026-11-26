"""Import and run the P0, P4, P5 and P6 scenes (or test its exported executable) with a timeout."""
import argparse
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
from tempfile import TemporaryDirectory

PROJECT = Path(__file__).resolve().parents[2] / "integrations/godot/runtime"


def run(command, marker=None):
    print("+", *command, flush=True)
    proc = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                          timeout=90, text=True, errors="replace")
    print(proc.stdout, end="")
    if proc.returncode or (marker and marker not in proc.stdout.splitlines()):
        sys.exit(proc.returncode or 1)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("godot", help="Godot 4.7.2 executable")
    parser.add_argument("--exported", action="store_true", help="test an exported executable")
    parser.add_argument("--simulator", type=Path, help="verify the Godot recording with this ofe-sim binary")
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
        run(base + ["--script", str(Path(__file__).with_name("scene_reentry.gd").resolve())],
            marker="OFE_REENTRY_PASS")
        run([sys.executable, str(Path(__file__).with_name("p4_external.py")),
             str(Path(shutil.which(args.godot) or args.godot).resolve())], marker="OFE_P4_EXTERNAL_PASS")
    run(base + ["--", "--p0-test"], marker="OFE_P0_PASS")
    run(base + ["--", "--p4-test"], marker="OFE_P4_PASS")
    run(base + ["--", "--p5-test"], marker="OFE_P5_PASS")
    with TemporaryDirectory(prefix="ofe-p6-") as scratch:
        tape = Path(scratch).resolve() / "training.ofr"
        record = base + ["--", "--p6-test", f"--qa-replay={tape}"]
        run(record, marker="OFE_P6_PASS")
        if args.simulator:
            run([str(args.simulator.resolve()), "replay", str(tape)])
        run(record + ["--qa-play"], marker="OFE_P6_PASS")
