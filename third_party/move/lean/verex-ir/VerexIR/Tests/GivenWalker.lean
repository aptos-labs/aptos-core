-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve
import VerexIR.Tests.GivenExecution

/-!
The normalizer and walker run real Given bodies and the concrete adapter.
The final proof inspection requires each changed marker rule to occur, so a
simplifier cannot bypass the interface under test.
-/

open VerexIR.Proofs VerexIR.Proofs.Denote
namespace VerexIR.Tests.GivenWalker
open VerexIR.Tests.GivenExecution
set_option maxHeartbeats 100000
set_option maxRecDepth 2000

def immediateLoop (unit : VerexIR.Validation.ValidatedUnit) : Function unit where
  params := .nil
  locals := .nil
  frame := .nil
  frame_eq := rfl
  result := .none
  body := .loop 7 (.break_ 0)
  mutables := .nil
  exitSite := 0

def handle : VerexIR.FunctionHandle := ⟨⟨0⟩, ⟨0⟩⟩

-- Real Given body evaluation and leaves use the ordinary solver.
theorem given_identity (G : GivenFrame) (State : Type) (unit : VerexIR.Validation.ValidatedUnit)
    (ops : InterpreterOps unit G.toCarriers State)
    (value : @NTy.carrier G.toCarriers (.param 0)) (initial : State) :
    wp (@Function.denoteIn unit G.toCarriers State ops (identity unit) (value, ()))
      (fun result final => result = value ∧ final = initial) (fun _ => False) initial := by
  simp only [identity, Function.denoteIn]
  verex_normalize
  all_goals verex_obligations []
  all_goals verex_solve

-- The invariant is already at the Given family and state; it has no
-- ValidatedUnit or Skolems factory to reconstruct at the loop marker.
theorem given_loop (G : GivenFrame) (State : Type) (unit : VerexIR.Validation.ValidatedUnit)
    (ops : InterpreterOps unit G.toCarriers State) (initial : State) :
    wp (@Function.denoteIn unit G.toCarriers State ops (immediateLoop unit) ())
      (fun _ final => final = initial) (fun _ => False) initial := by
  simp only [immediateLoop, Function.denoteIn]
  verex_normalize
  verex_obligations [(7, handle,
    fun (_ : Unit) (_ : State) (_ : @HEnv G.toCarriers .nil) (entry : State)
      (_ : @HEnv G.toCarriers .nil) (current : State) => current = entry)]
  all_goals verex_solve

-- Concrete predicates retain their frame explicitly; the result is the
-- same loop invariant under the concrete adapter.
theorem concrete_loop {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit] [Θ : Skolems unit]
    {executable : VerexIR.Validation.ExecutableUnit unit}
    (meanings : Meanings executable) (initial : Memory unit) :
    wp ((immediateLoop unit).denoteWith meanings ())
      (fun _ final => final = initial) (fun _ => False) initial := by
  simp only [immediateLoop, Function.denoteWith]
  verex_normalize
  verex_obligations [(7, handle, concreteCondition Θ (fun frame =>
    fun (_ : Unit) (_ : Memory unit) (_ : @HEnv frame.toCarriers .nil) (entry : Memory unit)
      (_ : @HEnv frame.toCarriers .nil) (current : Memory unit) => current = entry))]
  all_goals verex_solve

-- State-only call markers use the recorded ambient frame.
theorem concrete_call_marker {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit] [_Θ : Skolems unit]
    (initial : Memory unit) :
    wp (callAt 8 (Spec.pure true : Comp unit Bool))
      (fun result final => result = true ∧ final = initial) (fun _ => False) initial := by
  verex_obligations [(8, handle, concreteCondition _Θ (fun _ =>
    fun (_ : Unit) (_ : Memory unit) (before after : Memory unit) => after = before))]
  all_goals verex_solve

section Markers
variable [C : Carriers] {State : Type}

theorem generic_assertion (initial : State) :
    wp (@assertAt C State .none .nil 11 ())
      (fun result final => result = Flow.value () () ∧ final = initial)
      (fun _ => False) initial := by
  verex_obligations [(11, handle,
    fun (_ : Unit) (_ : State) (_ : @HEnv C .nil) (_ : State) => True)]
  all_goals verex_solve

