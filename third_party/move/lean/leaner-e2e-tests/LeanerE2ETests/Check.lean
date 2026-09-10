-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.TestInfra

/-!
# LeanerLang checks

Every `.lean` file below `LeanerE2ETests/Check/` is a LeanerLang source —
modules with specifications, `verify` commands, proof scripts where real
mathematics lives — that the driver elaborates in its own `lean` process.
What `lean` prints is the baseline, verbatim, beside the source as
`<name>.exp`: a clean check has no expectation file, and a check that
prints anything — a verification failure at its clause range, a rejected
borrow, an unsupported construct — has exactly that output as its
expectation.  `UB=1` writes or removes the file, as compiler-v2's baseline
tests do.

Each check runs under a heartbeat cap, so a proof that silently falls
back to search surfaces as a diff instead of a slow suite.
-/

namespace LeanerE2ETests.Check

open LeanerIR.TestInfra

private def checkDir : System.FilePath :=
  "LeanerE2ETests/Check"

/-- The heartbeat cap, in the units of `maxHeartbeats`: five times the
largest target of the cost benchmark (`total`, the three-variant match of
`LeanerLang/Tests/DenotePerformance`, at 35M).  It bounds every command
of a check and, through `leaner.verifyHeartbeats`, every generated
verification theorem, which otherwise carries its own budget.  Cost
itself is gated by the benchmark; the cap catches collapses. -/
private def heartbeatCap : Nat := 180000

/-- Elaborate one check in its own process, invoked on the package-relative
path so the paths in its messages are stable across machines. -/
private def elaborate (source : System.FilePath) : IO String := do
  let output ← IO.Process.output {
    cmd := "lake"
    args := #["env", "lean", s!"-DmaxHeartbeats={heartbeatCap}",
      s!"-Dweak.leaner.verifyHeartbeats={heartbeatCap}", source.toString] }
  pure (output.stdout ++ output.stderr)

private def testSource (source : System.FilePath) : IO Unit := do
  let output ← elaborate source
  Baseline.checkOutput (source.withExtension "exp") output

def testBaselines : IO Unit := do
  let sources ← Baseline.sourceFilesRecursive checkDir ".lean"
  unless !sources.isEmpty do
    throw <| IO.userError s!"no checks found under {checkDir}"
  for source in sources do
    testSource source

end LeanerE2ETests.Check
