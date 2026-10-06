# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location(
    "isolate_target", Path(__file__).with_name("isolate-target.py"))
isolate_target = importlib.util.module_from_spec(spec)
spec.loader.exec_module(isolate_target)


class IsolateTargetTest(unittest.TestCase):
    def test_missing_target_and_qualified_module(self):
        source = "import LeanerLang\nleaner module 0x1::example where\n  fun f() -> Unit := ()"
        result = isolate_target.isolate(source.splitlines(), "0x1::example", "f", [])
        self.assertIn("fun f()", result)
        with self.assertRaisesRegex(ValueError, "module.*not found"):
            isolate_target.isolate(source.splitlines(), "missing", "f", [])
        with self.assertRaisesRegex(ValueError, "function.*not found"):
            isolate_target.isolate(source.splitlines(), "example", "missing", [])

    def test_assertion_only_and_authored_targets(self):
        source = """import LeanerLang
leaner module 0x1::example where
  fun selected() -> Unit := ()
  spec selected where
    aborts_if false
  fun assertion_only() -> Unit := assert!(false, 1)
  public(friend) fun «other»() -> Unit := ()
  spec «other» where
    pragma verify = true
  verify «other» by
    first
      | omega
      | grind

  verify selected by
    trivial
leaner module 0x1::later where
  fun ignored() -> Unit := ()
"""
        result = isolate_target.isolate(source.splitlines(), "example", "selected", [])
        self.assertIn("  spec assertion_only where\n    pragma verify = false", result)
        self.assertIn("  spec «other» where\n    pragma verify = false", result)
        self.assertNotIn("pragma verify = true", result)
        self.assertNotIn("verify «other»", result)
        self.assertNotIn("| grind", result)
        self.assertIn("verify selected by\n    trivial", result)
        self.assertNotIn("::later", result)
        self.assertNotIn("spec selected where\n    pragma verify = false", result)


if __name__ == "__main__":
    unittest.main()
