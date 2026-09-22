-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang
set_option Elab.async false
set_option maxHeartbeats 50000
set_option leaner.verifyHeartbeats 50000
#leaner_measure

leaner module 0x42::nested_loops where
  fun labeled_proof() -> u64 := do
    loop@outer do
      loop do
        break@outer
    return 7
  spec labeled_proof where
    ensures result == 7
    aborts_if false
  verify labeled_proof

  fun labeled_exit(n : u64) -> u64 := do
    let mut remaining := n
    loop@outer do
      loop do
        if remaining < 1 then break@outer
        remaining := remaining - 1
        break
    return remaining
  spec labeled_exit where
    ensures result == 0
    aborts_if false
  verify labeled_exit

  fun labeled_continue(n : u64) -> u64 := do
    let mut remaining := n
    loop@outer do
      loop do
        if remaining < 1 then break@outer
        remaining := remaining - 1
        continue@outer
    return remaining
  spec labeled_continue where
    ensures result == 0
    aborts_if false
  verify labeled_continue

  fun different_headers(n : u64) -> u64 := do
    let mut remaining := n
    loop@outer do
      let inner_flag := true
      loop do
        if inner_flag then
          remaining := 0
          break@outer
    return remaining
  spec different_headers where
    ensures result == 0
    aborts_if false
  verify different_headers

  fun three_levels(n : u64) -> u64 := do
    let mut remaining := n
    loop@outer do
      loop do
        loop do
          if remaining < 1 then break@outer
          remaining := remaining - 1
          continue@outer
    return remaining
  spec three_levels where
    ensures result == 0
    aborts_if false
  verify three_levels

  fun normal_exit(n : u64) -> u64 := do
    let mut remaining := n
    while 0 < remaining do
      loop do
        remaining := 0
        break
    return remaining
  spec normal_exit where
    ensures result == 0
    aborts_if false
  verify normal_exit

  fun local_exit(n : u64) -> u64 := do
    let mut remaining := n
    loop do
      let mut temporary := remaining
      loop do
        temporary := 0
        break
      remaining := temporary
      break
    return remaining
  spec local_exit where
    ensures result == 0
    aborts_if false
  verify local_exit

  fun nested_abort(n : u64) -> u64 := do
    loop do
      loop do
        abort(n)
    return 0
  spec nested_abort where
    aborts_if true with n
  verify nested_abort

  fun bounded_nested(n : u64) -> u64 := do
    let mut remaining := n
    while 0 < remaining do
      while 10 < remaining do
        remaining := remaining - 10
      where
        invariant 0 < remaining
      remaining := remaining - 1
    return remaining
  spec bounded_nested where
    ensures result == 0
    aborts_if false
  verify bounded_nested

  fun wrong_exit() -> u64 := do
    loop@outer do
      loop do
        break@outer
    return 7
  spec wrong_exit where
    ensures result == 8
    aborts_if false

  fun wrong_abort(n : u64) -> u64 := do
    loop do
      loop do
        abort(n)
    return 0
  spec wrong_abort where
    aborts_if false

#guard_msgs (drop error) in
#leaner_verify 0x42::nested_loops::wrong_exit
#guard_msgs (drop error) in
#leaner_verify 0x42::nested_loops::wrong_abort

#leaner_require_native 0x42::nested_loops::labeled_proof
#leaner_require_native 0x42::nested_loops::labeled_exit
#leaner_require_native 0x42::nested_loops::labeled_continue
#leaner_require_native 0x42::nested_loops::different_headers
#leaner_require_native 0x42::nested_loops::three_levels
#leaner_require_native 0x42::nested_loops::normal_exit
#leaner_require_native 0x42::nested_loops::local_exit
#leaner_require_native 0x42::nested_loops::nested_abort
#leaner_require_native 0x42::nested_loops::bounded_nested

#leaner_require_native_all

