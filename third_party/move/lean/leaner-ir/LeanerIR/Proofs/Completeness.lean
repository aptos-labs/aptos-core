-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Fuel

/-!
# Interpreter completeness up to fuel

Every fuel-free big-step derivation is reproduced by the fuelled interpreter
with sufficient fuel, inverting `run_sound`. Because the interpreter is a
function, determinism of the big-step relation follows.
-/

namespace LeanerIR.Proofs.Completeness

open LeanerIR.Interpreter
open LeanerIR.Validation
open LeanerIR.BigStep
open SemanticOperations
open LeanerIR.Proofs.Fuel

private def valuesResult : Internal.ValuesEvaluation → ValuesResult
  | .values state frame values => .values state frame values
  | .control state frame control => .control state frame control.value

private def statementsResult : Internal.StatementsEvaluation → StatementsResult
  | .done state frame => .done state frame
  | .control state frame control => .control state frame control.value

/-- Completeness statement for one expression judgment. -/
private def ExprComplete (unit : ExecutableUnit) (namespaceId : LeanerIR.NamespaceId)
    (frame : LeanerIR.RuntimeFrame) (state : LeanerIR.RuntimeState)
    (exprId : LeanerIR.ExprId) (finalFrame : LeanerIR.RuntimeFrame)
    (finalState : LeanerIR.RuntimeState) (control : LeanerIR.Control) : Prop :=
  ∃ fuel result,
    Internal.evalExpr fuel unit namespaceId frame state exprId = .ok result ∧
      result.frame = finalFrame ∧ result.state = finalState ∧
      result.control.value = control

private def FunctionComplete (unit : ExecutableUnit) (handle : LeanerIR.FunctionHandle)
    (typeInstantiation : Array (LeanerIR.TypeId × LeanerIR.TypeId))
    (state : LeanerIR.RuntimeState) (arguments : Array LeanerIR.RuntimeValue)
    (finalState : LeanerIR.RuntimeState) (outcome : LeanerIR.Outcome) : Prop :=
  ∃ fuel result,
    Internal.evalFunction fuel unit handle typeInstantiation state arguments = .ok result ∧
      result.state = finalState ∧ result.outcome.value = outcome

private def ValuesComplete (unit : ExecutableUnit) (namespaceId : LeanerIR.NamespaceId)
    (frame : LeanerIR.RuntimeFrame) (state : LeanerIR.RuntimeState)
    (expressions : List LeanerIR.ExprId) (expected : ValuesResult) : Prop :=
  ∃ fuel result,
    Internal.evalValues fuel unit namespaceId frame state expressions = .ok result ∧
      valuesResult result = expected

private def StatementsComplete (unit : ExecutableUnit) (namespaceId : LeanerIR.NamespaceId)
    (frame : LeanerIR.RuntimeFrame) (state : LeanerIR.RuntimeState)
    (statements : List LeanerIR.ExprId) (expected : StatementsResult) : Prop :=
  ∃ fuel result,
    Internal.evalStatements fuel unit namespaceId frame state statements = .ok result ∧
      statementsResult result = expected

private def ArmsComplete (unit : ExecutableUnit) (namespaceId : LeanerIR.NamespaceId)
    (ns : ValidatedNamespace) (frame : LeanerIR.RuntimeFrame)
    (state : LeanerIR.RuntimeState) (value : LeanerIR.RuntimeValue)
    (arms : List LeanerIR.MatchArm) (finalFrame : LeanerIR.RuntimeFrame)
    (finalState : LeanerIR.RuntimeState) (control : LeanerIR.Control) : Prop :=
  ∀ ownerLoc, ∃ fuel result,
    Internal.evalArms fuel unit namespaceId ns ownerLoc frame state value arms
        = .ok result ∧
      result.frame = finalFrame ∧ result.state = finalState ∧
      result.control.value = control

