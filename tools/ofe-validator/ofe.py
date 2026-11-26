#!/usr/bin/env python3
"""Validate OFE 1.0 development documents and their local resource graph."""

import argparse
import json
import math
import sys
from functools import lru_cache
from pathlib import Path, PureWindowsPath

from jsonschema import Draft202012Validator

FORMATS = Path(__file__).resolve().parents[2] / "formats"
KINDS = ("OFC", "OFA", "OGP", "OFL")
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
    value = float(number)
    if not math.isfinite(value):
        raise ValueError(f"non-finite number {number}")
    return value


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


def validate(path, expected=None, checked=None):
    """Validate a document and all typed references; raise Invalid on failure."""
    if checked is None:
        checked = set()
    path = path.resolve()
    if (path, expected) in checked:
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
            if target_kind == "png":
                with target.open("rb") as stream:
                    if stream.read(8) != b"\x89PNG\r\n\x1a\n":
                        raise Invalid("expected PNG signature")
            elif target_kind != "file":
                validate(target, target_kind, checked)
        except (Invalid, OSError) as exc:
            raise Invalid(f"{path}{pointer(parts)} -> {exc}") from exc
    checked.add((path, expected))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    check = sub.add_parser("validate", help="validate OFC/OFA/OGP/OFL and resource references")
    check.add_argument("document", type=Path)
    args = parser.parse_args()
    try:
        validate(args.document)
    except (Invalid, OSError, RuntimeError) as exc:
        print(exc, file=sys.stderr)
        return 1
    print(f"OK {args.document}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