theorem generic_memory (initial : State) :
    wp (Flow.bind (@anchorAt C State .none .nil 12 ()) fun _ env => memoryWrittenAt 12 env)
      (fun result final => result = Flow.value () () ∧ final = initial)
      (fun _ => False) initial := by
  verex_obligations [(12, handle,
    fun (_ : Unit) (_ : State) (_ : @HEnv C .nil) (_ : State) => True)]
  all_goals verex_solve

theorem generic_ghost (value : NTy.carrier (.param 0)) (initial : State) :
    wp (@ghostAt C State .none .nil (.param 0) 13 (fun _ => some value) ())
      (fun result final => result = Flow.value value () ∧ final = initial)
      (fun _ => False) initial := by
  verex_obligations []
  all_goals verex_solve

theorem generic_construction (value : NTy.carrier (.param 0)) (initial : State) :
    wp (@constructedAt C State .none .nil _ 14 value ())
      (fun result final => result = Flow.value value () ∧ final = initial)
      (fun _ => False) initial := by
  verex_obligations [(14, handle,
    fun (_ : Unit) (_ : State) (current : NTy.carrier (.param 0)) (_ : State) => current = value)]
  all_goals verex_solve

theorem generic_mutation_end (initial : State) :
    wp (@mutationEndAt C State .none .nil 15 ())
      (fun result final => result = Flow.value () () ∧ final = initial)
      (fun _ => False) initial := by
  verex_obligations [(15, handle,
    fun (_ : Unit) (_ : State) (_ : @HEnv C .nil) (_ : State) => True)]
  all_goals verex_solve
end Markers

theorem given_certification (G : GivenFrame) (State : Type)
    (value : @NTy.plain G.toCarriers (.param 0)) (initial : State) :
    wp (@certifyingAt G.toCarriers State 16 .copyOut (.param 0) value)
      (fun _ final => final = initial) (fun _ => False) initial := by
  verex_obligations []
  all_goals verex_solve

theorem given_unroll (G : GivenFrame) (State : Type) (unit : VerexIR.Validation.ValidatedUnit)
    (ops : InterpreterOps unit G.toCarriers State) (initial : State) :
    wp (@Function.denoteIn unit G.toCarriers State ops (immediateLoop unit) ())
      (fun _ final => final = initial) (fun _ => False) initial := by
  simp only [immediateLoop, Function.denoteIn]
  verex_normalize
  verex_obligations [] unrolling [(7, 1)]
  all_goals verex_solve

open Lean Elab Command in
run_cmd do
  for (name, rule) in [(``given_loop, ``wp_loopAt), (``concrete_loop, ``wp_loopAt),
      (``concrete_call_marker, ``wp_callAt_checked),
      (``generic_assertion, ``wp_assertAt), (``generic_memory, ``wp_memoryWrittenAt),
      (``generic_memory, ``wp_anchorAt), (``generic_ghost, ``wp_ghostAt),
      (``generic_construction, ``wp_constructedAt), (``generic_mutation_end, ``wp_mutationEndAt),
      (``given_certification, ``wp_certifyingAt_intro), (``given_unroll, ``wp_loopAt_unroll),
      (``given_unroll, ``wp_loopUnroll_step)] do
    let .thmInfo theoremInfo ← getConstInfo name | throwError "expected theorem"
    unless (theoremInfo.value.find? (fun expression => expression.isConstOf rule)).isSome do
      throwError "{name} did not exercise the walker rule {rule}"

open Lean Elab Command in
run_cmd do
  for name in [``given_identity, ``given_loop, ``concrete_loop, ``concrete_call_marker,
      ``generic_assertion, ``generic_memory, ``generic_ghost, ``generic_construction,
      ``generic_mutation_end, ``given_certification, ``given_unroll] do
    let axioms ← collectAxioms name
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {name}: {axioms}"

end VerexIR.Tests.GivenWalker
