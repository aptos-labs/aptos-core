-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexRust.Tests.DifferentialCheck
import VerexRust.Tests.Programs.DifferentialCasts

/-!
# The differential check, natively

Run by `lake test`. `--verbose` prints a line per function; a directory argument
checks its fixtures instead of the suite's. `VEREX_DIFFERENTIAL=smoke` runs the most
telling boundary values only, for the smoke suite.
-/

open VerexIR.Rust.Tests.Differential

/-- The disagreements known, each to be resolved by removing it here. All are
functions no input runs to a return or an abort. -/
def knownDisagreements : Array String := #[
  -- The dependency callee has no executable body.
  "dependency_calls::checked", "dependency_calls::copied", "dependency_calls::length",
  "dependency_calls::or_zero", "dependency_calls::push_one", "dependency_projection::element",
  "declared_lifetimes::vec_index", "boxes::make", "polyfill::is_none",
  -- The inputs cover no `String` or `Vec`.
  "destructors::string", "owned_vector::named", "owned_vector::view",
  "string_length::byte_length", "string_slice::identity", "string_slice::identity_mut",
  -- No denotation.
  "destructors::list", "destructors::plain", "drop::consume",
  -- It diverges, so every input runs out of fuel.
  "never::spin_forever"]

/-- What refuses a rendering that verification would need to execute, known, each
to be resolved by removing it here. A Rust local first written by a declaration
and later mutably borrowed renders without `mut`. -/
def knownRefusals : Array String := #[]

/-- The fixtures the check cannot run in full, each to be resolved by removing it
here unless it is rejected on purpose. -/
def knownUnrun : Array String := #[
  -- A mutable borrow of an unsized place is rejected.
  "unsize_mutable"]

/-- The report's failures: a new disagreement, a known one gone, or, in a
fixture of this check, a function no input decided. -/
def failures (own reports : Array FixtureReport) : Array String := Id.run do
  let mut found : Array String := #[]
  let disagreeing := reports.flatMap fun report =>
    report.disagreements.map fun (label, line) => (s!"{report.name}::{label}", line)
  for (label, line) in disagreeing do
    unless knownDisagreements.contains label do
      found := found.push s!"DISAGREE {label}: {line}"
  for label in knownDisagreements do
    unless disagreeing.any (·.1 == label) do
      found := found.push s!"no longer disagrees, remove it from the known list: {label}"
  for known in knownRefusals do
    unless own.any (·.refused.any (·.endsWith known)) do
      found := found.push s!"no longer refused, remove it from the known list: {known}"
  for report in reports do
    if knownUnrun.contains report.name then
      if report.unrun.isEmpty then
        found := found.push s!"now runs, remove it from the known list: {report.name}"
    else
      for line in report.unrun do
        found := found.push s!"not run: {line}"
  for report in own do
    for label in report.unchecked do
      found := found.push s!"no input decided {report.name}::{label}"
    for message in report.refused do
      unless knownRefusals.any (message.endsWith ·) do
        found := found.push s!"the rendering of {report.name} does not execute: {message}"
  let total (field : FixtureReport → Nat) := reports.foldl (· + field ·) 0
  if total (·.returns) == 0 || total (·.aborts) == 0 || total (·.noOutcome) == 0 then
    found := found.push "the fixtures no longer return, abort, and have no outcome"
  return found

def main (args : List String) : IO UInt32 := do
  let environment ← VerexSL.SourceVerify.importLibraries
    #[`VerexSL, `VerexRust.Tests.Programs.DifferentialCasts]
  let smoke := (← IO.getEnv "VEREX_DIFFERENTIAL") == some "smoke"
  let coverage (exhaustive : Array String) (name : String) : Coverage :=
    if smoke then .smoke else if exhaustive.contains name then .exhaustive else .boundary
  let start ← IO.monoMsNow
  let verbose := args.contains "--verbose"
  let directories := args.filter (!·.startsWith "--")
  let own ← if directories.isEmpty then
      checkDirectory environment "Tests/Differential" (coverage #["bytes", "casts", "method_evidence"])
    else
      pure #[]
  let evidence ← if directories.isEmpty then
      checkDirectory environment "rust-exporter/tests/evidence-unit" (coverage #[])
    else pure #[]
  let mut reports := own ++ evidence
  if directories.isEmpty then
    reports := reports ++ (← checkDirectory environment "rust-exporter/tests/raw-unit" (coverage #[]))
    let some casts := VerexSL.registeredUnit? environment `differential_casts
      | throw <| IO.userError "the authored casts are not registered"
    reports := reports.push (← checkAuthored "differential_casts" casts (coverage #["differential_casts"] "differential_casts"))
  for directory in directories do
    reports := reports ++ (← checkDirectory environment directory (coverage #[]))
  if verbose then
    for report in reports do
      IO.println (String.intercalate "\n" report.lines.toList)
      for (label, line) in report.disagreements do
        IO.println s!"  DISAGREE {report.name}::{label}: {line}"
  let total (field : FixtureReport → Nat) := reports.foldl (· + field ·) 0
  IO.println s!"differential check: {total (·.inputs)} inputs, {total (·.returns)} return, \
    {total (·.aborts)} abort, {total (·.noOutcome)} no outcome, \
    {total (·.inconclusive)} inconclusive, {total (·.interpreterErrors)} interpreter errors, \
    in {(← IO.monoMsNow) - start} ms"
  if directories.isEmpty then
    let mut found := failures (own ++ evidence) reports
    -- Dictionary fixtures must exercise every input, not just one successful path.
    for report in evidence ++ own.filter (·.name == "method_evidence") do
      if report.inputs == 0 || report.inconclusive != 0 || report.interpreterErrors != 0 then
        found := found.push s!"incomplete method evidence comparison: {report.name}"
    for line in found do
      IO.eprintln line
    return if found.isEmpty then 0 else 1
  else
    return if reports.any (!·.disagreements.isEmpty) then 1 else 0
