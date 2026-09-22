from __future__ import annotations

import base64
import bz2
import gzip
import hashlib
import io
import tarfile
import tempfile
import tracemalloc
import unittest
import zipfile
import zlib
from contextlib import contextmanager
from pathlib import Path
from typing import Iterator
from unittest.mock import patch

from harness.publication import (
    PublicationError,
    _check_content,
    _check_encoded_content,
    _contains_move_source,
    _decode_deflate_candidates,
    _decode_html_entities,
    _decode_json_escapes,
    _decode_structured_content,
    _scan_file,
    build_public_archive,
    scan_public_archive,
)

LEGACY_ARCHIVES = {
    "corpus-v1.2/round-v1.2-005-opus-preparation.tar.gz": (
        "03109546fd5bee94a7f929b127c5a436a2a9b6670a877195d04f8bd4dff3dfec"
    ),
    "corpus-v1.2/round-v1.2-005-opus-xhigh-preparation.tar.gz": (
        "d703dd903f9b4d50fec37a947ab5c2286973e2ecb9a4c082da33cc1ab22a1b33"
    ),
    "corpus-v1.2/round-v1.2-005-opus-xhigh-results.tar.gz": (
        "6e33cadcc17ad34ac59acaa07dea78decfd97c26afbaa3720419f2f636b10b03"
    ),
    "corpus-v1.2/round-v1.2-006-opus-xhigh-preparation.tar.gz": (
        "1597c1203bb81c737e3f48c755a6b7e0c00dcef394cce24cfee3d930c1bdc007"
    ),
    "corpus-v1.2/round-v1.2-006-opus-xhigh-results.tar.gz": (
        "93a462f43e6855824e87955fe54e2203f27f58feb2391bd3816dc55332bc4a46"
    ),
}
MAX_TRACKED_ARCHIVES = 16
MAX_TRACKED_ARCHIVE_BYTES = 64 * 1024 * 1024
SUPPORTED_ARCHIVE_SUFFIXES = (".tar.gz", ".tgz")
UNSUPPORTED_ARCHIVE_SUFFIXES = (
    ".7z",
    ".cpio",
    ".rar",
    ".tar",
    ".tar.bz2",
    ".tar.xz",
    ".tar.zst",
    ".tbz2",
    ".txz",
    ".zip",
    ".zst",
)
UNSUPPORTED_ARCHIVE_MAGICS = (
    b"PK\x03\x04",
    b"PK\x05\x06",
    b"PK\x07\x08",
    b"7z\xbc\xaf\x27\x1c",
    b"Rar!\x1a\x07",
    b"BZh",
    b"\xfd7zXZ\x00",
    b"\x28\xb5\x2f\xfd",
    b"070701",
    b"070702",
    b"070707",
    b"\x71\xc7",
    b"\xc7\x71",
)
RESULT_SIDECAR_SUFFIXES = (".json", ".md")
MAX_ZIP_TRAILER_BYTES = 65_557


@contextmanager
def _archive_writer(
    path: Path, *, tar_format: int = tarfile.USTAR_FORMAT
) -> Iterator[tarfile.TarFile]:
    with path.open("wb") as raw:
        with gzip.GzipFile(fileobj=raw, mode="wb", filename="", mtime=0) as compressed:
            with tarfile.open(
                fileobj=compressed, mode="w", format=tar_format
            ) as archive:
                yield archive


def _result_archives(results: Path) -> list[Path]:
    archives: list[Path] = []
    for archive in results.rglob("*"):
        relative = archive.relative_to(results).as_posix()
        if not archive.is_file() or relative in LEGACY_ARCHIVES:
            continue
        name = archive.name.casefold()
        header = _file_header(archive)
        if name.endswith(SUPPORTED_ARCHIVE_SUFFIXES) or header.startswith(
            b"\x1f\x8b"
        ):
            archives.append(archive)
            if len(archives) > MAX_TRACKED_ARCHIVES:
                raise PublicationError(f"{results}: too many result archives")
        elif (
            name.endswith(UNSUPPORTED_ARCHIVE_SUFFIXES)
            or _is_archive_header(header)
            or _has_zip_end_record(archive)
        ):
            raise PublicationError(f"{archive}: unsupported result archive")
        elif not name.endswith(RESULT_SIDECAR_SUFFIXES):
            raise PublicationError(f"{archive}: unsupported result artifact")
    return sorted(archives)


def _file_header(path: Path) -> bytes:
    with path.open("rb") as stream:
        return stream.read(512)


def _is_archive_header(header: bytes) -> bool:
    return _has_tar_checksum(header) or any(
        header.startswith(magic) for magic in UNSUPPORTED_ARCHIVE_MAGICS
    )


def _has_zip_end_record(path: Path) -> bool:
    size = path.stat().st_size
    with path.open("rb") as stream:
        stream.seek(max(0, size - MAX_ZIP_TRAILER_BYTES))
        trailer = stream.read(MAX_ZIP_TRAILER_BYTES)
    return b"PK\x05\x06" in trailer or b"PK\x06\x06" in trailer


