-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option leaner.route "native"

#leaner_measure

leaner module 0x42::native_operands where
  fun left(value : u8) -> u8 := (value + 1) + 1
  spec left where
    ensures result == value + 2
    aborts_if value >= 254
  verify left

  fun right(value : u8) -> u8 := 1 + (value + 1)
  spec right where
    ensures result == value + 2
    aborts_if value >= 254
  verify right

  fun both(value : u8) -> u8 := (value + 1) + (value + 2)
  spec both where
    ensures result == value + value + 3
    aborts_if value >= 127
  verify both

  fun subtract(value : u8) -> u8 := (value - 1) + (2 - 1)
  spec subtract where
    ensures result == value
    aborts_if value == 0
  verify subtract

  fun bounded(value : u8) -> u8 := (value + 1) + 1
  spec bounded where
    requires value < 254
    ensures result == value + 2
    aborts_if false
  verify bounded

  fun local(value : u8) -> u8 := do
    let next := (value + 1) + 1
    return (next - 1) + 1
  spec local where
    ensures result == value + 2
    aborts_if value >= 254
  verify local

  fun four(value : u8) -> u8 := (((value + 1) + 1) + 1) + 1
  spec four where
    ensures result == value + 4
    aborts_if value >= 252
  verify four

  -- At 0 both operands would fail, with different payloads. Only the
  -- left operand's failure is observable. At 1 the right operand fails.
  fun ordered(value : u8) -> u8 := (value - 1) + (value - 2)
  spec ordered where
    ensures result == value + value - 3
    aborts_if value < 2 || value >= 130
  verify ordered

  fun increment(value : u8) -> u8 := value + 1
  spec increment where
    ensures result == value + 1
    aborts_if value == 255
  verify increment

  fun nested_then_call(value : u8) -> u8 := do
    let next := (value + 1) + 1
    let result := core.call increment::<>(next)
    return result
  spec nested_then_call where
    ensures result == value + 3
    aborts_if value >= 253
  verify nested_then_call

  fun call_then_nested(value : u8) -> u8 := do
    let next := core.call increment::<>(value)
    return (next + 1) + 1
  spec call_then_nested where
    ensures result == value + 3
    aborts_if value >= 253
  verify call_then_nested

  fun missing_left(value : u8) -> u8 := (value - 1) + (value - 2)
  spec missing_left where
    ensures result == value + value - 3
    aborts_if value == 1 || value >= 130

  fun missing_right(value : u8) -> u8 := (value - 1) + (value - 2)
  spec missing_right where
    ensures result == value + value - 3
    aborts_if value == 0 || value >= 130

  fun missing_outer(value : u8) -> u8 := (value - 1) + (value - 2)
  spec missing_outer where
    ensures result == value + value - 3
    aborts_if value < 2

  fun spurious(value : u8) -> u8 := (value + 1) + 1
  spec spurious where
    ensures result == value + 2
    aborts_if value >= 253

  fun wrong(value : u8) -> u8 := (value + 1) + 1
  spec wrong where
    ensures result == value + 1
    aborts_if value >= 254

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  let baseline := LeanerLang.Perf.parseBaseline
    (← IO.FS.readFile "LeanerLang/Tests/Performance.exp")
  let found := baseline.filter (fun (name, _, _) => name.startsWith "«0x42».perf_calls::guarded ")
  unless found.size == 2 do throwError "missing unchanged arithmetic ceiling"
  let limit := found.foldl (fun (h, o) (_, h', o') => (h + h', o + o')) (0, 0)
  let total (function : String) : CommandElabM (Nat × Nat) := do
    let found := samples.filter (·.target.startsWith s!"«0x42».native_operands::{function} ")
    unless found.size == 2 do throwError "missing operand measurements for {function}"
    return found.foldl (fun (h, o) sample => (h + sample.heartbeats, o + sample.objects)) (0, 0)
  for function in ["left", "right", "both", "subtract", "bounded", "local", "four", "ordered",
      "increment", "nested_then_call", "call_then_nested"] do
    let actual ← total function
    let multiplier := if ["local", "four", "nested_then_call", "call_then_nested"].contains function
      then 2 else 1
    logInfo m!"{function}: {actual.1} heartbeats / {actual.2} objects (all generated stages)"
    unless actual.1 ≤ multiplier * limit.1 && actual.2 ≤ multiplier * limit.2 do
      throwError "nested operands {function} exceeds unchanged arithmetic budgets"
  let two ← total "left"
  let four ← total "four"
  unless four.1 ≤ 2 * two.1 && four.2 ≤ 2 * two.2 do
    throwError "nested operand doubling regressed"

