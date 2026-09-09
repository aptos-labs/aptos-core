-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option leaner.route "native"
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000

#leaner_measure

leaner module 0x42::native_boolean_operations where
  fun logical_and(left : Bool, right : Bool) -> Bool := core.prim.logicalAnd(left, right)
  spec logical_and where
    ensures result == (left && right)
    aborts_if false
  verify logical_and

  fun logical_or(left : Bool, right : Bool) -> Bool := core.prim.logicalOr(left, right)
  spec logical_or where
    ensures result == (left || right)
    aborts_if false
  verify logical_or

  fun logical_not(value : Bool) -> Bool := !value
  spec logical_not where
    ensures result == !value
    aborts_if false
  verify logical_not

  fun eager_and(value : u8) -> Bool := core.prim.logicalAnd(false, value + 1 == 0)
  spec eager_and where
    ensures result == false
    aborts_if value == 255 with value + 1
  verify eager_and

  fun eager_or(value : u8) -> Bool := core.prim.logicalOr(true, value + 1 == 0)
  spec eager_or where
    ensures result == true
    aborts_if value == 255 with value + 1
  verify eager_or

  fun nested_not(value : u8) -> Bool := !(value + 1 == 2)
  spec nested_not where
    ensures result == (value != 1)
    aborts_if value == 255 with value + 1
  verify nested_not

  fun condition(flag : Bool, value : u8) -> u8 :=
    if core.prim.logicalAnd(flag, value + 1 == 2) then 1 else 0
  spec condition where
    ensures result == if flag && value == 1 then 1 else 0
    aborts_if value == 255 with value + 1
  verify condition

  fun condition_or(flag : Bool, value : u8) -> u8 :=
    if core.prim.logicalOr(flag, value + 1 == 2) then 1 else 0
  spec condition_or where
    ensures result == if flag || value == 1 then 1 else 0
    aborts_if value == 255 with value + 1
  verify condition_or

  fun eager_order(value : u8) -> Bool :=
    core.prim.logicalAnd(value + 1 == 0, value + 2 == 0)
  spec eager_order where
    ensures result == false
    aborts_if value >= 254 with if value == 255 then value + 1 else value + 2
  verify eager_order

  fun bad_skip(value : u8) -> Bool := core.prim.logicalAnd(false, value + 1 == 0)
  spec bad_skip where
    ensures result == false
    aborts_if false

  fun wrong_order(value : u8) -> Bool :=
    core.prim.logicalAnd(value + 1 == 0, value + 2 == 0)
  spec wrong_order where
    ensures result == false
    aborts_if value >= 254 with value + 2

#guard_msgs (drop error) in
#leaner_verify 0x42::native_boolean_operations::bad_skip
#guard_msgs (drop error) in
#leaner_verify 0x42::native_boolean_operations::wrong_order

#leaner_require_native 0x42::native_boolean_operations::logical_and
#leaner_require_native 0x42::native_boolean_operations::logical_or
#leaner_require_native 0x42::native_boolean_operations::logical_not
#leaner_require_native 0x42::native_boolean_operations::eager_and
#leaner_require_native 0x42::native_boolean_operations::eager_or
#leaner_require_native 0x42::native_boolean_operations::nested_not
#leaner_require_native 0x42::native_boolean_operations::condition
#leaner_require_native 0x42::native_boolean_operations::condition_or
#leaner_require_native 0x42::native_boolean_operations::eager_order

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  for function in ["logical_and", "logical_or", "logical_not", "eager_and", "eager_or",
      "nested_not", "condition", "condition_or", "eager_order"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».native_boolean_operations::{function} ")
    unless measured.size == 2 do throwError "missing Boolean operation stages for {function}"
    let total : Nat := measured.foldl (fun cost sample => cost + sample.heartbeats) 0
    logInfo m!"{function}: {total} heartbeats (all generated stages)"
    unless total ≤ 50000000 do throwError "native Boolean operation {function} exceeds aggregate 50M budget"
  for function in [`bad_skip, `wrong_order] do
    for suffix in [`computation, `nativeSummary, `computationVerified, `computationRepresents] do
      if (← getEnv).contains (`«0x42».native_boolean_operations ++ function ++ suffix) then
        throwError "rejected eager Boolean operation leaked {function}.{suffix}"

set_option maxHeartbeats 1000

open LeanerIR LeanerIR.Proofs «0x42».native_boolean_operations in
example (state : RuntimeState) (value : Bool) :
    (logical_not.computation ⟨value⟩).ok state (!value) state := by
  have executed : logical_not.computation ⟨value⟩ = Spec.pure (!value) := by
    simp [logical_not.computation]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_boolean_operations in
example (state : RuntimeState) (error : Failure) :
    (eager_and.computation ⟨⟨255, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 256]) := by
  have executed : eager_and.computation ⟨⟨255, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 256]) := by
    simp [eager_and.computation, NativeArithmetic.checkedInteger,
      NativeArithmetic.runtimeFailure, IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_boolean_operations in
example (state : RuntimeState) (error : Failure) :
    (eager_or.computation ⟨⟨255, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 256]) := by
  have executed : eager_or.computation ⟨⟨255, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 256]) := by
    simp [eager_or.computation, NativeArithmetic.checkedInteger,
      NativeArithmetic.runtimeFailure, IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_boolean_operations in
example (state : RuntimeState) (error : Failure) :
    (condition.computation ⟨false, ⟨255, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 256]) := by
  have executed : condition.computation ⟨false, ⟨255, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 256]) := by
    simp [condition.computation, NativeArithmetic.checkedInteger,
      NativeArithmetic.runtimeFailure, IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_boolean_operations in
example (state : RuntimeState) (error : Failure) :
    (eager_order.computation ⟨⟨255, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 256]) := by
  have executed : eager_order.computation ⟨⟨255, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 256]) := by
    simp [eager_order.computation, NativeArithmetic.checkedInteger,
      NativeArithmetic.runtimeFailure, IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl
