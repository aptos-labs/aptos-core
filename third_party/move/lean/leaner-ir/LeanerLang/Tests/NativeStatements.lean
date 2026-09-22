-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000

#leaner_measure

leaner module 0x42::native_statements where
  fun empty() -> Unit := ()
  spec empty where
    aborts_if false
  verify empty

  fun abort_code(code : u64) -> u64 := abort(code)
  spec abort_code where
    ensures result == 0
    aborts_if true with code
  verify abort_code

  fun assertion(flag : Bool) -> u64 := do
    assert(flag, 17)
    return 1
  spec assertion where
    ensures result == 1
    aborts_if !flag with 17
  verify assertion

  fun floor(value : u64) -> u64 := do
    if value < 1 then abort(11)
    return value - 1
  spec floor where
    ensures result == value - 1
    aborts_if value < 1 with 11
  verify floor

  fun assert_unit(flag : Bool) -> Unit := assert(flag, 19)
  spec assert_unit where
    aborts_if !flag with 19
  verify assert_unit

  fun unit_call(flag : Bool) -> u64 := do
    assert_unit(flag)
    return 7
  spec unit_call where
    ensures result == 7
    aborts_if !flag with 19
  verify unit_call

  fun unit_forward() -> Unit := empty()
  spec unit_forward where
    aborts_if false
  verify unit_forward

  fun two_assertions(left : Bool, right : Bool) -> Unit := do
    assert(left, 21)
    assert(right, 22)
  spec two_assertions where
    aborts_if !left with 21
    aborts_if left && !right with 22
  verify two_assertions

  fun code_arithmetic(value : u64) -> Unit := abort(value + 1)
  spec code_arithmetic where
    aborts_if true with value + 1
  verify code_arithmetic

  fun code_division(value : u64) -> Unit := abort(8 / value)
  spec code_division where
    aborts_if value == 0
    aborts_if value != 0 with 8 / value
  verify code_division

  fun code_choice(flag : Bool) -> Unit := abort(if flag then 31 else 32)
  spec code_choice where
    aborts_if true with if flag then 31 else 32
  verify code_choice

  fun nested_block(flag : Bool, permitted : Bool) -> u64 := do
    if flag then
      assert(permitted, 51)
      empty()
    else
      assert(permitted, 52)
      empty()
    return 1
  spec nested_block where
    ensures result == 1
    aborts_if !permitted with if flag then 51 else 52
  verify nested_block

  fun increment(value : u64) -> u64 := value + 1
  spec increment where
    ensures result == value + 1
    aborts_if value == 18446744073709551615 with value + 1
  verify increment

  fun discard_call(value : u64) -> u64 := do
    increment(value)
    return value
  spec discard_call where
    ensures result == value
    aborts_if value == 18446744073709551615 with value + 1
  verify discard_call

  fun unit_local(flag : Bool) -> Unit := do
    let mut value := ()
    value := ()
    if flag then value := () else value := ()
    return value
  spec unit_local where
    aborts_if false
  verify unit_local

  fun wrong_post(flag : Bool) -> u64 := do
    assert(flag, 17)
    return 1
  spec wrong_post where
    ensures result == 2
    aborts_if !flag with 17

  fun wrong_abort(flag : Bool) -> Unit := assert(flag, 19)
  spec wrong_abort where
    aborts_if false

  fun wrong_code(code : u64) -> Unit := abort(code)
  spec wrong_code where
    aborts_if true with 0

  fun unsupported_spec() -> Unit := do
    spec do
      assert false
    ()
  spec unsupported_spec where
    aborts_if false

#guard_msgs (drop error) in
#leaner_verify 0x42::native_statements::wrong_post
#guard_msgs (drop error) in
#leaner_verify 0x42::native_statements::wrong_abort
#guard_msgs (drop error) in
#leaner_verify 0x42::native_statements::wrong_code
#guard_msgs (drop error) in
#leaner_verify 0x42::native_statements::unsupported_spec