#guard_msgs (drop error) in
#leaner_verify 0x42::native_operands::missing_left
#guard_msgs (drop error) in
#leaner_verify 0x42::native_operands::missing_right
#guard_msgs (drop error) in
#leaner_verify 0x42::native_operands::missing_outer
#guard_msgs (drop error) in
#leaner_verify 0x42::native_operands::spurious
#guard_msgs (drop error) in
#leaner_verify 0x42::native_operands::wrong

open Lean Elab Command in
run_cmd do
  let artifactRoot := `«0x42».native_operands
  for function in ["missing_left", "missing_right", "missing_outer", "spurious", "wrong"] do
    for suffix in [`computation, `nativeSummary, `computationVerified,
        `computationState, `computationRepresents, `typedVerified] do
      let name := Name.str artifactRoot function ++ suffix
      if (← getEnv).contains name then throwError "rejected operand tree leaked {name}"
    let input ← LeanerLang.Contract.prepareVerification Syntax.missing
      #["0x42", "native_operands"] function
    if (LeanerLang.NativeRegistry.entries.getState (← getEnv)).contains input.generated.relation then
      throwError "rejected operand tree leaked its registry entry"
  for function in ["left", "right", "both", "subtract", "bounded", "local", "four", "ordered",
      "increment", "nested_then_call", "call_then_nested"] do
    for suffix in [`computation, `nativeSummary, `computationVerified] do
      let root := Name.str artifactRoot function ++ suffix
      let mut pending := #[root]
      let mut visited : Array Name := #[]
      while let some name := pending.back? do
        pending := pending.pop
        if visited.contains name then continue
        visited := visited.push name
        let some declaration := (← getEnv).find? name | throwError "missing operand artifact {name}"
        let constants := declaration.type.getUsedConstants ++
          ((declaration.value? (allowOpaque := true)).map (·.getUsedConstants)).getD #[]
        for dependency in constants do
          if dependency == ``LeanerIR.RuntimeFrame || dependency == ``LeanerIR.Proofs.typedFunction ||
              dependency == ``LeanerIR.Proofs.decodeSpec ||
              (`LeanerIR.Proofs.Denotation).isPrefixOf dependency ||
              (`LeanerIR.Proofs.ComputationAgreement).isPrefixOf dependency ||
              dependency.getString! == "computationRepresents" ||
              dependency.getString! == "computationState" || dependency == ``sorryAx ||
              (suffix == `computation && dependency == ``LeanerIR.RuntimeValue) then
            throwError "native operand artifact {root} has forbidden dependency {dependency}"
          if artifactRoot.isPrefixOf dependency then pending := pending.push dependency

open LeanerIR LeanerIR.Proofs in
example (state : RuntimeState) (error : Failure) :
    («0x42».native_operands.ordered.computation ⟨⟨0, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer (-1)]) := by
  have executed : «0x42».native_operands.ordered.computation ⟨⟨0, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer (-1)]) := by
    simp [«0x42».native_operands.ordered.computation,
      NativeArithmetic.checkedInteger, NativeArithmetic.runtimeFailure,
      IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs in
example (state : RuntimeState) (error : Failure) :
    («0x42».native_operands.ordered.computation ⟨⟨1, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer (-1)]) := by
  have executed : «0x42».native_operands.ordered.computation ⟨⟨1, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer (-1)]) := by
    simp [«0x42».native_operands.ordered.computation,
      NativeArithmetic.checkedInteger, NativeArithmetic.runtimeFailure,
      IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs in
example (state : RuntimeState) (error : Failure) :
    («0x42».native_operands.ordered.computation ⟨⟨130, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 257]) := by
  have executed : «0x42».native_operands.ordered.computation ⟨⟨130, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 257]) := by
    simp [«0x42».native_operands.ordered.computation,
      NativeArithmetic.checkedInteger, NativeArithmetic.runtimeFailure,
      IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl
