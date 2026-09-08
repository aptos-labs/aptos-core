-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option leaner.route "native"
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000

#leaner_measure

leaner module 0x42::native_bitwise where
  fun masked(left : u8, right : u8) -> u8 := left & right
  spec masked where
    ensures result == left & right
    aborts_if false
  verify masked

  fun local_mask(value : u8) -> u8 := do
    let masked := value & 15
    return masked
  spec local_mask where
    ensures result == value & 15
    aborts_if false
  verify local_mask

  fun nested(value : u8) -> u8 := (value & 15) & 3
  spec nested where
    ensures result == (value & 15) & 3
    aborts_if false
  verify nested

  fun ordered(value : u8) -> u8 := (value - 1) & (value - 2)
  spec ordered where
    ensures result == (value - 1) & (value - 2)
    aborts_if value < 2
  verify ordered

  struct Mask has Copy, Drop, Store where
    value : u8

  fun construct(value : u8) -> Mask := new Mask { value := value & 15 }
  spec construct where
    ensures result.value == value & 15
    aborts_if false
  verify construct

  fun condition(value : u8) -> u8 := if value & 1 == 0 then 0 else 1
  spec condition where
    ensures result == if value & 1 == 0 then 0 else 1
    aborts_if false
  verify condition

  fun disjunction(left : u8, right : u8) -> u8 := left | right
  spec disjunction where
    ensures true
    aborts_if false
  verify disjunction

  fun exclusive(left : u8, right : u8) -> u8 := left ^ right
  spec exclusive where
    ensures true
    aborts_if false
  verify exclusive

  fun signed_and(left : i8, right : i8) -> i8 := left & right
  spec signed_and where
    ensures true
    aborts_if false
  verify signed_and

  fun signed_or(left : i8, right : i8) -> i8 := left | right
  spec signed_or where
    ensures true
    aborts_if false
  verify signed_or

  fun signed_xor(left : i8, right : i8) -> i8 := left ^ right
  spec signed_xor where
    ensures true
    aborts_if false
  verify signed_xor

  fun preserve(value : u8) -> u8 := value + 0
  spec preserve where
    ensures result == value
    aborts_if false
  verify preserve

  fun mask_then_call(value : u8) -> u8 := do
    let masked := value & 15
    return core.call preserve::<>(masked)
  spec mask_then_call where
    ensures result == value & 15
    aborts_if false
  verify mask_then_call

  fun wrong_value(left : u8, right : u8) -> u8 := left & right
  spec wrong_value where
    ensures result == left
    aborts_if false

  fun wrong_order(value : u8) -> u8 := (value - 1) & (value - 2)
  spec wrong_order where
    ensures result == (value - 1) & (value - 2)
    aborts_if value < 2 with value - 2

#guard_msgs (drop error) in
#leaner_verify 0x42::native_bitwise::wrong_value
#guard_msgs (drop error) in
#leaner_verify 0x42::native_bitwise::wrong_order

