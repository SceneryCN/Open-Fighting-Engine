"""Portable .ofc ZIP transport for a validated OFC resource graph."""
from contextlib import contextmanager
from pathlib import Path, PureWindowsPath
import os
import stat
import tempfile
import zipfile

MAX_FILES = 4096
MAX_FILE = 64 * 1024 * 1024
MAX_TOTAL = 256 * 1024 * 1024


def portable_parts(name):
    parts = name.split("/")
    for part in parts:
        if (not part or part in (".", "..") or part.endswith((".", " "))
                or any(c in '<>:"\\|?*' or ord(c) < 32 for c in part)
                or PureWindowsPath(part).is_reserved()):
            raise ValueError(f"invalid archive path {name!r}")
    return parts


class Names:
    def __init__(self):
        self.entries = {}
        self.members = set()

    def add(self, name, directory=False):
        parts = portable_parts(name)
        if name in self.members:
            raise ValueError(f"duplicate archive member {name!r}")
        self.members.add(name)
        for index in range(1, len(parts) + 1):
            path = "/".join(parts[:index])
            expected = (path, directory or index < len(parts))
            if self.entries.setdefault(path.casefold(), expected) != expected:
                raise ValueError(f"conflicting archive path {path!r}")


def copy_limited(source, target, remaining):
    written = 0
    while chunk := source.read(64 * 1024):
        written += len(chunk)
        if written > min(MAX_FILE, remaining):
            raise ValueError("character archive exceeds size limit")
        target.write(chunk)
    return written


@contextmanager
def open_source(path):
    if path.is_dir():
        yield path.resolve()
        return
    if path.suffix.lower() != ".ofc":
        raise ValueError("expected a character directory or .ofc archive")
    with tempfile.TemporaryDirectory() as tmp, zipfile.ZipFile(path) as archive:
        root = Path(tmp).resolve()
        members = archive.infolist()
        if len(members) > MAX_FILES:
            raise ValueError("too many character archive members")
        total = 0
        names = Names()
        for item in members:
            directory = item.is_dir()
            name = item.orig_filename.removesuffix("/") if directory else item.orig_filename
            names.add(name, directory)
            mode = stat.S_IFMT(item.external_attr >> 16)
            if (mode not in (0, stat.S_IFDIR if directory else stat.S_IFREG)
                    or item.compress_type not in (zipfile.ZIP_STORED, zipfile.ZIP_DEFLATED)
                    or item.flag_bits & 1):
                raise ValueError(f"unsupported archive member {name!r}")
            if item.file_size > min(MAX_FILE, MAX_TOTAL - total) or (directory and item.file_size):
                raise ValueError(f"invalid member size for {name!r}")
            dest = root / name
            if directory:
                dest.mkdir(parents=True, exist_ok=True)
                continue
            dest.parent.mkdir(parents=True, exist_ok=True)
            with archive.open(item) as src, dest.open("xb") as out:
                total += copy_limited(src, out, MAX_TOTAL - total)
        yield root


def write_package(files, output):
    if output.suffix.lower() != ".ofc" or output.resolve() in files.values():
        raise ValueError("output must be a separate .ofc file")
    if len(files) > MAX_FILES:
        raise ValueError("too many character resources")
    names = Names()
    with tempfile.NamedTemporaryFile(dir=output.parent, delete=False) as handle:
        temporary = Path(handle.name)
    try:
        with zipfile.ZipFile(temporary, "w") as archive:
            total = 0
            for name, path in sorted(files.items()):
                names.add(name)
                entry = zipfile.ZipInfo(name)
                entry.compress_type = zipfile.ZIP_DEFLATED
                entry.external_attr = (stat.S_IFREG | 0o644) << 16
                with path.open("rb") as src, archive.open(entry, "w") as dest:
                    total += copy_limited(src, dest, MAX_TOTAL - total)
        os.replace(temporary, output)
    finally:
        temporary.unlink(missing_ok=True)