open Lean Elab Command in
run_cmd do
  LeanerLang.Perf.measuring.set false
  let samples ← LeanerLang.Perf.recorded.get
  for function in ["labeled_proof", "labeled_exit", "labeled_continue", "different_headers",
      "three_levels", "normal_exit", "local_exit", "nested_abort", "bounded_nested"] do
    let measured := samples.filter (·.target.startsWith s!"«0x42».nested_loops::{function} ")
    unless measured.size == 2 do throwError "missing native nested-loop stages for {function}"
    let total := measured.foldl (fun cost sample => cost + sample.heartbeats) 0
    let objects := measured.foldl (fun cost sample => cost + sample.objects) 0
    logInfo m!"{function}: {total} heartbeats / {objects} objects (all generated stages)"
    if total > 20000000 || objects > 30000 then
      throwError "native nested-loop {function} exceeds the aggregate 20M/30,000 budget"
    let base := `«0x42».nested_loops ++ Name.mkSimple function
    let some computation := (← getEnv).find? (base ++ `computation) |>.bind (·.value?)
      | throwError "missing native nested-loop computation {function}"
    for retired in [``LeanerIR.RuntimeFrame, ``LeanerIR.RuntimeValue,
        ``LeanerIR.Proofs.ComputationAgreement.fromFrame, ``LeanerIR.Proofs.Codec.decode?, ``Option] do
      if computation.getUsedConstants.contains retired then
        throwError "nested-loop {function} carries a dead/encoded cell: {retired}"
    let infos := (LeanerLang.NativeLoopInfo.entries.getState (← getEnv)).find? base |>.getD #[]
    unless infos.size == (if function == "three_levels" then 3 else 2) do
      throwError "wrong nested-loop invariant inventory for {function}"
    if function == "different_headers" || function == "local_exit" then
      unless (infos.map (·.slots.size)).contains 2 && (infos.map (·.slots.size)).contains 3 do
        throwError "nested-loop {function} did not exercise heterogeneous headers"
  for rejected in [`wrong_exit, `wrong_abort] do
    let base := `«0x42».nested_loops ++ rejected
    for suffix in [`computation, `nativeSummary, `computationVerified,
        `computationState, `computationRepresents, `typedVerified, `verified] do
      if (← getEnv).contains (base ++ suffix) then
        throwError "rejected nested-loop leaked {rejected}.{suffix}"
    if ((LeanerLang.NativeLoopInfo.entries.getState (← getEnv)).find? base).isSome then
      throwError "rejected nested-loop leaked invariant metadata: {rejected}"
  for name in [``LeanerIR.Proofs.ComputationAgreement.observed_nested_loop,
      ``LeanerIR.Proofs.ComputationAgreement.observed_routed_sequence,
      ``LeanerIR.Proofs.ComputationAgreement.observed_map,
      ``LeanerIR.Proofs.ComputationAgreement.ControlRoute.root_encode] do
    if (← collectAxioms name).contains ``sorryAx then
      throwError "nested execution law contains an admission: {name}"
  let rec occurrences (name : Name) (expression : Lean.Expr) : Nat :=
    (if expression.isConstOf name then 1 else 0) + match expression with
      | .app fn argument => occurrences name fn + occurrences name argument
      | .lam _ type body _ | .forallE _ type body _ => occurrences name type + occurrences name body
      | .letE _ type value body _ => occurrences name type + occurrences name value + occurrences name body
      | .mdata _ body | .proj _ _ body => occurrences name body
      | _ => 0
  for (function, expected) in [(`labeled_exit, 2), (`three_levels, 3)] do
    let some computation := (← getEnv).find? (`«0x42».nested_loops ++ function ++ `computation)
        |>.bind (·.value?) | throwError "missing nested-loop computation {function}"
    unless occurrences ``LeanerIR.Proofs.NativeLoop.run computation == expected do
      throwError "nested-loop {function} duplicated or unrolled its loop"
    unless occurrences ``LeanerIR.Proofs.NativeArithmetic.checkedInteger computation == 1 do
      throwError "nested-loop {function} duplicated its decrement"

-- Exact execution uses the generated native computation and finite runs.
set_option maxHeartbeats 1000
open LeanerIR LeanerIR.Proofs «0x42».nested_loops

