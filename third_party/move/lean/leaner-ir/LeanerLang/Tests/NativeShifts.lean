-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option leaner.route "native"
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000

#leaner_measure

leaner module 0x42::native_shifts where
  fun shifted(value : u64, amount : u8) -> u64 := value << amount
  spec shifted where
    ensures result == (value << amount) % 18446744073709551616
    aborts_if amount >= 64 with amount
  verify shifted

  fun halved(value : u16) -> u16 := value >> 1u8
  spec halved where
    ensures result == value >> 1u8
    aborts_if false
  verify halved

  fun right(value : u64, amount : u8) -> u64 := value >> amount
  spec right where
    ensures result == value >> amount
    aborts_if amount >= 64 with amount
  verify right

  fun signed_left(value : i8, amount : u8) -> i8 := value << amount
  spec signed_left where
    ensures true
    aborts_if amount >= 8 with amount
  verify signed_left

  fun signed_right(value : i8, amount : u8) -> i8 := value >> amount
  spec signed_right where
    ensures true
    aborts_if amount >= 8 with amount
  verify signed_right

  fun nested_amount(value : u64, amount : u8) -> u64 := value << (amount + 1)
  spec nested_amount where
    ensures result == (value << (amount + 1)) % 18446744073709551616
    aborts_if amount >= 63 with amount + 1
  verify nested_amount

  fun nested_value(value : u8) -> u8 := (value + 1) << 1u8
  spec nested_value where
    ensures result == ((value + 1) << 1u8) % 256
    aborts_if value == 255 with value + 1
  verify nested_value

  fun local_shift(value : u16) -> u16 := do
    let next := value >> 1u8
    return next + 1
  spec local_shift where
    ensures result == (value >> 1u8) + 1
    aborts_if false
  verify local_shift

  fun twice(value : u64) -> u64 := (value >> 1u8) >> 1u8
  spec twice where
    ensures result == (value >> 1u8) >> 1u8
    aborts_if false
  verify twice

  fun preserve(value : u16) -> u16 := value + 0
  spec preserve where
    ensures result == value
    aborts_if false
  verify preserve

  fun shift_then_call(value : u16) -> u16 := do
    let shifted := value >> 1u8
    return preserve(shifted)
  spec shift_then_call where
    ensures result == value >> 1u8
    aborts_if false
  verify shift_then_call

  fun shift_add_call(value : u16) -> u16 := do
    let shifted := value >> 1u8
    let next := shifted + 1
    return preserve(next)
  spec shift_add_call where
    ensures result == (value >> 1u8) + 1
    aborts_if false
  verify shift_add_call

  fun missing_limit(value : u64, amount : u8) -> u64 := value << amount
  spec missing_limit where
    ensures true
    aborts_if false

  fun wrong_wrap(value : u8) -> u8 := value << 1u8
  spec wrong_wrap where
    ensures result == value << 1u8
    aborts_if false

#guard_msgs (drop error) in
#leaner_verify 0x42::native_shifts::missing_limit
#guard_msgs (drop error) in
#leaner_verify 0x42::native_shifts::wrong_wrap

#leaner_require_native 0x42::native_shifts::shifted
#leaner_require_native 0x42::native_shifts::halved
#leaner_require_native 0x42::native_shifts::right
#leaner_require_native 0x42::native_shifts::signed_left
#leaner_require_native 0x42::native_shifts::signed_right
#leaner_require_native 0x42::native_shifts::nested_amount
#leaner_require_native 0x42::native_shifts::nested_value
#leaner_require_native 0x42::native_shifts::local_shift
#leaner_require_native 0x42::native_shifts::twice
#leaner_require_native 0x42::native_shifts::preserve
#leaner_require_native 0x42::native_shifts::shift_then_call
#leaner_require_native 0x42::native_shifts::shift_add_call

#leaner_require_native_all

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  for function in ["shifted", "halved", "right", "signed_left", "signed_right",
      "nested_amount", "nested_value", "local_shift", "twice", "preserve",
      "shift_then_call", "shift_add_call"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».native_shifts::{function} ")
    unless measured.size == 2 do throwError "missing shift stages for {function}"
    let total : Nat := measured.foldl (fun cost sample => cost + sample.heartbeats) 0
    let objects : Nat := measured.foldl (fun count sample => count + sample.objects) 0
    logInfo m!"{function}: {total} heartbeats / {objects} objects (all generated stages)"
    unless total ≤ 10000000 do throwError "native shift {function} exceeds aggregate 10M budget"
    unless objects ≤ 15000 do throwError "native shift {function} exceeds aggregate term-size budget"
  for caller in [`shift_then_call, `shift_add_call] do
    let some proof := (← getEnv).find? (`«0x42».native_shifts ++ caller ++ `nativeSummary)
        |>.bind (·.value? (allowOpaque := true))
      | throwError "missing shift caller summary {caller}"
    unless proof.getUsedConstants.contains `«0x42».native_shifts.preserve.nativeSummary do
      throwError "shift caller {caller} did not reuse its callee summary"
  for function in [`missing_limit, `wrong_wrap] do
    for suffix in [`computation, `nativeSummary, `computationVerified,
        `computationState, `computationRepresents, `typedVerified] do
      if (← getEnv).contains (`«0x42».native_shifts ++ function ++ suffix) then
        throwError "rejected shift leaked {function}.{suffix}"

