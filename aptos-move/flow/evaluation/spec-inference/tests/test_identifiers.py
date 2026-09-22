from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from harness.identifiers import require_plain_name, resolve_within


class PlainNameTest(unittest.TestCase):
    """The shared guard for manifest-supplied names that become paths.

    Every caller joins the returned name to a directory it then creates,
    writes or replaces, so what this admits is exactly what those callers
    cannot be made to escape.
    """

    def test_a_real_task_or_round_id_is_accepted(self) -> None:
        for value in (
            "MD-median-015",
            "AX-single-order-book-014",
            "round-001",
            "v1.2_x",
            "a",
        ):
            with self.subTest(value=value):
                self.assertEqual(value, require_plain_name(value, "task_id"))

    def test_a_name_may_not_reach_outside_its_directory(self) -> None:
        for value in (
            "..",
            ".",
            "../victim",
            "../../../etc/passwd",
            "/etc/passwd",
            "a/b",
            "a\\b",
            "task\x00id",
            "~/secret",
            "a b",
        ):
            with self.subTest(value=value):
                with self.assertRaises(ValueError):
                    require_plain_name(value, "task_id")

    def test_a_missing_or_non_string_name_is_refused(self) -> None:
        for value in ("", None, 7, ["MD-median-015"]):
            with self.subTest(value=value):
                with self.assertRaises(ValueError):
                    require_plain_name(value, "task_id")

    def test_the_message_names_the_field_and_the_offending_characters(self) -> None:
        with self.assertRaises(ValueError) as raised:
            require_plain_name("../victim", "task_id")
        message = str(raised.exception)
        self.assertIn("task_id", message)
        self.assertIn("'/'", message)


class ResolveWithinTest(unittest.TestCase):
    """The companion guard, for manifest fields that are relative paths.

    Callers write these paths and, in `refresh` and `promote_replacement`,
    recursively delete them, so containment has to hold before the write.
    """

    def test_a_path_inside_the_root_resolves(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "corpus"
            root.mkdir()
            self.assertEqual(
                root / "samples" / "MD-median-015",
                resolve_within(root, "samples/MD-median-015", "sample_path"),
            )
            self.assertEqual(
                root / "patches" / "x.patch",
                resolve_within(root, "samples/../patches/x.patch", "preparation_patch"),
            )

    def test_the_root_itself_is_refused_unless_opted_into(self) -> None:
        # `.` is inside the root and is also the root. A caller that writes or
        # deletes what it resolved must not be handed the whole tree, so only a
        # caller that means it opts in.
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "corpus"
            root.mkdir()
            for relative in (".", "./", "samples/.."):
                with self.subTest(relative=relative):
                    with self.assertRaises(ValueError) as raised:
                        resolve_within(root, relative, "sample_path")
                    self.assertIn("itself", str(raised.exception))
            self.assertEqual(
                root.resolve(),
                resolve_within(root, ".", "package_relpath", allow_root=True),
            )

    def test_a_path_may_not_leave_the_root(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "corpus"
            root.mkdir()
            (Path(temporary) / "victim").mkdir()
            for relative in ("../victim", "samples/../../victim", "/etc", "../"):
                with self.subTest(relative=relative):
                    with self.assertRaises(ValueError) as raised:
                        resolve_within(root, relative, "sample_path")
                    self.assertIn("escapes", str(raised.exception))

    def test_a_symlink_out_of_the_root_is_refused(self) -> None:
        # Resolution follows links, so containment is of the real target.
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "corpus"
            root.mkdir()
            outside = Path(temporary) / "victim"
            outside.mkdir()
            (root / "escape").symlink_to(outside)
            with self.assertRaises(ValueError):
                resolve_within(root, "escape", "sample_path")

    def test_a_missing_or_non_string_path_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for relative in ("", None, 7):
                with self.subTest(relative=relative):
                    with self.assertRaises(ValueError):
                        resolve_within(root, relative, "sample_path")


class RunSpecIdentifierTest(unittest.TestCase):
    """The run manifest's own identifiers name directories too.

    `run_id` reaches the artifacts root and `task_id` the mutants root, so the
    loader has to apply the same guard rather than its own charset -- a
    charset that permits `.` and `-` permits `..`.
    """

    def test_a_dotted_identifier_is_refused_by_the_loader(self) -> None:
        import json

        from harness.config import RunSpec

        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "run.json"
            base = json.loads((Path(__file__).parent / "fixtures" / "run.json").read_text())
            for field in ("run_id", "round_id", "task_id"):
                for value in ("..", "."):
                    with self.subTest(field=field, value=value):
                        spec = dict(base)
                        spec[field] = value
                        path.write_text(json.dumps(spec), encoding="utf-8")
                        with self.assertRaises(ValueError) as raised:
                            RunSpec.load(path)
                        self.assertIn(field, str(raised.exception))


if __name__ == "__main__":
    unittest.main()
