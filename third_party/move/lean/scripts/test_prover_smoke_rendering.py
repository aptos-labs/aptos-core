# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
"""The proof-consumer mode of `prover-smoke-rendering.py`: what it carries
and what it refuses. `python3 -m unittest scripts/test_prover_smoke_rendering.py`
from the lean directory."""

import importlib.util
import os
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location(
    "prover_smoke_rendering", Path(__file__).with_name("prover-smoke-rendering.py"))
generator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(generator)

PREAMBLE = """-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "fixture.move"
verex proofs where
"""

ITEMS = """
verify cell by
  on _ { }
"""

RENDERING = """import VerexSL

verex module 0x1::fixture where
  pragma proof_file = "VerexE2ETests/SourceVerify/fixture.proof.lean"
  fun cell() -> Unit := ()

verex sources at "/tmp" "/tmp/fixture.move" 1
"""


class ProofConsumerTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.fixtures = os.path.join(
            self.directory.name, "VerexE2ETests", "SourceVerify")
        os.makedirs(self.fixtures)
        self.addCleanup(self.directory.cleanup)
        # The mode is by name: `fixture` is this test's allowlisted consumer.
        consumers = generator.PROOF_CONSUMERS
        generator.PROOF_CONSUMERS = ("fixture",)
        self.addCleanup(setattr, generator, "PROOF_CONSUMERS", consumers)

    def write(self, text, name="fixture.proof.lean"):
        path = os.path.join(self.fixtures, name)
        with open(path, "w") as file:
            file.write(text)
        return path

    def rendering(self, text=RENDERING):
        path = os.path.join(self.directory.name, "fixture.move.lean")
        with open(path, "w") as file:
            file.write(text)
        return path

    def test_a_proof_file_of_the_supported_shape_becomes_the_module_items(self):
        self.write(PREAMBLE + ITEMS)
        items = generator.proof_items("fixture", self.fixtures)
        self.assertIn("  verify cell by", items)
        self.assertNotIn("verex proofs", items)
        before, target = generator.rendering(
            "fixture", self.rendering(), items,
            generator.proof_directive("fixture", self.fixtures))
        self.assertEqual(before, "")
        self.assertNotIn("pragma proof_file", target)
        self.assertIn("  verify cell by", target)

    def test_the_ordinary_mode_still_refuses_a_source_with_a_proof_file(self):
        self.write(PREAMBLE + ITEMS, "other.proof.lean")
        self.assertIn("it has a proof file, to which the driver defers the verification",
                      generator.refusals("other", self.fixtures, [], "", ""))
        self.assertEqual([], generator.refusals("fixture", self.fixtures, [], "", ""))

    def test_a_consumer_whose_cell_defers_to_its_proof_file_is_refused(self):
        deferred = ["error: the automatic verification of `cell` failed; "
                    "provide a proof in `VerexE2ETests/SourceVerify/fixture.proof.lean`:"]
        self.assertTrue(generator.refusals("fixture", self.fixtures, deferred, "", ""))

    def test_an_unsupported_preamble_command_is_refused_not_dropped(self):
        self.write(PREAMBLE.replace("import VerexMove.Proofs",
                                    "import VerexMove.Proofs\nset_option maxHeartbeats 9") + ITEMS)
        with self.assertRaisesRegex(SystemExit, "set_option maxHeartbeats 9"):
            generator.proof_items("fixture", self.fixtures)

    def test_a_proof_file_of_another_source_is_refused(self):
        self.write(PREAMBLE.replace('"fixture.move"', '"different.move"') + ITEMS)
        with self.assertRaisesRegex(SystemExit, "different.move"):
            generator.proof_items("fixture", self.fixtures)

    def test_a_missing_or_doubled_heading_is_refused(self):
        self.write(PREAMBLE.replace("verex proofs where\n", "") + ITEMS)
        with self.assertRaisesRegex(SystemExit, "0 `verex proofs where`"):
            generator.proof_items("fixture", self.fixtures)
        self.write(PREAMBLE + "\nverex proofs where\n" + ITEMS)
        with self.assertRaisesRegex(SystemExit, "2 `verex proofs where`"):
            generator.proof_items("fixture", self.fixtures)

    def test_a_proof_file_that_proves_nothing_is_refused(self):
        self.write(PREAMBLE + "\n-- the proofs are still to be written\n")
        with self.assertRaisesRegex(SystemExit, "no `verify` item"):
            generator.proof_items("fixture", self.fixtures)

    def test_a_rendering_that_does_not_name_the_proof_file_is_refused(self):
        self.write(PREAMBLE + ITEMS)
        items = generator.proof_items("fixture", self.fixtures)
        directive = generator.proof_directive("fixture", self.fixtures)
        without = RENDERING.replace('  pragma proof_file = '
                                    '"VerexE2ETests/SourceVerify/fixture.proof.lean"\n', "")
        with self.assertRaisesRegex(SystemExit, "does not name its proof file once"):
            generator.rendering("fixture", self.rendering(without), items, directive)
        other = RENDERING.replace("fixture.proof.lean", "different.proof.lean")
        with self.assertRaisesRegex(SystemExit, "does not name its proof file once"):
            generator.rendering("fixture", self.rendering(other), items, directive)

    def test_a_rendering_that_names_a_further_proof_file_is_refused(self):
        self.write(PREAMBLE + ITEMS)
        items = generator.proof_items("fixture", self.fixtures)
        directive = generator.proof_directive("fixture", self.fixtures)
        also = RENDERING.replace(
            '  pragma proof_file = "VerexE2ETests/SourceVerify/fixture.proof.lean"\n',
            '  pragma proof_file = "VerexE2ETests/SourceVerify/fixture.proof.lean"\n'
            '  pragma proof_file = "different.proof.lean"\n')
        with self.assertRaisesRegex(SystemExit, "different.proof.lean"):
            generator.rendering("fixture", self.rendering(also), items, directive)

    def test_a_rendering_that_names_its_proof_file_twice_is_refused(self):
        self.write(PREAMBLE + ITEMS)
        items = generator.proof_items("fixture", self.fixtures)
        directive = generator.proof_directive("fixture", self.fixtures)
        twice = RENDERING.replace(
            '  pragma proof_file = "VerexE2ETests/SourceVerify/fixture.proof.lean"\n',
            '  pragma proof_file = "VerexE2ETests/SourceVerify/fixture.proof.lean"\n'
            '  pragma proof_file = "VerexE2ETests/SourceVerify/fixture.proof.lean"\n')
        with self.assertRaisesRegex(SystemExit, "does not name its proof file once"):
            generator.rendering("fixture", self.rendering(twice), items, directive)


if __name__ == "__main__":
    unittest.main()
