-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option leaner.route "native"

#leaner_measure

leaner module 0x42::native_local_calls where
  fun increment(value : u8) -> u8 := value + 1
  spec increment where
    ensures result == value + 1
    aborts_if value == 255
  verify increment

  fun call_then_add(value : u8) -> u8 := do
    let next := core.call increment::<>(value)
    return next + 1
  spec call_then_add where
    ensures result == value + 2
    aborts_if value >= 254
  verify call_then_add

  fun call_then_subtract(value : u8) -> u8 := do
    let next := core.call increment::<>(value)
    return next - 2
  spec call_then_subtract where
    ensures result == value - 1
    aborts_if value == 0 || value == 255
  verify call_then_subtract

  fun add_then_call(value : u8) -> u8 := do
    let next := value + 1
    let result := core.call increment::<>(next)
    return result
  spec add_then_call where
    ensures result == value + 2
    aborts_if value >= 254
  verify add_then_call

  fun two_calls(value : u8) -> u8 := do
    let next := core.call increment::<>(value)
    let result := core.call increment::<>(next)
    return result
  spec two_calls where
    ensures result == value + 2
    aborts_if value >= 254
  verify two_calls

  fun four_calls(value : u8) -> u8 := do
    let a := core.call increment::<>(value)
    let b := core.call increment::<>(a)
    let c := core.call increment::<>(b)
    let d := core.call increment::<>(c)
    return d
  spec four_calls where
    ensures result == value + 4
    aborts_if value >= 252
  verify four_calls

  fun sequence_callee(value : u8) -> u8 := do
    let next := core.call two_calls::<>(value)
    return next + 1
  spec sequence_callee where
    ensures result == value + 3
    aborts_if value >= 253
  verify sequence_callee

  fun add(left : u8, right : u8) -> u8 := left + right
  spec add where
    ensures result == left + right
    aborts_if left + right > 255
  verify add

  fun reordered(left : u8, right : u8) -> u8 := do
    let sum := core.call add::<>(right, left)
    return sum - right
  spec reordered where
    ensures result == left
    aborts_if left + right > 255
  verify reordered

  fun restricted(value : u8) -> u8 := value + 1
  spec restricted where
    requires value < 254
    ensures result == value + 1
    aborts_if false
  verify restricted

  fun precondition_after_call(value : u8) -> u8 := do
    let next := core.call increment::<>(value)
    let result := core.call restricted::<>(next)
    return result
  spec precondition_after_call where
    requires value < 200
    ensures result == value + 2
    aborts_if false
  verify precondition_after_call

  fun unreported(value : u8) -> u8 := value + 1
  spec unreported where
    ensures result == value + 1
  verify unreported

  fun weak(value : u8) -> u8 := do
    let next := core.call unreported::<>(value)
    return next + 1
  spec weak where
    ensures result == value + 2
  verify weak

  fun approximate(value : u8) -> u8 := value + 1
  spec approximate where
    ensures result > value
    aborts_if value == 255
  verify approximate

  fun approximate_caller(value : u8) -> u8 := do
    let next := core.call approximate::<>(value)
    return next + 1
  spec approximate_caller where
    ensures result >= value + 2
  verify approximate_caller

  fun missing_first(value : u8) -> u8 := do
    let next := core.call increment::<>(value)
    return next + 1
  spec missing_first where
    ensures result == value + 2
    aborts_if value == 254

  fun missing_second(value : u8) -> u8 := do
    let next := core.call increment::<>(value)
    return next + 1
  spec missing_second where
    ensures result == value + 2
    aborts_if value == 255

  fun missing_precondition(value : u8) -> u8 := do
    let next := core.call increment::<>(value)
    let result := core.call restricted::<>(next)
    return result
  spec missing_precondition where
    ensures result == value + 2
    aborts_if value >= 254

  fun spurious(value : u8) -> u8 := do
    let next := core.call increment::<>(value)
    return next + 1
  spec spurious where
    ensures result == value + 2
    aborts_if value >= 253

  -- Both are true of the implementation, but not of the published summary.
  fun hidden_abort(value : u8) -> u8 := do
    let next := core.call unreported::<>(value)
    return next + 1
  spec hidden_abort where
    ensures result == value + 2
    aborts_if value >= 254

  fun hidden_result(value : u8) -> u8 := do
    let next := core.call approximate::<>(value)
    return next + 1
  spec hidden_result where
    ensures result == value + 2

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  let baseline := LeanerLang.Perf.parseBaseline
    (← IO.FS.readFile "LeanerLang/Tests/Performance.exp")
  let total (function : String) : CommandElabM (Nat × Nat) := do
    let found := samples.filter (·.target.startsWith s!"«0x42».native_local_calls::{function} ")
    unless found.size == 2 do throwError "missing call-binding measurements for {function}"
    return found.foldl (fun (heartbeats, objects) sample =>
      (heartbeats + sample.heartbeats, objects + sample.objects)) (0, 0)
  let ceiling (target : String) : CommandElabM (Nat × Nat) := do
    let found := baseline.filter (fun (name, _, _) => name.startsWith target)
    unless found.size == 2 do throwError "missing original cost ceiling {target}"
    return found.foldl (fun (heartbeats, objects) (_, h, o) => (heartbeats + h, objects + o)) (0, 0)
  let arithmetic ← ceiling "«0x42».perf_calls::guarded "
  let call ← ceiling "«0x42».perf_generics::carry_u64 "
  for function in ["increment", "add", "restricted", "unreported", "approximate",
      "call_then_add", "call_then_subtract", "add_then_call", "two_calls", "four_calls",
      "sequence_callee", "reordered", "precondition_after_call", "weak", "approximate_caller"] do
    let actual ← total function
    let limit := if ["increment", "add", "restricted", "unreported", "approximate",
        "call_then_subtract", "reordered"].contains function then arithmetic else call
    let multiplier := if function == "four_calls" then 4
      else if ["two_calls", "precondition_after_call"].contains function then 2 else 1
    logInfo m!"{function}: {actual.1} heartbeats / {actual.2} objects (all generated stages)"
    unless actual.1 ≤ multiplier * limit.1 && actual.2 ≤ multiplier * limit.2 do
      throwError "call binding {function} exceeds unchanged original cost budgets"
  let two ← total "two_calls"
  let four ← total "four_calls"
  unless four.1 ≤ 2 * two.1 && four.2 ≤ 2 * two.2 do
    throwError "repeated-call scaling regressed"

