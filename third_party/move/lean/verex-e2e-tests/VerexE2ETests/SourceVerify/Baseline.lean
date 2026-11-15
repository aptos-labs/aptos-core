-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.SourceVerify
import VerexRust.SourceVerify
import VerexIR.TestInfra
import VerexE2ETests.Workers

/-!
# Verifying Move and Rust sources

Every `.move` file and every `.rs` file of this directory is verified as
`verex-move verify` and `verex-rust verify` do it: rendered as VerexSL
(a Rust file with the `.spec.lean` items beside it) and elaborated, a Move
file with its proof file. A Move file's rendering is written where its proof
file's header finds it, `.verex/` beside it. The messages, in the
coordinates of the Move, Rust, specification, or proof file, are the
baseline `<name>.exp`; a source that verifies has none. A proof file is then
elaborated on its own, as the editor elaborates it, its header loading the
rendering just written: its messages are the baseline `<name>.proof.exp`.
The framework's Move standard library is verified in place as a package, its
modules linking the ones they use; its baseline records what does not verify
yet.

The unit of work is a name: a source file's stem, a crate, a package, or the
stale-rendering test, and `VEREX_E2E_FILTER` selects names. One verification
already uses more than a core, so a run of several names is a run of several
processes: the parent verifies each name in a child of this executable and
reports every name that differs, the names recorded as not verifying and the
longest ones first.
-/

namespace VerexE2ETests.SourceVerify

open VerexIR.TestInfra

private def featureDir : System.FilePath :=
  "VerexE2ETests/SourceVerify"

/-- Move packages verified whole in place, each named by its baseline in this
directory: the framework's standard library. -/
private def packages : Array (String × System.FilePath) :=
  #[("MoveStdlib", "../../../../aptos-move/framework/move-stdlib")]

/-- Rust crates verified whole, each named by its baseline in this directory,
with its manifest and package. -/
private def crates : Array (String × System.FilePath × String) :=
  #[("CargoFixture", "../verex-rust/Tests/CargoFixture/Cargo.toml", "verex-rust-cargo-fixture")]

/-- The reports of a crate, with its directory and the temporary rendering's
removed so the baseline is stable. -/
private def crateReportsOf (environment : Lean.Environment) (directory manifest : System.FilePath)
    (package : String) : IO String := do
  let reports ← VerexIR.Rust.SourceVerify.verifyCrate environment
    { manifest, package, output := directory / "crate.raw.json" } (directory / "crate.lean")
  let root ← IO.FS.realPath (manifest.parent.getD ".")
  let text := "\n".intercalate (reports.map (·.render)).toList
  pure <| (text.replace s!"{directory}/" "").replace s!"{root}/" ""

/-- A package's own files: its manifest, and the Move sources and proof files
under `sources`, without its build output and tests. -/
private def packageInput (package path : System.FilePath) : Bool :=
  let relative := path.toString.drop (package.toString.length + 1)
  relative == "Move.toml" || relative == "Move.lock" ||
    relative.startsWith "sources/" &&
      (relative.endsWith ".move" || relative.endsWith ".proof.lean")

/-- The reports of one source, with the temporary rendering's directory
removed so the baseline is stable. -/
private def reportsOf (environment : Lean.Environment) (directory source : System.FilePath) :
    IO String := do
  let reports ← if source.extension == some "move" || (← source.isDir) then
      VerexMove.SourceVerify.verifySource environment source
        (← VerexMove.SourceVerify.renderingOf source)
    else
      VerexIR.Rust.SourceVerify.verifyFile environment source none
        (directory / s!"{source.fileName.getD "source"}.lean")
  let text := "\n".intercalate (reports.map (·.render)).toList
  pure <| text.replace s!"{directory}/" ""

