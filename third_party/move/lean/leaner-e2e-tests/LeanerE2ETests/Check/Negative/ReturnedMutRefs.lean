-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! Port of all five v0 Negative/ReturnedMutRefs categories. Move's VM
rejects local and global roots at return, but permits parameter-derived
references. The positive companion exercises a returned handle that still
has a local alias when the callee frame is finalized. -/

set_option leaner.route "native"

leaner module 0x42::returned_local_invalid where
  fun return_local_reference() -> &mut u64 := do
    let mut owner : u64 := 0
    let escaped := &mut owner
    return escaped
  spec return_local_reference where
    ensures true
    aborts_if false

leaner module 0x42::returned_input_invalid where
  fun choose_input(takeLeft : Bool, left : &mut u64, right : &mut u64) -> &mut u64 :=
    if takeLeft then left else right
  spec choose_input where
    ensures true
    aborts_if false
  fun use_suspended_input(left : &mut u64, right : &mut u64) -> Unit := do
    let returned := choose_input(true, left, right)
    *left := 1
    *returned := 2
  spec use_suspended_input where
    ensures true
    aborts_if false

leaner module 0x42::returned_pair_invalid where
  fun return_reference_pair(left : &mut u64, right : &mut u64) -> (&mut u64, &mut u64) :=
    (left, right)
  spec return_reference_pair where
    ensures true
    aborts_if false
  fun use_pair_input(left : &mut u64, right : &mut u64) -> Unit := do
    let (returnedLeft, returnedRight) := return_reference_pair(left, right)
    *left := 1
    *returnedLeft := 2
    *returnedRight := 3
  spec use_pair_input where
    ensures true
    aborts_if false

leaner module 0x42::returned_native_invalid where
  native fun native_return_reference(slot : &mut u64) -> &mut u64
  spec native_return_reference where
    ensures true
    aborts_if false
  fun call_native_return_reference(slot : &mut u64) -> Unit := do
    let returned := native_return_reference(slot)
    *returned := 3
  spec call_native_return_reference where
    ensures true
    aborts_if false

leaner module 0x42::returned_global_invalid where
  struct EscapedResource has Key where
    value : u64
  fun return_global_reference(address : Address) -> &mut u64 := do
    let escaped := &mut EscapedResource[address].value
    return escaped
  spec return_global_reference where
    ensures true
    aborts_if false
  fun return_direct_global(address : Address) -> &mut u64 := &mut EscapedResource[address].value
  fun return_shared_global(address : Address) -> &u64 := &EscapedResource[address].value

leaner module 0x42::returned_parameter_positive where
  fun return_parameter_reference(slot : &mut u64) -> &mut u64 := do
    let escaped := &mut *slot
    return escaped
  spec return_parameter_reference where
    ensures result == old(slot) && slot == result
    aborts_if false
  verify return_parameter_reference
  fun update_returned_parameter(slot : &mut u64) -> Unit := do
    let returned := return_parameter_reference(slot)
    *returned := 3
  spec update_returned_parameter where
    ensures slot == 3
    aborts_if false
  verify update_returned_parameter

open Lean Elab Command LeanerLang LeanerIR in
run_cmd do
  for (name, expected) in #[
      (`«0x42».returned_local_invalid, "LIR-SEMANTIC-BORROW-ESCAPE"),
      (`«0x42».returned_global_invalid, "LIR-SEMANTIC-BORROW-ESCAPE"),
      (`«0x42».returned_input_invalid, "LIR-SEMANTIC-BORROW-CONFLICT"),
      (`«0x42».returned_pair_invalid, "LIR-SEMANTIC-BORROW-CONFLICT"),
      (`«0x42».returned_native_invalid, "LIR-EXEC-UNSUPPORTED")] do
    let some unit := registeredUnit? (← getEnv) name | throwError "missing negative fixture {name}"
    match Validation.prepareExecution #[Move.semantics] unit with
    | .ok _ => throwError "invalid returned-reference fixture accepted: {name}"
    | .error diagnostics =>
      unless diagnostics.any (fun diagnostic => diagnostic.code == expected &&
          diagnostic.primary.isSome) do
        throwError "wrong returned-reference rejection for {name}: {repr diagnostics}"
      if name == `«0x42».returned_global_invalid then
        unless (diagnostics.filter (·.code == expected)).size == 3 do
          throwError "bound, direct, and shared global returns must all be rejected: {repr diagnostics}"

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  assertRunsState `«0x42».returned_parameter_positive #[
    ⟨"update_returned_parameter", #[.borrow 0 (.integer 9)], .returned #[],
      { nextLoan := 1 }, { nextLoan := 3, pending := #[(0, .integer 3)] }⟩]

open Lean Elab Command LeanerLang in
run_cmd do
  let env ← getEnv
  for function in ["return_parameter_reference", "update_returned_parameter"] do
    let proof := ((`«0x42».returned_parameter_positive).str function).str "verified"
    unless env.contains proof do throwError "missing returned-parameter proof: {proof}"
    if (← collectAxioms proof).contains ``sorryAx then
      throwError "returned-parameter proof contains an admission: {proof}"
  for name in [`«0x42».returned_local_invalid, `«0x42».returned_input_invalid,
      `«0x42».returned_pair_invalid, `«0x42».returned_native_invalid,
      `«0x42».returned_global_invalid, `«0x42».returned_parameter_positive] do
    let some unit := registeredUnit? env name | throwError "missing returned-reference fixture"
    let printed ← match Print.render env unit with
      | .ok value => pure value
      | .error reason => throwError "returned-reference printing failed: {repr reason}"
    let formatted ← match Print.formatSource env printed with
      | .ok value => pure value
      | .error reason => throwError "returned-reference reimport failed: {repr reason}"
    unless printed == formatted do throwError "returned-reference source is not a fixed point"
