-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option Elab.async false
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000

#leaner_measure

leaner module 0x42::native_loops where
  fun count(limit : u64) -> u64 := do
    let mut current : u64 := 0
    loop if current < limit then do
      current := current + 1
    else break
    spec do
      invariant current <= limit
    return current
  spec count where
    ensures result == limit
    aborts_if false
  verify count

  fun continued(limit : u64) -> u64 := do
    let mut current : u64 := 0
    loop if current < limit then do
      current := current + 1
      continue
    else break
    spec do
      invariant current <= limit
    return current
  spec continued where
    ensures result == limit
    aborts_if false
  verify continued

  fun down(value : u64) -> u64 := do
    let mut remaining := value
    while 0 < remaining do
      remaining := remaining - 1
    return remaining
  spec down where
    ensures result == 0
    aborts_if false
  verify down

  fun early_break(value : u64) -> u64 := do
    let mut remaining := value
    loop do
      if remaining < 1 then break
      remaining := remaining - 1
    return remaining
  spec early_break where
    ensures result == 0
    aborts_if false
  verify early_break

  fun phases(value : u64) -> u64 := do
    let mut current := value
    while 0 < current do
      current := current - 1
    while current < 3 do
      current := current + 1
    where
      invariant current <= 3
    return current
  spec phases where
    ensures result == 3
    aborts_if false
  verify phases

  fun flag_loop(flag : Bool) -> Unit := do
    let mut current := flag
    while current do
      current := false
  spec flag_loop where
    aborts_if false
  verify flag_loop

  fun joined(value : u64) -> u64 := do
    let mut current := value
    loop do
      if current < 1 then break
      if 1 < current then
        current := current - 2
      else
        current := 0
    return current
  spec joined where
    ensures result == 0
    aborts_if false
  verify joined

  fun abort_in_loop(flag : Bool) -> Unit := do
    loop do
      if flag then do
        abort(19)
      break
  spec abort_in_loop where
    aborts_if flag with 19
  verify abort_in_loop

  fun decrement(value : u64) -> u64 := value - 1
  spec decrement where
    ensures result == value - 1
    aborts_if value == 0
  verify decrement

  fun called(value : u64) -> u64 := do
    let mut current := value
    while 0 < current do
      current := decrement(current)
    return current
  spec called where
    ensures result == 0
    aborts_if false
  verify called

  fun once(value : u64) -> u64 := do
    let mut current := value
    loop do
      current := 0
      break
    return current
  spec once where
    ensures result == 0
    aborts_if false
  verify once

  fun diverging() -> Unit := do
    loop do
      continue
  spec diverging where
    ensures false
    aborts_if false
  verify diverging

  fun wrong_entry() -> u64 := do
    let mut current : u64 := 1
    while 0 < current do
      current := current - 1
    where
      invariant current <= 0
    return current
  spec wrong_entry where
    ensures result == 0
    aborts_if false

  fun wrong_preservation() -> u64 := do
    let mut current : u64 := 0
    while current < 2 do
      current := current + 1
    where
      invariant current <= 1
    return current
  spec wrong_preservation where
    ensures result == 2
    aborts_if false

  fun wrong_exit(value : u64) -> u64 := do
    let mut current := value
    while 0 < current do
      current := current - 1
    return current
  spec wrong_exit where
    ensures result == 1
    aborts_if false

  fun unconsumed_assert() -> Unit := do
    spec do
      assert false
    loop do
      break
  spec unconsumed_assert where
    aborts_if false

#guard_msgs (drop error) in
#leaner_verify 0x42::native_loops::wrong_entry
#guard_msgs (drop error) in
#leaner_verify 0x42::native_loops::wrong_preservation
#guard_msgs (drop error) in
#leaner_verify 0x42::native_loops::wrong_exit
#guard_msgs (drop error) in
#leaner_verify 0x42::native_loops::unconsumed_assert

