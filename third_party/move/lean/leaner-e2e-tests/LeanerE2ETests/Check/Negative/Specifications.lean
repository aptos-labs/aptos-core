-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! Port of v0 Negative/Specifications. Global observations and modifies
clauses require a Key resource. V0's rejection of `old(address)` is retired:
the current logical semantics supports pre-state observations of values, so
that original case is retained below as an admission-free positive proof. -/

open Lean Elab Command LeanerLang in
run_cmd do
  let env ← getEnv
  for (name, clause, expected) in #[
      ("bad_exists", "    ensures exists<u64>(address)\n",
        "in fun unchanged: Move global operation resource does not have Key"),
      ("bad_modifies", "    modifies global<u64>(address)\n    ensures true\n",
        "in fun unchanged: Move specification global resource does not have Key")] do
    let source := "leaner module 0x42::" ++ name ++ " where\n" ++
      "  fun unchanged(address : Address) -> Bool := address == address\n" ++
      "  spec unchanged where\n" ++ clause ++ "    aborts_if false\n"
    let stx ← match Parser.runParserCategory env `command source with
      | .ok stx => pure stx
      | .error reason => throwError "specification fixture did not parse: {reason}"
    let parsed ← match compilationUnitOfSyntax stx "<specification rejection>" with
      | .ok parsed => pure parsed
      | .error (_, reason) => throwError "specification fixture did not elaborate: {reason}"
    match compile parsed with
    | .error (.lir diagnostics) =>
      unless diagnostics.size == 1 && diagnostics[0]!.code == "LIR-SEMANTIC-ABILITY" &&
          diagnostics[0]!.message == expected && diagnostics[0]!.primary.isSome do
        throwError "wrong specification rejection for {name}: {repr diagnostics}"
    | .error reason => throwError "wrong specification rejection stage: {repr reason}"
    | .ok _ => throwError "non-resource specification was accepted: {name}"

set_option leaner.route "native"

leaner module 0x42::specification_observations where
  fun unchanged(address : Address) -> Bool := address == address
  spec unchanged where
    ensures result && old(address) == address
    aborts_if false
  verify unchanged

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  assertRuns `«0x42».specification_observations #[
    ⟨"unchanged", #[.address "0x0"], .returned #[.bool true], {}⟩,
    ⟨"unchanged", #[.address "0x2"], .returned #[.bool true], {}⟩]

open Lean Elab Command LeanerLang in
run_cmd do
  let env ← getEnv
  let proof := `«0x42».specification_observations.unchanged.verified
  unless env.contains proof do throwError "missing old-value proof"
  if (← collectAxioms proof).contains ``sorryAx then
    throwError "old-value proof contains an admission"
  let some unit := registeredUnit? env `«0x42».specification_observations
    | throwError "missing old-value fixture"
  let printed ← match Print.render env unit with
    | .ok value => pure value
    | .error reason => throwError "old-value printing failed: {repr reason}"
  let formatted ← match Print.formatSource env printed with
    | .ok value => pure value
    | .error reason => throwError "old-value reimport failed: {repr reason}"
  unless printed == formatted do throwError "old-value source is not a fixed point"