def _has_tar_checksum(header: bytes) -> bool:
    if len(header) < 512:
        return False
    digits = header[148:156].strip(b"\0 ")
    if not digits:
        return False
    try:
        stored_checksum = int(digits, 8)
    except ValueError:
        return False
    checksum_header = header[:148] + b" " * 8 + header[156:512]
    unsigned_checksum = sum(checksum_header)
    signed_checksum = sum(
        byte if byte < 128 else byte - 256 for byte in checksum_header
    )
    return stored_checksum in (unsigned_checksum, signed_checksum)


def _scan_result_archives(results: Path) -> None:
    archives = _result_archives(results)
    if not archives:
        raise PublicationError(f"{results}: no result archives")
    remaining_bytes = MAX_TRACKED_ARCHIVE_BYTES
    for archive in archives:
        archive_bytes = scan_public_archive(
            archive, max_total_bytes=remaining_bytes
        )
        remaining_bytes -= archive_bytes
        if remaining_bytes < 0:
            raise PublicationError(
                f"{results}: aggregate result archives exceed "
                f"{MAX_TRACKED_ARCHIVE_BYTES} bytes"
            )
    archive_set = set(archives)
    legacy_set = {results / relative for relative in LEGACY_ARCHIVES}
    sidecar_content = bytearray()
    for artifact in sorted(results.rglob("*")):
        if (
            not artifact.is_file()
            or artifact in archive_set
            or artifact in legacy_set
        ):
            continue
        with artifact.open("rb") as stream:
            data, _ = _scan_file(
                stream,
                artifact.stat().st_size,
                str(artifact),
                check_source_paths=False,
            )
        sidecar_content.extend(data)
    combined = bytes(sidecar_content)
    _check_content(combined, str(results), check_source_paths=False)
    _check_encoded_content(combined, str(results), check_source_paths=False)