#leaner_require_native_all

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  for function in ["count", "continued", "down", "early_break", "phases", "flag_loop",
      "joined", "abort_in_loop", "decrement", "called", "once", "diverging"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».native_loops::{function} ")
    unless measured.size == 2 do throwError "missing native loop stages for {function}"
    let total := measured.foldl (fun cost sample => cost + sample.heartbeats) 0
    let objects := measured.foldl (fun cost sample => cost + sample.objects) 0
    logInfo m!"{function}: {total} heartbeats / {objects} objects (all generated stages)"
    if total > 20000000 || objects > 30000 then
      throwError "native loop {function} exceeds the aggregate 20M/30,000 budget"
    let base := `«0x42».native_loops ++ Name.mkSimple function
    let some computation := (← getEnv).find? (base ++ `computation) |>.bind (·.value?)
      | throwError "missing native loop computation {function}"
    for retired in [``LeanerIR.RuntimeFrame, ``LeanerIR.RuntimeValue,
        ``LeanerIR.Proofs.ComputationAgreement.fromFrame, ``LeanerIR.Proofs.Codec.decode?] do
      if computation.getUsedConstants.contains retired then
        throwError "native loop {function} contains retired representation {retired}"
    if function != "decrement" &&
        !computation.getUsedConstants.contains ``LeanerIR.Proofs.NativeLoop.run then
      throwError "native loop {function} does not use the typed fixed point"
    if function != "decrement" then
      let infos := (LeanerLang.NativeLoopInfo.entries.getState (← getEnv)).find? base |>.getD #[]
      unless infos.size == (if function == "phases" then 2 else 1) do
        throwError "native loop {function} has the wrong invariant inventory"
      for info in infos do
        let some predicate := (← getEnv).find? info.predicate |>.bind (·.value?)
          | throwError "missing typed invariant {info.predicate}"
        for retired in [``LeanerIR.RuntimeFrame, ``LeanerIR.RuntimeValue,
            ``LeanerIR.SemanticOperations.readLocal?, ``LeanerIR.Proofs.Codec.decode?] do
          if predicate.getUsedConstants.contains retired then
            throwError "native invariant {info.predicate} contains retired representation {retired}"
  let some called := (← getEnv).find? `«0x42».native_loops.called.nativeSummary
      |>.bind (·.value? (allowOpaque := true)) | throwError "missing loop caller summary"
  unless called.getUsedConstants.contains `«0x42».native_loops.decrement.nativeSummary do
    throwError "loop caller does not reuse the callee's native summary"
  let rec occurrences (name : Name) (expression : Lean.Expr) : Nat :=
    (if expression.isConstOf name then 1 else 0) + match expression with
      | .app fn argument => occurrences name fn + occurrences name argument
      | .lam _ type body _ | .forallE _ type body _ => occurrences name type + occurrences name body
      | .letE _ type value body _ => occurrences name type + occurrences name value + occurrences name body
      | .mdata _ body | .proj _ _ body => occurrences name body
      | _ => 0
  let some phases := (← getEnv).find? `«0x42».native_loops.phases.computation |>.bind (·.value?)
    | throwError "missing sequential loop computation"
  unless occurrences ``LeanerIR.Proofs.NativeLoop.run phases == 2 do
    throwError "sequential loop computation was duplicated or unrolled"
  let some joined := (← getEnv).find? `«0x42».native_loops.joined.computation |>.bind (·.value?)
    | throwError "missing loop branch join computation"
  unless occurrences ``LeanerIR.Proofs.NativeArithmetic.checkedInteger joined == 1 do
    throwError "loop branch join duplicated its arithmetic body"
  for rejected in [`wrong_entry, `wrong_preservation, `wrong_exit, `unconsumed_assert] do
    for suffix in [`computation, `nativeSummary, `computationVerified,
        `computationState, `computationRepresents, `typedVerified, `verified] do
      if (← getEnv).contains (`«0x42».native_loops ++ rejected ++ suffix) then
        throwError "rejected loop leaked {rejected}.{suffix}"

-- Check the generated computations themselves, including actual iterations and
-- selected/unselected aborts. No source interpreter or contract is substituted.
set_option maxHeartbeats 1000

open LeanerIR LeanerIR.Proofs «0x42».native_loops

example (state : RuntimeState) :
    (once.computation ⟨⟨7, by decide⟩⟩).ok state ⟨0, by decide⟩ state := by
  simp only [once.computation, Spec.pure_bind]
  refine ⟨(⟨7, by decide⟩, ⟨0, by decide⟩, ()), state, ?_, rfl, rfl⟩
  apply (NativeLoop.run_ok ..).mpr
  apply NativeLoop.Runs.done
  simp [NativeFlow.iteration, NativeFlow.Flow.step, Spec.pure]

example (state : RuntimeState) :
    (abort_in_loop.computation ⟨true⟩).aborts state (.abort, #[.integer 19]) := by
  simp only [abort_in_loop.computation, Spec.pure_bind]
  left
  apply (NativeLoop.run_aborts ..).mpr
  apply NativeLoop.Fails.here
  exact Or.inl (Or.inl rfl)

example (state : RuntimeState) :
    (abort_in_loop.computation ⟨false⟩).ok state () state := by
  simp only [abort_in_loop.computation, Spec.pure_bind]
  refine ⟨(false, ()), state, ?_, rfl, rfl⟩
  apply (NativeLoop.run_ok ..).mpr
  apply NativeLoop.Runs.done
  refine ⟨.break_ (false, ()), ?_, rfl⟩
  exact ⟨.normal (false, ()), state, ⟨rfl, rfl⟩, rfl, rfl⟩

example (state : RuntimeState) :
    (count.computation ⟨⟨0, by decide⟩⟩).ok state ⟨0, by decide⟩ state := by
  simp only [count.computation, Spec.pure_bind]
  refine ⟨(⟨0, by decide⟩, ⟨0, by decide⟩, ()), state, ?_, rfl, rfl⟩
  apply (NativeLoop.run_ok ..).mpr
  apply NativeLoop.Runs.done
  simp [NativeFlow.iteration, NativeFlow.Flow.step, Spec.pure]

example (state : RuntimeState) :
    (count.computation ⟨⟨2, by decide⟩⟩).ok state ⟨2, by decide⟩ state := by
  simp only [count.computation, Spec.pure_bind]
  refine ⟨(⟨2, by decide⟩, ⟨2, by decide⟩, ()), state, ?_, rfl, rfl⟩
  apply (NativeLoop.run_ok ..).mpr
  refine NativeLoop.Runs.next (next := (⟨2, by decide⟩, ⟨1, by decide⟩, ())) (middle := state) ?_ ?_
  · refine ⟨.normal (⟨2, by decide⟩, ⟨1, by decide⟩, ()), ?_, rfl⟩
    exact ⟨⟨1, by decide⟩, state, ⟨rfl, rfl⟩, rfl, rfl⟩
  refine NativeLoop.Runs.next (next := (⟨2, by decide⟩, ⟨2, by decide⟩, ())) (middle := state) ?_ ?_
  · refine ⟨.normal (⟨2, by decide⟩, ⟨2, by decide⟩, ()), ?_, rfl⟩
    exact ⟨⟨2, by decide⟩, state, ⟨rfl, rfl⟩, rfl, rfl⟩
  apply NativeLoop.Runs.done
  simp [NativeFlow.iteration, NativeFlow.Flow.step, Spec.pure]
