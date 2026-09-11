-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang
set_option Elab.async false
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000
#leaner_measure

leaner module 0x42::loop_locals where
  fun shadowed(n : u64) -> u64 := do
    loop do
      let mut n : u64 := 2
      n := 1
      break
    return n
  spec shadowed where
    ensures result == n
    aborts_if false
  verify shadowed

  fun stepped(n : u64) -> u64 := do
    let mut remaining := n
    while 0 < remaining do
      let next := remaining - 1
      remaining := next
    return remaining
  spec stepped where
    ensures result == 0
    aborts_if false
  verify stepped

  fun joined(n : u64) -> u64 := do
    let mut remaining := n
    while 0 < remaining do
      let mut next := remaining
      if 1 < next then
        next := next - 2
      else
        next := 0
      remaining := next
    return remaining
  spec joined where
    ensures result == 0
    aborts_if false
  verify joined

  fun continued(n : u64) -> u64 := do
    let mut remaining := n
    while 0 < remaining do
      let next := remaining - 1
      remaining := next
      continue
    return remaining
  spec continued where
    ensures result == 0
    aborts_if false
  verify continued

  fun early_break(n : u64) -> u64 := do
    let mut remaining := n
    loop do
      if remaining < 1 then break
      let next := remaining - 1
      remaining := next
    return remaining
  spec early_break where
    ensures result == 0
    aborts_if false
  verify early_break

  fun bool_local(flag : Bool) -> Bool := do
    let mut current := flag
    while current do
      let next := false
      current := next
    return current
  spec bool_local where
    ensures !result
    aborts_if false
  verify bool_local

  fun unit_local() -> Unit := do
    loop do
      let ignored := ()
      break
  spec unit_local where
    aborts_if false
  verify unit_local

  fun scoped(n : u64) -> u64 := do
    let mut remaining := n
    while 0 < remaining do
      if 1 < remaining then
        let next := remaining - 2
        remaining := next
      else
        remaining := 0
    return remaining
  spec scoped where
    ensures result == 0
    aborts_if false
  verify scoped

  fun abort_temp(n : u64) -> Unit := do
    loop do
      let next := n + 1
      abort(next)
  spec abort_temp where
    ensures false
    aborts_if true with n + 1
  verify abort_temp

  fun phases(n : u64) -> u64 := do
    let mut current := n
    while 0 < current do
      let next := current - 1
      current := next
    while current < 3 do
      let next := current + 1
      current := next
    where
      invariant current <= 3
    return current
  spec phases where
    ensures result == 3
    aborts_if false
  verify phases

  fun decrement(n : u64) -> u64 := n - 1
  spec decrement where
    ensures result == n - 1
    aborts_if n == 0
  verify decrement

  fun called(n : u64) -> u64 := do
    let mut remaining := n
    while 0 < remaining do
      let next := decrement(remaining)
      remaining := next
    return remaining
  spec called where
    ensures result == 0
    aborts_if false
  verify called

  fun wrong_shadow(n : u64) -> u64 := do
    loop do
      let mut n : u64 := 2
      n := 1
      break
    return n
  spec wrong_shadow where
    ensures result == 1
    aborts_if false

  fun wrong_abort(n : u64) -> Unit := do
    loop do
      let next := n + 1
      abort(next)
  spec wrong_abort where
    aborts_if false

#guard_msgs (drop error) in
#leaner_verify 0x42::loop_locals::wrong_shadow
#guard_msgs (drop error) in
#leaner_verify 0x42::loop_locals::wrong_abort

