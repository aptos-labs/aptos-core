-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.ScalarAgreement

namespace LeanerIR.Proofs.ComputationAgreement

open LeanerIR.SemanticOperations LeanerIR.BigStep Denotation

/-- Typed operand evaluation, including failures before the enclosing
operation runs. Runtime rows occur only in this agreement boundary. -/
structure Operands (body : ValuesDenotation) (entry : RuntimeFrame)
    (encode : Args → List RuntimeValue) (computation : Spec RuntimeState Failure Args) : Prop where
  normal : ∀ initial finalFrame finalState values,
    body entry initial (.values finalState finalFrame values) ↔
      ∃ args, computation.ok initial args finalState ∧ finalFrame = entry ∧ values = encode args
  aborts : ∀ initial error,
    (∃ finalFrame finalState, body entry initial (.control finalState finalFrame (.throw_ error.1 error.2))) ↔
      computation.aborts initial error
  control : ∀ initial finalFrame finalState flow,
    body entry initial (.control finalState finalFrame flow) → ∃ kind values, flow = .throw_ kind values
  defined : ∀ initial, ¬computation.undefined initial

theorem scalar_pure {body : ExprDenotation} {entry : RuntimeFrame}
    (encode : Result → RuntimeValue) (value : Result)
    (returns : Returns body entry entry (encode value)) :
    Scalar body entry encode (Spec.pure value) := by
  unfold Returns at returns
  constructor
  · intro initial finalFrame finalState actual
    simp [returns, Spec.pure, and_assoc, and_left_comm]
  · intro initial error
    simp [returns, Spec.pure]
  · intro initial finalFrame finalState flow executed abrupt
    obtain ⟨_, _, rfl⟩ := (returns _ _ _ _).mp executed
    cases abrupt
  · intro initial; exact id

theorem operands_nil (entry : RuntimeFrame) :
    Operands valuesNil entry (fun _ : Unit => []) (Spec.pure ()) := by
  constructor
  · intro initial finalFrame finalState values
    simp [valuesNil, Spec.pure, and_left_comm]
  · intro initial error; simp [valuesNil, Spec.pure]
  · intro initial finalFrame finalState flow executed; cases executed
  · intro initial; exact id

/-- Left-to-right composition with a typed product as the native operand row. -/
theorem operands_cons {head : ExprDenotation} {tail : ValuesDenotation} {entry : RuntimeFrame}
    {headEncode : Head → RuntimeValue} {tailEncode : Tail → List RuntimeValue}
    {first : Spec RuntimeState Failure Head} {rest : Spec RuntimeState Failure Tail}
    (headAgreement : Scalar head entry headEncode first)
    (tailAgreement : Operands tail entry tailEncode rest) :
    Operands (valuesCons head tail) entry (fun pair : Head × Tail => headEncode pair.1 :: tailEncode pair.2)
      (Spec.bind first (fun h => Spec.bind rest (fun t => Spec.pure (h, t)))) := by
  constructor
  · intro initial finalFrame finalState values
    constructor
    · intro executed
      rcases executed with ⟨_, _, _, _, _, impossible⟩ |
          ⟨headFrame, headState, headValue, tailFrame, tailState, tailValues, h, t, equal⟩ |
          ⟨_, _, _, _, _, _, _, _, impossible⟩
      · cases impossible
      · cases equal
        obtain ⟨nativeHead, firstStep, rfl, rfl⟩ := (headAgreement.normal _ _ _ _).mp h
        obtain ⟨nativeTail, restStep, rfl, rfl⟩ := (tailAgreement.normal _ _ _ _).mp t
        exact ⟨(nativeHead, nativeTail), ⟨nativeHead, headState, firstStep,
          nativeTail, finalState, restStep, rfl, rfl⟩, rfl, rfl⟩
      · cases impossible
    · rintro ⟨pair, ⟨nativeHead, headState, firstStep, nativeTail, tailState, restStep,
          samePair, sameState⟩, sameFrame, encoded⟩
      subst finalFrame
      cases samePair
      subst tailState
      exact Or.inr (Or.inl ⟨entry, headState, headEncode nativeHead, entry, finalState,
        tailEncode nativeTail, (headAgreement.normal _ _ _ _).mpr ⟨nativeHead, firstStep, rfl, rfl⟩,
        (tailAgreement.normal _ _ _ _).mpr ⟨nativeTail, restStep, rfl, rfl⟩, by rw [encoded]⟩)
  · intro initial error
    constructor
    · rintro ⟨finalFrame, finalState, executed⟩
      rcases executed with ⟨frame, state, flow, h, _, equal⟩ |
          ⟨_, _, _, _, _, _, _, _, impossible⟩ |
          ⟨headFrame, headState, headValue, tailFrame, tailState, flow, h, t, equal⟩
      · cases equal
        exact Or.inl ((headAgreement.aborts _ _).mp ⟨_, _, h⟩)
      · cases impossible
      · cases equal
        obtain ⟨nativeHead, firstStep, rfl, _⟩ := (headAgreement.normal _ _ _ _).mp h
        exact Or.inr ⟨nativeHead, headState, firstStep,
          Or.inl ((tailAgreement.aborts _ _).mp ⟨_, _, t⟩)⟩
    · rintro (aborted | ⟨nativeHead, headState, firstStep, restAbort | ⟨_, _, _, impossible⟩⟩)
      · obtain ⟨frame, state, h⟩ := (headAgreement.aborts _ _).mpr aborted
        exact ⟨frame, state, Or.inl ⟨frame, state, .throw_ error.1 error.2, h, .throw_ _ _, rfl⟩⟩
      · obtain ⟨frame, state, t⟩ := (tailAgreement.aborts _ _).mpr restAbort
        exact ⟨frame, state, Or.inr (Or.inr ⟨entry, headState, headEncode nativeHead,
          frame, state, .throw_ error.1 error.2,
          (headAgreement.normal _ _ _ _).mpr ⟨nativeHead, firstStep, rfl, rfl⟩, t, rfl⟩)⟩
      · exact impossible.elim
  · intro initial finalFrame finalState flow executed
    rcases executed with ⟨frame, state, control, h, abrupt, equal⟩ |
        ⟨_, _, _, _, _, _, _, _, impossible⟩ |
        ⟨headFrame, headState, headValue, tailFrame, tailState, control, h, t, equal⟩
    · cases equal
      exact headAgreement.abrupt _ _ _ _ h abrupt
    · cases impossible
    · cases equal
      obtain ⟨_, _, rfl, _⟩ := (headAgreement.normal _ _ _ _).mp h
      exact tailAgreement.control _ _ _ _ t
  · intro initial undefined
    rcases undefined with firstUndefined | ⟨_, state, _, restUndefined | ⟨_, _, _, impossible⟩⟩
    · exact headAgreement.defined initial firstUndefined
    · exact tailAgreement.defined state restUndefined
    · exact impossible

