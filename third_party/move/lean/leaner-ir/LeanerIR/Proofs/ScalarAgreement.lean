-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.CallAgreement
import LeanerIR.Proofs.SequenceAgreement

namespace LeanerIR.Proofs.ComputationAgreement

open LeanerIR.SemanticOperations LeanerIR.BigStep Denotation

/-- Boundary agreement for scalar expressions. Normal evaluation leaves the
local frame intact; failures may carry arbitrary frames and rollback states.
Neither a callee contract nor a decoder right inverse can establish this. -/
structure Scalar (body : ExprDenotation) (entry : RuntimeFrame)
    (encode : Result → RuntimeValue) (computation : Spec RuntimeState Failure Result) : Prop where
  normal : ∀ initial finalFrame finalState value,
    body entry initial finalFrame finalState (.value value) ↔
      ∃ result, computation.ok initial result finalState ∧ finalFrame = entry ∧ value = encode result
  aborts : ∀ initial error,
    (∃ finalFrame finalState, body entry initial finalFrame finalState (.throw_ error.1 error.2)) ↔
      computation.aborts initial error
  abrupt : ∀ initial finalFrame finalState control,
    body entry initial finalFrame finalState control → Abrupt control →
      ∃ kind values, control = .throw_ kind values
  defined : ∀ initial, ¬computation.undefined initial

theorem scalar_call (handle : FunctionHandle) (callee : FunctionDenotation)
    (operands : ValuesDenotation) (entry : RuntimeFrame) (values : List RuntimeValue)
    (codec : Codec Result (Array RuntimeValue)) (encode : Result → RuntimeValue)
    (computation : Spec RuntimeState Failure Result)
    (operandsAgreement : ValuesReturn operands entry entry values)
    (agreement : Spec.Equiv (relationSpec callee values.toArray) (encodeSpec codec computation))
    (preserves : StatePreserving computation)
    (packed : ∀ result, packResults (codec.encode result) = encode result) :
    Scalar (nativeCall handle none callee operands) entry encode computation := by
  unfold ValuesReturn at operandsAgreement
  have evaluated : ∀ initial finalFrame finalState control,
      nativeCall handle none callee operands entry initial finalFrame finalState control ↔
        ∃ calleeState outcome, callee initial values.toArray calleeState outcome ∧
          finalFrame = callFrame none outcome (applyPendingFrom initial.pending entry calleeState).1 ∧
          finalState = (applyPendingFrom initial.pending entry calleeState).2 ∧
          control = callControl outcome := by
    intro initial finalFrame finalState control
    simp [nativeCall, Denotation.call, operandsAgreement]
    constructor
    · rintro ⟨_, _, _, ⟨rfl, rfl, rfl⟩, invoked⟩; exact invoked
    · intro invoked; exact ⟨entry, initial, values, ⟨rfl, rfl, rfl⟩, invoked⟩
  constructor
  · intro initial finalFrame finalState value
    constructor
    · intro executed
      obtain ⟨calleeState, outcome, invoked, rfl, rfl, controlled⟩ :=
        (evaluated _ _ _ _).mp executed
      cases outcome with
      | threw kind thrown => cases controlled
      | returned results =>
        obtain ⟨result, returned, rfl⟩ := (agreement.ok _ _ _).mp invoked
        have same := preserves _ _ _ returned
        subst calleeState
        simp only [callControl, packed, Control.value.injEq] at controlled
        exact ⟨result, by simpa only [applyPendingFrom_none rfl] using returned,
          by simp [applyPendingFrom_none rfl, callFrame_returned, registerReturnedLoan_none], controlled⟩
    · rintro ⟨result, returned, rfl, rfl⟩
      have same := preserves _ _ _ returned
      subst finalState
      apply (evaluated _ _ _ _).mpr
      exact ⟨initial, .returned (codec.encode result),
        (agreement.ok _ _ _).mpr ⟨result, returned, rfl⟩,
        by simp [applyPendingFrom_none rfl, callFrame_returned, registerReturnedLoan_none],
        by simp [applyPendingFrom_none rfl], by simp [callControl, packed]⟩
  · intro initial error
    constructor
    · rintro ⟨finalFrame, finalState, executed⟩
      obtain ⟨calleeState, outcome, invoked, _, _, controlled⟩ := (evaluated _ _ _ _).mp executed
      cases outcome with
      | returned results => cases controlled
      | threw kind thrown =>
        cases error with | mk actualKind actualThrown =>
          cases controlled
          exact (agreement.aborts _ _).mp ⟨calleeState, invoked⟩
    · intro aborted
      obtain ⟨calleeState, invoked⟩ := (agreement.aborts _ _).mpr aborted
      exact ⟨_, _, (evaluated _ _ _ _).mpr
        ⟨calleeState, .threw error.1 error.2, invoked, rfl, rfl, rfl⟩⟩
  · intro initial finalFrame finalState control executed abrupt
    obtain ⟨calleeState, outcome, _, _, _, rfl⟩ := (evaluated _ _ _ _).mp executed
    cases outcome with
    | returned results => cases abrupt
    | threw kind thrown => exact ⟨kind, thrown, rfl⟩
  · intro initial undefined
    exact (agreement.undefined initial).mpr undefined

