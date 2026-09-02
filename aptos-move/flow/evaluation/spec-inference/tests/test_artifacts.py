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


if __name__ == "__main__":
    unittest.main()
