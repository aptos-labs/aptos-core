-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.TestInfra
-- The checks below are elaborated in their own `lean` process and are not
-- modules of this library, so nothing else builds what they import. Naming
-- their support module here is what puts its `.olean` on the path the driver
-- hands those processes; without it every check fails to import on a clean
-- checkout, where no earlier build left one behind.
import LeanerE2ETests.CheckSupport

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

/-- The number of checks elaborated at once: `LEANER_E2E_JOBS`, or half the
hardware threads, since each check is a whole Lean process of its own. -/
private def jobs : IO Nat := do
  if let some value ← IO.getEnv "LEANER_E2E_JOBS" then
    match value.toNat? with
    | some count => if count > 0 then return count
    | none => pure ()
    throw <| IO.userError s!"LEANER_E2E_JOBS must be a positive number, not `{value}`"
  return max 1 ((System.Platform.Internal.getHardwareConcurrency ()).toNat / 2)

/-- Elaborate every check, `workers` at a time, from one queue that starts the
largest sources first, so that the longest checks do not end the run alone.
The outputs come back in the order of `sources`. -/
private def elaborateAll (sources : Array System.FilePath) (workers : Nat) :
    IO (Array String) := do
  let sizes ← sources.mapM fun source => return (← source.metadata).byteSize
  let order := (Array.range sources.size).qsort fun a b => sizes[a]! > sizes[b]!
  let next ← IO.mkRef 0
  let tasks ← (List.range (max 1 (min workers sources.size))).toArray.mapM fun _ =>
    IO.asTask (prio := .dedicated) do
      let mut done : Array (Nat × String) := #[]
      repeat
        let position ← next.modifyGet fun position => (position, position + 1)
        let some index := order[position]? | break
        done := done.push (index, ← elaborate sources[index]!)
      return done
  let mut outputs := Array.replicate sources.size ""
  for task in tasks do
    match ← IO.wait task with
    | .ok done => for (index, output) in done do outputs := outputs.set! index output
    | .error error => throw error
  return outputs

def testBaselines : IO Unit := do
  let sources ← Baseline.sourceFilesRecursive checkDir ".lean"
  unless !sources.isEmpty do
    throw <| IO.userError s!"no checks found under {checkDir}"
  let outputs ← elaborateAll sources (← jobs)
  for source in sources, output in outputs do
    Baseline.checkOutput (source.withExtension "exp") output

end LeanerE2ETests.Check
