-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.OperandAgreement

namespace LeanerIR.Proofs.ComputationAgreement
open LeanerIR.SemanticOperations LeanerIR.BigStep Denotation

/-- A pure operation over effectful typed operands. Operand evaluation order,
failures, and intermediate state are preserved exactly at this boundary. -/
theorem scalar_operation_value (evaluate : NativeEvaluator)
    (operands : ValuesDenotation) (entry : RuntimeFrame)
    (encodeArgs : Args → List RuntimeValue) (arguments : Spec RuntimeState Failure Args)
    (encodeResult : Result → RuntimeValue) (value : Args → Result)
    (operandAgreement : Operands operands entry encodeArgs arguments)
    (evaluates : ∀ args state, evaluate (encodeArgs args).toArray entry state =
      some (.value entry state (encodeResult (value args)))) :
    Scalar (nativeOperation evaluate operands) entry encodeResult
      (Spec.bind arguments (fun args => Spec.pure (value args))) := by
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
        cases evaluated
        cases controlled
        exact ⟨value args, ⟨args, finalState, argsStep, rfl, rfl⟩, rfl, rfl⟩
      · obtain ⟨_, _, _, controlled⟩ := failure
        cases controlled
    · rintro ⟨result, ⟨args, state, argsStep, rfl, sameState⟩, sameFrame, rfl⟩
      subst finalFrame
      subst state
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
  · intro initial finalFrame finalState flow executed abrupt
    rcases executed with ⟨frame, state, control, ran, _, _, rfl⟩ |
      ⟨_, _, _, _, success | failure⟩
    · exact operandAgreement.control _ _ _ _ ran
    · obtain ⟨_, _, rfl⟩ := success; cases abrupt
    · obtain ⟨kind, values, _, rfl⟩ := failure; exact ⟨kind, values, rfl⟩
  · intro initial undefined
    rcases undefined with undefined | ⟨_, _, _, impossible⟩
    · exact operandAgreement.defined _ undefined
    · exact impossible

/-- Copying an observed value adds no native work, even when its operand is
effectful. Exact execution agreement retains the source copy operation. -/
theorem scalar_copy {body : ExprDenotation} {entry : RuntimeFrame}
    {encode : Result → RuntimeValue} {computation : Spec RuntimeState Failure Result}
    (ty : Ty) (agreement : Scalar body entry encode computation) :
    Scalar (nativePrimitiveOperation (.copyValue ty) (valuesCons body valuesNil))
      entry encode computation := by
  have composed := scalar_operation_value
    (PrimitiveLocationOperation.copyValue ty).evaluate?
    (valuesCons body valuesNil) entry
    (fun pair : Result × Unit => [encode pair.1]) _ encode (fun pair => pair.1)
    (operands_cons agreement (operands_nil entry)) (by intro args state; rfl)
  simpa only [nativePrimitiveOperation, Spec.pure_bind, Spec.bind_assoc, Spec.bind_pure] using composed

/-- An immutable local borrow observes the available value without minting a
loan or changing state. Both runtime checks remain explicit premises. -/
theorem shared_local (frame : RuntimeFrame) (localId : LocalId) (value : RuntimeValue)
    (referenceType : ReferenceType) (lexical : Nat)
    (shared : referenceType.kind = .shared)
    (present : readLocal? frame localId = some value)
    (inBounds : localId.index < frame.locals.size) :
    Returns (nativeLocalOperation (.borrow ⟨localId⟩ referenceType .immutable lexical) valuesNil)
      frame frame value := by
  apply nativeOperation_value
  · exact nil frame
  · intro state
    simp [LocalLocationOperation.evaluate?, liftPlaceEvaluator, inBounds,
      borrowRuntimePlaceAt?, shared, readRuntimePlace?, readRoot?, readProjections?, present,
      show (ReferenceKind.shared != ReferenceKind.shared) = false from rfl]

