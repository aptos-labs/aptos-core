-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.TestInfra
-- The checks below are elaborated in their own `lean` process and are not
-- modules of this library, so nothing else builds what they import. Naming
-- their support module here is what puts its `.olean` on the path the driver
-- hands those processes; without it every check fails to import on a clean
-- checkout, where no earlier build left one behind.
import VerexE2ETests.CheckSupport
import VerexE2ETests.Workers

/-!
# VerexSL checks

Every `.lean` file below `VerexE2ETests/Check/` is a VerexSL source —
modules with specifications, `verify` commands, proof scripts where real
mathematics lives — that the driver elaborates in its own `lean` process.
What `lean` prints is the baseline, verbatim, beside the source as
`<name>.exp`: a clean check has no expectation file, and a check that
prints anything — a verification failure at its clause range, a rejected
borrow, an unsupported construct — has exactly that output as its
expectation.  `UB=1` writes or removes the file, as compiler-v2's baseline
tests do.

Each check runs under a heartbeat cap, so a proof that silently falls
back to search surfaces as a diff instead of a slow suite, and under the
pool's timeout, so one that stops making progress costs its own process.
-/

namespace VerexE2ETests.Check

open VerexIR.TestInfra

private def checkDir : System.FilePath :=
  "VerexE2ETests/Check"

/-- The heartbeat cap, in the units of `maxHeartbeats`: five times the
largest target of the cost benchmark (`total`, the three-variant match of
`VerexSL/Tests/DenotePerformance`, at 35M).  It bounds every command
of a check and, through `verex.verifyHeartbeats`, every generated
verification theorem, which otherwise carries its own budget.  Cost
itself is gated by the benchmark; the cap catches collapses. -/
private def heartbeatCap : Nat := 180000

/-- Elaborate one check in its own process, invoked on the package-relative
path so the paths in its messages are stable across machines. A module's
targets are proved `jobs` at a time in it: the processes share the threads.
`lake` and the `lean` it starts are killed together at the expiry of
`timeout`. -/
private def elaborate (source : System.FilePath) (jobs timeout : Nat) : IO Workers.Run :=
  Workers.runUnderTimeout timeout {
    cmd := "lake"
    args := #["env", "lean", s!"-DmaxHeartbeats={heartbeatCap}",
      s!"-Dweak.verex.verifyHeartbeats={heartbeatCap}", s!"-Dverex.jobs={jobs}",
      source.toString] }

/-- The standing record of the checks known to differ from their baselines,
their file stems comma-separated on one line, so that
`VEREX_E2E_FILTER=$(cat .lake/check-failed)` reruns them. A run removes the
checks that matched and adds the ones that differed; the checks it did not run
stay. -/
private def failedFile : System.FilePath :=
  ".lake/check-failed"

/-- Elaborate every check, `workers` at a time, from one queue that starts the
checks of `recorded` and then the largest sources, so that what is known to
differ is reported early and the longest checks do not end the run alone.
The runs come back in the order of `sources`. -/
private def elaborateAll (sources : Array System.FilePath) (workers : Nat)
    (recorded : Array String) : IO (Array Workers.Run) := do
  let sizes ← sources.mapM fun source => return (← source.metadata).byteSize
  let isRecorded (index : Nat) : Bool := recorded.contains (sources[index]!.fileStem.getD "")
  let order := (Array.range sources.size).qsort fun a b =>
    if isRecorded a != isRecorded b then isRecorded a else sizes[a]! > sizes[b]!
  let jobs := Workers.targetJobs (min workers sources.size)
  let timeout ← Workers.timeoutSeconds
  Workers.runQueue sources.size workers order fun index =>
    elaborate sources[index]! jobs timeout

def testBaselines : IO Unit := do
  let sources ← Baseline.sourceFilesRecursive checkDir ".lean"
  unless !sources.isEmpty do
    throw <| IO.userError s!"no checks found under {checkDir}"
  let recorded ← Workers.readFailures failedFile
  let runs ← elaborateAll sources (← Workers.workerCount) recorded
  -- Every differing check is reported: one run of the suite names them all.
  let mut mismatches : Array String := #[]
  let mut failed : Array String := #[]
  for source in sources, run in runs do
    match run with
    | .timedOut seconds =>
        -- A check that was killed produced no output to compare, so its
        -- baseline is left as it is, `UB=1` or not, and the run fails.
        mismatches := mismatches.push s!"{source} timed out after {seconds} s"
        failed := failed.push (source.fileStem.getD "")
    | .ended _ output =>
        if let .error error ←
            (Baseline.checkOutput (source.withExtension "exp") output).toBaseIO then
          mismatches := mismatches.push (toString error)
          failed := failed.push (source.fileStem.getD "")
  Workers.writeFailures failedFile recorded (sources.map (·.fileStem.getD "")) failed
  unless mismatches.isEmpty do
    throw <| IO.userError <|
      s!"{mismatches.size} of {sources.size} checks did not pass:\n"
        ++ "\n".intercalate mismatches.toList

end VerexE2ETests.Check
