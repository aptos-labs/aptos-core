-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.BigStep
import LeanerIR.Semantics.LoanRenamingFunctions
import LeanerIR.Semantics.LoanRenamingPrimitives

/-!
# Loan independence

A run mirrors from any start whose loan bookkeeping differs: with the
loans it mints raised by the difference of the frontiers
(`designs/static-typing.md`, "Loan independence"). Each relation of the
big-step semantics is simulated from a state `StateShifted` from the
first run's, by a forward induction over the first run's derivation.
-/

namespace LeanerIR

open Validation
open SemanticOperations
open BigStep

theorem BigStep.Abrupt.shift {control : Control} (abrupt : Abrupt control) (offset : Nat) :
    Abrupt (control.shift offset) := by
  cases abrupt <;> constructor

@[simp] theorem Control.shift_value (offset : Nat) (value : RuntimeValue) :
    (Control.value value).shift offset = .value (value.shift offset) := rfl
@[simp] theorem Control.shift_break (offset nest : Nat) (value : Option RuntimeValue) :
    (Control.break_ nest value).shift offset = .break_ nest (value.map (·.shift offset)) := rfl
@[simp] theorem Control.shift_continue (offset nest : Nat) :
    (Control.continue_ nest).shift offset = .continue_ nest := rfl
@[simp] theorem Control.shift_return (offset : Nat) (values : Array RuntimeValue) :
    (Control.return_ values).shift offset = .return_ (values.map (·.shift offset)) := rfl
@[simp] theorem Control.shift_throw (offset : Nat) (kind : ThrowKind) (arguments : Array RuntimeValue) :
    (Control.throw_ kind arguments).shift offset = .throw_ kind (arguments.map (·.shift offset)) :=
  rfl

/-- A pattern mismatch throws values that hold no loans. -/
theorem patternMismatchThrow_shift {profile : Option Profile} {kind : ThrowKind}
    {arguments : Array RuntimeValue}
    (mismatch_eq : patternMismatchThrow? profile = some (kind, arguments)) (offset frontier : Nat) :
    (Control.throw_ kind arguments).shift offset = .throw_ kind arguments ∧
      (Control.throw_ kind arguments).Above frontier := by
  cases profile with
  | none => simp [patternMismatchThrow?, patternMismatch?] at mismatch_eq
  | some profile =>
      cases profile <;> simp [patternMismatchThrow?, patternMismatch?] at mismatch_eq
      obtain ⟨rfl, rfl⟩ := mismatch_eq
      simp [Control.shift, Control.Above, RuntimeValue.shift, RuntimeValue.Above]

