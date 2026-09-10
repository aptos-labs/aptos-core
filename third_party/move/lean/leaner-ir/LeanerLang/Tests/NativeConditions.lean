-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option leaner.route "native"
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000

#leaner_measure

leaner module 0x42::native_conditions where
  fun arithmetic_condition(value : u8) -> u8 := if value + 1 < 2 then 1 else 0
  spec arithmetic_condition where
    ensures result == if value == 0 then 1 else 0
    aborts_if value == 255 with value + 1
  verify arithmetic_condition

  fun comparison(value : u8) -> Bool := value + 1 == 2
  spec comparison where
    ensures result == (value == 1)
    aborts_if value == 255 with value + 1
  verify comparison

  fun local_condition(value : u8) -> u8 := do
    let flag := value + 1 < 2
    return if flag then 1 else 0
  spec local_condition where
    ensures result == if value == 0 then 1 else 0
    aborts_if value == 255 with value + 1
  verify local_condition

  fun guarded(value : u8) -> u8 :=
    if value < 255 then (if value + 1 == 2 then 1 else 0) else 0
  spec guarded where
    ensures result == if value == 1 then 1 else 0
    aborts_if false
  verify guarded

  fun both_operands(value : u8) -> u8 := if value + 1 < value + 2 then 1 else 0
  spec both_operands where
    ensures result == 1
    aborts_if value >= 254 with if value == 255 then value + 1 else value + 2
  verify both_operands

  fun short_circuit_and(value : u8) -> u8 :=
    if value == 0 && value + 1 == 2 then 1 else 0
  spec short_circuit_and where
    ensures result == 0
    aborts_if false
  verify short_circuit_and

  fun short_circuit_or(value : u8) -> Bool := value == 255 || value + 1 == 2
  spec short_circuit_or where
    ensures result == (value == 255 || value == 1)
    aborts_if false
  verify short_circuit_or

  fun short_circuit_local(value : u8) -> u8 := do
    let flag := value == 255 || value + 1 == 2
    return if flag then 1 else 0
  spec short_circuit_local where
    ensures result == if value == 255 || value == 1 then 1 else 0
    aborts_if false
  verify short_circuit_local

  fun short_circuit_aborts(flag : Bool, value : u8) -> Bool := flag && value + 1 == 2
  spec short_circuit_aborts where
    ensures result == (flag && value == 1)
    aborts_if flag && value == 255 with value + 1
  verify short_circuit_aborts

  fun bad(value : u8) -> u8 := if value + 1 < 2 then 1 else 0
  spec bad where
    ensures result == if value == 0 then 1 else 0
    aborts_if false

  fun wrong_order(value : u8) -> u8 := if value + 1 < value + 2 then 1 else 0
  spec wrong_order where
    ensures result == 1
    aborts_if value >= 254 with value + 2

#guard_msgs (drop error) in
#leaner_verify 0x42::native_conditions::bad
#guard_msgs (drop error) in
#leaner_verify 0x42::native_conditions::wrong_order

#leaner_require_native 0x42::native_conditions::arithmetic_condition
#leaner_require_native 0x42::native_conditions::comparison
#leaner_require_native 0x42::native_conditions::local_condition
#leaner_require_native 0x42::native_conditions::guarded
#leaner_require_native 0x42::native_conditions::both_operands
#leaner_require_native 0x42::native_conditions::short_circuit_and
#leaner_require_native 0x42::native_conditions::short_circuit_or
#leaner_require_native 0x42::native_conditions::short_circuit_local
#leaner_require_native 0x42::native_conditions::short_circuit_aborts

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  for function in ["arithmetic_condition", "comparison", "local_condition", "guarded", "both_operands",
      "short_circuit_and", "short_circuit_or", "short_circuit_local", "short_circuit_aborts"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».native_conditions::{function} ")
    unless measured.size == 2 do throwError "missing condition stages for {function}"
    let total : Nat := measured.foldl (fun cost sample => cost + sample.heartbeats) 0
    logInfo m!"{function}: {total} heartbeats (all generated stages)"
    unless total ≤ 50000000 do throwError "native condition {function} exceeds aggregate 50M budget"
  for function in [`bad, `wrong_order] do
    for suffix in [`computation, `nativeSummary, `computationVerified, `computationRepresents] do
      if (← getEnv).contains (`«0x42».native_conditions ++ function ++ suffix) then
        throwError "rejected condition leaked {function}.{suffix}"

set_option maxHeartbeats 1000

open LeanerIR LeanerIR.Proofs «0x42».native_conditions in
example (state : RuntimeState) :
    (arithmetic_condition.computation ⟨⟨0, by decide⟩⟩).ok state ⟨1, by decide⟩ state := by
  have executed : arithmetic_condition.computation ⟨⟨0, by decide⟩⟩ =
      Spec.pure ⟨1, by decide⟩ := by
    simp [arithmetic_condition.computation, NativeArithmetic.checkedInteger,
      IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_conditions in
example (state : RuntimeState) (error : Failure) :
    (arithmetic_condition.computation ⟨⟨255, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 256]) := by
  have executed : arithmetic_condition.computation ⟨⟨255, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 256]) := by
    simp [arithmetic_condition.computation, NativeArithmetic.checkedInteger,
      NativeArithmetic.runtimeFailure, IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_conditions in
example (state : RuntimeState) :
    (short_circuit_or.computation ⟨⟨255, by decide⟩⟩).ok state true state := by
  have executed : short_circuit_or.computation ⟨⟨255, by decide⟩⟩ = Spec.pure true := by
    simp [short_circuit_or.computation]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_conditions in
example (state : RuntimeState) :
    (short_circuit_and.computation ⟨⟨255, by decide⟩⟩).ok state ⟨0, by decide⟩ state := by
  have executed : short_circuit_and.computation ⟨⟨255, by decide⟩⟩ = Spec.pure ⟨0, by decide⟩ := by
    simp [short_circuit_and.computation]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_conditions in
example (state : RuntimeState) :
    (short_circuit_aborts.computation ⟨false, ⟨255, by decide⟩⟩).ok state false state := by
  have executed : short_circuit_aborts.computation ⟨false, ⟨255, by decide⟩⟩ = Spec.pure false := by
    simp [short_circuit_aborts.computation]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_conditions in
example (state : RuntimeState) (error : Failure) :
    (short_circuit_aborts.computation ⟨true, ⟨255, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 256]) := by
  have executed : short_circuit_aborts.computation ⟨true, ⟨255, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 256]) := by
    simp [short_circuit_aborts.computation, NativeArithmetic.checkedInteger,
      NativeArithmetic.runtimeFailure, IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_conditions in
example (state : RuntimeState) (error : Failure) :
    (both_operands.computation ⟨⟨255, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 256]) := by
  have executed : both_operands.computation ⟨⟨255, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 256]) := by
    simp [both_operands.computation, NativeArithmetic.checkedInteger,
      NativeArithmetic.runtimeFailure, IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl
