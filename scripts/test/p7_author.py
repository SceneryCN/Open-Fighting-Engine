"""Exercise two externally authored fighters through the real editor or exported game."""
import argparse
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
PROJECT = ROOT / "integrations/godot/runtime"

# launch accepts a single executable. This portable adapter only adds Godot test
# switches and passes the production CLI's arguments and exit status unchanged.
ADAPTER = r"""
use std::{env, fs, process::{Command, exit}};
fn main() {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
    fs::write(env::var_os("QA_MARKER").unwrap(), now.as_nanos().to_string()).unwrap();
    let catalog = env::args().find_map(|arg| arg.strip_prefix("--ofe-project=").map(str::to_owned)).unwrap();
    fs::write(env::var_os("QA_CATALOG_PATH").unwrap(), &catalog).unwrap();
    fs::copy(catalog, env::var_os("QA_CATALOG_COPY").unwrap()).unwrap();
    let mut game = Command::new(env::var_os("QA_GAME").unwrap());
    if env::var_os("QA_PACING").is_none() { game.arg("--headless"); }
    else { game.args(["--max-fps", "60"]); }
    if let Some(project) = env::var_os("QA_PROJECT") { game.arg("--path").arg(project); }
    game.args(env::args_os().skip(1)).arg("--p7-test");
    if env::var_os("QA_PACING").is_some() { game.arg("--qa-pacing"); }
    exit(game.status().unwrap().code().unwrap_or(1));
}
"""


def doc(path):
    return json.loads(path.read_text(encoding="utf-8"))


def save(path, data):
    path.write_text(json.dumps(data, ensure_ascii=False), encoding="utf-8")


def run(command, cwd, env=None, ok=True):
    proc = subprocess.run(command, cwd=cwd, env=env, text=True, errors="replace",
                          stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=180)
    if (proc.returncode == 0) != ok:
        raise RuntimeError(f"{command} returned {proc.returncode}\n{proc.stdout}")
    return proc.stdout


def write_variant(author, source, output, name, health, fighter_id):
    changes = {'name: "Rook"': f'name: "{name}"', 'id: "rook"': f'id: "{fighter_id}"',
               "max_health: 96": f"max_health: {health}", "damage: 12": "damage: 23",
               "116, 247": "9, 17", "116, 249": "9, 17"}
    for original, replacement in changes.items():
        assert original in source, f"author fixture missing {original!r}"
        source = source.replace(original, replacement)
    (author / "src/main.rs").write_text(source, encoding="utf-8")
    run(["cargo", "run", "--locked", "--", output], author)


def file_contents(folder):
    files = {}
    for path in folder.rglob("*"):
        if path.is_file():
            data = path.read_bytes()
            files[path.relative_to(folder)] = data if path.suffix == ".png" else data.replace(b"\r\n", b"\n")
    return files


def exercise(args, temp):
    kit = args.kit.resolve() if args.kit else ROOT
    author_cli = kit / "tools/ofe-validator/ofe.py"
    def cli(*params, ok=True):
        return run([sys.executable, author_cli, *params], temp, env=env, ok=ok)

    game = Path(shutil.which(args.game) or args.game).resolve()
    if args.exported and game.suffix == ".app":
        contents = game / "Contents"
        with (contents / "Info.plist").open("rb") as fh:
            game = contents / "MacOS" / plistlib.load(fh)["CFBundleExecutable"]
    marker = temp / "child-started"
    stage = temp / "staging"
    stage.mkdir()
    catalog_path, catalog_copy = temp / "catalog-path", temp / "catalog-copy"
    env = dict(os.environ, QA_GAME=str(game), QA_MARKER=str(marker),
               QA_CATALOG_PATH=str(catalog_path), QA_CATALOG_COPY=str(catalog_copy),
               TMPDIR=str(stage), TEMP=str(stage), TMP=str(stage))
    env.pop("QA_PROJECT", None)
    env.pop("QA_PACING", None)
    if not args.exported:
        env["QA_PROJECT"] = str(PROJECT)
    if args.pacing:
        env["QA_PACING"] = "1"
    adapter = temp / ("runtime.exe" if os.name == "nt" else "runtime")
    adapter_src = temp / "adapter.rs"
    adapter_src.write_text(ADAPTER, encoding="utf-8")
    run(["rustc", adapter_src, "-o", adapter], ROOT)

    # Only distribute the SDKs and author example to the temporary author.
    author = temp / "samples/sdk-author"
    shutil.copytree(kit / "samples/sdk-author", author, ignore=shutil.ignore_patterns("target"))
    shutil.copytree(kit / "sdk", temp / "sdk", ignore=shutil.ignore_patterns("target"))
    shutil.copyfile(kit / "rust-toolchain.toml", temp / "rust-toolchain.toml")
    rook = temp / "author output"
    run(["cargo", "run", "--locked", "--", rook], author)
    base = kit / "samples/characters/rook"
    actual = file_contents(rook)
    expected = file_contents(base)
    assert actual and actual == expected, "SDK output differs from the distributed fighter"
    echo = temp / "echo"
    source = (author / "src/main.rs").read_text(encoding="utf-8")
    shutil.rmtree(rook)
    write_variant(author, source, rook, "QA Rook", 137, "rook")
    write_variant(author, source, echo, "QA Echo", 113, "qa_echo")

    project = temp / "editable project"
    shutil.copytree(kit / "samples/projects/starter", project)
    entrance = project / "project.json"
    config = doc(entrance)
    config.update(title="QA Open Dojo", modes=["1v1"])
    config["settings"]["depth_projection"] = 0.067
    save(entrance, config)
    packed_project, packed_rook = temp / "QA Dojo.ogp", temp / "QA Rook.ofc"
    cli("package", project, packed_project)
    cli("package", rook, packed_rook)
    for source in (project, entrance, packed_project, packed_rook, echo):
        cli("validate", source)
        assert isinstance(json.loads(cli("inspect", source)), dict)
    bad = temp / "incomplete"
    shutil.copytree(echo, bad)
    (bad / doc(bad / "manifest.json")["attributes"]).unlink()
    def rejected(source, mods=(), diagnostic=""):
        options = [part for mod in mods for part in ("--mod", mod)]
        log = cli("launch", source, "--runtime", adapter, *options, ok=False)
        assert diagnostic.lower() in log.lower(), log
        assert not marker.exists() and not list(stage.iterdir()), "invalid launch leaked staging or started game"

    rejected(packed_project, (rook, packed_rook), "duplicate character id")
    rejected(packed_project, (rook, bad), "attributes")
    bad_pack = temp / "invalid.ogp"
    bad_pack.write_bytes(b"invalid OGP")
    rejected(bad_pack)
    invalid = temp / "invalid version"
    shutil.copytree(echo, invalid)
    manifest = invalid / "manifest.json"
    save(manifest, {**doc(manifest), "format_version": "2.0"})
    rejected(packed_project, (invalid,), "format_version")
    dependency = [{"id": "missing_demo", "version": "1"}]
    save(manifest, {**doc(manifest), "format_version": "1.0", "dependencies": dependency})
    rejected(packed_project, (invalid,), "missing_demo")
    save(entrance, {**config, "dependencies": dependency})
    rejected(entrance, diagnostic="missing_demo")
    shutil.rmtree(project)
    shutil.rmtree(rook)
    start = time.perf_counter()
    start_wall = time.time_ns()
    output = cli("launch", packed_project, "--runtime", adapter, "--mod", packed_rook, "--mod", echo)
    assert marker.exists() and "OFE_P7_PASS" in output.splitlines(), output
    assert "SCRIPT ERROR" not in output and "ERROR:" not in output, output
    catalog = doc(catalog_copy)
    assert catalog["format"] == "OFE_LAUNCH" and catalog["version"] == 1
    paths = [Path(item["dir"]) for item in catalog["characters"]]
    assert all(path.is_absolute() for path in paths)
    assert paths[2] == echo and echo.is_dir()
    assert not any(path.exists() for path in [Path(catalog_path.read_text()), *paths[:2]]), "staging leak"
    print(output, end="")
    print(f"OFE_P7_PASS_EXTERNAL generated_files={len(actual)} rejected_launches=6 "
          f"launch_to_child_ms={(int(marker.read_text())-start_wall)/1e6:.1f} "
          f"launch_to_exit_ms={(time.perf_counter()-start)*1000:.1f}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("game", help="Godot editor or exported game")
    parser.add_argument("--exported", action="store_true")
    parser.add_argument("--kit", type=Path, help="unpacked portable author kit, defaults to repository")
    parser.add_argument("--pacing", action="store_true", help="measure 600 real graphical frames")
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="ofe p7 qa ") as scratch:
        exercise(args, Path(scratch).resolve())
