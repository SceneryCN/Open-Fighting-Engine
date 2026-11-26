#!/usr/bin/env python3
"""Validate OFE 1.0 development documents and their local resource graph."""

import argparse
import json
import math
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


def validate(path, expected=None, checked=None, files=None, label=None):
    """Validate a document and all typed references; raise Invalid on failure."""
    if checked is None:
        checked = set()
    path = path.resolve()
    if files is not None:
        files[label] = path
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
            if files is not None:
                files[child] = target
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
            elif target_kind != "file":
                validate(target, target_kind, checked, files, child)
        except (Invalid, OSError) as exc:
            raise Invalid(f"{path}{pointer(parts)} -> {exc}") from exc
    checked.add(key)


def character(root):
    files = {}
    manifest_path = resource_path(root, "manifest.json")
    validate(manifest_path, "OFC", files=files, label="manifest.json")
    manifest = read_document(manifest_path)
    info = {k: manifest[k] for k in ("id", "author", "license", "dependencies")}
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
    replay = sub.add_parser("replay", help="verify an OFE_REPLAY simulation or game recording")
    replay.add_argument("source", type=Path)
    replay.add_argument("--simulator", type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.command == "replay":
            return subprocess.run([args.simulator.resolve(), "replay", args.source]).returncode
        if args.command == "validate" and args.source.is_file() and args.source.suffix.lower() != ".ofc":
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
    except (ValueError, OSError, RuntimeError, zipfile.BadZipFile, EOFError, zlib.error) as exc:
        print(exc, file=sys.stderr)
        return 1
    print(f"OK {args.source}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
