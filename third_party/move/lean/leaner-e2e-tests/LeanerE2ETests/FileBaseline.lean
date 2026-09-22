-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.TestInfra

/-!
# Discoverable side-by-side file baseline driver

The driver owns discovery and expectation comparison. A use case supplies its
pipeline and can attach extra verification steps such as a format/parse
roundtrip without changing the shared LIR test infrastructure.
-/

namespace LeanerE2ETests.FileBaseline

open LeanerIR.TestInfra

structure Suite where
  directory : System.FilePath
  sourceSuffix : String
  produce : System.FilePath → IO String
  expectationExtension : String := "lean"
  includeSource : System.FilePath → Bool := fun _ => true
  verify : System.FilePath → String → IO Unit := fun _ _ => pure ()
  description : String := "file"

def run (suite : Suite) : IO Unit := do
  let sources ← Baseline.sourceFiles suite.directory suite.sourceSuffix
  let sources := sources.filter suite.includeSource
  unless !sources.isEmpty do
    throw <| IO.userError s!"no {suite.description} baselines found under {suite.directory}"
  for source in sources do
    let actual ← suite.produce source
    suite.verify source actual
    Baseline.check (Baseline.expectationPath source suite.expectationExtension) actual

end LeanerE2ETests.FileBaseline
