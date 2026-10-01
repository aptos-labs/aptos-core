-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove.SourceVerify

/-!
# The `leaner-move` command

`leaner-move verify <source>` exports a Move file or package directory
through compiler-v2, renders its modules as LeanerLang beside it
(`<source>.lean`, or `--output`), verifies every specified function, and
reports each message at its position in the Move sources.
-/

namespace LeanerMove.Cli

open LeanerLang.SourceVerify

private def usage : String :=
  "usage: leaner-move verify <source.move | package directory> [--output <generated.lean>] \
   [--export <xast directory>] [--filter <file name part>] [--modules <a::m>,...] \
   [--heartbeats <thousands>] [--render-only]"

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
  | "--render-only" :: rest, options => parseOptions rest { options with renderOnly := true }
  | "--modules" :: names :: rest, options =>
      if !options.modules.isEmpty then fail "--modules may be specified only once"
      else parseOptions rest { options with modules := (names.splitOn ",").toArray }
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
  let start ← IO.monoNanosNow
  let output := options.output.getD (source.addExtension "lean")
  let environment ← LeanerLang.Perf.withPhase .load importLeanerLang
  let reports ← LeanerMove.SourceVerify.verifySource environment source output
    options.exported options.filter options.renderOnly options.heartbeats options.modules
  for report in reports do IO.println report.render
  IO.println s!"leaner-move: generated {output}"
  -- The wall time per phase is a report on the run, not one of its messages.
  IO.eprintln s!"leaner-move: {← LeanerLang.Perf.phaseSummary ((← IO.monoNanosNow) - start)}"
  pure <| if reports.any (·.severity == .error) then 1 else 0

def run (arguments : List String) : IO UInt32 := do
  match arguments with
  | "verify" :: source :: rest => verifySource source (← parseOptions rest {})
  | _ => fail "expected a command"

end LeanerMove.Cli