#leaner_require_native 0x42::native_bitwise::masked
#leaner_require_native 0x42::native_bitwise::local_mask
#leaner_require_native 0x42::native_bitwise::nested
#leaner_require_native 0x42::native_bitwise::ordered
#leaner_require_native 0x42::native_bitwise::construct
#leaner_require_native 0x42::native_bitwise::condition
#leaner_require_native 0x42::native_bitwise::disjunction
#leaner_require_native 0x42::native_bitwise::exclusive
#leaner_require_native 0x42::native_bitwise::signed_and
#leaner_require_native 0x42::native_bitwise::signed_or
#leaner_require_native 0x42::native_bitwise::signed_xor
#leaner_require_native 0x42::native_bitwise::preserve
#leaner_require_native 0x42::native_bitwise::mask_then_call

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  for function in ["masked", "local_mask", "nested", "ordered", "construct", "condition",
      "disjunction", "exclusive", "signed_and", "signed_or", "signed_xor", "preserve", "mask_then_call"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».native_bitwise::{function} ")
    unless measured.size == 2 do throwError "missing bitwise stages for {function}"
    let total : Nat := measured.foldl (fun cost sample => cost + sample.heartbeats) 0
    logInfo m!"{function}: {total} heartbeats (all generated stages)"
    unless total ≤ 50000000 do throwError "native bitwise {function} exceeds aggregate 50M budget"
  for function in [`wrong_value, `wrong_order] do
    for suffix in [`computation, `nativeSummary, `computationVerified, `computationRepresents] do
      if (← getEnv).contains (`«0x42».native_bitwise ++ function ++ suffix) then
        throwError "rejected bitwise leaked {function}.{suffix}"
  let caller := `«0x42».native_bitwise.mask_then_call.nativeSummary
  let some proof := (← getEnv).find? caller |>.bind (·.value? (allowOpaque := true))
    | throwError "missing bitwise-to-call summary"
  unless proof.getUsedConstants.contains `«0x42».native_bitwise.preserve.nativeSummary do
    throwError "bitwise continuation does not reuse its callee's summary"

set_option maxHeartbeats 1000

open LeanerIR LeanerIR.Proofs «0x42».native_bitwise in
example (state : RuntimeState) :
    (masked.computation ⟨⟨12, by decide⟩, ⟨10, by decide⟩⟩).ok state ⟨8, by decide⟩ state := by
  have executed : masked.computation ⟨⟨12, by decide⟩, ⟨10, by decide⟩⟩ =
      Spec.pure ⟨8, by decide⟩ := by
    simp [masked.computation, NativeArithmetic.bitwise, NativeArithmetic.wrap, NativeArithmetic.wrapValue]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_bitwise in
example (state : RuntimeState) :
    (disjunction.computation ⟨⟨12, by decide⟩, ⟨10, by decide⟩⟩).ok state ⟨14, by decide⟩ state := by
  have executed : disjunction.computation ⟨⟨12, by decide⟩, ⟨10, by decide⟩⟩ =
      Spec.pure ⟨14, by decide⟩ := by
    simp [disjunction.computation, NativeArithmetic.bitwise, NativeArithmetic.wrap, NativeArithmetic.wrapValue]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_bitwise in
example (state : RuntimeState) :
    (exclusive.computation ⟨⟨12, by decide⟩, ⟨10, by decide⟩⟩).ok state ⟨6, by decide⟩ state := by
  have executed : exclusive.computation ⟨⟨12, by decide⟩, ⟨10, by decide⟩⟩ =
      Spec.pure ⟨6, by decide⟩ := by
    simp [exclusive.computation, NativeArithmetic.bitwise, NativeArithmetic.wrap, NativeArithmetic.wrapValue]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_bitwise in
example (state : RuntimeState) :
    (signed_and.computation ⟨⟨-128, by decide⟩, ⟨-1, by decide⟩⟩).ok state ⟨-128, by decide⟩ state := by
  have executed : signed_and.computation ⟨⟨-128, by decide⟩, ⟨-1, by decide⟩⟩ =
      Spec.pure ⟨-128, by decide⟩ := by
    simp [signed_and.computation, NativeArithmetic.bitwise, NativeArithmetic.wrap, NativeArithmetic.wrapValue]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_bitwise in
example (state : RuntimeState) :
    (signed_or.computation ⟨⟨-128, by decide⟩, ⟨127, by decide⟩⟩).ok state ⟨-1, by decide⟩ state := by
  have executed : signed_or.computation ⟨⟨-128, by decide⟩, ⟨127, by decide⟩⟩ =
      Spec.pure ⟨-1, by decide⟩ := by
    simp [signed_or.computation, NativeArithmetic.bitwise, NativeArithmetic.wrap, NativeArithmetic.wrapValue]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_bitwise in
example (state : RuntimeState) :
    (signed_xor.computation ⟨⟨-1, by decide⟩, ⟨127, by decide⟩⟩).ok state ⟨-128, by decide⟩ state := by
  have executed : signed_xor.computation ⟨⟨-1, by decide⟩, ⟨127, by decide⟩⟩ =
      Spec.pure ⟨-128, by decide⟩ := by
    simp [signed_xor.computation, NativeArithmetic.bitwise, NativeArithmetic.wrap, NativeArithmetic.wrapValue]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_bitwise in
example (state : RuntimeState) (error : Failure) :
    (ordered.computation ⟨⟨0, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer (-1)]) := by
  have executed : ordered.computation ⟨⟨0, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer (-1)]) := by
    simp [ordered.computation, NativeArithmetic.checkedInteger, NativeArithmetic.runtimeFailure,
      IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl

#leaner_require_native_all
