"""Build and audit compact public evaluation archives.

Publication bundles use a small allowlist of aggregate files and a constrained,
deterministic tar format. The same contract is checked after construction and
when tracked bundles are tested.
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
import zlib
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
MAX_TAR_BYTES = MAX_TOTAL_BYTES + (MAX_MEMBERS + 20) * 1024
SOURCE_PATH = re.compile(
    br"(?i)(?<![A-Za-z0-9_.-])sources[/\\][A-Za-z0-9_.-]+"
)
DIFF_LINE = re.compile(br"(?m)^(?:diff --git |--- a/|\+\+\+ b/|@@ )")
MOVE_SOURCE_LINE = re.compile(
    br"(?i)(?<![A-Za-z0-9_])(?:"
    br"(?:spec[ \t]+)?module[ \t]+(?:0x[0-9a-f]+|[A-Za-z_]\w*)::[A-Za-z_]\w*"
    br"|(?:(?:public(?:\([^\r\n)]*\))?|entry|native)[ \t]+)*"
    br"fun[ \t]+[A-Za-z_]\w*[ \t]*(?:<[^\r\n>]*>[ \t]*)?\("
    br"|(?:struct|enum)[ \t]+[A-Za-z_]\w*(?:[ \t]*<[^\r\n>]*>)?"
    br"[ \t]*(?:has[ \t]+[^\r\n{]+)?[ \t]*\{"
    br")"
)
_ZERO_BLOCK = b"\0" * 512
_EXTENDED_TAR_TYPES = {b"g", b"x", b"L", b"K", b"S"}


class PublicationError(ValueError):
    """A publication bundle violates the artifact contract."""


def build_public_archive(source: Path, output: Path, archive_name: str) -> None:
    """Create a deterministic archive from aggregate files only."""
    _validate_archive_name(archive_name)
    files = _publication_files(source)
    if not files:
        raise PublicationError(f"{source}: no public aggregate files")
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
    """Reject archives that do not satisfy the publication contract."""
    _validate_gzip_container(path)
    _preflight_tar(path)
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
            roots.add(member_path.parts[0])
            if member.isdir():
                if len(member_path.parts) != 1:
                    raise PublicationError(
                        f"{path}: publication archives cannot contain directories: {member.name}"
                    )
                continue
            if not member.isfile():
                raise PublicationError(
                    f"{path}: publication archives cannot contain links or special files: "
                    f"{member.name}"
                )
            file_count += 1
            if file_count > MAX_MEMBERS:
                raise PublicationError(f"{path}: archive has more than {MAX_MEMBERS} files")
            if len(member_path.parts) != 2:
                raise PublicationError(
                    f"{path}: only top-level aggregate files may be published: {member.name}"
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
        raise PublicationError(f"{path}: archive contains no public aggregate files")
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
        if entry.stat().st_size > MAX_MEMBER_BYTES:
            raise PublicationError(f"{entry}: file exceeds {MAX_MEMBER_BYTES} bytes")
        total_bytes += entry.stat().st_size
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
    if size > MAX_MEMBER_BYTES:
        raise PublicationError(f"{name}: file exceeds {MAX_MEMBER_BYTES} bytes")
    data = stream.read(MAX_MEMBER_BYTES + 1)
    if len(data) != size:
        raise PublicationError(f"{name}: declared size does not match content")
    if SOURCE_PATH.search(data):
        raise PublicationError(f"{name}: contains a disallowed source path")
    if DIFF_LINE.search(data):
        raise PublicationError(f"{name}: contains unified-diff source content")
    if MOVE_SOURCE_LINE.search(data):
        raise PublicationError(f"{name}: contains Move source content")
    return data, hashlib.sha256(data).hexdigest()


def _preflight_tar(path: Path) -> None:
    """Bound raw tar parsing and reject extension records before ``tarfile``."""
    raw_bytes = 0
    payload_bytes = 0
    members = 0
    saw_end = False
    try:
        with gzip.open(path, "rb") as stream:
            while True:
                block = stream.read(512)
                if not block:
                    break
                raw_bytes += len(block)
                if raw_bytes > MAX_TAR_BYTES:
                    raise PublicationError(
                        f"{path}: expanded tar stream exceeds {MAX_TAR_BYTES} bytes"
                    )
                if len(block) != 512:
                    raise PublicationError(f"{path}: truncated tar header")
                if block == _ZERO_BLOCK:
                    saw_end = True
                    _check_zero_padding(path, stream, raw_bytes)
                    break

                members += 1
                if members > MAX_MEMBERS + 1:
                    raise PublicationError(
                        f"{path}: archive has more than {MAX_MEMBERS} files"
                    )
                kind = block[156:157] or b"\0"
                if kind in _EXTENDED_TAR_TYPES:
                    raise PublicationError(
                        f"{path}: archive uses unsupported extended metadata"
                    )
                if kind != b"0":
                    raise PublicationError(
                        f"{path}: archive contains an unsupported member type"
                    )
                size = _parse_tar_size(path, block[124:136])
                _check_canonical_tar_header(path, block, size)
                if size > MAX_MEMBER_BYTES:
                    raise PublicationError(
                        f"{path}: tar member exceeds {MAX_MEMBER_BYTES} bytes"
                    )
                payload_bytes += size
                if payload_bytes > MAX_TOTAL_BYTES:
                    raise PublicationError(
                        f"{path}: expanded archive exceeds {MAX_TOTAL_BYTES} bytes"
                    )
                padded_size = ((size + 511) // 512) * 512
                raw_bytes += padded_size
                if raw_bytes > MAX_TAR_BYTES:
                    raise PublicationError(
                        f"{path}: expanded tar stream exceeds {MAX_TAR_BYTES} bytes"
                    )
                _discard_exact(path, stream, size)
                _check_member_padding(path, stream, padded_size - size)
    except PublicationError:
        raise
    except (EOFError, OSError) as error:
        raise PublicationError(f"{path}: invalid gzip tar archive: {error}") from error
    if not saw_end:
        raise PublicationError(f"{path}: tar stream is missing its end marker")


def _validate_gzip_container(path: Path) -> None:
    """Require one metadata-free gzip member and bound decompression."""
    try:
        with path.open("rb") as stream:
            header = stream.read(10)
            if len(header) != 10 or header[:3] != b"\x1f\x8b\x08":
                raise PublicationError(f"{path}: invalid gzip header")
            if header[3] != 0:
                raise PublicationError(f"{path}: unsupported gzip header metadata")
            stream.seek(0)
            decoder = zlib.decompressobj(16 + zlib.MAX_WBITS)
            expanded = 0
            while chunk := stream.read(64 * 1024):
                pending = chunk
                while pending:
                    output = decoder.decompress(pending, 1024 * 1024)
                    expanded += len(output)
                    if expanded > MAX_TAR_BYTES:
                        raise PublicationError(
                            f"{path}: expanded tar stream exceeds {MAX_TAR_BYTES} bytes"
                        )
                    pending = decoder.unconsumed_tail
                    if decoder.eof:
                        if decoder.unused_data or stream.read(1):
                            raise PublicationError(
                                f"{path}: archive must contain a single gzip member"
                            )
                        return
            raise PublicationError(f"{path}: truncated gzip stream")
    except PublicationError:
        raise
    except (OSError, zlib.error) as error:
        raise PublicationError(f"{path}: invalid gzip archive: {error}") from error


def _parse_tar_size(path: Path, field: bytes) -> int:
    stripped = field.strip(b" \0")
    if not stripped:
        return 0
    if any(byte < ord("0") or byte > ord("7") for byte in stripped):
        raise PublicationError(f"{path}: tar member has a non-octal size")
    return int(stripped, 8)


def _check_canonical_tar_header(path: Path, block: bytes, size: int) -> None:
    name_bytes, separator, padding = block[:100].partition(b"\0")
    if not separator or padding.strip(b"\0"):
        raise PublicationError(f"{path}: non-canonical tar header")
    try:
        name = name_bytes.decode("ascii")
    except UnicodeDecodeError as error:
        raise PublicationError(f"{path}: non-canonical tar header") from error
    info = tarfile.TarInfo(name)
    info.size = size
    info.mode = 0o644
    info.mtime = 0
    info.uid = 0
    info.gid = 0
    info.uname = ""
    info.gname = ""
    try:
        expected = info.tobuf(format=tarfile.USTAR_FORMAT)
    except (UnicodeError, ValueError) as error:
        raise PublicationError(f"{path}: non-canonical tar header") from error
    if block != expected:
        raise PublicationError(f"{path}: non-canonical tar header")


def _discard_exact(path: Path, stream: BinaryIO, size: int) -> None:
    remaining = size
    while remaining:
        chunk = stream.read(min(remaining, 1024 * 1024))
        if not chunk:
            raise PublicationError(f"{path}: truncated tar member")
        remaining -= len(chunk)


def _check_member_padding(path: Path, stream: BinaryIO, size: int) -> None:
    if not size:
        return
    padding = stream.read(size)
    if len(padding) != size:
        raise PublicationError(f"{path}: truncated tar member padding")
    if padding.strip(b"\0"):
        raise PublicationError(f"{path}: nonzero tar padding")


def _check_zero_padding(path: Path, stream: BinaryIO, raw_bytes: int) -> None:
    while True:
        chunk = stream.read(1024 * 1024)
        if not chunk:
            return
        raw_bytes += len(chunk)
        if raw_bytes > MAX_TAR_BYTES:
            raise PublicationError(
                f"{path}: expanded tar stream exceeds {MAX_TAR_BYTES} bytes"
            )
        if chunk.strip(b"\0"):
            raise PublicationError(f"{path}: data follows the tar end marker")


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
    scan = subparsers.add_parser("scan", help="audit public archives")
    scan.add_argument("archives", nargs="+", type=Path)
    build = subparsers.add_parser("build", help="build a public aggregate archive")
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
