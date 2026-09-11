-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option leaner.route "native"
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000

#leaner_measure

leaner module 0x42::native_division where
  fun quotient(left : u8, right : u8) -> u8 := left / right
  spec quotient where
    ensures result == left / right
    aborts_if right == 0
  verify quotient

  fun remainder(left : u8, right : u8) -> u8 := left % right
  spec remainder where
    ensures result == left % right
    aborts_if right == 0
  verify remainder

  fun signed_quotient(left : i8, right : i8) -> i8 := left / right
  spec signed_quotient where
    ensures result == left / right
    aborts_if right == 0 || (left == -128 && right == -1)
  verify signed_quotient

  fun signed_remainder(left : i8, right : i8) -> i8 := left % right
  spec signed_remainder where
    ensures result == left % right
    aborts_if right == 0 || (left == -128 && right == -1)
  verify signed_remainder

  fun nested_dividend(value : u8) -> u8 := (value + 1) / 2
  spec nested_dividend where
    ensures result == (value + 1) / 2
    aborts_if value == 255 with value + 1
  verify nested_dividend

  fun nested_divisor(value : u8, divisor : u8) -> u8 := value / (divisor + 1)
  spec nested_divisor where
    ensures result == value / (divisor + 1)
    aborts_if divisor == 255 with divisor + 1
  verify nested_divisor

  fun local_result(value : u8) -> u8 := do
    let quotient := value / 2
    return quotient + 1
  spec local_result where
    ensures result == value / 2 + 1
    aborts_if false
  verify local_result

  fun ordered(value : u8) -> u8 := (value - 1) / (value - 2)
  spec ordered where
    ensures result == (value - 1) / (value - 2)
    aborts_if value <= 2
  verify ordered

  struct Quotient has Copy, Drop, Store where
    value : u8

  fun construct(value : u8, divisor : u8) -> Quotient := new Quotient { value := value / divisor }
  spec construct where
    ensures result.value == value / divisor
    aborts_if divisor == 0
  verify construct

  fun condition(value : u8) -> u8 := if value / 2 == 1 then 1 else 0
  spec condition where
    ensures result == if value == 2 || value == 3 then 1 else 0
    aborts_if false
  verify condition

  fun increment(value : u8) -> u8 := value + 1
  spec increment where
    requires value < 255
    ensures result == value + 1
    aborts_if false
  verify increment

  fun half_then_call(value : u8) -> u8 := do
    let quotient := value / 2
    let next := core.call increment::<>(quotient)
    return next
  spec half_then_call where
    ensures result == value / 2 + 1
    aborts_if false
  verify half_then_call

  fun twice(value : u8) -> u8 := do
    let first := value % 3
    let second := first % 2
    return second
  spec twice where
    ensures result == (value % 3) % 2
    aborts_if false
  verify twice

  fun missing_zero(left : u8, right : u8) -> u8 := left / right
  spec missing_zero where
    ensures result == left / right
    aborts_if false

  fun missing_quotient_overflow(left : i8, right : i8) -> i8 := left % right
  spec missing_quotient_overflow where
    ensures result == left % right
    aborts_if right == 0

#guard_msgs (drop error) in
#leaner_verify 0x42::native_division::missing_zero
#guard_msgs (drop error) in
#leaner_verify 0x42::native_division::missing_quotient_overflow

