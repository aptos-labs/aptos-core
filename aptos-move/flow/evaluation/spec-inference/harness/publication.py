"""Build and validate compact evaluation result archives.

Publication bundles are constructed from a small allowlist of aggregate files.
Raw run artifacts such as workspace diffs, transcripts, and event streams are
not valid inputs or archive members.
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import re
import sys
import tarfile
import tempfile
from pathlib import Path, PurePosixPath
from typing import BinaryIO, Iterable


PUBLIC_AGGREGATE_FILES = frozenset(
    {
        "CORPUS.json",
        "DEBRIEF.md",
        "REPORT.md",
        "analysis.json",
        "apparatus.json",
        "attempts.csv",
        "audit.json",
        "cells.csv",
        "config.json",
        "corpus-manifest.json",
        "cost-summary.json",
        "final-status.json",
        "final-summary.json",
        "mined.json",
        "mined.md",
        "mutation-summary.json",
        "pilot-manifest.json",
        "plugins.json",
        "preflight.json",
        "preparation.json",
        "pricing.json",
        "queries.csv",
        "requests.csv",
        "run-conditions.json",
        "sdk-sessions.csv",
        "status.md",
        "summary.json",
        "taxonomy.json",
        "taxonomy.md",
        "token-cost-summary.md",
        "turns.csv",
    }
)
CHECKSUM_FILE = "SHA256SUMS"
MAX_MEMBERS = 256
MAX_MEMBER_BYTES = 64 * 1024 * 1024
MAX_TOTAL_BYTES = 256 * 1024 * 1024
DIFF_LINE = re.compile(br"(?m)^(?:diff --git |--- a/|\+\+\+ b/|@@ )")


class PublicationError(ValueError):
    """An evaluation archive violates the publication format."""


def build_public_archive(source: Path, output: Path, archive_name: str) -> None:
    """Create a deterministic archive from allowlisted aggregate files."""
    _validate_archive_name(archive_name)
    files = _publication_files(source)
    if not files:
        raise PublicationError(f"{source}: no aggregate files")
    for path in files:
        with path.open("rb") as stream:
            _scan_file(stream, path.stat().st_size, path.name)

    checksums = "".join(
        f"{_sha256_path(path)}  {path.name}\n" for path in files
    ).encode("utf-8")
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(
        prefix=f".{output.name}.", suffix=".tmp", dir=output.parent, delete=False
    ) as temporary:
        temporary_path = Path(temporary.name)
    try:
        with temporary_path.open("wb") as raw:
            with gzip.GzipFile(fileobj=raw, mode="wb", filename="", mtime=0) as compressed:
                with tarfile.open(
                    fileobj=compressed, mode="w", format=tarfile.USTAR_FORMAT
                ) as archive:
                    for path in files:
                        with path.open("rb") as stream:
                            _add_file(
                                archive,
                                f"{archive_name}/{path.name}",
                                stream,
                                path.stat().st_size,
                            )
                    _add_file(
                        archive,
                        f"{archive_name}/{CHECKSUM_FILE}",
                        io.BytesIO(checksums),
                        len(checksums),
                    )
        scan_public_archive(temporary_path)
        temporary_path.replace(output)
    except Exception:
        temporary_path.unlink(missing_ok=True)
        raise


def scan_public_archive(path: Path) -> None:
    """Validate an archive's members, bounds, checksums, and structure."""
    seen: set[str] = set()
    roots: set[str] = set()
    digests: dict[str, str] = {}
    checksum_data: bytes | None = None
    total_bytes = 0
    file_count = 0
    try:
        archive = tarfile.open(path, mode="r:gz")
    except (OSError, tarfile.TarError) as error:
        raise PublicationError(f"{path}: invalid gzip tar archive: {error}") from error
    with archive:
        for member in archive:
            member_path = _safe_member_path(path, member.name)
            normalized_name = member_path.as_posix()
            if normalized_name in seen:
                raise PublicationError(f"{path}: duplicate archive member {member.name!r}")
            seen.add(normalized_name)
            root = member_path.parts[0]
            try:
                _validate_archive_name(root)
            except PublicationError as error:
                raise PublicationError(f"{path}: invalid archive root: {error}") from error
            roots.add(root)
            if member.isdir():
                if len(member_path.parts) != 1:
                    raise PublicationError(
                        f"{path}: archives cannot contain nested directories: {member.name}"
                    )
                continue
            if not member.isfile():
                raise PublicationError(
                    f"{path}: archives cannot contain links or special files: "
                    f"{member.name}"
                )
            file_count += 1
            if file_count > MAX_MEMBERS:
                raise PublicationError(f"{path}: archive has more than {MAX_MEMBERS} files")
            if len(member_path.parts) != 2:
                raise PublicationError(
                    f"{path}: only top-level aggregate files may be published: "
                    f"{member.name}"
                )
            name = member_path.name
            if name not in PUBLIC_AGGREGATE_FILES and name != CHECKSUM_FILE:
                raise PublicationError(f"{path}: forbidden publication artifact: {member.name}")
            total_bytes += member.size
            if total_bytes > MAX_TOTAL_BYTES:
                raise PublicationError(
                    f"{path}: expanded archive exceeds {MAX_TOTAL_BYTES} bytes"
                )
            stream = archive.extractfile(member)
            if stream is None:
                raise PublicationError(f"{path}: cannot read archive member {member.name}")
            with stream:
                data, digest = _scan_file(stream, member.size, member.name)
            if name == CHECKSUM_FILE:
                checksum_data = data
            else:
                digests[name] = digest
    if len(roots) != 1:
        raise PublicationError(f"{path}: archive must have exactly one root directory")
    if checksum_data is None:
        raise PublicationError(f"{path}: archive is missing {CHECKSUM_FILE}")
    if not digests:
        raise PublicationError(f"{path}: archive contains no aggregate files")
    _check_checksums(path, checksum_data, digests)


