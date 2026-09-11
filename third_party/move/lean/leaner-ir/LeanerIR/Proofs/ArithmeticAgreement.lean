-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.ComputationAgreement
import LeanerIR.Proofs.NativeArithmetic

namespace LeanerIR.Proofs.ComputationAgreement

open LeanerIR.SemanticOperations LeanerIR.BigStep Denotation

theorem literal (frame : RuntimeFrame) (value : RuntimeValue) :
    Returns (Denotation.value value) frame frame value :=
  fun _ _ _ _ => Iff.rfl

/-- A native integer-local boundary contains no references. Prove the list
fold once, rather than simplifying loan collection in each generated body. -/
theorem integerLocals_borrowFree (values : List (Option Int)) :
    frameBorrows { locals := (values.map (Option.map RuntimeValue.integer)).toArray } = #[] := by
  induction values with
  | nil => simp [frameBorrows]
  | cons value values ih =>
    cases value <;>
      simpa [frameBorrows, outermostBorrows, collectPruned, borrowEntry?] using ih

/-- Exact failure of an expression after evaluating its operands. -/
def Throws (body : ExprDenotation) (entry exit : RuntimeFrame) (error : Failure) : Prop :=
  ∀ state finalFrame finalState control,
    body entry state finalFrame finalState control ↔
      finalFrame = exit ∧ finalState = state ∧ control = .throw_ error.1 error.2

theorem function_abort (unit : Validation.ExecutableUnit) (shape : FunctionShape)
    (types : Array (TypeId × TypeId)) (arguments : Array RuntimeValue)
    (body : ExprDenotation) (entry exit : RuntimeFrame) (error : Failure)
    (initialized : nativeInitialFrame? shape arguments types = some entry)
    (evaluates : Throws body entry exit error) :
    Spec.Equiv (nativeFunctionAt unit shape types body arguments) (Spec.abort error) := by
  constructor
  · intro initial result final
    constructor
    · rintro ⟨frame, finalFrame, evaluated, control, entered, executed, ended, _⟩
      rw [initialized] at entered
      cases entered
      obtain ⟨rfl, rfl, rfl⟩ := (evaluates _ _ _ _).mp executed
      cases ended
    · exact False.elim
  · intro initial actual
    constructor
    · rintro ⟨frame, finalFrame, evaluated, control, final, entered, executed, ended, _⟩
      rw [initialized] at entered
      cases entered
      obtain ⟨rfl, rfl, rfl⟩ := (evaluates _ _ _ _).mp executed
      cases error with | mk kind thrown =>
        cases actual with | mk actualKind actualThrown =>
          cases ended
          rfl
    · intro equal
      change actual = error at equal
      subst actual
      exact ⟨_, _, _, _, _, initialized,
        (evaluates _ _ _ _).mpr ⟨rfl, rfl, rfl⟩, rfl, rfl⟩
  · intro initial; rfl

/-- Pure native evaluators share one operand-sequencing certificate, including
nominal constructors. Runtime values remain on the agreement side. -/
theorem nativeOperation_value {operands : ValuesDenotation}
    {entry exit : RuntimeFrame} {values : List RuntimeValue}
    (evaluate : NativeEvaluator) (value : RuntimeValue)
    (operandsAgreement : ValuesReturn operands entry exit values)
    (evaluates : ∀ state, evaluate values.toArray exit state =
      some (.value exit state value)) :
    Returns (nativeOperation evaluate operands) entry exit value := by
  intro state finalFrame finalState control
  simp only [nativeOperation]
  simp [ValuesReturn] at operandsAgreement
  simp [operandsAgreement, evaluates, and_assoc, eq_comm]

theorem operation_value {operands : ValuesDenotation}
    {entry exit : RuntimeFrame} {values : List RuntimeValue}
    (operation : PrimitiveLocationOperation) (value : RuntimeValue)
    (operandsAgreement : ValuesReturn operands entry exit values)
    (evaluates : ∀ state, operation.evaluate? values.toArray exit state =
      some (.value exit state value)) :
    Returns (nativePrimitiveOperation operation operands) entry exit value := by
  intro state finalFrame finalState control
  simp only [nativePrimitiveOperation, nativeOperation]
  simp [ValuesReturn] at operandsAgreement
  simp [operandsAgreement, evaluates, and_assoc, eq_comm]

