-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Semantics.BigStep
import VerexIR.Semantics.LoanRenamingFunctions
import VerexIR.Semantics.TableRoleShifts
import VerexIR.Semantics.LoanRenamingPrimitives

/-!
# Loan independence

A run mirrors from any start whose loan bookkeeping differs, with its
loans renamed injectively: the loans it mints raised by the difference of
the frontiers, and the loans it is lent mapped to the other start's
(`designs/static-typing.md`, "Loan independence"). Each relation of the
big-step semantics is simulated from a state `StateRenamed` from the
first run's, by a forward induction over the first run's derivation.
-/

namespace VerexIR

open Validation
open SemanticOperations
open BigStep

theorem BigStep.Abrupt.rename {control : Control} (abrupt : Abrupt control) (ρ : Nat → Nat) :
    Abrupt (control.rename ρ) := by
  cases abrupt <;> constructor

@[simp] theorem Control.rename_value (ρ : Nat → Nat) (value : RuntimeValue) :
    (Control.value value).rename ρ = .value (value.rename ρ) := rfl
@[simp] theorem Control.rename_break (ρ : Nat → Nat) (nest : Nat) (value : Option RuntimeValue) :
    (Control.break_ nest value).rename ρ = .break_ nest (value.map (·.rename ρ)) := rfl
@[simp] theorem Control.rename_continue (ρ : Nat → Nat) (nest : Nat) :
    (Control.continue_ nest).rename ρ = .continue_ nest := rfl
@[simp] theorem Control.rename_return (ρ : Nat → Nat) (values : Array RuntimeValue) :
    (Control.return_ values).rename ρ = .return_ (values.map (·.rename ρ)) := rfl
@[simp] theorem Control.rename_throw (ρ : Nat → Nat) (kind : ThrowKind) (arguments : Array RuntimeValue) :
    (Control.throw_ kind arguments).rename ρ = .throw_ kind (arguments.map (·.rename ρ)) :=
  rfl

/-- A pattern mismatch throws values that hold no loans. -/
theorem patternMismatchThrow_rename {profile : Option Profile} {kind : ThrowKind}
    {arguments : Array RuntimeValue}
    (mismatch_eq : patternMismatchThrow? profile = some (kind, arguments)) (ρ : Nat → Nat)
    (inside : Nat → Prop) :
    (Control.throw_ kind arguments).rename ρ = .throw_ kind arguments ∧
      (Control.throw_ kind arguments).Within inside := by
  cases profile with
  | none => simp [patternMismatchThrow?, patternMismatch?] at mismatch_eq
  | some profile =>
      cases profile <;> simp [patternMismatchThrow?, patternMismatch?] at mismatch_eq
      obtain ⟨rfl, rfl⟩ := mismatch_eq
      simp [Control.rename, Control.Within]

