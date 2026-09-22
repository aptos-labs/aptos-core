-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.ArithmeticAgreement

namespace LeanerIR.Proofs.ComputationAgreement

open LeanerIR.SemanticOperations LeanerIR.BigStep Denotation

/-- Observe a callee's normal state and existentially hide its abort state,
exactly as the transaction-level computation does. -/
def relationSpec (callee : FunctionDenotation) (arguments : Array RuntimeValue) :
    Spec RuntimeState Failure (Array RuntimeValue) where
  ok := fun initial result final => callee initial arguments final (.returned result)
  aborts := fun initial error => ∃ final, callee initial arguments final (.threw error.1 error.2)

theorem relationSpec_native (unit : Validation.ExecutableUnit) (shape : FunctionShape)
    (types : Array (TypeId × TypeId)) (body : ExprDenotation) (arguments : Array RuntimeValue) :
    Spec.Equiv (relationSpec (nativeFunctionRelationAt unit shape types body) arguments)
      (nativeFunctionAt unit shape types body arguments) := by
  constructor
  · intro initial result final; rfl
  · intro initial error
    simp only [relationSpec, nativeFunctionRelationAt, nativeFunctionAt]
    constructor
    · rintro ⟨final, frame, finalFrame, evaluated, control, entered, executed, ended, exported⟩
      exact ⟨frame, finalFrame, evaluated, control, final, entered, executed, ended, exported⟩
    · rintro ⟨frame, finalFrame, evaluated, control, final, entered, executed, ended, exported⟩
      exact ⟨final, frame, finalFrame, evaluated, control, entered, executed, ended, exported⟩
  · intro initial; rfl

theorem function_call (unit : Validation.ExecutableUnit) (shape : FunctionShape)
    (types : Array (TypeId × TypeId)) (arguments : Array RuntimeValue)
    (handle : FunctionHandle) (callee : FunctionDenotation) (operands : ValuesDenotation)
    (entry exit : RuntimeFrame) (values : List RuntimeValue)
    (initialized : nativeInitialFrame? shape arguments types = some entry)
    (operandsAgreement : ValuesReturn operands entry exit values)
    (preserves : ∀ initial result final,
      callee initial values.toArray final (.returned result) → final = initial)
    (finished : ∀ initial result final,
      callee initial values.toArray final (.returned result) →
        finishControl? shape.resultCount (callControl (.returned result)) = some (.returned result))
    (borrowFree : frameBorrows exit = #[]) :
    Spec.Equiv (nativeFunctionAt unit shape types (nativeCall handle none callee operands) arguments)
      (relationSpec callee values.toArray) := by
  simp only [ValuesReturn] at operandsAgreement
  have evaluated : ∀ initial finalFrame finalState control,
      nativeCall handle none callee operands entry initial finalFrame finalState control ↔
        ∃ calleeState outcome, callee initial values.toArray calleeState outcome ∧
          finalFrame = callFrame none outcome (applyPendingFrom initial.pending exit calleeState).1 ∧
          finalState = (applyPendingFrom initial.pending exit calleeState).2 ∧
          control = callControl outcome := by
    intro initial finalFrame finalState control
    simp [nativeCall, Denotation.call, operandsAgreement]
    constructor
    · rintro ⟨_, _, _, ⟨rfl, rfl, rfl⟩, invoked⟩
      exact invoked
    · intro invoked
      exact ⟨exit, initial, values, ⟨rfl, rfl, rfl⟩, invoked⟩
  constructor
  · intro initial result final
    constructor
    · rintro ⟨frame, finalFrame, state, control, entered, executed, ended, exported⟩
      rw [initialized] at entered
      cases entered
      obtain ⟨calleeState, outcome, invoked, rfl, rfl, rfl⟩ := (evaluated _ _ _ _).mp executed
      cases outcome with
      | threw kind thrown => cases ended
      | returned results =>
        rw [finished _ _ _ invoked] at ended
        cases ended
        have same := preserves _ _ _ invoked
        subst calleeState
        simp only [applyPendingFrom_none rfl, callFrame_returned,
          registerReturnedLoan_none, finalizeFunctionState,
          exportReturnedFrameLoans_borrowFree _ _ _ borrowFree] at exported
        subst final
        exact invoked
    · intro invoked
      have same := preserves _ _ _ invoked
      subst final
      refine ⟨entry, exit, initial, callControl (.returned result), initialized, ?_,
        finished _ _ _ invoked, ?_⟩
      · apply (evaluated _ _ _ _).mpr
        exact ⟨initial, .returned result, invoked,
          by simp [applyPendingFrom_none rfl, callFrame_returned, registerReturnedLoan_none],
          by simp [applyPendingFrom_none rfl], rfl⟩
      · exact exportReturnedFrameLoans_borrowFree _ _ _ borrowFree
  · intro initial error
    constructor
    · rintro ⟨frame, finalFrame, state, control, final, entered, executed, ended, _⟩
      rw [initialized] at entered
      cases entered
      obtain ⟨calleeState, outcome, invoked, rfl, rfl, rfl⟩ := (evaluated _ _ _ _).mp executed
      cases outcome with
      | returned results => rw [finished _ _ _ invoked] at ended; cases ended
      | threw kind thrown =>
        cases error with | mk actualKind actualThrown =>
          cases ended
          exact ⟨calleeState, invoked⟩
    · rintro ⟨calleeState, invoked⟩
      refine ⟨entry, callFrame none (.threw error.1 error.2)
          (applyPendingFrom initial.pending exit calleeState).1,
        (applyPendingFrom initial.pending exit calleeState).2,
        .throw_ error.1 error.2, _, initialized, ?_, rfl, rfl⟩
      exact (evaluated _ _ _ _).mpr ⟨calleeState, .threw error.1 error.2, invoked, rfl, rfl, rfl⟩
  · intro initial; rfl

theorem function_call_represents (unit : Validation.ExecutableUnit) (shape : FunctionShape)
    (types : Array (TypeId × TypeId)) (arguments : Array RuntimeValue)
    (handle : FunctionHandle) (callee : FunctionDenotation) (operands : ValuesDenotation)
    (entry exit : RuntimeFrame) (values : List RuntimeValue)
    (codec : Codec Result (Array RuntimeValue)) (computation : Spec RuntimeState Failure Result)
    (initialized : nativeInitialFrame? shape arguments types = some entry)
    (operandsAgreement : ValuesReturn operands entry exit values)
    (agreement : Spec.Equiv (relationSpec callee values.toArray) (encodeSpec codec computation))
    (preserves : StatePreserving computation)
    (finished : ∀ result, finishControl? shape.resultCount
      (callControl (.returned (codec.encode result))) = some (.returned (codec.encode result)))
    (borrowFree : frameBorrows exit = #[]) :
    Spec.Equiv (nativeFunctionAt unit shape types (nativeCall handle none callee operands) arguments)
      (encodeSpec codec computation) := by
  refine (function_call unit shape types arguments handle callee operands entry exit values
    initialized operandsAgreement ?_ ?_ borrowFree).trans agreement
  · intro initial result final invoked
    obtain ⟨native, executed, _⟩ := (agreement.ok _ _ _).mp invoked
    exact preserves _ _ _ executed
  · intro initial result final invoked
    obtain ⟨native, _, rfl⟩ := (agreement.ok _ _ _).mp invoked
    exact finished native

end LeanerIR.Proofs.ComputationAgreement
