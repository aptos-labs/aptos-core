-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000

#leaner_measure

leaner module 0x42::native_conditional_assignments where
  struct Pair has Copy, Drop, Store where
    left : u64
    right : u64

  fun clamp(value : u64) -> u64 := do
    let mut result := value
    if value < 10 then result := value else result := 10
    return result
  spec clamp where
    ensures result <= 10
    aborts_if false
  verify clamp

  fun then_else(flag : Bool) -> u64 := do
    let mut value : u64 := 0
    if flag then value := 1 else value := 2
    return value + 1
  spec then_else where
    ensures result == if flag then 2 else 3
    aborts_if false
  verify then_else

  fun choice(flag : Bool, value : u8) -> u8 := do
    let result := if flag then value + 1 else 0
    return result
  spec choice where
    ensures result == if flag then value + 1 else 0
    aborts_if flag && value == 255 with value + 1
  verify choice

  fun updated(flag : Bool, value : u8) -> u8 := do
    let mut result := value
    if flag then result := result + 1 else result := result - 1
    return result
  spec updated where
    ensures result == if flag then value + 1 else value - 1
    aborts_if (flag && value == 255) || (!flag && value == 0)
      with if flag then value + 1 else value - 1
  verify updated

  fun nested(first : Bool, second : Bool) -> u8 := do
    let mut result : u8 := 0
    if first then
      if second then result := 1 else result := 2
    else
      result := 3
    return result
  spec nested where
    ensures result == if first then (if second then 1 else 2) else 3
    aborts_if false
  verify nested

  fun pair(flag : Bool, value : u64) -> u64 := do
    let mut result := new Pair { left := 0, right := 0 }
    if flag then
      result := new Pair { left := value, right := 1 }
    else
      result := new Pair { left := 7, right := value }
    return result.left
  spec pair where
    ensures result == if flag then value else 7
    aborts_if false
  verify pair

  fun vector_value(flag : Bool) -> u64 := do
    let mut values := vector<u64>[]
    if flag then values := vector<u64>[1, 2] else values := vector<u64>[7, 9]
    let view := &values[1]
    return *view
  spec vector_value where
    ensures result == if flag then 2 else 9
    aborts_if false
  verify vector_value

  fun guarded_call(flag : Bool, value : u8) -> u8 := do
    let mut result : u8 := 0
    if flag then result := choice(true, value) else result := 7
    return result
  spec guarded_call where
    ensures result == if flag then value + 1 else 7
    aborts_if flag && value == 255 with value + 1
  verify guarded_call

  fun prepared_call(flag : Bool, value : u8) -> u8 := do
    let next := (value + 1) + 1
    let mut result : u8 := 0
    if flag then result := choice(true, next) else result := 7
    return result
  spec prepared_call where
    requires value < 253
    ensures result == if flag then value + 3 else 7
    aborts_if false
  verify prepared_call

  fun repeated(first : Bool, second : Bool, value : u8) -> u8 := do
    let mut result := value
    if first then result := 1 else result := 2
    if second then result := result + 1 else result := result + 2
    return result + 1
  spec repeated where
    ensures result == (if first then 1 else 2) + (if second then 1 else 2) + 1
    aborts_if false
  verify repeated

  fun tested(value : u8) -> u8 := do
    let mut result : u8 := 0
    if value + 1 < 2 then result := 1 else result := 2
    return result
  spec tested where
    ensures result == if value == 0 then 1 else 2
    aborts_if value == 255 with value + 1
  verify tested

  fun wrong_branch(flag : Bool) -> u8 := do
    let mut result : u8 := 0
    if flag then result := 1 else result := 2
    return result
  spec wrong_branch where
    ensures result == 1
    aborts_if false

  fun wrong_abort(flag : Bool, value : u8) -> u8 := do
    let mut result := value
    if flag then result := result + 1 else result := 0
    return result
  spec wrong_abort where
    ensures result == if flag then value + 1 else 0
    aborts_if value == 255 with value + 1

#guard_msgs (drop error) in
#leaner_verify 0x42::native_conditional_assignments::wrong_branch
#guard_msgs (drop error) in
#leaner_verify 0x42::native_conditional_assignments::wrong_abort

