"""Build and audit compact public evaluation archives.

Publication bundles use a small allowlist of aggregate files and a constrained,
deterministic tar format. The same contract is checked after construction and
when tracked bundles are tested.
"""

from __future__ import annotations

import argparse
import codecs
import gzip
import hashlib
import io
import re
import sys
import tarfile
import tempfile
import zlib
from html.entities import html5 as HTML_ENTITIES
from pathlib import Path, PurePosixPath
from typing import BinaryIO, Iterable, Iterator


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
MOVE_PUNCTUATION = {
    ord("{"): b"{",
    ord("}"): b"}",
    ord("("): b"(",
    ord(")"): b")",
    ord("<"): b"<",
    ord(">"): b">",
    ord(","): b",",
    ord(";"): b";",
    ord(":"): b":",
    ord("="): b"=",
    ord('"'): b'"',
}
JSON_SIMPLE_ESCAPES = {
    ord('"'): ord('"'),
    ord("\\"): ord("\\"),
    ord("/"): ord("/"),
    ord("b"): ord("\b"),
    ord("f"): ord("\f"),
    ord("n"): ord("\n"),
    ord("r"): ord("\r"),
    ord("t"): ord("\t"),
}
MAX_HTML_ENTITY_NAME_BYTES = max(len(name) for name in HTML_ENTITIES)
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


