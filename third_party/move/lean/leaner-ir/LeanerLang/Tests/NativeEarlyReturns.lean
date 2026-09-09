-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang
set_option Elab.async false
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000
#leaner_measure

leaner module 0x42::early_returns where
  fun early(flag : Bool) -> u64 := do
    if flag then return 7
    return 8
  spec early where
    ensures result == if flag then 7 else 8
    aborts_if false
  verify early

  fun checked(flag : Bool, n : u64) -> u64 := do
    if flag then return n + 1
    return n
  spec checked where
    requires n < 100
    ensures result == if flag then n + 1 else n
    aborts_if false
  verify checked

  fun in_loop(n : u64) -> u64 := do
    let mut remaining := n
    while 0 < remaining do
      if remaining == 3 then return 1
      remaining := remaining - 1
    return remaining
  spec in_loop where
    ensures result <= 1
    aborts_if false
  verify in_loop

  fun nested(n : u64) -> u64 := do
    let mut remaining := n
    loop do
      loop do
        if remaining < 1 then return remaining
        remaining := remaining - 1
        break
    return 0
  spec nested where
    ensures result == 0
    aborts_if false
  verify nested

  fun local_return(n : u64) -> u64 := do
    loop do
      let next := n + 1
      return next
    return 0
  spec local_return where
    requires n < 100
    ensures result == n + 1
    aborts_if false
  verify local_return

  fun unit_return(flag : Bool) -> Unit := do
    if flag then return ()
    assert(!flag, 7)
  spec unit_return where
    aborts_if false
  verify unit_return

  fun bool_return(flag : Bool) -> Bool := do
    if flag then return false
    return true
  spec bool_return where
    ensures result == !flag
    aborts_if false
  verify bool_return

  fun bypass_abort(flag : Bool) -> u64 := do
    if flag then return 7
    abort(9)
    return 0
  spec bypass_abort where
    ensures result == 7
    aborts_if !flag with 9
  verify bypass_abort

  fun shared_tail(flag : Bool, n : u64) -> u64 := do
    let mut value := n
    if flag then
      if n < 1 then return 7
      value := 1
    else
      value := 2
    value := value + 1
    return value
  spec shared_tail where
    ensures result == if flag then (if n < 1 then 7 else 2) else 3
    aborts_if false
  verify shared_tail

  fun called(flag : Bool) -> u64 := do
    if flag then return early(flag)
    return 8
  spec called where
    ensures result == if flag then 7 else 8
    aborts_if false
  verify called

  fun overflow(flag : Bool, n : u64) -> u64 := do
    if flag then return n + 1
    return 0
  spec overflow where
    ensures result == if flag then n + 1 else 0
    aborts_if flag && n == 18446744073709551615 with n + 1
  verify overflow

  fun wrong_return(flag : Bool) -> u64 := do
    if flag then return 7
    return 8
  spec wrong_return where
    ensures result == 8
    aborts_if false

  fun wrong_abort(flag : Bool, n : u64) -> u64 := do
    if flag then return n + 1
    return 0
  spec wrong_abort where
    aborts_if false

#guard_msgs (drop error) in
#leaner_verify 0x42::early_returns::wrong_return
#guard_msgs (drop error) in
#leaner_verify 0x42::early_returns::wrong_abort

#leaner_require_native 0x42::early_returns::early
#leaner_require_native 0x42::early_returns::checked
#leaner_require_native 0x42::early_returns::in_loop
#leaner_require_native 0x42::early_returns::nested
#leaner_require_native 0x42::early_returns::local_return
#leaner_require_native 0x42::early_returns::unit_return
#leaner_require_native 0x42::early_returns::bool_return
#leaner_require_native 0x42::early_returns::bypass_abort
#leaner_require_native 0x42::early_returns::shared_tail
#leaner_require_native 0x42::early_returns::called
#leaner_require_native 0x42::early_returns::overflow