theorem operation_abort {operands : ValuesDenotation}
    {entry exit : RuntimeFrame} {values : List RuntimeValue}
    (operation : PrimitiveLocationOperation) (error : Failure)
    (operandsAgreement : ValuesReturn operands entry exit values)
    (evaluates : ∀ state, operation.evaluate? values.toArray exit state =
      some (.throw_ exit state error.1 error.2)) :
    Throws (nativePrimitiveOperation operation operands) entry exit error := by
  intro state finalFrame finalState control
  simp only [nativePrimitiveOperation, nativeOperation]
  simp [ValuesReturn] at operandsAgreement
  simp [operandsAgreement, evaluates, and_assoc, eq_comm]

theorem checkedInteger_fits (width : Nat) (signed : Bool) (kind : ThrowKind)
    (value lower upper : Int)
    (bounds : (Ty.integer (.bits width) signed).integerBounds? = some (lower, upper))
    (fits : IntegerValueFits (.bits width) signed value) :
    checkedInteger kind (.integer (.bits width) signed) value = .ok (.integer value) := by
  simp [IntegerValueFits, Ty.integerValueFits?, bounds] at fits
  simp [checkedInteger, bounds, fits]

theorem checkedInteger_overflow (width : Nat) (signed : Bool) (kind : ThrowKind)
    (value lower upper : Int)
    (bounds : (Ty.integer (.bits width) signed).integerBounds? = some (lower, upper))
    (overflow : ¬IntegerValueFits (.bits width) signed value) :
    checkedInteger kind (.integer (.bits width) signed) value =
      .error (kind, #[.integer value]) := by
  simp [IntegerValueFits, Ty.integerValueFits?, bounds] at overflow
  simpa [checkedInteger, bounds] using overflow

/-- Closed descriptor equations keep generated arithmetic certificates from
unfolding the entire primitive dispatcher at every operation. -/
theorem evaluate_checkedAdd (kind : ThrowKind) (type : Ty)
    (left right : Int) (frame : RuntimeFrame) (state : RuntimeState) :
    (PrimitiveLocationOperation.checkedAdd kind type).evaluate?
      #[.integer left, .integer right] frame state =
      match checkedInteger kind type (left + right) with
      | .ok value => some (.value frame state value)
      | .error (failure, thrown) => some (.throw_ frame state failure thrown) := rfl

theorem evaluate_checkedSubtract (kind : ThrowKind) (type : Ty)
    (left right : Int) (frame : RuntimeFrame) (state : RuntimeState) :
    (PrimitiveLocationOperation.checkedSubtract kind type).evaluate?
      #[.integer left, .integer right] frame state =
      match checkedInteger kind type (left - right) with
      | .ok value => some (.value frame state value)
      | .error (failure, thrown) => some (.throw_ frame state failure thrown) := rfl

theorem evaluate_checkedAdd_fits {width : Nat} {signed : Bool} {kind : ThrowKind}
    {left right lower upper : Int} {frame : RuntimeFrame} {state : RuntimeState}
    (bounds : (Ty.integer (.bits width) signed).integerBounds? = some (lower, upper))
    (fits : IntegerValueFits (.bits width) signed (left + right)) :
    (PrimitiveLocationOperation.checkedAdd kind (.integer (.bits width) signed)).evaluate?
      #[.integer left, .integer right] frame state =
      some (.value frame state (.integer (left + right))) := by
  rw [evaluate_checkedAdd, checkedInteger_fits _ _ _ _ _ _ bounds fits]

theorem evaluate_checkedAdd_overflow {width : Nat} {signed : Bool} {kind : ThrowKind}
    {left right lower upper : Int} {frame : RuntimeFrame} {state : RuntimeState}
    (bounds : (Ty.integer (.bits width) signed).integerBounds? = some (lower, upper))
    (overflow : ¬IntegerValueFits (.bits width) signed (left + right)) :
    (PrimitiveLocationOperation.checkedAdd kind (.integer (.bits width) signed)).evaluate?
      #[.integer left, .integer right] frame state =
      some (.throw_ frame state kind #[.integer (left + right)]) := by
  rw [evaluate_checkedAdd, checkedInteger_overflow _ _ _ _ _ _ bounds overflow]

theorem evaluate_checkedSubtract_fits {width : Nat} {signed : Bool} {kind : ThrowKind}
    {left right lower upper : Int} {frame : RuntimeFrame} {state : RuntimeState}
    (bounds : (Ty.integer (.bits width) signed).integerBounds? = some (lower, upper))
    (fits : IntegerValueFits (.bits width) signed (left - right)) :
    (PrimitiveLocationOperation.checkedSubtract kind (.integer (.bits width) signed)).evaluate?
      #[.integer left, .integer right] frame state =
      some (.value frame state (.integer (left - right))) := by
  rw [evaluate_checkedSubtract, checkedInteger_fits _ _ _ _ _ _ bounds fits]

theorem evaluate_checkedSubtract_overflow {width : Nat} {signed : Bool} {kind : ThrowKind}
    {left right lower upper : Int} {frame : RuntimeFrame} {state : RuntimeState}
    (bounds : (Ty.integer (.bits width) signed).integerBounds? = some (lower, upper))
    (overflow : ¬IntegerValueFits (.bits width) signed (left - right)) :
    (PrimitiveLocationOperation.checkedSubtract kind (.integer (.bits width) signed)).evaluate?
      #[.integer left, .integer right] frame state =
      some (.throw_ frame state kind #[.integer (left - right)]) := by
  rw [evaluate_checkedSubtract, checkedInteger_overflow _ _ _ _ _ _ bounds overflow]

theorem function_checkedInteger (unit : Validation.ExecutableUnit) (shape : FunctionShape)
    (types : Array (TypeId × TypeId)) (arguments : Array RuntimeValue)
    (width : IntWidth) (signed : Bool) (kind : ThrowKind) (value : Int)
    (results : Codec (SpecInt width signed) (Array RuntimeValue))
    (operation : PrimitiveLocationOperation) (operands : ValuesDenotation)
    (entry exit : RuntimeFrame) (values : List RuntimeValue)
    (initialized : nativeInitialFrame? shape arguments types = some entry)
    (operandsAgreement : ValuesReturn operands entry exit values)
    (successful : IntegerValueFits width signed value → ∀ state,
      operation.evaluate? values.toArray exit state = some (.value exit state (.integer value)))
    (failing : ¬IntegerValueFits width signed value → ∀ state,
      operation.evaluate? values.toArray exit state =
        some (.throw_ exit state kind #[.integer value]))
    (finished : ∀ result : SpecInt width signed,
      finishControl? shape.resultCount (.value (.integer result.val)) =
        some (.returned (results.encode result)))
    (borrowFree : frameBorrows exit = #[]) :
    Spec.Equiv
      (nativeFunctionAt unit shape types (nativePrimitiveOperation operation operands) arguments)
      (encodeSpec results (NativeArithmetic.checkedInteger width signed
        (NativeArithmetic.runtimeFailure kind) value)) := by
  by_cases fits : IntegerValueFits width signed value
  · simp only [NativeArithmetic.checkedInteger, dif_pos fits]
    apply Spec.Equiv.trans ?_ (encodeSpec_pure results ⟨value, fits⟩).symm
    exact function unit shape types arguments _ entry exit (.integer value) _ initialized
      (operation_value operation _ operandsAgreement (successful fits))
      (finished ⟨value, fits⟩) borrowFree
  · simp only [NativeArithmetic.checkedInteger, dif_neg fits]
    apply Spec.Equiv.trans ?_ (encodeSpec_abort results _).symm
    exact function_abort unit shape types arguments _ entry exit _ initialized
      (operation_abort operation _ operandsAgreement (failing fits))

end LeanerIR.Proofs.ComputationAgreement