def ExprShifts {unit : ValidatedUnit} (executable : ExecutableUnit unit) (namespaceId : NamespaceId)
    (frame : RuntimeFrame)
    (state : RuntimeState) (id : ExprId) (frame' : RuntimeFrame) (state' : RuntimeState)
    (control : Control) : Prop :=
  ∀ (offset frontier inert inert' : Nat) (state₂ : RuntimeState),
    StateShifted offset frontier inert inert' state state₂ → frame.Above frontier →
    ∃ state₂', StateShifted offset frontier inert inert' state' state₂' ∧
      frame'.Above frontier ∧ control.Above frontier ∧
      EvalExprWith executable (EvalFunction executable) namespaceId (frame.shift offset) state₂ id
        (frame'.shift offset) state₂' (control.shift offset)

def NodeShifts {unit : ValidatedUnit} (executable : ExecutableUnit unit) (namespaceId : NamespaceId)
    (frame : RuntimeFrame)
    (state : RuntimeState) (id : ExprId) (frame' : RuntimeFrame) (state' : RuntimeState)
    (control : Control) : Prop :=
  ∀ (offset frontier inert inert' : Nat) (state₂ : RuntimeState),
    StateShifted offset frontier inert inert' state state₂ → frame.Above frontier →
    ∃ state₂', StateShifted offset frontier inert inert' state' state₂' ∧
      frame'.Above frontier ∧ control.Above frontier ∧
      EvalNodeWith executable (EvalFunction executable) namespaceId (frame.shift offset) state₂ id
        (frame'.shift offset) state₂' (control.shift offset)

def ValuesShift {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (namespaceId : NamespaceId) (frame : RuntimeFrame)
    (state : RuntimeState) (ids : List ExprId) (result : ValuesResult) : Prop :=
  ∀ (offset frontier inert inert' : Nat) (state₂ : RuntimeState),
    StateShifted offset frontier inert inert' state state₂ → frame.Above frontier →
    match result with
    | .values state' frame' values => ∃ state₂',
        StateShifted offset frontier inert inert' state' state₂' ∧ frame'.Above frontier ∧
        (∀ value ∈ values, value.Above frontier) ∧
        EvalValuesWith executable (EvalFunction executable) namespaceId (frame.shift offset) state₂ ids
          (.values state₂' (frame'.shift offset) (values.map (·.shift offset)))
    | .control state' frame' control => ∃ state₂',
        StateShifted offset frontier inert inert' state' state₂' ∧ frame'.Above frontier ∧
        control.Above frontier ∧
        EvalValuesWith executable (EvalFunction executable) namespaceId (frame.shift offset) state₂ ids
          (.control state₂' (frame'.shift offset) (control.shift offset))

def StatementsShift {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (namespaceId : NamespaceId) (frame : RuntimeFrame)
    (state : RuntimeState) (statements : List ExprId) (result : StatementsResult) : Prop :=
  ∀ (offset frontier inert inert' : Nat) (state₂ : RuntimeState),
    StateShifted offset frontier inert inert' state state₂ → frame.Above frontier →
    match result with
    | .done state' frame' => ∃ state₂',
        StateShifted offset frontier inert inert' state' state₂' ∧ frame'.Above frontier ∧
        EvalStatementsWith executable (EvalFunction executable) namespaceId (frame.shift offset) state₂
          statements (.done state₂' (frame'.shift offset))
    | .control state' frame' control => ∃ state₂',
        StateShifted offset frontier inert inert' state' state₂' ∧ frame'.Above frontier ∧
        control.Above frontier ∧
        EvalStatementsWith executable (EvalFunction executable) namespaceId (frame.shift offset) state₂
          statements (.control state₂' (frame'.shift offset) (control.shift offset))

def ArmsShift {unit : ValidatedUnit} (executable : ExecutableUnit unit) (namespaceId : NamespaceId)
    (ns : ValidatedNamespace)
    (frame : RuntimeFrame) (state : RuntimeState) (value : RuntimeValue) (arms : List MatchArm)
    (frame' : RuntimeFrame) (state' : RuntimeState) (control : Control) : Prop :=
  ∀ (offset frontier inert inert' : Nat) (state₂ : RuntimeState),
    StateShifted offset frontier inert inert' state state₂ → frame.Above frontier →
    value.Above frontier →
    ∃ state₂', StateShifted offset frontier inert inert' state' state₂' ∧
      frame'.Above frontier ∧ control.Above frontier ∧
      EvalArmsWith executable (EvalFunction executable) namespaceId ns (frame.shift offset) state₂
        (value.shift offset) arms (frame'.shift offset) state₂' (control.shift offset)

def FunctionShifts {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (handle : FunctionHandle)
    (instantiation : Array (TypeId × TypeId)) (state : RuntimeState)
    (arguments : Array RuntimeValue) (state' : RuntimeState) (outcome : Outcome) : Prop :=
  ∀ (offset frontier inert inert' : Nat) (state₂ : RuntimeState),
    StateShifted offset frontier inert inert' state state₂ →
    (∀ argument ∈ arguments, argument.Above frontier) →
    ∃ state₂', StateShifted offset frontier inert inert' state' state₂' ∧
      outcome.Above frontier ∧
      EvalFunction executable handle instantiation state₂ (arguments.map (·.shift offset)) state₂'
        (outcome.shift offset)

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
  intro offset frontier inert inert' state₂ shifted frame_above
  obtain ⟨finalState₂, finalShifted, final_above, control_above, operands₂⟩ :=
    ih offset frontier inert inert' state₂ shifted frame_above
  exact ⟨finalState₂, finalShifted, final_above, control_above, build operands₂⟩

/-- The natives mirror their runs: the runtime provides them, and
`nativeCall` is opaque. -/
def NativesShift {unit : ValidatedUnit} (executable : ExecutableUnit unit) : Prop :=
  ∀ handle instantiation state arguments state' outcome,
    nativeCall executable handle instantiation state arguments = some (state', outcome) →
    ∀ (offset frontier inert inert' : Nat) (state₂ : RuntimeState),
      StateShifted offset frontier inert inert' state state₂ →
      (∀ argument ∈ arguments, argument.Above frontier) →
      ∃ state₂', StateShifted offset frontier inert inert' state' state₂' ∧
        outcome.Above frontier ∧
        nativeCall executable handle instantiation state₂ (arguments.map (·.shift offset)) =
          some (state₂', outcome.shift offset)

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
    intro offset frontier inert inert' state₂ shifted frame_above
    have settled := settleLoans_shift (loans := (loanDeathsAt unit namespaceId exprId).before)
      shifted frame_above
    rw [before_eq] at settled
    generalize start_eq : settleLoans (loanDeathsAt unit namespaceId exprId).before
      (frame.shift offset) state₂ = start₂ at settled
    obtain ⟨startFrame₂, startState₂⟩ := start₂
    obtain ⟨rfl, startShifted, start_above⟩ := settled
    obtain ⟨nodeState₂, nodeShifted, node_above, control_above, node_step₂⟩ :=
      ih offset frontier inert inert' startState₂ startShifted start_above
    have after := settleAfter_shift (loans := (loanDeathsAt unit namespaceId exprId).after)
      (control := control) nodeShifted node_above
    rw [after_eq] at after
    generalize final_eq : settleAfter (loanDeathsAt unit namespaceId exprId).after
      (control.shift offset) (nodeFrame.shift offset) nodeState₂ = final₂ at after
    obtain ⟨finalFrame₂, finalState₂⟩ := final₂
    obtain ⟨rfl, finalShifted, final_above⟩ := after
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .node _ _ _ _ _ _ _ _ _ _ _ start_eq node_step₂ final_eq⟩
  case value =>
    intro namespaceId frame state exprId ns expression literal source runtimeValue
      namespace_eq expression_eq kind_eq value_eq
    intro offset frontier inert inert' state₂ shifted frame_above
    have plain := constValue?_plain _ literal (Nat.le_refl _) value_eq
    refine ⟨state₂, shifted, frame_above, RuntimeValue.above_of_plain frontier plain, ?_⟩
    rw [Control.shift_value, RuntimeValue.shift_of_plain offset plain]
    exact .value _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq value_eq
  case localVar =>
    intro namespaceId frame state exprId ns expression localId runtimeValue
      namespace_eq expression_eq kind_eq local_eq
    intro offset frontier inert inert' state₂ shifted frame_above
    refine ⟨state₂, shifted, frame_above, ?_, ?_⟩
    · unfold readLocal? at local_eq
      cases slot_eq : frame.locals[localId.index]? with
      | none => simp [slot_eq] at local_eq
      | some slot =>
          rw [slot_eq] at local_eq
          cases slot with
          | none => cases local_eq
          | some value =>
              cases local_eq
              exact frame_above.slot slot_eq
    · exact .localVar _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
        (by rw [readLocal?_shift, local_eq]; rfl)
  case spec =>
    intro namespaceId frame state exprId ns expression block namespace_eq expression_eq kind_eq
    intro offset frontier inert inert' state₂ shifted frame_above
    refine ⟨state₂, shifted, frame_above, by simp [Control.Above, RuntimeValue.Above], ?_⟩
    rw [Control.shift_value, RuntimeValue.shift_unit]
    exact .spec _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
  case continue_ =>
    intro namespaceId frame state exprId ns expression nest namespace_eq expression_eq kind_eq
    intro offset frontier inert inert' state₂ shifted frame_above
    exact ⟨state₂, shifted, frame_above, trivial,
      .continue_ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq⟩
  case breakNone =>
    intro namespaceId frame state exprId ns expression nest namespace_eq expression_eq kind_eq
    intro offset frontier inert inert' state₂ shifted frame_above
    exact ⟨state₂, shifted, frame_above, by simp [Control.Above],
      .breakNone _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq⟩
  case breakControl =>
    intro namespaceId frame state exprId ns expression nest child finalFrame finalState control
      namespace_eq expression_eq kind_eq child_step abrupt ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, child_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .breakControl _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq child_step₂
        (abrupt.shift offset)⟩
  case breakValue =>
    intro namespaceId frame state exprId ns expression nest child finalFrame finalState
      runtimeValue namespace_eq expression_eq kind_eq child_step ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, child_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    exact ⟨finalState₂, finalShifted, final_above, by simpa [Control.Above] using control_above,
      .breakValue _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq child_step₂⟩
  case returnValues =>
    intro namespaceId frame state exprId ns expression values finalFrame finalState
      runtimeValues namespace_eq expression_eq kind_eq values_step ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, values_above, values_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    refine ⟨finalState₂, finalShifted, final_above, by simpa [Control.Above] using values_above,
      ?_⟩
    rw [Control.shift_return, List.map_toArray]
    exact .returnValues _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq values_step₂
  case returnControl =>
    intro namespaceId frame state exprId ns expression values finalFrame finalState control
      namespace_eq expression_eq kind_eq values_step ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, values_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .returnControl _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq values_step₂⟩
  case throwValues =>
    intro namespaceId frame state exprId ns expression kind arguments finalFrame finalState
      runtimeValues namespace_eq expression_eq kind_eq values_step ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, values_above, values_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    refine ⟨finalState₂, finalShifted, final_above, by simpa [Control.Above] using values_above,
      ?_⟩
    rw [Control.shift_throw, List.map_toArray]
    exact .throwValues _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq values_step₂
  case throwControl =>
    intro namespaceId frame state exprId ns expression kind arguments finalFrame finalState control
      namespace_eq expression_eq kind_eq values_step ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, values_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .throwControl _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq values_step₂⟩
  case nil =>
    intro namespaceId frame state
    intro offset frontier inert inert' state₂ shifted frame_above
    exact ⟨state₂, shifted, frame_above, by simp, .nil _ _ _⟩
  case headControl =>
    intro namespaceId frame state expression expressions finalFrame finalState control
      head_step abrupt ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, head_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .headControl _ _ _ _ _ _ _ _ head_step₂ (abrupt.shift offset)⟩
  case tailValues =>
    intro namespaceId frame state expression expressions headFrame headState value finalFrame
      finalState values head_step tail_step ih_head ih_tail
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨headState₂, headShifted, head_above, value_above, head_step₂⟩ :=
      ih_head offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, values_above, tail_step₂⟩ :=
      ih_tail offset frontier inert inert' headState₂ headShifted head_above
    refine ⟨finalState₂, finalShifted, final_above, ?_, ?_⟩
    · intro element member
      rcases List.mem_cons.mp member with rfl | member
      · exact value_above
      · exact values_above element member
    · rw [List.map_cons]
      exact .tailValues _ _ _ _ _ _ _ _ _ _ _ head_step₂ tail_step₂
  case tailControl =>
    intro namespaceId frame state expression expressions headFrame headState value finalFrame
      finalState control head_step tail_step ih_head ih_tail
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨headState₂, headShifted, head_above, -, head_step₂⟩ :=
      ih_head offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, tail_step₂⟩ :=
      ih_tail offset frontier inert inert' headState₂ headShifted head_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .tailControl _ _ _ _ _ _ _ _ _ _ _ head_step₂ tail_step₂⟩
  case nil =>
    intro namespaceId frame state
    intro offset frontier inert inert' state₂ shifted frame_above
    exact ⟨state₂, shifted, frame_above, .nil _ _ _⟩
  case headControl =>
    intro namespaceId frame state statement statements finalFrame finalState control
      head_step abrupt ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, head_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .headControl _ _ _ _ _ _ _ _ head_step₂ (abrupt.shift offset)⟩
  case cons =>
    intro namespaceId frame state statement statements headFrame headState value result
      head_step tail_step ih_head ih_tail
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨headState₂, headShifted, head_above, -, head_step₂⟩ :=
      ih_head offset frontier inert inert' state₂ shifted frame_above
    have tail := ih_tail offset frontier inert inert' headState₂ headShifted head_above
    cases result with
    | done finalState finalFrame =>
        obtain ⟨finalState₂, finalShifted, final_above, tail_step₂⟩ := tail
        exact ⟨finalState₂, finalShifted, final_above, .cons _ _ _ _ _ _ _ _ _ head_step₂ tail_step₂⟩
    | control finalState finalFrame control =>
        obtain ⟨finalState₂, finalShifted, final_above, control_above, tail_step₂⟩ := tail
        exact ⟨finalState₂, finalShifted, final_above, control_above,
          .cons _ _ _ _ _ _ _ _ _ head_step₂ tail_step₂⟩
  case blockControl =>
    intro namespaceId frame state exprId ns expression statements result finalState finalFrame
      control namespace_eq expression_eq kind_eq steps ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, steps₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .blockControl _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq steps₂⟩
  case blockUnit =>
    intro namespaceId frame state exprId ns expression statements finalState finalFrame
      namespace_eq expression_eq kind_eq steps ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, steps₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    refine ⟨finalState₂, finalShifted, final_above, by simp [Control.Above, RuntimeValue.Above], ?_⟩
    rw [Control.shift_value, RuntimeValue.shift_unit]
    exact .blockUnit _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq steps₂
  case blockResult =>
    intro namespaceId frame state exprId ns expression statements result statementState
      statementFrame finalState finalFrame control namespace_eq expression_eq kind_eq steps
      result_step ih_steps ih_result
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨statementState₂, statementShifted, statement_above, steps₂⟩ :=
      ih_steps offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, result_step₂⟩ :=
      ih_result offset frontier inert inert' statementState₂ statementShifted statement_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .blockResult _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq steps₂
        result_step₂⟩
  case letNoValue =>
    intro namespaceId frame state exprId ns expression pattern body finalFrame finalState control
      namespace_eq expression_eq kind_eq body_step ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, body_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .letNoValue _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq body_step₂⟩
  case letValueControl =>
    intro namespaceId frame state exprId ns expression pattern initializer body finalFrame
      finalState control namespace_eq expression_eq kind_eq initializer_step abrupt ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, initializer_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .letValueControl _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
        initializer_step₂ (abrupt.shift offset)⟩
  case ifControl =>
    intro namespaceId frame state exprId ns expression condition thenBranch elseBranch finalFrame
      finalState control namespace_eq expression_eq kind_eq condition_step abrupt ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, condition_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .ifControl _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq condition_step₂
        (abrupt.shift offset)⟩
  case ifTrue =>
    intro namespaceId frame state exprId ns expression condition thenBranch elseBranch
      conditionFrame conditionState finalFrame finalState control namespace_eq expression_eq
      kind_eq condition_step branch_step ih_condition ih_branch
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨conditionState₂, conditionShifted, condition_above, -, condition_step₂⟩ :=
      ih_condition offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, branch_step₂⟩ :=
      ih_branch offset frontier inert inert' conditionState₂ conditionShifted condition_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .ifTrue (namespace_eq := namespace_eq) (expression_eq := expression_eq) (kind_eq := kind_eq)
        (condition_step := by simpa using condition_step₂) (branch_step := branch_step₂)⟩
  case ifFalseUnit =>
    intro namespaceId frame state exprId ns expression condition thenBranch conditionFrame
      conditionState namespace_eq expression_eq kind_eq condition_step ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨conditionState₂, conditionShifted, condition_above, -, condition_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    refine ⟨conditionState₂, conditionShifted, condition_above,
      by simp [Control.Above, RuntimeValue.Above], ?_⟩
    rw [Control.shift_value, RuntimeValue.shift_unit]
    exact .ifFalseUnit _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
      (by simpa using condition_step₂)
  case ifFalse =>
    intro namespaceId frame state exprId ns expression condition thenBranch elseBranch
      conditionFrame conditionState finalFrame finalState control namespace_eq expression_eq
      kind_eq condition_step branch_step ih_condition ih_branch
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨conditionState₂, conditionShifted, condition_above, -, condition_step₂⟩ :=
      ih_condition offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, branch_step₂⟩ :=
      ih_branch offset frontier inert inert' conditionState₂ conditionShifted condition_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .ifFalse (namespace_eq := namespace_eq) (expression_eq := expression_eq) (kind_eq := kind_eq)
        (condition_step := by simpa using condition_step₂) (branch_step := branch_step₂)⟩
  case matchControl =>
    intro namespaceId frame state exprId ns expression scrutinee arms finalFrame finalState control
      namespace_eq expression_eq kind_eq scrutinee_step abrupt ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, scrutinee_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .matchControl _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq scrutinee_step₂
        (abrupt.shift offset)⟩
  case matchValue =>
    intro namespaceId frame state exprId ns expression scrutinee arms scrutineeFrame
      scrutineeState runtimeValue finalFrame finalState control namespace_eq expression_eq kind_eq
      scrutinee_step arm_step ih_scrutinee ih_arms
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨scrutineeState₂, scrutineeShifted, scrutinee_above, value_above, scrutinee_step₂⟩ :=
      ih_scrutinee offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, arm_step₂⟩ :=
      ih_arms offset frontier inert inert' scrutineeState₂ scrutineeShifted scrutinee_above
        value_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .matchValue _ _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
        scrutinee_step₂ arm_step₂⟩
  case assignControl =>
    intro namespaceId frame state exprId ns expression place child finalFrame finalState control
      namespace_eq expression_eq kind_eq child_step abrupt ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, child_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .assignControl _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq child_step₂
        (abrupt.shift offset)⟩
  case assignPatternControl =>
    intro namespaceId frame state exprId ns expression pattern child finalFrame finalState control
      namespace_eq expression_eq kind_eq child_step abrupt ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, child_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .assignPatternControl _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq child_step₂
        (abrupt.shift offset)⟩
  case loopRepeatValue =>
    intro namespaceId frame state exprId ns expression label body bodyFrame bodyState
      runtimeValue finalFrame finalState control namespace_eq expression_eq kind_eq body_step
      repeat_step ih_body ih_repeat
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨bodyState₂, bodyShifted, body_above, -, body_step₂⟩ :=
      ih_body offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, repeat_step₂⟩ :=
      ih_repeat offset frontier inert inert' bodyState₂ bodyShifted body_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .loopRepeatValue (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (body_step := body_step₂) (repeat_step := repeat_step₂)⟩
  case loopRepeatContinue =>
    intro namespaceId frame state exprId ns expression label body bodyFrame bodyState finalFrame
      finalState control namespace_eq expression_eq kind_eq body_step repeat_step ih_body ih_repeat
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨bodyState₂, bodyShifted, body_above, -, body_step₂⟩ :=
      ih_body offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, repeat_step₂⟩ :=
      ih_repeat offset frontier inert inert' bodyState₂ bodyShifted body_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .loopRepeatContinue (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (body_step := body_step₂) (repeat_step := repeat_step₂)⟩
  case loopBreak =>
    intro namespaceId frame state exprId ns expression label body finalFrame finalState value
      namespace_eq expression_eq kind_eq body_step ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, body_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    refine ⟨finalState₂, finalShifted, final_above, ?_, ?_⟩
    · cases value with
      | none => simp [Control.Above, RuntimeValue.Above]
      | some value => simpa [Control.Above] using control_above
    · have same : (Control.value (value.getD .unit)).shift offset =
          .value ((value.map (·.shift offset)).getD .unit) := by
        cases value <;> simp
      rw [same]
      exact .loopBreak (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (body_step := body_step₂)
  case loopOuterBreak =>
    intro namespaceId frame state exprId ns expression label body finalFrame finalState nest value
      namespace_eq expression_eq kind_eq body_step ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, body_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .loopOuterBreak (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (body_step := body_step₂)⟩
  case loopOuterContinue =>
    intro namespaceId frame state exprId ns expression label body finalFrame finalState nest
      namespace_eq expression_eq kind_eq body_step ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, body_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .loopOuterContinue (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (body_step := body_step₂)⟩
  case loopReturn =>
    intro namespaceId frame state exprId ns expression label body finalFrame finalState values
      namespace_eq expression_eq kind_eq body_step ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, body_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .loopReturn (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (body_step := body_step₂)⟩
  case loopThrow =>
    intro namespaceId frame state exprId ns expression label body finalFrame finalState kind
      arguments namespace_eq expression_eq kind_eq body_step ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, body_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .loopThrow (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (body_step := body_step₂)⟩
  case letValue =>
    intro namespaceId frame state exprId ns expression pattern initializer body initializedFrame
      initializedState runtimeValue boundFrame finalFrame finalState control namespace_eq
      expression_eq kind_eq initializer_step bind_eq body_step ih_initializer ih_body
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨initializedState₂, initializedShifted, initialized_above, value_above,
      initializer_step₂⟩ := ih_initializer offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, body_step₂⟩ :=
      ih_body offset frontier inert inert' initializedState₂ initializedShifted
        (bindPattern_above initialized_above value_above bind_eq)
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .letValue (namespace_eq := namespace_eq) (expression_eq := expression_eq) (kind_eq := kind_eq)
        (initializer_step := initializer_step₂)
        (bind_eq := by rw [bindPattern_shift, bind_eq]; rfl) (body_step := body_step₂)⟩
  case letMismatch =>
    intro namespaceId frame state exprId ns expression pattern initializer body initializedFrame
      initializedState runtimeValue kind arguments namespace_eq expression_eq kind_eq
      initializer_step bind_eq mismatch_eq ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨initializedState₂, initializedShifted, initialized_above, -, initializer_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨shift_eq, above⟩ := patternMismatchThrow_shift mismatch_eq offset frontier
    refine ⟨initializedState₂, initializedShifted, initialized_above, above, ?_⟩
    rw [shift_eq]
    exact .letMismatch (namespace_eq := namespace_eq) (expression_eq := expression_eq)
      (kind_eq := kind_eq) (initializer_step := initializer_step₂)
      (bind_eq := by rw [bindPattern_shift, bind_eq]; rfl) (mismatch_eq := mismatch_eq)
  case assignPatternMismatch =>
    intro namespaceId frame state exprId ns expression pattern child childFrame finalState
      runtimeValue kind arguments namespace_eq expression_eq kind_eq child_step bind_eq
      mismatch_eq ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, child_above, -, child_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨shift_eq, above⟩ := patternMismatchThrow_shift mismatch_eq offset frontier
    refine ⟨finalState₂, finalShifted, child_above, above, ?_⟩
    rw [shift_eq]
    exact .assignPatternMismatch (namespace_eq := namespace_eq) (expression_eq := expression_eq)
      (kind_eq := kind_eq) (child_step := child_step₂)
      (bind_eq := by rw [bindPattern_shift, bind_eq]; rfl) (mismatch_eq := mismatch_eq)
  case assignPatternValue =>
    intro namespaceId frame state exprId ns expression pattern child childFrame finalFrame
      finalState runtimeValue namespace_eq expression_eq kind_eq child_step bind_eq ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, child_above, value_above, child_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    refine ⟨finalState₂, finalShifted, bindPattern_above child_above value_above bind_eq,
      by simp [Control.Above, RuntimeValue.Above], ?_⟩
    rw [Control.shift_value, RuntimeValue.shift_unit]
    exact .assignPatternValue (namespace_eq := namespace_eq) (expression_eq := expression_eq)
      (kind_eq := kind_eq) (child_step := child_step₂)
      (bind_eq := by rw [bindPattern_shift, bind_eq]; rfl)
  case exhausted =>
    intro namespaceId ns frame state value kind arguments mismatch_eq
    intro offset frontier inert inert' state₂ shifted frame_above value_above
    obtain ⟨shift_eq, above⟩ := patternMismatchThrow_shift mismatch_eq offset frontier
    refine ⟨state₂, shifted, frame_above, above, ?_⟩
    rw [shift_eq]
    exact .exhausted _ _ _ _ _ _ _ mismatch_eq
  case reject =>
    intro namespaceId ns frame state value arm arms finalFrame finalState control bind_eq tail_step
      ih
    intro offset frontier inert inert' state₂ shifted frame_above value_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, tail_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above value_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .reject (arms := arms) (bind_eq := by rw [bindPattern_shift, bind_eq]; rfl) (tail_step := tail_step₂)⟩
  case noGuard =>
    intro namespaceId ns frame state value arm arms armFrame finalFrame finalState control
      guard_eq bind_eq body_step ih
    intro offset frontier inert inert' state₂ shifted frame_above value_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, body_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted (bindPattern_above frame_above value_above bind_eq)
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .noGuard (arms := arms) (guard_eq := guard_eq) (bind_eq := by rw [bindPattern_shift, bind_eq]; rfl)
        (body_step := body_step₂)⟩
  case guardControl =>
    intro namespaceId ns frame state value arm arms guard armFrame finalFrame finalState control
      guard_eq bind_eq guard_step abrupt ih
    intro offset frontier inert inert' state₂ shifted frame_above value_above
    obtain ⟨finalState₂, finalShifted, final_above, control_above, guard_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted (bindPattern_above frame_above value_above bind_eq)
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .guardControl (arms := arms) (guard_eq := guard_eq) (bind_eq := by rw [bindPattern_shift, bind_eq]; rfl)
        (guard_step := guard_step₂) (abrupt := abrupt.shift offset)⟩
  case guardTrue =>
    intro namespaceId ns frame state value arm arms guard armFrame guardFrame guardState
      finalFrame finalState control guard_eq bind_eq guard_step body_step ih_guard ih_body
    intro offset frontier inert inert' state₂ shifted frame_above value_above
    obtain ⟨guardState₂, guardShifted, guard_above, -, guard_step₂⟩ :=
      ih_guard offset frontier inert inert' state₂ shifted
        (bindPattern_above frame_above value_above bind_eq)
    obtain ⟨finalState₂, finalShifted, final_above, control_above, body_step₂⟩ :=
      ih_body offset frontier inert inert' guardState₂ guardShifted guard_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .guardTrue (arms := arms) (guard_eq := guard_eq) (bind_eq := by rw [bindPattern_shift, bind_eq]; rfl)
        (guard_step := by simpa using guard_step₂) (body_step := body_step₂)⟩
  case guardFalse =>
    intro namespaceId ns frame state value arm arms guard armFrame guardFrame guardState
      finalFrame finalState control guard_eq bind_eq guard_step tail_step ih_guard ih_tail
    intro offset frontier inert inert' state₂ shifted frame_above value_above
    obtain ⟨guardState₂, guardShifted, -, -, guard_step₂⟩ :=
      ih_guard offset frontier inert inert' state₂ shifted
        (bindPattern_above frame_above value_above bind_eq)
    obtain ⟨finalState₂, finalShifted, final_above, control_above, tail_step₂⟩ :=
      ih_tail offset frontier inert inert' state₂ shifted frame_above value_above
    exact ⟨finalState₂, finalShifted, final_above, control_above,
      .guardFalse (arms := arms) (guard_eq := guard_eq) (bind_eq := by rw [bindPattern_shift, bind_eq]; rfl)
        (guard_step := by simpa using guard_step₂) (tail_step := tail_step₂)⟩
  case assignValue =>
    intro namespaceId frame state exprId ns expression place child childFrame childState resolved
      finalFrame finalState runtimeValue namespace_eq expression_eq kind_eq child_step resolve_eq
      write_eq ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨childState₂, childShifted, child_above, value_above, child_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, write₂, finalShifted, final_above⟩ :=
      writeRuntimePlace?_mirror childShifted child_above value_above write_eq
    refine ⟨finalState₂, finalShifted, final_above, by simp [Control.Above, RuntimeValue.Above], ?_⟩
    rw [Control.shift_value, RuntimeValue.shift_unit]
    exact .assignValue (namespace_eq := namespace_eq) (expression_eq := expression_eq)
      (kind_eq := kind_eq) (child_step := child_step₂)
      (resolve_eq := by rw [resolvePlace?_shift unit ns childShifted.globals]; exact resolve_eq)
      (write_eq := write₂)
  case assignMismatch =>
    intro namespaceId frame state exprId ns expression place child childFrame childState
      runtimeValue kind thrown namespace_eq expression_eq kind_eq child_step resolve_eq mismatch
      mismatch_eq ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨childState₂, childShifted, child_above, -, child_step₂⟩ :=
      ih offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨shift_eq, above⟩ := patternMismatchThrow_shift mismatch_eq offset frontier
    refine ⟨childState₂, childShifted, child_above, above, ?_⟩
    rw [shift_eq]
    exact .assignMismatch (namespace_eq := namespace_eq) (expression_eq := expression_eq)
      (kind_eq := kind_eq) (child_step := child_step₂)
      (resolve_eq := by rw [resolvePlace?_shift unit ns childShifted.globals]; exact resolve_eq)
      (mismatch := by
        rw [placeVariantMismatchFuel?_shift unit ns childShifted.globals]; exact mismatch)
      (mismatch_eq := mismatch_eq)
  case constantValue =>
    intro namespaceId frame state exprId ns expression reference handle targetNs declaration
      targetFrame finalState runtimeValue namespace_eq expression_eq kind_eq resolve_eq
      target_namespace_eq declaration_eq initializer ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, -, value_above, initializer₂⟩ :=
      ih offset frontier inert inert' state₂ shifted (RuntimeFrame.empty_above frontier)
    rw [RuntimeFrame.shift_empty] at initializer₂
    exact ⟨finalState₂, finalShifted, frame_above, value_above,
      .constantValue (frame := frame.shift offset) (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (resolve_eq := resolve_eq) (target_namespace_eq := target_namespace_eq)
        (declaration_eq := declaration_eq) (initializer := initializer₂)⟩
  case constantControl =>
    intro namespaceId frame state exprId ns expression reference handle targetNs declaration
      targetFrame finalState control namespace_eq expression_eq kind_eq resolve_eq
      target_namespace_eq declaration_eq initializer abrupt ih
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, -, control_above, initializer₂⟩ :=
      ih offset frontier inert inert' state₂ shifted (RuntimeFrame.empty_above frontier)
    rw [RuntimeFrame.shift_empty] at initializer₂
    exact ⟨finalState₂, finalShifted, frame_above, control_above,
      .constantControl (frame := frame.shift offset) (namespace_eq := namespace_eq) (expression_eq := expression_eq)
        (kind_eq := kind_eq) (resolve_eq := resolve_eq) (target_namespace_eq := target_namespace_eq)
        (declaration_eq := declaration_eq) (initializer := initializer₂)
        (abrupt := abrupt.shift offset)⟩
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
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨argumentState₂, argumentShifted, argument_above, values_above, operands₂⟩ :=
      ih_operands offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, outcome_above, callee₂⟩ :=
      ih_callee offset frontier inert inert' argumentState₂ argumentShifted
        (fun value member => values_above value (by simpa using member))
    have pending := applyPendingFrom_mirror argument_above argumentShifted finalShifted
    generalize pending_eq : applyPendingFrom argumentState₂.pending (argumentFrame.shift offset)
      finalState₂ = pending₂ at pending
    obtain ⟨pendingFrame₂, pendingState₂⟩ := pending₂
    obtain ⟨rfl, pendingShifted, pending_above⟩ := pending
    have results_above : ∀ value ∈ results, value.Above frontier := outcome_above
    refine ⟨pendingState₂, pendingShifted, registerReturnedLoan_above pending_above results_above,
      packResults_above results_above, ?_⟩
    rw [← registerReturnedLoan_shift, Control.shift_value, ← packResults_shift]
    have step := EvalNodeWith.callReturned (executable := executable) (callee := EvalFunction executable)
      (frame := frame.shift offset) (state := state₂) (exprId := exprId)
      (namespace_eq := namespace_eq) (expression_eq := expression_eq) (kind_eq := kind_eq)
      (operands := operands₂) (resolve_eq := resolve_eq)
      (calleeStep := by simpa [List.map_toArray] using callee₂)
    rw [pending_eq] at step
    exact step
  case callThrew =>
    intro namespaceId frame state exprId ns expression reference instantiations arguments surface
      argumentState argumentFrame values handle finalState kind thrown namespace_eq expression_eq
      kind_eq operands resolve_eq calleeStep ih_operands ih_callee
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨argumentState₂, argumentShifted, argument_above, values_above, operands₂⟩ :=
      ih_operands offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, outcome_above, callee₂⟩ :=
      ih_callee offset frontier inert inert' argumentState₂ argumentShifted
        (fun value member => values_above value (by simpa using member))
    have pending := applyPendingFrom_mirror argument_above argumentShifted finalShifted
    generalize pending_eq : applyPendingFrom argumentState₂.pending (argumentFrame.shift offset)
      finalState₂ = pending₂ at pending
    obtain ⟨pendingFrame₂, pendingState₂⟩ := pending₂
    obtain ⟨rfl, pendingShifted, pending_above⟩ := pending
    refine ⟨pendingState₂, pendingShifted, pending_above, outcome_above, ?_⟩
    have step := EvalNodeWith.callThrew (executable := executable) (callee := EvalFunction executable)
      (frame := frame.shift offset) (state := state₂) (exprId := exprId)
      (namespace_eq := namespace_eq) (expression_eq := expression_eq) (kind_eq := kind_eq)
      (operands := operands₂) (resolve_eq := resolve_eq)
      (calleeStep := by simpa [List.map_toArray] using callee₂)
    rw [pending_eq] at step
    exact step
  case constructorValue =>
    intro namespaceId frame state exprId ns expression reference variant instantiations arguments
      surface finalState finalFrame values runtimeValue namespace_eq expression_eq kind_eq operands
      construct_eq ih_operands
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, values_above, operands₂⟩ :=
      ih_operands offset frontier inert inert' state₂ shifted frame_above
    refine ⟨finalState₂, finalShifted, final_above,
      constructNominal?_above (fun value member => values_above value (by simpa using member))
        construct_eq, ?_⟩
    have construct₂ := constructNominal?_shift (offset := offset) unit namespaceId reference
      variant values.toArray
    rw [construct_eq, Option.map_some] at construct₂
    exact .constructorValue _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
      operands₂ (by simpa [List.map_toArray] using construct₂)
  case destructorValue =>
    intro namespaceId frame state exprId ns expression reference variant instantiations arguments
      surface finalState finalFrame value fields namespace_eq expression_eq kind_eq operands
      destruct_eq ih_operands
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, values_above, operands₂⟩ :=
      ih_operands offset frontier inert inert' state₂ shifted frame_above
    have fields_above := destructNominal?_above (values_above value List.mem_cons_self) destruct_eq
    refine ⟨finalState₂, finalShifted, final_above, packResults_above fields_above, ?_⟩
    have destruct₂ := destructNominal?_shift (offset := offset) unit namespaceId reference
      variant value
    rw [destruct_eq, Option.map_some] at destruct₂
    rw [Control.shift_value, ← packResults_shift]
    exact .destructorValue _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
      operands₂ destruct₂
  case closureValue =>
    intro namespaceId frame state exprId ns expression reference mask instantiations captures
      surface finalState finalFrame values handle namespace_eq expression_eq kind_eq operands
      resolve_eq ih_operands
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, values_above, operands₂⟩ :=
      ih_operands offset frontier inert inert' state₂ shifted frame_above
    refine ⟨finalState₂, finalShifted, final_above, ?_, ?_⟩
    · simpa [Control.Above] using values_above
    · have step := EvalNodeWith.closureValue (executable := executable) (callee := EvalFunction executable)
        (frame := frame.shift offset) (state := state₂) (exprId := exprId)
        (namespace_eq := namespace_eq) (expression_eq := expression_eq) (kind_eq := kind_eq)
        (operands := operands₂) (resolve_eq := resolve_eq)
      simpa [List.map_toArray, RuntimeFrame.shift] using step
  case invokeReturned =>
    intro namespaceId frame state exprId ns expression instantiations arguments surface
      argumentState argumentFrame handle mask typeInstantiation captures values composed finalState
      results namespace_eq expression_eq kind_eq operands compose_eq calleeStep ih_operands
      ih_callee
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨argumentState₂, argumentShifted, argument_above, values_above, operands₂⟩ :=
      ih_operands offset frontier inert inert' state₂ shifted frame_above
    have closure_above := values_above _ List.mem_cons_self
    rw [RuntimeValue.above_closure] at closure_above
    have composed_above : ∀ value ∈ composed.toArray, value.Above frontier := by
      intro value member
      rcases ClosureMask.compose_mem compose_eq value (by simpa using member) with member | member
      · exact closure_above value (by simpa using member)
      · exact values_above value (List.mem_cons_of_mem _ member)
    obtain ⟨finalState₂, finalShifted, outcome_above, callee₂⟩ :=
      ih_callee offset frontier inert inert' argumentState₂ argumentShifted composed_above
    have pending := applyPendingFrom_mirror argument_above argumentShifted finalShifted
    generalize pending_eq : applyPendingFrom argumentState₂.pending (argumentFrame.shift offset)
      finalState₂ = pending₂ at pending
    obtain ⟨pendingFrame₂, pendingState₂⟩ := pending₂
    obtain ⟨rfl, pendingShifted, pending_above⟩ := pending
    have results_above : ∀ value ∈ results, value.Above frontier := outcome_above
    refine ⟨pendingState₂, pendingShifted, registerReturnedLoan_above pending_above results_above,
      packResults_above results_above, ?_⟩
    rw [← registerReturnedLoan_shift, Control.shift_value, ← packResults_shift]
    have compose₂ := ClosureMask.compose_map (·.shift offset) mask captures.toList values
    rw [compose_eq, Option.map_some] at compose₂
    have step := EvalNodeWith.invokeReturned (executable := executable) (callee := EvalFunction executable)
      (frame := frame.shift offset) (state := state₂) (exprId := exprId)
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
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨argumentState₂, argumentShifted, argument_above, values_above, operands₂⟩ :=
      ih_operands offset frontier inert inert' state₂ shifted frame_above
    have closure_above := values_above _ List.mem_cons_self
    rw [RuntimeValue.above_closure] at closure_above
    have composed_above : ∀ value ∈ composed.toArray, value.Above frontier := by
      intro value member
      rcases ClosureMask.compose_mem compose_eq value (by simpa using member) with member | member
      · exact closure_above value (by simpa using member)
      · exact values_above value (List.mem_cons_of_mem _ member)
    obtain ⟨finalState₂, finalShifted, outcome_above, callee₂⟩ :=
      ih_callee offset frontier inert inert' argumentState₂ argumentShifted composed_above
    have pending := applyPendingFrom_mirror argument_above argumentShifted finalShifted
    generalize pending_eq : applyPendingFrom argumentState₂.pending (argumentFrame.shift offset)
      finalState₂ = pending₂ at pending
    obtain ⟨pendingFrame₂, pendingState₂⟩ := pending₂
    obtain ⟨rfl, pendingShifted, pending_above⟩ := pending
    refine ⟨pendingState₂, pendingShifted, pending_above, outcome_above, ?_⟩
    have compose₂ := ClosureMask.compose_map (·.shift offset) mask captures.toList values
    rw [compose_eq, Option.map_some] at compose₂
    have step := EvalNodeWith.invokeThrew (executable := executable) (callee := EvalFunction executable)
      (frame := frame.shift offset) (state := state₂) (exprId := exprId)
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
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, -, operands₂⟩ :=
      ih_operands offset frontier inert inert' state₂ shifted frame_above
    have plain := (evaluateProfileOperation?_plain evaluate_eq).1 runtimeValue rfl
    refine ⟨finalState₂, finalShifted, final_above, RuntimeValue.above_of_plain frontier plain, ?_⟩
    rw [Control.shift_value, RuntimeValue.shift_of_plain offset plain]
    exact .profileValue _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq operands₂
      (by rw [← List.map_toArray, evaluateProfileOperation?_shift]; exact evaluate_eq)
  case profileThrow =>
    intro namespaceId frame state exprId ns expression operation targets instantiations arguments
      surface finalState finalFrame values kind thrown namespace_eq expression_eq kind_eq operands
      evaluate_eq ih_operands
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, -, operands₂⟩ :=
      ih_operands offset frontier inert inert' state₂ shifted frame_above
    have plain := (evaluateProfileOperation?_plain evaluate_eq).2 kind thrown rfl
    refine ⟨finalState₂, finalShifted, final_above,
      fun value member => RuntimeValue.above_of_plain frontier (plain value member), ?_⟩
    have thrown_eq : thrown.map (·.shift offset) = thrown := by
      conv => rhs; rw [← Array.map_id thrown]
      exact Array.map_congr_left (fun value member => RuntimeValue.shift_of_plain offset
        (plain value member))
    rw [Control.shift_throw, thrown_eq]
    exact .profileThrow _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
      operands₂ (by rw [← List.map_toArray, evaluateProfileOperation?_shift]; exact evaluate_eq)
  case assertTrue =>
    intro namespaceId frame state exprId ns expression instantiations arguments surface finalState
      finalFrame namespace_eq expression_eq kind_eq operands ih_operands
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, -, operands₂⟩ :=
      ih_operands offset frontier inert inert' state₂ shifted frame_above
    refine ⟨finalState₂, finalShifted, final_above, by simp [Control.Above, RuntimeValue.Above], ?_⟩
    rw [Control.shift_value, RuntimeValue.shift_unit]
    exact .assertTrue _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
      (by simpa using operands₂)
  case assertFalse =>
    intro namespaceId frame state exprId ns expression instantiations arguments surface finalState
      finalFrame namespace_eq expression_eq kind_eq operands ih_operands
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, -, operands₂⟩ :=
      ih_operands offset frontier inert inert' state₂ shifted frame_above
    refine ⟨finalState₂, finalShifted, final_above, fun _ member => by simp at member, ?_⟩
    rw [Control.shift_throw, Array.map_empty]
    exact .assertFalse _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
      (by simpa using operands₂)
  case primitiveValue =>
    intro namespaceId frame state exprId ns expression operation instantiations arguments surface
      finalState finalFrame values runtimeValue namespace_eq expression_eq kind_eq operands
      evaluate_eq ih_operands
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, values_above, operands₂⟩ :=
      ih_operands offset frontier inert inert' state₂ shifted frame_above
    refine ⟨finalState₂, finalShifted, final_above,
      (evaluatePrimitiveOperation?_above (fun value member => values_above value (by simpa using member))
        evaluate_eq).1 runtimeValue rfl, ?_⟩
    have evaluate₂ := evaluatePrimitiveOperation?_shift offset ns expression.typeId operation
      values.toArray executable.targetPointerWidth
    rw [evaluate_eq, Option.map_some, shiftResult_ok] at evaluate₂
    exact .primitiveValue _ _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq operands₂
      (by rw [← List.map_toArray]; exact evaluate₂)
  case primitiveThrow =>
    intro namespaceId frame state exprId ns expression operation instantiations arguments surface
      finalState finalFrame values kind thrown namespace_eq expression_eq kind_eq operands
      evaluate_eq ih_operands
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, finalShifted, final_above, values_above, operands₂⟩ :=
      ih_operands offset frontier inert inert' state₂ shifted frame_above
    refine ⟨finalState₂, finalShifted, final_above,
      (evaluatePrimitiveOperation?_above (fun value member => values_above value (by simpa using member))
        evaluate_eq).2 kind thrown rfl, ?_⟩
    have evaluate₂ := evaluatePrimitiveOperation?_shift offset ns expression.typeId operation
      values.toArray executable.targetPointerWidth
    rw [evaluate_eq, Option.map_some, shiftResult_error] at evaluate₂
    rw [Control.shift_throw]
    exact .primitiveThrow _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq operands₂
      (by rw [← List.map_toArray]; exact evaluate₂)
  case globalValue =>
    intro namespaceId frame state exprId ns expression kind instantiations arguments surface
      argumentState argumentFrame values finalFrame finalState runtimeValue namespace_eq
      expression_eq kind_eq operands evaluate_eq ih_operands
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨argumentState₂, argumentShifted, argument_above, values_above, operands₂⟩ :=
      ih_operands offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨result₂, evaluate₂, mirrors⟩ := evaluateGlobalOperation?_mirror argumentShifted
      argument_above (fun value member => values_above value (by simpa using member)) evaluate_eq
    cases result₂ with
    | value finalFrame₂ finalState₂ runtimeValue₂ =>
        obtain ⟨⟨rfl, finalShifted, final_above⟩, rfl, value_above⟩ := mirrors
        refine ⟨finalState₂, finalShifted, final_above, value_above, ?_⟩
        exact .globalValue _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
          operands₂ (by rw [← List.map_toArray]; exact evaluate₂)
    | throw_ => cases mirrors
  case globalThrow =>
    intro namespaceId frame state exprId ns expression globalKind instantiations arguments surface
      argumentState argumentFrame values finalFrame finalState throwKind thrown namespace_eq
      expression_eq kind_eq operands evaluate_eq ih_operands
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨argumentState₂, argumentShifted, argument_above, values_above, operands₂⟩ :=
      ih_operands offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨result₂, evaluate₂, mirrors⟩ := evaluateGlobalOperation?_mirror argumentShifted
      argument_above (fun value member => values_above value (by simpa using member)) evaluate_eq
    cases result₂ with
    | value => cases mirrors
    | throw_ finalFrame₂ finalState₂ throwKind₂ thrown₂ =>
        obtain ⟨⟨rfl, finalShifted, final_above⟩, rfl, rfl, thrown_above⟩ := mirrors
        refine ⟨finalState₂, finalShifted, final_above, thrown_above, ?_⟩
        exact .globalThrow _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
          operands₂ (by rw [← List.map_toArray]; exact evaluate₂)
  case operationValue =>
    intro namespaceId frame state exprId ns expression operation instantiations arguments surface
      argumentState argumentFrame values finalFrame finalState runtimeValue namespace_eq
      expression_eq kind_eq operands evaluate_eq ih_operands
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨argumentState₂, argumentShifted, argument_above, values_above, operands₂⟩ :=
      ih_operands offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨finalState₂, evaluate₂, finalShifted, final_above, value_above⟩ :=
      evaluatePlaceOperation?_mirror argumentShifted argument_above
        (fun value member => values_above value (by simpa using member)) evaluate_eq
    refine ⟨finalState₂, finalShifted, final_above, value_above, ?_⟩
    exact .operationValue _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
      operands₂ (by rw [← List.map_toArray]; exact evaluate₂)
  case operationMismatch =>
    intro namespaceId frame state exprId ns expression operation instantiations arguments surface
      argumentState argumentFrame values kind thrown namespace_eq expression_eq kind_eq operands
      evaluate_eq mismatch mismatch_eq ih_operands
    intro offset frontier inert inert' state₂ shifted frame_above
    obtain ⟨argumentState₂, argumentShifted, argument_above, -, operands₂⟩ :=
      ih_operands offset frontier inert inert' state₂ shifted frame_above
    have mismatch₂ : variantMismatch? unit ns operation
        ((values.map (·.shift offset)).toArray) (argumentFrame.shift offset) argumentState₂ =
          true := by
      rw [← List.map_toArray, variantMismatch?_shift unit ns operation _ argumentShifted.globals]
      exact mismatch
    obtain ⟨shift_eq, above⟩ := patternMismatchThrow_shift mismatch_eq offset frontier
    refine ⟨argumentState₂, argumentShifted, argument_above, above, ?_⟩
    rw [shift_eq]
    exact .operationMismatch _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq expression_eq kind_eq
      operands₂ (variantMismatch?_evaluate_none mismatch₂) mismatch₂ mismatch_eq
  case body =>
    intro handle typeInstantiation initialState arguments ns declaration frame root finalFrame
      evaluatedState finalState control outcome namespace_eq declaration_eq frame_eq body_eq
      body_step outcome_eq hole_free finalize_eq ih_body
    intro offset frontier inert inert' state₂ shifted arguments_above
    have frame_above := initialFrame?_above arguments_above frame_eq
    obtain ⟨evaluated₂, evaluatedShifted, final_above, control_above, body₂⟩ :=
      ih_body offset frontier inert inert' state₂ shifted frame_above
    refine ⟨finalizeFunctionState executable declaration.profile state₂ evaluated₂
        (finalFrame.shift offset) (outcome.shift offset), ?_,
      finishControl?_above control_above outcome_eq, ?_⟩
    · rw [← finalize_eq]
      exact finalizeFunctionState_mirror shifted evaluatedShifted executable declaration.profile
        final_above outcome
    · exact .body _ _ _ _ _ _ _ _ _ _ _ _ _ namespace_eq declaration_eq
        (by rw [initialFrame?_shift, frame_eq]; rfl) body_eq body₂
        (by rw [finishControl?_shift, outcome_eq]; rfl)
        (by rw [Outcome.holeFree_shift]; exact hole_free) rfl
  case native =>
    intro handle typeInstantiation initialState arguments ns declaration finalState outcome
      namespace_eq declaration_eq arity_eq body_eq native_eq hole_free
    intro offset frontier inert inert' state₂ shifted arguments_above
    obtain ⟨finalState₂, finalShifted, outcome_above, native₂⟩ :=
      natives _ _ _ _ _ _ native_eq offset frontier inert inert' state₂ shifted arguments_above
    exact ⟨finalState₂, finalShifted, outcome_above,
      .native _ _ _ _ _ _ _ _ namespace_eq declaration_eq (by rw [Array.size_map]; exact arity_eq)
        body_eq native₂ (by rw [Outcome.holeFree_shift]; exact hole_free)⟩

theorem Array.map_shift_of_plain {values : Array RuntimeValue} (offset : Nat)
    (plain : ∀ value ∈ values, Plain value) : values.map (·.shift offset) = values := by
  conv => rhs; rw [← Array.map_id values]
  exact Array.map_congr_left fun value member => RuntimeValue.shift_of_plain offset (plain value member)

theorem GlobalMap.shift_of_plain {globals : GlobalMap} (offset : Nat)
    (plain : ∀ slot ∈ globals.entries, Plain slot.value) : globals.shift offset = globals := by
  obtain ⟨entries⟩ := globals
  simp only [GlobalMap.shift, GlobalMap.mk.injEq]
  conv => rhs; rw [← Array.map_id entries]
  exact Array.map_congr_left fun slot member => by
    simp [RuntimeValue.shift_of_plain offset (plain slot member)]

theorem SemanticOperations.FreshGlobalLoanIds.below {state : RuntimeState} (fresh : FreshGlobalLoanIds state) :
    ∀ entry ∈ state.globalLoans, entry.1 < state.nextLoan := by
  intro entry member
  refine Nat.lt_of_not_le fun beyond => ?_
  have absent := fresh entry.1 beyond
  simp only [globalLoanKeyIn?, Option.map_eq_none_iff, List.find?_eq_none] at absent
  exact absent entry member (by simp)

/-- Starts with the same global memory, holding no loan, and fresh loan
registries mirror each other: the second's frontier is raised by the
difference. -/
theorem StateShifted.ofStarts {start start₂ : RuntimeState}
    (globals_eq : start₂.globals = start.globals)
    (plain : ∀ slot ∈ start.globals.entries, Plain slot.value)
    (fresh : FreshGlobalLoanIds start) (fresh₂ : FreshGlobalLoanIds start₂)
    (le : start.nextLoan ≤ start₂.nextLoan) :
    StateShifted (start₂.nextLoan - start.nextLoan) start.nextLoan start.pending.size
      start₂.pending.size start start₂ where
  globals := by rw [globals_eq, GlobalMap.shift_of_plain _ plain]
  nextLoan := by omega
  frontier_le := Nat.le_refl _
  inert_le := Nat.le_refl _
  inert_le' := Nat.le_refl _
  pending := by simp
  registry := ⟨[], start.globalLoans, start₂.globalLoans, rfl, rfl, by simp, fresh.below,
    fun entry member => by have := fresh₂.below entry member; omega⟩
  globalsAbove := fun slot member => RuntimeValue.above_of_plain _ (plain slot member)
  pendingAbove := by simp

/-- Loan independence: a run from a start whose global memory holds no loan,
on arguments holding none, is mirrored from every start with that memory and
a fresh registry at a frontier no lower, with its loans raised by the
frontiers' difference. -/
theorem runs_mirror {unit : ValidatedUnit} {executable : ExecutableUnit unit}
    (natives : NativesShift executable)
    {handle : FunctionHandle} {instantiation : Array (TypeId × TypeId)}
    {start start₂ final : RuntimeState} {arguments : Array RuntimeValue} {outcome : Outcome}
    (run : EvalFunction executable handle instantiation start arguments final outcome)
    (globals_eq : start₂.globals = start.globals)
    (plain : ∀ slot ∈ start.globals.entries, Plain slot.value)
    (fresh : FreshGlobalLoanIds start) (fresh₂ : FreshGlobalLoanIds start₂)
    (le : start.nextLoan ≤ start₂.nextLoan)
    (arguments_plain : ∀ argument ∈ arguments, Plain argument) :
    ∃ final₂, EvalFunction executable handle instantiation start₂ arguments final₂
        (outcome.shift (start₂.nextLoan - start.nextLoan)) ∧
      final₂.globals = final.globals.shift (start₂.nextLoan - start.nextLoan) := by
  obtain ⟨final₂, finalShifted, -, run₂⟩ :=
    shifts natives run _ _ _ _ start₂ (StateShifted.ofStarts globals_eq plain fresh fresh₂ le)
      fun argument member => RuntimeValue.above_of_plain _ (arguments_plain argument member)
  rw [Array.map_shift_of_plain _ arguments_plain] at run₂
  exact ⟨final₂, run₂, finalShifted.globals⟩

end LeanerIR