#leaner_require_native 0x42::native_division::quotient
#leaner_require_native 0x42::native_division::remainder
#leaner_require_native 0x42::native_division::signed_quotient
#leaner_require_native 0x42::native_division::signed_remainder
#leaner_require_native 0x42::native_division::nested_dividend
#leaner_require_native 0x42::native_division::nested_divisor
#leaner_require_native 0x42::native_division::local_result
#leaner_require_native 0x42::native_division::ordered
#leaner_require_native 0x42::native_division::construct
#leaner_require_native 0x42::native_division::condition
#leaner_require_native 0x42::native_division::increment
#leaner_require_native 0x42::native_division::half_then_call
#leaner_require_native 0x42::native_division::twice

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  for function in ["quotient", "remainder", "signed_quotient", "signed_remainder",
      "nested_dividend", "nested_divisor", "local_result", "ordered", "construct", "condition",
      "increment", "half_then_call", "twice"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».native_division::{function} ")
    unless measured.size == 2 do throwError "missing division stages for {function}"
    let total : Nat := measured.foldl (fun cost sample => cost + sample.heartbeats) 0
    logInfo m!"{function}: {total} heartbeats (all generated stages)"
    unless total ≤ 50000000 do throwError "native division {function} exceeds aggregate 50M budget"
  for function in [`missing_zero, `missing_quotient_overflow] do
    for suffix in [`computation, `nativeSummary, `computationVerified, `computationRepresents] do
      if (← getEnv).contains (`«0x42».native_division ++ function ++ suffix) then
        throwError "rejected division leaked {function}.{suffix}"
  let summary := `«0x42».native_division.half_then_call.nativeSummary
  let some proof := (← getEnv).find? summary |>.bind (·.value? (allowOpaque := true))
    | throwError "missing division-to-call summary"
  unless proof.getUsedConstants.contains `«0x42».native_division.increment.nativeSummary do
    throwError "division continuation does not reuse its callee's contract"

set_option maxHeartbeats 1000

open LeanerIR LeanerIR.Proofs «0x42».native_division in
example (state : RuntimeState) :
    (quotient.computation ⟨⟨7, by decide⟩, ⟨3, by decide⟩⟩).ok state ⟨2, by decide⟩ state := by
  have executed : quotient.computation ⟨⟨7, by decide⟩, ⟨3, by decide⟩⟩ =
      Spec.pure ⟨2, by decide⟩ := by
    simp [quotient.computation, NativeArithmetic.divisionResult, NativeArithmetic.checkedResult,
      Spec.ofExcept, IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_division in
example (state : RuntimeState) :
    (signed_quotient.computation ⟨⟨-7, by decide⟩, ⟨3, by decide⟩⟩).ok state ⟨-2, by decide⟩ state := by
  have executed : signed_quotient.computation ⟨⟨-7, by decide⟩, ⟨3, by decide⟩⟩ =
      Spec.pure ⟨-2, by decide⟩ := by
    simp [signed_quotient.computation, NativeArithmetic.divisionResult, NativeArithmetic.checkedResult,
      Spec.ofExcept, IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_division in
example (state : RuntimeState) :
    (signed_remainder.computation ⟨⟨-7, by decide⟩, ⟨3, by decide⟩⟩).ok state ⟨-1, by decide⟩ state := by
  have executed : signed_remainder.computation ⟨⟨-7, by decide⟩, ⟨3, by decide⟩⟩ =
      Spec.pure ⟨-1, by decide⟩ := by
    simp [signed_remainder.computation, NativeArithmetic.remainderResult, NativeArithmetic.checkedResult,
      Spec.ofExcept, IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_division in
example (state : RuntimeState) (error : Failure) :
    (signed_remainder.computation ⟨⟨-128, by decide⟩, ⟨-1, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 128]) := by
  have executed : signed_remainder.computation ⟨⟨-128, by decide⟩, ⟨-1, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 128]) := by
    simp [signed_remainder.computation, NativeArithmetic.remainderResult, NativeArithmetic.checkedResult,
      NativeArithmetic.runtimeFailure, Spec.ofExcept, IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_division in
example (state : RuntimeState) (error : Failure) :
    (quotient.computation ⟨⟨7, by decide⟩, ⟨0, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[]) := by
  have executed : quotient.computation ⟨⟨7, by decide⟩, ⟨0, by decide⟩⟩ =
      Spec.abort (.abort, #[]) := by
    simp [quotient.computation, NativeArithmetic.divisionResult, NativeArithmetic.emptyFailure,
      Spec.ofExcept]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_division in
example (state : RuntimeState) (error : Failure) :
    (remainder.computation ⟨⟨7, by decide⟩, ⟨0, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[]) := by
  have executed : remainder.computation ⟨⟨7, by decide⟩, ⟨0, by decide⟩⟩ =
      Spec.abort (.abort, #[]) := by
    simp [remainder.computation, NativeArithmetic.remainderResult, NativeArithmetic.emptyFailure,
      Spec.ofExcept]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_division in
example (state : RuntimeState) (error : Failure) :
    (ordered.computation ⟨⟨0, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer (-1)]) := by
  have executed : ordered.computation ⟨⟨0, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer (-1)]) := by
    simp [ordered.computation, NativeArithmetic.checkedInteger, NativeArithmetic.runtimeFailure,
      IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl
