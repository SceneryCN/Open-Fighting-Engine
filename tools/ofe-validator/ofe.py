#!/usr/bin/env python3
"""Validate OFE 1.0 development documents and their local resource graph."""

import argparse
import json
import math
import shutil
import tempfile
from contextlib import ExitStack, contextmanager
import sys
import struct
import subprocess
import zipfile
import zlib
from ofc_archive import open_source, write_package
from decimal import Decimal, InvalidOperation
from functools import lru_cache
from pathlib import Path, PureWindowsPath

from jsonschema import Draft202012Validator

FORMATS = Path(__file__).resolve().parents[2] / "formats"
KINDS = ("OFC", "OFA", "OGP", "OFL", "OFE_MOVE_PROFILE")
CHARACTER_PARTS = ("character", "attributes", "move", "boxes")


class Invalid(ValueError):
    """An input document or a resource failed validation."""


def pointer(parts):
    """Escape a path as an RFC 6901 fragment for diagnostic display."""
    return "#" + "".join("/" + str(k).replace("~", "~0").replace("/", "~1") for k in parts)


def unique_object(pairs):
    obj = {}
    for key, value in pairs:
        if key in obj:
            raise ValueError(f"duplicate member {key!r}")
        obj[key] = value
    return obj


def finite_float(number):
    if not math.isfinite(float(number)):
        raise ValueError(f"non-finite number {number}")
    try:
        value = Decimal(number)
    except InvalidOperation as exc:
        raise ValueError(f"invalid number {number}") from exc
    # Draft 2020-12 defines integers by mathematical value. Binary floating
    # point would round near-integers into integers before type checking.
    return int(value) if value == value.to_integral_value() else value


def read_document(path):
    try:
        return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_object,
                          parse_float=finite_float, parse_constant=finite_float)
    except (OSError, ValueError, RecursionError) as exc:
        raise Invalid(f"{path}: {exc}") from exc