def _publication_files(source: Path) -> list[Path]:
    if not source.is_dir():
        raise PublicationError(f"{source}: publication source must be a directory")
    entries = sorted(source.iterdir(), key=lambda path: path.name)
    files: list[Path] = []
    total_bytes = 0
    for entry in entries:
        if entry.name == CHECKSUM_FILE:
            continue
        if entry.is_symlink() or not entry.is_file():
            raise PublicationError(f"{source}: unexpected publication input {entry.name!r}")
        if entry.name not in PUBLIC_AGGREGATE_FILES:
            raise PublicationError(f"{source}: forbidden publication artifact {entry.name!r}")
        size = entry.stat().st_size
        if size > MAX_MEMBER_BYTES:
            raise PublicationError(f"{entry}: file exceeds {MAX_MEMBER_BYTES} bytes")
        total_bytes += size
        if total_bytes > MAX_TOTAL_BYTES:
            raise PublicationError(
                f"{source}: publication input exceeds {MAX_TOTAL_BYTES} bytes"
            )
        files.append(entry)
    return files


def _validate_archive_name(name: str) -> None:
    if name in (".", "..") or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]*", name):
        raise PublicationError("archive name must be a safe path component")


def _safe_member_path(archive: Path, name: str) -> PurePosixPath:
    if "\\" in name:
        raise PublicationError(f"{archive}: archive member uses a backslash: {name!r}")
    path = PurePosixPath(name)
    if (
        path.is_absolute()
        or not path.parts
        or any(part in ("", ".", "..") for part in path.parts)
    ):
        raise PublicationError(f"{archive}: unsafe archive member path: {name!r}")
    return path


def _scan_file(stream: BinaryIO, size: int, name: str) -> tuple[bytes, str]:
    if size < 0 or size > MAX_MEMBER_BYTES:
        raise PublicationError(f"{name}: file exceeds {MAX_MEMBER_BYTES} bytes")
    data = stream.read(MAX_MEMBER_BYTES + 1)
    if len(data) != size:
        raise PublicationError(f"{name}: declared size does not match content")
    if DIFF_LINE.search(data):
        raise PublicationError(f"{name}: contains unified-diff content")
    return data, hashlib.sha256(data).hexdigest()


def _sha256_path(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _add_file(
    archive: tarfile.TarFile, name: str, stream: BinaryIO, size: int
) -> None:
    info = tarfile.TarInfo(name)
    info.size = size
    info.mode = 0o644
    info.mtime = 0
    info.uid = 0
    info.gid = 0
    info.uname = ""
    info.gname = ""
    archive.addfile(info, stream)


def _check_checksums(path: Path, data: bytes, digests: dict[str, str]) -> None:
    try:
        text = data.decode("ascii")
    except UnicodeDecodeError as error:
        raise PublicationError(f"{path}: {CHECKSUM_FILE} is not ASCII") from error
    expected: dict[str, str] = {}
    for line in text.splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  ([A-Za-z0-9._-]+)", line)
        if match is None:
            raise PublicationError(f"{path}: malformed {CHECKSUM_FILE}")
        digest, name = match.groups()
        if name in expected:
            raise PublicationError(f"{path}: duplicate checksum for {name}")
        expected[name] = digest
    if expected != digests:
        raise PublicationError(f"{path}: {CHECKSUM_FILE} does not match archive contents")


def _parse_args(argv: Iterable[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    scan = subparsers.add_parser("scan", help="validate result archives")
    scan.add_argument("archives", nargs="+", type=Path)
    build = subparsers.add_parser("build", help="build an aggregate result archive")
    build.add_argument("--source", required=True, type=Path)
    build.add_argument("--output", required=True, type=Path)
    build.add_argument("--name", required=True)
    return parser.parse_args(argv)


def main(argv: Iterable[str] | None = None) -> int:
    args = _parse_args(argv)
    try:
        if args.command == "scan":
            for archive in args.archives:
                scan_public_archive(archive)
        else:
            build_public_archive(args.source, args.output, args.name)
    except PublicationError as error:
        print(error, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