/-- Compose a scalar initializer with a local continuation without inspecting
the initializer's body. Only successful execution retains state; abort-state
existentials are preserved rather than assumed equal to the initial state. -/
theorem fromFrame_let_scalar (unit : Validation.ExecutableUnit) (shape : FunctionShape)
    (binder : NativePatternBinder) (initializer body : ExprDenotation)
    (entry : RuntimeFrame) (encode : Local → RuntimeValue) (bound : Local → RuntimeFrame)
    (first : Spec RuntimeState Failure Local) (next : Local → Spec RuntimeState Failure Result)
    (codec : Codec Result (Array RuntimeValue))
    (scalar : Scalar initializer entry encode first) (preserves : StatePreserving first)
    (binds : ∀ value, binder.bind entry (encode value) = some (bound value))
    (continuation : ∀ value, Spec.Equiv (fromFrame unit shape body (bound value))
      (encodeSpec codec (next value))) :
    Spec.Equiv (fromFrame unit shape (letNativeValue binder initializer body) entry)
      (encodeSpec codec (Spec.bind first next)) := by
  constructor
  · intro initial result final
    constructor
    · rintro ⟨finalFrame, state, control, executed, ended, exported⟩
      rcases executed with ⟨initialized, abrupt⟩ | ⟨initFrame, initState, runtimeValue, boundFrame,
          initialized, binding, executed⟩
      · obtain ⟨kind, values, rfl⟩ := scalar.abrupt _ _ _ _ initialized abrupt
        cases ended
      · obtain ⟨value, firstStep, rfl, rfl⟩ := (scalar.normal _ _ _ _).mp initialized
        rw [binds value] at binding
        cases binding
        have same := preserves _ _ _ firstStep
        subst initState
        obtain ⟨native, nextStep, encoded⟩ := (continuation value).ok _ _ _ |>.mp
          ⟨finalFrame, state, control, executed, ended, exported⟩
        exact ⟨native, ⟨value, initial, firstStep, nextStep⟩, encoded⟩
    · rintro ⟨native, ⟨value, middle, firstStep, nextStep⟩, encoded⟩
      have same := preserves _ _ _ firstStep
      subst middle
      obtain ⟨finalFrame, state, control, executed, ended, exported⟩ :=
        (continuation value).ok _ _ _ |>.mpr ⟨native, nextStep, encoded⟩
      exact ⟨finalFrame, state, control, Or.inr ⟨entry, initial, encode value, bound value,
        (scalar.normal _ _ _ _).mpr ⟨value, firstStep, rfl, rfl⟩, binds value, executed⟩,
        ended, exported⟩
  · intro initial error
    constructor
    · rintro ⟨finalFrame, state, control, executed, ended⟩
      rcases executed with ⟨initialized, abrupt⟩ | ⟨initFrame, initState, runtimeValue, boundFrame,
          initialized, binding, executed⟩
      · obtain ⟨kind, values, rfl⟩ := scalar.abrupt _ _ _ _ initialized abrupt
        cases error with | mk actualKind actualValues =>
          cases ended
          exact Or.inl ((scalar.aborts _ _).mp ⟨finalFrame, state, initialized⟩)
      · obtain ⟨value, firstStep, rfl, rfl⟩ := (scalar.normal _ _ _ _).mp initialized
        rw [binds value] at binding
        cases binding
        have same := preserves _ _ _ firstStep
        subst initState
        exact Or.inr ⟨value, initial, firstStep,
          (continuation value).aborts _ _ |>.mp ⟨finalFrame, state, control, executed, ended⟩⟩
    · rintro (aborted | ⟨value, middle, firstStep, aborted⟩)
      · obtain ⟨finalFrame, state, executed⟩ := (scalar.aborts _ _).mpr aborted
        exact ⟨finalFrame, state, .throw_ error.1 error.2, Or.inl ⟨executed, .throw_ _ _⟩, rfl⟩
      · have same := preserves _ _ _ firstStep
        subst middle
        obtain ⟨finalFrame, state, control, executed, ended⟩ :=
          (continuation value).aborts _ _ |>.mpr aborted
        exact ⟨finalFrame, state, control, Or.inr ⟨entry, initial, encode value, bound value,
          (scalar.normal _ _ _ _).mpr ⟨value, firstStep, rfl, rfl⟩, binds value, executed⟩, ended⟩
  · intro initial
    constructor
    · exact False.elim
    · rintro (undefined | ⟨value, middle, _, undefined⟩)
      · exact scalar.defined initial undefined
      · exact (continuation value).undefined middle |>.mpr undefined

end LeanerIR.Proofs.ComputationAgreement