#leaner_require_native_all

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  for function in ["early", "checked", "in_loop", "nested", "local_return", "unit_return",
      "bool_return", "bypass_abort", "shared_tail", "called", "overflow"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».early_returns::{function} ")
    unless measured.size == 2 do throwError "missing early-return stages for {function}"
    let total := measured.foldl (fun cost sample => cost + sample.heartbeats) 0
    let objects := measured.foldl (fun cost sample => cost + sample.objects) 0
    logInfo m!"{function}: {total} heartbeats / {objects} objects (all generated stages)"
    if total > 20000000 || objects > 30000 then
      throwError "native early return {function} exceeds the aggregate 20M/30,000 budget"
    let base := `«0x42».early_returns ++ Name.mkSimple function
    let some computation := (← getEnv).find? (base ++ `computation) |>.bind (·.value?)
      | throwError "missing native early-return computation {function}"
    for retired in [``LeanerIR.RuntimeFrame, ``LeanerIR.RuntimeValue,
        ``LeanerIR.Proofs.ComputationAgreement.fromFrame, ``LeanerIR.Proofs.Codec.decode?, ``Option] do
      if computation.getUsedConstants.contains retired then
        throwError "early return {function} carries a dead/encoded cell: {retired}"
  for rejected in [`wrong_return, `wrong_abort] do
    let base := `«0x42».early_returns ++ rejected
    for suffix in [`computation, `nativeSummary, `computationVerified,
        `computationState, `computationRepresents, `typedVerified, `verified] do
      if (← getEnv).contains (base ++ suffix) then
        throwError "rejected early return leaked {rejected}.{suffix}"
    if ((LeanerLang.NativeLoopInfo.entries.getState (← getEnv)).find? base).isSome then
      throwError "rejected early return leaked invariant metadata: {rejected}"
  for name in [``LeanerIR.Proofs.ComputationAgreement.fromFrame_returning_discard,
      ``LeanerIR.Proofs.ComputationAgreement.controlled_return,
      ``LeanerIR.Proofs.ComputationAgreement.operands_single] do
    if (← collectAxioms name).contains ``sorryAx then
      throwError "early-return agreement contains an admission: {name}"
  let some caller := (← getEnv).find? `«0x42».early_returns.called.nativeSummary
      |>.bind (·.value? (allowOpaque := true)) | throwError "missing early-return caller summary"
  unless caller.getUsedConstants.contains `«0x42».early_returns.early.nativeSummary do
    throwError "early-return caller reopened its callee instead of using its summary"
  let rec occurrences (name : Name) (expression : Lean.Expr) : Nat :=
    (if expression.isConstOf name then 1 else 0) + match expression with
      | .app fn argument => occurrences name fn + occurrences name argument
      | .lam _ type body _ | .forallE _ type body _ => occurrences name type + occurrences name body
      | .letE _ type value body _ => occurrences name type + occurrences name value + occurrences name body
      | .mdata _ body | .proj _ _ body => occurrences name body
      | _ => 0
  let some joined := (← getEnv).find? `«0x42».early_returns.shared_tail.computation |>.bind (·.value?)
    | throwError "missing shared early-return continuation"
  unless occurrences ``LeanerIR.Proofs.NativeArithmetic.checkedInteger joined == 1 do
    throwError "early-return branches duplicated the shared arithmetic continuation"

set_option maxHeartbeats 1000
open LeanerIR LeanerIR.Proofs «0x42».early_returns

example (state : RuntimeState) :
    (early.computation ⟨true⟩).ok state ⟨7, by decide⟩ state := by
  simp only [early.computation, NativeReturnFlow.finish, ↓reduceIte, Spec.pure_bind]
  exact ⟨rfl, rfl⟩

example (state : RuntimeState) :
    (early.computation ⟨false⟩).ok state ⟨8, by decide⟩ state := by
  simp only [early.computation, NativeReturnFlow.finish, Bool.false_eq_true, ↓reduceIte, Spec.pure_bind]
  exact ⟨rfl, rfl⟩

example (state : RuntimeState) :
    (bypass_abort.computation ⟨true⟩).ok state ⟨7, by decide⟩ state := by
  simp only [bypass_abort.computation, NativeReturnFlow.finish, ↓reduceIte, Spec.pure_bind]
  exact ⟨rfl, rfl⟩

example (state : RuntimeState) :
    (bypass_abort.computation ⟨false⟩).aborts state (.abort, #[.integer 9]) := by
  simp only [bypass_abort.computation, NativeReturnFlow.finish, Bool.false_eq_true, ↓reduceIte, Spec.pure_bind, Spec.abort_bind]
  rfl

example (state : RuntimeState) :
    (unit_return.computation ⟨true⟩).ok state () state := by
  simp only [unit_return.computation, NativeReturnFlow.finish, ↓reduceIte, Spec.pure_bind]
  exact ⟨rfl, rfl⟩

example (state : RuntimeState) :
    (bool_return.computation ⟨true⟩).ok state false state := by
  simp only [bool_return.computation, NativeReturnFlow.finish, ↓reduceIte, Spec.pure_bind]
  exact ⟨rfl, rfl⟩

example (state : RuntimeState) :
    (local_return.computation ⟨⟨4, by decide⟩⟩).ok state ⟨5, by decide⟩ state := by
  simp only [local_return.computation, NativeReturnFlow.finish, Spec.pure_bind]
  refine ⟨.break_ ⟨5, by decide⟩, state, ?_, rfl, rfl⟩
  refine ⟨.break_ ⟨5, by decide⟩, state, ?_, rfl, rfl⟩
  apply (NativeLoop.run_ok ..).mpr
  apply NativeLoop.Runs.done
  refine ⟨.break_ (.inr ⟨5, by decide⟩), ?_, rfl⟩
  exact ⟨⟨5, by decide⟩, state, ⟨rfl, rfl⟩, rfl, rfl⟩

example (state : RuntimeState) :
    (overflow.computation ⟨true, ⟨18446744073709551615, by decide⟩⟩).aborts
      state (.abort, #[.integer 18446744073709551616]) := by
  simp only [overflow.computation, NativeReturnFlow.finish, Spec.pure_bind]
  left
  left
  rfl

example (state : RuntimeState) :
    (nested.computation ⟨⟨0, by decide⟩⟩).ok state ⟨0, by decide⟩ state := by
  let result : SpecInt (.bits 64) false := ⟨0, by decide⟩
  simp only [nested.computation, NativeReturnFlow.finish, Spec.pure_bind]
  refine ⟨.break_ result, state, ?_, rfl, rfl⟩
  refine ⟨.break_ result, state, ?_, rfl, rfl⟩
  apply (NativeLoop.run_ok ..).mpr
  apply NativeLoop.Runs.done
  refine ⟨.break_ (.inr result), ?_, rfl⟩
  refine ⟨.break_ (.inr result), state, ?_, rfl, rfl⟩
  refine ⟨.break_ (.inr result), state, ?_, rfl, rfl⟩
  apply (NativeLoop.run_ok ..).mpr
  apply NativeLoop.Runs.done
  refine ⟨.break_ (.inr (.inr result)), ?_, rfl⟩
  exact ⟨.break_ (.inr (.inr result)), state, ⟨rfl, rfl⟩, rfl, rfl⟩