#leaner_require_native_all

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  let mut excessive : Array String := #[]
  for function in ["empty", "abort_code", "assertion", "floor", "assert_unit", "unit_call",
      "unit_forward", "two_assertions", "code_arithmetic", "code_division", "code_choice",
      "nested_block", "increment", "discard_call", "unit_local"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».native_statements::{function} ")
    unless measured.size == 2 do throwError "missing native statement stages for {function}"
    let total : Nat := measured.foldl (fun cost sample => cost + sample.heartbeats) 0
    let objects : Nat := measured.foldl (fun cost sample => cost + sample.objects) 0
    logInfo m!"{function}: {total} heartbeats / {objects} objects (all generated stages)"
    if total > 10000000 || objects > 15000 then excessive := excessive.push function
  unless excessive.isEmpty do
    throwError "native statements exceed aggregate 10M/15,000 budgets: {excessive}"
  for (caller, callee) in [(`unit_call, `assert_unit), (`unit_forward, `empty),
      (`discard_call, `increment), (`nested_block, `empty)] do
    let some proof := (← getEnv).find? (`«0x42».native_statements ++ caller ++ `nativeSummary)
        |>.bind (·.value? (allowOpaque := true))
      | throwError "missing native statement caller {caller}"
    unless proof.getUsedConstants.contains (`«0x42».native_statements ++ callee ++ `nativeSummary) do
      throwError "native statement {caller} did not reuse {callee}'s summary"
  for rejected in [`wrong_post, `wrong_abort, `wrong_code, `unsupported_spec] do
    for suffix in [`computation, `nativeSummary, `computationVerified,
        `computationState, `computationRepresents, `typedVerified, `verified] do
      if (← getEnv).contains (`«0x42».native_statements ++ rejected ++ suffix) then
        throwError "rejected statement leaked {rejected}.{suffix}"

set_option maxHeartbeats 1000

open LeanerIR LeanerIR.Proofs «0x42».native_statements in
example (state : RuntimeState) : (empty.computation ⟨⟩).ok state () state := by
  have executed : empty.computation ⟨⟩ = Spec.pure () := by simp [empty.computation]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_statements in
example (state : RuntimeState) : (assert_unit.computation ⟨true⟩).ok state () state := by
  have executed : assert_unit.computation ⟨true⟩ = Spec.pure () := by simp [assert_unit.computation]
  rw [executed]
  exact ⟨rfl, rfl⟩

open LeanerIR LeanerIR.Proofs «0x42».native_statements in
example (state : RuntimeState) (error : Failure) :
    (assertion.computation ⟨false⟩).aborts state error ↔ error = (.abort, #[.integer 17]) := by
  have executed : assertion.computation ⟨false⟩ = Spec.abort (.abort, #[.integer 17]) := by
    simp [assertion.computation, NativeArithmetic.runtimeFailure]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_statements in
example (state : RuntimeState) (error : Failure) :
    (two_assertions.computation ⟨false, false⟩).aborts state error ↔ error = (.abort, #[.integer 21]) := by
  have executed : two_assertions.computation ⟨false, false⟩ = Spec.abort (.abort, #[.integer 21]) := by
    simp [two_assertions.computation, NativeArithmetic.runtimeFailure]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_statements in
example (state : RuntimeState) (error : Failure) :
    (two_assertions.computation ⟨true, false⟩).aborts state error ↔ error = (.abort, #[.integer 22]) := by
  have executed : two_assertions.computation ⟨true, false⟩ = Spec.abort (.abort, #[.integer 22]) := by
    simp [two_assertions.computation, NativeArithmetic.runtimeFailure]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_statements in
example (state : RuntimeState) (error : Failure) :
    (code_division.computation ⟨⟨0, by decide⟩⟩).aborts state error ↔ error = (.abort, #[]) := by
  have executed : code_division.computation ⟨⟨0, by decide⟩⟩ = Spec.abort (.abort, #[]) := by
    simp [code_division.computation, NativeArithmetic.divisionResult, NativeArithmetic.emptyFailure,
      Spec.ofExcept]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_statements in
example (state : RuntimeState) (error : Failure) :
    (code_arithmetic.computation ⟨⟨18446744073709551615, by decide⟩⟩).aborts state error ↔
      error = (.abort, #[.integer 18446744073709551616]) := by
  have executed : code_arithmetic.computation ⟨⟨18446744073709551615, by decide⟩⟩ =
      Spec.abort (.abort, #[.integer 18446744073709551616]) := by
    simp [code_arithmetic.computation, NativeArithmetic.checkedInteger, NativeArithmetic.runtimeFailure,
      IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]
  rw [executed]
  rfl

open LeanerIR LeanerIR.Proofs «0x42».native_statements in
example (state : RuntimeState) : (unit_call.computation ⟨true⟩).ok state ⟨7, by decide⟩ state := by
  have executed : unit_call.computation ⟨true⟩ = Spec.pure ⟨7, by decide⟩ := by
    simp [unit_call.computation, assert_unit.computation]
  rw [executed]
  exact ⟨rfl, rfl⟩