/-- The messages of a proof file elaborated on its own, as the editor
elaborates it, but verifying every item, as the command line does. The
options of `VEREX_OPTIONS` hold here too, so that `verex.jobs` reaches a
proof file's elaboration. -/
private def editorReportsOf (environment : Lean.Environment) (proofFile : System.FilePath) :
    IO String := do
  let inputCtx := Lean.Parser.mkInputContext (← IO.FS.readFile proofFile) proofFile.toString
  let (_, parserState, messages) ← Lean.Parser.parseHeader inputCtx
  let options ← VerexSL.SourceVerify.environmentOptions
    (({} : Lean.Options).setBool `verex.skip_verify false)
  let state ← Lean.Elab.IO.processCommands inputCtx parserState
    (Lean.Elab.Command.mkState environment messages options)
  let reports ← state.commandState.messages.toList.filterMapM fun message => do
    if message.isSilent then return none
    return some ({ file := message.fileName, line := message.pos.line
                   column := message.pos.column + 1, severity := message.severity
                   text := ← message.data.toString } : VerexSL.SourceVerify.Report).render
  pure ("\n".intercalate reports)

/-- A proof file whose Move file changed since its rendering warns at its
header. -/
private def testStaleRendering (environment : Lean.Environment) (directory : System.FilePath) :
    IO Unit := do
  let staged := directory / "stale"
  IO.FS.createDirAll staged
  for name in ["proofs.move", "proofs.proof.lean"] do
    IO.FS.writeFile (staged / name) (← IO.FS.readFile (featureDir / name))
  let source := staged / "proofs.move"
  discard <| VerexMove.SourceVerify.verifySource environment source
    (← VerexMove.SourceVerify.renderingOf source)
  IO.FS.writeFile source ((← IO.FS.readFile source) ++ "\n// changed\n")
  let reports ← editorReportsOf environment (staged / "proofs.proof.lean")
  unless (reports.splitOn "is older than `").length > 1 do
    throw <| IO.userError s!"a stale rendering does not warn:\n{reports}"

/-- A fixture whose stem ends in `_errors` or `_false` is expected to produce
at least one `error:` line.  Zero errors is a harness failure regardless of
`UB=1`, and never removes the `.exp`. -/
private def isErrorFixture (source : System.FilePath) : Bool :=
  let stem := source.fileStem.getD ""
  stem.endsWith "_errors" || stem.endsWith "_false"

private def checkSourceOutput (source : System.FilePath) (output : String) : IO Unit := do
  if isErrorFixture source && !(output.splitOn "\n" |>.any (·.contains "error:")) then
    throw <| IO.userError s!"fixture {source} is an `_errors`/`_false` fixture \
      but produced no `error:` lines"
  Baseline.checkOutput (source.withExtension "exp") output

/-- The name of the stale-rendering test, which is a name of the suite like a
fixture's, so that a selection can run it alone. -/
private def staleRenderingName : String := "StaleRendering"

/-- The names the suite runs: the stems of the `.move` and `.rs` sources, the
crates, the packages, and the stale-rendering test, each one where
`VEREX_E2E_FILTER` admits it. A stem whose source is both a `.move` and a
`.rs` file is one name. -/
private def selectedNames : IO (Array String) := do
  let sources := (← Baseline.sourceFiles featureDir ".move") ++
    (← Baseline.sourceFiles featureDir ".rs")
  let selected ← Baseline.selectedFixtures
  let mut names : Array String := #[]
  for source in sources do
    let stem := source.fileStem.getD ""
    unless names.contains stem do
      names := names.push stem
  for (name, _, _) in crates do
    if selected.all (·.contains name) then
      names := names.push name
  for (name, _) in packages do
    if selected.all (·.contains name) then
      names := names.push name
  if selected.all (·.contains staleRenderingName) then
    names := names.push staleRenderingName
  pure names

/-- Verify one name in this process: a crate, a package, the stale-rendering
test, or a source file's `.move` and `.rs`, a Move file with its proof file.
Throws on a mismatch. -/
private def verifyName (environment : Lean.Environment) (directory : System.FilePath)
    (name : String) : IO Unit := do
  if name == staleRenderingName then
    return ← testStaleRendering environment directory
  if let some (_, manifest, package) := crates.find? (·.1 == name) then
    return ← Baseline.checkOutput (featureDir / s!"{name}.exp")
      (← crateReportsOf environment directory manifest package)
  if let some (_, package) := packages.find? (·.1 == name) then
    let reports ← Baseline.withStagedDirectory package (directory / name)
      (packageInput package) fun staged => reportsOf environment directory staged
    return ← Baseline.checkOutput (featureDir / s!"{name}.exp") reports
  for suffix in [".move", ".rs"] do
    let source := featureDir / (name ++ suffix)
    if ← source.pathExists then
      checkSourceOutput source (← reportsOf environment directory source)
      let proofFile := VerexMove.Proofs.proofFile source
      if source.extension == some "move" && (← proofFile.pathExists) then
        Baseline.checkOutput (source.withExtension "proof.exp")
          (← editorReportsOf environment proofFile)

/-- Verify the names one after another in this process, the first mismatch
ending the run. -/
private def verifyHere (names : Array String) : IO Unit := do
  let environment ← VerexMove.SourceVerify.importEnvironment
  IO.FS.withTempDir fun directory =>
    for name in names do
      verifyName environment directory name

/-- What one name took in a fan-out run, and what its child printed. -/
private structure Outcome where
  name : String
  verified : Bool
  output : String
  milliseconds : Nat
  deriving Inhabited

/-- Milliseconds as seconds with one decimal. -/
private def seconds (milliseconds : Nat) : String :=
  s!"{milliseconds / 1000}.{milliseconds % 1000 / 100}"

/-- The durations of the names a fan-out run verified, one `<milliseconds>
<name>` per line, which the next run orders its queue by. -/
private def durationsFile : System.FilePath :=
  ".lake/verify-durations"

/-- The standing record of the names known not to verify, comma-separated on
one line, so that `VEREX_E2E_FILTER=$(cat .lake/verify-failed)` reruns them. A
fan-out run removes the names it verified and adds the ones it failed; the
names it did not run stay. -/
private def failedFile : System.FilePath :=
  ".lake/verify-failed"

/-- The duration a name without a record counts as: longer than any real
one, so that a new fixture starts first. -/
private def unknownDuration : Nat := 1000000000

private def readDurations : IO (Std.HashMap String Nat) := do
  unless ← durationsFile.pathExists do return {}
  let mut durations := {}
  for line in (← IO.FS.readFile durationsFile).splitOn "\n" do
    match (line.splitOn " ").filter (!·.isEmpty) with
    | [milliseconds, name] =>
        if let some milliseconds := milliseconds.toNat? then
          durations := durations.insert name milliseconds
    | _ => pure ()
  return durations

/-- Record what this run verified: the durations of its names, keeping the
lines of the names it did not run, and the names known not to verify. -/
private def writeRecords (previous : Std.HashMap String Nat) (recorded : Array String)
    (outcomes : Array Outcome) : IO Unit := do
  if let some parent := durationsFile.parent then
    IO.FS.createDirAll parent
  let mut durations := previous
  for outcome in outcomes do
    durations := durations.insert outcome.name outcome.milliseconds
  let lines := (durations.toArray.qsort (·.1 < ·.1)).map fun (name, milliseconds) =>
    s!"{milliseconds} {name}"
  IO.FS.writeFile durationsFile ("\n".intercalate lines.toList ++ "\n")
  Workers.writeFailures failedFile recorded (outcomes.map (·.name))
    ((outcomes.filter (!·.verified)).map (·.name))

/-- The queue order over `names`: the names recorded as not verifying first,
then the longest previous duration first. -/
private def queueOrder (names : Array String) (recorded : Array String)
    (durations : Std.HashMap String Nat) : Array Nat :=
  let key (index : Nat) : Bool × Nat :=
    (recorded.contains names[index]!, durations.getD names[index]! unknownDuration)
  (Array.range names.size).qsort fun a b =>
    let (aFailed, aDuration) := key a
    let (bFailed, bDuration) := key b
    if aFailed != bFailed then aFailed else aDuration > bDuration

/-- The child's `VEREX_OPTIONS`: the caller's, with the threads a worker
plans for itself added where the caller does not set them itself. -/
private def childOptions (jobs : Nat) : IO String := do
  let inherited := (← IO.getEnv "VEREX_OPTIONS").getD ""
  let assignments := (inherited.splitOn ",").filter (!·.isEmpty)
  if assignments.any (·.trimAscii.toString.startsWith "verex.jobs=") then
    return inherited
  return ",".intercalate (assignments ++ [s!"verex.jobs={jobs}"])

/-- Verify one name in a child of this executable, which runs it in its own
process by the in-process rule: one name, one worker. Everything but the
suite, the name, the worker count, and `verex.jobs` is inherited, so `UB=1`
updates the baselines in the child. A child that outlives `timeout` is
killed, with everything it started, and the name fails. -/
private def verifyInChild (name : String) (jobs timeout : Nat) : IO Outcome := do
  let options ← childOptions jobs
  let started ← IO.monoMsNow
  let run ← Workers.runUnderTimeout timeout {
    cmd := (← IO.appPath).toString
    env := #[("VEREX_E2E_SUITE", some "verify"), ("VEREX_E2E_FILTER", some name),
      ("VEREX_E2E_JOBS", some "1"), ("VEREX_E2E_MANAGED", some "1"),
      ("VEREX_OPTIONS", some options)] }
  let (verified, output) := match run with
    | .ended exitCode output => (exitCode == 0, output)
    | .timedOut seconds => (false, s!"timed out after {seconds} s\n")
  return { name, verified, output, milliseconds := (← IO.monoMsNow) - started }

