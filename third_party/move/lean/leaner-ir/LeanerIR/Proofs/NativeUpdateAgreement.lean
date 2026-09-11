-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeMutationAgreement
import LeanerIR.Proofs.NativeFlowAgreement

namespace LeanerIR.Proofs.ComputationAgreement

open Denotation
set_option maxHeartbeats 1000

/-- Effectful operands feed a frame-changing operation. The native result
retains the updated typed owner; the execution frame appears only in this
certificate. An operand failure bypasses the update entirely. -/
theorem controlled_operation_update (evaluate : NativeEvaluator)
    (operands : ValuesDenotation) (entry : RuntimeFrame)
    (encodeArgs : Args → List RuntimeValue) (arguments : Spec RuntimeState Failure Args)
    (exit : Result → RuntimeFrame) (encodeResult : Result → RuntimeValue) (value : Args → Result)
    (operandAgreement : Operands operands entry encodeArgs arguments)
    (evaluates : ∀ args state, evaluate (encodeArgs args).toArray entry state =
      some (.value (exit (value args)) state (encodeResult (value args)))) :
    Controlled (nativeOperation evaluate operands) entry exit (fun result => .value (encodeResult result))
      (Spec.bind arguments (fun args => Spec.pure (value args))) := by
  constructor
  · intro initial finalFrame finalState control nonThrow
    constructor
    · rintro (⟨frame, state, flow, aborted, _, _, equal⟩ |
        ⟨frame, state, values, ran, success | failure⟩)
      · obtain ⟨kind, thrown, rfl⟩ := operandAgreement.control _ _ _ _ aborted
        subst control
        exact nonThrow.elim
      · obtain ⟨args, argsStep, rfl, rfl⟩ := (operandAgreement.normal _ _ _ _).mp ran
        obtain ⟨runtimeValue, evaluated, controlled⟩ := success
        rw [evaluates] at evaluated
        cases evaluated
        exact ⟨value args, ⟨args, finalState, argsStep, rfl, rfl⟩, rfl, controlled⟩
      · obtain ⟨_, _, _, equal⟩ := failure
        subst control
        exact nonThrow.elim
    · rintro ⟨result, ⟨args, state, argsStep, equal, sameState⟩, sameFrame, controlled⟩
      subst result state finalFrame control
      exact Or.inr ⟨entry, finalState, encodeArgs args,
        (operandAgreement.normal _ _ _ _).mpr ⟨args, argsStep, rfl, rfl⟩,
        Or.inl ⟨encodeResult (value args), evaluates args finalState, rfl⟩⟩
  · intro initial error
    constructor
    · rintro ⟨finalFrame, finalState, executed⟩
      rcases executed with ⟨frame, state, control, aborted, _, _, equal⟩ |
        ⟨frame, state, values, ran, success | failure⟩
      · cases error with | mk actualKind actualValues =>
          cases equal
          exact Or.inl ((operandAgreement.aborts _ _).mp ⟨frame, state, aborted⟩)
      · obtain ⟨_, _, impossible⟩ := success
        cases impossible
      · obtain ⟨args, _, rfl, rfl⟩ := (operandAgreement.normal _ _ _ _).mp ran
        obtain ⟨_, _, evaluated, _⟩ := failure
        rw [evaluates] at evaluated
        cases evaluated
    · rintro (aborted | ⟨_, _, _, impossible⟩)
      · obtain ⟨frame, state, ran⟩ := (operandAgreement.aborts _ _).mpr aborted
        exact ⟨frame, state, Or.inl ⟨frame, state, .throw_ error.1 error.2, ran, rfl, rfl, rfl⟩⟩
      · exact impossible.elim
  · intro initial undefined
    rcases undefined with undefined | ⟨_, _, _, impossible⟩
    · exact operandAgreement.defined _ undefined
    · exact impossible
  · exact fun _ => trivial

end LeanerIR.Proofs.ComputationAgreement
