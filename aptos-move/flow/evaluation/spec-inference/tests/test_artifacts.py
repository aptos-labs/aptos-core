from __future__ import annotations

import shutil
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from harness.artifacts import copy_final_tree


class FinalTreeCopyTest(unittest.TestCase):
    def test_transient_source_race_retries_from_clean_destination(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            output = root / "output"
            source.mkdir()
            (source / "file.move").write_text("module 0x1::m {}\n")
            real_copytree = shutil.copytree
            calls = 0

            def transient_failure(*args: object, **kwargs: object) -> Path:
                nonlocal calls
                calls += 1
                if calls == 1:
                    output.mkdir()
                    (output / "partial").touch()
                    raise shutil.Error([("source", "destination", "disappeared")])
                return real_copytree(*args, **kwargs)  # type: ignore[arg-type]

            with patch("harness.artifacts.shutil.copytree", side_effect=transient_failure):
                copy_final_tree(source, output)

            self.assertEqual(2, calls)
            self.assertFalse((output / "partial").exists())
            self.assertEqual("module 0x1::m {}\n", (output / "file.move").read_text())


class FinalTreeSymlinkTest(unittest.TestCase):
    """`final/` is untrusted output, not trusted input.

    The evaluated agent may create symlinks in its own workspace, and the
    controller copies that workspace verbatim. A link into the host left in an
    artifact that is later read, exported and diffed is a payload waiting for
    whatever follows it.
    """

    def test_a_link_out_of_the_tree_becomes_an_inert_record(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workspace = root / "workspace" / "sources"
            workspace.mkdir(parents=True)
            (workspace / "m.move").write_text("module 0x1::m {}\n", encoding="utf-8")
            (workspace / "escape.move").symlink_to("/etc/passwd")
            (workspace / "inside.move").symlink_to("m.move")
            outside = root / "outside"
            outside.mkdir()
            (outside / "secret.txt").write_text("s", encoding="utf-8")
            (outside / "leak.move").symlink_to("/etc/hosts")
            (workspace / "dir-escape").symlink_to(outside, target_is_directory=True)
            (workspace / "loop-a.move").symlink_to("loop-b.move")
            (workspace / "loop-b.move").symlink_to("loop-a.move")

            final = root / "final"
            copy_final_tree(root / "workspace", final)

            escaped = final / "sources" / "escape.move"
            self.assertFalse(escaped.is_symlink(), "the escaping link must not survive")
            self.assertIn("/etc/passwd", escaped.read_text(encoding="utf-8"))
            self.assertIn("symlink removed", escaped.read_text(encoding="utf-8"))
            # A symlink cycle must not abort finalize -- that would let one
            # planted link discard a finished session, which is the outcome
            # this function exists to deny. It resolves inside the tree on this
            # Python, so it is left as the record of what the agent wrote; it
            # is not an escape, and nothing here follows it.
            self.assertTrue((final / "sources" / "loop-a.move").is_symlink())
            # A directory link is an entry, never a way in: the walk that
            # unlinks and rewrites must not reach outside the tree it was
            # given, whatever the interpreter's glob semantics happen to be.
            outside_file = root / "outside" / "secret.txt"
            self.assertEqual("s", outside_file.read_text(encoding="utf-8"))
            # The tell: this link is outside the tree and escaping, so a walk
            # that descended through `dir-escape` would unlink and rewrite it.
            self.assertTrue((root / "outside" / "leak.move").is_symlink())
            # A link that stays inside the tree is ordinary run output.
            self.assertTrue((final / "sources" / "inside.move").is_symlink())
            self.assertEqual(
                "module 0x1::m {}\n",
                (final / "sources" / "m.move").read_text(encoding="utf-8"),
            )


class WalkContractTest(unittest.TestCase):
    """The walk states its own symlink behaviour rather than inheriting it.

    `Path.rglob` stopped descending through directory links in Python 3.13,
    and this project supports 3.11. These assertions therefore cannot fail on
    3.13 -- both implementations agree there -- but they pin the contract the
    callers depend on, one of which unlinks and rewrites what it visits.
    """

    def test_a_directory_link_is_an_entry_not_a_way_in(self) -> None:
        from harness.artifacts import _walk

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "root"
            (root / "sub").mkdir(parents=True)
            (root / "sub" / "f.txt").write_text("x", encoding="utf-8")
            outside = Path(temporary) / "outside"
            outside.mkdir()
            (outside / "secret.txt").write_text("s", encoding="utf-8")
            (root / "link").symlink_to(outside, target_is_directory=True)

            seen = sorted(p.relative_to(root).as_posix() for p in _walk(root))

            self.assertIn("link", seen, "the link itself is part of the record")
            self.assertEqual(
                [], [p for p in seen if "secret" in p], "the walk must not descend"
            )


if __name__ == "__main__":
    unittest.main()