/-- An operation consumes the successfully evaluated native operand product.
Failure of either an operand or the checked operation is preserved exactly. -/
theorem scalar_checkedOperation (operation : PrimitiveLocationOperation)
    (operands : ValuesDenotation) (entry : RuntimeFrame)
    (encode : Args → List RuntimeValue) (arguments : Spec RuntimeState Failure Args)
    (width : IntWidth) (signed : Bool) (kind : ThrowKind) (value : Args → Int)
    (operandAgreement : Operands operands entry encode arguments)
    (evaluates : ∀ args state, operation.evaluate? (encode args).toArray entry state =
      if _fits : IntegerValueFits width signed (value args) then
        some (.value entry state (.integer (value args)))
      else some (.throw_ entry state kind #[.integer (value args)])) :
    Scalar (nativePrimitiveOperation operation operands) entry (fun result => .integer result.val)
      (Spec.bind arguments (fun args => NativeArithmetic.checkedInteger width signed
        (NativeArithmetic.runtimeFailure kind) (value args))) := by
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
        split at evaluated
        next fits =>
          cases evaluated
          cases controlled
          exact ⟨⟨value args, fits⟩, ⟨args, finalState, argsStep,
            by simp [NativeArithmetic.checkedInteger, fits, Spec.pure]⟩, rfl, rfl⟩
        next overflow => cases evaluated
      · obtain ⟨_, _, _, controlled⟩ := failure
        cases controlled
    · rintro ⟨result, ⟨args, state, argsStep, checked⟩, sameFrame, rfl⟩
      subst finalFrame
      dsimp only [NativeArithmetic.checkedInteger] at checked
      split at checked
      next fits =>
        obtain ⟨rfl, sameState⟩ := checked
        subst state
        exact Or.inr ⟨entry, finalState, encode args,
          (operandAgreement.normal _ _ _ _).mpr ⟨args, argsStep, rfl, rfl⟩,
          Or.inl ⟨.integer (value args), by rw [evaluates, dif_pos fits], rfl⟩⟩
      next overflow => exact checked.elim
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
        split at evaluated
        next fits => cases evaluated
        next overflow =>
          cases evaluated
          cases error with | mk actualKind actualValues =>
            cases controlled
            exact Or.inr ⟨args, finalState, argsStep,
              by simp [NativeArithmetic.checkedInteger, overflow, Spec.abort, NativeArithmetic.runtimeFailure]⟩
    · rintro (aborted | ⟨args, state, argsStep, checked⟩)
      · obtain ⟨frame, state, ran⟩ := (operandAgreement.aborts _ _).mpr aborted
        exact ⟨frame, state, Or.inl ⟨frame, state, .throw_ error.1 error.2, ran, rfl, rfl, rfl⟩⟩
      · dsimp only [NativeArithmetic.checkedInteger] at checked
        split at checked
        next fits => exact checked.elim
        next overflow =>
          change error = NativeArithmetic.runtimeFailure kind (value args) at checked
          subst error
          exact ⟨entry, state, Or.inr ⟨entry, state, encode args,
            (operandAgreement.normal _ _ _ _).mpr ⟨args, argsStep, rfl, rfl⟩,
            Or.inr ⟨kind, #[.integer (value args)], by rw [evaluates, dif_neg overflow], rfl⟩⟩⟩
  · intro initial finalFrame finalState flow executed abrupt
    rcases executed with ⟨frame, state, control, ran, _, _, rfl⟩ |
      ⟨_, _, _, _, success | failure⟩
    · exact operandAgreement.control _ _ _ _ ran
    · obtain ⟨_, _, rfl⟩ := success; cases abrupt
    · obtain ⟨kind, values, _, rfl⟩ := failure; exact ⟨kind, values, rfl⟩
  · intro initial undefined
    rcases undefined with undefined | ⟨args, _, _, undefined⟩
    · exact operandAgreement.defined _ undefined
    · dsimp only [NativeArithmetic.checkedInteger] at undefined
      split at undefined <;> exact undefined

/-- Close a scalar expression at the execution boundary. No frame or encoded
operand row enters the native computation or its verification conditions. -/
theorem fromFrame_scalar (unit : Validation.ExecutableUnit) (shape : FunctionShape)
    (body : ExprDenotation) (entry : RuntimeFrame)
    (encode : Result → RuntimeValue) (codec : Codec Result (Array RuntimeValue))
    (computation : Spec RuntimeState Failure Result)
    (scalar : Scalar body entry encode computation)
    (finished : ∀ result, finishControl? shape.resultCount (.value (encode result)) =
      some (.returned (codec.encode result)))
    (borrowFree : frameBorrows entry = #[]) :
    Spec.Equiv (fromFrame unit shape body entry) (encodeSpec codec computation) := by
  have classify : ∀ initial finalFrame finalState control,
      body entry initial finalFrame finalState control →
      (∃ value, control = .value value) ∨ (∃ kind values, control = .throw_ kind values) := by
    intro initial finalFrame finalState control executed
    cases control with
    | value value => exact Or.inl ⟨value, rfl⟩
    | break_ nest value => exact Or.inr (scalar.abrupt _ _ _ _ executed (.break_ nest value))
    | continue_ nest => exact Or.inr (scalar.abrupt _ _ _ _ executed (.continue_ nest))
    | return_ values => exact Or.inr (scalar.abrupt _ _ _ _ executed (.return_ values))
    | throw_ kind values => exact Or.inr ⟨kind, values, rfl⟩
  constructor
  · intro initial result final
    constructor
    · rintro ⟨finalFrame, state, control, executed, ended, exported⟩
      rcases classify _ _ _ _ executed with ⟨value, rfl⟩ | ⟨kind, values, rfl⟩
      · obtain ⟨native, step, sameFrame, rfl⟩ := (scalar.normal _ _ _ _).mp executed
        subst finalFrame
        rw [finished] at ended
        cases ended
        have same : state = final := by
          simpa only [finalizeFunctionState,
            exportReturnedFrameLoans_borrowFree _ _ _ borrowFree] using exported
        subst state
        exact ⟨native, step, rfl⟩
      · cases ended
    · rintro ⟨native, step, rfl⟩
      exact ⟨entry, final, .value (encode native),
        (scalar.normal _ _ _ _).mpr ⟨native, step, rfl, rfl⟩,
        finished native, exportReturnedFrameLoans_borrowFree _ _ _ borrowFree⟩
  · intro initial error
    constructor
    · rintro ⟨finalFrame, state, control, executed, ended⟩
      rcases classify _ _ _ _ executed with ⟨value, rfl⟩ | ⟨kind, values, rfl⟩
      · obtain ⟨native, _, _, rfl⟩ := (scalar.normal _ _ _ _).mp executed
        rw [finished] at ended
        cases ended
      · cases error with | mk actualKind actualValues =>
          cases ended
          exact (scalar.aborts _ _).mp ⟨finalFrame, state, executed⟩
    · intro aborted
      obtain ⟨finalFrame, state, executed⟩ := (scalar.aborts _ _).mpr aborted
      exact ⟨finalFrame, state, .throw_ error.1 error.2, executed, rfl⟩
  · intro initial
    exact ⟨False.elim, scalar.defined initial⟩

end LeanerIR.Proofs.ComputationAgreement
