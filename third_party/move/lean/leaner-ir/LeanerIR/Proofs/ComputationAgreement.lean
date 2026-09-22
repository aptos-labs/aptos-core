-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Computation
import LeanerIR.Proofs.Denotation

/-!
# Execution agreement for native computations

Frame reasoning lives here, not in native contract proofs. The first laws
cover pure expressions and calls. Their side conditions are deliberately
explicit: a pure callee does not imply a pure caller if the caller still
holds a borrow that its function boundary must export.
-/

namespace LeanerIR.Proofs.ComputationAgreement

open LeanerIR.SemanticOperations LeanerIR.BigStep Denotation

/-- Exact execution of a pure expression, including its local availability
change. The store is unchanged; there are no abrupt executions. -/
def Returns (body : ExprDenotation) (entry exit : RuntimeFrame)
    (value : RuntimeValue) : Prop :=
  ∀ state finalFrame finalState control,
    body entry state finalFrame finalState control ↔
      finalFrame = exit ∧ finalState = state ∧ control = .value value

/-- Moving a represented value does not inspect its representation. In
particular this law also holds when a generic carrier encodes a borrow. -/
theorem move (frame : RuntimeFrame) (localId : LocalId) (value : RuntimeValue)
    (present : readLocal? frame localId = some value)
    (inBounds : localId.index < frame.locals.size) :
    Returns (nativeLocalOperation (.move ⟨localId⟩) valuesNil) frame
      { frame with locals := frame.locals.set! localId.index none } value := by
  intro state finalFrame finalState control
  simp only [nativeLocalOperation, nativeOperation, valuesNil,
    reduceCtorEq, exists_false, false_and, false_or, ValuesResult.values.injEq]
  simp only [and_assoc]
  simp [LocalLocationOperation.evaluate?, readRuntimePlace?, readRoot?,
    readProjections?, present, inBounds, and_assoc]
  simp only [eq_comm]

/-- Read a local without changing its availability. -/
theorem read (frame : RuntimeFrame) (localId : LocalId) (value : RuntimeValue)
    (present : readLocal? frame localId = some value) :
    Returns (localVar localId) frame frame value := by
  intro state finalFrame finalState control
  simp [localVar, present]

/-- Exact left-to-right operand evaluation for a pure expression list. -/
def ValuesReturn (body : ValuesDenotation) (entry exit : RuntimeFrame)
    (values : List RuntimeValue) : Prop :=
  ∀ state result, body entry state result ↔ result = .values state exit values

theorem nil (frame : RuntimeFrame) : ValuesReturn valuesNil frame frame [] :=
  fun _ _ => Iff.rfl

theorem cons {head : ExprDenotation} {tail : ValuesDenotation}
    {entry middle exit : RuntimeFrame} {value : RuntimeValue} {values : List RuntimeValue}
    (headAgreement : Returns head entry middle value)
    (tailAgreement : ValuesReturn tail middle exit values) :
    ValuesReturn (valuesCons head tail) entry exit (value :: values) := by
  intro state result
  unfold Returns at headAgreement
  unfold ValuesReturn at tailAgreement
  have ordinary : ¬ Abrupt (.value value) := by intro abrupt; cases abrupt
  simp [valuesCons, headAgreement, tailAgreement, and_assoc, ordinary]

/-- A call to a pure callee reuses its agreement certificate. Neither the
callee's body nor the pending-writeback fold is expanded by a caller proof. -/
theorem call {operands : ValuesDenotation}
    {entry exit : RuntimeFrame} {values : List RuntimeValue}
    {callee : Array (TypeId × TypeId) → FunctionDenotation}
    (handle : FunctionHandle) (results : Array RuntimeValue)
    (operandsAgreement : ValuesReturn operands entry exit values)
    (calleeAgreement : ∀ initial final outcome,
      callee exit.typeInstantiation initial values.toArray final outcome ↔
        outcome = .returned results ∧ final = initial) :
    Returns (nativeCallAt handle none callee operands) entry exit (packResults results) := by
  intro state finalFrame finalState control
  constructor
  · rintro (⟨operandFrame, operandState, propagated, evaluated, _, _, _⟩ |
      ⟨operandFrame, operandState, actual, calleeState, outcome,
        evaluated, invoked, framed, stored, controlled⟩)
    · have impossible := (operandsAgreement _ _).mp evaluated
      cases impossible
    · have equal := (operandsAgreement _ _).mp evaluated
      cases equal
      obtain ⟨rfl, rfl⟩ := (calleeAgreement _ _ _).mp invoked
      simp only [applyPendingFrom_none rfl, callFrame_returned,
        registerReturnedLoan_none] at framed stored
      exact ⟨framed, stored, controlled⟩
  · rintro ⟨framed, stored, controlled⟩
    subst finalFrame finalState control
    refine Or.inr ⟨exit, state, values, state, .returned results,
      (operandsAgreement _ _).mpr rfl, (calleeAgreement _ _ _).mpr ⟨rfl, rfl⟩,
      ?_, ?_, rfl⟩
    · simp only [applyPendingFrom_none rfl, callFrame_returned, registerReturnedLoan_none]
    · simp only [applyPendingFrom_none rfl]