def ExprShifts {unit : ValidatedUnit} (executable : ExecutableUnit unit) (namespaceId : NamespaceId)
    (frame : RuntimeFrame)
    (state : RuntimeState) (id : ExprId) (frame' : RuntimeFrame) (state' : RuntimeState)
    (control : Control) : Prop :=
  ∀ (ρ : Nat → Nat) (offset frontier : Nat) (inside : Nat → Prop) (inert inert' : Nat)
    (state₂ : RuntimeState),
    StateRenamed ρ offset frontier inside inert inert' state state₂ → frame.Within inside →
    ∃ state₂', StateRenamed ρ offset frontier inside inert inert' state' state₂' ∧
      frame'.Within inside ∧ control.Within inside ∧
      EvalExprWith executable (EvalFunction executable) namespaceId (frame.rename ρ) state₂ id
        (frame'.rename ρ) state₂' (control.rename ρ)

def NodeShifts {unit : ValidatedUnit} (executable : ExecutableUnit unit) (namespaceId : NamespaceId)
    (frame : RuntimeFrame)
    (state : RuntimeState) (id : ExprId) (frame' : RuntimeFrame) (state' : RuntimeState)
    (control : Control) : Prop :=
  ∀ (ρ : Nat → Nat) (offset frontier : Nat) (inside : Nat → Prop) (inert inert' : Nat)
    (state₂ : RuntimeState),
    StateRenamed ρ offset frontier inside inert inert' state state₂ → frame.Within inside →
    ∃ state₂', StateRenamed ρ offset frontier inside inert inert' state' state₂' ∧
      frame'.Within inside ∧ control.Within inside ∧
      EvalNodeWith executable (EvalFunction executable) namespaceId (frame.rename ρ) state₂ id
        (frame'.rename ρ) state₂' (control.rename ρ)

def ValuesShift {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (namespaceId : NamespaceId) (frame : RuntimeFrame)
    (state : RuntimeState) (ids : List ExprId) (result : ValuesResult) : Prop :=
  ∀ (ρ : Nat → Nat) (offset frontier : Nat) (inside : Nat → Prop) (inert inert' : Nat)
    (state₂ : RuntimeState),
    StateRenamed ρ offset frontier inside inert inert' state state₂ → frame.Within inside →
    match result with
    | .values state' frame' values => ∃ state₂',
        StateRenamed ρ offset frontier inside inert inert' state' state₂' ∧ frame'.Within inside ∧
        (∀ value ∈ values, value.Within inside) ∧
        EvalValuesWith executable (EvalFunction executable) namespaceId (frame.rename ρ) state₂ ids
          (.values state₂' (frame'.rename ρ) (values.map (·.rename ρ)))
    | .control state' frame' control => ∃ state₂',
        StateRenamed ρ offset frontier inside inert inert' state' state₂' ∧ frame'.Within inside ∧
        control.Within inside ∧
        EvalValuesWith executable (EvalFunction executable) namespaceId (frame.rename ρ) state₂ ids
          (.control state₂' (frame'.rename ρ) (control.rename ρ))

def StatementsShift {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (namespaceId : NamespaceId) (frame : RuntimeFrame)
    (state : RuntimeState) (statements : List ExprId) (result : StatementsResult) : Prop :=
  ∀ (ρ : Nat → Nat) (offset frontier : Nat) (inside : Nat → Prop) (inert inert' : Nat)
    (state₂ : RuntimeState),
    StateRenamed ρ offset frontier inside inert inert' state state₂ → frame.Within inside →
    match result with
    | .done state' frame' => ∃ state₂',
        StateRenamed ρ offset frontier inside inert inert' state' state₂' ∧ frame'.Within inside ∧
        EvalStatementsWith executable (EvalFunction executable) namespaceId (frame.rename ρ) state₂
          statements (.done state₂' (frame'.rename ρ))
    | .control state' frame' control => ∃ state₂',
        StateRenamed ρ offset frontier inside inert inert' state' state₂' ∧ frame'.Within inside ∧
        control.Within inside ∧
        EvalStatementsWith executable (EvalFunction executable) namespaceId (frame.rename ρ) state₂
          statements (.control state₂' (frame'.rename ρ) (control.rename ρ))

def ArmsShift {unit : ValidatedUnit} (executable : ExecutableUnit unit) (namespaceId : NamespaceId)
    (ns : ValidatedNamespace)
    (frame : RuntimeFrame) (state : RuntimeState) (value : RuntimeValue) (arms : List MatchArm)
    (frame' : RuntimeFrame) (state' : RuntimeState) (control : Control) : Prop :=
  ∀ (ρ : Nat → Nat) (offset frontier : Nat) (inside : Nat → Prop) (inert inert' : Nat)
    (state₂ : RuntimeState),
    StateRenamed ρ offset frontier inside inert inert' state state₂ → frame.Within inside →
    value.Within inside →
    ∃ state₂', StateRenamed ρ offset frontier inside inert inert' state' state₂' ∧
      frame'.Within inside ∧ control.Within inside ∧
      EvalArmsWith executable (EvalFunction executable) namespaceId ns (frame.rename ρ) state₂
        (value.rename ρ) arms (frame'.rename ρ) state₂' (control.rename ρ)

def FunctionShifts {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (handle : FunctionHandle)
    (instantiation : Array (TypeId × TypeId)) (state : RuntimeState)
    (arguments : Array RuntimeValue) (state' : RuntimeState) (outcome : Outcome) : Prop :=
  ∀ (ρ : Nat → Nat) (offset frontier : Nat) (inside : Nat → Prop) (inert inert' : Nat)
    (state₂ : RuntimeState),
    StateRenamed ρ offset frontier inside inert inert' state state₂ →
    (∀ argument ∈ arguments, argument.Within inside) →
    ∃ state₂', StateRenamed ρ offset frontier inside inert inert' state' state₂' ∧
      outcome.Within inside ∧
      EvalFunction executable handle instantiation state₂ (arguments.map (·.rename ρ)) state₂'
        (outcome.rename ρ)

theorem NodeShifts.ofOperandsControl {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId} {finalFrame : RuntimeFrame}
    {finalState : RuntimeState} {control : Control} {arguments : List ExprId}
    (ih : ValuesShift executable namespaceId frame state arguments (.control finalState finalFrame control))
    (build : ∀ {frame₂ : RuntimeFrame} {state₂ : RuntimeState} {finalFrame₂ : RuntimeFrame}
      {finalState₂ : RuntimeState} {control₂ : Control},
      EvalValuesWith executable (EvalFunction executable) namespaceId frame₂ state₂ arguments
        (.control finalState₂ finalFrame₂ control₂) →
      EvalNodeWith executable (EvalFunction executable) namespaceId frame₂ state₂ exprId finalFrame₂
        finalState₂ control₂) :
    NodeShifts executable namespaceId frame state exprId finalFrame finalState control := by
  intro ρ offset frontier inside inert inert' state₂ shifted frame_within
  obtain ⟨finalState₂, finalShifted, final_within, control_within, operands₂⟩ :=
    ih ρ offset frontier inside inert inert' state₂ shifted frame_within
  exact ⟨finalState₂, finalShifted, final_within, control_within, build operands₂⟩

/-- The host's natives mirror their runs: the host provides them, and
`hostCall` is opaque. The table roles, which the runtime provides, mirror
theirs (`tableRoleCall_shifts`). -/
def NativesShift {unit : ValidatedUnit} (executable : ExecutableUnit unit) : Prop :=
  ∀ handle instantiation state arguments state' outcome,
    hostCall executable handle instantiation state arguments = some (state', outcome) →
    ∀ (ρ : Nat → Nat) (offset frontier : Nat) (inside : Nat → Prop) (inert inert' : Nat)
    (state₂ : RuntimeState),
      StateRenamed ρ offset frontier inside inert inert' state state₂ →
      (∀ argument ∈ arguments, argument.Within inside) →
      ∃ state₂', StateRenamed ρ offset frontier inside inert inert' state' state₂' ∧
        outcome.Within inside ∧
        hostCall executable handle instantiation state₂ (arguments.map (·.rename ρ)) =
          some (state₂', outcome.rename ρ)

theorem shifts {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    (natives : NativesShift executable) {handle : FunctionHandle}
    {instantiation : Array (TypeId × TypeId)} {state : RuntimeState}
    {arguments : Array RuntimeValue} {state' : RuntimeState} {outcome : Outcome}
    (step : EvalFunction executable handle instantiation state arguments state' outcome) :
    FunctionShifts executable handle instantiation state arguments state' outcome := by
  apply EvalFunction.rec
    (motive_1 := fun handle instantiation state arguments state' outcome _ =>
      FunctionShifts executable handle instantiation state arguments state' outcome)
    (motive_2 := fun namespaceId frame state id frame' state' control _ =>
      ExprShifts executable namespaceId frame state id frame' state' control)
    (motive_3 := fun namespaceId frame state id frame' state' control _ =>
      NodeShifts executable namespaceId frame state id frame' state' control)
    (motive_4 := fun namespaceId frame state ids result _ =>
      ValuesShift executable namespaceId frame state ids result)
    (motive_5 := fun namespaceId frame state statements result _ =>
      StatementsShift executable namespaceId frame state statements result)
    (motive_6 := fun namespaceId ns frame state value arms frame' state' control _ =>
      ArmsShift executable namespaceId ns frame state value arms frame' state' control)
    (t := step)
  case node =>
    intro namespaceId frame state exprId startFrame startState nodeFrame nodeState control
      finalFrame finalState before_eq node_step after_eq ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    have settled := settleLoans_rename (loans := (loanDeathsAt unit namespaceId exprId).before)
      shifted frame_within
    rw [before_eq] at settled
    generalize start_eq : settleLoans (loanDeathsAt unit namespaceId exprId).before
      (frame.rename ρ) state₂ = start₂ at settled
    obtain ⟨startFrame₂, startState₂⟩ := start₂
    obtain ⟨rfl, startShifted, start_within⟩ := settled
    obtain ⟨nodeState₂, nodeShifted, node_within, control_within, node_step₂⟩ :=
      ih ρ offset frontier inside inert inert' startState₂ startShifted start_within
    have after := settleAfter_rename (loans := (loanDeathsAt unit namespaceId exprId).after)
      (control := control) nodeShifted node_within
    rw [after_eq] at after
    generalize final_eq : settleAfter (loanDeathsAt unit namespaceId exprId).after
      (control.rename ρ) (nodeFrame.rename ρ) nodeState₂ = final₂ at after
    obtain ⟨finalFrame₂, finalState₂⟩ := final₂
    obtain ⟨rfl, finalShifted, final_within⟩ := after
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .node _ _ _ _ _ _ _ _ _ _ _ start_eq node_step₂ final_eq⟩
  case value =>
    intro namespaceId frame state exprId ns expression literal source runtimeValue
      namespace_eq expression_eq kind_eq value_eq
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    have plain := constValue?_plain _ literal (Nat.le_refl _) value_eq
    refine ⟨state₂, shifted, frame_within, RuntimeValue.within_of_plain inside plain, ?_⟩
    rw [Control.rename_value, RuntimeValue.rename_of_plain ρ plain]
    exact .value _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq value_eq
  case localVar =>
    intro namespaceId frame state exprId ns expression localId runtimeValue
      namespace_eq expression_eq kind_eq local_eq
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    refine ⟨state₂, shifted, frame_within, ?_, ?_⟩
    · unfold readLocal? at local_eq
      cases slot_eq : frame.locals[localId.index]? with
      | none => simp [slot_eq] at local_eq
      | some slot =>
          rw [slot_eq] at local_eq
          cases slot with
          | none => cases local_eq
          | some value =>
              cases local_eq
              exact frame_within.slot slot_eq
    · exact .localVar _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
        (by rw [readLocal?_rename, local_eq]; rfl)
  case spec =>
    intro namespaceId frame state exprId ns expression block namespace_eq expression_eq kind_eq
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    refine ⟨state₂, shifted, frame_within, by simp [Control.Within], ?_⟩
    rw [Control.rename_value, RuntimeValue.rename_unit]
    exact .spec _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
  case continue_ =>
    intro namespaceId frame state exprId ns expression nest namespace_eq expression_eq kind_eq
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    exact ⟨state₂, shifted, frame_within, trivial,
      .continue_ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq⟩
  case breakNone =>
    intro namespaceId frame state exprId ns expression nest namespace_eq expression_eq kind_eq
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    exact ⟨state₂, shifted, frame_within, by simp [Control.Within],
      .breakNone _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq⟩
  case breakControl =>
    intro namespaceId frame state exprId ns expression nest child finalFrame finalState control
      namespace_eq expression_eq kind_eq child_step abrupt ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, child_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .breakControl _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq child_step₂
        (abrupt.rename ρ)⟩
  case breakValue =>
    intro namespaceId frame state exprId ns expression nest child finalFrame finalState
      runtimeValue namespace_eq expression_eq kind_eq child_step ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, child_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    exact ⟨finalState₂, finalShifted, final_within, by simpa [Control.Within] using control_within,
      .breakValue _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq child_step₂⟩
  case returnValues =>
    intro namespaceId frame state exprId ns expression values finalFrame finalState
      runtimeValues namespace_eq expression_eq kind_eq values_step ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, values_within, values_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    refine ⟨finalState₂, finalShifted, final_within, by simpa [Control.Within] using values_within,
      ?_⟩
    rw [Control.rename_return, List.map_toArray]
    exact .returnValues _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq values_step₂
  case returnControl =>
    intro namespaceId frame state exprId ns expression values finalFrame finalState control
      namespace_eq expression_eq kind_eq values_step ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, values_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .returnControl _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq values_step₂⟩
  case throwValues =>
    intro namespaceId frame state exprId ns expression kind arguments finalFrame finalState
      runtimeValues namespace_eq expression_eq kind_eq values_step ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, values_within, values_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    refine ⟨finalState₂, finalShifted, final_within, by simpa [Control.Within] using values_within,
      ?_⟩
    rw [Control.rename_throw, List.map_toArray]
    exact .throwValues _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq values_step₂
  case throwControl =>
    intro namespaceId frame state exprId ns expression kind arguments finalFrame finalState control
      namespace_eq expression_eq kind_eq values_step ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, values_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .throwControl _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq values_step₂⟩
  case nil =>
    intro namespaceId frame state
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    exact ⟨state₂, shifted, frame_within, by simp, .nil _ _ _⟩
  case headControl =>
    intro namespaceId frame state expression expressions finalFrame finalState control
      head_step abrupt ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, head_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .headControl _ _ _ _ _ _ _ _ head_step₂ (abrupt.rename ρ)⟩
  case tailValues =>
    intro namespaceId frame state expression expressions headFrame headState value finalFrame
      finalState values head_step tail_step ih_head ih_tail
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨headState₂, headShifted, head_within, value_within, head_step₂⟩ :=
      ih_head ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, values_within, tail_step₂⟩ :=
      ih_tail ρ offset frontier inside inert inert' headState₂ headShifted head_within
    refine ⟨finalState₂, finalShifted, final_within, ?_, ?_⟩
    · intro element member
      rcases List.mem_cons.mp member with rfl | member
      · exact value_within
      · exact values_within element member
    · rw [List.map_cons]
      exact .tailValues _ _ _ _ _ _ _ _ _ _ _ head_step₂ tail_step₂
  case tailControl =>
    intro namespaceId frame state expression expressions headFrame headState value finalFrame
      finalState control head_step tail_step ih_head ih_tail
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨headState₂, headShifted, head_within, -, head_step₂⟩ :=
      ih_head ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, tail_step₂⟩ :=
      ih_tail ρ offset frontier inside inert inert' headState₂ headShifted head_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .tailControl _ _ _ _ _ _ _ _ _ _ _ head_step₂ tail_step₂⟩
  case nil =>
    intro namespaceId frame state
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    exact ⟨state₂, shifted, frame_within, .nil _ _ _⟩
  case headControl =>
    intro namespaceId frame state statement statements finalFrame finalState control
      head_step abrupt ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, head_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .headControl _ _ _ _ _ _ _ _ head_step₂ (abrupt.rename ρ)⟩
  case cons =>
    intro namespaceId frame state statement statements headFrame headState value result
      head_step tail_step ih_head ih_tail
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨headState₂, headShifted, head_within, -, head_step₂⟩ :=
      ih_head ρ offset frontier inside inert inert' state₂ shifted frame_within
    have tail := ih_tail ρ offset frontier inside inert inert' headState₂ headShifted head_within
    cases result with
    | done finalState finalFrame =>
        obtain ⟨finalState₂, finalShifted, final_within, tail_step₂⟩ := tail
        exact ⟨finalState₂, finalShifted, final_within, .cons _ _ _ _ _ _ _ _ _ head_step₂ tail_step₂⟩
    | control finalState finalFrame control =>
        obtain ⟨finalState₂, finalShifted, final_within, control_within, tail_step₂⟩ := tail
        exact ⟨finalState₂, finalShifted, final_within, control_within,
          .cons _ _ _ _ _ _ _ _ _ head_step₂ tail_step₂⟩
  case blockControl =>
    intro namespaceId frame state exprId ns expression statements result finalState finalFrame
      control namespace_eq expression_eq kind_eq steps ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, steps₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .blockControl _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq steps₂⟩
  case blockUnit =>
    intro namespaceId frame state exprId ns expression statements finalState finalFrame
      namespace_eq expression_eq kind_eq steps ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, steps₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    refine ⟨finalState₂, finalShifted, final_within, by simp [Control.Within], ?_⟩
    rw [Control.rename_value, RuntimeValue.rename_unit]
    exact .blockUnit _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq steps₂
  case blockResult =>
    intro namespaceId frame state exprId ns expression statements result statementState
      statementFrame finalState finalFrame control namespace_eq expression_eq kind_eq steps
      result_step ih_steps ih_result
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨statementState₂, statementShifted, statement_within, steps₂⟩ :=
      ih_steps ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, result_step₂⟩ :=
      ih_result ρ offset frontier inside inert inert' statementState₂ statementShifted statement_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .blockResult _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq steps₂
        result_step₂⟩
  case letNoValue =>
    intro namespaceId frame state exprId ns expression pattern body finalFrame finalState control
      namespace_eq expression_eq kind_eq body_step ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, body_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .letNoValue _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq body_step₂⟩
  case letValueControl =>
    intro namespaceId frame state exprId ns expression pattern initializer body finalFrame
      finalState control namespace_eq expression_eq kind_eq initializer_step abrupt ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, initializer_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .letValueControl _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
        initializer_step₂ (abrupt.rename ρ)⟩
  case ifControl =>
    intro namespaceId frame state exprId ns expression condition thenBranch elseBranch finalFrame
      finalState control namespace_eq expression_eq kind_eq condition_step abrupt ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, condition_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .ifControl _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq condition_step₂
        (abrupt.rename ρ)⟩
  case ifTrue =>
    intro namespaceId frame state exprId ns expression condition thenBranch elseBranch
      conditionFrame conditionState finalFrame finalState control namespace_eq expression_eq
      kind_eq condition_step branch_step ih_condition ih_branch
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨conditionState₂, conditionShifted, condition_within, -, condition_step₂⟩ :=
      ih_condition ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, branch_step₂⟩ :=
      ih_branch ρ offset frontier inside inert inert' conditionState₂ conditionShifted condition_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .ifTrue (namespace_eq := namespace_eq) (expression_eq := expression_eq) (kind_eq := kind_eq)
        (condition_step := by simpa using condition_step₂) (branch_step := branch_step₂)⟩
  case ifFalseUnit =>
    intro namespaceId frame state exprId ns expression condition thenBranch conditionFrame
      conditionState namespace_eq expression_eq kind_eq condition_step ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨conditionState₂, conditionShifted, condition_within, -, condition_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    refine ⟨conditionState₂, conditionShifted, condition_within,
      by simp [Control.Within], ?_⟩
    rw [Control.rename_value, RuntimeValue.rename_unit]
    exact .ifFalseUnit _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
      (by simpa using condition_step₂)
  case ifFalse =>
    intro namespaceId frame state exprId ns expression condition thenBranch elseBranch
      conditionFrame conditionState finalFrame finalState control namespace_eq expression_eq
      kind_eq condition_step branch_step ih_condition ih_branch
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨conditionState₂, conditionShifted, condition_within, -, condition_step₂⟩ :=
      ih_condition ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, branch_step₂⟩ :=
      ih_branch ρ offset frontier inside inert inert' conditionState₂ conditionShifted condition_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .ifFalse (namespace_eq := namespace_eq) (expression_eq := expression_eq) (kind_eq := kind_eq)
        (condition_step := by simpa using condition_step₂) (branch_step := branch_step₂)⟩
  case matchControl =>
    intro namespaceId frame state exprId ns expression scrutinee arms finalFrame finalState control
      namespace_eq expression_eq kind_eq scrutinee_step abrupt ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, scrutinee_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .matchControl _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq scrutinee_step₂
        (abrupt.rename ρ)⟩
  case matchValue =>
    intro namespaceId frame state exprId ns expression scrutinee arms scrutineeFrame
      scrutineeState runtimeValue finalFrame finalState control namespace_eq expression_eq kind_eq
      scrutinee_step arm_step ih_scrutinee ih_arms
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨scrutineeState₂, scrutineeShifted, scrutinee_within, value_within, scrutinee_step₂⟩ :=
      ih_scrutinee ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, arm_step₂⟩ :=
      ih_arms ρ offset frontier inside inert inert' scrutineeState₂ scrutineeShifted scrutinee_within
        value_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .matchValue _ _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
        scrutinee_step₂ arm_step₂⟩
  case assignControl =>
    intro namespaceId frame state exprId ns expression place child finalFrame finalState control
      namespace_eq expression_eq kind_eq child_step abrupt ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, child_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .assignControl _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq child_step₂
        (abrupt.rename ρ)⟩
  case assignPatternControl =>
    intro namespaceId frame state exprId ns expression pattern child finalFrame finalState control
      namespace_eq expression_eq kind_eq child_step abrupt ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, child_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .assignPatternControl _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq child_step₂
        (abrupt.rename ρ)⟩
  case loopRepeatValue =>
    intro namespaceId frame state exprId ns expression label body bodyFrame bodyState
      runtimeValue finalFrame finalState control namespace_eq expression_eq kind_eq body_step
      repeat_step ih_body ih_repeat
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨bodyState₂, bodyShifted, body_within, -, body_step₂⟩ :=
      ih_body ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, repeat_step₂⟩ :=
      ih_repeat ρ offset frontier inside inert inert' bodyState₂ bodyShifted body_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .loopRepeatValue (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (body_step := body_step₂) (repeat_step := repeat_step₂)⟩
  case loopRepeatContinue =>
    intro namespaceId frame state exprId ns expression label body bodyFrame bodyState finalFrame
      finalState control namespace_eq expression_eq kind_eq body_step repeat_step ih_body ih_repeat
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨bodyState₂, bodyShifted, body_within, -, body_step₂⟩ :=
      ih_body ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, repeat_step₂⟩ :=
      ih_repeat ρ offset frontier inside inert inert' bodyState₂ bodyShifted body_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .loopRepeatContinue (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (body_step := body_step₂) (repeat_step := repeat_step₂)⟩
  case loopBreak =>
    intro namespaceId frame state exprId ns expression label body finalFrame finalState value
      namespace_eq expression_eq kind_eq body_step ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, body_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    refine ⟨finalState₂, finalShifted, final_within, ?_, ?_⟩
    · cases value with
      | none => simp [Control.Within]
      | some value => simpa [Control.Within] using control_within
    · have same : (Control.value (value.getD .unit)).rename ρ =
          .value ((value.map (·.rename ρ)).getD .unit) := by
        cases value <;> simp
      rw [same]
      exact .loopBreak (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (body_step := body_step₂)
  case loopOuterBreak =>
    intro namespaceId frame state exprId ns expression label body finalFrame finalState nest value
      namespace_eq expression_eq kind_eq body_step ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, body_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .loopOuterBreak (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (body_step := body_step₂)⟩
  case loopOuterContinue =>
    intro namespaceId frame state exprId ns expression label body finalFrame finalState nest
      namespace_eq expression_eq kind_eq body_step ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, body_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .loopOuterContinue (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (body_step := body_step₂)⟩
  case loopReturn =>
    intro namespaceId frame state exprId ns expression label body finalFrame finalState values
      namespace_eq expression_eq kind_eq body_step ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, body_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .loopReturn (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (body_step := body_step₂)⟩
  case loopThrow =>
    intro namespaceId frame state exprId ns expression label body finalFrame finalState kind
      arguments namespace_eq expression_eq kind_eq body_step ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, body_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .loopThrow (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (body_step := body_step₂)⟩
  case letValue =>
    intro namespaceId frame state exprId ns expression pattern initializer body initializedFrame
      initializedState runtimeValue boundFrame finalFrame finalState control namespace_eq
      expression_eq kind_eq initializer_step bind_eq body_step ih_initializer ih_body
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨β, γ, base⟩ := shifted.renames.base
    obtain ⟨initializedState₂, initializedShifted, initialized_within, value_within,
      initializer_step₂⟩ := ih_initializer ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, body_step₂⟩ :=
      ih_body ρ offset frontier inside inert inert' initializedState₂ initializedShifted
        (bindPattern_within base initialized_within value_within bind_eq)
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .letValue (namespace_eq := namespace_eq) (expression_eq := expression_eq) (kind_eq := kind_eq)
        (initializer_step := initializer_step₂)
        (bind_eq := by rw [bindPattern_rename shifted.renames.injective, bind_eq]; rfl) (body_step := body_step₂)⟩
  case letMismatch =>
    intro namespaceId frame state exprId ns expression pattern initializer body initializedFrame
      initializedState runtimeValue kind arguments namespace_eq expression_eq kind_eq
      initializer_step bind_eq mismatch_eq ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨initializedState₂, initializedShifted, initialized_within, -, initializer_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨shift_eq, above⟩ := patternMismatchThrow_rename mismatch_eq ρ inside
    refine ⟨initializedState₂, initializedShifted, initialized_within, above, ?_⟩
    rw [shift_eq]
    exact .letMismatch (namespace_eq := namespace_eq) (expression_eq := expression_eq)
      (kind_eq := kind_eq) (initializer_step := initializer_step₂)
      (bind_eq := by rw [bindPattern_rename shifted.renames.injective, bind_eq]; rfl) (mismatch_eq := mismatch_eq)
  case assignPatternMismatch =>
    intro namespaceId frame state exprId ns expression pattern child childFrame finalState
      runtimeValue kind arguments namespace_eq expression_eq kind_eq child_step bind_eq
      mismatch_eq ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, child_within, -, child_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨shift_eq, above⟩ := patternMismatchThrow_rename mismatch_eq ρ inside
    refine ⟨finalState₂, finalShifted, child_within, above, ?_⟩
    rw [shift_eq]
    exact .assignPatternMismatch (namespace_eq := namespace_eq) (expression_eq := expression_eq)
      (kind_eq := kind_eq) (child_step := child_step₂)
      (bind_eq := by rw [bindPattern_rename shifted.renames.injective, bind_eq]; rfl) (mismatch_eq := mismatch_eq)
  case assignPatternValue =>
    intro namespaceId frame state exprId ns expression pattern child childFrame finalFrame
      finalState runtimeValue namespace_eq expression_eq kind_eq child_step bind_eq ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨β, γ, base⟩ := shifted.renames.base
    obtain ⟨finalState₂, finalShifted, child_within, value_within, child_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    refine ⟨finalState₂, finalShifted, bindPattern_within base child_within value_within bind_eq,
      by simp [Control.Within], ?_⟩
    rw [Control.rename_value, RuntimeValue.rename_unit]
    exact .assignPatternValue (namespace_eq := namespace_eq) (expression_eq := expression_eq)
      (kind_eq := kind_eq) (child_step := child_step₂)
      (bind_eq := by rw [bindPattern_rename shifted.renames.injective, bind_eq]; rfl)
  case exhausted =>
    intro namespaceId ns frame state value kind arguments mismatch_eq
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within value_within
    obtain ⟨shift_eq, above⟩ := patternMismatchThrow_rename mismatch_eq ρ inside
    refine ⟨state₂, shifted, frame_within, above, ?_⟩
    rw [shift_eq]
    exact .exhausted _ _ _ _ _ _ _ mismatch_eq
  case reject =>
    intro namespaceId ns frame state value arm arms finalFrame finalState control bind_eq tail_step
      ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within value_within
    obtain ⟨finalState₂, finalShifted, final_within, control_within, tail_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within value_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .reject (arms := arms) (bind_eq := by rw [bindPattern_rename shifted.renames.injective, bind_eq]; rfl) (tail_step := tail_step₂)⟩
  case noGuard =>
    intro namespaceId ns frame state value arm arms armFrame finalFrame finalState control
      guard_eq bind_eq body_step ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within value_within
    obtain ⟨β, γ, base⟩ := shifted.renames.base
    obtain ⟨finalState₂, finalShifted, final_within, control_within, body_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted (bindPattern_within base frame_within value_within bind_eq)
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .noGuard (arms := arms) (guard_eq := guard_eq) (bind_eq := by rw [bindPattern_rename shifted.renames.injective, bind_eq]; rfl)
        (body_step := body_step₂)⟩
  case guardControl =>
    intro namespaceId ns frame state value arm arms guard armFrame finalFrame finalState control
      guard_eq bind_eq guard_step abrupt ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within value_within
    obtain ⟨β, γ, base⟩ := shifted.renames.base
    obtain ⟨finalState₂, finalShifted, final_within, control_within, guard_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted (bindPattern_within base frame_within value_within bind_eq)
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .guardControl (arms := arms) (guard_eq := guard_eq) (bind_eq := by rw [bindPattern_rename shifted.renames.injective, bind_eq]; rfl)
        (guard_step := guard_step₂) (abrupt := abrupt.rename ρ)⟩
  case guardTrue =>
    intro namespaceId ns frame state value arm arms guard armFrame guardFrame guardState
      finalFrame finalState control guard_eq bind_eq guard_step body_step ih_guard ih_body
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within value_within
    obtain ⟨β, γ, base⟩ := shifted.renames.base
    obtain ⟨guardState₂, guardShifted, guard_within, -, guard_step₂⟩ :=
      ih_guard ρ offset frontier inside inert inert' state₂ shifted
        (bindPattern_within base frame_within value_within bind_eq)
    obtain ⟨finalState₂, finalShifted, final_within, control_within, body_step₂⟩ :=
      ih_body ρ offset frontier inside inert inert' guardState₂ guardShifted guard_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .guardTrue (arms := arms) (guard_eq := guard_eq) (bind_eq := by rw [bindPattern_rename shifted.renames.injective, bind_eq]; rfl)
        (guard_step := by simpa using guard_step₂) (body_step := body_step₂)⟩
  case guardFalse =>
    intro namespaceId ns frame state value arm arms guard armFrame guardFrame guardState
      finalFrame finalState control guard_eq bind_eq guard_step tail_step ih_guard ih_tail
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within value_within
    obtain ⟨β, γ, base⟩ := shifted.renames.base
    obtain ⟨guardState₂, guardShifted, -, -, guard_step₂⟩ :=
      ih_guard ρ offset frontier inside inert inert' state₂ shifted
        (bindPattern_within base frame_within value_within bind_eq)
    obtain ⟨finalState₂, finalShifted, final_within, control_within, tail_step₂⟩ :=
      ih_tail ρ offset frontier inside inert inert' state₂ shifted frame_within value_within
    exact ⟨finalState₂, finalShifted, final_within, control_within,
      .guardFalse (arms := arms) (guard_eq := guard_eq) (bind_eq := by rw [bindPattern_rename shifted.renames.injective, bind_eq]; rfl)
        (guard_step := by simpa using guard_step₂) (tail_step := tail_step₂)⟩
  case assignValue =>
    intro namespaceId frame state exprId ns expression place child childFrame childState resolved
      finalFrame finalState runtimeValue namespace_eq expression_eq kind_eq child_step resolve_eq
      write_eq ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨childState₂, childShifted, child_within, value_within, child_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, write₂, finalShifted, final_within⟩ :=
      writeRuntimePlace?_mirror childShifted child_within value_within write_eq
    refine ⟨finalState₂, finalShifted, final_within, by simp [Control.Within], ?_⟩
    rw [Control.rename_value, RuntimeValue.rename_unit]
    exact .assignValue (namespace_eq := namespace_eq) (expression_eq := expression_eq)
      (kind_eq := kind_eq) (child_step := child_step₂)
      (resolve_eq := by rw [resolvePlace?_rename unit ns childShifted.globals]; exact resolve_eq)
      (write_eq := write₂)
  case assignMismatch =>
    intro namespaceId frame state exprId ns expression place child childFrame childState
      runtimeValue kind thrown namespace_eq expression_eq kind_eq child_step resolve_eq mismatch
      mismatch_eq ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨childState₂, childShifted, child_within, -, child_step₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨shift_eq, above⟩ := patternMismatchThrow_rename mismatch_eq ρ inside
    refine ⟨childState₂, childShifted, child_within, above, ?_⟩
    rw [shift_eq]
    exact .assignMismatch (namespace_eq := namespace_eq) (expression_eq := expression_eq)
      (kind_eq := kind_eq) (child_step := child_step₂)
      (resolve_eq := by rw [resolvePlace?_rename unit ns childShifted.globals]; exact resolve_eq)
      (mismatch := by
        rw [placeVariantMismatchFuel?_rename unit ns childShifted.globals]; exact mismatch)
      (mismatch_eq := mismatch_eq)
  case constantValue =>
    intro namespaceId frame state exprId ns expression reference handle targetNs declaration
      targetFrame finalState runtimeValue namespace_eq expression_eq kind_eq resolve_eq
      target_namespace_eq declaration_eq initializer ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, -, value_within, initializer₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted (RuntimeFrame.empty_within inside)
    rw [RuntimeFrame.rename_empty] at initializer₂
    exact ⟨finalState₂, finalShifted, frame_within, value_within,
      .constantValue (frame := frame.rename ρ) (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (resolve_eq := resolve_eq) (target_namespace_eq := target_namespace_eq)
        (declaration_eq := declaration_eq) (initializer := initializer₂)⟩
  case constantControl =>
    intro namespaceId frame state exprId ns expression reference handle targetNs declaration
      targetFrame finalState control namespace_eq expression_eq kind_eq resolve_eq
      target_namespace_eq declaration_eq initializer abrupt ih
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, -, control_within, initializer₂⟩ :=
      ih ρ offset frontier inside inert inert' state₂ shifted (RuntimeFrame.empty_within inside)
    rw [RuntimeFrame.rename_empty] at initializer₂
    exact ⟨finalState₂, finalShifted, frame_within, control_within,
      .constantControl (frame := frame.rename ρ) (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (resolve_eq := resolve_eq) (target_namespace_eq := target_namespace_eq)
        (declaration_eq := declaration_eq) (initializer := initializer₂)
        (abrupt := abrupt.rename ρ)⟩
  case callArgumentsControl =>
    intros
    rename_i namespace_eq expression_eq kind_eq operands ih
    exact NodeShifts.ofOperandsControl ih fun operands₂ =>
      .callArgumentsControl (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (operands := operands₂)
  case constructorArgumentsControl =>
    intros
    rename_i namespace_eq expression_eq kind_eq operands ih
    exact NodeShifts.ofOperandsControl ih fun operands₂ =>
      .constructorArgumentsControl (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (operands := operands₂)
  case destructorArgumentsControl =>
    intros
    rename_i namespace_eq expression_eq kind_eq operands ih
    exact NodeShifts.ofOperandsControl ih fun operands₂ =>
      .destructorArgumentsControl (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (operands := operands₂)
  case closureArgumentsControl =>
    intros
    rename_i namespace_eq expression_eq kind_eq operands ih
    exact NodeShifts.ofOperandsControl ih fun operands₂ =>
      .closureArgumentsControl (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (operands := operands₂)
  case invokeArgumentsControl =>
    intros
    rename_i namespace_eq expression_eq kind_eq operands ih
    exact NodeShifts.ofOperandsControl ih fun operands₂ =>
      .invokeArgumentsControl (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (operands := operands₂)
  case profileArgumentsControl =>
    intros
    rename_i namespace_eq expression_eq kind_eq operands ih
    exact NodeShifts.ofOperandsControl ih fun operands₂ =>
      .profileArgumentsControl (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (operands := operands₂)
  case primitiveArgumentsControl =>
    intros
    rename_i namespace_eq expression_eq kind_eq operands ih
    exact NodeShifts.ofOperandsControl ih fun operands₂ =>
      .primitiveArgumentsControl (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (operands := operands₂)
  case globalArgumentsControl =>
    intros
    rename_i namespace_eq expression_eq kind_eq operands ih
    exact NodeShifts.ofOperandsControl ih fun operands₂ =>
      .globalArgumentsControl (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (operands := operands₂)
  case assertArgumentsControl =>
    intros
    rename_i namespace_eq expression_eq kind_eq operands ih
    exact NodeShifts.ofOperandsControl ih fun operands₂ =>
      .assertArgumentsControl (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (operands := operands₂)
  case operationArgumentsControl =>
    intros
    rename_i namespace_eq expression_eq kind_eq operands ih
    exact NodeShifts.ofOperandsControl ih fun operands₂ =>
      .operationArgumentsControl (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (operands := operands₂)
  case callReturned =>
    intro namespaceId frame state exprId ns expression reference instantiations arguments surface
      argumentState argumentFrame values handle finalState results namespace_eq expression_eq
      kind_eq operands resolve_eq calleeStep ih_operands ih_callee
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨β, γ, base⟩ := shifted.renames.base
    obtain ⟨argumentState₂, argumentShifted, argument_within, values_within, operands₂⟩ :=
      ih_operands ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, outcome_within, callee₂⟩ :=
      ih_callee ρ offset frontier inside inert inert' argumentState₂ argumentShifted
        (fun value member => values_within value (by simpa using member))
    have pending := applyPendingFrom_mirror argument_within argumentShifted finalShifted
    generalize pending_eq : applyPendingFrom argumentState₂.pending (argumentFrame.rename ρ)
      finalState₂ = pending₂ at pending
    obtain ⟨pendingFrame₂, pendingState₂⟩ := pending₂
    obtain ⟨rfl, pendingShifted, pending_within⟩ := pending
    have results_within : ∀ value ∈ results, value.Within inside := outcome_within
    refine ⟨pendingState₂, pendingShifted, registerReturnedLoan_within base pending_within results_within,
      packResults_within base results_within, ?_⟩
    rw [← registerReturnedLoan_rename, Control.rename_value, ← packResults_rename]
    have step := EvalNodeWith.callReturned (executable := executable) (callee := EvalFunction executable)
      (frame := frame.rename ρ) (state := state₂) (exprId := exprId)
      (namespace_eq := namespace_eq) (expression_eq := expression_eq) (kind_eq := kind_eq)
      (operands := operands₂) (resolve_eq := resolve_eq)
      (calleeStep := by simpa [List.map_toArray] using callee₂)
    rw [pending_eq] at step
    exact step
  case callThrew =>
    intro namespaceId frame state exprId ns expression reference instantiations arguments surface
      argumentState argumentFrame values handle finalState kind thrown namespace_eq expression_eq
      kind_eq operands resolve_eq calleeStep ih_operands ih_callee
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨argumentState₂, argumentShifted, argument_within, values_within, operands₂⟩ :=
      ih_operands ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, outcome_within, callee₂⟩ :=
      ih_callee ρ offset frontier inside inert inert' argumentState₂ argumentShifted
        (fun value member => values_within value (by simpa using member))
    have pending := applyPendingFrom_mirror argument_within argumentShifted finalShifted
    generalize pending_eq : applyPendingFrom argumentState₂.pending (argumentFrame.rename ρ)
      finalState₂ = pending₂ at pending
    obtain ⟨pendingFrame₂, pendingState₂⟩ := pending₂
    obtain ⟨rfl, pendingShifted, pending_within⟩ := pending
    refine ⟨pendingState₂, pendingShifted, pending_within, outcome_within, ?_⟩
    have step := EvalNodeWith.callThrew (executable := executable) (callee := EvalFunction executable)
      (frame := frame.rename ρ) (state := state₂) (exprId := exprId)
      (namespace_eq := namespace_eq) (expression_eq := expression_eq) (kind_eq := kind_eq)
      (operands := operands₂) (resolve_eq := resolve_eq)
      (calleeStep := by simpa [List.map_toArray] using callee₂)
    rw [pending_eq] at step
    exact step
  case constructorValue =>
    intro namespaceId frame state exprId ns expression reference variant instantiations arguments
      surface finalState finalFrame values runtimeValue namespace_eq expression_eq kind_eq operands
      construct_eq ih_operands
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨β, γ, base⟩ := shifted.renames.base
    obtain ⟨finalState₂, finalShifted, final_within, values_within, operands₂⟩ :=
      ih_operands ρ offset frontier inside inert inert' state₂ shifted frame_within
    refine ⟨finalState₂, finalShifted, final_within,
      constructNominal?_within base (fun value member => values_within value (by simpa using member))
        construct_eq, ?_⟩
    have construct₂ := constructNominal?_rename (ρ := ρ) unit namespaceId reference
      variant values.toArray
    rw [construct_eq, Option.map_some] at construct₂
    exact .constructorValue _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
      operands₂ (by simpa [List.map_toArray] using construct₂)
  case destructorValue =>
    intro namespaceId frame state exprId ns expression reference variant instantiations arguments
      surface finalState finalFrame value fields namespace_eq expression_eq kind_eq operands
      destruct_eq ih_operands
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨β, γ, base⟩ := shifted.renames.base
    obtain ⟨finalState₂, finalShifted, final_within, values_within, operands₂⟩ :=
      ih_operands ρ offset frontier inside inert inert' state₂ shifted frame_within
    have fields_within := destructNominal?_within base (values_within value List.mem_cons_self) destruct_eq
    refine ⟨finalState₂, finalShifted, final_within, packResults_within base fields_within, ?_⟩
    have destruct₂ := destructNominal?_rename (ρ := ρ) unit namespaceId reference
      variant value
    rw [destruct_eq, Option.map_some] at destruct₂
    rw [Control.rename_value, ← packResults_rename]
    exact .destructorValue _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
      operands₂ destruct₂
  case closureValue =>
    intro namespaceId frame state exprId ns expression reference mask instantiations captures
      surface finalState finalFrame values handle namespace_eq expression_eq kind_eq operands
      resolve_eq ih_operands
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, values_within, operands₂⟩ :=
      ih_operands ρ offset frontier inside inert inert' state₂ shifted frame_within
    refine ⟨finalState₂, finalShifted, final_within, ?_, ?_⟩
    · simpa [Control.Within] using values_within
    · have step := EvalNodeWith.closureValue (executable := executable) (callee := EvalFunction executable)
        (frame := frame.rename ρ) (state := state₂) (exprId := exprId)
        (namespace_eq := namespace_eq) (expression_eq := expression_eq) (kind_eq := kind_eq)
        (operands := operands₂) (resolve_eq := resolve_eq)
      simpa [List.map_toArray, RuntimeFrame.rename] using step
  case invokeReturned =>
    intro namespaceId frame state exprId ns expression instantiations arguments surface
      argumentState argumentFrame handle mask typeInstantiation captures values composed finalState
      results namespace_eq expression_eq kind_eq operands compose_eq calleeStep ih_operands
      ih_callee
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨β, γ, base⟩ := shifted.renames.base
    obtain ⟨argumentState₂, argumentShifted, argument_within, values_within, operands₂⟩ :=
      ih_operands ρ offset frontier inside inert inert' state₂ shifted frame_within
    have closure_within := values_within _ List.mem_cons_self
    rw [RuntimeValue.within_closure] at closure_within
    have composed_within : ∀ value ∈ composed.toArray, value.Within inside := by
      intro value member
      rcases ClosureMask.compose_mem compose_eq value (by simpa using member) with member | member
      · exact closure_within value (by simpa using member)
      · exact values_within value (List.mem_cons_of_mem _ member)
    obtain ⟨finalState₂, finalShifted, outcome_within, callee₂⟩ :=
      ih_callee ρ offset frontier inside inert inert' argumentState₂ argumentShifted composed_within
    have pending := applyPendingFrom_mirror argument_within argumentShifted finalShifted
    generalize pending_eq : applyPendingFrom argumentState₂.pending (argumentFrame.rename ρ)
      finalState₂ = pending₂ at pending
    obtain ⟨pendingFrame₂, pendingState₂⟩ := pending₂
    obtain ⟨rfl, pendingShifted, pending_within⟩ := pending
    have results_within : ∀ value ∈ results, value.Within inside := outcome_within
    refine ⟨pendingState₂, pendingShifted, registerReturnedLoan_within base pending_within results_within,
      packResults_within base results_within, ?_⟩
    rw [← registerReturnedLoan_rename, Control.rename_value, ← packResults_rename]
    have compose₂ := ClosureMask.compose_map (·.rename ρ) mask captures.toList values
    rw [compose_eq, Option.map_some] at compose₂
    have step := EvalNodeWith.invokeReturned (executable := executable) (callee := EvalFunction executable)
      (frame := frame.rename ρ) (state := state₂) (exprId := exprId)
      (namespace_eq := namespace_eq) (expression_eq := expression_eq) (kind_eq := kind_eq)
      (operands := by simpa using operands₂)
      (compose_eq := by simpa using compose₂)
      (calleeStep := by simpa [List.map_toArray] using callee₂)
    rw [pending_eq] at step
    exact step
  case invokeThrew =>
    intro namespaceId frame state exprId ns expression instantiations arguments surface
      argumentState argumentFrame handle mask typeInstantiation captures values composed finalState
      kind thrown namespace_eq expression_eq kind_eq operands compose_eq calleeStep ih_operands
      ih_callee
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨argumentState₂, argumentShifted, argument_within, values_within, operands₂⟩ :=
      ih_operands ρ offset frontier inside inert inert' state₂ shifted frame_within
    have closure_within := values_within _ List.mem_cons_self
    rw [RuntimeValue.within_closure] at closure_within
    have composed_within : ∀ value ∈ composed.toArray, value.Within inside := by
      intro value member
      rcases ClosureMask.compose_mem compose_eq value (by simpa using member) with member | member
      · exact closure_within value (by simpa using member)
      · exact values_within value (List.mem_cons_of_mem _ member)
    obtain ⟨finalState₂, finalShifted, outcome_within, callee₂⟩ :=
      ih_callee ρ offset frontier inside inert inert' argumentState₂ argumentShifted composed_within
    have pending := applyPendingFrom_mirror argument_within argumentShifted finalShifted
    generalize pending_eq : applyPendingFrom argumentState₂.pending (argumentFrame.rename ρ)
      finalState₂ = pending₂ at pending
    obtain ⟨pendingFrame₂, pendingState₂⟩ := pending₂
    obtain ⟨rfl, pendingShifted, pending_within⟩ := pending
    refine ⟨pendingState₂, pendingShifted, pending_within, outcome_within, ?_⟩
    have compose₂ := ClosureMask.compose_map (·.rename ρ) mask captures.toList values
    rw [compose_eq, Option.map_some] at compose₂
    have step := EvalNodeWith.invokeThrew (executable := executable) (callee := EvalFunction executable)
      (frame := frame.rename ρ) (state := state₂) (exprId := exprId)
      (namespace_eq := namespace_eq) (expression_eq := expression_eq) (kind_eq := kind_eq)
      (operands := by simpa using operands₂)
      (compose_eq := by simpa using compose₂)
      (calleeStep := by simpa [List.map_toArray] using callee₂)
    rw [pending_eq] at step
    exact step
  case profileValue =>
    intro namespaceId frame state exprId ns expression operation targets instantiations arguments
      surface finalState finalFrame values runtimeValue namespace_eq expression_eq kind_eq operands
      evaluate_eq ih_operands
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, -, operands₂⟩ :=
      ih_operands ρ offset frontier inside inert inert' state₂ shifted frame_within
    have plain := (evaluateProfileOperation?_plain evaluate_eq).1 runtimeValue rfl
    refine ⟨finalState₂, finalShifted, final_within, RuntimeValue.within_of_plain inside plain, ?_⟩
    rw [Control.rename_value, RuntimeValue.rename_of_plain ρ plain]
    exact .profileValue _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq operands₂
      (by rw [← List.map_toArray, evaluateProfileOperation?_rename]; exact evaluate_eq)
  case profileThrow =>
    intro namespaceId frame state exprId ns expression operation targets instantiations arguments
      surface finalState finalFrame values kind thrown namespace_eq expression_eq kind_eq operands
      evaluate_eq ih_operands
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, -, operands₂⟩ :=
      ih_operands ρ offset frontier inside inert inert' state₂ shifted frame_within
    have plain := (evaluateProfileOperation?_plain evaluate_eq).2 kind thrown rfl
    refine ⟨finalState₂, finalShifted, final_within,
      fun value member => RuntimeValue.within_of_plain inside (plain value member), ?_⟩
    have thrown_eq : thrown.map (·.rename ρ) = thrown := by
      conv => rhs; rw [← Array.map_id thrown]
      exact Array.map_congr_left (fun value member => RuntimeValue.rename_of_plain ρ
        (plain value member))
    rw [Control.rename_throw, thrown_eq]
    exact .profileThrow _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
      operands₂ (by rw [← List.map_toArray, evaluateProfileOperation?_rename]; exact evaluate_eq)
  case assertTrue =>
    intro namespaceId frame state exprId ns expression instantiations arguments surface finalState
      finalFrame namespace_eq expression_eq kind_eq operands ih_operands
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, -, operands₂⟩ :=
      ih_operands ρ offset frontier inside inert inert' state₂ shifted frame_within
    refine ⟨finalState₂, finalShifted, final_within, by simp [Control.Within], ?_⟩
    rw [Control.rename_value, RuntimeValue.rename_unit]
    exact .assertTrue _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
      (by simpa using operands₂)
  case assertFalse =>
    intro namespaceId frame state exprId ns expression instantiations arguments surface finalState
      finalFrame namespace_eq expression_eq kind_eq operands ih_operands
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, finalShifted, final_within, -, operands₂⟩ :=
      ih_operands ρ offset frontier inside inert inert' state₂ shifted frame_within
    refine ⟨finalState₂, finalShifted, final_within, fun _ member => by simp at member, ?_⟩
    rw [Control.rename_throw, Array.map_empty]
    exact .assertFalse _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
      (by simpa using operands₂)
  case primitiveValue =>
    intro namespaceId frame state exprId ns expression operation instantiations arguments surface
      finalState finalFrame values runtimeValue namespace_eq expression_eq kind_eq operands
      evaluate_eq ih_operands
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨β, γ, base⟩ := shifted.renames.base
    obtain ⟨finalState₂, finalShifted, final_within, values_within, operands₂⟩ :=
      ih_operands ρ offset frontier inside inert inert' state₂ shifted frame_within
    refine ⟨finalState₂, finalShifted, final_within,
      (evaluatePrimitiveOperation?_within base (fun value member => values_within value (by simpa using member))
        evaluate_eq).1 runtimeValue rfl, ?_⟩
    have evaluate₂ := evaluatePrimitiveOperation?_rename ρ ns expression.typeId operation
      values.toArray executable.targetPointerWidth
    rw [evaluate_eq, Option.map_some, renameResult_ok] at evaluate₂
    exact .primitiveValue _ _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq operands₂
      (by rw [← List.map_toArray]; exact evaluate₂)
  case primitiveThrow =>
    intro namespaceId frame state exprId ns expression operation instantiations arguments surface
      finalState finalFrame values kind thrown namespace_eq expression_eq kind_eq operands
      evaluate_eq ih_operands
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨β, γ, base⟩ := shifted.renames.base
    obtain ⟨finalState₂, finalShifted, final_within, values_within, operands₂⟩ :=
      ih_operands ρ offset frontier inside inert inert' state₂ shifted frame_within
    refine ⟨finalState₂, finalShifted, final_within,
      (evaluatePrimitiveOperation?_within base (fun value member => values_within value (by simpa using member))
        evaluate_eq).2 kind thrown rfl, ?_⟩
    have evaluate₂ := evaluatePrimitiveOperation?_rename ρ ns expression.typeId operation
      values.toArray executable.targetPointerWidth
    rw [evaluate_eq, Option.map_some, renameResult_error] at evaluate₂
    rw [Control.rename_throw]
    exact .primitiveThrow _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq operands₂
      (by rw [← List.map_toArray]; exact evaluate₂)
  case globalValue =>
    intro namespaceId frame state exprId ns expression kind instantiations arguments surface
      argumentState argumentFrame values finalFrame finalState runtimeValue namespace_eq
      expression_eq kind_eq operands evaluate_eq ih_operands
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨argumentState₂, argumentShifted, argument_within, values_within, operands₂⟩ :=
      ih_operands ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨result₂, evaluate₂, mirrors⟩ := evaluateGlobalOperation?_mirror argumentShifted
      argument_within (fun value member => values_within value (by simpa using member)) evaluate_eq
    cases result₂ with
    | value finalFrame₂ finalState₂ runtimeValue₂ =>
        obtain ⟨⟨rfl, finalShifted, final_within⟩, rfl, value_within⟩ := mirrors
        refine ⟨finalState₂, finalShifted, final_within, value_within, ?_⟩
        exact .globalValue _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
          operands₂ (by rw [← List.map_toArray]; exact evaluate₂)
    | throw_ => cases mirrors
  case globalThrow =>
    intro namespaceId frame state exprId ns expression globalKind instantiations arguments surface
      argumentState argumentFrame values finalFrame finalState throwKind thrown namespace_eq
      expression_eq kind_eq operands evaluate_eq ih_operands
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨argumentState₂, argumentShifted, argument_within, values_within, operands₂⟩ :=
      ih_operands ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨result₂, evaluate₂, mirrors⟩ := evaluateGlobalOperation?_mirror argumentShifted
      argument_within (fun value member => values_within value (by simpa using member)) evaluate_eq
    cases result₂ with
    | value => cases mirrors
    | throw_ finalFrame₂ finalState₂ throwKind₂ thrown₂ =>
        obtain ⟨⟨rfl, finalShifted, final_within⟩, rfl, rfl, thrown_within⟩ := mirrors
        refine ⟨finalState₂, finalShifted, final_within, thrown_within, ?_⟩
        exact .globalThrow _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
          operands₂ (by rw [← List.map_toArray]; exact evaluate₂)
  case operationValue =>
    intro namespaceId frame state exprId ns expression operation instantiations arguments surface
      argumentState argumentFrame values finalFrame finalState runtimeValue namespace_eq
      expression_eq kind_eq operands evaluate_eq ih_operands
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨argumentState₂, argumentShifted, argument_within, values_within, operands₂⟩ :=
      ih_operands ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨finalState₂, evaluate₂, finalShifted, final_within, value_within⟩ :=
      evaluatePlaceOperation?_mirror argumentShifted argument_within
        (fun value member => values_within value (by simpa using member)) evaluate_eq
    refine ⟨finalState₂, finalShifted, final_within, value_within, ?_⟩
    exact .operationValue _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
      operands₂ (by rw [← List.map_toArray]; exact evaluate₂)
  case operationMismatch =>
    intro namespaceId frame state exprId ns expression operation instantiations arguments surface
      argumentState argumentFrame values kind thrown namespace_eq expression_eq kind_eq operands
      evaluate_eq mismatch mismatch_eq ih_operands
    intro ρ offset frontier inside inert inert' state₂ shifted frame_within
    obtain ⟨argumentState₂, argumentShifted, argument_within, -, operands₂⟩ :=
      ih_operands ρ offset frontier inside inert inert' state₂ shifted frame_within
    have mismatch₂ : variantMismatch? unit ns operation
        ((values.map (·.rename ρ)).toArray) (argumentFrame.rename ρ) argumentState₂ =
          true := by
      rw [← List.map_toArray, variantMismatch?_rename unit ns operation _ argumentShifted.globals]
      exact mismatch
    obtain ⟨shift_eq, above⟩ := patternMismatchThrow_rename mismatch_eq ρ inside
    refine ⟨argumentState₂, argumentShifted, argument_within, above, ?_⟩
    rw [shift_eq]
    exact .operationMismatch _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
      operands₂ (variantMismatch?_evaluate_none mismatch₂) mismatch₂ mismatch_eq
  case body =>
    intro handle typeInstantiation initialState arguments ns declaration frame root finalFrame
      evaluatedState finalState control outcome namespace_eq declaration_eq provided_eq frame_eq
      body_eq body_step outcome_eq hole_free finalize_eq ih_body
    intro ρ offset frontier inside inert inert' state₂ shifted arguments_within
    obtain ⟨β, γ, base⟩ := shifted.renames.base
    have frame_within := initialFrame?_within base arguments_within frame_eq
    obtain ⟨evaluated₂, evaluatedShifted, final_within, control_within, body₂⟩ :=
      ih_body ρ offset frontier inside inert inert' state₂ shifted frame_within
    refine ⟨finalizeFunctionState executable declaration.profile state₂ evaluated₂
        (finalFrame.rename ρ) (outcome.rename ρ), ?_,
      finishControl?_within base control_within outcome_eq, ?_⟩
    · rw [← finalize_eq]
      exact finalizeFunctionState_mirror shifted evaluatedShifted executable declaration.profile
        final_within outcome
    · exact .body _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq declaration_eq provided_eq
        (by rw [initialFrame?_rename, frame_eq]; rfl) body_eq body₂
        (by rw [finishControl?_rename, outcome_eq]; rfl)
        (by rw [Outcome.holeFree_rename]; exact hole_free) rfl
  case native =>
    intro handle typeInstantiation initialState arguments ns declaration finalState outcome
      namespace_eq declaration_eq arity_eq provided_eq native_eq hole_free
    intro ρ offset frontier inside inert inert' state₂ shifted arguments_within
    obtain ⟨finalState₂, finalShifted, outcome_within, native₂⟩ :
        ∃ state₂', StateRenamed ρ offset frontier inside inert inert' finalState state₂' ∧
          outcome.Within inside ∧
          nativeCall executable handle typeInstantiation state₂ (arguments.map (·.rename ρ)) =
            some (state₂', outcome.rename ρ) := by
      unfold nativeCall at native_eq ⊢
      split at native_eq
      · exact tableRoleCall_shifts native_eq shifted arguments_within
      · exact natives _ _ _ _ _ _ native_eq ρ offset frontier inside inert inert' state₂ shifted
          arguments_within
    exact ⟨finalState₂, finalShifted, outcome_within,
      .native _ _ _ _ _ _ _ _ namespace_eq declaration_eq (by rw [Array.size_map]; exact arity_eq)
        provided_eq native₂ (by rw [Outcome.holeFree_rename]; exact hole_free)⟩

theorem Array.map_rename_of_plain {values : Array RuntimeValue} (ρ : Nat → Nat)
    (plain : ∀ value ∈ values, Plain value) : values.map (·.rename ρ) = values := by
  conv => rhs; rw [← Array.map_id values]
  exact Array.map_congr_left fun value member => RuntimeValue.rename_of_plain ρ (plain value member)

theorem GlobalMap.rename_of_plain {globals : GlobalMap} (ρ : Nat → Nat)
    (plain : ∀ slot ∈ globals.entries, Plain slot.value) : globals.rename ρ = globals := by
  obtain ⟨entries⟩ := globals
  simp only [GlobalMap.rename, GlobalMap.mk.injEq]
  conv => rhs; rw [← Array.map_id entries]
  exact Array.map_congr_left fun slot member => by
    simp [RuntimeValue.rename_of_plain ρ (plain slot member)]

theorem SemanticOperations.FreshGlobalLoanIds.below {state : RuntimeState} (fresh : FreshGlobalLoanIds state) :
    ∀ entry ∈ state.globalLoans, entry.1 < state.nextLoan := by
  intro entry member
  refine Nat.lt_of_not_le fun beyond => ?_
  have absent := fresh entry.1 beyond
  simp only [globalLoanKeyIn?, Option.map_eq_none_iff, List.find?_eq_none] at absent
  exact absent entry member (by simp)

/-- An unregistered loan has no registry entry. -/
theorem SemanticOperations.globalLoanKeyIn?_none {registry : List (Nat × GlobalKey)} {loan : Nat}
    (absent : globalLoanKeyIn? registry loan = none) : ∀ entry ∈ registry, entry.1 ≠ loan := by
  intro entry member same
  simp only [globalLoanKeyIn?, Option.map_eq_none_iff, List.find?_eq_none] at absent
  exact absent entry member (by simp [same])

/-! ## Lent starts

A start lending references holds loans below its frontier, distinct and
unregistered. Its run mirrors onto a start with the same memory lending the
same references under consecutive loans from a frontier no lower and minting
beyond them (`StateRenamed.ofLent`); two lent starts mirror onto the same
such start, where runs are deterministic. -/

/-- The loans a run from a start lending `loans` below `frontier` holds:
those it lends and those it mints. -/
def lentInside (loans : List Nat) (frontier : Nat) (loan : Nat) : Prop :=
  frontier ≤ loan ∨ loan ∈ loans

/-- The renaming of a start lending `loans` below `frontier` onto one lending
them in order from `target` on and minting beyond: lent loans to their
positions, minted ones past them, the others kept. -/
def lentRenaming (loans : List Nat) (frontier target : Nat) (loan : Nat) : Nat :=
  if loan < frontier then (if loan ∈ loans then target + loans.idxOf loan else loan)
  else loan + (target + loans.length - frontier)

/-- A lent loan's position names it. -/
theorem List.getElem?_idxOf_of_mem {loans : List Nat} {loan : Nat} (member : loan ∈ loans) :
    loans[loans.idxOf loan]? = some loan := by
  induction loans with
  | nil => simp at member
  | cons head tail ih =>
      by_cases same : head = loan
      · subst same; simp
      · have later : loan ∈ tail := by simpa [Ne.symm same] using member
        simp only [List.idxOf_cons, cond_eq_ite, beq_iff_eq, same, ite_false,
          List.getElem?_cons_succ, ih later]

/-- Distinct lent loans are at their own positions. -/
theorem List.idxOf_getElem_of_nodup {loans : List Nat} (nodup : loans.Nodup) {index : Nat}
    (bound : index < loans.length) : loans.idxOf loans[index] = index := by
  have member : loans[index] ∈ loans := List.getElem_mem bound
  refine (List.getElem?_inj (List.idxOf_lt_length_of_mem member) nodup).1 ?_
  rw [List.getElem?_idxOf_of_mem member, List.getElem?_eq_getElem bound]

section Lent

variable {loans : List Nat} {frontier target : Nat}

theorem lentRenaming_injective (le : frontier ≤ target) :
    Function.Injective (lentRenaming loans frontier target) := by
  intro left right same
  unfold lentRenaming at same
  have leftBound := fun (member : left ∈ loans) => List.idxOf_lt_length_of_mem member
  have rightBound := fun (member : right ∈ loans) => List.idxOf_lt_length_of_mem member
  by_cases leftBelow : left < frontier <;> by_cases rightBelow : right < frontier <;>
    simp only [leftBelow, rightBelow, ite_true, ite_false] at same
  · by_cases leftLent : left ∈ loans <;> by_cases rightLent : right ∈ loans <;>
      simp only [leftLent, rightLent, ite_true, ite_false] at same
    · have index_eq : loans.idxOf left = loans.idxOf right := by omega
      have found := List.getElem?_idxOf_of_mem leftLent
      rw [index_eq, List.getElem?_idxOf_of_mem rightLent] at found
      exact (Option.some.inj found).symm
    · omega
    · omega
    · exact same
  · by_cases leftLent : left ∈ loans <;> simp only [leftLent, ite_true, ite_false] at same
    · have := leftBound leftLent; omega
    · omega
  · by_cases rightLent : right ∈ loans <;> simp only [rightLent, ite_true, ite_false] at same
    · have := rightBound rightLent; omega
    · omega
  · omega

theorem lentRenaming_minted {loan : Nat} (beyond : frontier ≤ loan) :
    lentRenaming loans frontier target loan = loan + (target + loans.length - frontier) := by
  simp [lentRenaming, Nat.not_lt.mpr beyond]

/-- The lent loans are renamed to consecutive ones from `target` on. -/
theorem lentRenaming_lent (nodup : loans.Nodup) (below : ∀ loan ∈ loans, loan < frontier) :
    loans.map (lentRenaming loans frontier target) = List.range' target loans.length := by
  apply List.ext_getElem (by simp)
  intro index bound _
  simp only [List.getElem_map, List.getElem_range']
  have member : loans[index]'(by simpa using bound) ∈ loans := List.getElem_mem _
  simp only [lentRenaming, below _ member, member, ite_true,
    List.idxOf_getElem_of_nodup nodup (by simpa using bound), Nat.one_mul]

/-- The loans of a lent start enumerated: the lent ones first, then those
minted from the frontier on. -/
theorem lentInside_base (nodup : loans.Nodup) (below : ∀ loan ∈ loans, loan < frontier) :
    LoanBase (fun index => if index < loans.length then loans.getD index 0
        else index - loans.length + frontier)
      (fun loan => if loan ∈ loans then loans.idxOf loan else loan - frontier + loans.length)
      (lentInside loans frontier) where
  injective left right same := by
    simp only [List.getD_eq_getElem?_getD] at same
    by_cases leftLent : left < loans.length <;> by_cases rightLent : right < loans.length <;>
      simp only [leftLent, rightLent, ite_true, ite_false] at same
    · rw [List.getElem?_eq_getElem leftLent, List.getElem?_eq_getElem rightLent] at same
      exact (List.getElem_inj nodup).1 same
    · rw [List.getElem?_eq_getElem leftLent] at same
      have := below _ (List.getElem_mem leftLent); simp only [Option.getD_some] at same; omega
    · rw [List.getElem?_eq_getElem rightLent] at same
      have := below _ (List.getElem_mem rightLent); simp only [Option.getD_some] at same; omega
    · omega
  inverse loan inside := by
    simp only [List.getD_eq_getElem?_getD]
    by_cases lent : loan ∈ loans
    · simp only [lent, ite_true, List.idxOf_lt_length_of_mem lent, List.getElem?_idxOf_of_mem lent,
        Option.getD_some]
    · have beyond : frontier ≤ loan := inside.resolve_right lent
      simp only [lent, ite_false]
      rw [if_neg (by omega)]
      omega
  image index := by
    by_cases lent : index < loans.length
    · simp only [lent, ite_true, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem lent,
        Option.getD_some]
      exact .inr (List.getElem_mem lent)
    · simp only [lent, ite_false]
      exact .inl (by omega)

/-- A start lending distinct unregistered `loans` below its frontier, with
memory holding no loan, mirrors onto one with that memory whose frontier is
`target` past them and whose registry is below `target`, lending the same
references under consecutive loans from `target`. -/
theorem StateRenamed.ofLent {start start₂ : RuntimeState}
    (globals_eq : start₂.globals = start.globals)
    (tableHandles_eq : start₂.tableHandles = start.tableHandles)
    (plain : ∀ slot ∈ start.globals.entries, Plain slot.value)
    (nodup : loans.Nodup) (below : ∀ loan ∈ loans, loan < start.nextLoan)
    (unregistered : ∀ loan ∈ loans, globalLoanKeyIn? start.globalLoans loan = none)
    (fresh : FreshGlobalLoanIds start)
    (le : start.nextLoan ≤ target) (next : start₂.nextLoan = target + loans.length)
    (registered : ∀ entry ∈ start₂.globalLoans, entry.1 < target) :
    StateRenamed (lentRenaming loans start.nextLoan target)
      (target + loans.length - start.nextLoan) start.nextLoan (lentInside loans start.nextLoan)
      start.pending.size start₂.pending.size start start₂ where
  renames := {
    injective := lentRenaming_injective le
    minted := fun _ beyond => lentRenaming_minted beyond
    covers := fun _ beyond => .inl beyond
    base := ⟨_, _, lentInside_base nodup below⟩ }
  globals := by rw [globals_eq, GlobalMap.rename_of_plain _ plain]
  tableHandles := tableHandles_eq
  nextLoan := by omega
  frontier_le := Nat.le_refl _
  inert_le := Nat.le_refl _
  inert_le' := Nat.le_refl _
  pending := by simp
  registry := ⟨[], start.globalLoans, start₂.globalLoans, rfl, rfl, by simp,
    fun entry member inside => by
      have := fresh.below entry member
      rcases inside with beyond | lent
      · omega
      · exact SemanticOperations.globalLoanKeyIn?_none (unregistered _ lent) entry member rfl,
    fun entry member loan inside => by
      have := registered entry member
      unfold lentRenaming
      rcases inside with beyond | lent
      · rw [if_neg (by omega)]; omega
      · rw [if_pos (below _ lent), if_pos lent]; omega⟩
  globalsWithin := fun slot member => RuntimeValue.within_of_plain _ (plain slot member)
  pendingWithin := by simp

end Lent

/-- Loan independence: a run from a start lending distinct unregistered
loans, whose global memory holds no loan, on arguments holding
only lent loans, is mirrored from a start with that memory lending them in
order from a frontier no lower and minting beyond them, with its loans
renamed. -/
theorem runs_mirror {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    (natives : NativesShift executable)
    {handle : FunctionHandle} {instantiation : Array (TypeId × TypeId)}
    {start start₂ final : RuntimeState} {arguments : Array RuntimeValue} {outcome : Outcome}
    {loans : List Nat} {target : Nat}
    (run : EvalFunction executable handle instantiation start arguments final outcome)
    (globals_eq : start₂.globals = start.globals)
    (tableHandles_eq : start₂.tableHandles = start.tableHandles)
    (plain : ∀ slot ∈ start.globals.entries, Plain slot.value)
    (nodup : loans.Nodup) (below : ∀ loan ∈ loans, loan < start.nextLoan)
    (unregistered : ∀ loan ∈ loans, globalLoanKeyIn? start.globalLoans loan = none)
    (fresh : FreshGlobalLoanIds start)
    (le : start.nextLoan ≤ target) (next : start₂.nextLoan = target + loans.length)
    (registered : ∀ entry ∈ start₂.globalLoans, entry.1 < target)
    (arguments_within : ∀ argument ∈ arguments, argument.Within (lentInside loans start.nextLoan)) :
    ∃ final₂, EvalFunction executable handle instantiation start₂
        (arguments.map (·.rename (lentRenaming loans start.nextLoan target))) final₂
        (outcome.rename (lentRenaming loans start.nextLoan target)) ∧
      final₂.globals = final.globals.rename (lentRenaming loans start.nextLoan target) ∧
      final₂.tableHandles = final.tableHandles := by
  obtain ⟨final₂, finalShifted, -, run₂⟩ :=
    shifts natives run _ _ _ _ _ _ start₂
      (StateRenamed.ofLent globals_eq tableHandles_eq plain nodup below unregistered fresh
        le next registered) arguments_within
  exact ⟨final₂, run₂, finalShifted.globals, finalShifted.tableHandles⟩

end VerexIR