def scan_public_archive(
    path: Path, *, max_total_bytes: int = MAX_TOTAL_BYTES
) -> int:
    """Reject archives that do not satisfy the publication contract."""
    max_total_bytes = min(max_total_bytes, MAX_TOTAL_BYTES)
    if max_total_bytes < 0:
        raise PublicationError("archive byte budget cannot be negative")
    max_tar_bytes = min(
        max_total_bytes + (MAX_MEMBERS + 20) * 1024, MAX_TAR_BYTES
    )
    _validate_gzip_container(path, max_tar_bytes)
    _preflight_tar(path, max_total_bytes, max_tar_bytes)
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
            if total_bytes > max_total_bytes:
                raise PublicationError(
                    f"{path}: expanded archive exceeds {max_total_bytes} bytes"
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
    return total_bytes


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
    for filename in (*PUBLIC_AGGREGATE_FILES, CHECKSUM_FILE):
        try:
            tarfile.TarInfo(f"{name}/{filename}").tobuf(format=tarfile.USTAR_FORMAT)
        except ValueError as error:
            raise PublicationError(
                "archive name is too long for the publication format"
            ) from error


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
    _validate_utf8(data, name)
    _check_content(data, name)
    decoded = _decode_structured_content(data)
    if decoded is not data:
        _check_content(decoded, name)
    return data, hashlib.sha256(data).hexdigest()


def _validate_utf8(data: bytes, name: str) -> None:
    decoder = codecs.getincrementaldecoder("utf-8")("strict")
    try:
        view = memoryview(data)
        for offset in range(0, len(data), 1024 * 1024):
            decoder.decode(view[offset : offset + 1024 * 1024], final=False)
        decoder.decode(b"", final=True)
    except UnicodeDecodeError as error:
        raise PublicationError(f"{name}: publication content is not UTF-8") from error


def _check_content(data: bytes, name: str) -> None:
    if SOURCE_PATH.search(data):
        raise PublicationError(f"{name}: contains a disallowed source path")
    if DIFF_LINE.search(data):
        raise PublicationError(f"{name}: contains unified-diff source content")
    if _contains_move_source(data):
        raise PublicationError(f"{name}: contains Move source content")


def _decode_json_escapes(data: bytes) -> bytes:
    return _decode_encoded_content(data, decode_json=True, decode_html=False)


def _decode_html_entities(data: bytes) -> bytes:
    return _decode_encoded_content(data, decode_json=False, decode_html=True)


def _decode_structured_content(data: bytes) -> bytes:
    return _decode_encoded_content(data, decode_json=True, decode_html=True)


def _decode_encoded_content(
    data: bytes, *, decode_json: bool, decode_html: bool
) -> bytes:
    if not ((decode_json and b"\\" in data) or (decode_html and b"&" in data)):
        return data
    output = bytearray()
    entity_offsets = bytearray()
    entity_start: int | None = None
    index = 0
    changed = False
    while index < len(data):
        decoded = None
        if decode_json and data[index] == ord("\\"):
            json_decoded = _decode_json_escape_body(data, index + 1)
            if json_decoded is not None:
                replacement, end = json_decoded
                decoded = bytes((replacement,)), end
        elif decode_html and data[index] == ord("&"):
            decoded = _decode_html_entity_body(data, index + 1)
        if decoded is None:
            replacement = bytes((data[index],))
            index += 1
        else:
            replacement, index = decoded
            changed = True
        entity_start = _append_encoded_replacement(
            output, entity_offsets, entity_start, replacement
        )

        while True:
            if decode_json:
                json_suffix = _decode_json_escape_suffix(output)
                if json_suffix is not None:
                    escape_start, replacement = json_suffix
                    del output[escape_start:]
                    entity_start = _append_encoded_replacement(
                        output,
                        entity_offsets,
                        entity_start,
                        bytes((replacement,)),
                    )
                    changed = True
                    continue
            if decode_html and output[-1:] == b";" and entity_start is not None:
                html_suffix = _decode_html_entity_body(output, entity_start + 1)
                if html_suffix is not None and html_suffix[1] == len(output):
                    replacement, _ = html_suffix
                    del output[entity_start:]
                    entity_start = _pop_entity_offset(
                        entity_offsets, entity_start
                    )
                    entity_start = _append_encoded_replacement(
                        output, entity_offsets, entity_start, replacement
                    )
                    changed = True
                    continue
            break
    return bytes(output) if changed else data


def _decode_json_escape_body(data: bytes, index: int) -> tuple[int, int] | None:
    if index >= len(data):
        return None
    escaped = data[index]
    if escaped == ord("u") and index + 5 <= len(data):
        digits = data[index + 1 : index + 5]
        if all(_is_ascii_hex(byte) for byte in digits):
            codepoint = int(digits, 16)
            return (codepoint if codepoint <= 0x7F else ord(" "), index + 5)
    replacement = JSON_SIMPLE_ESCAPES.get(escaped)
    if replacement is None:
        return None
    return replacement, index + 1


def _decode_json_escape_suffix(data: bytearray) -> tuple[int, int] | None:
    if len(data) >= 2 and data[-2] == ord("\\"):
        replacement = JSON_SIMPLE_ESCAPES.get(data[-1])
        if replacement is not None:
            return len(data) - 2, replacement
    if (
        len(data) >= 6
        and data[-6] == ord("\\")
        and data[-5] == ord("u")
        and all(_is_ascii_hex(digit) for digit in data[-4:])
    ):
        codepoint = int(bytes(data[-4:]), 16)
        return len(data) - 6, codepoint if codepoint <= 0x7F else ord(" ")
    return None


def _append_encoded_replacement(
    output: bytearray,
    entity_offsets: bytearray,
    entity_start: int | None,
    replacement: bytes,
) -> int | None:
    offset = len(output)
    output.extend(replacement)
    if b"&" not in replacement:
        return entity_start
    for relative in _find_ampersands(replacement):
        position = offset + relative
        delta = position + 1 if entity_start is None else position - entity_start
        _push_offset_delta(entity_offsets, delta)
        entity_start = position
    return entity_start


def _find_ampersands(data: bytes) -> Iterator[int]:
    index = data.find(b"&")
    while index >= 0:
        yield index
        index = data.find(b"&", index + 1)


def _push_offset_delta(encoded: bytearray, delta: int) -> None:
    while delta >= 0x80:
        encoded.append((delta & 0x7F) | 0x80)
        delta >>= 7
    encoded.append(delta)


def _pop_entity_offset(encoded: bytearray, current: int) -> int | None:
    end = len(encoded)
    start = end - 1
    while start > 0 and encoded[start - 1] & 0x80:
        start -= 1
    delta = 0
    shift = 0
    for byte in encoded[start:end]:
        delta |= (byte & 0x7F) << shift
        shift += 7
    del encoded[start:end]
    previous = current - delta
    return previous if previous >= 0 else None


def _decode_html_entity_body(
    data: bytes | bytearray, index: int
) -> tuple[bytes, int] | None:
    if index >= len(data):
        return None
    if data[index] == ord("#"):
        index += 1
        hexadecimal = index < len(data) and data[index] in (ord("x"), ord("X"))
        if hexadecimal:
            index += 1
        digits_start = index
        predicate = _is_ascii_hex if hexadecimal else _is_ascii_digit
        while index < len(data) and predicate(data[index]):
            index += 1
        if index == digits_start or index >= len(data) or data[index] != ord(";"):
            return None
        digits = bytes(data[digits_start:index]).lstrip(b"0") or b"0"
        max_digits = 2 if hexadecimal else 3
        codepoint = (
            int(digits, 16 if hexadecimal else 10)
            if len(digits) <= max_digits
            else 128
        )
        replacement = codepoint if codepoint <= 0x7F else ord(" ")
        return bytes((replacement,)), index + 1

    entity_end = data.find(
        b";", index, min(len(data), index + MAX_HTML_ENTITY_NAME_BYTES)
    )
    if entity_end < 0:
        return None
    try:
        name = bytes(data[index : entity_end + 1]).decode("ascii")
    except UnicodeDecodeError:
        return None
    value = HTML_ENTITIES.get(name)
    if value is None:
        return None
    replacement = bytes(
        ord(character) if ord(character) <= 0x7F else ord(" ")
        for character in value
    )
    return replacement, entity_end + 1


def _contains_move_source(data: bytes) -> bool:
    return _move_tokens_contain_declaration(
        _iter_move_tokens(data, skip_comments=False)
    ) or _move_tokens_contain_declaration(
        _iter_move_tokens(data, skip_comments=True)
    ) or _move_tokens_contain_body(_iter_move_tokens(data, skip_comments=True))


def _move_tokens_contain_body(tokens: Iterable[bytes]) -> bool:
    depth = 0
    let_state = 0
    type_tokens_left = 0
    for token in tokens:
        if token == b"{":
            depth += 1
            let_state = 0
            continue
        if token == b"}":
            depth = max(0, depth - 1)
            let_state = 0
            continue
        if not depth:
            continue

        if let_state == 0:
            if token == b"let":
                let_state = 1
        elif let_state == 1:
            let_state = 2 if _is_move_name(token) else 0
        elif let_state == 2:
            if token == b"=":
                return True
            if token == b":":
                let_state = 3
                type_tokens_left = 32
            else:
                let_state = 0
        else:
            if token == b"=":
                return True
            type_tokens_left -= 1
            if token == b";" or type_tokens_left == 0:
                let_state = 0
    return False


def _move_tokens_contain_declaration(tokens: Iterable[bytes]) -> bool:
    module_state = 0
    address_state = 0
    script_state = 0
    function_state = 0
    function_type_depth = 0
    data_type_state = 0
    data_type_depth = 0
    data_type_tokens_left = 0

    for token in tokens:
        if module_state == 1:
            module_state = 2 if _is_move_name(token) else 0
        elif module_state == 2:
            if token == b"{":
                return True
            module_state = 3 if token == b"::" else 0
        elif module_state == 3:
            module_state = 4 if _is_move_identifier(token) else 0
        elif module_state == 4:
            if token == b"{":
                return True
            module_state = 0
        if token == b"module":
            module_state = 1

        if address_state == 1:
            address_state = 2 if _is_move_name(token) else 0
        elif address_state == 2:
            if token == b"{":
                return True
            address_state = 0
        if token == b"address":
            address_state = 1

        if script_state == 1:
            if token == b"{":
                return True
            script_state = 0
        if token == b"script":
            script_state = 1

        if function_state == 1:
            function_state = 2 if _is_move_identifier(token) else 0
        elif function_state == 2:
            if token == b"(":
                return True
            if token == b"<":
                function_state = 3
                function_type_depth = 1
            else:
                function_state = 0
        elif function_state == 3:
            if token == b"<":
                function_type_depth += 1
            elif token == b">":
                function_type_depth -= 1
                if function_type_depth == 0:
                    function_state = 4
        elif function_state == 4:
            if token == b"(":
                return True
            function_state = 0
        if token == b"fun":
            function_state = 1

        if data_type_state == 1:
            data_type_state = 2 if _is_move_identifier(token) else 0
        elif data_type_state == 2:
            if token == b"{":
                return True
            if token == b"<":
                data_type_state = 3
                data_type_depth = 1
            elif token == b"has":
                data_type_state = 4
                data_type_tokens_left = 16
            else:
                data_type_state = 0
        elif data_type_state == 3:
            if token == b"<":
                data_type_depth += 1
            elif token == b">":
                data_type_depth -= 1
                if data_type_depth == 0:
                    data_type_state = 2
        elif data_type_state == 4:
            if token == b"{":
                return True
            data_type_tokens_left -= 1
            if token == b";" or data_type_tokens_left == 0:
                data_type_state = 0
        if token in (b"struct", b"enum"):
            data_type_state = 1

    return False


def _is_move_name(token: bytes) -> bool:
    return _is_move_identifier(token) or (
        len(token) > 2
        and token.startswith(b"0x")
        and all(_is_ascii_hex(byte) for byte in token[2:])
    )


def _is_move_identifier(token: bytes) -> bool:
    if not token or not _is_identifier_start(token[0]):
        return False
    return all(_is_identifier_continue(byte) for byte in token[1:])


def _iter_move_tokens(data: bytes, *, skip_comments: bool) -> Iterator[bytes]:
    index = 0
    while index < len(data):
        if skip_comments and data[index : index + 2] == b"//":
            newline = data.find(b"\n", index + 2)
            if newline < 0:
                return
            index = newline + 1
            continue
        if skip_comments and data[index : index + 2] == b"/*":
            index += 2
            depth = 1
            while index < len(data) and depth:
                if data[index : index + 2] == b"/*":
                    depth += 1
                    index += 2
                elif data[index : index + 2] == b"*/":
                    depth -= 1
                    index += 2
                else:
                    index += 1
            continue

        byte = data[index]
        if (
            byte == ord("0")
            and data[index + 1 : index + 2].lower() == b"x"
            and index + 2 < len(data)
            and _is_ascii_hex(data[index + 2])
        ):
            end = index + 3
            while end < len(data) and _is_ascii_hex(data[end]):
                end += 1
            yield data[index:end].lower()
            index = end
            continue
        if _is_identifier_start(byte):
            end = index + 1
            while end < len(data) and _is_identifier_continue(data[end]):
                end += 1
            yield data[index:end].lower()
            index = end
            continue
        if data[index : index + 2] == b"::":
            yield b"::"
            index += 2
            continue
        punctuation = MOVE_PUNCTUATION.get(byte)
        if punctuation is not None:
            yield punctuation
        index += 1


def _is_identifier_start(byte: int) -> bool:
    return (
        byte == ord("_")
        or ord("A") <= byte <= ord("Z")
        or ord("a") <= byte <= ord("z")
    )


def _is_identifier_continue(byte: int) -> bool:
    return _is_identifier_start(byte) or ord("0") <= byte <= ord("9")


def _is_ascii_hex(byte: int) -> bool:
    return (
        ord("0") <= byte <= ord("9")
        or ord("A") <= byte <= ord("F")
        or ord("a") <= byte <= ord("f")
    )


def _is_ascii_digit(byte: int) -> bool:
    return ord("0") <= byte <= ord("9")


def _preflight_tar(
    path: Path, max_total_bytes: int, max_tar_bytes: int
) -> None:
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
                if raw_bytes > max_tar_bytes:
                    raise PublicationError(
                        f"{path}: expanded tar stream exceeds {max_tar_bytes} bytes"
                    )
                if len(block) != 512:
                    raise PublicationError(f"{path}: truncated tar header")
                if block == _ZERO_BLOCK:
                    saw_end = True
                    _check_zero_padding(path, stream, raw_bytes, max_tar_bytes)
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
                if payload_bytes > max_total_bytes:
                    raise PublicationError(
                        f"{path}: expanded archive exceeds {max_total_bytes} bytes"
                    )
                padded_size = ((size + 511) // 512) * 512
                raw_bytes += padded_size
                if raw_bytes > max_tar_bytes:
                    raise PublicationError(
                        f"{path}: expanded tar stream exceeds {max_tar_bytes} bytes"
                    )
                _discard_exact(path, stream, size)
                _check_member_padding(path, stream, padded_size - size)
    except PublicationError:
        raise
    except (EOFError, OSError) as error:
        raise PublicationError(f"{path}: invalid gzip tar archive: {error}") from error
    if not saw_end:
        raise PublicationError(f"{path}: tar stream is missing its end marker")


def _validate_gzip_container(path: Path, max_tar_bytes: int) -> None:
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
                    if expanded > max_tar_bytes:
                        raise PublicationError(
                            f"{path}: expanded tar stream exceeds {max_tar_bytes} bytes"
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
    name = _decode_ustar_field(path, block[:100])
    prefix = _decode_ustar_field(path, block[345:500])
    full_name = f"{prefix}/{name}" if prefix else name
    info = tarfile.TarInfo(full_name)
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


def _decode_ustar_field(path: Path, field: bytes) -> str:
    content, separator, padding = field.partition(b"\0")
    if separator and padding.strip(b"\0"):
        raise PublicationError(f"{path}: non-canonical tar header")
    encoded = content if separator else field
    try:
        return encoded.decode("ascii")
    except UnicodeDecodeError as error:
        raise PublicationError(f"{path}: non-canonical tar header") from error


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


def _check_zero_padding(
    path: Path, stream: BinaryIO, raw_bytes: int, max_tar_bytes: int
) -> None:
    while True:
        chunk = stream.read(1024 * 1024)
        if not chunk:
            return
        raw_bytes += len(chunk)
        if raw_bytes > max_tar_bytes:
            raise PublicationError(
                f"{path}: expanded tar stream exceeds {max_tar_bytes} bytes"
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
