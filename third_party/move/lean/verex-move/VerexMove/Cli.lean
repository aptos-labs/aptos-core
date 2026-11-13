-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.SourceVerify

/-!
# The `verex-move` command

`verex-move verify <source>` exports a Move file or package directory
through compiler-v2, renders its modules as VerexSL where its proof files
find them (`.verex/<file>.lean` beside a file, `build/verex-verify.lean` of a
package, or `--output`), verifies every specified function, and reports each
message at its position in the Move sources.
-/

namespace VerexMove.Cli

open VerexSL.SourceVerify

private def usage : String :=
  "usage: verex-move verify <source.move | package directory> [--output <generated.lean>] \
   [--export <xast directory>] [--filter <file name part>] [--modules <a::m>,...] \
   [--heartbeats <thousands>] [--jobs <targets at once>] [--ignore-proofs] [--render-only] \
   [--dev] [--namespaces <directory>]\n\
   verex-move render-namespaces <package directory> <directory> [--dev]\n\
   verex-move compile-namespace <directory> <module> [--heartbeats <thousands>] \
   [--ignore-proofs]\n\
   verex-move elaborate <file.lean> [--namespaces <directory>]"

private def fail (message : String) : IO α := throw <| IO.userError s!"{message}\n{usage}"

/-- The options of `verify`: where the rendering goes, and an existing
export to read the package from. -/
private structure VerifyOptions where
  output : Option System.FilePath := none
  exported : Option System.FilePath := none
  /-- Only the package's modules whose source file name contains this are
  verified; the others are linked. -/
  filter : Option String := none
  /-- Write the rendering without verifying it. -/
  renderOnly : Bool := false
  /-- The heartbeat budget of a function without `pragma heartbeats`, in
  thousands of `maxHeartbeats` units. -/
  heartbeats : Option Nat := none
  /-- Only these modules of the package are verified, and only what
  verifying them reads is exported. -/
  modules : Array String := #[]
  /-- The package is compiled in dev mode, with its `[dev-addresses]` and
  `[dev-dependencies]`. -/
  dev : Bool := false
  /-- Every function is verified automatically, its authored proof ignored. -/
  ignoreProofs : Bool := false
  /-- How many targets of a module are proved at once; half the hardware
  threads without. -/
  jobs : Option Nat := none
  /-- The module named by `modules` is verified over the namespaces compiled
  under this directory. -/
  namespaces : Option System.FilePath := none

private partial def parseOptions : List String → VerifyOptions → IO VerifyOptions
  | [], options => pure options
  | "--output" :: path :: rest, options =>
      if options.output.isSome then fail "--output may be specified only once"
      else parseOptions rest { options with output := some path }
  | "--export" :: path :: rest, options =>
      if options.exported.isSome then fail "--export may be specified only once"
      else parseOptions rest { options with exported := some path }
  | "--filter" :: part :: rest, options =>
      if options.filter.isSome then fail "--filter may be specified only once"
      else parseOptions rest { options with filter := some part }
  | "--namespaces" :: path :: rest, options =>
      if options.namespaces.isSome then fail "--namespaces may be specified only once"
      else parseOptions rest { options with namespaces := some path }
  | "--render-only" :: rest, options => parseOptions rest { options with renderOnly := true }
  | "--dev" :: rest, options => parseOptions rest { options with dev := true }
  | "--ignore-proofs" :: rest, options => parseOptions rest { options with ignoreProofs := true }
  | "--modules" :: names :: rest, options =>
      if !options.modules.isEmpty then fail "--modules may be specified only once"
      else parseOptions rest { options with modules := (names.splitOn ",").toArray }
  | "--jobs" :: value :: rest, options =>
      if options.jobs.isSome then fail "--jobs may be specified only once"
      else match value.toNat? with
        | some jobs => if jobs == 0 then fail "--jobs expects a positive number"
            else parseOptions rest { options with jobs := some jobs }
        | none => fail s!"--jobs expects a number, not `{value}`"
  | "--heartbeats" :: value :: rest, options =>
      if options.heartbeats.isSome then fail "--heartbeats may be specified only once"
      else match value.toNat? with
        | some heartbeats => parseOptions rest { options with heartbeats := some heartbeats }
        | none => fail s!"--heartbeats expects a number, not `{value}`"
  | argument :: _, _ => fail s!"unknown verify argument `{argument}`"