/-- One finished name's progress report: its line, and the child's output
below it where it failed. -/
private def reportOf (outcome : Outcome) : String :=
  let line := if outcome.verified then "ok" else "FAIL"
  let head := s!"{line} {outcome.name} ({seconds outcome.milliseconds}s)\n"
  if outcome.verified || outcome.output.isEmpty then head
  else if outcome.output.endsWith "\n" then head ++ outcome.output
  else head ++ outcome.output ++ "\n"

/-- Verify every name in a child of its own, `workers` at a time, and report
each one as it ends. Throws naming every name that failed. -/
private def verifyInChildren (names : Array String) (workers : Nat) : IO Unit := do
  let durations ← readDurations
  let recorded ← Workers.readFailures failedFile
  let order := queueOrder names recorded durations
  let jobs := Workers.targetJobs workers
  let timeout ← Workers.timeoutSeconds
  let stdout ← IO.getStdout
  stdout.putStrLn s!"verify: {names.size} fixtures, {workers} at a time, verex.jobs={jobs}"
  stdout.flush
  let started ← IO.monoMsNow
  let outcomes ← Workers.runQueue names.size workers order
    (work := fun index => verifyInChild names[index]! jobs timeout)
    (finished := fun _ outcome => do stdout.putStr (reportOf outcome); stdout.flush)
  writeRecords durations recorded outcomes
  let failed := (outcomes.filter (!·.verified)).map (·.name)
  stdout.putStrLn s!"verify: {names.size} fixtures in \
    {seconds ((← IO.monoMsNow) - started)}s, {failed.size} failed"
  unless failed.isEmpty do
    stdout.putStrLn s!"verify: failed: {",".intercalate failed.toList}"
  stdout.flush
  unless failed.isEmpty do
    throw <| IO.userError
      s!"{failed.size} of {names.size} fixtures did not verify as their baselines \
        record: {",".intercalate failed.toList}"

def testBaselines : IO Unit := do
  let names ← selectedNames
  if names.isEmpty then
    throw <| IO.userError s!"no fixtures found under {featureDir}"
  let workers := min (← Workers.workerCount) names.size
  if workers ≤ 1 then verifyHere names else verifyInChildren names workers

end VerexE2ETests.SourceVerify