set_option maxHeartbeats 1000

open LeanerIR LeanerIR.Proofs «0x42».native_shifts in
example (state : RuntimeState) :
    (shifted.computation ⟨⟨2 ^ 63, by decide⟩, ⟨1, by decide⟩⟩).ok state ⟨0, by decide⟩ state := by
  have executed : shifted.computation ⟨⟨2 ^ 63, by decide⟩, ⟨1, by decide⟩⟩ = Spec.pure ⟨0, by decide⟩ := by
    simp [shifted.computation, NativeArithmetic.shiftResult, NativeArithmetic.shiftValue,
      NativeArithmetic.wrap, NativeArithmetic.wrapValue, Spec.ofExcept] <;>
      (apply congrArg Spec.pure; apply SpecInt.ext; decide)
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_shifts in
example (state : RuntimeState) :
    (halved.computation ⟨⟨9, by decide⟩⟩).ok state ⟨4, by decide⟩ state := by
  have executed : halved.computation ⟨⟨9, by decide⟩⟩ = Spec.pure ⟨4, by decide⟩ := by
    simp [halved.computation, NativeArithmetic.shiftResult, NativeArithmetic.shiftValue,
      NativeArithmetic.wrap, NativeArithmetic.wrapValue, Spec.ofExcept] <;>
      (apply congrArg Spec.pure; apply SpecInt.ext; decide)
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_shifts in
example (state : RuntimeState) :
    (signed_right.computation ⟨⟨-128, by decide⟩, ⟨7, by decide⟩⟩).ok state ⟨-1, by decide⟩ state := by
  have executed : signed_right.computation ⟨⟨-128, by decide⟩, ⟨7, by decide⟩⟩ = Spec.pure ⟨-1, by decide⟩ := by
    simp [signed_right.computation, NativeArithmetic.shiftResult, NativeArithmetic.shiftValue,
      NativeArithmetic.wrap, NativeArithmetic.wrapValue, Spec.ofExcept] <;>
      (apply congrArg Spec.pure; apply SpecInt.ext; decide)
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_shifts in
example (state : RuntimeState) :
    (signed_left.computation ⟨⟨127, by decide⟩, ⟨1, by decide⟩⟩).ok state ⟨-2, by decide⟩ state := by
  have executed : signed_left.computation ⟨⟨127, by decide⟩, ⟨1, by decide⟩⟩ = Spec.pure ⟨-2, by decide⟩ := by
    simp [signed_left.computation, NativeArithmetic.shiftResult, NativeArithmetic.shiftValue,
      NativeArithmetic.wrap, NativeArithmetic.wrapValue, Spec.ofExcept] <;>
      (apply congrArg Spec.pure; apply SpecInt.ext; decide)
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_shifts in
example (state : RuntimeState) (error : Failure) :
    (shifted.computation ⟨⟨1, by decide⟩, ⟨64, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 64]) := by
  have executed : shifted.computation ⟨⟨1, by decide⟩, ⟨64, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 64]) := by
    simp [shifted.computation, NativeArithmetic.shiftResult, NativeArithmetic.runtimeFailure, Spec.ofExcept]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_shifts in
example (state : RuntimeState) :
    (local_shift.computation ⟨⟨9, by decide⟩⟩).ok state ⟨5, by decide⟩ state := by
  have shifted : (max (9 : Int) 0 >>> 1) % 65536 = 4 := by decide
  have executed : local_shift.computation ⟨⟨9, by decide⟩⟩ = Spec.pure ⟨5, by decide⟩ := by
    simp [local_shift.computation, NativeArithmetic.shiftResult, NativeArithmetic.shiftValue,
      NativeArithmetic.wrap, NativeArithmetic.wrapValue, Spec.ofExcept,
      NativeArithmetic.checkedInteger, IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?, shifted]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_shifts in
example (state : RuntimeState) :
    (shift_add_call.computation ⟨⟨9, by decide⟩⟩).ok state ⟨5, by decide⟩ state := by
  have shifted : (max (9 : Int) 0 >>> 1) % 65536 = 4 := by decide
  have executed : shift_add_call.computation ⟨⟨9, by decide⟩⟩ = Spec.pure ⟨5, by decide⟩ := by
    simp [shift_add_call.computation, preserve.computation,
      NativeArithmetic.shiftResult, NativeArithmetic.shiftValue,
      NativeArithmetic.wrap, NativeArithmetic.wrapValue, Spec.ofExcept,
      NativeArithmetic.checkedInteger, IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?, shifted]
  rw [executed]
  exact ⟨rfl, rfl⟩