class PublicationTest(unittest.TestCase):
    def test_tracked_archives_follow_publication_contract(self) -> None:
        results = Path(__file__).resolve().parents[1] / "results"
        for relative, expected_digest in LEGACY_ARCHIVES.items():
            archive = results / relative
            self.assertTrue(archive.is_file())
            self.assertEqual(
                expected_digest, hashlib.sha256(archive.read_bytes()).hexdigest()
            )
        _scan_result_archives(results)

    def test_discovers_archives_case_insensitively(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            results = Path(temporary)
            for name in (
                "lower.tar.gz",
                "upper.TAR.GZ",
                "mixed.Tar.Gz",
                "compact.tgz",
            ):
                (results / name).touch()
            (results / "opaque.data").write_bytes(b"\x1f\x8b")
            self.assertEqual(
                [
                    "compact.tgz",
                    "lower.tar.gz",
                    "mixed.Tar.Gz",
                    "opaque.data",
                    "upper.TAR.GZ",
                ],
                [archive.name for archive in _result_archives(results)],
            )

    def test_rejects_unsupported_result_archive(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            results = Path(temporary)
            (results / "archive.zip").write_bytes(b"PK\x03\x04")
            with self.assertRaisesRegex(
                PublicationError, "unsupported result archive"
            ):
                _result_archives(results)

    def test_rejects_v7_tar_with_opaque_name(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            results = Path(temporary)
            info = tarfile.TarInfo("source.move")
            header = bytearray(info.tobuf(format=tarfile.USTAR_FORMAT))
            header[257:265] = b"\0" * 8
            header[148:156] = b" " * 8
            checksum = sum(header)
            header[148:156] = f"{checksum:06o}\0 ".encode()
            (results / "opaque.data").write_bytes(header + bytes(1024))
            with self.assertRaisesRegex(
                PublicationError, "unsupported result archive"
            ):
                _result_archives(results)

    def test_rejects_prepended_zip_with_opaque_name(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            results = Path(temporary)
            payload = io.BytesIO()
            with zipfile.ZipFile(payload, "w") as archive:
                archive.writestr("source.move", "module 0x1::sample {}")
            (results / "opaque.data").write_bytes(bytes(512) + payload.getvalue())
            with self.assertRaisesRegex(
                PublicationError, "unsupported result archive"
            ):
                _result_archives(results)

    def test_rejects_cpio_with_opaque_name(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            results = Path(temporary)
            (results / "opaque.data").write_bytes(b"070701" + bytes(506))
            with self.assertRaisesRegex(
                PublicationError, "unsupported result archive"
            ):
                _result_archives(results)

    def test_rejects_unexpected_result_artifact(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            results = Path(temporary)
            (results / "raw.move").write_text("module 0x1::sample {}")
            with self.assertRaisesRegex(
                PublicationError, "unsupported result artifact"
            ):
                _result_archives(results)

    def test_rejects_excessive_archive_count(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            results = Path(temporary)
            for index in range(MAX_TRACKED_ARCHIVES + 1):
                (results / f"archive-{index}.tar.gz").touch()
            with self.assertRaisesRegex(PublicationError, "too many result archives"):
                _result_archives(results)

    def test_rejects_excessive_aggregate_archive_content(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            results = Path(temporary)
            (results / "first.tar.gz").touch()
            (results / "second.tar.gz").touch()
            with patch(
                "tests.test_publication.scan_public_archive",
                side_effect=[MAX_TRACKED_ARCHIVE_BYTES, 1],
            ):
                with self.assertRaisesRegex(
                    PublicationError, "aggregate result archives exceed"
                ):
                    _scan_result_archives(results)

    def test_scanner_enforces_custom_byte_budget(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "REPORT.md").write_text("aggregate report\n", encoding="utf-8")
            archive = root / "archive.tar.gz"
            build_public_archive(root, archive, "round")
            with self.assertRaisesRegex(PublicationError, "expanded archive exceeds"):
                scan_public_archive(archive, max_total_bytes=1)

    def test_builder_rejects_markdown_encoded_move_source(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "REPORT.md").write_text(
                "m&amp;#111;dule 0x1::sample { public f&#117;n value(): u64 { 1 } }\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_semicolonless_html_entities(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "REPORT.md").write_text(
                "&#109odule 0x1::sample { public &#102un value(): u64 { 1 } }\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_composed_content_encodings(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "REPORT.md").write_text(
                "\\u0026#92;u006dodule 0x1::sample { "
                "public \\u0026#92;u0066un value(): u64 { 1 } }\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_move_function_body(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "REPORT.md").write_text(
                "{\n    let value = 1;\n    value + 1\n}\n", encoding="utf-8"
            )
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_nested_html_entity_chain_normalizes_in_one_pass(self) -> None:
        chain = b"&amp;" + b"amp;" * 1000
        self.assertEqual(b"&", _decode_html_entities(chain))

    def test_alternating_encoding_chain_normalizes_in_one_pass(self) -> None:
        chain = b"&#92;u0026#92;" * 1000 + b"u006d"
        self.assertEqual(b"m", _decode_structured_content(chain))
        self.assertEqual(
            b"module", _decode_structured_content(b"\\u0026#109odule")
        )
        self.assertEqual(b"module", _decode_structured_content(b"%256dodule"))
        long_numeric = b"\\u0026#" + b"0" * 10_000 + b"109odule"
        self.assertEqual(b"module", _decode_structured_content(long_numeric))
        self.assertEqual(b"\\777", _decode_structured_content(b"\\777"))

    def test_repeated_entities_after_large_prefix_normalize(self) -> None:
        prefix = b"x" * 4096
        content = prefix + b"\\u0026#111;" * 1000
        self.assertEqual(
            prefix + b"o" * 1000, _decode_structured_content(content)
        )

    def test_html_entity_scan_has_bounded_state_memory(self) -> None:
        content = b"&" * (1024 * 1024)
        tracemalloc.start()
        try:
            self.assertIs(content, _decode_html_entities(content))
            _, peak = tracemalloc.get_traced_memory()
        finally:
            tracemalloc.stop()
        self.assertLess(peak, 8 * 1024 * 1024)

    def test_builds_deterministic_source_free_archive(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            source.mkdir()
            (source / "REPORT.md").write_text("aggregate report\n", encoding="utf-8")
            (source / "cells.csv").write_text("run,cost\nr1,1.25\n", encoding="utf-8")
            first = root / "first.tar.gz"
            second = root / "second.tar.gz"
            build_public_archive(source, first, "round")
            build_public_archive(source, second, "round")
            self.assertEqual(first.read_bytes(), second.read_bytes())
            scan_public_archive(first)
            with tarfile.open(first, "r:gz") as archive:
                self.assertEqual(
                    ["round/REPORT.md", "round/cells.csv", "round/SHA256SUMS"],
                    archive.getnames(),
                )

    def test_builds_archive_with_ustar_prefix(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            source.mkdir()
            (source / "REPORT.md").write_text("report\n", encoding="utf-8")
            output = root / "long-name.tar.gz"
            archive_name = "r" * 100
            build_public_archive(source, output, archive_name)
            scan_public_archive(output)
            with tarfile.open(output, "r:gz") as archive:
                self.assertEqual(
                    [
                        f"{archive_name}/REPORT.md",
                        f"{archive_name}/SHA256SUMS",
                    ],
                    archive.getnames(),
                )

    def test_builder_accepts_json_keyword_objects(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "analysis.json").write_text(
                '{"script": {}, "module": {}, "address": {}, '
                '"fun": {}, "struct": {}, "enum": {}}\n',
                encoding="utf-8",
            )
            archive_path = root / "archive.tar.gz"
            build_public_archive(root, archive_path, "round")
            scan_public_archive(archive_path)

    def test_builder_rejects_json_escaped_move_source(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = "module 0x1::sample { public fun value(): u64 { 1 } }"
            encoded = "".join(f"\\u{ord(character):04x}" for character in source)
            (root / "analysis.json").write_text(
                f'{{"payload": "{encoded}"}}\n', encoding="utf-8"
            )
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_json_escaped_source_path(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "analysis.json").write_text(
                '{"path": "sources\\/internal\\/implementation.move"}\n',
                encoding="utf-8",
            )
            with self.assertRaisesRegex(PublicationError, "disallowed source path"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_nested_json_escaped_move_source(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = "module 0x1::sample { public fun value(): u64 { 1 } }"
            encoded = "".join(f"\\u{ord(character):04x}" for character in source)
            nested = encoded.replace("\\", "\\\\")
            (root / "analysis.json").write_text(
                f'{{"payload": "{nested}"}}\n', encoding="utf-8"
            )
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_base64_encoded_move_source(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = b"module 0x1::sample { public fun value(): u64 { 1 } }"
            encoded = base64.b64encode(source).decode("ascii")
            (root / "analysis.json").write_text(
                f'{{"payload": "{encoded}"}}\n', encoding="utf-8"
            )
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_other_base_encoded_move_source(self) -> None:
        source = b"module 0x1::sample { public fun value(): u64 { 1 } }"
        encoders = {
            "base16": base64.b16encode,
            "base32": base64.b32encode,
            "base32hex": base64.b32hexencode,
            "base85": base64.b85encode,
            "ascii85": base64.a85encode,
        }
        for encoding, encoder in encoders.items():
            with self.subTest(encoding=encoding):
                with tempfile.TemporaryDirectory() as temporary:
                    root = Path(temporary)
                    (root / "REPORT.md").write_bytes(encoder(source) + b"\n")
                    with self.assertRaisesRegex(
                        PublicationError, "Move source content"
                    ):
                        build_public_archive(
                            root, root / "archive.tar.gz", "round"
                        )

    def test_builder_rejects_wrapped_base_n_move_source(self) -> None:
        source = b"module 0x1::sample { public fun value(): u64 { 1 } }"
        encoders = {
            "base16": (base64.b16encode, 2),
            "base32": (base64.b32encode, 4),
            "base85": (base64.b85encode, 5),
            "ascii85": (base64.a85encode, 5),
        }
        for encoding, (encoder, width) in encoders.items():
            with self.subTest(encoding=encoding):
                with tempfile.TemporaryDirectory() as temporary:
                    root = Path(temporary)
                    encoded = encoder(source)
                    wrapped = b"\n".join(
                        encoded[offset : offset + width]
                        for offset in range(0, len(encoded), width)
                    )
                    (root / "REPORT.md").write_bytes(wrapped + b"\n")
                    with self.assertRaisesRegex(
                        PublicationError, "Move source content"
                    ):
                        build_public_archive(
                            root, root / "archive.tar.gz", "round"
                        )

    def test_builder_rejects_wrapped_base_n_after_prose(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = b"module 0x1::sample { public fun value(): u64 { 1 } }"
            encoded = base64.b16encode(source)
            wrapped = b"\n".join(
                encoded[offset : offset + 2]
                for offset in range(0, len(encoded), 2)
            )
            (root / "REPORT.md").write_bytes(b"report 2026\n" + wrapped + b"\n")
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_delimited_base_n_move_source(self) -> None:
        source = b"module 0x1::sample { public fun value(): u64 { 1 } }"
        encoders = {
            "base16": (base64.b16encode, 2, b","),
            "base32": (base64.b32encode, 4, b","),
            "base85": (base64.b85encode, 5, b","),
            "ascii85": (base64.a85encode, 5, b"|"),
        }
        for encoding, (encoder, width, delimiter) in encoders.items():
            with self.subTest(encoding=encoding):
                with tempfile.TemporaryDirectory() as temporary:
                    root = Path(temporary)
                    encoded = encoder(source)
                    fragmented = delimiter.join(
                        encoded[offset : offset + width]
                        for offset in range(0, len(encoded), width)
                    )
                    (root / "REPORT.md").write_bytes(fragmented + b"\n")
                    with self.assertRaisesRegex(
                        PublicationError, "Move source content"
                    ):
                        build_public_archive(
                            root, root / "archive.tar.gz", "round"
                        )

    def test_builder_rejects_variable_width_base16_fragments(self) -> None:
        encoded = (
            "6d6f6475~6c65203078~313a3a657669~6c207b2066756e~"
            "206c6561~6b2829207b~7d207d"
        )
        for suffix in ("", "~f"):
            with self.subTest(suffix=suffix):
                with tempfile.TemporaryDirectory() as temporary:
                    root = Path(temporary)
                    (root / "REPORT.md").write_text(encoded + suffix + "\n")
                    with self.assertRaisesRegex(
                        PublicationError, "Move source content"
                    ):
                        build_public_archive(
                            root, root / "archive.tar.gz", "round"
                        )

    def test_builder_rejects_ascii85_zero_shorthand(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = b"\0" * 4 + b"module 0x1::sample {}"
            encoded = base64.a85encode(source)
            self.assertIn(b"z", encoded)
            (root / "REPORT.md").write_bytes(encoded + b"\n")
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_base64_next_to_markdown_text(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = b"module 0x1::sample { public fun value(): u64 { 1 } }"
            encoded = base64.b64encode(source).decode("ascii")
            (root / "REPORT.md").write_text(
                f"payload {encoded}\n", encoding="utf-8"
            )
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_line_wrapped_base64_move_source(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = b"module 0x1::sample { public fun value(): u64 { 1 } }"
            encoded = base64.b64encode(source).decode("ascii")
            wrapped = "\n".join(
                encoded[offset : offset + 16]
                for offset in range(0, len(encoded), 16)
            )
            (root / "REPORT.md").write_text(wrapped + "\n", encoding="utf-8")
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_narrowly_wrapped_base64_move_source(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = b"module 0x1::sample { public fun value(): u64 { 1 } }"
            encoded = base64.b64encode(source).decode("ascii")
            wrapped = "\n".join(
                encoded[offset : offset + 3]
                for offset in range(0, len(encoded), 3)
            )
            (root / "REPORT.md").write_text(wrapped + "\n", encoding="utf-8")
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_zero_width_separated_base64(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = b"module 0x1::sample { public fun value(): u64 { 1 } }"
            encoded = base64.b64encode(source).decode("ascii")
            separated = "\u200b".join(encoded)
            (root / "REPORT.md").write_text(separated + "\n", encoding="utf-8")
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_base64_encoded_gzip(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = b"module 0x1::sample { public fun value(): u64 { 1 } }"
            encoded = base64.b64encode(gzip.compress(source)).decode("ascii")
            (root / "REPORT.md").write_text(encoded + "\n", encoding="utf-8")
            with self.assertRaisesRegex(PublicationError, "binary container"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_base64_encoded_prefixed_zip(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            payload = io.BytesIO()
            with zipfile.ZipFile(payload, "w", zipfile.ZIP_DEFLATED) as archive:
                archive.writestr("source.move", "module 0x1::sample {}")
            encoded = base64.b64encode(b"X" + payload.getvalue()).decode("ascii")
            (root / "REPORT.md").write_text(encoded + "\n", encoding="utf-8")
            with self.assertRaisesRegex(PublicationError, "binary container"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_base64_encoded_prefixed_bzip2(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = b"module 0x1::sample { public fun value(): u64 { 1 } }"
            encoded = base64.b64encode(b"X" + bz2.compress(source))
            (root / "REPORT.md").write_bytes(encoded + b"\n")
            with self.assertRaisesRegex(PublicationError, "binary container"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_base64_encoded_prefixed_gzip(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = b"module 0x1::sample { public fun value(): u64 { 1 } }"
            encoded = base64.b64encode(b"X" + gzip.compress(source)).decode(
                "ascii"
            )
            (root / "REPORT.md").write_text(encoded + "\n", encoding="utf-8")
            with self.assertRaisesRegex(PublicationError, "binary container"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_base64_encoded_zlib(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = b"module 0x1::sample { public fun value(): u64 { 1 } }"
            encoded = base64.b64encode(zlib.compress(source)).decode("ascii")
            (root / "REPORT.md").write_text(encoded + "\n", encoding="utf-8")
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_concatenated_zlib_streams(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = b"module 0x1::sample { public fun value(): u64 { 1 } }"
            compressed = zlib.compress(b"prefix") + zlib.compress(source)
            encoded = base64.b64encode(compressed).decode("ascii")
            (root / "REPORT.md").write_text(encoded + "\n", encoding="utf-8")
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_preserves_compressed_bytes_before_text_decoding(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = b"module 0x1::sample { public fun value(): u64 { 1 } }"
            prefix = hashlib.sha256(b"128").digest()
            compressed = zlib.compress(prefix + source)
            self.assertIn(b'\\"', compressed)
            encoded = base64.b64encode(compressed).decode("ascii")
            (root / "REPORT.md").write_text(encoded + "\n", encoding="utf-8")
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_base64_encoded_raw_deflate(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = b"module 0x1::sample { public fun value(): u64 { 1 } }"
            compressor = zlib.compressobj(wbits=-zlib.MAX_WBITS)
            compressed = compressor.compress(source) + compressor.flush()
            encoded = base64.b64encode(compressed).decode("ascii")
            (root / "REPORT.md").write_text(encoded + "\n", encoding="utf-8")
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_prefixed_deflate_streams(self) -> None:
        source = b"module 0x1::sample { public fun value(): u64 { 1 } }"
        compressor = zlib.compressobj(wbits=-zlib.MAX_WBITS)
        raw_stream = compressor.compress(source) + compressor.flush()
        streams = {
            "zlib": b"X" + zlib.compress(source),
            "zlib-after-long-prefix": b"X" * 65 + zlib.compress(source),
            "raw": b"X" + raw_stream,
            "raw-after-long-prefix": b"X" * 65 + raw_stream,
        }
        for encoding, payload in streams.items():
            with self.subTest(encoding=encoding):
                with tempfile.TemporaryDirectory() as temporary:
                    root = Path(temporary)
                    encoded = base64.b64encode(payload)
                    (root / "REPORT.md").write_bytes(encoded + b"\n")
                    with self.assertRaisesRegex(
                        PublicationError, "Move source content"
                    ):
                        build_public_archive(
                            root, root / "archive.tar.gz", "round"
                        )

    def test_builder_bounds_invalid_gzip_marker_probes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            encoded = base64.b64encode(b"\x1f\x8b" * 4097)
            (root / "REPORT.md").write_bytes(encoded + b"\n")
            with self.assertRaisesRegex(PublicationError, "binary container"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_accepts_repeated_zlib_header_text(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "REPORT.md").write_text("80" * 5000 + "\n")
            archive_path = root / "archive.tar.gz"
            build_public_archive(root, archive_path, "round")
            scan_public_archive(archive_path)

    def test_builder_rejects_padded_base64_fragments(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = b"module 0x1::sample { public fun value(): u64 { 1 } }"
            encoded = "\n".join(
                base64.b64encode(bytes((byte,))).decode("ascii") for byte in source
            )
            (root / "REPORT.md").write_text(encoded + "\n", encoding="utf-8")
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_delimited_base64_fragments(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = b"module 0x1::sample { public fun value(): u64 { 1 } }"
            encoded = base64.b64encode(source).decode("ascii")
            fragmented = ",".join(
                encoded[offset : offset + 4]
                for offset in range(0, len(encoded), 4)
            )
            (root / "REPORT.md").write_text(
                fragmented + "\n", encoding="utf-8"
            )
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_long_delimited_base64_fragments(self) -> None:
        source = b"module 0x1::sample { public fun value(): u64 { 1 } }"
        encoded = base64.b64encode(source).decode("ascii")
        for width in (16, 17):
            with self.subTest(width=width):
                with tempfile.TemporaryDirectory() as temporary:
                    root = Path(temporary)
                    fragmented = ",".join(
                        encoded[offset : offset + width]
                        for offset in range(0, len(encoded), width)
                    )
                    (root / "REPORT.md").write_text(
                        fragmented + "\n", encoding="utf-8"
                    )
                    with self.assertRaisesRegex(
                        PublicationError, "Move source content"
                    ):
                        build_public_archive(
                            root, root / "archive.tar.gz", "round"
                        )

    def test_plain_text_avoids_full_deflate_probes(self) -> None:
        content = b"." * (256 * 1024)
        with patch("harness.publication._decode_deflate_at_offset") as decode:
            list(
                _decode_deflate_candidates(
                    content,
                    64 * 1024 * 1024,
                    "REPORT.md",
                    scan_raw=False,
                )
            )
        decode.assert_not_called()

    def test_builder_rejects_short_delimited_base64_source(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            encoded = base64.b64encode(b"module a {}").decode("ascii")
            fragmented = ",".join(
                encoded[offset : offset + 4]
                for offset in range(0, len(encoded), 4)
            )
            (root / "REPORT.md").write_text(
                fragmented + "\n", encoding="utf-8"
            )
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_percent_encoded_move_source(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = b"module 0x1::sample { public fun value(): u64 { 1 } }"
            encoded = "".join(f"%{byte:02x}" for byte in source)
            (root / "REPORT.md").write_text(encoded + "\n", encoding="utf-8")
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_octal_encoded_move_source(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = b"module 0x1::sample { public fun value(): u64 { 1 } }"
            encoded = "".join(f"\\{byte:03o}" for byte in source)
            (root / "REPORT.md").write_text(encoded + "\n", encoding="utf-8")
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_percent_encoded_gzip(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = b"module 0x1::sample { public fun value(): u64 { 1 } }"
            encoded = "".join(f"%{byte:02x}" for byte in gzip.compress(source))
            (root / "REPORT.md").write_text(encoded + "\n", encoding="utf-8")
            with self.assertRaisesRegex(PublicationError, "binary container"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_bounds_base64_candidate_work(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            content = (("A" * 16 + ",") * 65) + "\n"
            (root / "REPORT.md").write_text(content, encoding="utf-8")
            with patch("harness.publication.MAX_ENCODED_CANDIDATES", 64):
                with self.assertRaisesRegex(PublicationError, "candidate limit"):
                    build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_nested_base64_and_json_move_source(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = b"\\u006dodule 0x1::sample { public fun value(): u64 { 1 } }"
            encoded = base64.b64encode(base64.b64encode(source)).decode("ascii")
            (root / "analysis.json").write_text(
                f'{{"payload": "{encoded}"}}\n', encoding="utf-8"
            )
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_utf16_json(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "analysis.json").write_bytes(
                '{"payload": "module 0x1::sample {}"}\n'.encode("utf-16")
            )
            with self.assertRaisesRegex(PublicationError, "UTF-8"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_nested_json_escape_chain_normalizes_in_one_pass(self) -> None:
        chain = b"\\u005c" + b"u005c" * 1000 + b"u006d"
        self.assertEqual(b"m", _decode_json_escapes(chain))

        encoded_components = (
            b"\\u005c\\u0075\\u0030\\u0030\\u0036\\u0064"
        )
        self.assertEqual(b"m", _decode_json_escapes(encoded_components))

    def test_builder_rejects_raw_run_artifacts(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "REPORT.md").write_text("report\n", encoding="utf-8")
            (root / "workspace.diff").write_text("harmless-looking\n", encoding="utf-8")
            with self.assertRaisesRegex(
                PublicationError, "forbidden publication artifact"
            ):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_source_split_across_members(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fragments = {
                "DEBRIEF.md": "module 0x1::sample",
                "REPORT.md": " { public fun",
                "mined.md": " value",
                "status.md": "(): u64 { 1 } }",
            }
            for name, content in fragments.items():
                (root / name).write_text(content, encoding="utf-8")
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_scan_rejects_source_in_result_sidecar(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            source.mkdir()
            (source / "REPORT.md").write_text("aggregate report\n")
            results = root / "results"
            results.mkdir()
            build_public_archive(source, results / "bundle.tar.gz", "round")
            (results / "extra.md").write_text("module 0x1::sample {}\n")
            with patch.dict(LEGACY_ARCHIVES, clear=True):
                with self.assertRaisesRegex(
                    PublicationError, "Move source content"
                ):
                    _scan_result_archives(results)

    def test_scan_rejects_source_split_across_sidecars(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            source.mkdir()
            (source / "REPORT.md").write_text("aggregate report\n")
            results = root / "results"
            results.mkdir()
            build_public_archive(source, results / "bundle.tar.gz", "round")
            fragments = {
                "a.md": "module 0x1::sample ",
                "b.json": "{ public fun value",
                "c.md": "(): u64 { 1 } }",
            }
            for name, content in fragments.items():
                (results / name).write_text(content)
            with patch.dict(LEGACY_ARCHIVES, clear=True):
                with self.assertRaisesRegex(
                    PublicationError, "Move source content"
                ):
                    _scan_result_archives(results)

    def test_builder_rejects_source_path_in_allowed_file(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "cells.csv").write_text(
                "target\nsources/internal/implementation.move\n", encoding="utf-8"
            )
            with self.assertRaisesRegex(PublicationError, "disallowed source path"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_source_path_with_repeated_separators(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "REPORT.md").write_text(
                "target: sources//internal/implementation.move\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(PublicationError, "disallowed source path"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_diff_in_allowed_file(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "REPORT.md").write_text(
                "diff --git a/implementation.move b/implementation.move\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(
                PublicationError, "unified-diff source content"
            ):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_move_source_in_allowed_file(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "REPORT.md").write_text(
                "module example::sample {\n    public fun value(): u64 { 1 }\n}\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_quoted_move_source_in_allowed_file(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "REPORT.md").write_text(
                "> module example::sample {\n>     public fun value(): u64 { 1 }\n> }\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_multiline_move_source_in_allowed_file(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "REPORT.md").write_text(
                "module\nexample::sample\n{\npublic\nfun\nvalue(): u64\n{ 1 }\n}\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_comment_split_move_source_in_allowed_file(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "REPORT.md").write_text(
                "module /* gap */ example::sample {\n"
                "    public /* gap */ fun value(): u64 { 1 }\n"
                "}\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_address_block_move_source(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "REPORT.md").write_text(
                "address 0x42 {\n"
                "    module sample {\n"
                "        const VALUE: u64 = 1;\n"
                "    }\n"
                "}\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(PublicationError, "Move source content"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_move_source_scan_has_bounded_token_memory(self) -> None:
        payload = b"aa " * (1024 * 1024 // 3)
        tracemalloc.start()
        try:
            self.assertFalse(_contains_move_source(payload))
            _, peak = tracemalloc.get_traced_memory()
        finally:
            tracemalloc.stop()
        self.assertLess(peak, 8 * 1024 * 1024)

    def test_scanner_rejects_nested_or_linked_members(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            nested = root / "nested.tar.gz"
            with _archive_writer(nested) as archive:
                data = b"value\n"
                info = tarfile.TarInfo("round/raw/run.json")
                info.size = len(data)
                archive.addfile(info, io.BytesIO(data))
            with self.assertRaisesRegex(PublicationError, "only top-level aggregate"):
                scan_public_archive(nested)

            linked = root / "linked.tar.gz"
            with _archive_writer(linked) as archive:
                info = tarfile.TarInfo("round/REPORT.md")
                info.type = tarfile.SYMTYPE
                info.linkname = "/etc/passwd"
                archive.addfile(info)
            with self.assertRaisesRegex(PublicationError, "unsupported member type"):
                scan_public_archive(linked)

    def test_scanner_rejects_invalid_archive_root(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            archive_path = Path(temporary) / "invalid-root.tar.gz"
            report = b"report\n"
            checksum = (
                f"{hashlib.sha256(report).hexdigest()}  REPORT.md\n".encode("utf-8")
            )
            invalid_root = "module 0x1::sample { public fun value() {} }"
            with _archive_writer(archive_path) as archive:
                for name, data in (
                    (f"{invalid_root}/REPORT.md", report),
                    (f"{invalid_root}/SHA256SUMS", checksum),
                ):
                    info = tarfile.TarInfo(name)
                    info.size = len(data)
                    archive.addfile(info, io.BytesIO(data))
            with self.assertRaisesRegex(PublicationError, "archive root"):
                scan_public_archive(archive_path)

    def test_scanner_rejects_bad_checksums(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            archive_path = Path(temporary) / "bad.tar.gz"
            with _archive_writer(archive_path) as archive:
                for name, data in (
                    ("round/REPORT.md", b"report\n"),
                    ("round/SHA256SUMS", b"0" * 64 + b"  REPORT.md\n"),
                ):
                    info = tarfile.TarInfo(name)
                    info.size = len(data)
                    archive.addfile(info, io.BytesIO(data))
            with self.assertRaisesRegex(
                PublicationError, "does not match archive contents"
            ):
                scan_public_archive(archive_path)

    def test_scanner_rejects_extended_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            archive_path = Path(temporary) / "extended.tar.gz"
            report = b"report\n"
            checksum = (
                f"{hashlib.sha256(report).hexdigest()}  REPORT.md\n".encode("utf-8")
            )
            with _archive_writer(
                archive_path, tar_format=tarfile.PAX_FORMAT
            ) as archive:
                for name, data in (
                    ("round/REPORT.md", report),
                    ("round/SHA256SUMS", checksum),
                ):
                    info = tarfile.TarInfo(name)
                    info.size = len(data)
                    info.pax_headers = {"comment": "x" * 1024}
                    archive.addfile(info, io.BytesIO(data))
            with self.assertRaisesRegex(PublicationError, "extended metadata"):
                scan_public_archive(archive_path)

    def test_scanner_rejects_trailing_gzip_member(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            archive_path = Path(temporary) / "trailing.tar.gz"
            report = b"report\n"
            checksum = (
                f"{hashlib.sha256(report).hexdigest()}  REPORT.md\n".encode("utf-8")
            )
            with _archive_writer(archive_path) as archive:
                for name, data in (
                    ("round/REPORT.md", report),
                    ("round/SHA256SUMS", checksum),
                ):
                    info = tarfile.TarInfo(name)
                    info.size = len(data)
                    archive.addfile(info, io.BytesIO(data))
            with archive_path.open("ab") as stream:
                stream.write(gzip.compress(b"unexpected trailing data", mtime=0))
            with self.assertRaisesRegex(PublicationError, "single gzip member"):
                scan_public_archive(archive_path)

    def test_scanner_rejects_empty_trailing_gzip_member(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            archive_path = Path(temporary) / "trailing-empty.tar.gz"
            report = b"report\n"
            checksum = (
                f"{hashlib.sha256(report).hexdigest()}  REPORT.md\n".encode("utf-8")
            )
            with _archive_writer(archive_path) as archive:
                for name, data in (
                    ("round/REPORT.md", report),
                    ("round/SHA256SUMS", checksum),
                ):
                    info = tarfile.TarInfo(name)
                    info.size = len(data)
                    archive.addfile(info, io.BytesIO(data))
            with archive_path.open("ab") as stream:
                with gzip.GzipFile(
                    filename="metadata.txt", fileobj=stream, mode="wb", mtime=0
                ):
                    pass
            with self.assertRaisesRegex(PublicationError, "single gzip member"):
                scan_public_archive(archive_path)

    def test_scanner_rejects_gzip_header_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            archive_path = Path(temporary) / "metadata.tar.gz"
            report = b"report\n"
            checksum = (
                f"{hashlib.sha256(report).hexdigest()}  REPORT.md\n".encode("utf-8")
            )
            tar_data = io.BytesIO()
            with tarfile.open(fileobj=tar_data, mode="w") as archive:
                for name, data in (
                    ("round/REPORT.md", report),
                    ("round/SHA256SUMS", checksum),
                ):
                    info = tarfile.TarInfo(name)
                    info.size = len(data)
                    archive.addfile(info, io.BytesIO(data))
            with archive_path.open("wb") as stream:
                with gzip.GzipFile(
                    filename="metadata.txt", fileobj=stream, mode="wb", mtime=0
                ) as compressed:
                    compressed.write(tar_data.getvalue())
            with self.assertRaisesRegex(PublicationError, "gzip header metadata"):
                scan_public_archive(archive_path)

    def test_scanner_bounds_compressed_gzip_input(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "REPORT.md").write_text("report\n", encoding="utf-8")
            archive_path = root / "archive.tar.gz"
            build_public_archive(root, archive_path, "round")
            tar_data = gzip.decompress(archive_path.read_bytes())
            compressor = zlib.compressobj(wbits=-zlib.MAX_WBITS)
            compressed_tar = compressor.compress(tar_data) + compressor.flush()
            header = b"\x1f\x8b\x08\x00" + b"\0" * 4 + b"\x00\xff"
            empty_blocks = b"\x00\x00\x00\xff\xff" * 20_000
            trailer = zlib.crc32(tar_data).to_bytes(4, "little") + (
                len(tar_data) & 0xFFFF_FFFF
            ).to_bytes(4, "little")
            archive_path.write_bytes(
                header + empty_blocks + compressed_tar + trailer
            )
            with self.assertRaisesRegex(
                PublicationError, "compressed gzip stream exceeds input budget"
            ):
                scan_public_archive(archive_path)

    def test_scanner_rejects_tar_header_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            archive_path = Path(temporary) / "tar-metadata.tar.gz"
            report = b"report\n"
            checksum = (
                f"{hashlib.sha256(report).hexdigest()}  REPORT.md\n".encode("utf-8")
            )
            with _archive_writer(archive_path) as archive:
                for index, (name, data) in enumerate(
                    (
                        ("round/REPORT.md", report),
                        ("round/SHA256SUMS", checksum),
                    )
                ):
                    info = tarfile.TarInfo(name)
                    info.size = len(data)
                    if index == 0:
                        info.uname = "unexpected"
                    archive.addfile(info, io.BytesIO(data))
            with self.assertRaisesRegex(PublicationError, "non-canonical tar header"):
                scan_public_archive(archive_path)

    def test_scanner_rejects_nonzero_tar_padding(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            archive_path = Path(temporary) / "tar-padding.tar.gz"
            report = b"report\n"
            checksum = (
                f"{hashlib.sha256(report).hexdigest()}  REPORT.md\n".encode("utf-8")
            )
            tar_data = io.BytesIO()
            with tarfile.open(
                fileobj=tar_data, mode="w", format=tarfile.USTAR_FORMAT
            ) as archive:
                for name, data in (
                    ("round/REPORT.md", report),
                    ("round/SHA256SUMS", checksum),
                ):
                    info = tarfile.TarInfo(name)
                    info.size = len(data)
                    info.mode = 0o644
                    archive.addfile(info, io.BytesIO(data))
            raw_tar = bytearray(tar_data.getvalue())
            raw_tar[512 + len(report)] = ord("x")
            with archive_path.open("wb") as stream:
                with gzip.GzipFile(
                    filename="", fileobj=stream, mode="wb", mtime=0
                ) as compressed:
                    compressed.write(raw_tar)
            with self.assertRaisesRegex(PublicationError, "nonzero tar padding"):
                scan_public_archive(archive_path)


if __name__ == "__main__":
    unittest.main()
