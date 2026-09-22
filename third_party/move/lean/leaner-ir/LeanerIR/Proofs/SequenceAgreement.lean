-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.ArithmeticAgreement

namespace LeanerIR.Proofs.ComputationAgreement

open LeanerIR.SemanticOperations LeanerIR.BigStep Denotation

/-- Execution from an already initialized frame. This boundary-only wrapper
lets local sequencing reuse agreement rules without allocating another frame. -/
def fromFrame (unit : Validation.ExecutableUnit) (shape : FunctionShape)
    (body : ExprDenotation) (entry : RuntimeFrame) :
    Spec RuntimeState Failure (Array RuntimeValue) where
  ok initial results final := ∃ finalFrame state control,
    body entry initial finalFrame state control ∧
    finishControl? shape.resultCount control = some (.returned results) ∧
    finalizeFunctionState unit shape.profile initial state finalFrame (.returned results) = final
  aborts initial error := ∃ finalFrame state control,
    body entry initial finalFrame state control ∧
    finishControl? shape.resultCount control = some (.threw error.1 error.2)

theorem function_fromFrame (unit : Validation.ExecutableUnit) (shape : FunctionShape)
    (types : Array (TypeId × TypeId)) (arguments : Array RuntimeValue)
    (body : ExprDenotation) (entry : RuntimeFrame)
    (initialized : nativeInitialFrame? shape arguments types = some entry) :
    Spec.Equiv (nativeFunctionAt unit shape types body arguments) (fromFrame unit shape body entry) := by
  constructor
  · intro initial result final
    simp [nativeFunctionAt, fromFrame, initialized]
  · intro initial error
    simp [nativeFunctionAt, fromFrame, initialized]
  · intro initial; rfl

theorem fromFrame_congr (unit : Validation.ExecutableUnit) (shape : FunctionShape)
    {left right : ExprDenotation} {entry other : RuntimeFrame}
    (equal : ∀ initial finalFrame finalState control,
      left entry initial finalFrame finalState control ↔
      right other initial finalFrame finalState control) :
    Spec.Equiv (fromFrame unit shape left entry) (fromFrame unit shape right other) := by
  constructor
  · intro initial result final; simp only [fromFrame, equal]
  · intro initial error; simp only [fromFrame, equal]
  · intro initial; rfl

/-- A pure typed test selects a continuation without executing either branch
in the verification condition. The execution frame occurs only in agreement. -/
theorem fromFrame_branch_select (unit : Validation.ExecutableUnit) (shape : FunctionShape)
    (condition yes no : ExprDenotation) (entry : RuntimeFrame) (test : Bool)
    (evaluates : Returns condition entry entry (.bool test)) :
    Spec.Equiv (fromFrame unit shape (nativeBranch condition yes (some no)) entry)
      (fromFrame unit shape (if test then yes else no) entry) := by
  have ordinary : ∀ control, ¬(control = .value (.bool test) ∧ Abrupt control) := by
    rintro _ ⟨rfl, impossible⟩; cases impossible
  apply fromFrame_congr
  intro initial finalFrame finalState control
  unfold Returns at evaluates
  cases test <;> simp [nativeBranch, evaluates, and_assoc, ordinary]

theorem fromFrame_branch (unit : Validation.ExecutableUnit) (shape : FunctionShape)
    (condition yes no : ExprDenotation) (entry : RuntimeFrame) (test : Bool)
    (codec : Codec Result (Array RuntimeValue))
    (left right : Spec RuntimeState Failure Result)
    (evaluates : Returns condition entry entry (.bool test))
    (yesAgreement : Spec.Equiv (fromFrame unit shape yes entry) (encodeSpec codec left))
    (noAgreement : Spec.Equiv (fromFrame unit shape no entry) (encodeSpec codec right)) :
    Spec.Equiv (fromFrame unit shape (nativeBranch condition yes (some no)) entry)
      (encodeSpec codec (if test then left else right)) := by
  have ordinary : ∀ control, ¬(control = .value (.bool test) ∧ Abrupt control) := by
    rintro _ ⟨rfl, impossible⟩; cases impossible
  have selected := fromFrame_congr unit shape (entry := entry) (other := entry)
    (left := nativeBranch condition yes (some no))
    (right := if test then yes else no) (by
      intro initial finalFrame finalState control
      unfold Returns at evaluates
      cases test <;> simp [nativeBranch, evaluates, and_assoc, ordinary])
  apply selected.trans
  cases test <;> simp only [Bool.false_eq_true, ↓reduceIte] <;> assumption