private theorem completeExpr {unit : ExecutableUnit} {namespaceId frame state exprId
    finalFrame finalState control}
    (h : EvalExpr unit namespaceId frame state exprId finalFrame finalState control) :
    ExprComplete unit namespaceId frame state exprId finalFrame finalState control := by
  apply BigStep.EvalFunction.rec_1
    (motive_1 := fun handle typeInstantiation state arguments finalState outcome _ =>
      FunctionComplete unit handle typeInstantiation state arguments finalState outcome)
    (motive_2 := fun namespaceId frame state exprId finalFrame finalState control _ =>
      ExprComplete unit namespaceId frame state exprId finalFrame finalState control)
    (motive_3 := fun namespaceId frame state expressions expected _ =>
      ValuesComplete unit namespaceId frame state expressions expected)
    (motive_4 := fun namespaceId frame state statements expected _ =>
      StatementsComplete unit namespaceId frame state statements expected)
    (motive_5 := fun namespaceId ns frame state value arms finalFrame finalState
        control _ =>
      ArmsComplete unit namespaceId ns frame state value arms finalFrame finalState
        control)
    (t := h)
  case value =>
    intro namespaceId frame state exprId ns expression literal source runtimeValue
      namespace_eq expression_eq kind_eq value_eq
    refine ⟨1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, value_eq, bind, Except.bind]
    exact ⟨_, rfl, rfl, rfl, rfl⟩
  case constantValue =>
    intro namespaceId frame state exprId ns expression reference handle targetNs
      declaration targetFrame finalState runtimeValue namespace_eq expression_eq kind_eq
      resolve_eq target_namespace_eq declaration_eq initializer ih
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, resolve_eq, target_namespace_eq,
      declaration_eq, e₀, bind, Except.bind, rc]
    exact ⟨_, rfl, rfl, rs, rfl⟩
  case constantControl =>
    intro namespaceId frame state exprId ns expression reference handle targetNs
      declaration targetFrame finalState control namespace_eq expression_eq kind_eq
      resolve_eq target_namespace_eq declaration_eq initializer abrupt ih
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, resolve_eq, target_namespace_eq,
      declaration_eq, e₀, bind, Except.bind]
    cases abrupt <;> simp only [rc] <;>
      exact ⟨_, rfl, rfl, rs, by simp [Located.pushCaller, rc]⟩
  case localVar =>
    intro namespaceId frame state exprId ns expression localId runtimeValue
      namespace_eq expression_eq kind_eq local_eq
    refine ⟨1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, local_eq, bind, Except.bind]
    exact ⟨_, rfl, rfl, rfl, rfl⟩
  case callArgumentsControl =>
    intro namespaceId frame state exprId ns expression reference
      instantiations arguments surface finalState finalFrame control
      namespace_eq expression_eq kind_eq operands ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | values s f vs => simp [valuesResult] at er₀
    | control s f c =>
        simp only [valuesResult, ValuesResult.control.injEq] at er₀
        obtain ⟨es, ef, ec⟩ := er₀
        exact ⟨_, rfl, ef, es, ec⟩
  case callReturned =>
    intro namespaceId frame state exprId ns expression reference instantiations
      arguments surface argumentState argumentFrame values handle finalState results
      namespace_eq expression_eq kind_eq operands resolve_eq callee ih_operands ih_callee
    obtain ⟨f₁, r₁, e₁, er₁⟩ := ih_operands
    obtain ⟨f₂, r₂, e₂, rs₂, ro₂⟩ := ih_callee
    refine ⟨max f₁ f₂ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq,
      evalValues_mono (Nat.le_max_left f₁ f₂) e₁, bind, Except.bind]
    cases r₁ with
    | control s f c => simp [valuesResult] at er₁
    | values s f vs =>
        simp only [valuesResult, ValuesResult.values.injEq] at er₁
        obtain ⟨es, ef, ev⟩ := er₁
        simp only [es, ef, ev, resolve_eq,
          evalFunction_mono (Nat.le_max_right f₁ f₂) e₂, bind, Except.bind,
          pure, Except.pure, Internal.callResult, ro₂]
        subst rs₂
        exact ⟨_, rfl, rfl, rfl, rfl⟩
  case callThrew =>
    intro namespaceId frame state exprId ns expression reference instantiations
      arguments surface argumentState argumentFrame values handle finalState kind thrown
      namespace_eq expression_eq kind_eq operands resolve_eq callee ih_operands ih_callee
    obtain ⟨f₁, r₁, e₁, er₁⟩ := ih_operands
    obtain ⟨f₂, r₂, e₂, rs₂, ro₂⟩ := ih_callee
    refine ⟨max f₁ f₂ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq,
      evalValues_mono (Nat.le_max_left f₁ f₂) e₁, bind, Except.bind]
    cases r₁ with
    | control s f c => simp [valuesResult] at er₁
    | values s f vs =>
        simp only [valuesResult, ValuesResult.values.injEq] at er₁
        obtain ⟨es, ef, ev⟩ := er₁
        simp only [es, ef, ev, resolve_eq,
          evalFunction_mono (Nat.le_max_right f₁ f₂) e₂, bind, Except.bind,
          pure, Except.pure, Internal.callResult, ro₂]
        subst rs₂
        exact ⟨_, rfl, rfl, rfl, rfl⟩
  case constructorArgumentsControl =>
    intro namespaceId frame state exprId ns expression reference variant
      instantiations arguments surface finalState finalFrame control
      namespace_eq expression_eq kind_eq operands ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | values s f vs => simp [valuesResult] at er₀
    | control s f c =>
        simp only [valuesResult, ValuesResult.control.injEq] at er₀
        obtain ⟨es, ef, ec⟩ := er₀
        exact ⟨_, rfl, ef, es, ec⟩
  case constructorValue =>
    intro namespaceId frame state exprId ns expression reference variant
      instantiations arguments surface finalState finalFrame values runtimeValue
      namespace_eq expression_eq kind_eq operands construct_eq ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | control s f c => simp [valuesResult] at er₀
    | values s f vs =>
        simp only [valuesResult, ValuesResult.values.injEq] at er₀
        obtain ⟨es, ef, ev⟩ := er₀
        simp only [es, ef, ev, construct_eq, bind, Except.bind]
        exact ⟨_, rfl, rfl, rfl, rfl⟩
  case destructorArgumentsControl =>
    intro namespaceId frame state exprId ns expression reference variant
      instantiations arguments surface finalState finalFrame control
      namespace_eq expression_eq kind_eq operands ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | values s f vs => simp [valuesResult] at er₀
    | control s f c =>
        simp only [valuesResult, ValuesResult.control.injEq] at er₀
        obtain ⟨es, ef, ec⟩ := er₀
        exact ⟨_, rfl, ef, es, ec⟩
  case destructorValue =>
    intro namespaceId frame state exprId ns expression reference variant
      instantiations arguments surface finalState finalFrame value fields
      namespace_eq expression_eq kind_eq operands destruct_eq ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | control s f c => simp [valuesResult] at er₀
    | values s f vs =>
        simp only [valuesResult, ValuesResult.values.injEq] at er₀
        obtain ⟨es, ef, ev⟩ := er₀
        simp only [es, ef, ev, destruct_eq, bind, Except.bind]
        exact ⟨_, rfl, rfl, rfl, rfl⟩
  case closureArgumentsControl =>
    intro namespaceId frame state exprId ns expression reference
      instantiations captures surface finalState finalFrame control
      namespace_eq expression_eq kind_eq operands ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | values s f vs => simp [valuesResult] at er₀
    | control s f c =>
        simp only [valuesResult, ValuesResult.control.injEq] at er₀
        obtain ⟨es, ef, ec⟩ := er₀
        exact ⟨_, rfl, ef, es, ec⟩
  case closureValue =>
    intro namespaceId frame state exprId ns expression reference instantiations captures
      surface finalState finalFrame values handle
      namespace_eq expression_eq kind_eq operands resolve_eq ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | control s f c => simp [valuesResult] at er₀
    | values s f vs =>
        simp only [valuesResult, ValuesResult.values.injEq] at er₀
        obtain ⟨es, ef, ev⟩ := er₀
        simp only [es, ef, ev, resolve_eq, bind, Except.bind]
        exact ⟨_, rfl, rfl, rfl, rfl⟩
  case invokeArgumentsControl =>
    intro namespaceId frame state exprId ns expression instantiations arguments
      surface finalState finalFrame control
      namespace_eq expression_eq kind_eq operands ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | values s f vs => simp [valuesResult] at er₀
    | control s f c =>
        simp only [valuesResult, ValuesResult.control.injEq] at er₀
        obtain ⟨es, ef, ec⟩ := er₀
        exact ⟨_, rfl, ef, es, ec⟩
  case invokeReturned =>
    intro namespaceId frame state exprId ns expression instantiations arguments surface
      argumentState argumentFrame handle captures values finalState results
      namespace_eq expression_eq kind_eq operands callee ih_operands ih_callee
    obtain ⟨f₁, r₁, e₁, er₁⟩ := ih_operands
    obtain ⟨f₂, r₂, e₂, rs₂, ro₂⟩ := ih_callee
    refine ⟨max f₁ f₂ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq,
      evalValues_mono (Nat.le_max_left f₁ f₂) e₁, bind, Except.bind]
    cases r₁ with
    | control s f c => simp [valuesResult] at er₁
    | values s f vs =>
        simp only [valuesResult, ValuesResult.values.injEq] at er₁
        obtain ⟨es, ef, ev⟩ := er₁
        simp only [es, ef, ev,
          evalFunction_mono (Nat.le_max_right f₁ f₂) e₂, bind, Except.bind,
          pure, Except.pure, Internal.callResult, ro₂]
        subst rs₂
        exact ⟨_, rfl, rfl, rfl, rfl⟩
  case invokeThrew =>
    intro namespaceId frame state exprId ns expression instantiations arguments surface
      argumentState argumentFrame handle captures values finalState kind thrown
      namespace_eq expression_eq kind_eq operands callee ih_operands ih_callee
    obtain ⟨f₁, r₁, e₁, er₁⟩ := ih_operands
    obtain ⟨f₂, r₂, e₂, rs₂, ro₂⟩ := ih_callee
    refine ⟨max f₁ f₂ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq,
      evalValues_mono (Nat.le_max_left f₁ f₂) e₁, bind, Except.bind]
    cases r₁ with
    | control s f c => simp [valuesResult] at er₁
    | values s f vs =>
        simp only [valuesResult, ValuesResult.values.injEq] at er₁
        obtain ⟨es, ef, ev⟩ := er₁
        simp only [es, ef, ev,
          evalFunction_mono (Nat.le_max_right f₁ f₂) e₂, bind, Except.bind,
          pure, Except.pure, Internal.callResult, ro₂]
        subst rs₂
        exact ⟨_, rfl, rfl, rfl, rfl⟩
  case profileArgumentsControl =>
    intro namespaceId frame state exprId ns expression operation targets
      instantiations arguments surface finalState finalFrame control
      namespace_eq expression_eq kind_eq operands ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | values s f vs => simp [valuesResult] at er₀
    | control s f c =>
        simp only [valuesResult, ValuesResult.control.injEq] at er₀
        obtain ⟨es, ef, ec⟩ := er₀
        exact ⟨_, rfl, ef, es, ec⟩
  case profileValue =>
    intro namespaceId frame state exprId ns expression operation targets instantiations
      arguments surface finalState finalFrame values runtimeValue
      namespace_eq expression_eq kind_eq operands evaluate_eq ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | control s f c => simp [valuesResult] at er₀
    | values s f vs =>
        simp only [valuesResult, ValuesResult.values.injEq] at er₀
        obtain ⟨es, ef, ev⟩ := er₀
        simp only [es, ef, ev, evaluate_eq, bind, Except.bind]
        exact ⟨_, rfl, rfl, rfl, rfl⟩
  case profileThrow =>
    intro namespaceId frame state exprId ns expression operation targets instantiations
      arguments surface finalState finalFrame values kind thrown
      namespace_eq expression_eq kind_eq operands evaluate_eq ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | control s f c => simp [valuesResult] at er₀
    | values s f vs =>
        simp only [valuesResult, ValuesResult.values.injEq] at er₀
        obtain ⟨es, ef, ev⟩ := er₀
        simp only [es, ef, ev, evaluate_eq, bind, Except.bind]
        exact ⟨_, rfl, rfl, rfl, rfl⟩
  case primitiveArgumentsControl =>
    intro namespaceId frame state exprId ns expression operation
      instantiations arguments surface finalState finalFrame control
      namespace_eq expression_eq kind_eq operands ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | values s f vs => simp [valuesResult] at er₀
    | control s f c =>
        simp only [valuesResult, ValuesResult.control.injEq] at er₀
        obtain ⟨es, ef, ec⟩ := er₀
        exact ⟨_, rfl, ef, es, ec⟩
  case primitiveValue =>
    intro namespaceId frame state exprId ns expression operation instantiations
      arguments surface finalState finalFrame values runtimeValue
      namespace_eq expression_eq kind_eq operands evaluate_eq ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | control s f c => simp [valuesResult] at er₀
    | values s f vs =>
        simp only [valuesResult, ValuesResult.values.injEq] at er₀
        obtain ⟨es, ef, ev⟩ := er₀
        simp only [es, ef, ev, evaluate_eq, bind, Except.bind]
        exact ⟨_, rfl, rfl, rfl, rfl⟩
  case primitiveThrow =>
    intro namespaceId frame state exprId ns expression operation instantiations
      arguments surface finalState finalFrame values kind thrown
      namespace_eq expression_eq kind_eq operands evaluate_eq ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | control s f c => simp [valuesResult] at er₀
    | values s f vs =>
        simp only [valuesResult, ValuesResult.values.injEq] at er₀
        obtain ⟨es, ef, ev⟩ := er₀
        simp only [es, ef, ev, evaluate_eq, bind, Except.bind]
        exact ⟨_, rfl, rfl, rfl, rfl⟩
  case globalArgumentsControl =>
    intro namespaceId frame state exprId ns expression kind instantiations
      arguments surface finalState finalFrame control
      namespace_eq expression_eq kind_eq operands ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | values s f vs => simp [valuesResult] at er₀
    | control s f c =>
        simp only [valuesResult, ValuesResult.control.injEq] at er₀
        obtain ⟨es, ef, ec⟩ := er₀
        exact ⟨_, rfl, ef, es, ec⟩
  case globalValue =>
    intro namespaceId frame state exprId ns expression kind instantiations arguments
      surface argumentState argumentFrame values finalFrame finalState runtimeValue
      namespace_eq expression_eq kind_eq operands evaluate_eq ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | control s f c => simp [valuesResult] at er₀
    | values s f vs =>
        simp only [valuesResult, ValuesResult.values.injEq] at er₀
        obtain ⟨es, ef, ev⟩ := er₀
        simp only [es, ef, ev, evaluate_eq, bind, Except.bind]
        exact ⟨_, rfl, rfl, rfl, rfl⟩
  case globalThrow =>
    intro namespaceId frame state exprId ns expression globalKind instantiations arguments
      surface argumentState argumentFrame values finalFrame finalState throwKind thrown
      namespace_eq expression_eq kind_eq operands evaluate_eq ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | control s f c => simp [valuesResult] at er₀
    | values s f vs =>
        simp only [valuesResult, ValuesResult.values.injEq] at er₀
        obtain ⟨es, ef, ev⟩ := er₀
        simp only [es, ef, ev, evaluate_eq, bind, Except.bind]
        exact ⟨_, rfl, rfl, rfl, rfl⟩
  case assertArgumentsControl =>
    intro namespaceId frame state exprId ns expression instantiations arguments
      surface finalState finalFrame control
      namespace_eq expression_eq kind_eq operands ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | values s f vs => simp [valuesResult] at er₀
    | control s f c =>
        simp only [valuesResult, ValuesResult.control.injEq] at er₀
        obtain ⟨es, ef, ec⟩ := er₀
        exact ⟨_, rfl, ef, es, ec⟩
  case assertTrue =>
    intro namespaceId frame state exprId ns expression instantiations arguments surface
      finalState finalFrame
      namespace_eq expression_eq kind_eq operands ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | control s f c => simp [valuesResult] at er₀
    | values s f vs =>
        simp only [valuesResult, ValuesResult.values.injEq] at er₀
        obtain ⟨es, ef, ev⟩ := er₀
        simp only [es, ef, ev, bind, Except.bind]
        exact ⟨_, rfl, rfl, rfl, rfl⟩
  case assertFalse =>
    intro namespaceId frame state exprId ns expression instantiations arguments surface
      finalState finalFrame
      namespace_eq expression_eq kind_eq operands ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | control s f c => simp [valuesResult] at er₀
    | values s f vs =>
        simp only [valuesResult, ValuesResult.values.injEq] at er₀
        obtain ⟨es, ef, ev⟩ := er₀
        simp only [es, ef, ev, bind, Except.bind]
        exact ⟨_, rfl, rfl, rfl, rfl⟩
  case operationArgumentsControl =>
    intro namespaceId frame state exprId ns expression operation instantiations
      arguments surface finalState finalFrame control namespace_eq expression_eq kind_eq
      operands ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, bind, Except.bind]
    cases r₀ with
    | values s f vs =>
        exfalso
        cases operation <;> first
          | simp [valuesResult] at er₀
          | (rename_i kind; cases kind <;> simp [valuesResult] at er₀)
    | control s f c =>
        simp only [valuesResult, ValuesResult.control.injEq] at er₀
        obtain ⟨es, ef, ec⟩ := er₀
        cases operation
        case call kind =>
            cases kind <;> dsimp only <;> simp only [e₀, bind, Except.bind] <;>
              exact ⟨_, rfl, ef, es, ec⟩
        all_goals
          dsimp only
          simp only [e₀, bind, Except.bind]
          exact ⟨_, rfl, ef, es, ec⟩
  case operationValue =>
    intro namespaceId frame state exprId ns expression operation instantiations
      arguments surface argumentState argumentFrame values finalFrame finalState
      runtimeValue namespace_eq expression_eq kind_eq operands evaluate_eq ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, bind, Except.bind]
    cases r₀ with
    | values s f vs =>
        simp only [valuesResult, ValuesResult.values.injEq] at er₀
        obtain ⟨es, ef, ev⟩ := er₀
        cases operation
        case call kind => cases kind <;> simp [evaluatePlaceOperation?] at evaluate_eq
        case profile value targets => simp [evaluatePlaceOperation?] at evaluate_eq
        case primitive kind => simp [evaluatePlaceOperation?] at evaluate_eq
    -- The remaining operations reach the interpreter's shared place-operation
    -- arm; those the evaluator does not model refute `evaluate_eq` the same
    -- way once it reduces.
        case data kind =>
            simp only [evaluatePlaceOperation?, bind, Option.bind] at evaluate_eq
            cases data_eq : evaluateDataOperation? unit.unit ns.identity kind
                values.toArray with
            | none => simp [data_eq] at evaluate_eq
            | some value =>
                simp only [data_eq, Option.some.injEq, Prod.mk.injEq] at evaluate_eq
                obtain ⟨eff, efs, efv⟩ := evaluate_eq
                dsimp only
                simp only [e₀, bind, Except.bind, es, ef, ev, data_eq, eff, efs, efv]
                exact ⟨_, rfl, rfl, rfl, rfl⟩
        case global kind => simp [evaluatePlaceOperation?] at evaluate_eq
        case assert => simp [evaluatePlaceOperation?] at evaluate_eq
        case specification kind => simp [evaluatePlaceOperation?] at evaluate_eq
        case reference kind =>
            cases kind <;> dsimp only <;>
              first
              | (simp only [e₀, bind, Except.bind, es, ef, ev, evaluate_eq]
                 exact ⟨_, rfl, rfl, rfl, rfl⟩)
              | simp [evaluatePlaceOperation?] at evaluate_eq
        all_goals
          dsimp only
          simp only [e₀, bind, Except.bind, es, ef, ev, evaluate_eq]
          exact ⟨_, rfl, rfl, rfl, rfl⟩
    | control s f c =>
        exfalso
        cases operation <;> first
          | simp [valuesResult] at er₀
          | (rename_i kind; cases kind <;> simp [valuesResult] at er₀)
  case blockControl =>
    intro namespaceId frame state exprId ns expression statements result
      finalState finalFrame control namespace_eq expression_eq kind_eq steps ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | done s f => simp [statementsResult] at er₀
    | control s f c =>
        simp only [statementsResult, StatementsResult.control.injEq] at er₀
        obtain ⟨es, ef, ec⟩ := er₀
        exact ⟨_, rfl, ef, es, ec⟩
  case blockUnit =>
    intro namespaceId frame state exprId ns expression statements finalState finalFrame
      namespace_eq expression_eq kind_eq steps ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | control s f c => simp [statementsResult] at er₀
    | done s f =>
        simp only [statementsResult, StatementsResult.done.injEq] at er₀
        obtain ⟨es, ef⟩ := er₀
        simp only [es, ef]
        exact ⟨_, rfl, rfl, rfl, rfl⟩
  case blockResult =>
    intro namespaceId frame state exprId ns expression statements result
      statementState statementFrame finalState finalFrame control
      namespace_eq expression_eq kind_eq steps result_step ih_steps ih_result
    obtain ⟨f₁, r₁, e₁, er₁⟩ := ih_steps
    obtain ⟨f₂, r₂, e₂, rf₂, rs₂, rc₂⟩ := ih_result
    refine ⟨max f₁ f₂ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq,
      evalStatements_mono (Nat.le_max_left f₁ f₂) e₁, bind, Except.bind]
    cases r₁ with
    | control s f c => simp [statementsResult] at er₁
    | done s f =>
        simp only [statementsResult, StatementsResult.done.injEq] at er₁
        obtain ⟨es, ef⟩ := er₁
        simp only [es, ef, evalExpr_mono (Nat.le_max_right f₁ f₂) e₂,
          bind, Except.bind]
        cases control <;> simp only [rc₂] <;>
          first
          | exact ⟨_, rfl, rf₂, rs₂, rfl⟩
          | exact ⟨_, rfl, rf₂, rs₂, rc₂⟩
  case letNoValue =>
    intro namespaceId frame state exprId ns expression pattern body
      finalFrame finalState control namespace_eq expression_eq kind_eq body_step ih
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases control <;> simp only [rc] <;>
      first
      | exact ⟨_, rfl, rf, rs, rfl⟩
      | exact ⟨_, rfl, rf, rs, rc⟩
  case letValueControl =>
    intro namespaceId frame state exprId ns expression pattern initializer body
      finalFrame finalState control namespace_eq expression_eq kind_eq
      initializer_step abrupt ih
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases abrupt <;> simp only [rc] <;> exact ⟨_, rfl, rf, rs, rc⟩
  case letValue =>
    intro namespaceId frame state exprId ns expression pattern initializer body
      initializedFrame initializedState runtimeValue boundFrame finalFrame finalState
      control namespace_eq expression_eq kind_eq initializer_step bind_eq body_step
      ih_initializer ih_body
    obtain ⟨f₁, r₁, e₁, rf₁, rs₁, rc₁⟩ := ih_initializer
    obtain ⟨f₂, r₂, e₂, rf₂, rs₂, rc₂⟩ := ih_body
    refine ⟨max f₁ f₂ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq,
      evalExpr_mono (Nat.le_max_left f₁ f₂) e₁, bind, Except.bind, rc₁, rf₁, rs₁,
      bind_eq, evalExpr_mono (Nat.le_max_right f₁ f₂) e₂]
    cases control <;> simp only [rc₂] <;>
      first
      | exact ⟨_, rfl, rf₂, rs₂, rfl⟩
      | exact ⟨_, rfl, rf₂, rs₂, rc₂⟩
  case ifControl =>
    intro namespaceId frame state exprId ns expression condition thenBranch elseBranch
      finalFrame finalState control namespace_eq expression_eq kind_eq
      condition_step abrupt ih
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases abrupt <;> simp only [rc] <;> exact ⟨_, rfl, rf, rs, rc⟩
  case ifTrue =>
    intro namespaceId frame state exprId ns expression condition thenBranch elseBranch
      conditionFrame conditionState finalFrame finalState control
      namespace_eq expression_eq kind_eq condition_step branch_step
      ih_condition ih_branch
    obtain ⟨f₁, r₁, e₁, rf₁, rs₁, rc₁⟩ := ih_condition
    obtain ⟨f₂, r₂, e₂, rf₂, rs₂, rc₂⟩ := ih_branch
    refine ⟨max f₁ f₂ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq,
      evalExpr_mono (Nat.le_max_left f₁ f₂) e₁, bind, Except.bind, rc₁, rf₁, rs₁,
      evalExpr_mono (Nat.le_max_right f₁ f₂) e₂]
    cases control <;> simp only [rc₂] <;>
      first
      | exact ⟨_, rfl, rf₂, rs₂, rfl⟩
      | exact ⟨_, rfl, rf₂, rs₂, rc₂⟩
  case ifFalseUnit =>
    intro namespaceId frame state exprId ns expression condition thenBranch
      conditionFrame conditionState namespace_eq expression_eq kind_eq
      condition_step ih
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind, rc]
    exact ⟨_, rfl, rf, rs, rfl⟩
  case ifFalse =>
    intro namespaceId frame state exprId ns expression condition thenBranch elseBranch
      conditionFrame conditionState finalFrame finalState control
      namespace_eq expression_eq kind_eq condition_step branch_step
      ih_condition ih_branch
    obtain ⟨f₁, r₁, e₁, rf₁, rs₁, rc₁⟩ := ih_condition
    obtain ⟨f₂, r₂, e₂, rf₂, rs₂, rc₂⟩ := ih_branch
    refine ⟨max f₁ f₂ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq,
      evalExpr_mono (Nat.le_max_left f₁ f₂) e₁, bind, Except.bind, rc₁, rf₁, rs₁,
      evalExpr_mono (Nat.le_max_right f₁ f₂) e₂]
    cases control <;> simp only [rc₂] <;>
      first
      | exact ⟨_, rfl, rf₂, rs₂, rfl⟩
      | exact ⟨_, rfl, rf₂, rs₂, rc₂⟩
  case matchControl =>
    intro namespaceId frame state exprId ns expression scrutinee arms
      finalFrame finalState control namespace_eq expression_eq kind_eq
      scrutinee_step abrupt ih
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases abrupt <;> simp only [rc] <;> exact ⟨_, rfl, rf, rs, rc⟩
  case matchValue =>
    intro namespaceId frame state exprId ns expression scrutinee arms
      scrutineeFrame scrutineeState runtimeValue finalFrame finalState control
      namespace_eq expression_eq kind_eq scrutinee_step arm_step
      ih_scrutinee ih_arms
    obtain ⟨f₁, r₁, e₁, rf₁, rs₁, rc₁⟩ := ih_scrutinee
    obtain ⟨f₂, r₂, e₂, rf₂, rs₂, rc₂⟩ := ih_arms expression.loc
    refine ⟨max f₁ f₂ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq,
      evalExpr_mono (Nat.le_max_left f₁ f₂) e₁, bind, Except.bind, rc₁, rf₁, rs₁,
      evalArms_mono (Nat.le_max_right f₁ f₂) e₂]
    cases control <;> simp only [rc₂] <;>
      first
      | exact ⟨_, rfl, rf₂, rs₂, rfl⟩
      | exact ⟨_, rfl, rf₂, rs₂, rc₂⟩
  case loopRepeatValue =>
    intro namespaceId frame state exprId ns expression label body
      bodyFrame bodyState runtimeValue finalFrame finalState control
      namespace_eq expression_eq kind_eq body_step repeat_step ih_body ih_repeat
    obtain ⟨f₁, r₁, e₁, rf₁, rs₁, rc₁⟩ := ih_body
    obtain ⟨f₂, r₂, e₂, rf₂, rs₂, rc₂⟩ := ih_repeat
    refine ⟨max f₁ f₂ + 1, r₂, ?_, rf₂, rs₂, rc₂⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq,
      evalExpr_mono (Nat.le_max_left f₁ f₂) e₁, bind, Except.bind, rc₁, rf₁, rs₁]
    exact evalExpr_mono (Nat.le_max_right f₁ f₂) e₂
  case loopRepeatContinue =>
    intro namespaceId frame state exprId ns expression label body
      bodyFrame bodyState finalFrame finalState control
      namespace_eq expression_eq kind_eq body_step repeat_step ih_body ih_repeat
    obtain ⟨f₁, r₁, e₁, rf₁, rs₁, rc₁⟩ := ih_body
    obtain ⟨f₂, r₂, e₂, rf₂, rs₂, rc₂⟩ := ih_repeat
    refine ⟨max f₁ f₂ + 1, r₂, ?_, rf₂, rs₂, rc₂⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq,
      evalExpr_mono (Nat.le_max_left f₁ f₂) e₁, bind, Except.bind, rc₁, rf₁, rs₁]
    exact evalExpr_mono (Nat.le_max_right f₁ f₂) e₂
  case loopBreak =>
    intro namespaceId frame state exprId ns expression label body
      finalFrame finalState value namespace_eq expression_eq kind_eq body_step ih
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind, rc]
    exact ⟨_, rfl, rf, rs, rfl⟩
  case loopOuterBreak =>
    intro namespaceId frame state exprId ns expression label body
      finalFrame finalState nest value namespace_eq expression_eq kind_eq body_step ih
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind, rc]
    exact ⟨_, rfl, rf, rs, rfl⟩
  case loopOuterContinue =>
    intro namespaceId frame state exprId ns expression label body
      finalFrame finalState nest namespace_eq expression_eq kind_eq body_step ih
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind, rc]
    exact ⟨_, rfl, rf, rs, rfl⟩
  case loopReturn =>
    intro namespaceId frame state exprId ns expression label body
      finalFrame finalState values namespace_eq expression_eq kind_eq body_step ih
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind, rc]
    exact ⟨_, rfl, rf, rs, rc⟩
  case loopThrow =>
    intro namespaceId frame state exprId ns expression label body
      finalFrame finalState kind arguments namespace_eq expression_eq kind_eq
      body_step ih
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind, rc]
    exact ⟨_, rfl, rf, rs, rc⟩
  case breakNone =>
    intro namespaceId frame state exprId ns expression nest
      namespace_eq expression_eq kind_eq
    refine ⟨1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, bind, Except.bind]
    exact ⟨_, rfl, rfl, rfl, rfl⟩
  case breakControl =>
    intro namespaceId frame state exprId ns expression nest child
      finalFrame finalState control namespace_eq expression_eq kind_eq
      child_step abrupt ih
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases abrupt <;> simp only [rc] <;> exact ⟨_, rfl, rf, rs, rc⟩
  case breakValue =>
    intro namespaceId frame state exprId ns expression nest child
      finalFrame finalState runtimeValue namespace_eq expression_eq kind_eq
      child_step ih
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind, rc]
    exact ⟨_, rfl, rf, rs, rfl⟩
  case continue_ =>
    intro namespaceId frame state exprId ns expression nest
      namespace_eq expression_eq kind_eq
    refine ⟨1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, bind, Except.bind]
    exact ⟨_, rfl, rfl, rfl, rfl⟩
  case returnValues =>
    intro namespaceId frame state exprId ns expression values
      finalFrame finalState runtimeValues
      namespace_eq expression_eq kind_eq values_step ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | control s f c => simp [valuesResult] at er₀
    | values s f vs =>
        simp only [valuesResult, ValuesResult.values.injEq] at er₀
        obtain ⟨es, ef, ev⟩ := er₀
        simp only [es, ef, ev, bind, Except.bind]
        exact ⟨_, rfl, rfl, rfl, rfl⟩
  case returnControl =>
    intro namespaceId frame state exprId ns expression values
      finalFrame finalState control
      namespace_eq expression_eq kind_eq values_step ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | values s f vs => simp [valuesResult] at er₀
    | control s f c =>
        simp only [valuesResult, ValuesResult.control.injEq] at er₀
        obtain ⟨es, ef, ec⟩ := er₀
        exact ⟨_, rfl, ef, es, ec⟩
  case throwValues =>
    intro namespaceId frame state exprId ns expression kind arguments
      finalFrame finalState runtimeValues
      namespace_eq expression_eq kind_eq values_step ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | control s f c => simp [valuesResult] at er₀
    | values s f vs =>
        simp only [valuesResult, ValuesResult.values.injEq] at er₀
        obtain ⟨es, ef, ev⟩ := er₀
        simp only [es, ef, ev, bind, Except.bind]
        exact ⟨_, rfl, rfl, rfl, rfl⟩
  case throwControl =>
    intro namespaceId frame state exprId ns expression kind arguments
      finalFrame finalState control
      namespace_eq expression_eq kind_eq values_step ih
    obtain ⟨f₀, r₀, e₀, er₀⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases r₀ with
    | values s f vs => simp [valuesResult] at er₀
    | control s f c =>
        simp only [valuesResult, ValuesResult.control.injEq] at er₀
        obtain ⟨es, ef, ec⟩ := er₀
        exact ⟨_, rfl, ef, es, ec⟩
  case assignControl =>
    intro namespaceId frame state exprId ns expression place child
      finalFrame finalState control namespace_eq expression_eq kind_eq
      child_step abrupt ih
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases abrupt <;> simp only [rc] <;> exact ⟨_, rfl, rf, rs, rc⟩
  case assignValue =>
    intro namespaceId frame state exprId ns expression place child
      childFrame childState resolved finalFrame finalState runtimeValue
      namespace_eq expression_eq kind_eq child_step resolve_eq write_eq ih
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind, rc, rf, rs,
      resolve_eq, write_eq]
    exact ⟨_, rfl, rfl, rfl, rfl⟩
  case assignPatternControl =>
    intro namespaceId frame state exprId ns expression pattern child
      finalFrame finalState control namespace_eq expression_eq kind_eq
      child_step abrupt ih
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind]
    cases abrupt <;> simp only [rc] <;> exact ⟨_, rfl, rf, rs, rc⟩
  case assignPatternValue =>
    intro namespaceId frame state exprId ns expression pattern child
      childFrame finalFrame finalState runtimeValue
      namespace_eq expression_eq kind_eq child_step bind_eq ih
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, e₀, bind, Except.bind, rc, rf, rs,
      bind_eq]
    exact ⟨_, rfl, rfl, rfl, rfl⟩
  case spec =>
    intro namespaceId frame state exprId ns expression block
      namespace_eq expression_eq kind_eq
    refine ⟨1, ?_⟩
    rw [Internal.evalExpr.eq_2]
    simp only [namespace_eq, expression_eq, kind_eq, bind, Except.bind]
    exact ⟨_, rfl, rfl, rfl, rfl⟩
  case body =>
    intro handle typeInstantiation initialState arguments ns declaration frame root finalFrame
      evaluatedState finalState control outcome namespace_eq declaration_eq frame_eq
      body_eq body_step outcome_eq finalize_eq ih
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih
    have arity : (arguments.size != declaration.signature.parameters.size) = false := by
      cases hb : arguments.size != declaration.signature.parameters.size with
      | false => rfl
      | true => simp [initialFrame?, hb] at frame_eq
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalFunction.eq_2]
    simp only [namespace_eq, declaration_eq, arity, frame_eq, body_eq, e₀,
      bind, Except.bind, rc]
    cases control with
    | value value =>
        cases unpack_eq : unpackFallthrough declaration.signature.results.size value with
        | none => simp [finishControl?, unpack_eq] at outcome_eq
        | some values =>
            simp [finishControl?, unpack_eq] at outcome_eq
            subst outcome_eq
            simp only [unpack_eq, rf, rs]
            exact ⟨_, rfl, finalize_eq, rfl⟩
    | «return_» values =>
        cases size_eq : values.size == declaration.signature.results.size with
        | false => simp [finishControl?, size_eq] at outcome_eq
        | true =>
            simp [finishControl?, size_eq] at outcome_eq
            subst outcome_eq
            simp only [bne, size_eq, Bool.not_true, rf, rs]
            exact ⟨_, rfl, finalize_eq, rfl⟩
    | «throw_» kind thrown =>
        simp [finishControl?] at outcome_eq
        subst outcome_eq
        simp only [rf, rs]
        exact ⟨_, rfl, finalize_eq, rfl⟩
    | break_ nest value => simp [finishControl?] at outcome_eq
    | continue_ nest => simp [finishControl?] at outcome_eq
  case nil =>
    intro namespaceId frame state
    exact ⟨0, .values state frame [], rfl, rfl⟩
  case headControl =>
    intro namespaceId frame state expression expressions finalFrame finalState control
      head_step abrupt ih
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalValues.eq_3]
    simp only [e₀, bind, Except.bind]
    cases abrupt <;> simp only [rc] <;>
      exact ⟨_, rfl, by simp [valuesResult, rf, rs, rc]⟩
  case tailValues =>
    intro namespaceId frame state expression expressions headFrame headState value
      finalFrame finalState values head_step tail_step ih_head ih_tail
    obtain ⟨f₁, r₁, e₁, rf₁, rs₁, rc₁⟩ := ih_head
    obtain ⟨f₂, r₂, e₂, er₂⟩ := ih_tail
    refine ⟨max f₁ f₂ + 1, ?_⟩
    rw [Internal.evalValues.eq_3]
    simp only [evalExpr_mono (Nat.le_max_left f₁ f₂) e₁, bind, Except.bind, rc₁, rf₁, rs₁,
      evalValues_mono (Nat.le_max_right f₁ f₂) e₂]
    cases r₂ with
    | values s f vs =>
        simp only [valuesResult] at er₂
        cases er₂
        exact ⟨_, rfl, rfl⟩
    | control s f c => simp [valuesResult] at er₂
  case tailControl =>
    intro namespaceId frame state expression expressions headFrame headState value
      finalFrame finalState control head_step tail_step ih_head ih_tail
    obtain ⟨f₁, r₁, e₁, rf₁, rs₁, rc₁⟩ := ih_head
    obtain ⟨f₂, r₂, e₂, er₂⟩ := ih_tail
    refine ⟨max f₁ f₂ + 1, ?_⟩
    rw [Internal.evalValues.eq_3]
    simp only [evalExpr_mono (Nat.le_max_left f₁ f₂) e₁, bind, Except.bind, rc₁, rf₁, rs₁,
      evalValues_mono (Nat.le_max_right f₁ f₂) e₂]
    cases r₂ with
    | values s f vs => simp [valuesResult] at er₂
    | control s f c =>
        simp only [valuesResult, ValuesResult.control.injEq] at er₂
        obtain ⟨es, ef, ec⟩ := er₂
        exact ⟨_, rfl, by simp [valuesResult, es, ef, ec]⟩
  case nil =>
    intro namespaceId frame state
    exact ⟨0, .done state frame, rfl, rfl⟩
  case headControl =>
    intro namespaceId frame state statement statements finalFrame finalState control
      head_step abrupt ih
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalStatements.eq_3]
    simp only [e₀, bind, Except.bind]
    cases abrupt <;> simp only [rc] <;>
      exact ⟨_, rfl, by simp [statementsResult, rf, rs, rc]⟩
  case cons =>
    intro namespaceId frame state statement statements headFrame headState value result
      head_step tail_step ih_head ih_tail
    obtain ⟨f₁, r₁, e₁, rf₁, rs₁, rc₁⟩ := ih_head
    obtain ⟨f₂, r₂, e₂, er₂⟩ := ih_tail
    refine ⟨max f₁ f₂ + 1, ?_⟩
    rw [Internal.evalStatements.eq_3]
    simp only [evalExpr_mono (Nat.le_max_left f₁ f₂) e₁, bind, Except.bind, rc₁, rf₁, rs₁]
    exact ⟨r₂, evalStatements_mono (Nat.le_max_right f₁ f₂) e₂, er₂⟩
  case reject =>
    intro namespaceId ns frame state value arm arms finalFrame finalState control
      bind_eq tail_step ih ownerLoc
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih ownerLoc
    refine ⟨f₀ + 1, r₀, ?_, rf, rs, rc⟩
    rw [Internal.evalArms.eq_3]
    simp only [bind_eq, bind, Except.bind]
    exact e₀
  case noGuard =>
    intro namespaceId ns frame state value arm arms armFrame finalFrame finalState
      control guard_eq bind_eq body_step ih ownerLoc
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih
    refine ⟨f₀ + 1, r₀, ?_, rf, rs, rc⟩
    rw [Internal.evalArms.eq_3]
    simp only [bind_eq, guard_eq, bind, Except.bind]
    exact e₀
  case guardControl =>
    intro namespaceId ns frame state value arm arms guard armFrame finalFrame finalState
      control guard_eq bind_eq guard_step abrupt ih ownerLoc
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := ih
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalArms.eq_3]
    simp only [bind_eq, guard_eq, e₀, bind, Except.bind]
    cases abrupt <;> simp only [rc] <;> exact ⟨r₀, rfl, rf, rs, rc⟩
  case guardTrue =>
    intro namespaceId ns frame state value arm arms guard armFrame guardFrame guardState
      finalFrame finalState control guard_eq bind_eq guard_step body_step
      ih_guard ih_body ownerLoc
    obtain ⟨f₁, r₁, e₁, rf₁, rs₁, rc₁⟩ := ih_guard
    obtain ⟨f₂, r₂, e₂, rf₂, rs₂, rc₂⟩ := ih_body
    refine ⟨max f₁ f₂ + 1, r₂, ?_, rf₂, rs₂, rc₂⟩
    rw [Internal.evalArms.eq_3]
    simp only [bind_eq, guard_eq, evalExpr_mono (Nat.le_max_left f₁ f₂) e₁,
      bind, Except.bind, rc₁, rf₁, rs₁]
    exact evalExpr_mono (Nat.le_max_right f₁ f₂) e₂
  case guardFalse =>
    intro namespaceId ns frame state value arm arms guard armFrame guardFrame guardState
      finalFrame finalState control guard_eq bind_eq guard_step tail_step
      ih_guard ih_tail ownerLoc
    obtain ⟨f₁, r₁, e₁, rf₁, rs₁, rc₁⟩ := ih_guard
    obtain ⟨f₂, r₂, e₂, rf₂, rs₂, rc₂⟩ := ih_tail ownerLoc
    refine ⟨max f₁ f₂ + 1, r₂, ?_, rf₂, rs₂, rc₂⟩
    rw [Internal.evalArms.eq_3]
    simp only [bind_eq, guard_eq, evalExpr_mono (Nat.le_max_left f₁ f₂) e₁,
      bind, Except.bind, rc₁]
    exact evalArms_mono (Nat.le_max_right f₁ f₂) e₂

