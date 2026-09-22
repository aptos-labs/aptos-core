from __future__ import annotations

import io
import tarfile
import tempfile
import unittest
from pathlib import Path

from harness.publication import (
    PublicationError,
    build_public_archive,
    scan_public_archive,
)


class PublicationTest(unittest.TestCase):
    def test_tracked_etna_archives_are_source_free(self) -> None:
        results = Path(__file__).resolve().parents[1] / "results" / "etna-v3"
        archives = sorted(results.glob("*.tar.gz"))
        self.assertTrue(archives)
        for archive in archives:
            with self.subTest(archive=archive.name):
                scan_public_archive(archive)

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

    def test_builder_rejects_raw_run_artifacts(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "REPORT.md").write_text("report\n", encoding="utf-8")
            (root / "workspace.diff").write_text("harmless-looking\n", encoding="utf-8")
            with self.assertRaisesRegex(
                PublicationError, "forbidden publication artifact"
            ):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_private_path_in_allowed_file(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "cells.csv").write_text(
                "target\nsources/etna/private.move\n", encoding="utf-8"
            )
            with self.assertRaisesRegex(PublicationError, "private Etna source path"):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_builder_rejects_diff_in_allowed_file(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "REPORT.md").write_text(
                "diff --git a/private.move b/private.move\n", encoding="utf-8"
            )
            with self.assertRaisesRegex(
                PublicationError, "unified-diff source content"
            ):
                build_public_archive(root, root / "archive.tar.gz", "round")

    def test_scanner_rejects_nested_or_linked_members(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            nested = root / "nested.tar.gz"
            with tarfile.open(nested, "w:gz") as archive:
                data = b"value\n"
                info = tarfile.TarInfo("round/raw/run.json")
                info.size = len(data)
                archive.addfile(info, io.BytesIO(data))
            with self.assertRaisesRegex(PublicationError, "only top-level aggregate"):
                scan_public_archive(nested)

            linked = root / "linked.tar.gz"
            with tarfile.open(linked, "w:gz") as archive:
                info = tarfile.TarInfo("round/REPORT.md")
                info.type = tarfile.SYMTYPE
                info.linkname = "/etc/passwd"
                archive.addfile(info)
            with self.assertRaisesRegex(PublicationError, "links or special files"):
                scan_public_archive(linked)

    def test_scanner_rejects_bad_checksums(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            archive_path = Path(temporary) / "bad.tar.gz"
            with tarfile.open(archive_path, "w:gz") as archive:
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


if __name__ == "__main__":
    unittest.main()