theorem fromFrame_returns (unit : Validation.ExecutableUnit) (shape : FunctionShape)
    {body : ExprDenotation} {entry exit : RuntimeFrame} {value : RuntimeValue}
    (results : Array RuntimeValue) (evaluates : Returns body entry exit value)
    (finished : finishControl? shape.resultCount (.value value) = some (.returned results))
    (borrowFree : frameBorrows exit = #[]) :
    Spec.Equiv (fromFrame unit shape body entry) (Spec.pure results) := by
  unfold Returns at evaluates
  constructor
  · intro initial result final
    constructor
    · rintro ⟨finalFrame, state, control, executed, ended, exported⟩
      obtain ⟨rfl, rfl, rfl⟩ := (evaluates _ _ _ _).mp executed
      rw [finished] at ended
      cases ended
      exact ⟨rfl, by simpa only [finalizeFunctionState,
        exportReturnedFrameLoans_borrowFree _ _ _ borrowFree] using exported.symm⟩
    · rintro ⟨rfl, rfl⟩
      exact ⟨exit, _, .value value, (evaluates _ _ _ _).mpr ⟨rfl, rfl, rfl⟩,
        finished, exportReturnedFrameLoans_borrowFree _ _ _ borrowFree⟩
  · intro initial error
    constructor
    · rintro ⟨finalFrame, state, control, executed, ended⟩
      obtain ⟨rfl, rfl, rfl⟩ := (evaluates _ _ _ _).mp executed
      rw [finished] at ended
      cases ended
    · exact False.elim
  · intro initial; rfl

theorem fromFrame_throws (unit : Validation.ExecutableUnit) (shape : FunctionShape)
    {body : ExprDenotation} {entry exit : RuntimeFrame} {error : Failure}
    (evaluates : Throws body entry exit error) :
    Spec.Equiv (fromFrame unit shape body entry) (Spec.abort error) := by
  unfold Throws at evaluates
  constructor
  · intro initial result final
    constructor
    · rintro ⟨finalFrame, state, control, executed, ended, _⟩
      obtain ⟨rfl, rfl, rfl⟩ := (evaluates _ _ _ _).mp executed
      cases ended
    · exact False.elim
  · intro initial actual
    constructor
    · rintro ⟨finalFrame, state, control, executed, ended⟩
      obtain ⟨rfl, rfl, rfl⟩ := (evaluates _ _ _ _).mp executed
      cases error
      cases actual
      cases ended
      rfl
    · intro equal
      change actual = error at equal
      subst actual
      exact ⟨exit, initial, .throw_ error.1 error.2,
        (evaluates _ _ _ _).mpr ⟨rfl, rfl, rfl⟩, rfl⟩
  · intro initial; rfl

theorem fromFrame_checkedInteger (unit : Validation.ExecutableUnit) (shape : FunctionShape)
    (width : IntWidth) (signed : Bool) (kind : ThrowKind) (value : Int)
    (results : Codec (SpecInt width signed) (Array RuntimeValue))
    (operation : PrimitiveLocationOperation) (operands : ValuesDenotation)
    (entry : RuntimeFrame) (values : List RuntimeValue)
    (operandsAgreement : ValuesReturn operands entry entry values)
    (successful : IntegerValueFits width signed value → ∀ state,
      operation.evaluate? values.toArray entry state = some (.value entry state (.integer value)))
    (failing : ¬IntegerValueFits width signed value → ∀ state,
      operation.evaluate? values.toArray entry state = some (.throw_ entry state kind #[.integer value]))
    (finished : ∀ result : SpecInt width signed,
      finishControl? shape.resultCount (.value (.integer result.val)) =
        some (.returned (results.encode result)))
    (borrowFree : frameBorrows entry = #[]) :
    Spec.Equiv (fromFrame unit shape (nativePrimitiveOperation operation operands) entry)
      (encodeSpec results (NativeArithmetic.checkedInteger width signed
        (NativeArithmetic.runtimeFailure kind) value)) := by
  by_cases fits : IntegerValueFits width signed value
  · simp only [NativeArithmetic.checkedInteger, dif_pos fits]
    exact (fromFrame_returns unit shape _
      (operation_value operation _ operandsAgreement (successful fits))
      (finished ⟨value, fits⟩) borrowFree).trans
        (encodeSpec_pure results ⟨value, fits⟩).symm
  · simp only [NativeArithmetic.checkedInteger, dif_neg fits]
    exact (fromFrame_throws unit shape
      (operation_abort operation _ operandsAgreement (failing fits))).trans
        (encodeSpec_abort results _).symm

theorem let_returns (binder : NativePatternBinder) (body : ExprDenotation)
    {initializer : ExprDenotation} {entry middle bound : RuntimeFrame} {value : RuntimeValue}
    (evaluates : Returns initializer entry middle value)
    (binds : binder.bind middle value = some bound) :
    ∀ initial finalFrame finalState control,
      letNativeValue binder initializer body entry initial finalFrame finalState control ↔
      body bound initial finalFrame finalState control := by
  unfold Returns at evaluates
  intro initial finalFrame finalState control
  have ordinary : ¬Abrupt (.value value) := by intro impossible; cases impossible
  simp only [letNativeValue, evaluates, and_assoc]
  simp [binds]
  intro sameFrame sameState sameControl abrupt
  exact (ordinary (sameControl ▸ abrupt)).elim

theorem let_throws (binder : NativePatternBinder) (body : ExprDenotation)
    {initializer : ExprDenotation} {entry exit : RuntimeFrame} {error : Failure}
    (evaluates : Throws initializer entry exit error) :
    Throws (letNativeValue binder initializer body) entry exit error := by
  unfold Throws at evaluates
  intro initial finalFrame finalState control
  have abrupt : Abrupt (.throw_ error.1 error.2) := .throw_ _ _
  simp [letNativeValue, evaluates]
  rintro _ _ rfl
  exact abrupt

/-- Checked initialization enters the continuation only on success. The
native local is a range-carrying integer, not a runtime slot or universal value. -/
theorem fromFrame_let_checkedInteger (unit : Validation.ExecutableUnit) (shape : FunctionShape)
    (binder : NativePatternBinder) (initializer body : ExprDenotation)
    (entry : RuntimeFrame) (bound : SpecInt width signed → RuntimeFrame)
    (kind : ThrowKind) (value : Int)
    (codec : Codec Result (Array RuntimeValue))
    (next : SpecInt width signed → Spec RuntimeState Failure Result)
    (successful : ∀ _fits : IntegerValueFits width signed value,
      Returns initializer entry entry (.integer value))
    (failing : ¬IntegerValueFits width signed value →
      Throws initializer entry entry (NativeArithmetic.runtimeFailure kind value))
    (binds : ∀ result, binder.bind entry (.integer result.val) = some (bound result))
    (continuation : ∀ result, Spec.Equiv (fromFrame unit shape body (bound result))
      (encodeSpec codec (next result))) :
    Spec.Equiv (fromFrame unit shape (letNativeValue binder initializer body) entry)
      (encodeSpec codec (Spec.bind
        (NativeArithmetic.checkedInteger width signed (NativeArithmetic.runtimeFailure kind) value) next)) := by
  by_cases fits : IntegerValueFits width signed value
  · simp only [NativeArithmetic.checkedInteger, dif_pos fits, Spec.pure_bind]
    exact (fromFrame_congr unit shape
      (let_returns binder body (successful fits) (binds ⟨value, fits⟩))).trans
        (continuation ⟨value, fits⟩)
  · simp only [NativeArithmetic.checkedInteger, dif_neg fits, Spec.abort_bind]
    exact (fromFrame_throws unit shape (let_throws binder body (failing fits))).trans
      (encodeSpec_abort codec _).symm

end LeanerIR.Proofs.ComputationAgreement