@lru_cache(maxsize=None)
def schema_for(kind):
    if kind in CHARACTER_PARTS:
        return schema_for("OFC")["$defs"][kind]
    schema = json.loads((FORMATS / kind.lower() / "schema.json").read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    return schema


def resources(schema, value, root, parts=()):
    """Walk the validated branch and yield annotated local references."""
    if "x-ofe-resource" in schema:
        yield parts, value, schema["x-ofe-resource"]
    if "$ref" in schema:
        target = root["$defs"][schema["$ref"].removeprefix("#/$defs/")]
        yield from resources(target, value, root, parts)
    for branch in schema.get("oneOf", []):
        if Draft202012Validator(branch).is_valid(value):
            yield from resources(branch, value, root, parts)
    for key, subschema in schema.get("properties", {}).items():
        if key in value:
            yield from resources(subschema, value[key], root, (*parts, key))
    if "items" in schema:
        for i, item in enumerate(value):
            yield from resources(schema["items"], item, root, (*parts, i))


def resource_path(base, ref):
    windows = PureWindowsPath(ref)
    if (windows.drive or windows.root or "\\" in ref or ":" in ref
            or any(part in ("", ".", "..") for part in ref.split("/"))
            or any(ord(char) < 32 for char in ref)):
        raise Invalid(f"invalid package-relative path {ref!r}")
    try:
        resolved = (base / ref).resolve(strict=True)
    except (OSError, RuntimeError) as exc:
        raise Invalid(f"unavailable resource {ref!r}: {exc}") from exc
    if not resolved.is_relative_to(base) or not resolved.is_file():
        raise Invalid(f"resource must be a file inside {base}: {ref!r}")
    return resolved


def relations(kind, data):
    if kind == "OFE_MOVE_PROFILE":
        moves = data["moves"]
        ids = {move["id"] for move in moves}
        seen = set()
        for i, move in enumerate(moves):
            if move["id"] in seen:
                yield ("moves", i, "id"), "duplicate move id"
            seen.add(move["id"])
            for j, target in enumerate(move["cancel_into"]):
                if target not in ids or target == move["id"]:
                    yield ("moves", i, "cancel_into", j), "expected a different move in this profile"
            last = -1
            for j, window in enumerate(move["windows"]):
                if window["start"] <= last or window["end"] < window["start"]:
                    yield ("moves", i, "windows", j), "expected ordered non-overlapping windows"
                last = window["end"]
    if kind == "OFA" and data["source"]["kind"] == "sheet":
        cells = data["source"]["columns"] * data["source"]["rows"]
        for i, frame in enumerate(data["frames"]):
            if frame["index"] >= cells:
                yield ("frames", i, "index"), f"sheet index must be below {cells}"
    if kind == "move":
        for i, window in enumerate(data["cancel_windows"]):
            if window["end"] < window["start"]:
                yield ("cancel_windows", i, "end"), "end precedes start"
    if kind == "OFL":
        for screen, elements in data["screens"].items():
            ids = set()
            for i, element in enumerate(elements):
                if element["id"] in ids:
                    yield ("screens", screen, i, "id"), "duplicate element id within screen"
                ids.add(element["id"])


def record_resource(files, label, path):
    if files is not None and files.setdefault(label, path) != path:
        raise Invalid(f"conflicting package resource {label!r}")


def validate(path, expected=None, checked=None, files=None, label=None, on_character=None):
    """Validate a document and all typed references; raise Invalid on failure."""
    if checked is None:
        checked = set()
    path = path.resolve()
    record_resource(files, label, path)
    key = (path, expected, label)
    if key in checked:
        return
    data = read_document(path)
    actual = data.get("format") if isinstance(data, dict) else None
    kind = expected if expected is not None else actual
    if kind not in (*KINDS, *CHARACTER_PARTS):
        raise Invalid(f"{path}#/format: expected one of {KINDS}")
    schema = schema_for(kind)
    failure = next(Draft202012Validator(schema).iter_errors(data), None)
    if failure:
        raise Invalid(f"{path}{pointer(failure.absolute_path)}: {failure.message}")
    for parts, message in relations(kind, data):
        raise Invalid(f"{path}{pointer(parts)}: {message}")
    for parts, ref, target_kind in resources(schema, data, schema):
        try:
            target = resource_path(path.parent, ref)
            child = str(Path(label).parent.joinpath(ref).as_posix()) if label else None
            record_resource(files, child, target)
            if target_kind == "png":
                with target.open("rb") as stream:
                    header = stream.read(33)
                if (len(header) != 33 or header[:16] != b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR"
                        or zlib.crc32(header[12:29]) != int.from_bytes(header[29:33], "big")):
                    raise Invalid("invalid PNG IHDR header")
                width, height = struct.unpack(">II", header[16:24])
                if not (0 < width < 2**31 and 0 < height < 2**31):
                    raise Invalid("invalid PNG dimensions")
                depths = {0: (1, 2, 4, 8, 16), 2: (8, 16), 3: (1, 2, 4, 8), 4: (8, 16), 6: (8, 16)}
                if (header[24] not in depths.get(header[25], ())
                        or header[26:28] != bytes(2) or header[28] > 1):
                    raise Invalid("invalid PNG IHDR encoding")
                if kind == "OFA" and data["source"]["kind"] == "sheet":
                    source = data["source"]
                    if width % source["columns"] or height % source["rows"]:
                        raise Invalid("PNG dimensions must be divisible by sheet columns and rows")
            elif target_kind == "OFC" and on_character is not None:
                on_character(target, child)
            elif target_kind != "file":
                validate(target, target_kind, checked, files, child)
        except (Invalid, OSError) as exc:
            raise Invalid(f"{path}{pointer(parts)} -> {exc}") from exc
    checked.add(key)
    return data


def character(root, manifest_name="manifest.json"):
    files = {}
    manifest_path = resource_path(root, manifest_name)
    manifest = validate(manifest_path, "OFC", files=files, label=manifest_name)
    info = {k: manifest[k] for k in ("id", "author", "license", "dependencies")}
    info["name"] = read_document(files[manifest["character"]])["name"]
    groups = {}
    for key in ("moves", "animations", "hitboxes"):
        entries = [read_document(files[name]) for name in manifest[key]]
        groups[key] = {item["id"]: item for item in entries}
        if len(groups[key]) != len(entries):
            raise Invalid(f"duplicate {key} id")
        info[key] = list(groups[key])
    profile_path = root / "move_profile.json"
    profiles = {}
    if profile_path.exists() or profile_path.is_symlink():
        validate(resource_path(root, "move_profile.json"), "OFE_MOVE_PROFILE",
                 files=files, label="move_profile.json")
        profiles = {m["id"]: m for m in read_document(profile_path)["moves"]}
        if profiles.keys() != groups["moves"].keys():
            raise Invalid("move_profile.json IDs must match manifest moves")
    for move_id, move in groups["moves"].items():
        end = move["startup"] + move["active"]
        profile = profiles.get(move_id, {})
        if any(w["start"] < move["startup"] or w["end"] >= end for w in profile.get("windows", [])):
            raise Invalid(f"{move_id}: hit window outside active frames")
        if any(w["end"] >= end + move["recovery"] for w in move["cancel_windows"]):
            raise Invalid(f"{move_id}: cancel window outside move")
        if bool(profile.get("cancel_into")) != bool(move["cancel_windows"]):
            raise Invalid(f"{move_id}: cancel windows require matching targets")
    return files, info


def project_input(source):
    if source.is_dir():
        return (source / "project.json").exists()
    if source.suffix.lower() in (".ofc", ".ogp"):
        return source.suffix.lower() == ".ogp"
    data = read_document(source)
    return isinstance(data, dict) and data.get("format") == "OGP"


@contextmanager
def open_project(source):
    if source.is_file() and source.suffix.lower() != ".ogp":
        yield source.resolve()
    else:
        with open_source(source, ".ogp") as root:
            yield resource_path(root, "project.json")


def project(entry):
    files = {}
    fighters = []
    ids = set()

    def add_character(manifest, label):
        assets, info = character(manifest.parent, manifest.name)
        if info["id"] in ids:
            raise Invalid(f"duplicate character id {info['id']!r}")
        ids.add(info["id"])
        fighters.append((manifest, assets, info))
        for name, path in assets.items():
            if name == manifest.name:
                continue  # Already recorded by the OGP resource walker.
            key = (Path(label).parent / name).as_posix()
            record_resource(files, key, path)

    data = validate(entry, "OGP", files=files, label="project.json", on_character=add_character)
    return files, data, fighters


def launch(entry, mods, runtime):
    _, data, fighters = project(entry)
    with ExitStack() as stack:
        scratch = Path(stack.enter_context(tempfile.TemporaryDirectory())).resolve()
        for source in mods:
            root = stack.enter_context(open_source(source))
            files, info = character(root)
            fighters.append((root / "manifest.json", files, info))

        roster = []
        ids = set()
        for owner in [data, *(info for _, _, info in fighters)]:
            if owner["dependencies"]:
                unresolved = ", ".join(f"{d['id']} {d['version']}" for d in owner["dependencies"])
                raise Invalid(f"{owner['id']}: unresolved dependency: {unresolved}")
        for manifest, files, info in fighters:
            identifier = info["id"]
            if identifier in ids:
                raise Invalid(f"duplicate character id {identifier!r}")
            ids.add(identifier)
            directory = manifest.parent
            if manifest.name != "manifest.json":
                # Rust accepts character roots with a canonical manifest name.
                directory = scratch / f"character-{len(roster)}"
                for name, source in files.items():
                    target = directory / ("manifest.json" if name == manifest.name else name)
                    if target.exists():
                        raise Invalid("character resource conflicts with manifest.json")
                    target.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(source, target)
            roster.append({"id": identifier, "name": info["name"], "dir": str(directory)})

        catalog = scratch / "catalog.json"
        catalog.write_text(json.dumps({
            "format": "OFE_LAUNCH", "version": 1, "title": data["title"], "modes": data["modes"],
            "depth_projection": float(data["settings"]["depth_projection"]), "characters": roster,
        }, ensure_ascii=False, allow_nan=False), encoding="utf-8")
        return subprocess.run([runtime.resolve(), "--", f"--ofe-project={catalog}"]).returncode


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    for command in ("validate", "inspect", "run", "package"):
        cmd = sub.add_parser(command)
        cmd.add_argument("source", type=Path)
        if command == "package":
            cmd.add_argument("output", type=Path)
        if command == "run":
            cmd.add_argument("--simulator", type=Path, required=True)
            cmd.add_argument("--script", type=Path)
    launcher = sub.add_parser("launch", help="play an OGP project with optional external characters")
    launcher.add_argument("source", type=Path)
    launcher.add_argument("--runtime", type=Path, required=True)
    launcher.add_argument("--mod", type=Path, action="append", default=[])
    replay = sub.add_parser("replay", help="verify an OFE_REPLAY simulation or game recording")
    replay.add_argument("source", type=Path)
    replay.add_argument("--simulator", type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.command == "replay":
            return subprocess.run([args.simulator.resolve(), "replay", args.source]).returncode
        if args.command == "launch":
            with open_project(args.source) as entry:
                return launch(entry, args.mod, args.runtime)
        if args.command != "run" and project_input(args.source):
            if args.command == "package" and args.source.is_file() and args.source.suffix.lower() == ".ogp":
                raise Invalid("package expects a project directory or JSON entry")
            with open_project(args.source) as entry:
                files, data, fighters = project(entry)
                if args.command == "inspect":
                    info = dict(data)
                    info["characters"] = [fighter for _, _, fighter in fighters]
                    print(json.dumps(info, ensure_ascii=False, default=float))
                    return 0
                if args.command == "package":
                    write_package(files, args.output, ".ogp")
        elif args.command == "validate" and args.source.is_file() and args.source.suffix.lower() != ".ofc":
            validate(args.source)
        else:
            if args.command == "package" and not args.source.is_dir():
                raise Invalid("package expects a character directory")
            with open_source(args.source) as root:
                files, info = character(root)
                if args.command == "inspect":
                    print(json.dumps(info, ensure_ascii=False))
                    return 0
                if args.command == "package":
                    write_package(files, args.output)
                if args.command == "run":
                    command = [args.simulator.resolve(), "--character", root]
                    if args.script:
                        command += ["--script", args.script]
                    return subprocess.run(command).returncode
    except (ValueError, OverflowError, OSError, RuntimeError, zipfile.BadZipFile, EOFError, zlib.error) as exc:
        print(exc, file=sys.stderr)
        return 1
    print(f"OK {args.source}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