/-- Reinterpret a native scalar result without changing its runtime encoding,
state, failure, or undefined behavior. Used for mathematical shift distances. -/
theorem scalar_map (body : ExprDenotation) (entry : RuntimeFrame)
    (encode : Local → RuntimeValue) (encodeResult : Result → RuntimeValue)
    (first : Spec RuntimeState Failure Local) (transform : Local → Result)
    (scalar : Scalar body entry encode first)
    (encoded : ∀ value, encodeResult (transform value) = encode value) :
    Scalar body entry encodeResult (Spec.bind first (fun value => Spec.pure (transform value))) := by
  constructor
  · intro initial frame state value
    rw [scalar.normal]
    constructor
    · rintro ⟨native, executed, frameEq, valueEq⟩
      exact ⟨transform native, ⟨native, state, executed, rfl, rfl⟩,
        frameEq, by rw [encoded]; exact valueEq⟩
    · rintro ⟨_, ⟨native, state, executed, rfl, rfl⟩, frameEq, valueEq⟩
      exact ⟨native, executed, frameEq, by rw [encoded] at valueEq; exact valueEq⟩
  · intro initial error
    simpa only [Spec.bind, Spec.pure, and_false, exists_false, or_false] using scalar.aborts initial error
  · exact scalar.abrupt
  · intro initial
    simpa only [Spec.bind, Spec.pure, and_false, exists_false, or_false] using scalar.defined initial


/-- Preserve an unconditional throw without manufacturing a normal value. -/
theorem scalar_abort {body : ExprDenotation} {entry exit : RuntimeFrame}
    (encode : Result → RuntimeValue) (error : Failure)
    (throws : Throws body entry exit error) :
    Scalar body entry encode (Spec.abort error) := by
  unfold Throws at throws
  constructor
  · intro initial finalFrame finalState value
    simp [throws, Spec.abort]
  · intro initial failure
    simp [throws, Spec.abort, Prod.ext_iff, eq_comm]
  · intro initial finalFrame finalState control executed _
    obtain ⟨_, _, rfl⟩ := (throws _ _ _ _).mp executed
    exact ⟨error.1, error.2, rfl⟩
  · intro initial; exact id

theorem scalar_checked_value (guard : Prop) [Decidable guard]
    (binder : NativePatternBinder) (check body : ExprDenotation) (entry : RuntimeFrame)
    (encode : Result → RuntimeValue) (value : guard → Result) (error : Failure)
    (success : guard → Returns check entry entry .unit)
    (failure : ¬guard → Throws check entry entry error)
    (binds : binder.bind entry .unit = some entry)
    (result : ∀ h : guard, Returns body entry entry (encode (value h))) :
    Scalar (letNativeValue binder check body) entry encode
      (if h : guard then Spec.pure (value h) else Spec.abort error) := by
  split
  next h =>
    apply scalar_pure encode (value h)
    intro initial finalFrame finalState control
    rw [let_returns binder body (success h) binds]
    exact result h _ _ _ _
  next h => exact scalar_abort encode error (let_throws binder body (failure h))

/-- Reassociate nested native initializers without reordering evaluation or
changing abrupt control. Local slots and their availability remain identical. -/
theorem let_assoc (outer inner : NativePatternBinder) (first second third : ExprDenotation)
    (entry : RuntimeFrame) (initial : RuntimeState) (finalFrame : RuntimeFrame)
    (finalState : RuntimeState) (control : Control) :
    letNativeValue outer (letNativeValue inner first second) third
      entry initial finalFrame finalState control ↔
    letNativeValue inner first (letNativeValue outer second third)
      entry initial finalFrame finalState control := by
  simp only [letNativeValue]
  constructor
  · rintro (⟨(⟨executed, abrupt⟩ | ⟨frame, state, value, bound, executed, binding, continued⟩), propagated⟩ |
      ⟨frame, state, value, bound, (⟨executed, abrupt⟩ |
        ⟨middleFrame, middleState, middleValue, middleBound, executed, firstBind, secondStep⟩), binding, continued⟩)
    · exact Or.inl ⟨executed, abrupt⟩
    · exact Or.inr ⟨frame, state, value, bound, executed, binding, Or.inl ⟨continued, propagated⟩⟩
    · cases abrupt
    · exact Or.inr ⟨middleFrame, middleState, middleValue, middleBound, executed, firstBind,
        Or.inr ⟨frame, state, value, bound, secondStep, binding, continued⟩⟩
  · rintro (⟨executed, abrupt⟩ | ⟨frame, state, value, bound, executed, binding,
      (⟨continued, propagated⟩ | ⟨lastFrame, lastState, lastValue, lastBound, secondStep, secondBind, lastStep⟩)⟩)
    · exact Or.inl ⟨Or.inl ⟨executed, abrupt⟩, abrupt⟩
    · exact Or.inl ⟨Or.inr ⟨frame, state, value, bound, executed, binding, continued⟩, propagated⟩
    · exact Or.inr ⟨lastFrame, lastState, lastValue, lastBound,
        Or.inr ⟨frame, state, value, bound, executed, binding, secondStep⟩, secondBind, lastStep⟩
end LeanerIR.Proofs.ComputationAgreement
