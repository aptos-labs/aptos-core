from __future__ import annotations

import gzip
import hashlib
import io
import tarfile
import tempfile
import tracemalloc
import unittest
from contextlib import contextmanager
from pathlib import Path
from typing import Iterator
from unittest.mock import patch

from harness.publication import (
    PublicationError,
    _contains_move_source,
    _decode_html_entities,
    _decode_json_escapes,
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
        if (
            archive.is_file()
            and relative not in LEGACY_ARCHIVES
            and (
                archive.name.casefold().endswith((".tar.gz", ".tgz"))
                or _has_gzip_header(archive)
            )
        ):
            archives.append(archive)
            if len(archives) > MAX_TRACKED_ARCHIVES:
                raise PublicationError(f"{results}: too many result archives")
    return sorted(archives)


def _has_gzip_header(path: Path) -> bool:
    with path.open("rb") as stream:
        return stream.read(2) == b"\x1f\x8b"


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

    def test_nested_html_entity_chain_normalizes_in_one_pass(self) -> None:
        chain = b"&amp;" + b"amp;" * 1000
        self.assertEqual(b"&", _decode_html_entities(chain))

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

    def test_builder_rejects_source_path_in_allowed_file(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "cells.csv").write_text(
                "target\nsources/internal/implementation.move\n", encoding="utf-8"
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