example (state : RuntimeState) :
    (labeled_proof.computation ⟨⟩).ok state ⟨7, by decide⟩ state := by
  simp only [labeled_proof.computation]
  refine ⟨(), state, ?_, rfl, rfl⟩
  apply (NativeLoop.run_ok ..).mpr
  apply NativeLoop.Runs.done
  refine ⟨.break_ (), ?_, rfl⟩
  refine ⟨.break_ (), state, ?_, rfl, rfl⟩
  refine ⟨.break_ (), state, ?_, rfl, rfl⟩
  apply (NativeLoop.run_ok ..).mpr
  apply NativeLoop.Runs.done
  exact ⟨.break_ (.inr ()), ⟨rfl, rfl⟩, rfl⟩

example (state : RuntimeState) :
    (nested_abort.computation ⟨⟨4, by decide⟩⟩).aborts state (.abort, #[.integer 4]) := by
  simp only [nested_abort.computation, Spec.pure_bind]
  left
  apply (NativeLoop.run_aborts ..).mpr
  apply NativeLoop.Fails.here
  left
  left
  apply (NativeLoop.run_aborts ..).mpr
  apply NativeLoop.Fails.here
  exact Or.inl rfl

example (state : RuntimeState) :
    (three_levels.computation ⟨⟨0, by decide⟩⟩).ok state ⟨0, by decide⟩ state := by
  let header : SpecInt (.bits 64) false × SpecInt (.bits 64) false × Unit :=
    (⟨0, by decide⟩, ⟨0, by decide⟩, ())
  simp only [three_levels.computation, Spec.pure_bind]
  refine ⟨header, state, ?_, rfl, rfl⟩
  apply (NativeLoop.run_ok ..).mpr
  apply NativeLoop.Runs.done
  refine ⟨.break_ header, ?_, rfl⟩
  refine ⟨.break_ header, state, ?_, rfl, rfl⟩
  refine ⟨.break_ header, state, ?_, rfl, rfl⟩
  apply (NativeLoop.run_ok ..).mpr
  apply NativeLoop.Runs.done
  refine ⟨.break_ (.inr header), ?_, rfl⟩
  refine ⟨.break_ (.inr header), state, ?_, rfl, rfl⟩
  refine ⟨.break_ (.inr header), state, ?_, rfl, rfl⟩
  apply (NativeLoop.run_ok ..).mpr
  apply NativeLoop.Runs.done
  refine ⟨.break_ (.inr (.inr header)), ?_, rfl⟩
  exact ⟨.break_ (.inr (.inr header)), state, ⟨rfl, rfl⟩, rfl, rfl⟩

example (state : RuntimeState) :
    (labeled_continue.computation ⟨⟨1, by decide⟩⟩).ok state ⟨0, by decide⟩ state := by
  let before : SpecInt (.bits 64) false × SpecInt (.bits 64) false × Unit :=
    (⟨1, by decide⟩, ⟨1, by decide⟩, ())
  let after : SpecInt (.bits 64) false × SpecInt (.bits 64) false × Unit :=
    (⟨1, by decide⟩, ⟨0, by decide⟩, ())
  simp only [labeled_continue.computation, Spec.pure_bind]
  refine ⟨after, state, ?_, rfl, rfl⟩
  apply (NativeLoop.run_ok ..).mpr
  refine NativeLoop.Runs.next (next := after) (middle := state) ?_ ?_
  · refine ⟨.continue_ after, ?_, rfl⟩
    refine ⟨.continue_ after, state, ?_, rfl, rfl⟩
    refine ⟨.continue_ after, state, ?_, rfl, rfl⟩
    apply (NativeLoop.run_ok ..).mpr
    apply NativeLoop.Runs.done
    refine ⟨.continue_ (.inr after), ?_, rfl⟩
    exact ⟨.normal before, state, ⟨rfl, rfl⟩,
      ⟨⟨0, by decide⟩, state, ⟨rfl, rfl⟩, rfl, rfl⟩⟩
  apply NativeLoop.Runs.done
  refine ⟨.break_ after, ?_, rfl⟩
  refine ⟨.break_ after, state, ?_, rfl, rfl⟩
  refine ⟨.break_ after, state, ?_, rfl, rfl⟩
  apply (NativeLoop.run_ok ..).mpr
  apply NativeLoop.Runs.done
  refine ⟨.break_ (.inr after), ?_, rfl⟩
  exact ⟨.break_ (.inr after), state, ⟨rfl, rfl⟩, rfl, rfl⟩