private def verifySource (source : System.FilePath) (options : VerifyOptions) : IO UInt32 := do
  unless options.modules.isEmpty do
    if options.filter.isSome then fail "--modules and --filter exclude each other"
    if options.exported.isSome then fail "--modules exports the package itself; drop --export"
  if options.dev && options.exported.isSome then
    fail "--dev compiles the package itself; drop --export"
  if options.namespaces.isSome && options.modules.size != 1 then
    fail "--namespaces verifies one module, named by --modules"
  let start ← IO.monoNanosNow
  let output ← match options.output with
    | some output => pure output
    | none => VerexMove.SourceVerify.renderingOf source
  let reports ← match options.namespaces, options.modules with
    | some root, #[module] =>
        VerexMove.SourceVerify.verifyOverNamespaces source output root module options.renderOnly
          options.heartbeats options.dev options.ignoreProofs options.jobs
    | _, _ =>
        let environment ← VerexSL.Perf.withPhase .load VerexMove.SourceVerify.importEnvironment
        VerexMove.SourceVerify.verifySource environment source output
          options.exported options.filter options.renderOnly options.heartbeats options.modules
          options.dev options.ignoreProofs options.jobs
  for report in reports do IO.println report.render
  -- The rendering and the wall time per phase are reports on the run, not
  -- its messages.
  -- Import errors return reports before a rendering is written. Report its
  -- path as generated only on success; an existing file may be from an older run.
  unless reports.any (·.severity == .error) do
    IO.eprintln s!"verex-move: generated {output}"
  IO.eprintln s!"verex-move: {← VerexSL.Perf.phaseSummary ((← IO.monoNanosNow) - start)}"
  -- A completed diagnostic run is distinct from an import failure or crash.
  IO.eprintln "verex-move: verification complete"
  pure <| if reports.any (·.severity == .error) then 1 else 0

/-- Render a package's modules for `compile-namespace`, and print each as one
line of JSON: its path, Lean module, file, and the modules it imports. -/
private def renderNamespaces (source : System.FilePath) (root : System.FilePath) (dev : Bool) :
    IO UInt32 := do
  let environment ← VerexMove.SourceVerify.importEnvironment
  match ← VerexMove.SourceVerify.renderPackageNamespaces environment source root dev with
  | .error message =>
      IO.eprintln message
      return 1
  | .ok rendered =>
      for entry in rendered do
        IO.println <| Lean.Json.compress <| Lean.Json.mkObj [
          ("path", Lean.toJson entry.path),
          ("module", Lean.toJson (toString entry.module)),
          ("file", Lean.toJson entry.file.toString),
          ("imports", Lean.toJson (entry.imports.map toString))]
      return 0

/-- Compile a rendered namespace to its `.olean`, reporting its messages. -/
private def compileNamespace (root : System.FilePath) (module : String)
    (options : VerifyOptions) : IO UInt32 := do
  let messages ← VerexSL.SourceVerify.compileNamespace
    VerexMove.SourceVerify.compilationLibraries root (module.toName) options.heartbeats
    options.ignoreProofs
  for message in messages do IO.print (← message.toString)
  return if messages.any (·.severity == .error) then 1 else 0

def run (arguments : List String) : IO UInt32 := do
  match arguments with
  | ["render-namespaces", source, root] => renderNamespaces source root false
  | ["render-namespaces", source, root, "--dev"] => renderNamespaces source root true
  | "compile-namespace" :: root :: module :: rest =>
      compileNamespace root module (← parseOptions rest {})
  | ["snapshot-modules"] => do
      let environment ← VerexMove.SourceVerify.importEnvironment
      for name in environment.header.moduleNames do
        IO.println (← Lean.findOLean name)
      return 0
  | "verify" :: source :: rest => verifySource source (← parseOptions rest {})
  | ["elaborate", file] => do
      let environment ← VerexMove.SourceVerify.importEnvironment
      let messages ← VerexSL.SourceVerify.elaborateFile environment file
      for message in messages do IO.print (← message.toString)
      return if messages.any (·.severity == .error) then 1 else 0
  | ["elaborate", file, "--namespaces", root] => do
      let messages ← VerexSL.SourceVerify.elaborateFileOver
        VerexMove.SourceVerify.compilationLibraries file root
      for message in messages do IO.print (← message.toString)
      return if messages.any (·.severity == .error) then 1 else 0
  | _ => fail "expected a command"

end VerexMove.Cli
