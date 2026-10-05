-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove.SourceVerify
import LeanerRust.SourceVerify
import LeanerE2ETests.CheckSupport

/-!
# `leaner-bench`

Runs one problem of the verification benchmark natively and writes what it
cost as JSON (`designs/verification-benchmarks.md`): wall time and
heartbeats per phase, the verified targets with their wall time and
heartbeats, and the errors reported.

```text
leaner-bench move <package> --modules <a::m>,… [--dev] --out <result.json>
leaner-bench lean <file.lean> --out <result.json>
leaner-bench rust <file.rs> [--spec <file.spec.lean>] --out <result.json>
leaner-bench warmup
```

`warmup` imports what the problems import and exits, so that the first
problem measured does not pay for a cold file cache.
-/

namespace LeanerBench

open Lean LeanerLang

private def usage : String :=
  "usage: leaner-bench move <package> --modules <a::m>,... [--dev] --out <result.json>\n       \
   leaner-bench lean <file.lean> --out <result.json>\n       \
   leaner-bench rust <file.rs> [--spec <file.spec.lean>] --out <result.json>\n       \
   leaner-bench warmup"

private structure Request where
  kind : String
  source : System.FilePath
  modules : Array String := #[]
  /-- The Move package is compiled in dev mode. -/
  dev : Bool := false
  specFile : Option System.FilePath := none
  out : Option System.FilePath := none

private partial def parseOptions : List String → Request → Except String Request
  | [], request => .ok request
  | "--modules" :: names :: rest, request =>
      parseOptions rest { request with modules := (names.splitOn ",").toArray }
  | "--dev" :: rest, request => parseOptions rest { request with dev := true }
  | "--spec" :: path :: rest, request =>
      parseOptions rest { request with specFile := some path }
  | "--out" :: path :: rest, request => parseOptions rest { request with out := some path }
  | argument :: _, _ => .error s!"unknown argument `{argument}`"

/-- A message of the run: its severity and its rendering. -/
private structure Message where
  isError : Bool
  text : String

/-- Elaborate a Lean file with the imports of its header, as `lean` does,
but natively. -/
private unsafe def elaborateFile (path : System.FilePath) : IO (Array Message) := do
  let input ← IO.FS.readFile path
  let inputCtx := Parser.mkInputContext input path.toString
  let (header, parserState, messages) ← Parser.parseHeader inputCtx
  let (environment, messages) ← Perf.withPhase .load do
    enableInitializersExecution
    initSearchPath (← findSysroot)
    Elab.processHeader header {} messages inputCtx (trustLevel := 1024)
      (mainModule := `LeanerBenchProblem)
  let state ← Elab.IO.processCommands inputCtx parserState
    (Elab.Command.mkState environment messages {})
  state.commandState.messages.toList.toArray.filterMapM fun message => do
    if message.isSilent then return none
    let position := s!"{path}:{message.pos.line}:{message.pos.column + 1}"
    return some { isError := message.severity == .error,
                  text := s!"{position}: {← message.data.toString}" }

/-- Verify a Move package's modules or a Rust file through its frontend. -/
private def verifySource (request : Request) : IO (Array Message) := do
  let environment ← Perf.withPhase .load SourceVerify.importLeanerLang
  IO.FS.withTempDir fun directory => do
    let output := directory / "rendering.lean"
    let reports ← match request.kind with
      | "move" => do
          if request.modules.isEmpty then throw <| IO.userError "a move problem names --modules"
          LeanerMove.SourceVerify.verifySource environment request.source output
            (modules := request.modules) (dev := request.dev)
      | _ =>
          LeanerIR.Rust.SourceVerify.verifyFile environment request.source request.specFile
            output
    return reports.map fun report =>
      { isError := report.severity == .error, text := report.render }

private def phaseJson (values : Array (String × Nat)) (total : Nat) : Json :=
  Json.mkObj ((values.map fun (name, value) => (name, toJson value)).toList ++
    [("total", toJson total)])

/-- The verified targets: the samples of one function, its typed theorem and
its transport, summed. -/
private def targetsJson (samples : Array Perf.Sample) : Array Json := Id.run do
  let mut targets : Array (String × Nat × Nat) := #[]
  for sample in samples do
    let words := sample.target.splitOn " "
    let target := " ".intercalate (words.take (words.length - 1))
    match targets.findIdx? (·.1 == target) with
    | some index => targets := targets.modify index fun (name, heartbeats, wall) =>
        (name, heartbeats + sample.heartbeats, wall + sample.elapsedMs)
    | none => targets := targets.push (target, sample.heartbeats, sample.elapsedMs)
  return targets.map fun (target, heartbeats, wall) =>
    Json.mkObj [("target", toJson target), ("wall_ms", toJson wall),
      ("heartbeats", toJson heartbeats)]

unsafe def run (request : Request) : IO UInt32 := do
  let some out := request.out | throw <| IO.userError s!"--out is required\n{usage}"
  Perf.recorded.set #[]
  Perf.measuring.set true
  Perf.countingObjects.set false
  let started ← IO.monoNanosNow
  let messages ← match request.kind with
    | "lean" => elaborateFile request.source
    | "move" | "rust" => verifySource request
    | kind => throw <| IO.userError s!"unknown problem kind `{kind}`\n{usage}"
  let wall := (← IO.monoNanosNow) - started
  let phases ← Perf.phaseTotals
  -- Commands elaborate on threads of their own, so the heartbeats of the
  -- run are those charged to its phases.
  let beats := phases.foldl (fun sum (_, _, heartbeats) => sum + heartbeats) 0
  let samples ← Perf.recorded.get
  let errors := messages.filter (·.isError)
  let result := Json.mkObj [
    ("status", toJson (if errors.isEmpty then "verified" else "failed")),
    ("wall_ms", phaseJson (phases.map fun (phase, nanos, _) => (phase.name, nanos / 1000000))
      (wall / 1000000)),
    ("heartbeats", phaseJson (phases.map fun (phase, _, heartbeats) => (phase.name, heartbeats))
      beats),
    ("targets", toJson (targetsJson samples)),
    ("errors", toJson errors.size),
    ("error_messages", toJson ((errors.extract 0 10).map (·.text)))]
  IO.FS.writeFile out (result.pretty ++ "\n")
  for message in messages do IO.eprintln message.text
  return 0

end LeanerBench

unsafe def main (arguments : List String) : IO UInt32 := do
  match arguments with
  | ["warmup"] =>
      Lean.enableInitializersExecution
      Lean.initSearchPath (← Lean.findSysroot)
      discard <| Lean.importModules #[{ module := `LeanerE2ETests.CheckSupport }] {}
        (trustLevel := 1024) (loadExts := true)
      return 0
  | kind :: source :: rest =>
      match LeanerBench.parseOptions rest { kind, source } with
      | .ok request => LeanerBench.run request
      | .error message => throw <| IO.userError s!"{message}\n{LeanerBench.usage}"
  | _ => throw <| IO.userError LeanerBench.usage
