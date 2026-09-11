-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000

#leaner_measure

leaner module 0x42::native_assignments where
  struct Pair has Copy, Drop, Store where
    left : u64
    right : u64

  fun increment(value : u64) -> u64 := do
    let mut result := value
    result := result + 1
    return result
  spec increment where
    ensures result == value + 1
    aborts_if value == 18446744073709551615 with value + 1
  verify increment

  fun overwrite(value : u64) -> u64 := do
    let mut result := value
    result := 7
    result := 9
    return result
  spec overwrite where
    ensures result == 9
    aborts_if false
  verify overwrite

  fun twice(value : u64) -> u64 := do
    let mut result := value
    result := result + 1
    result := result + 1
    return result
  spec twice where
    ensures result == value + 2
    aborts_if value >= 18446744073709551614 with 18446744073709551616
  verify twice

  fun saved(value : u64) -> u64 := do
    let mut result := value
    let saved_value := result
    result := 9
    return saved_value
  spec saved where
    ensures result == value
    aborts_if false
  verify saved

  fun swap(first : u8, second : u8) -> u8 := do
    let mut left := first
    let mut right := second
    let saved_left := left
    left := right
    right := saved_left
    return left - right
  spec swap where
    ensures result == second - first
    aborts_if second < first with second - first
  verify swap

  fun flag(value : Bool) -> Bool := do
    let mut result := value
    result := !result
    return result
  spec flag where
    ensures result == !value
    aborts_if false
  verify flag

  fun pair(value : u64) -> u64 := do
    let mut result := new Pair { left := value, right := 1 }
    result := new Pair { left := 7, right := value }
    return result.left
  spec pair where
    ensures result == 7
    aborts_if false
  verify pair

  fun vector_value() -> u64 := do
    let mut values := vector<u64>[1]
    values := vector<u64>[7, 9]
    let view := &values[1]
    return *view
  spec vector_value where
    ensures result == 9
    aborts_if false
  verify vector_value

  fun call(value : u64) -> u64 := do
    let mut result := value
    result := increment(result)
    return result
  spec call where
    ensures result == value + 1
    aborts_if value == 18446744073709551615 with value + 1
  verify call

  fun compound() -> u64 := do
    let mut value : u64 := 4
    value := value + 2
    value := value * 3
    return value
  spec compound where
    ensures result == 18
    aborts_if false
  verify compound

  fun product(left : u8, right : u8) -> u8 := do
    let mut result := left
    result := result * right
    return result
  spec product where
    ensures result == left * right
    aborts_if left * right > 255 with left * right
  verify product

  fun nested_product(value : u8) -> u8 := do
    let mut result := value
    result := (result + 1) * 2
    return result
  spec nested_product where
    ensures result == (value + 1) * 2
    aborts_if value >= 127 with if value == 255 then 256 else (value + 1) * 2
  verify nested_product

  fun wrong_result(value : u64) -> u64 := do
    let mut result := value
    result := 7
    return result
  spec wrong_result where
    ensures result == value
    aborts_if false

  fun wrong_abort(value : u64) -> u64 := do
    let mut result := value
    result := result + 1
    return result
  spec wrong_abort where
    ensures result == value + 1
    aborts_if false

#guard_msgs (drop error) in
#leaner_verify 0x42::native_assignments::wrong_result
#guard_msgs (drop error) in
#leaner_verify 0x42::native_assignments::wrong_abort

#leaner_require_native_all

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  for function in ["increment", "overwrite", "twice", "saved", "swap", "flag", "pair",
      "vector_value", "call", "compound", "product", "nested_product"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».native_assignments::{function} ")
    unless measured.size == 2 do throwError "missing assignment stages for {function}"
    let total : Nat := measured.foldl (fun cost sample => cost + sample.heartbeats) 0
    let objects : Nat := measured.foldl (fun cost sample => cost + sample.objects) 0
    logInfo m!"{function}: {total} heartbeats / {objects} objects (all generated stages)"
    unless total ≤ 10000000 do throwError "native assignment {function} exceeds aggregate 10M budget"
    unless objects ≤ 15000 do throwError "native assignment {function} exceeds aggregate term-size budget"
  let some proof := (← getEnv).find? `«0x42».native_assignments.call.nativeSummary
      |>.bind (·.value? (allowOpaque := true))
    | throwError "missing native assignment caller summary"
  unless proof.getUsedConstants.contains `«0x42».native_assignments.increment.nativeSummary do
    throwError "native assignment did not reuse the callee summary"
  for rejected in [`wrong_result, `wrong_abort] do
    for suffix in [`computation, `nativeSummary, `computationVerified,
        `computationState, `computationRepresents, `typedVerified, `verified] do
      if (← getEnv).contains (`«0x42».native_assignments ++ rejected ++ suffix) then
        throwError "rejected assignment contract leaked {rejected}.{suffix}"

set_option maxHeartbeats 1000

open LeanerIR LeanerIR.Proofs «0x42».native_assignments in
example (state : RuntimeState) :
    (overwrite.computation ⟨⟨3, by decide⟩⟩).ok state ⟨9, by decide⟩ state := by
  have executed : overwrite.computation ⟨⟨3, by decide⟩⟩ = Spec.pure ⟨9, by decide⟩ := by
    simp [overwrite.computation]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_assignments in
example (state : RuntimeState) (error : Failure) :
    (product.computation ⟨⟨16, by decide⟩, ⟨16, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 256]) := by
  have executed : product.computation ⟨⟨16, by decide⟩, ⟨16, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 256]) := by
    simp [product.computation, NativeArithmetic.checkedInteger, NativeArithmetic.runtimeFailure,
      IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_assignments in
example (state : RuntimeState) (error : Failure) :
    (increment.computation ⟨⟨18446744073709551615, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 18446744073709551616]) := by
  have executed : increment.computation ⟨⟨18446744073709551615, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 18446744073709551616]) := by
    simp [increment.computation, NativeArithmetic.checkedInteger, NativeArithmetic.runtimeFailure,
      IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_assignments in
example (state : RuntimeState) :
    (saved.computation ⟨⟨3, by decide⟩⟩).ok state ⟨3, by decide⟩ state := by
  have executed : saved.computation ⟨⟨3, by decide⟩⟩ = Spec.pure ⟨3, by decide⟩ := by
    simp [saved.computation]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_assignments in
example (state : RuntimeState) (error : Failure) :
    (twice.computation ⟨⟨18446744073709551614, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 18446744073709551616]) := by
  have executed : twice.computation ⟨⟨18446744073709551614, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 18446744073709551616]) := by
    simp [twice.computation, NativeArithmetic.checkedInteger, NativeArithmetic.runtimeFailure,
      IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_assignments in
example (state : RuntimeState) (value : Bool) :
    (flag.computation ⟨value⟩).ok state (!value) state := by
  have executed : flag.computation ⟨value⟩ = Spec.pure (!value) := by
    simp [flag.computation]
  rw [executed]
  exact ⟨rfl, rfl⟩