/-- Completeness up to fuel: every big-step function derivation is reproduced
by the fuelled interpreter with sufficient fuel, up to erasing locations. -/
theorem evalFunction_complete {unit : ExecutableUnit} {handle typeInstantiation state arguments
    finalState outcome}
    (h : EvalFunction unit handle typeInstantiation state arguments finalState outcome) :
    ∃ fuel result,
      Internal.evalFunction fuel unit handle typeInstantiation state arguments = .ok result ∧
        result.state = finalState ∧ result.outcome.value = outcome := by
  cases h with
  | body handle typeInstantiation initialState arguments ns declaration frame root finalFrame
      evaluatedState finalState control outcome namespace_eq declaration_eq frame_eq
      body_eq body_step outcome_eq finalize_eq =>
    obtain ⟨f₀, r₀, e₀, rf, rs, rc⟩ := completeExpr body_step
    have arity : (arguments.size != declaration.signature.parameters.size) = false := by
      cases hb : arguments.size != declaration.signature.parameters.size with
      | false => rfl
      | true => simp [initialFrame?, hb] at frame_eq
    refine ⟨f₀ + 1, ?_⟩
    rw [Internal.evalFunction.eq_2]
    simp only [namespace_eq, declaration_eq, arity, frame_eq, body_eq, e₀,
      bind, Except.bind, rc]
    cases control with
    | value value =>
        cases unpack_eq : unpackFallthrough declaration.signature.results.size value with
        | none => simp [finishControl?, unpack_eq] at outcome_eq
        | some values =>
            simp [finishControl?, unpack_eq] at outcome_eq
            subst outcome_eq
            simp only [unpack_eq, rf, rs]
            exact ⟨_, rfl, finalize_eq, rfl⟩
    | «return_» values =>
        cases size_eq : values.size == declaration.signature.results.size with
        | false => simp [finishControl?, size_eq] at outcome_eq
        | true =>
            simp [finishControl?, size_eq] at outcome_eq
            subst outcome_eq
            simp only [bne, size_eq, Bool.not_true, rf, rs]
            exact ⟨_, rfl, finalize_eq, rfl⟩
    | «throw_» kind thrown =>
        simp [finishControl?] at outcome_eq
        subst outcome_eq
        simp only [rf, rs]
        exact ⟨_, rfl, finalize_eq, rfl⟩
    | break_ nest value => simp [finishControl?] at outcome_eq
    | continue_ nest => simp [finishControl?] at outcome_eq

/-- Completeness of `Interpreter.run` up to fuel and location erasure. -/
theorem run_complete {unit : ExecutableUnit} {handle state arguments finalState outcome}
    (h : EvalFunction unit handle #[] state arguments finalState outcome) :
    ∃ fuel finalOutcome,
      Interpreter.run unit fuel handle arguments state = .ok (finalState, finalOutcome) ∧
        finalOutcome.value = outcome := by
  obtain ⟨fuel, result, eval, rs, ro⟩ := evalFunction_complete h
  refine ⟨fuel, result.outcome, ?_, ro⟩
  simp only [Interpreter.run, bind, Except.bind, pure, Except.pure, eval, rs]

/-- The big-step function relation is deterministic: the interpreter is a
function, and completeness maps both derivations onto it. -/
theorem evalFunction_deterministic {unit : ExecutableUnit} {handle typeInstantiation state arguments
    finalState₁ outcome₁ finalState₂ outcome₂}
    (h₁ : EvalFunction unit handle typeInstantiation state arguments finalState₁ outcome₁)
    (h₂ : EvalFunction unit handle typeInstantiation state arguments finalState₂ outcome₂) :
    finalState₁ = finalState₂ ∧ outcome₁ = outcome₂ := by
  obtain ⟨f₁, r₁, e₁, rs₁, ro₁⟩ := evalFunction_complete h₁
  obtain ⟨f₂, r₂, e₂, rs₂, ro₂⟩ := evalFunction_complete h₂
  have lifted₁ := Fuel.evalFunction_mono (Nat.le_max_left f₁ f₂) e₁
  have lifted₂ := Fuel.evalFunction_mono (Nat.le_max_right f₁ f₂) e₂
  rw [lifted₁, Except.ok.injEq] at lifted₂
  subst lifted₂
  exact ⟨rs₁ ▸ rs₂, ro₁ ▸ ro₂⟩

/-- Determinism restated over the canonical function meaning. -/
theorem meaning_deterministic (unit : ExecutableUnit) (function : LeanerIR.FunctionHandle)
    {state arguments finalState₁ outcome₁ finalState₂ outcome₂}
    (h₁ : (BigStep.meaning unit function).relates state arguments finalState₁ outcome₁)
    (h₂ : (BigStep.meaning unit function).relates state arguments finalState₂ outcome₂) :
    finalState₁ = finalState₂ ∧ outcome₁ = outcome₂ :=
  evalFunction_deterministic h₁ h₂

end LeanerIR.Proofs.Completeness