#leaner_require_native_all

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  let mut excessive : Array String := #[]
  for function in ["clamp", "then_else", "choice", "updated", "nested", "pair",
      "vector_value", "guarded_call", "prepared_call", "repeated", "tested"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».native_conditional_assignments::{function} ")
    unless measured.size == 2 do throwError "missing conditional assignment stages for {function}"
    let total : Nat := measured.foldl (fun cost sample => cost + sample.heartbeats) 0
    let objects : Nat := measured.foldl (fun cost sample => cost + sample.objects) 0
    logInfo m!"{function}: {total} heartbeats / {objects} objects (all generated stages)"
    if total > 10000000 || objects > 15000 then excessive := excessive.push function
  unless excessive.isEmpty do
    throwError "native conditional assignments exceed aggregate 10M/15,000 budgets: {excessive}"
  let some proof := (← getEnv).find? `«0x42».native_conditional_assignments.guarded_call.nativeSummary
      |>.bind (·.value? (allowOpaque := true))
    | throwError "missing conditional assignment caller summary"
  unless proof.getUsedConstants.contains `«0x42».native_conditional_assignments.choice.nativeSummary do
    throwError "conditional assignment did not reuse the callee summary"
  let some repeated := (← getEnv).find? `«0x42».native_conditional_assignments.repeated.computation
      |>.bind (·.value? (allowOpaque := true))
    | throwError "missing repeated conditional computation"
  let rec countConstants (target : Name) : Expr → Nat
    | .const name _ => if name == target then 1 else 0
    | .app function argument => countConstants target function + countConstants target argument
    | .lam _ type body _ | .forallE _ type body _ => countConstants target type + countConstants target body
    | .letE _ type value body _ =>
        countConstants target type + countConstants target value + countConstants target body
    | .mdata _ body | .proj _ _ body => countConstants target body
    | _ => 0
  unless countConstants ``LeanerIR.Proofs.NativeArithmetic.checkedInteger repeated == 3 do
    throwError "conditional assignment duplicated the arithmetic continuation"
  let some nested := (← getEnv).find? `«0x42».native_conditional_assignments.nested.computation
      |>.bind (·.value? (allowOpaque := true))
    | throwError "missing pure conditional computation"
  -- One binding joins the selected value to its continuation; no bindings
  -- remain inside either of the two nested pure choices.
  unless countConstants ``LeanerIR.Proofs.Spec.bind nested == 1 do
    throwError "a pure conditional duplicated its join"
  for rejected in [`wrong_branch, `wrong_abort] do
    for suffix in [`computation, `nativeSummary, `computationVerified,
        `computationState, `computationRepresents, `typedVerified, `verified] do
      if (← getEnv).contains (`«0x42».native_conditional_assignments ++ rejected ++ suffix) then
        throwError "rejected conditional assignment leaked {rejected}.{suffix}"

set_option maxHeartbeats 1000

open LeanerIR LeanerIR.Proofs «0x42».native_conditional_assignments in
example (state : RuntimeState) :
    (then_else.computation ⟨false⟩).ok state ⟨3, by decide⟩ state := by
  have executed : then_else.computation ⟨false⟩ = Spec.pure ⟨3, by decide⟩ := by
    simp [then_else.computation, NativeArithmetic.checkedInteger,
      IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_conditional_assignments in
example (state : RuntimeState) :
    (choice.computation ⟨false, ⟨255, by decide⟩⟩).ok state ⟨0, by decide⟩ state := by
  have executed : choice.computation ⟨false, ⟨255, by decide⟩⟩ = Spec.pure ⟨0, by decide⟩ := by
    simp [choice.computation]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_conditional_assignments in
example (state : RuntimeState) (error : Failure) :
    (updated.computation ⟨true, ⟨255, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 256]) := by
  have executed : updated.computation ⟨true, ⟨255, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 256]) := by
    simp [updated.computation, NativeArithmetic.checkedInteger, NativeArithmetic.runtimeFailure,
      IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_conditional_assignments in
example (state : RuntimeState) (error : Failure) :
    (updated.computation ⟨false, ⟨0, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer (-1)]) := by
  have executed : updated.computation ⟨false, ⟨0, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer (-1)]) := by
    simp [updated.computation, NativeArithmetic.checkedInteger, NativeArithmetic.runtimeFailure,
      IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_conditional_assignments in
example (state : RuntimeState) (error : Failure) :
    (tested.computation ⟨⟨255, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 256]) := by
  have executed : tested.computation ⟨⟨255, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 256]) := by
    simp [tested.computation, NativeArithmetic.checkedInteger, NativeArithmetic.runtimeFailure,
      IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl
