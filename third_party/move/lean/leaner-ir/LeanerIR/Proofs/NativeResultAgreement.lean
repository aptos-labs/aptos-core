-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.OperandAgreement
import LeanerIR.Proofs.NativeResult

namespace LeanerIR.Proofs.ComputationAgreement
open LeanerIR.SemanticOperations LeanerIR.BigStep Denotation

/-- One shared conversion avoids independently generated match helpers at
each evaluator certificate. This encoding is confined to agreement. -/
def resultControl (encode : Result → RuntimeValue) (frame : RuntimeFrame)
    (state : RuntimeState) (result : Except Failure Result) : GlobalOperationResult :=
  match result with
  | .ok value => .value frame state (encode value)
  | .error error => .throw_ frame state error.1 error.2

/-- Exact native operation results after ordered, effectful operands. Neither
operand nor operation failures can be replaced by a successful value. -/
theorem scalar_operation_result (evaluate : NativeEvaluator)
    (operands : ValuesDenotation) (entry : RuntimeFrame)
    (encodeArgs : Args → List RuntimeValue) (arguments : Spec RuntimeState Failure Args)
    (encodeResult : Result → RuntimeValue) (result : Args → Except Failure Result)
    (operandAgreement : Operands operands entry encodeArgs arguments)
    (evaluates : ∀ args state, evaluate (encodeArgs args).toArray entry state =
      some (resultControl encodeResult entry state (result args))) :
    Scalar (nativeOperation evaluate operands) entry encodeResult
      (Spec.bind arguments (fun args => Spec.ofExcept (result args))) := by
  constructor
  · intro initial finalFrame finalState actual
    constructor
    · rintro (⟨frame, state, control, aborted, _, _, equal⟩ |
        ⟨frame, state, values, ran, success | failure⟩)
      · obtain ⟨kind, thrown, rfl⟩ := operandAgreement.control _ _ _ _ aborted
        cases equal
      · obtain ⟨args, argsStep, rfl, rfl⟩ := (operandAgreement.normal _ _ _ _).mp ran
        obtain ⟨runtimeValue, evaluated, controlled⟩ := success
        rw [evaluates] at evaluated
        cases step : result args with
        | error error => simp [step, resultControl] at evaluated
        | ok value =>
          simp only [step, resultControl] at evaluated
          cases evaluated
          cases controlled
          exact ⟨value, ⟨args, finalState, argsStep,
            by simp [Spec.ofExcept, step, Spec.pure]⟩, rfl, rfl⟩
      · obtain ⟨_, _, _, controlled⟩ := failure
        cases controlled
    · rintro ⟨value, ⟨args, state, argsStep, checked⟩, sameFrame, rfl⟩
      subst finalFrame
      cases step : result args with
      | error error => simp [Spec.ofExcept, step, Spec.abort] at checked
      | ok actual =>
        simp only [Spec.ofExcept, step, Spec.pure] at checked
        obtain ⟨rfl, rfl⟩ := checked
        exact Or.inr ⟨entry, finalState, encodeArgs args,
          (operandAgreement.normal _ _ _ _).mpr ⟨args, argsStep, rfl, rfl⟩,
          Or.inl ⟨encodeResult value, by rw [evaluates, step]; rfl, rfl⟩⟩
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
      · obtain ⟨args, argsStep, rfl, rfl⟩ := (operandAgreement.normal _ _ _ _).mp ran
        obtain ⟨throwKind, thrown, evaluated, controlled⟩ := failure
        rw [evaluates] at evaluated
        cases step : result args with
        | ok value => simp [step, resultControl] at evaluated
        | error actual =>
          simp only [step, resultControl] at evaluated
          cases evaluated
          cases error with | mk kind payload =>
            cases controlled
            exact Or.inr ⟨args, finalState, argsStep,
              by simp [Spec.ofExcept, step, Spec.abort]⟩
    · rintro (aborted | ⟨args, state, argsStep, checked⟩)
      · obtain ⟨frame, state, ran⟩ := (operandAgreement.aborts _ _).mpr aborted
        exact ⟨frame, state, Or.inl ⟨frame, state, .throw_ error.1 error.2, ran, rfl, rfl, rfl⟩⟩
      · cases step : result args with
        | ok value => simp [Spec.ofExcept, step, Spec.pure] at checked
        | error actual =>
          simp only [Spec.ofExcept, step, Spec.abort] at checked
          subst error
          exact ⟨entry, state, Or.inr ⟨entry, state, encodeArgs args,
            (operandAgreement.normal _ _ _ _).mpr ⟨args, argsStep, rfl, rfl⟩,
            Or.inr ⟨actual.1, actual.2, by rw [evaluates, step]; rfl, rfl⟩⟩⟩
  · intro initial finalFrame finalState flow executed abrupt
    rcases executed with ⟨frame, state, control, ran, _, _, rfl⟩ |
      ⟨_, _, _, _, success | failure⟩
    · exact operandAgreement.control _ _ _ _ ran
    · obtain ⟨_, _, rfl⟩ := success; cases abrupt
    · obtain ⟨kind, values, _, rfl⟩ := failure; exact ⟨kind, values, rfl⟩
  · intro initial undefined
    rcases undefined with undefined | ⟨args, _, _, impossible⟩
    · exact operandAgreement.defined _ undefined
    · cases step : result args <;>
        simp [Spec.ofExcept, step, Spec.pure, Spec.abort] at impossible

end LeanerIR.Proofs.ComputationAgreement