/-- The non-generic call spelling uses the same modular agreement law. -/
theorem callMonomorphic {operands : ValuesDenotation}
    {entry exit : RuntimeFrame} {values : List RuntimeValue}
    {callee : FunctionDenotation}
    (handle : FunctionHandle) (results : Array RuntimeValue)
    (operandsAgreement : ValuesReturn operands entry exit values)
    (calleeAgreement : ∀ initial final outcome,
      callee initial values.toArray final outcome ↔
        outcome = .returned results ∧ final = initial) :
    Returns (nativeCall handle none callee operands) entry exit (packResults results) :=
  call (callee := fun _ => callee) handle results operandsAgreement calleeAgreement

/-- Close an expression's agreement at a function boundary. All frame
allocation and export conditions are discharged here, once, independently
of the function's authored contract. -/
theorem function (unit : Validation.ExecutableUnit) (shape : FunctionShape)
    (types : Array (TypeId × TypeId)) (arguments : Array RuntimeValue)
    (body : ExprDenotation) (entry exit : RuntimeFrame) (value : RuntimeValue)
    (results : Array RuntimeValue)
    (initialized : nativeInitialFrame? shape arguments types = some entry)
    (evaluates : Returns body entry exit value)
    (finished : finishControl? shape.resultCount (.value value) = some (.returned results))
    (borrowFree : frameBorrows exit = #[]) :
    Spec.Equiv (nativeFunctionAt unit shape types body arguments) (Spec.pure results) := by
  constructor
  · intro initial result final
    constructor
    · rintro ⟨frame, finalFrame, evaluated, control, entered, executed, ended, exported⟩
      rw [initialized] at entered
      cases entered
      obtain ⟨rfl, rfl, rfl⟩ := (evaluates _ _ _ _).mp executed
      rw [finished] at ended
      cases ended
      exact ⟨rfl, by simpa only [finalizeFunctionState,
        exportReturnedFrameLoans_borrowFree _ _ _ borrowFree] using exported.symm⟩
    · rintro ⟨rfl, rfl⟩
      exact ⟨_, _, _, _, initialized,
        (evaluates _ _ _ _).mpr ⟨rfl, rfl, rfl⟩, finished,
        exportReturnedFrameLoans_borrowFree _ _ _ borrowFree⟩
  · intro initial error
    constructor
    · rintro ⟨frame, finalFrame, evaluated, control, final, entered, executed, ended, _⟩
      rw [initialized] at entered
      cases entered
      obtain ⟨rfl, rfl, rfl⟩ := (evaluates _ _ _ _).mp executed
      rw [finished] at ended
      cases ended
    · exact False.elim
  · intro initial; rfl

/-- Recover the full callee relation from an exact pure agreement. Unlike
contract satisfaction, this gives both directions and the canonical result. -/
theorem relation (unit : Validation.ExecutableUnit) (shape : FunctionShape)
    (types : Array (TypeId × TypeId)) (body : ExprDenotation)
    (arguments results : Array RuntimeValue)
    (agreement : Spec.Equiv (nativeFunctionAt unit shape types body arguments)
      (Spec.pure results)) (initial final : RuntimeState) (outcome : Outcome) :
    nativeFunctionRelationAt unit shape types body initial arguments final outcome ↔
      outcome = .returned results ∧ final = initial := by
  cases outcome with
  | returned actual =>
      simpa only [Outcome.returned.injEq, nativeFunctionRelationAt,
        nativeFunctionAt, Spec.pure] using agreement.ok initial actual final
  | threw kind thrown =>
      constructor
      · rintro ⟨frame, finalFrame, evaluated, control, entered, executed, ended, exported⟩
        exact False.elim ((agreement.aborts initial (kind, thrown)).mp
          ⟨frame, finalFrame, evaluated, control, final, entered, executed, ended, exported⟩)
      · rintro ⟨impossible, _⟩; cases impossible

end LeanerIR.Proofs.ComputationAgreement