#guard_msgs (drop error) in
#leaner_verify 0x42::native_local_calls::missing_first
#guard_msgs (drop error) in
#leaner_verify 0x42::native_local_calls::missing_second
#guard_msgs (drop error) in
#leaner_verify 0x42::native_local_calls::missing_precondition
#guard_msgs (drop error) in
#leaner_verify 0x42::native_local_calls::spurious
#guard_msgs (drop error) in
#leaner_verify 0x42::native_local_calls::hidden_abort
#guard_msgs (drop error) in
#leaner_verify 0x42::native_local_calls::hidden_result

open Lean Elab Command in
run_cmd do
  let artifactRoot := `«0x42».native_local_calls
  for function in ["missing_first", "missing_second", "missing_precondition", "spurious",
      "hidden_abort", "hidden_result"] do
    for suffix in [`computation, `nativeSummary, `computationVerified,
        `computationState, `computationRepresents, `typedVerified] do
      let name := Name.str artifactRoot function ++ suffix
      if (← getEnv).contains name then throwError "rejected call binding leaked {name}"
    let input ← LeanerLang.Contract.prepareVerification Syntax.missing
      #["0x42", "native_local_calls"] function
    if (LeanerLang.NativeRegistry.entries.getState (← getEnv)).contains input.generated.relation then
      throwError "rejected call binding leaked its registry entry"
  for (function, callee) in [("call_then_add", "increment"), ("call_then_subtract", "increment"),
      ("add_then_call", "increment"),
      ("two_calls", "increment"), ("four_calls", "increment"), ("sequence_callee", "two_calls"),
      ("reordered", "add"), ("precondition_after_call", "restricted"),
      ("weak", "unreported"), ("approximate_caller", "approximate")] do
    let summary := Name.str artifactRoot function ++ `nativeSummary
    let some proof := (← getEnv).find? summary |>.bind (·.value? (allowOpaque := true))
      | throwError "missing call-binding summary {summary}"
    unless proof.getUsedConstants.contains (Name.str artifactRoot callee ++ `nativeSummary) do
      throwError "{summary} does not reuse the callee's contract"
    for suffix in [`computation, `nativeSummary, `computationVerified] do
      let root := Name.str artifactRoot function ++ suffix
      let nativeValues := suffix == `computation
      let mut pending := #[root]
      let mut visited : Array Name := #[]
      while let some name := pending.back? do
        pending := pending.pop
        if visited.contains name then continue
        visited := visited.push name
        let some declaration := (← getEnv).find? name | throwError "missing native artifact {name}"
        let constants := declaration.type.getUsedConstants ++
          ((declaration.value? (allowOpaque := true)).map (·.getUsedConstants)).getD #[]
        for dependency in constants do
          if dependency == ``LeanerIR.RuntimeFrame || dependency == ``LeanerIR.Proofs.typedFunction ||
              dependency == ``LeanerIR.Proofs.decodeSpec ||
              (`LeanerIR.Proofs.Denotation).isPrefixOf dependency ||
              (`LeanerIR.Proofs.ComputationAgreement).isPrefixOf dependency ||
              dependency.getString! == "computationRepresents" ||
              dependency.getString! == "computationState" || dependency == ``sorryAx ||
              (nativeValues && dependency == ``LeanerIR.RuntimeValue) then
            throwError "call binding {root} has forbidden dependency {dependency}"
          if artifactRoot.isPrefixOf dependency then pending := pending.push dependency

-- Exact outcomes, not merely permitted outcomes: an initializer failure and
-- a continuation failure have distinct payloads, and allow no extra error.
open LeanerIR LeanerIR.Proofs in
example (state : RuntimeState) (error : Failure) :
    («0x42».native_local_calls.call_then_subtract.computation ⟨⟨255, by decide⟩⟩).aborts
      state error ↔ error = (.abort, #[.integer 256]) := by
  simp [«0x42».native_local_calls.call_then_subtract.computation,
    «0x42».native_local_calls.increment.computation, Spec.bind, Spec.pure, Spec.abort,
    NativeArithmetic.checkedInteger, NativeArithmetic.runtimeFailure,
    IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]

open LeanerIR LeanerIR.Proofs in
example (state : RuntimeState) (error : Failure) :
    («0x42».native_local_calls.call_then_subtract.computation ⟨⟨0, by decide⟩⟩).aborts
      state error ↔ error = (.abort, #[.integer (-1)]) := by
  simp [«0x42».native_local_calls.call_then_subtract.computation,
    «0x42».native_local_calls.increment.computation, Spec.bind, Spec.pure, Spec.abort,
    NativeArithmetic.checkedInteger, NativeArithmetic.runtimeFailure,
    IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
