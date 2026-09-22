-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport
import LeanerRust.Profile

/-! Port of v0 Negative/Borrows: source-level lifetime and lexical-shadowing
regressions. Invalid aliases are checked at execution preparation, not merely
when their source unit is materialized. -/

set_option leaner.route "native"

leaner module 0x42::borrow_diagnostics where
  fun discard_competing_handle() -> u64 := do
    let mut owner : u64 := 0
    let selected := &mut owner
    let _discarded := &mut owner
    *selected := 1
    let result := *selected
    return result
  spec discard_competing_handle where
    ensures result == 1
    aborts_if false
  verify discard_competing_handle

  fun discard_and_restore_owner() -> u64 := do
    let mut owner : u64 := 7
    let selected := &mut owner
    let _first := &mut owner
    let _second := &mut owner
    *selected := 9
    return owner
  spec discard_and_restore_owner where
    ensures result == 9
    aborts_if false
  verify discard_and_restore_owner

  fun shadowed_mutable_reference() -> u64 := do
    let mut owner : u64 := 0
    let valueRef := &mut owner
    *valueRef := 1
    let valueRef : u64 := 2
    let output := valueRef
    return output
  spec shadowed_mutable_reference where
    ensures result == 2
    aborts_if false
  verify shadowed_mutable_reference

  fun shadowed_reference_initializer() -> u64 := do
    let mut owner : u64 := 0
    let valueRef := &mut owner
    let valueRef := *valueRef
    let output := valueRef
    return output
  spec shadowed_reference_initializer where
    ensures result == 0
    aborts_if false
  verify shadowed_reference_initializer

  fun nested_shadowed_reference(takeBranch : Bool) -> u64 := do
    let mut owner : u64 := 0
    let valueRef := &mut owner
    *valueRef := 1
    if takeBranch then
      let valueRef : u64 := 2
      let _ignored := valueRef
    let output := *valueRef
    return output
  spec nested_shadowed_reference where
    ensures result == 1
    aborts_if false
  verify nested_shadowed_reference

  fun pattern_shadowed_reference() -> u64 := do
    let mut owner : u64 := 0
    let valueRef := &mut owner
    *valueRef := 1
    let pair := (2, 3)
    let (valueRef, other) := pair
    return valueRef + other
  spec pattern_shadowed_reference where
    ensures result == 5
    aborts_if false
  verify pattern_shadowed_reference

leaner module 0x42::borrow_conflict where
  fun poisoned_use() -> u64 := do
    let mut owner : u64 := 0
    let selected := &mut owner
    let poisoned := &mut owner
    *selected := 1
    let result := *poisoned
    return result
  spec poisoned_use where
    ensures true

leaner namespace borrow_conflict::rust using rust where
  fun unused_competing_handle() -> u64 := do
    let mut owner : u64 := 0
    let selected := &mut owner
    let _discarded := &mut owner
    *selected := 1
    return owner

open Lean Elab Command LeanerIR in
set_option maxHeartbeats 1000 in
run_cmd do
  let some unit := LeanerLang.registeredUnit? (← getEnv) `«0x42».borrow_conflict
    | throwError "missing invalid alias fixture"
  match Validation.prepareExecution #[Move.semantics] unit with
  | .ok _ => throwError "overlapping mutable aliases were accepted"
  | .error diagnostics =>
    unless diagnostics.size == 1 && diagnostics.all (fun diagnostic =>
        diagnostic.code == "LIR-SEMANTIC-BORROW-CONFLICT" &&
          diagnostic.message == "borrow conflicts with an active LeanerIR.ReferenceKind.mutable loan" &&
          diagnostic.primary.isSome && diagnostic.related.size == 1) do
      throwError "wrong alias rejection: {repr diagnostics}"
  let some rustUnit := LeanerLang.registeredUnit? (← getEnv) `borrow_conflict.rust
    | throwError "missing Rust alias fixture"
  match Validation.prepareExecution #[Rust.semantics] rustUnit with
  | .ok _ => throwError "Move unused-handle rule leaked into Rust"
  | .error diagnostics =>
    unless diagnostics.any (fun diagnostic =>
        diagnostic.code == "LIR-SEMANTIC-BORROW-CONFLICT" &&
          diagnostic.primary.isSome) do
      throwError "wrong Rust alias rejection: {repr diagnostics}"

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  assertRuns `«0x42».borrow_diagnostics #[
    ⟨"discard_competing_handle", #[], .returned #[.integer 1], {}⟩,
    ⟨"discard_and_restore_owner", #[], .returned #[.integer 9], {}⟩,
    ⟨"shadowed_mutable_reference", #[], .returned #[.integer 2], {}⟩,
    ⟨"shadowed_reference_initializer", #[], .returned #[.integer 0], {}⟩,
    ⟨"nested_shadowed_reference", #[.bool false], .returned #[.integer 1], {}⟩,
    ⟨"nested_shadowed_reference", #[.bool true], .returned #[.integer 1], {}⟩,
    ⟨"pattern_shadowed_reference", #[], .returned #[.integer 5], {}⟩]
  assertRunsState `«0x42».borrow_diagnostics #[
    ⟨"discard_competing_handle", #[], .returned #[.integer 1], {}, { nextLoan := 2 }⟩,
    ⟨"discard_and_restore_owner", #[], .returned #[.integer 9], {}, { nextLoan := 3 }⟩]

open Lean Elab Command LeanerLang in
run_cmd do
  let env ← getEnv
  for function in ["discard_competing_handle", "discard_and_restore_owner", "shadowed_mutable_reference",
      "shadowed_reference_initializer", "nested_shadowed_reference", "pattern_shadowed_reference"] do
    let proof := ((`«0x42».borrow_diagnostics).str function).str "verified"
    unless env.contains proof do throwError "missing source borrow proof: {proof}"
    if (← collectAxioms proof).contains ``sorryAx then
      throwError "source borrow proof contains an admission: {proof}"
  for name in [`«0x42».borrow_diagnostics, `«0x42».borrow_conflict, `borrow_conflict.rust] do
    let some unit := registeredUnit? env name | throwError "missing borrow fixture"
    let printed ← match Print.render env unit with
      | .ok value => pure value
      | .error reason => throwError "borrow printing failed: {repr reason}"
    let formatted ← match Print.formatSource env printed with
      | .ok value => pure value
      | .error reason => throwError "borrow reimport failed: {repr reason}"
    unless printed == formatted do throwError "borrow source is not a fixed point"