#leaner_require_native_all

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  for function in ["shadowed", "stepped", "joined", "continued", "early_break", "bool_local", "unit_local",
      "scoped", "abort_temp", "phases", "decrement", "called"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».loop_locals::{function} ")
    unless measured.size == 2 do throwError "missing native local-loop stages for {function}"
    let total := measured.foldl (fun cost sample => cost + sample.heartbeats) 0
    let objects := measured.foldl (fun cost sample => cost + sample.objects) 0
    logInfo m!"{function}: {total} heartbeats / {objects} objects (all generated stages)"
    if total > 20000000 || objects > 30000 then
      throwError "native local-loop {function} exceeds the aggregate 20M/30,000 budget"
    let base := `«0x42».loop_locals ++ Name.mkSimple function
    let some computation := (← getEnv).find? (base ++ `computation) |>.bind (·.value?)
      | throwError "missing native local-loop computation {function}"
    for retired in [``LeanerIR.RuntimeFrame, ``LeanerIR.RuntimeValue,
        ``LeanerIR.Proofs.ComputationAgreement.fromFrame, ``LeanerIR.Proofs.Codec.decode?, ``Option] do
      if computation.getUsedConstants.contains retired then
        throwError "local-loop {function} carries a dead/encoded cell in its computation: {retired}"
    if function != "decrement" then
      let infos := (LeanerLang.NativeLoopInfo.entries.getState (← getEnv)).find? base |>.getD #[]
      unless infos.size == (if function == "phases" then 2 else 1) do
        throwError "wrong local-loop invariant inventory for {function}"
      let expectedLocals := if function == "unit_local" then 0
        else if function == "shadowed" || function == "abort_temp" then 1 else 2
      for info in infos do
        unless info.slots.size == expectedLocals do
          throwError "local-loop {function} carried a lexical temporary across iterations"
  let some caller := (← getEnv).find? `«0x42».loop_locals.called.nativeSummary
      |>.bind (·.value? (allowOpaque := true)) | throwError "missing local-loop caller summary"
  unless caller.getUsedConstants.contains `«0x42».loop_locals.decrement.nativeSummary do
    throwError "local-loop caller reopened its callee instead of using its summary"
  for rejected in [`wrong_shadow, `wrong_abort] do
    let base := `«0x42».loop_locals ++ rejected
    for suffix in [`computation, `nativeSummary, `computationVerified,
        `computationState, `computationRepresents, `typedVerified, `verified] do
      if (← getEnv).contains (base ++ suffix) then
        throwError "rejected local-loop leaked {rejected}.{suffix}"
    if ((LeanerLang.NativeLoopInfo.entries.getState (← getEnv)).find? base).isSome then
      throwError "rejected local-loop leaked invariant metadata: {rejected}"
  for name in [``LeanerIR.Proofs.ComputationAgreement.observed_loop,
      ``LeanerIR.Proofs.ComputationAgreement.observed_sequence,
      ``LeanerIR.Proofs.ComputationAgreement.fromFrame_observed_discard,
      ``LeanerIR.Proofs.ComputationAgreement.borrowFreeCells] do
    if (← collectAxioms name).contains ``sorryAx then
      throwError "observed execution law contains an admission: {name}"
  let rec occurrences (name : Name) (expression : Lean.Expr) : Nat :=
    (if expression.isConstOf name then 1 else 0) + match expression with
      | .app fn argument => occurrences name fn + occurrences name argument
      | .lam _ type body _ | .forallE _ type body _ => occurrences name type + occurrences name body
      | .letE _ type value body _ => occurrences name type + occurrences name value + occurrences name body
      | .mdata _ body | .proj _ _ body => occurrences name body
      | _ => 0
  for (function, symbol, expected) in [
      (`joined, ``LeanerIR.Proofs.NativeArithmetic.checkedInteger, 1),
      (`phases, ``LeanerIR.Proofs.NativeLoop.run, 2)] do
    let some computation := (← getEnv).find? (`«0x42».loop_locals ++ function ++ `computation) |>.bind (·.value?)
      | throwError "missing local-loop computation {function}"
    unless occurrences symbol computation == expected do
      throwError "local-loop {function} duplicated or unrolled {symbol}"

-- Execute the generated native computations, not just their contracts.
set_option maxHeartbeats 1000
open LeanerIR LeanerIR.Proofs «0x42».loop_locals

example (state : RuntimeState) :
    (shadowed.computation ⟨⟨7, by decide⟩⟩).ok state ⟨7, by decide⟩ state := by
  simp only [shadowed.computation, Spec.pure_bind]
  refine ⟨(⟨7, by decide⟩, ()), state, ?_, rfl, rfl⟩
  apply (NativeLoop.run_ok ..).mpr
  apply NativeLoop.Runs.done
  exact ⟨.break_ (⟨7, by decide⟩, ()), ⟨rfl, rfl⟩, rfl⟩

example (state : RuntimeState) :
    (stepped.computation ⟨⟨2, by decide⟩⟩).ok state ⟨0, by decide⟩ state := by
  simp only [stepped.computation, Spec.pure_bind]
  refine ⟨(⟨2, by decide⟩, ⟨0, by decide⟩, ()), state, ?_, rfl, rfl⟩
  apply (NativeLoop.run_ok ..).mpr
  refine NativeLoop.Runs.next (next := (⟨2, by decide⟩, ⟨1, by decide⟩, ())) (middle := state) ?_ ?_
  · refine ⟨.normal (⟨2, by decide⟩, ⟨1, by decide⟩, ()), ?_, rfl⟩
    exact ⟨⟨1, by decide⟩, state, ⟨rfl, rfl⟩, rfl, rfl⟩
  refine NativeLoop.Runs.next (next := (⟨2, by decide⟩, ⟨0, by decide⟩, ())) (middle := state) ?_ ?_
  · refine ⟨.normal (⟨2, by decide⟩, ⟨0, by decide⟩, ()), ?_, rfl⟩
    exact ⟨⟨0, by decide⟩, state, ⟨rfl, rfl⟩, rfl, rfl⟩
  apply NativeLoop.Runs.done
  exact ⟨.break_ (⟨2, by decide⟩, ⟨0, by decide⟩, ()), ⟨rfl, rfl⟩, rfl⟩

example (state : RuntimeState) :
    (bool_local.computation ⟨true⟩).ok state false state := by
  simp only [bool_local.computation, Spec.pure_bind]
  refine ⟨(true, false, ()), state, ?_, rfl, rfl⟩
  apply (NativeLoop.run_ok ..).mpr
  refine NativeLoop.Runs.next (next := (true, false, ())) (middle := state) ?_ ?_
  · exact ⟨.normal (true, false, ()), ⟨rfl, rfl⟩, rfl⟩
  apply NativeLoop.Runs.done
  exact ⟨.break_ (true, false, ()), ⟨rfl, rfl⟩, rfl⟩

example (state : RuntimeState) :
    (abort_temp.computation ⟨⟨4, by decide⟩⟩).aborts state (.abort, #[.integer 5]) := by
  simp only [abort_temp.computation, Spec.pure_bind]
  left
  apply (NativeLoop.run_aborts ..).mpr
  apply NativeLoop.Fails.here
  exact Or.inr ⟨⟨5, by decide⟩, state, ⟨rfl, rfl⟩, Or.inl rfl⟩
