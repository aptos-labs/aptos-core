-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Globals
import LeanerIR.Proofs.Meaning
import LeanerIR.Proofs.Representation

/-!
# Calculated weakest preconditions for structured LIR

Demonic partial-correctness predicate transformers over the five big-step
judgments, with one calculation rule per expression kind.  The rules are
conditioned on the expression-arena lookups, so on a concrete validated unit
they fire by reduction and symbolically execute the body; soundness against
execution is definitional because the transformers quantify over the
authoritative big-step derivations directly.
-/

namespace LeanerIR.Proofs

open LeanerIR.Validation
open LeanerIR.BigStep
open SemanticOperations

/-! ## Transformers -/

/-- Demonic weakest precondition of one structured expression: every
derivable evaluation satisfies the postcondition.  Partial correctness — a
diverging expression satisfies everything. -/
def wpExpr (unit : ExecutableUnit) (namespaceId : NamespaceId)
    (frame : RuntimeFrame) (state : RuntimeState) (exprId : ExprId)
    (post : RuntimeFrame → RuntimeState → Control → Prop) : Prop :=
  ∀ finalFrame finalState control,
    EvalExpr unit namespaceId frame state exprId finalFrame finalState control →
    post finalFrame finalState control

/-- Weakest precondition of an operand list. -/
def wpValues (unit : ExecutableUnit) (namespaceId : NamespaceId)
    (frame : RuntimeFrame) (state : RuntimeState) (expressions : List ExprId)
    (post : ValuesResult → Prop) : Prop :=
  ∀ result,
    EvalValues unit namespaceId frame state expressions result → post result

/-- Weakest precondition of a statement list. -/
def wpStatements (unit : ExecutableUnit) (namespaceId : NamespaceId)
    (frame : RuntimeFrame) (state : RuntimeState) (statements : List ExprId)
    (post : StatementsResult → Prop) : Prop :=
  ∀ result,
    EvalStatements unit namespaceId frame state statements result → post result

/-- Weakest precondition of match-arm selection. -/
def wpArms (unit : ExecutableUnit) (namespaceId : NamespaceId)
    (ns : ValidatedNamespace) (frame : RuntimeFrame) (state : RuntimeState)
    (value : RuntimeValue) (arms : List MatchArm)
    (post : RuntimeFrame → RuntimeState → Control → Prop) : Prop :=
  ∀ finalFrame finalState control,
    EvalArms unit namespaceId ns frame state value arms
      finalFrame finalState control →
    post finalFrame finalState control

/-- Weakest precondition of a function invocation. -/
def wpFunction (unit : ExecutableUnit) (handle : FunctionHandle)
    (state : RuntimeState) (arguments : Array RuntimeValue)
    (post : RuntimeState → Outcome → Prop) : Prop :=
  ∀ finalState outcome,
    EvalFunction unit handle state arguments finalState outcome →
    post finalState outcome

/-! ## Monotonicity -/

theorem wpExpr_mono {unit namespaceId frame state exprId}
    {post post' : RuntimeFrame → RuntimeState → Control → Prop}
    (established : wpExpr unit namespaceId frame state exprId post)
    (weaken : ∀ frame state control, post frame state control →
      post' frame state control) :
    wpExpr unit namespaceId frame state exprId post' :=
  fun finalFrame finalState control step =>
    weaken _ _ _ (established finalFrame finalState control step)

theorem wpValues_mono {unit namespaceId frame state expressions}
    {post post' : ValuesResult → Prop}
    (established : wpValues unit namespaceId frame state expressions post)
    (weaken : ∀ result, post result → post' result) :
    wpValues unit namespaceId frame state expressions post' :=
  fun result step => weaken _ (established result step)

theorem wpStatements_mono {unit namespaceId frame state statements}
    {post post' : StatementsResult → Prop}
    (established : wpStatements unit namespaceId frame state statements post)
    (weaken : ∀ result, post result → post' result) :
    wpStatements unit namespaceId frame state statements post' :=
  fun result step => weaken _ (established result step)

theorem wpFunction_mono {unit handle state arguments}
    {post post' : RuntimeState → Outcome → Prop}
    (established : wpFunction unit handle state arguments post)
    (weaken : ∀ state outcome, post state outcome → post' state outcome) :
    wpFunction unit handle state arguments post' :=
  fun finalState outcome step => weaken _ _ (established finalState outcome step)

/-! ## Bridges to the contract calculus -/

/-- Unfold one function invocation into its body's transformer. -/
theorem wpFunction_body {unit : ExecutableUnit} {handle : FunctionHandle}
    {initialState : RuntimeState} {arguments : Array RuntimeValue}
    {ns : ValidatedNamespace} {declaration : FunctionDecl FunctionBody}
    {frame : RuntimeFrame}
    {root : ExprId} {post : RuntimeState → Outcome → Prop}
    (namespace_eq : unit.unit.namespaces[handle.namespaceId.index]? = some ns)
    (declaration_eq : ns.functions[handle.functionId.index]? = some declaration)
    (frame_eq : initialFrame? declaration arguments = some frame)
    (body_eq : declaration.body = .structured root) :
    wpFunction unit handle initialState arguments post ↔
      wpExpr unit handle.namespaceId frame initialState root
        fun finalFrame evaluatedState control =>
          ∀ outcome,
            finishControl? declaration.signature.results.size control
              = some outcome →
            post
              (finalizeFunctionState unit declaration.profile initialState
                evaluatedState finalFrame outcome)
              outcome := by
  constructor
  · intro h finalFrame evaluatedState control step outcome outcome_eq
    exact h _ outcome
      (.body handle initialState arguments ns declaration frame root finalFrame
        evaluatedState _ control outcome namespace_eq declaration_eq frame_eq
        body_eq step outcome_eq rfl)
  · intro h finalState outcome step
    cases step with
    | body handle initialState arguments ns' declaration' frame' root' finalFrame
        evaluatedState finalState control outcome namespace_eq' declaration_eq'
        frame_eq' body_eq' body_step outcome_eq finalize_eq =>
      rw [namespace_eq] at namespace_eq'
      cases namespace_eq'
      rw [declaration_eq] at declaration_eq'
      cases declaration_eq'
      rw [frame_eq] at frame_eq'
      cases frame_eq'
      rw [body_eq] at body_eq'
      cases body_eq'
      rw [← finalize_eq]
      exact h _ _ _ body_step outcome outcome_eq

/-! ## Control classification -/

@[simp] theorem abrupt_value_iff (value : RuntimeValue) :
    ¬Abrupt (.value value) := fun h => nomatch h

@[simp] theorem abrupt_break (nest : Nat) (value : Option RuntimeValue) :
    Abrupt (.break_ nest value) := .break_ nest value

@[simp] theorem abrupt_continue (nest : Nat) : Abrupt (.continue_ nest) :=
  .continue_ nest

@[simp] theorem abrupt_return (values : Array RuntimeValue) :
    Abrupt (.return_ values) := .return_ values

@[simp] theorem abrupt_throw (kind : ThrowKind) (arguments : Array RuntimeValue) :
    Abrupt (.throw_ kind arguments) := .throw_ kind arguments

/-- Continue on a value result; propagate every abrupt control unchanged. -/
def valueOr (post : RuntimeFrame → RuntimeState → Control → Prop)
    (next : RuntimeFrame → RuntimeState → RuntimeValue → Prop) :
    RuntimeFrame → RuntimeState → Control → Prop :=
  fun frame state control =>
    match control with
    | .value value => next frame state value
    | control => post frame state control

@[simp] theorem valueOr_value (post next frame state value) :
    valueOr post next frame state (.value value) = next frame state value := rfl

@[simp] theorem valueOr_break (post next frame state nest value) :
    valueOr post next frame state (.break_ nest value)
      = post frame state (.break_ nest value) := rfl

@[simp] theorem valueOr_continue (post next frame state nest) :
    valueOr post next frame state (.continue_ nest)
      = post frame state (.continue_ nest) := rfl

@[simp] theorem valueOr_return (post next frame state values) :
    valueOr post next frame state (.return_ values)
      = post frame state (.return_ values) := rfl

@[simp] theorem valueOr_throw (post next frame state kind arguments) :
    valueOr post next frame state (.throw_ kind arguments)
      = post frame state (.throw_ kind arguments) := rfl

theorem valueOr_of_abrupt {post next frame state control}
    (abrupt : Abrupt control) (h : post frame state control) :
    valueOr post next frame state control := by
  cases abrupt <;> exact h

/-- Extend an operand-list postcondition by a prepended head value. -/
def consValues (value : RuntimeValue) (post : ValuesResult → Prop) :
    ValuesResult → Prop
  | .values state frame values => post (.values state frame (value :: values))
  | result => post result

@[simp] theorem consValues_values (value post state frame values) :
    consValues value post (.values state frame values)
      = post (.values state frame (value :: values)) := rfl

@[simp] theorem consValues_control (value post state frame control) :
    consValues value post (.control state frame control)
      = post (.control state frame control) := rfl

/-! ## Calculation rules: leaves -/

theorem wpExpr_value {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {literal : ConstValue}
    {source : Option String}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .value literal source) :
    wpExpr unit namespaceId frame state exprId post ↔
      ∀ value, constValue? literal = some value →
        post frame state (.value value) := by
  constructor
  · intro h value value_eq
    exact h frame state _ (.value namespaceId frame state exprId ns expression
      literal source value namespace_eq expression_eq kind_eq value_eq)
  · intro h finalFrame finalState control step
    cases step <;> simp_all

theorem wpExpr_localVar {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {localId : LocalId}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .localVar localId) :
    wpExpr unit namespaceId frame state exprId post ↔
      ∀ value, readLocal? frame localId = some value →
        post frame state (.value value) := by
  constructor
  · intro h value local_eq
    exact h frame state _ (.localVar namespaceId frame state exprId ns expression
      localId value namespace_eq expression_eq kind_eq local_eq)
  · intro h finalFrame finalState control step
    cases step <;> simp_all

theorem wpExpr_continue {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {nest : Nat}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .continue_ nest) :
    wpExpr unit namespaceId frame state exprId post ↔
      post frame state (.continue_ nest) := by
  constructor
  · intro h
    exact h frame state _ (.continue_ namespaceId frame state exprId ns
      expression nest namespace_eq expression_eq kind_eq)
  · intro h finalFrame finalState control step
    cases step <;> simp_all

theorem wpExpr_breakNone {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {nest : Nat}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .break_ nest none) :
    wpExpr unit namespaceId frame state exprId post ↔
      post frame state (.break_ nest none) := by
  constructor
  · intro h
    exact h frame state _ (.breakNone namespaceId frame state exprId ns
      expression nest namespace_eq expression_eq kind_eq)
  · intro h finalFrame finalState control step
    cases step <;> simp_all

theorem wpExpr_spec {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {block : SpecBlock}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .spec block) :
    wpExpr unit namespaceId frame state exprId post ↔
      post frame state (.value .unit) := by
  constructor
  · intro h
    exact h frame state _ (.spec namespaceId frame state exprId ns expression
      block namespace_eq expression_eq kind_eq)
  · intro h finalFrame finalState control step
    cases step <;> simp_all

/-! ## Calculation rules: sequencing -/

theorem wpValues_nil {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {post : ValuesResult → Prop} :
    wpValues unit namespaceId frame state [] post ↔
      post (.values state frame []) := by
  constructor
  · intro h
    exact h _ (.nil namespaceId frame state)
  · intro h result step
    cases step
    exact h

theorem wpValues_cons {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {expression : ExprId}
    {expressions : List ExprId} {post : ValuesResult → Prop} :
    wpValues unit namespaceId frame state (expression :: expressions) post ↔
      wpExpr unit namespaceId frame state expression
        (valueOr (fun headFrame headState control =>
            post (.control headState headFrame control))
          fun headFrame headState value =>
            wpValues unit namespaceId headFrame headState expressions
              (consValues value post)) := by
  constructor
  · intro h headFrame headState control step
    cases control with
    | value value =>
        intro result tail
        cases result with
        | values state' frame' values' =>
            exact h _ (.tailValues namespaceId frame state expression expressions
              headFrame headState value _ _ _ step tail)
        | control state' frame' control' =>
            exact h _ (.tailControl namespaceId frame state expression expressions
              headFrame headState value _ _ _ step tail)
    | break_ nest value =>
        exact h _ (.headControl namespaceId frame state expression expressions
          headFrame headState _ step (by simp))
    | continue_ nest =>
        exact h _ (.headControl namespaceId frame state expression expressions
          headFrame headState _ step (by simp))
    | return_ values =>
        exact h _ (.headControl namespaceId frame state expression expressions
          headFrame headState _ step (by simp))
    | throw_ kind arguments =>
        exact h _ (.headControl namespaceId frame state expression expressions
          headFrame headState _ step (by simp))
  · intro h result step
    cases step with
    | headControl _ _ _ _ _ _ _ _ head_step abrupt =>
        have := h _ _ _ head_step
        cases abrupt <;> exact this
    | tailValues _ _ _ _ _ headFrame headState value _ _ _ head_step tail_step =>
        exact h _ _ _ head_step _ tail_step
    | tailControl _ _ _ _ _ headFrame headState value _ _ _ head_step tail_step =>
        exact h _ _ _ head_step _ tail_step

theorem wpStatements_nil {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState}
    {post : StatementsResult → Prop} :
    wpStatements unit namespaceId frame state [] post ↔
      post (.done state frame) := by
  constructor
  · intro h
    exact h _ (.nil namespaceId frame state)
  · intro h result step
    cases step
    exact h

theorem wpStatements_cons {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {statement : ExprId}
    {statements : List ExprId} {post : StatementsResult → Prop} :
    wpStatements unit namespaceId frame state (statement :: statements) post ↔
      wpExpr unit namespaceId frame state statement
        (valueOr (fun headFrame headState control =>
            post (.control headState headFrame control))
          fun headFrame headState _ =>
            wpStatements unit namespaceId headFrame headState statements post) := by
  constructor
  · intro h headFrame headState control step
    cases control with
    | value value =>
        intro result tail
        exact h _ (.cons namespaceId frame state statement statements
          headFrame headState value _ step tail)
    | break_ nest value =>
        exact h _ (.headControl namespaceId frame state statement statements
          headFrame headState _ step (by simp))
    | continue_ nest =>
        exact h _ (.headControl namespaceId frame state statement statements
          headFrame headState _ step (by simp))
    | return_ values =>
        exact h _ (.headControl namespaceId frame state statement statements
          headFrame headState _ step (by simp))
    | throw_ kind arguments =>
        exact h _ (.headControl namespaceId frame state statement statements
          headFrame headState _ step (by simp))
  · intro h result step
    cases step with
    | headControl _ _ _ _ _ _ _ _ head_step abrupt =>
        have := h _ _ _ head_step
        cases abrupt <;> exact this
    | cons _ _ _ _ _ headFrame headState value _ head_step tail_step =>
        exact h _ _ _ head_step _ tail_step

/-! ## Calculation rules: blocks and bindings -/

/-- Route a statement-list result into an expression postcondition. -/
def blockPost (post : RuntimeFrame → RuntimeState → Control → Prop)
    (onDone : RuntimeFrame → RuntimeState → Prop) : StatementsResult → Prop
  | .done state frame => onDone frame state
  | .control state frame control => post frame state control

@[simp] theorem blockPost_done (post onDone state frame) :
    blockPost post onDone (.done state frame) = onDone frame state := rfl

@[simp] theorem blockPost_control (post onDone state frame control) :
    blockPost post onDone (.control state frame control)
      = post frame state control := rfl

theorem wpExpr_blockUnit {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {statements : Array ExprId}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .block statements none) :
    wpExpr unit namespaceId frame state exprId post ↔
      wpStatements unit namespaceId frame state statements.toList
        (blockPost post fun doneFrame doneState =>
          post doneFrame doneState (.value .unit)) := by
  constructor
  · intro h result step
    cases result with
    | done state' frame' =>
        exact h _ _ _ (.blockUnit namespaceId frame state exprId ns expression
          statements _ _ namespace_eq expression_eq kind_eq step)
    | control state' frame' control' =>
        exact h _ _ _ (.blockControl namespaceId frame state exprId ns expression
          statements none _ _ _ namespace_eq expression_eq kind_eq step)
  · intro h finalFrame finalState control step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals try obtain ⟨rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl⟩ := kind_eq
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals exact h _ (by assumption)

theorem wpExpr_blockResult {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {statements : Array ExprId}
    {result : ExprId} {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .block statements (some result)) :
    wpExpr unit namespaceId frame state exprId post ↔
      wpStatements unit namespaceId frame state statements.toList
        (blockPost post fun doneFrame doneState =>
          wpExpr unit namespaceId doneFrame doneState result post) := by
  constructor
  · intro h statementsResult step
    cases statementsResult with
    | done state' frame' =>
        intro finalFrame finalState control result_step
        exact h _ _ _ (.blockResult namespaceId frame state exprId ns expression
          statements result _ _ _ _ _ namespace_eq expression_eq kind_eq step
          result_step)
    | control state' frame' control' =>
        exact h _ _ _ (.blockControl namespaceId frame state exprId ns expression
          statements (some result) _ _ _ namespace_eq expression_eq kind_eq step)
  · intro h finalFrame finalState control step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals try obtain ⟨rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl⟩ := kind_eq
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals first
      | exact h (.control _ _ _) (by assumption)
      | exact h (.done _ _) (by assumption) _ _ _ (by assumption)

theorem wpExpr_letNoValue {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {pattern : PatternId}
    {body : ExprId} {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .letDecl pattern none body) :
    wpExpr unit namespaceId frame state exprId post ↔
      wpExpr unit namespaceId frame state body post := by
  constructor
  · intro h finalFrame finalState control step
    exact h _ _ _ (.letNoValue namespaceId frame state exprId ns expression
      pattern body _ _ _ namespace_eq expression_eq kind_eq step)
  · intro h finalFrame finalState control step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals try obtain ⟨rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl⟩ := kind_eq
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals exact h _ _ _ (by assumption)

theorem wpExpr_letValue {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {pattern : PatternId}
    {initializer body : ExprId}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .letDecl pattern (some initializer) body) :
    wpExpr unit namespaceId frame state exprId post ↔
      wpExpr unit namespaceId frame state initializer
        (valueOr post fun initializedFrame initializedState value =>
          ∀ boundFrame,
            bindPattern unit.unit ns initializedFrame pattern value = some boundFrame →
            wpExpr unit namespaceId boundFrame initializedState body post) := by
  constructor
  · intro h initializedFrame initializedState control step
    cases control with
    | value value =>
        intro boundFrame bind_eq finalFrame finalState control body_step
        exact h _ _ _ (.letValue namespaceId frame state exprId ns expression
          pattern initializer body _ _ _ _ _ _ _ namespace_eq expression_eq
          kind_eq step bind_eq body_step)
    | break_ nest value =>
        exact h _ _ _ (.letValueControl namespaceId frame state exprId ns
          expression pattern initializer body _ _ _ namespace_eq expression_eq
          kind_eq step (by simp))
    | continue_ nest =>
        exact h _ _ _ (.letValueControl namespaceId frame state exprId ns
          expression pattern initializer body _ _ _ namespace_eq expression_eq
          kind_eq step (by simp))
    | return_ values =>
        exact h _ _ _ (.letValueControl namespaceId frame state exprId ns
          expression pattern initializer body _ _ _ namespace_eq expression_eq
          kind_eq step (by simp))
    | throw_ kind arguments =>
        exact h _ _ _ (.letValueControl namespaceId frame state exprId ns
          expression pattern initializer body _ _ _ namespace_eq expression_eq
          kind_eq step (by simp))
  · intro h finalFrame finalState control step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals try obtain ⟨rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl⟩ := kind_eq
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals first
      | exact h _ _ (.value _) (by assumption) _ (by assumption) _ _ _
          (by assumption)
      | (have abrupt : Abrupt control := by assumption
         have propagated := h _ _ _ (by assumption :
           EvalExpr unit namespaceId frame state _ finalFrame
             finalState control)
         cases abrupt <;> exact propagated)

/-- Route an operand-list result into an expression postcondition. -/
def valuesPost (post : RuntimeFrame → RuntimeState → Control → Prop)
    (onValues : RuntimeFrame → RuntimeState → List RuntimeValue → Prop) :
    ValuesResult → Prop
  | .values state frame values => onValues frame state values
  | .control state frame control => post frame state control

@[simp] theorem valuesPost_values (post onValues state frame values) :
    valuesPost post onValues (.values state frame values)
      = onValues frame state values := rfl

@[simp] theorem valuesPost_control (post onValues state frame control) :
    valuesPost post onValues (.control state frame control)
      = post frame state control := rfl

/-- The place dispatcher owns no primitive operations. -/
@[simp] theorem evaluatePlaceOperation?_primitive {unit : ValidatedUnit}
    {ns : ValidatedNamespace} {resultType : TypeId} {site : ExprId}
    {operation : PrimitiveOperation} {arguments : Array RuntimeValue}
    {frame : RuntimeFrame} {state : RuntimeState} :
    evaluatePlaceOperation? unit ns resultType site (.primitive operation) arguments
      frame state = none := rfl

/-- The place dispatcher owns no assertions. -/
@[simp] theorem evaluatePlaceOperation?_assert {unit : ValidatedUnit}
    {ns : ValidatedNamespace} {resultType : TypeId} {site : ExprId}
    {arguments : Array RuntimeValue} {frame : RuntimeFrame}
    {state : RuntimeState} :
    evaluatePlaceOperation? unit ns resultType site .assert arguments
      frame state = none := rfl

/-! ## Calculation rules: control flow -/

theorem wpExpr_ifElse {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {condition thenBranch elseBranch : ExprId}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .ifElse condition thenBranch (some elseBranch)) :
    wpExpr unit namespaceId frame state exprId post ↔
      wpExpr unit namespaceId frame state condition
        (valueOr post fun conditionFrame conditionState value =>
          (value = .bool true →
            wpExpr unit namespaceId conditionFrame conditionState thenBranch post) ∧
          (value = .bool false →
            wpExpr unit namespaceId conditionFrame conditionState elseBranch post)) := by
  constructor
  · intro h conditionFrame conditionState control step
    cases control with
    | value value =>
        refine ⟨?_, ?_⟩ <;> rintro rfl <;> intro finalFrame finalState control branch
        · exact h _ _ _ (.ifTrue (namespace_eq := namespace_eq)
            (expression_eq := expression_eq) (kind_eq := kind_eq)
            (condition_step := step) (branch_step := branch))
        · exact h _ _ _ (.ifFalse (namespace_eq := namespace_eq)
            (expression_eq := expression_eq) (kind_eq := kind_eq)
            (condition_step := step) (branch_step := branch))
    | break_ nest value =>
        exact h _ _ _ (.ifControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (condition_step := step) (abrupt := by simp))
    | continue_ nest =>
        exact h _ _ _ (.ifControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (condition_step := step) (abrupt := by simp))
    | return_ values =>
        exact h _ _ _ (.ifControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (condition_step := step) (abrupt := by simp))
    | throw_ kind arguments =>
        exact h _ _ _ (.ifControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (condition_step := step) (abrupt := by simp))
  · intro h finalFrame finalState control step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals try obtain ⟨rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl⟩ := kind_eq
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals first
      | exact (h _ _ (.value (.bool true)) (by assumption)).1 rfl _ _ _
          (by assumption)
      | exact (h _ _ (.value (.bool false)) (by assumption)).2 rfl _ _ _
          (by assumption)
      | (have abrupt : Abrupt control := by assumption
         have propagated := h _ _ _ (by assumption :
           EvalExpr unit namespaceId frame state _ finalFrame finalState control)
         cases abrupt <;> exact propagated)

theorem wpExpr_ifNoElse {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {condition thenBranch : ExprId}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .ifElse condition thenBranch none) :
    wpExpr unit namespaceId frame state exprId post ↔
      wpExpr unit namespaceId frame state condition
        (valueOr post fun conditionFrame conditionState value =>
          (value = .bool true →
            wpExpr unit namespaceId conditionFrame conditionState thenBranch post) ∧
          (value = .bool false →
            post conditionFrame conditionState (.value .unit))) := by
  constructor
  · intro h conditionFrame conditionState control step
    cases control with
    | value value =>
        refine ⟨?_, ?_⟩ <;> rintro rfl
        · intro finalFrame finalState control branch
          exact h _ _ _ (.ifTrue (namespace_eq := namespace_eq)
            (expression_eq := expression_eq) (kind_eq := kind_eq)
            (condition_step := step) (branch_step := branch))
        · exact h _ _ _ (.ifFalseUnit (namespace_eq := namespace_eq)
            (expression_eq := expression_eq) (kind_eq := kind_eq)
            (condition_step := step))
    | break_ nest value =>
        exact h _ _ _ (.ifControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (condition_step := step) (abrupt := by simp))
    | continue_ nest =>
        exact h _ _ _ (.ifControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (condition_step := step) (abrupt := by simp))
    | return_ values =>
        exact h _ _ _ (.ifControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (condition_step := step) (abrupt := by simp))
    | throw_ kind arguments =>
        exact h _ _ _ (.ifControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (condition_step := step) (abrupt := by simp))
  · intro h finalFrame finalState control step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals try obtain ⟨rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl⟩ := kind_eq
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals first
      | exact (h _ _ (.value (.bool true)) (by assumption)).1 rfl _ _ _
          (by assumption)
      | exact (h _ _ (.value (.bool false)) (by assumption)).2 rfl
      | (have abrupt : Abrupt control := by assumption
         have propagated := h _ _ _ (by assumption :
           EvalExpr unit namespaceId frame state _ finalFrame finalState control)
         cases abrupt <;> exact propagated)

theorem wpExpr_breakValue {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {nest : Nat} {child : ExprId}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .break_ nest (some child)) :
    wpExpr unit namespaceId frame state exprId post ↔
      wpExpr unit namespaceId frame state child
        (valueOr post fun childFrame childState value =>
          post childFrame childState (.break_ nest (some value))) := by
  constructor
  · intro h childFrame childState control step
    cases control with
    | value value =>
        exact h _ _ _ (.breakValue (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (child_step := step))
    | break_ nest value =>
        exact h _ _ _ (.breakControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (child_step := step) (abrupt := by simp))
    | continue_ nest =>
        exact h _ _ _ (.breakControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (child_step := step) (abrupt := by simp))
    | return_ values =>
        exact h _ _ _ (.breakControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (child_step := step) (abrupt := by simp))
    | throw_ kind arguments =>
        exact h _ _ _ (.breakControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (child_step := step) (abrupt := by simp))
  · intro h finalFrame finalState control step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals try obtain ⟨rfl, rfl⟩ := kind_eq
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals first
      | exact h _ _ (.value _) (by assumption)
      | (have abrupt : Abrupt control := by assumption
         have propagated := h _ _ _ (by assumption :
           EvalExpr unit namespaceId frame state _ finalFrame finalState control)
         cases abrupt <;> exact propagated)

theorem wpExpr_return {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {values : Array ExprId}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .return_ values) :
    wpExpr unit namespaceId frame state exprId post ↔
      wpValues unit namespaceId frame state values.toList
        (valuesPost post fun finalFrame finalState runtimeValues =>
          post finalFrame finalState (.return_ runtimeValues.toArray)) := by
  constructor
  · intro h result step
    cases result with
    | values state' frame' values' =>
        exact h _ _ _ (.returnValues (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (values_step := step))
    | control state' frame' control' =>
        exact h _ _ _ (.returnControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (values_step := step))
  · intro h finalFrame finalState control step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals first
      | exact h (.values _ _ _) (by assumption)
      | exact h (.control _ _ _) (by assumption)

theorem wpExpr_throw {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {kind : ThrowKind}
    {arguments : Array ExprId}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .throw_ kind arguments) :
    wpExpr unit namespaceId frame state exprId post ↔
      wpValues unit namespaceId frame state arguments.toList
        (valuesPost post fun finalFrame finalState runtimeValues =>
          post finalFrame finalState (.throw_ kind runtimeValues.toArray)) := by
  constructor
  · intro h result step
    cases result with
    | values state' frame' values' =>
        exact h _ _ _ (.throwValues (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (values_step := step))
    | control state' frame' control' =>
        exact h _ _ _ (.throwControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (values_step := step))
  · intro h finalFrame finalState control step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals try obtain ⟨rfl, rfl⟩ := kind_eq
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals first
      | exact h (.values _ _ _) (by assumption)
      | exact h (.control _ _ _) (by assumption)

theorem wpExpr_assign {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {place : PlaceId}
    {child : ExprId} {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .assign place child) :
    wpExpr unit namespaceId frame state exprId post ↔
      wpExpr unit namespaceId frame state child
        (valueOr post fun childFrame childState value =>
          ∀ resolved finalFrame finalState,
            resolvePlace? unit.unit ns childFrame childState place
              = some resolved →
            writeRuntimePlace? childFrame childState resolved value
              = some (finalFrame, finalState) →
            post finalFrame finalState (.value .unit)) := by
  constructor
  · intro h childFrame childState control step
    cases control with
    | value value =>
        intro resolved finalFrame finalState resolve_eq write_eq
        exact h _ _ _ (.assignValue (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (child_step := step) (resolve_eq := resolve_eq) (write_eq := write_eq))
    | break_ nest value =>
        exact h _ _ _ (.assignControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (child_step := step) (abrupt := by simp))
    | continue_ nest =>
        exact h _ _ _ (.assignControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (child_step := step) (abrupt := by simp))
    | return_ values =>
        exact h _ _ _ (.assignControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (child_step := step) (abrupt := by simp))
    | throw_ kind arguments =>
        exact h _ _ _ (.assignControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (child_step := step) (abrupt := by simp))
  · intro h finalFrame finalState control step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals try obtain ⟨rfl, rfl⟩ := kind_eq
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals first
      | exact h _ _ (.value _) (by assumption) _ _ _ (by assumption)
          (by assumption)
      | (have abrupt : Abrupt control := by assumption
         have propagated := h _ _ _ (by assumption :
           EvalExpr unit namespaceId frame state _ finalFrame finalState control)
         cases abrupt <;> exact propagated)

theorem wpExpr_assignPattern {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {pattern : PatternId}
    {child : ExprId} {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .assignPattern pattern child) :
    wpExpr unit namespaceId frame state exprId post ↔
      wpExpr unit namespaceId frame state child
        (valueOr post fun childFrame childState value =>
          ∀ finalFrame,
            bindPattern unit.unit ns childFrame pattern value = some finalFrame →
            post finalFrame childState (.value .unit)) := by
  constructor
  · intro h childFrame childState control step
    cases control with
    | value value =>
        intro finalFrame bind_eq
        exact h _ _ _ (.assignPatternValue (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (child_step := step) (bind_eq := bind_eq))
    | break_ nest value =>
        exact h _ _ _ (.assignPatternControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (child_step := step) (abrupt := by simp))
    | continue_ nest =>
        exact h _ _ _ (.assignPatternControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (child_step := step) (abrupt := by simp))
    | return_ values =>
        exact h _ _ _ (.assignPatternControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (child_step := step) (abrupt := by simp))
    | throw_ kind arguments =>
        exact h _ _ _ (.assignPatternControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (child_step := step) (abrupt := by simp))
  · intro h finalFrame finalState control step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals try obtain ⟨rfl, rfl⟩ := kind_eq
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals first
      | exact h _ _ (.value _) (by assumption) _ (by assumption)
      | (have abrupt : Abrupt control := by assumption
         have propagated := h _ _ _ (by assumption :
           EvalExpr unit namespaceId frame state _ finalFrame finalState control)
         cases abrupt <;> exact propagated)

/-! ## Calculation rules: operations -/

theorem wpExpr_assert {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {instantiations : Array GenericArgument} {arguments : Array ExprId}
    {surface : Option SurfaceSyntax}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation .assert instantiations arguments
      surface) :
    wpExpr unit namespaceId frame state exprId post ↔
      wpValues unit namespaceId frame state arguments.toList
        (valuesPost post fun finalFrame finalState values =>
          (values = [.bool true] → post finalFrame finalState (.value .unit)) ∧
          (values = [.bool false] →
            post finalFrame finalState (.throw_ .abort #[]))) := by
  constructor
  · intro h result step
    cases result with
    | values state' frame' values' =>
        refine ⟨?_, ?_⟩ <;> rintro rfl
        · exact h _ _ _ (.assertTrue (namespace_eq := namespace_eq)
            (expression_eq := expression_eq) (kind_eq := kind_eq)
            (operands := step))
        · exact h _ _ _ (.assertFalse (namespace_eq := namespace_eq)
            (expression_eq := expression_eq) (kind_eq := kind_eq)
            (operands := step))
    | control state' frame' control' =>
        exact h _ _ _ (.assertArgumentsControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (operands := step))
  · intro h finalFrame finalState control step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals try obtain ⟨rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl⟩ := kind_eq
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals first
      | exact (h (.values _ _ _) (by assumption)).1 rfl
      | exact (h (.values _ _ _) (by assumption)).2 rfl
      | exact h (.control _ _ _) (by assumption)

theorem wpExpr_primitive {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {operation : PrimitiveOperation}
    {instantiations : Array GenericArgument} {arguments : Array ExprId}
    {surface : Option SurfaceSyntax}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation (.primitive operation)
      instantiations arguments surface) :
    wpExpr unit namespaceId frame state exprId post ↔
      wpValues unit namespaceId frame state arguments.toList
        (valuesPost post fun finalFrame finalState values =>
          (∀ value,
            evaluatePrimitiveOperation? ns expression.typeId operation
              values.toArray unit.targetPointerWidth = some (.ok value) →
            post finalFrame finalState (.value value)) ∧
          (∀ kind thrown,
            evaluatePrimitiveOperation? ns expression.typeId operation
              values.toArray unit.targetPointerWidth
              = some (.error (kind, thrown)) →
            post finalFrame finalState (.throw_ kind thrown))) := by
  constructor
  · intro h result step
    cases result with
    | values state' frame' values' =>
        refine ⟨?_, ?_⟩
        · intro value evaluate_eq
          exact h _ _ _ (.primitiveValue (namespace_eq := namespace_eq)
            (expression_eq := expression_eq) (kind_eq := kind_eq)
            (operands := step) (evaluate_eq := evaluate_eq))
        · intro kind thrown evaluate_eq
          exact h _ _ _ (.primitiveThrow (namespace_eq := namespace_eq)
            (expression_eq := expression_eq) (kind_eq := kind_eq)
            (operands := step) (evaluate_eq := evaluate_eq))
    | control state' frame' control' =>
        exact h _ _ _ (.primitiveArgumentsControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (operands := step))
  · intro h finalFrame finalState control step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals try obtain ⟨rfl, rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl⟩ := kind_eq
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals first
      | exact (h (.values _ _ _) (by assumption)).1 _ (by assumption)
      | exact (h (.values _ _ _) (by assumption)).2 _ _ (by assumption)
      | exact h (.control _ _ _) (by assumption)

/-- The place dispatcher owns no calls. -/
@[simp] theorem evaluatePlaceOperation?_call {unit : ValidatedUnit}
    {ns : ValidatedNamespace} {resultType : TypeId} {site : ExprId}
    {callKind : CallKind}
    {arguments : Array RuntimeValue} {frame : RuntimeFrame}
    {state : RuntimeState} :
    evaluatePlaceOperation? unit ns resultType site (.call callKind) arguments
      frame state = none := rfl

/-- Route a callee outcome into an expression postcondition at the caller's
argument frame. -/
def outcomePost (onReturn : RuntimeState → Array RuntimeValue → Prop)
    (onThrow : RuntimeState → ThrowKind → Array RuntimeValue → Prop) :
    RuntimeState → Outcome → Prop
  | finalState, .returned results => onReturn finalState results
  | finalState, .threw kind thrown => onThrow finalState kind thrown

@[simp] theorem outcomePost_returned (onReturn onThrow finalState results) :
    outcomePost onReturn onThrow finalState (.returned results)
      = onReturn finalState results := rfl

@[simp] theorem outcomePost_threw (onReturn onThrow finalState kind thrown) :
    outcomePost onReturn onThrow finalState (.threw kind thrown)
      = onThrow finalState kind thrown := rfl

/-! ## Calculation rules: constants and calls -/

theorem wpExpr_constant {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {reference : QualifiedRef}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .constant reference) :
    wpExpr unit namespaceId frame state exprId post ↔
      ∀ handle, resolveConstant? unit.unit namespaceId reference = some handle →
        ∀ targetNs declaration,
          unit.unit.namespaces[handle.namespaceId.index]? = some targetNs →
          targetNs.constants[handle.constantId]? = some declaration →
          wpExpr unit handle.namespaceId { locals := #[] } state
            declaration.value
            (fun _ finalState control => post frame finalState control) := by
  constructor
  · intro h handle resolve_eq targetNs declaration target_namespace_eq
      declaration_eq targetFrame finalState control step
    cases control with
    | value value =>
        exact h _ _ _ (.constantValue (frame := frame) (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (resolve_eq := resolve_eq) (target_namespace_eq := target_namespace_eq)
          (declaration_eq := declaration_eq) (initializer := step))
    | break_ nest value =>
        exact h _ _ _ (.constantControl (frame := frame) (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (resolve_eq := resolve_eq) (target_namespace_eq := target_namespace_eq)
          (declaration_eq := declaration_eq) (initializer := step)
          (abrupt := by simp))
    | continue_ nest =>
        exact h _ _ _ (.constantControl (frame := frame) (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (resolve_eq := resolve_eq) (target_namespace_eq := target_namespace_eq)
          (declaration_eq := declaration_eq) (initializer := step)
          (abrupt := by simp))
    | return_ values =>
        exact h _ _ _ (.constantControl (frame := frame) (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (resolve_eq := resolve_eq) (target_namespace_eq := target_namespace_eq)
          (declaration_eq := declaration_eq) (initializer := step)
          (abrupt := by simp))
    | throw_ kind arguments =>
        exact h _ _ _ (.constantControl (frame := frame) (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (resolve_eq := resolve_eq) (target_namespace_eq := target_namespace_eq)
          (declaration_eq := declaration_eq) (initializer := step)
          (abrupt := by simp))
  · intro h finalFrame finalState control step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals first
      | exact h _ (by assumption) _ _ (by assumption) (by assumption) _ _ _
          (by assumption)
      | exact h _ _ rfl (by assumption) _ _ _ (by assumption)

theorem wpExpr_call {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {reference : QualifiedRef}
    {instantiations : Array GenericArgument} {arguments : Array ExprId}
    {surface : Option SurfaceSyntax}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation (.call (.function reference))
      instantiations arguments surface) :
    wpExpr unit namespaceId frame state exprId post ↔
      wpValues unit namespaceId frame state arguments.toList
        (valuesPost post fun argumentFrame argumentState values =>
          ∀ handle,
            resolveFunction? unit.unit namespaceId reference = some handle →
            wpFunction unit handle argumentState values.toArray
              (outcomePost
                (fun finalState results =>
                  post (registerReturnedLoan
                      (certificateLoanId? unit.unit namespaceId exprId) results
                      (applyPendingFrom argumentState.pending argumentFrame finalState).1)
                    (applyPendingFrom argumentState.pending argumentFrame finalState).2
                    (.value (packResults results)))
                (fun finalState kind thrown =>
                  post (applyPendingFrom argumentState.pending argumentFrame finalState).1
                    (applyPendingFrom argumentState.pending argumentFrame finalState).2
                    (.throw_ kind thrown)))) := by
  constructor
  · intro h result step
    cases result with
    | values state' frame' values' =>
        intro handle resolve_eq finalState outcome callee
        cases outcome with
        | returned results =>
            exact h _ _ _ (.callReturned (namespace_eq := namespace_eq)
              (expression_eq := expression_eq) (kind_eq := kind_eq)
              (operands := step) (resolve_eq := resolve_eq) (calleeStep := callee))
        | threw kind thrown =>
            exact h _ _ _ (.callThrew (namespace_eq := namespace_eq)
              (expression_eq := expression_eq) (kind_eq := kind_eq)
              (operands := step) (resolve_eq := resolve_eq) (calleeStep := callee))
    | control state' frame' control' =>
        exact h _ _ _ (.callArgumentsControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (operands := step))
  · intro h finalFrame finalState control step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals try obtain ⟨rfl, rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl⟩ := kind_eq
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals first
      | exact h (.values _ _ _) (by assumption) _ (by assumption) _
          (.returned _) (by assumption)
      | exact h (.values _ _ _) (by assumption) _ (by assumption) _
          (.threw _ _) (by assumption)
      | exact h (.values _ _ _) (by assumption) _ (.returned _) (by assumption)
      | exact h (.values _ _ _) (by assumption) _ (.threw _ _) (by assumption)
      | exact h (.control _ _ _) (by assumption)

theorem wpExpr_constructor {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {reference : QualifiedRef}
    {variant : Option String} {instantiations : Array GenericArgument}
    {arguments : Array ExprId} {surface : Option SurfaceSyntax}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation (.call (.constructor reference variant))
      instantiations arguments surface) :
    wpExpr unit namespaceId frame state exprId post ↔
      wpValues unit namespaceId frame state arguments.toList
        (valuesPost post fun finalFrame finalState values =>
          ∀ value,
            constructNominal? unit.unit namespaceId reference variant
              values.toArray = some value →
            post finalFrame finalState (.value value)) := by
  constructor
  · intro h result step
    cases result with
    | values state' frame' values' =>
        intro value construct_eq
        exact h _ _ _ (.constructorValue (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (operands := step) (construct_eq := construct_eq))
    | control state' frame' control' =>
        exact h _ _ _ (.constructorArgumentsControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (operands := step))
  · intro h finalFrame finalState control step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals try obtain ⟨rfl, rfl, rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl⟩ := kind_eq
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals first
      | exact h (.values _ _ _) (by assumption) _ (by assumption)
      | exact h (.control _ _ _) (by assumption)

theorem wpExpr_destructor {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {reference : QualifiedRef}
    {variant : Option String} {instantiations : Array GenericArgument}
    {arguments : Array ExprId} {surface : Option SurfaceSyntax}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation (.call (.destructor reference variant))
      instantiations arguments surface) :
    wpExpr unit namespaceId frame state exprId post ↔
      wpValues unit namespaceId frame state arguments.toList
        (valuesPost post fun finalFrame finalState values =>
          ∀ value fields, values = [value] →
            destructNominal? unit.unit namespaceId reference variant value
              = some fields →
            post finalFrame finalState (.value (packResults fields))) := by
  constructor
  · intro h result step
    cases result with
    | values state' frame' values' =>
        rintro value fields rfl destruct_eq
        exact h _ _ _ (.destructorValue (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (operands := step) (destruct_eq := destruct_eq))
    | control state' frame' control' =>
        exact h _ _ _ (.destructorArgumentsControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (operands := step))
  · intro h finalFrame finalState control step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals try obtain ⟨rfl, rfl, rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl⟩ := kind_eq
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals first
      | exact h (.values _ _ _) (by assumption) _ _ rfl (by assumption)
      | exact h (.control _ _ _) (by assumption)

theorem wpExpr_closure {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {reference : QualifiedRef}
    {instantiations : Array GenericArgument} {captures : Array ExprId}
    {surface : Option SurfaceSyntax}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation (.call (.closure reference))
      instantiations captures surface) :
    wpExpr unit namespaceId frame state exprId post ↔
      wpValues unit namespaceId frame state captures.toList
        (valuesPost post fun finalFrame finalState values =>
          ∀ handle,
            resolveFunction? unit.unit namespaceId reference = some handle →
            post finalFrame finalState
              (.value (.closure handle values.toArray))) := by
  constructor
  · intro h result step
    cases result with
    | values state' frame' values' =>
        intro handle resolve_eq
        exact h _ _ _ (.closureValue (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (operands := step) (resolve_eq := resolve_eq))
    | control state' frame' control' =>
        exact h _ _ _ (.closureArgumentsControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (operands := step))
  · intro h finalFrame finalState control step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals try obtain ⟨rfl, rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl⟩ := kind_eq
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals first
      | exact h (.values _ _ _) (by assumption) _ (by assumption)
      | exact h (.values _ _ _) (by assumption)
      | exact h (.control _ _ _) (by assumption)

theorem wpExpr_invoke {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr}
    {instantiations : Array GenericArgument} {arguments : Array ExprId}
    {surface : Option SurfaceSyntax}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation (.call .invoke)
      instantiations arguments surface) :
    wpExpr unit namespaceId frame state exprId post ↔
      wpValues unit namespaceId frame state arguments.toList
        (valuesPost post fun argumentFrame argumentState values =>
          ∀ handle captures rest,
            values = .closure handle captures :: rest →
            wpFunction unit handle argumentState (captures ++ rest.toArray)
              (outcomePost
                (fun finalState results =>
                  post (applyPendingFrom argumentState.pending argumentFrame finalState).1
                    (applyPendingFrom argumentState.pending argumentFrame finalState).2
                    (.value (packResults results)))
                (fun finalState kind thrown =>
                  post (applyPendingFrom argumentState.pending argumentFrame finalState).1
                    (applyPendingFrom argumentState.pending argumentFrame finalState).2
                    (.throw_ kind thrown)))) := by
  constructor
  · intro h result step
    cases result with
    | values state' frame' values' =>
        rintro handle captures rest rfl finalState outcome callee
        cases outcome with
        | returned results =>
            exact h _ _ _ (.invokeReturned (namespace_eq := namespace_eq)
              (expression_eq := expression_eq) (kind_eq := kind_eq)
              (operands := step) (calleeStep := callee))
        | threw kind thrown =>
            exact h _ _ _ (.invokeThrew (namespace_eq := namespace_eq)
              (expression_eq := expression_eq) (kind_eq := kind_eq)
              (operands := step) (calleeStep := callee))
    | control state' frame' control' =>
        exact h _ _ _ (.invokeArgumentsControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (operands := step))
  · intro h finalFrame finalState control step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals try obtain ⟨rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl⟩ := kind_eq
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals first
      | exact h (.values _ _ _) (by assumption) _ _ _ rfl _ (.returned _)
          (by assumption)
      | exact h (.values _ _ _) (by assumption) _ _ _ rfl _ (.threw _ _)
          (by assumption)
      | exact h (.control _ _ _) (by assumption)

/-- The place dispatcher owns no profile operations. -/
@[simp] theorem evaluatePlaceOperation?_profile {unit : ValidatedUnit}
    {ns : ValidatedNamespace} {resultType : TypeId} {site : ExprId}
    {value : ProfileValue}
    {targets : Array QualifiedRef} {arguments : Array RuntimeValue}
    {frame : RuntimeFrame} {state : RuntimeState} :
    evaluatePlaceOperation? unit ns resultType site (.profile value targets) arguments
      frame state = none := rfl

/-- The place dispatcher owns no global operations. -/
@[simp] theorem evaluatePlaceOperation?_global {unit : ValidatedUnit}
    {ns : ValidatedNamespace} {resultType : TypeId} {site : ExprId}
    {kind : GlobalKind}
    {arguments : Array RuntimeValue} {frame : RuntimeFrame}
    {state : RuntimeState} :
    evaluatePlaceOperation? unit ns resultType site (.global kind) arguments
      frame state = none := rfl

theorem wpExpr_profile {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {operation : ProfileValue}
    {targets : Array QualifiedRef} {instantiations : Array GenericArgument}
    {arguments : Array ExprId} {surface : Option SurfaceSyntax}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation (.profile operation targets)
      instantiations arguments surface) :
    wpExpr unit namespaceId frame state exprId post ↔
      wpValues unit namespaceId frame state arguments.toList
        (valuesPost post fun finalFrame finalState values =>
          (∀ value,
            evaluateProfileOperation? unit ns expression.typeId operation
              values.toArray = some (.ok value) →
            post finalFrame finalState (.value value)) ∧
          (∀ kind thrown,
            evaluateProfileOperation? unit ns expression.typeId operation
              values.toArray = some (.error (kind, thrown)) →
            post finalFrame finalState (.throw_ kind thrown))) := by
  constructor
  · intro h result step
    cases result with
    | values state' frame' values' =>
        refine ⟨?_, ?_⟩
        · intro value evaluate_eq
          exact h _ _ _ (.profileValue (namespace_eq := namespace_eq)
            (expression_eq := expression_eq) (kind_eq := kind_eq)
            (operands := step) (evaluate_eq := evaluate_eq))
        · intro kind thrown evaluate_eq
          exact h _ _ _ (.profileThrow (namespace_eq := namespace_eq)
            (expression_eq := expression_eq) (kind_eq := kind_eq)
            (operands := step) (evaluate_eq := evaluate_eq))
    | control state' frame' control' =>
        exact h _ _ _ (.profileArgumentsControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (operands := step))
  · intro h finalFrame finalState control step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals try obtain ⟨rfl, rfl, rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl⟩ := kind_eq
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals first
      | exact (h (.values _ _ _) (by assumption)).1 _ (by assumption)
      | exact (h (.values _ _ _) (by assumption)).2 _ _ (by assumption)
      | exact h (.control _ _ _) (by assumption)

theorem wpExpr_global {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {globalKind : GlobalKind}
    {instantiations : Array GenericArgument} {arguments : Array ExprId}
    {surface : Option SurfaceSyntax}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation (.global globalKind)
      instantiations arguments surface) :
    wpExpr unit namespaceId frame state exprId post ↔
      wpValues unit namespaceId frame state arguments.toList
        (valuesPost post fun argumentFrame argumentState values =>
          (∀ finalFrame finalState value,
            evaluateGlobalOperation? unit.unit ns expression.typeId exprId globalKind
              instantiations values.toArray argumentFrame argumentState
              = some (.value finalFrame finalState value) →
            post finalFrame finalState (.value value)) ∧
          (∀ finalFrame finalState kind thrown,
            evaluateGlobalOperation? unit.unit ns expression.typeId exprId globalKind
              instantiations values.toArray argumentFrame argumentState
              = some (.throw_ finalFrame finalState kind thrown) →
            post finalFrame finalState (.throw_ kind thrown))) := by
  constructor
  · intro h result step
    cases result with
    | values state' frame' values' =>
        refine ⟨?_, ?_⟩
        · intro finalFrame finalState value evaluate_eq
          exact h _ _ _ (.globalValue (namespace_eq := namespace_eq)
            (expression_eq := expression_eq) (kind_eq := kind_eq)
            (operands := step) (evaluate_eq := evaluate_eq))
        · intro finalFrame finalState kind thrown evaluate_eq
          exact h _ _ _ (.globalThrow (namespace_eq := namespace_eq)
            (expression_eq := expression_eq) (kind_eq := kind_eq)
            (operands := step) (evaluate_eq := evaluate_eq))
    | control state' frame' control' =>
        exact h _ _ _ (.globalArgumentsControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (operands := step))
  · intro h finalFrame finalState control step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals try obtain ⟨rfl, rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl⟩ := kind_eq
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals first
      | exact (h (.values _ _ _) (by assumption)).1 _ _ _ (by assumption)
      | exact (h (.values _ _ _) (by assumption)).2 _ _ _ _ (by assumption)
      | exact h (.control _ _ _) (by assumption)

/-- Operations evaluated by the place dispatcher. -/
def isPlaceOperation : Operation → Bool
  | .move _ | .copy _ | .borrow _ _ | .read _ | .write _ | .drop _
  | .reference _ | .data _ | .specification _ => true
  | .call (.extension _ _) => true
  | _ => false

theorem wpExpr_place {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {operation : Operation}
    {instantiations : Array GenericArgument} {arguments : Array ExprId}
    {surface : Option SurfaceSyntax}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .operation operation instantiations arguments
      surface)
    (operation_place : isPlaceOperation operation = true) :
    wpExpr unit namespaceId frame state exprId post ↔
      wpValues unit namespaceId frame state arguments.toList
        (valuesPost post fun argumentFrame argumentState values =>
          ∀ finalFrame finalState value,
            evaluatePlaceOperation? unit.unit ns expression.typeId exprId operation
              values.toArray argumentFrame argumentState
              = some (finalFrame, finalState, value) →
            post finalFrame finalState (.value value)) := by
  constructor
  · intro h result step
    cases result with
    | values state' frame' values' =>
        intro finalFrame finalState value evaluate_eq
        exact h _ _ _ (.operationValue (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (operands := step) (evaluate_eq := evaluate_eq))
    | control state' frame' control' =>
        exact h _ _ _ (.operationArgumentsControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (operands := step))
  · intro h finalFrame finalState control step
    cases step <;>
      try (exfalso
           simp_all
           try obtain ⟨rfl, rfl, rfl, rfl⟩ := kind_eq
           try obtain ⟨rfl, rfl, rfl⟩ := kind_eq
           try obtain ⟨rfl, rfl⟩ := kind_eq
           try obtain rfl := kind_eq
           simp [isPlaceOperation] at operation_place
           done)
    all_goals simp_all
    all_goals try obtain ⟨rfl, rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl, rfl⟩ := kind_eq
    all_goals try obtain ⟨rfl, rfl⟩ := kind_eq
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals first
      | exact h (.values _ _ _) (by assumption) _ _ _ (by assumption)
      | exact h (.control _ _ _) (by assumption)

/-! ## Calculation rules: match and loops -/

theorem wpExpr_match {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {scrutinee : ExprId}
    {arms : Array MatchArm}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .match_ scrutinee arms) :
    wpExpr unit namespaceId frame state exprId post ↔
      wpExpr unit namespaceId frame state scrutinee
        (valueOr post fun scrutineeFrame scrutineeState value =>
          wpArms unit namespaceId ns scrutineeFrame scrutineeState value
            arms.toList post) := by
  constructor
  · intro h scrutineeFrame scrutineeState control step
    cases control with
    | value value =>
        intro finalFrame finalState control armStep
        exact h _ _ _ (.matchValue (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (scrutinee_step := step) (arm_step := armStep))
    | break_ nest value =>
        exact h _ _ _ (.matchControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (scrutinee_step := step) (abrupt := by simp))
    | continue_ nest =>
        exact h _ _ _ (.matchControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (scrutinee_step := step) (abrupt := by simp))
    | return_ values =>
        exact h _ _ _ (.matchControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (scrutinee_step := step) (abrupt := by simp))
    | throw_ kind arguments =>
        exact h _ _ _ (.matchControl (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (scrutinee_step := step) (abrupt := by simp))
  · intro h finalFrame finalState control step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals try obtain ⟨rfl, rfl⟩ := kind_eq
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals first
      | exact h _ _ (.value _) (by assumption) _ _ _ (by assumption)
      | (have abrupt : Abrupt control := by assumption
         have propagated := h _ _ _ (by assumption :
           EvalExpr unit namespaceId frame state _ finalFrame finalState control)
         cases abrupt <;> exact propagated)

/-- No arm matches: evaluation is stuck, so every postcondition holds. -/
theorem wpArms_nil {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {ns : ValidatedNamespace} {frame : RuntimeFrame} {state : RuntimeState}
    {value : RuntimeValue}
    {post : RuntimeFrame → RuntimeState → Control → Prop} :
    wpArms unit namespaceId ns frame state value [] post :=
  fun _ _ _ step => nomatch step

theorem wpArms_cons {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {ns : ValidatedNamespace} {frame : RuntimeFrame} {state : RuntimeState}
    {value : RuntimeValue} {arm : MatchArm} {arms : List MatchArm}
    {post : RuntimeFrame → RuntimeState → Control → Prop} :
    wpArms unit namespaceId ns frame state value (arm :: arms) post ↔
      (bindPattern unit.unit ns frame arm.pattern value = none →
        wpArms unit namespaceId ns frame state value arms post) ∧
      (∀ armFrame, bindPattern unit.unit ns frame arm.pattern value = some armFrame →
        (arm.guard = none →
          wpExpr unit namespaceId armFrame state arm.body post) ∧
        (∀ guard, arm.guard = some guard →
          wpExpr unit namespaceId armFrame state guard
            (valueOr post fun guardFrame guardState guardValue =>
              (guardValue = .bool true →
                wpExpr unit namespaceId guardFrame guardState arm.body post) ∧
              (guardValue = .bool false →
                wpArms unit namespaceId ns frame state value arms post)))) := by
  constructor
  · intro h
    refine ⟨?_, ?_⟩
    · intro bind_none finalFrame finalState control tailStep
      exact h _ _ _ (.reject (bind_eq := bind_none) (tail_step := tailStep))
    · intro armFrame bind_eq
      refine ⟨?_, ?_⟩
      · intro guard_none finalFrame finalState control bodyStep
        exact h _ _ _ (.noGuard (arms := arms) (guard_eq := guard_none)
          (bind_eq := bind_eq) (body_step := bodyStep))
      · intro guard guard_eq guardFrame guardState guardControl guardStep
        cases guardControl with
        | value guardValue =>
            refine ⟨?_, ?_⟩ <;> rintro rfl <;>
              intro finalFrame finalState control innerStep
            · exact h _ _ _ (.guardTrue (arms := arms) (guard_eq := guard_eq)
                (bind_eq := bind_eq) (guard_step := guardStep)
                (body_step := innerStep))
            · exact h _ _ _ (.guardFalse (guard_eq := guard_eq)
                (bind_eq := bind_eq) (guard_step := guardStep)
                (tail_step := innerStep))
        | break_ nest value =>
            exact h _ _ _ (.guardControl (arms := arms) (guard_eq := guard_eq)
              (bind_eq := bind_eq) (guard_step := guardStep)
              (abrupt := by simp))
        | continue_ nest =>
            exact h _ _ _ (.guardControl (arms := arms) (guard_eq := guard_eq)
              (bind_eq := bind_eq) (guard_step := guardStep)
              (abrupt := by simp))
        | return_ values =>
            exact h _ _ _ (.guardControl (arms := arms) (guard_eq := guard_eq)
              (bind_eq := bind_eq) (guard_step := guardStep)
              (abrupt := by simp))
        | throw_ kind arguments =>
            exact h _ _ _ (.guardControl (arms := arms) (guard_eq := guard_eq)
              (bind_eq := bind_eq) (guard_step := guardStep)
              (abrupt := by simp))
  · intro h finalFrame finalState control step
    cases step with
    | reject _ _ _ _ _ _ _ _ _ _ bind_eq tail_step =>
        exact h.1 bind_eq _ _ _ tail_step
    | noGuard _ _ _ _ _ _ _ _ _ _ _ guard_eq bind_eq body_step =>
        exact (h.2 _ bind_eq).1 guard_eq _ _ _ body_step
    | guardControl _ _ _ _ _ _ _ _ _ _ _ _ guard_eq bind_eq guard_step abrupt =>
        have propagated := ((h.2 _ bind_eq).2 _ guard_eq) _ _ _ guard_step
        cases abrupt <;> exact propagated
    | guardTrue _ _ _ _ _ _ _ _ _ _ _ _ _ _ guard_eq bind_eq guard_step
        body_step =>
        exact (((h.2 _ bind_eq).2 _ guard_eq) _ _ (.value (.bool true))
          guard_step).1 rfl _ _ _ body_step
    | guardFalse _ _ _ _ _ _ _ _ _ _ _ _ _ _ guard_eq bind_eq guard_step
        tail_step =>
        exact (((h.2 _ bind_eq).2 _ guard_eq) _ _ (.value (.bool false))
          guard_step).2 rfl _ _ _ tail_step

/-- Unfold one loop step: the body runs, a value or direct `continue`
re-enters the loop, `break 0` exits with the loop's value, and outer or
function-level control unnests and propagates. This is an unfolding
equivalence, not a terminating calculation rule — loop verification composes
it with an invariant argument. -/
theorem wpExpr_loop {unit : ExecutableUnit} {namespaceId : NamespaceId}
    {frame : RuntimeFrame} {state : RuntimeState} {exprId : ExprId}
    {ns : ValidatedNamespace} {expression : Expr} {label : Option String}
    {body : ExprId} {post : RuntimeFrame → RuntimeState → Control → Prop}
    (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
    (expression_eq : ns.expressions[exprId.index]? = some expression)
    (kind_eq : expression.kind = .loop label body) :
    wpExpr unit namespaceId frame state exprId post ↔
      wpExpr unit namespaceId frame state body
        (fun bodyFrame bodyState control =>
          match control with
          | .value _ =>
              wpExpr unit namespaceId bodyFrame bodyState exprId post
          | .continue_ 0 =>
              wpExpr unit namespaceId bodyFrame bodyState exprId post
          | .continue_ (nest + 1) => post bodyFrame bodyState (.continue_ nest)
          | .break_ 0 value =>
              post bodyFrame bodyState (.value (value.getD .unit))
          | .break_ (nest + 1) value =>
              post bodyFrame bodyState (.break_ nest value)
          | .return_ values => post bodyFrame bodyState (.return_ values)
          | .throw_ kind arguments =>
              post bodyFrame bodyState (.throw_ kind arguments)) := by
  constructor
  · intro h bodyFrame bodyState control step
    cases control with
    | value value =>
        intro finalFrame finalState control repeatStep
        exact h _ _ _ (.loopRepeatValue (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (body_step := step) (repeat_step := repeatStep))
    | continue_ nest =>
        cases nest with
        | zero =>
            intro finalFrame finalState control repeatStep
            exact h _ _ _ (.loopRepeatContinue (namespace_eq := namespace_eq)
              (expression_eq := expression_eq) (kind_eq := kind_eq)
              (body_step := step) (repeat_step := repeatStep))
        | succ nest =>
            exact h _ _ _ (.loopOuterContinue (namespace_eq := namespace_eq)
              (expression_eq := expression_eq) (kind_eq := kind_eq)
              (body_step := step))
    | break_ nest value =>
        cases nest with
        | zero =>
            exact h _ _ _ (.loopBreak (namespace_eq := namespace_eq)
              (expression_eq := expression_eq) (kind_eq := kind_eq)
              (body_step := step))
        | succ nest =>
            exact h _ _ _ (.loopOuterBreak (namespace_eq := namespace_eq)
              (expression_eq := expression_eq) (kind_eq := kind_eq)
              (body_step := step))
    | return_ values =>
        exact h _ _ _ (.loopReturn (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (body_step := step))
    | throw_ kind arguments =>
        exact h _ _ _ (.loopThrow (namespace_eq := namespace_eq)
          (expression_eq := expression_eq) (kind_eq := kind_eq)
          (body_step := step))
  · intro h finalFrame finalState control step
    cases step <;> try (exfalso; simp_all; done)
    all_goals simp_all
    all_goals try obtain ⟨rfl, rfl⟩ := kind_eq
    all_goals try obtain rfl := kind_eq
    all_goals subst_vars
    all_goals first
      | exact h _ _ (.value _) (by assumption) _ _ _ (by assumption)
      | exact h _ _ (.continue_ 0) (by assumption) _ _ _ (by assumption)
      | exact h _ _ (.continue_ (_ + 1)) (by assumption)
      | exact h _ _ (.break_ 0 _) (by assumption)
      | exact h _ _ (.break_ (_ + 1) _) (by assumption)
      | exact h _ _ (.return_ _) (by assumption)
      | exact h _ _ (.throw_ _ _) (by assumption)

/-! ## Unconditional step equations

The rules above are conditioned on arena lookups, so they instantiate
metavariables and cannot drive `simp`.  The step definitions below package
the same content as total functions of the expression node, so
`wpExpr_eq` and its siblings rewrite unconditionally and the matches
iota-reduce once the unit is a literal.  A construct with no derivation —
an out-of-range lookup, a stuck arm list — steps to `True`, which is what
the transformer means there.

A loop steps to `wpLoopExpr`, a distinct head, so symbolic execution stops
at a loop instead of unfolding it forever; loop verification supplies the
invariant through `wpExpr_loop`. -/

/-- Weakest precondition of a loop expression: the transformer under a head
`simp` will not unfold. -/
def wpLoopExpr (unit : ExecutableUnit) (namespaceId : NamespaceId)
    (frame : RuntimeFrame) (state : RuntimeState) (exprId : ExprId)
    (post : RuntimeFrame → RuntimeState → Control → Prop) : Prop :=
  wpExpr unit namespaceId frame state exprId post

theorem wpLoopExpr_eq (unit : ExecutableUnit) (namespaceId : NamespaceId)
    (frame : RuntimeFrame) (state : RuntimeState) (exprId : ExprId)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpLoopExpr unit namespaceId frame state exprId post
      = wpExpr unit namespaceId frame state exprId post := rfl

/-- Reduction unfolds a conditional into the recursor it is defined by;
folding it back keeps a decided branch splittable and readable. -/
theorem decidableRec_eq_dite {c : Prop} {α : Sort u} [decision : Decidable c]
    (onFalse : ¬c → α) (onTrue : c → α) :
    (Decidable.rec (motive := fun _ => α) onFalse onTrue decision)
      = dite c onTrue onFalse := rfl

/-! An obligation over an evaluator result reads better, and drives
symbolic execution further, as a match on that result: the value it names
is then substituted rather than carried as an equation. -/

/-- Rewriting under an operand-list postcondition. -/
theorem wpValues_congr {unit namespaceId frame state expressions}
    {post post' : ValuesResult → Prop}
    (pointwise : ∀ result, post result = post' result) :
    wpValues unit namespaceId frame state expressions post
      = wpValues unit namespaceId frame state expressions post' := by
  have : post = post' := funext pointwise
  rw [this]

/-- Rewriting under the value case of an operand-list postcondition. -/
theorem valuesPost_congr {post : RuntimeFrame → RuntimeState → Control → Prop}
    {onValues onValues' : RuntimeFrame → RuntimeState → List RuntimeValue → Prop}
    (pointwise : ∀ frame state values,
      onValues frame state values = onValues' frame state values)
    (result : ValuesResult) :
    valuesPost post onValues result = valuesPost post onValues' result := by
  have : onValues = onValues' := funext fun frame => funext fun state =>
    funext fun values => pointwise frame state values
  rw [this]

theorem forall_some_iff_match {α : Type u} (result : Option α) (post : α → Prop) :
    (∀ value, result = some value → post value) ↔
      (match result with
       | some value => post value
       | none => True) := by
  cases result <;> simp

theorem forall_outcome_iff_match
    (result : Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue))
    (onValue : RuntimeValue → Prop)
    (onThrow : ThrowKind → Array RuntimeValue → Prop) :
    ((∀ value, result = some (.ok value) → onValue value) ∧
      (∀ kind thrown, result = some (.error (kind, thrown)) → onThrow kind thrown)) ↔
      (match result with
       | some (.ok value) => onValue value
       | some (.error (kind, thrown)) => onThrow kind thrown
       | none => True) := by
  cases result with
  | none => simp
  | some outcome =>
      cases outcome with
      | ok value => simp
      | error failure => obtain ⟨kind, thrown⟩ := failure; simp

theorem forall_place_iff_match
    (result : Option (RuntimeFrame × RuntimeState × RuntimeValue))
    (post : RuntimeFrame → RuntimeState → RuntimeValue → Prop) :
    (∀ finalFrame finalState value,
        result = some (finalFrame, finalState, value) →
        post finalFrame finalState value) ↔
      (match result with
       | some (finalFrame, finalState, value) => post finalFrame finalState value
       | none => True) := by
  cases result with
  | none => simp
  | some triple =>
      obtain ⟨finalFrame, finalState, value⟩ := triple
      simp

/-! A global-storage obligation quantifies over the outcome the primitive
produces.  These put it in the goal-shaped `match` form the stepping tactic
splits, so the slot search a symbolic state leaves stuck is resolved by an
ordinary case split on the goal rather than on a hypothesis. -/

theorem forall_global_value_iff_match
    (result : Option SemanticOperations.GlobalOperationResult)
    (post : RuntimeFrame → RuntimeState → RuntimeValue → Prop) :
    (∀ finalFrame finalState value,
        result = some (.value finalFrame finalState value) →
        post finalFrame finalState value) ↔
      (match result with
       | some (.value finalFrame finalState value) => post finalFrame finalState value
       | _ => True) := by
  cases result with
  | none => simp
  | some outcome => cases outcome <;> simp

theorem forall_global_throw_iff_match
    (result : Option SemanticOperations.GlobalOperationResult)
    (post : RuntimeFrame → RuntimeState → ThrowKind → Array RuntimeValue → Prop) :
    (∀ finalFrame finalState kind thrown,
        result = some (.throw_ finalFrame finalState kind thrown) →
        post finalFrame finalState kind thrown) ↔
      (match result with
       | some (.throw_ finalFrame finalState kind thrown) =>
           post finalFrame finalState kind thrown
       | _ => True) := by
  cases result with
  | none => simp
  | some outcome =>
      cases outcome with
      | value => simp
      | throw_ frame state kind thrown =>
          simp only [Option.some.injEq]
          constructor
          · intro h
            exact h frame state kind thrown (by simp)
          · rintro h _ _ _ _ equation
            injection equation with frame_eq state_eq kind_eq thrown_eq
            subst frame_eq; subst state_eq; subst kind_eq; subst thrown_eq
            exact h

/-- Symbolic step of an operation the place dispatcher evaluates. -/
def wpPlaceStep (unit : ExecutableUnit) (namespaceId : NamespaceId)
    (frame : RuntimeFrame) (state : RuntimeState) (ns : ValidatedNamespace)
    (expression : Expr) (site : ExprId) (operation : Operation)
    (arguments : Array ExprId)
    (post : RuntimeFrame → RuntimeState → Control → Prop) : Prop :=
  wpValues unit namespaceId frame state arguments.toList
    (valuesPost post fun argumentFrame argumentState values =>
      ∀ finalFrame finalState value,
        evaluatePlaceOperation? unit.unit ns expression.typeId site operation
          values.toArray argumentFrame argumentState
          = some (finalFrame, finalState, value) →
        post finalFrame finalState (.value value))

/-- One symbolic step of an expression transformer. -/
def wpExprStep (unit : ExecutableUnit) (namespaceId : NamespaceId)
    (frame : RuntimeFrame) (state : RuntimeState) (exprId : ExprId)
    (post : RuntimeFrame → RuntimeState → Control → Prop) : Prop :=
  match unit.unit.namespaces[namespaceId.index]? with
  | none => True
  | some ns =>
      match ns.expressions[exprId.index]? with
      | none => True
      | some expression =>
          match expression.kind with
          | .value literal _ =>
              ∀ value, constValue? literal = some value →
                post frame state (.value value)
          | .localVar localId =>
              ∀ value, readLocal? frame localId = some value →
                post frame state (.value value)
          | .continue_ nest => post frame state (.continue_ nest)
          | .break_ nest none => post frame state (.break_ nest none)
          | .spec _ => post frame state (.value .unit)
          | .constant reference =>
              ∀ handle, resolveConstant? unit.unit namespaceId reference = some handle →
                ∀ targetNs declaration,
                  unit.unit.namespaces[handle.namespaceId.index]? = some targetNs →
                  targetNs.constants[handle.constantId]? = some declaration →
                  wpExpr unit handle.namespaceId { locals := #[] } state
                    declaration.value
                    (fun _ finalState control => post frame finalState control)
          | .break_ nest (some child) =>
              wpExpr unit namespaceId frame state child
                (valueOr post fun childFrame childState value =>
                  post childFrame childState (.break_ nest (some value)))
          | .block statements none =>
              wpStatements unit namespaceId frame state statements.toList
                (blockPost post fun doneFrame doneState =>
                  post doneFrame doneState (.value .unit))
          | .block statements (some result) =>
              wpStatements unit namespaceId frame state statements.toList
                (blockPost post fun doneFrame doneState =>
                  wpExpr unit namespaceId doneFrame doneState result post)
          | .letDecl _ none body => wpExpr unit namespaceId frame state body post
          | .letDecl pattern (some initializer) body =>
              wpExpr unit namespaceId frame state initializer
                (valueOr post fun initializedFrame initializedState value =>
                  ∀ boundFrame,
                    bindPattern unit.unit ns initializedFrame pattern value = some boundFrame →
                    wpExpr unit namespaceId boundFrame initializedState body post)
          | .ifElse condition thenBranch (some elseBranch) =>
              wpExpr unit namespaceId frame state condition
                (valueOr post fun conditionFrame conditionState value =>
                  (value = .bool true →
                    wpExpr unit namespaceId conditionFrame conditionState thenBranch post) ∧
                  (value = .bool false →
                    wpExpr unit namespaceId conditionFrame conditionState elseBranch post))
          | .ifElse condition thenBranch none =>
              wpExpr unit namespaceId frame state condition
                (valueOr post fun conditionFrame conditionState value =>
                  (value = .bool true →
                    wpExpr unit namespaceId conditionFrame conditionState thenBranch post) ∧
                  (value = .bool false →
                    post conditionFrame conditionState (.value .unit)))
          | .match_ scrutinee arms =>
              wpExpr unit namespaceId frame state scrutinee
                (valueOr post fun scrutineeFrame scrutineeState value =>
                  wpArms unit namespaceId ns scrutineeFrame scrutineeState value
                    arms.toList post)
          | .loop _ _ => wpLoopExpr unit namespaceId frame state exprId post
          | .return_ values =>
              wpValues unit namespaceId frame state values.toList
                (valuesPost post fun finalFrame finalState runtimeValues =>
                  post finalFrame finalState (.return_ runtimeValues.toArray))
          | .throw_ kind arguments =>
              wpValues unit namespaceId frame state arguments.toList
                (valuesPost post fun finalFrame finalState runtimeValues =>
                  post finalFrame finalState (.throw_ kind runtimeValues.toArray))
          | .assign place child =>
              wpExpr unit namespaceId frame state child
                (valueOr post fun childFrame childState value =>
                  ∀ resolved finalFrame finalState,
                    resolvePlace? unit.unit ns childFrame childState place
                      = some resolved →
                    writeRuntimePlace? childFrame childState resolved value
                      = some (finalFrame, finalState) →
                    post finalFrame finalState (.value .unit))
          | .assignPattern pattern child =>
              wpExpr unit namespaceId frame state child
                (valueOr post fun childFrame childState value =>
                  ∀ finalFrame,
                    bindPattern unit.unit ns childFrame pattern value = some finalFrame →
                    post finalFrame childState (.value .unit))
          | .quantifier .. => True
          | .operation operation instantiations arguments _ =>
              match operation with
              | .call (.function reference) =>
                  wpValues unit namespaceId frame state arguments.toList
                    (valuesPost post fun argumentFrame argumentState values =>
                      ∀ handle,
                        resolveFunction? unit.unit namespaceId reference = some handle →
                        wpFunction unit handle argumentState values.toArray
                          (outcomePost
                            (fun finalState results =>
                              post (registerReturnedLoan
                                  (certificateLoanId? unit.unit namespaceId exprId) results
                                  (applyPendingFrom argumentState.pending argumentFrame finalState).1)
                                (applyPendingFrom argumentState.pending argumentFrame finalState).2
                                (.value (packResults results)))
                            (fun finalState kind thrown =>
                              post (applyPendingFrom argumentState.pending argumentFrame finalState).1
                                (applyPendingFrom argumentState.pending argumentFrame finalState).2
                                (.throw_ kind thrown))))
              | .call (.constructor reference variant) =>
                  wpValues unit namespaceId frame state arguments.toList
                    (valuesPost post fun finalFrame finalState values =>
                      ∀ value,
                        constructNominal? unit.unit namespaceId reference variant
                          values.toArray = some value →
                        post finalFrame finalState (.value value))
              | .call (.destructor reference variant) =>
                  wpValues unit namespaceId frame state arguments.toList
                    (valuesPost post fun finalFrame finalState values =>
                      ∀ value fields, values = [value] →
                        destructNominal? unit.unit namespaceId reference variant value
                          = some fields →
                        post finalFrame finalState (.value (packResults fields)))
              | .call (.closure reference) =>
                  wpValues unit namespaceId frame state arguments.toList
                    (valuesPost post fun finalFrame finalState values =>
                      ∀ handle,
                        resolveFunction? unit.unit namespaceId reference = some handle →
                        post finalFrame finalState
                          (.value (.closure handle values.toArray)))
              | .call .invoke =>
                  wpValues unit namespaceId frame state arguments.toList
                    (valuesPost post fun argumentFrame argumentState values =>
                      ∀ handle captures rest,
                        values = .closure handle captures :: rest →
                        wpFunction unit handle argumentState (captures ++ rest.toArray)
                          (outcomePost
                            (fun finalState results =>
                              post (applyPendingFrom argumentState.pending argumentFrame finalState).1
                                (applyPendingFrom argumentState.pending argumentFrame finalState).2
                                (.value (packResults results)))
                            (fun finalState kind thrown =>
                              post (applyPendingFrom argumentState.pending argumentFrame finalState).1
                                (applyPendingFrom argumentState.pending argumentFrame finalState).2
                                (.throw_ kind thrown))))
              | .assert =>
                  wpValues unit namespaceId frame state arguments.toList
                    (valuesPost post fun finalFrame finalState values =>
                      (values = [.bool true] →
                        post finalFrame finalState (.value .unit)) ∧
                      (values = [.bool false] →
                        post finalFrame finalState (.throw_ .abort #[])))
              | .primitive primitive =>
                  wpValues unit namespaceId frame state arguments.toList
                    (valuesPost post fun finalFrame finalState values =>
                      (∀ value,
                        evaluatePrimitiveOperation? ns expression.typeId primitive
                          values.toArray unit.targetPointerWidth = some (.ok value) →
                        post finalFrame finalState (.value value)) ∧
                      (∀ kind thrown,
                        evaluatePrimitiveOperation? ns expression.typeId primitive
                          values.toArray unit.targetPointerWidth
                          = some (.error (kind, thrown)) →
                        post finalFrame finalState (.throw_ kind thrown)))
              | .profile value targets =>
                  wpValues unit namespaceId frame state arguments.toList
                    (valuesPost post fun finalFrame finalState values =>
                      (∀ result,
                        evaluateProfileOperation? unit ns expression.typeId value
                          values.toArray = some (.ok result) →
                        post finalFrame finalState (.value result)) ∧
                      (∀ kind thrown,
                        evaluateProfileOperation? unit ns expression.typeId value
                          values.toArray = some (.error (kind, thrown)) →
                        post finalFrame finalState (.throw_ kind thrown)))
              | .global globalKind =>
                  wpValues unit namespaceId frame state arguments.toList
                    (valuesPost post fun argumentFrame argumentState values =>
                      (∀ finalFrame finalState value,
                        evaluateGlobalOperation? unit.unit ns expression.typeId exprId globalKind
                          instantiations values.toArray argumentFrame argumentState
                          = some (.value finalFrame finalState value) →
                        post finalFrame finalState (.value value)) ∧
                      (∀ finalFrame finalState kind thrown,
                        evaluateGlobalOperation? unit.unit ns expression.typeId exprId globalKind
                          instantiations values.toArray argumentFrame argumentState
                          = some (.throw_ finalFrame finalState kind thrown) →
                        post finalFrame finalState (.throw_ kind thrown)))
              | .call (.extension value targets) =>
                  wpPlaceStep unit namespaceId frame state ns expression exprId
                    (.call (.extension value targets)) arguments post
              | .move place => wpPlaceStep unit namespaceId frame state ns expression exprId (.move place) arguments post
              | .copy place => wpPlaceStep unit namespaceId frame state ns expression exprId (.copy place)
                  arguments post
              | .borrow kind place => wpPlaceStep unit namespaceId frame state ns expression exprId (.borrow kind place)
                  arguments post
              | .read place => wpPlaceStep unit namespaceId frame state ns expression exprId (.read place)
                  arguments post
              | .write place => wpPlaceStep unit namespaceId frame state ns expression exprId (.write place)
                  arguments post
              | .drop place => wpPlaceStep unit namespaceId frame state ns expression exprId (.drop place)
                  arguments post
              | .reference kind => wpPlaceStep unit namespaceId frame state ns expression exprId (.reference kind)
                  arguments post
              | .data kind => wpPlaceStep unit namespaceId frame state ns expression exprId (.data kind)
                  arguments post
              | .specification kind => wpPlaceStep unit namespaceId frame state ns expression exprId (.specification kind)
                  arguments post

/-- Symbolic execution of one expression: an unconditional rewrite whose
right-hand side reduces once the unit and expression are literals. -/
theorem wpExpr_eq (unit : ExecutableUnit) (namespaceId : NamespaceId)
    (frame : RuntimeFrame) (state : RuntimeState) (exprId : ExprId)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr unit namespaceId frame state exprId post
      = wpExprStep unit namespaceId frame state exprId post := by
  apply propext
  unfold wpExprStep
  split
  case _ missing =>
    constructor
    · intro; trivial
    · intro _ finalFrame finalState control step
      exfalso
      cases step <;> simp_all
  case _ ns namespace_eq =>
  split
  case _ missing =>
    constructor
    · intro; trivial
    · intro _ finalFrame finalState control step
      exfalso
      cases step <;> simp_all
  case _ expression expression_eq =>
  split
  all_goals rename_i kind_eq
  all_goals first
    | exact wpExpr_value namespace_eq expression_eq kind_eq
    | exact wpExpr_localVar namespace_eq expression_eq kind_eq
    | exact wpExpr_continue namespace_eq expression_eq kind_eq
    | exact wpExpr_breakNone namespace_eq expression_eq kind_eq
    | exact wpExpr_spec namespace_eq expression_eq kind_eq
    | exact wpExpr_constant namespace_eq expression_eq kind_eq
    | exact wpExpr_breakValue namespace_eq expression_eq kind_eq
    | exact wpExpr_blockUnit namespace_eq expression_eq kind_eq
    | exact wpExpr_blockResult namespace_eq expression_eq kind_eq
    | exact wpExpr_letNoValue namespace_eq expression_eq kind_eq
    | exact wpExpr_letValue namespace_eq expression_eq kind_eq
    | exact wpExpr_ifElse namespace_eq expression_eq kind_eq
    | exact wpExpr_ifNoElse namespace_eq expression_eq kind_eq
    | exact wpExpr_match namespace_eq expression_eq kind_eq
    | exact wpExpr_return namespace_eq expression_eq kind_eq
    | exact wpExpr_throw namespace_eq expression_eq kind_eq
    | exact wpExpr_assign namespace_eq expression_eq kind_eq
    | exact wpExpr_assignPattern namespace_eq expression_eq kind_eq
    | rfl
    | (constructor
       · intro; trivial
       · intro _ finalFrame finalState control step
         exfalso
         cases step <;> simp_all)
    | (split
       all_goals first
         | exact wpExpr_call namespace_eq expression_eq kind_eq
         | exact wpExpr_constructor namespace_eq expression_eq kind_eq
         | exact wpExpr_destructor namespace_eq expression_eq kind_eq
         | exact wpExpr_closure namespace_eq expression_eq kind_eq
         | exact wpExpr_invoke namespace_eq expression_eq kind_eq
         | exact wpExpr_assert namespace_eq expression_eq kind_eq
         | exact wpExpr_primitive namespace_eq expression_eq kind_eq
         | exact wpExpr_profile namespace_eq expression_eq kind_eq
         | exact wpExpr_global namespace_eq expression_eq kind_eq
         | (refine wpExpr_place namespace_eq expression_eq kind_eq ?_
            simp [isPlaceOperation]))

/-- No arm can match, so the transformer holds vacuously. -/
theorem wpArms_nil_eq (unit : ExecutableUnit) (namespaceId : NamespaceId)
    (ns : ValidatedNamespace) (frame : RuntimeFrame) (state : RuntimeState)
    (value : RuntimeValue)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpArms unit namespaceId ns frame state value [] post = True := by
  simp only [eq_iff_iff, iff_true]
  exact wpArms_nil

/-- One symbolic step of a function transformer. -/
def wpFunctionStep (unit : ExecutableUnit) (handle : FunctionHandle)
    (initialState : RuntimeState) (arguments : Array RuntimeValue)
    (post : RuntimeState → Outcome → Prop) : Prop :=
  match unit.unit.namespaces[handle.namespaceId.index]? with
  | none => True
  | some ns =>
      match ns.functions[handle.functionId.index]? with
      | none => True
      | some declaration =>
          match declaration.body with
          | .absent => True
          | .structured root =>
              match initialFrame? declaration arguments with
              | none => True
              | some frame =>
                  wpExpr unit handle.namespaceId frame initialState root
                    fun finalFrame evaluatedState control =>
                      ∀ outcome,
                        finishControl? declaration.signature.results.size control
                          = some outcome →
                        post
                          (finalizeFunctionState unit declaration.profile
                            initialState evaluatedState finalFrame outcome)
                          outcome

/-- Symbolic execution of one invocation: an unconditional rewrite. -/
theorem wpFunction_eq (unit : ExecutableUnit) (handle : FunctionHandle)
    (initialState : RuntimeState) (arguments : Array RuntimeValue)
    (post : RuntimeState → Outcome → Prop) :
    wpFunction unit handle initialState arguments post
      = wpFunctionStep unit handle initialState arguments post := by
  apply propext
  unfold wpFunctionStep
  split
  case _ missing =>
    constructor
    · intro; trivial
    · intro _ finalState outcome step
      exfalso
      cases step <;> simp_all
  case _ ns namespace_eq =>
  split
  case _ missing =>
    constructor
    · intro; trivial
    · intro _ finalState outcome step
      exfalso
      cases step <;> simp_all
  case _ declaration declaration_eq =>
  split
  case _ body_eq =>
    constructor
    · intro; trivial
    · intro _ finalState outcome step
      exfalso
      cases step <;> simp_all
  case _ root body_eq =>
  split
  case _ missing =>
    constructor
    · intro; trivial
    · intro _ finalState outcome step
      exfalso
      cases step <;> simp_all
  case _ frame frame_eq =>
    exact wpFunction_body namespace_eq declaration_eq frame_eq body_eq

/-! ## Normalization inventory

`lir_wp_norm` holds the rules that always terminate: operand and statement
list traversal, arm selection, and the routing combinators.  The step
equations `wpExpr_eq` and `wpFunction_eq` are deliberately absent — they
are applied one node at a time by the stepping tactic, because their
right-hand sides mention the transformer again and only whnf of a literal
unit makes the next node concrete.  The loop head `wpLoopExpr` is likewise
absent, so execution stops where an invariant is required. -/

attribute [lir_wp_norm]
  decidableRec_eq_dite
  forall_some_iff_match
  forall_outcome_iff_match
  forall_place_iff_match
  forall_global_value_iff_match
  forall_global_throw_iff_match

  wpValues_nil
  wpValues_cons
  wpStatements_nil
  wpStatements_cons
  wpArms_nil_eq
  wpArms_cons
  valueOr_value
  valueOr_break
  valueOr_continue
  valueOr_return
  valueOr_throw
  consValues_values
  consValues_control
  blockPost_done
  blockPost_control
  valuesPost_values
  valuesPost_control
  outcomePost_returned
  outcomePost_threw

/-! Symbolic execution reduces the arena lookups of a literal unit by
ordinary whnf.  The transformers are sealed afterwards so that reduction
stops at a transformer head instead of unfolding it into its defining
quantifier; the step equations above remain the way to advance. -/

attribute [irreducible] wpExpr wpValues wpStatements wpArms wpFunction wpLoopExpr

/-! `lir_data_norm` is the data inventory: once symbolic execution has
reached a concrete program point, these unfold the frame reads, literal
reification, and pure evaluators the remaining obligations mention. -/

attribute [lir_data_norm]
  SemanticOperations.globalValue
  SemanticOperations.globalValue?
  SemanticOperations.globalExists
  SemanticOperations.globalKey
  SemanticOperations.readRoot?
  SemanticOperations.readRuntimePlace?
  SemanticOperations.readProjections?
  SemanticOperations.writeRuntimePlace?
  SemanticOperations.writeProjections?
  SemanticOperations.writeRoot?
  RuntimeValue.field
  RuntimeValue.asInt
  RuntimeValue.asBool
  RuntimeValue.asString
  RuntimeValue.storageKey
  RuntimeValue.storageKey?

/-! The typed storage view: unfolding a representation hypothesis in the
closing normalization exposes its per-key equation, which routes every
remaining runtime read of a represented family onto the typed contents the
contract quantifies over.  The certificate unfolds turn a destructured
`SpecInt`'s range certificate into the arithmetic bounds `omega` reads. -/

attribute [lir_data_norm]
  FamilyRepresentation
  updateContents
  IntegerValueFits
  Ty.integerValueFits?
  Ty.integerBounds?
  RuntimeValue.size
  RuntimeValue.variant?
  SemanticOperations.readLocal?
  SemanticOperations.constValue?
  SemanticOperations.parameterLoanLocations_singleBorrow
  SemanticOperations.parameterLoanLocations_twoBorrows
  SemanticOperations.parameterLoanLocations_borrowInteger
  SemanticOperations.parameterLoanLocations_borrowBool
  SemanticOperations.parameterLoanLocations_singleInteger
  SemanticOperations.array_singleton_setIfInBounds_zero
  SemanticOperations.array_pair_set_zero
  SemanticOperations.array_pair_set_one
  SemanticOperations.array_triple_set_zero
  SemanticOperations.array_triple_set_two
  SemanticOperations.array_four_setIfInBounds_zero
  SemanticOperations.array_four_setIfInBounds_one
  SemanticOperations.array_filter_empty
  SemanticOperations.array_singleton_push
  SemanticOperations.array_empty_push
  SemanticOperations.activeLoans_singleton_retire
  SemanticOperations.activeLoans_zero_then_one
  SemanticOperations.activeLoans_zero_then_one_list
  Array.push_eq_push
  SemanticOperations.initialFrame?
  SemanticOperations.finishControl?
  SemanticOperations.finalizeFunctionState
  SemanticOperations.packResults
  SemanticOperations.evaluatePrimitiveOperation?
  SemanticOperations.resolveTargetIntegerType?
  SemanticOperations.checkedInteger
  SemanticOperations.modularInteger
  SemanticOperations.checkedBinaryInteger
  SemanticOperations.modularBinaryInteger
  SemanticOperations.checkedUnaryInteger
  SemanticOperations.modularUnaryInteger
  SemanticOperations.compareOrdered
  Ty.integerBounds?
  Ty.integerValueFits?

attribute [lir_data_norm high]
  SemanticOperations.exportFrameLoans_singleInteger_state
  SemanticOperations.exportFrameLoans_plainInteger_state
  SemanticOperations.exportFrameLoans_clearedSingleton_state
  SemanticOperations.exportFrameLoans_plainBool_state
  SemanticOperations.exportFrameLoans_plainAddress_state
  SemanticOperations.exportFrameLoans_plainTwoIntegers_state
  SemanticOperations.exportFrameLoans_addressNominalInteger_state
  SemanticOperations.exportFrameLoans_globalNominalThirdLocal
  SemanticOperations.exportFrameLoans_focusedGlobalNominal
  SemanticOperations.exportFrameLoans_returnedFocusedGlobalNominal
  SemanticOperations.exportFrameLoans_focusedGlobalNominalSaved
  SemanticOperations.exportFrameLoans_borrowInteger_state
  SemanticOperations.exportFrameLoans_borrowBool_state
  SemanticOperations.exportFrameLoans_borrowTwoIntegers_state
  SemanticOperations.exportFrameLoans_singleInteger_fromState
  SemanticOperations.exportFrameLoans_borrowInteger_fromState
  SemanticOperations.exportFrameLoans_borrowBool_fromState
  SemanticOperations.exportFrameLoans_twoIntegers_state
  SemanticOperations.localLoanPlace?_singleLocal
  SemanticOperations.localLoanPlace?_twoLocals_left
  SemanticOperations.localLoanPlace?_twoLocals_right
  SemanticOperations.updateBorrowValue?_localZero
  SemanticOperations.updateBorrowValue?_singleLocal
  SemanticOperations.updateBorrowValue?_singleLocal_pair
  SemanticOperations.updateBorrowValue?_returnedProjectedReborrow_pair
  SemanticOperations.updateBorrowValue?_addressReturnedBorrow
  SemanticOperations.updateBorrowValue?_twoLocals_left
  SemanticOperations.updateBorrowValue?_twoLocals_right
  SemanticOperations.updateBorrowValue?_twoReturnedReborrows_left
  SemanticOperations.updateBorrowValue?_twoReturnedReborrows_right
  SemanticOperations.updateBorrowValue?_globalBorrowThirdLocal
  SemanticOperations.updateBorrowValue?_focusedBorrowPair
  SemanticOperations.updateBorrowValue?_focusedBorrowPairSaved
  SemanticOperations.applyPendingFrom_twoDerefLocals
  SemanticOperations.applyPendingFrom_twoReturnedReborrows_derefLocals
  SemanticOperations.applyPendingFrom_returnedProjectedReborrow_derefLocalZero_pair
  SemanticOperations.applyPendingFrom_returnedFocusedGlobalNominal
  SemanticOperations.applyPendingFrom_samePending
  SemanticOperations.registerReturnedLoan_some_singleBorrow

/-! The loan-bookkeeping walkers are well-founded recursions, which whnf
cannot unfold; their equations make the simp phase execute them instead.
The frames and values symbolic execution builds are constructor-shaped, so
these equations always make progress. -/

attribute [lir_data_norm]
  SemanticOperations.rewriteFirst
  SemanticOperations.rewriteFirstList
  SemanticOperations.findFirst
  SemanticOperations.findFirstList
  SemanticOperations.collectPruned
  SemanticOperations.collectPrunedList
  SemanticOperations.fillHole?
  SemanticOperations.fillLocalLoanHole?
  SemanticOperations.fillVisibleHole
  SemanticOperations.applyPendingWriteBack
  SemanticOperations.applyPending
  SemanticOperations.clearBorrowValue
  SemanticOperations.findBorrowValue?
  SemanticOperations.outermostBorrows
  SemanticOperations.resolveReturnedBorrows
  SemanticOperations.borrowEntry?
  SemanticOperations.frameBorrows
  SemanticOperations.settleFrameLoans
  SemanticOperations.holeInFrame_mk
  SemanticOperations.holeWithin
  SemanticOperations.removeGlobalLoan
  SemanticOperations.holeInGlobals
  List.find?
  decodeInt?
  decodeBool?
  decodeString?
  decodeAddress?
  decodeSigner?
  decodeBytes?
  decodeUnit?

/-! A dying loan with no visible hole exports through the pending set.
`fillVisibleHole` guards its scans behind the visibility predicates, so a
contract's hole-freedom precondition decides the branch by rewriting; the
equation is also available directly. -/

attribute [lir_data_norm]
  SemanticOperations.applyWriteBack_export
  SemanticOperations.applyWriteBack_empty_export
  SemanticOperations.applyWriteBack_empty
  SemanticOperations.globalLoanKey?_registry
  SemanticOperations.globalLoanKey?_registered
  SemanticOperations.globalLoanKeyIn?_head
  SemanticOperations.FreshGlobalLoanIds.lookup_next
  SemanticOperations.FreshGlobalLoanIds.lookup_add
  SemanticOperations.priorLoan_beq_next_false
  SemanticOperations.nextLoan_beq_prior_false
  SemanticOperations.priorLoan_ne_next
  SemanticOperations.nextLoan_ne_prior
  SemanticOperations.priorLoan_beq_future_false
  SemanticOperations.futureLoan_beq_prior_false
  SemanticOperations.priorLoan_ne_future
  SemanticOperations.futureLoan_ne_prior
  SemanticOperations.applyPendingWriteBack_globals
  SemanticOperations.applyPendingFrom_derefLocalZero
  SemanticOperations.applyPendingFrom_derefLocalZero_afterBorrow
  SemanticOperations.applyPendingFrom_derefLocalZero_pair
  SemanticOperations.applyPendingFrom_globals
  SemanticOperations.applyPendingFrom_self
  SemanticOperations.applyPending_globals

/-! The walkers drive their searches through core list folds whose stepping
lemmas are not in the default simp set. -/

attribute [lir_data_norm]
  List.findIdx?_nil
  List.findIdx?_cons
  List.findSome?_nil
  List.findSome?_cons

/-! The division evaluators implement truncation through a sign analysis of
`natAbs` magnitudes; unfolding that implementation buries a symbolic operand
under sign tests nothing later consumes.  These equations characterize them
by the `Int.tdiv`/`Int.tmod` the arithmetic finish understands, and the ops
themselves are reduction-sealed so only the equations fire. -/

@[lir_data_norm] theorem truncatingQuotient?_eq (left right : Int) :
    SemanticOperations.truncatingQuotient? left right =
      if right = 0 then none else some (left.tdiv right) := by
  unfold SemanticOperations.truncatingQuotient?
  rcases left with m | m <;> rcases right with n | n <;>
    simp [Int.tdiv, Int.natAbs, beq_iff_eq]
  · by_cases h : n = 0 <;> simp [h]
    omega
  · rintro rfl; simp
  · by_cases h : n = 0 <;> simp [h]

@[lir_data_norm] theorem truncatingRemainder?_eq (left right : Int) :
    SemanticOperations.truncatingRemainder? left right =
      if right = 0 then none else some (left.tmod right) := by
  unfold SemanticOperations.truncatingRemainder?
  simp only [truncatingQuotient?_eq, Int.tmod_def]
  by_cases h : right = 0 <;> simp [h, Option.bind, Int.mul_comm]

/-! The declaration resolvers a call, constant, or constructor step consults
search the quoted unit through `Array.findIdx?` and `Array.any` — well-founded
recursions whnf cannot execute.  The simp phase runs them instead: the
resolver definitions unfold, the array searches bridge to their list forms
over the unit's literal arrays, and the name comparisons evaluate by the
default simp set's ground `BEq` support. -/

attribute [lir_data_norm]
  SemanticOperations.declaredName?
  SemanticOperations.referencedName?
  SemanticOperations.resolveFunction?
  SemanticOperations.resolveConstant?
  SemanticOperations.resolveStruct?
  SemanticOperations.structName?
  SemanticOperations.evaluateDataOperation?_select
  SemanticOperations.evaluateDataOperation?_selectVariants
  SemanticOperations.evaluateDataOperation?_testVariants
  SemanticOperations.evaluateDataOperation?_discriminant
  SemanticOperations.evaluateDataOperation?_updateField
  SemanticOperations.resolveNominal?
  SemanticOperations.handleFields?
  SemanticOperations.handleFieldIndex?
  SemanticOperations.referencedFieldIndex?
  SemanticOperations.variantIndex?
  SemanticOperations.variantDiscriminant?
  List.findIdx?_toArray

/-- Reduction renders an array literal in its constructor form, which the
list bridge for the search does not match.  Without this the declaration
search a field selection performs stalls half executed. -/
@[lir_data_norm] theorem findIdx?_mk {α : Type _} {p : α → Bool} {l : List α} :
    (Array.mk l).findIdx? p = l.findIdx? p := by
  rw [show (Array.mk l) = l.toArray from by simp]
  exact List.findIdx?_toArray p l

/-- Derived equality for a lowered local identifier reduces to equality of
its native numeric index. -/
@[lir_data_norm] theorem localId_beq_mk (left right : Nat) :
    ((⟨left⟩ : LocalId) == ⟨right⟩) = (left == right) := rfl

/-- Substitute an equation hypothesis into the goal.  An operand
obligation reads `evaluator … = value`, naming the value a step
introduced; substituting it immediately keeps the next program point
concrete.  Reduction can wrap both sides in a constructor
(`some result = some value`); injection peels the constructors so the
substitution still fires.  A hypothesis that is not a usable equation is
left in place. -/
private partial def substituteHypothesis (goal : Lean.MVarId)
    (fvarId : Lean.FVarId) (fuel : Nat) : Lean.MetaM (Option Lean.MVarId) := do
  match fuel with
  | 0 => return goal
  | fuel + 1 =>
    try
      let (_, substituted) ← Lean.Meta.substCore goal fvarId (symm := true)
      return substituted
    catch _ =>
    try
      let (_, substituted) ← Lean.Meta.substCore goal fvarId
      return substituted
    catch _ =>
    try
      match ← Lean.Meta.injection goal fvarId with
      | .solved => return none
      | .subgoal subgoal fvarIds _ =>
          let mut goal := subgoal
          for fvarId in fvarIds do
            match ← substituteHypothesis goal fvarId fuel with
            | some next => goal := next
            | none => return none
          return goal
    catch _ => return goal

/-- Destructure every twin-typed hypothesis down to its scalar fields —
certified integers included, so a field is a raw `Int` binder everywhere
and never a projection one spelling of which blocks the arithmetic — and
assert each exposed range certificate as an independent arithmetic
hypothesis, since the certificate proof itself ends up inside the branch
equation where the closing normalization cannot touch it. -/
private partial def destructureTwins (goal : Lean.MVarId) :
    Lean.MetaM Lean.MVarId := do
  let target? ← goal.withContext do
    (← Lean.getLCtx).findDeclM? fun declaration => do
      if declaration.isImplementationDetail then return none
      match (← Lean.instantiateMVars declaration.type).getAppFn with
      | .const name _ =>
          if name == ``SpecInt ||
              Proofs.leanerTwinAttribute.hasTag (← Lean.getEnv) name then
            return some declaration.fvarId
          else return none
      | _ => return none
  match target? with
  | some fvarId =>
      let goal ← goal.withContext do
        let type ← Lean.instantiateMVars
          (← Lean.Meta.inferType (Lean.Expr.fvar fvarId))
        if type.getAppFn.constName? == some ``SpecInt then
          /- Keep the certified decoder equation before eliminating the
          wrapper.  Packed native result codecs may later reconstruct this
          exact scalar from a runtime borrow; after `cases` the range proof
          is otherwise present but the canonical `SpecInt` witness is not. -/
          let fact ← Lean.Meta.mkAppM ``decodeInt?_val #[Lean.Expr.fvar fvarId]
          let asserted ← goal.assert `decodedSpecInt
            (← Lean.Meta.inferType fact) fact
          return (← asserted.intro1P).2
        return goal
      match (← goal.cases fvarId).toList with
      | [subgoal] => destructureTwins subgoal.mvarId
      | _ => return goal
  | none =>
      -- Range facts of the certificates the split exposed, asserted in the
      -- arithmetic form `omega` reads directly.
      let facts ← goal.withContext do
        let mut facts : Array Lean.Expr := #[]
        let mut known : Array Lean.Expr := #[]
        for declaration in ← Lean.getLCtx do
          if declaration.isImplementationDetail then continue
          known := known.push (← Lean.instantiateMVars declaration.type)
        for declaration in ← Lean.getLCtx do
          if declaration.isImplementationDetail then continue
          let type ← Lean.instantiateMVars declaration.type
          if type.isAppOfArity ``IntegerValueFits 3 then
            let width ← Lean.Meta.whnf (type.getArg! 0)
            unless width.isAppOfArity ``IntWidth.bits 1 do continue
            let signed ← Lean.Meta.whnf (type.getArg! 1)
            let bounds := if signed.isConstOf ``Bool.true then
              ``IntegerValueFits.signed_bounds
            else ``IntegerValueFits.unsigned_bounds
            let fact ← Lean.Meta.mkAppM bounds #[declaration.toExpr]
            let factType ← Lean.instantiateMVars (← Lean.Meta.inferType fact)
            unless known.contains factType do
              facts := facts.push fact
              known := known.push factType
        pure facts
      let mut current := goal
      for fact in facts do
        let goal ← current.assert `fits (← Lean.Meta.inferType fact) fact
        let (_, goal) ← goal.intro1P
        current := goal
      return current

/-- Certify every typed value in the context: destructure twin-typed
hypotheses into scalar fields and assert each certificate's bounds in the
arithmetic form `omega` reads. -/
elab "leaner_certify!" : tactic =>
  Lean.Elab.Tactic.liftMetaTactic1 fun goal => destructureTwins goal

/-! ### Naming the facts a contract's precondition leaves

A generated script refers to the precondition's facts by name, so their
names are derived from their shapes — the variable a fact is about and
the kind of fact — never from their positions, which shift with the unit's
storable families and the function's spec clauses. -/

/-- The accessible name of a free variable, if it is one. -/
private def variableName? (e : Lean.Expr) : Lean.MetaM (Option String) := do
  let .fvar id := e | return none
  return some (← id.getUserName).eraseMacroScopes.toString

/-- The name a precondition fact gets from its shape. -/
private def factName? (type : Lean.Expr) : Lean.MetaM (Option String) := do
  let type ← Lean.instantiateMVars type
  if type.isAppOfArity ``SemanticOperations.FreshGlobalLoanIds 1 then
    return some "freshLoans"
  if type.isAppOfArity ``LeanerIR.IntegerValueFits 3 then
    let some v ← variableName? (type.getArg! 2) | return none
    return some s!"{v}_fits"
  if type.isAppOfArity ``Eq 3 then
    let lhs := type.getArg! 1
    let rhs := type.getArg! 2
    if rhs.isAppOf ``Option.none then
      if lhs.isAppOfArity ``SemanticOperations.globalLoanKeyIn? 2 ||
          lhs.isAppOfArity ``SemanticOperations.globalLoanKey? 2 then
        let some v ← variableName? (lhs.getArg! 1) | return none
        return some s!"{v}_keyFree"
    return none
  if type.isAppOfArity ``LT.lt 4 then
    if (type.getArg! 3).isAppOf ``RuntimeState.nextLoan then
      let some v ← variableName? (type.getArg! 2) | return none
      return some s!"{v}_bound"
    return none
  if type.isAppOfArity ``LE.le 4 then
    let lower := type.getArg! 2
    let upper := type.getArg! 3
    if lower.nat?.isSome || lower.int?.isSome || lower.isAppOf ``OfNat.ofNat then
      let some v ← variableName? upper | return none
      return some s!"{v}_nonNeg"
    if upper.nat?.isSome || upper.int?.isSome || upper.isAppOf ``OfNat.ofNat then
      let some v ← variableName? lower | return none
      return some s!"{v}_max"
    return none
  if type.isAppOfArity ``And 2 then
    let left := type.getArg! 0
    if left.isAppOfArity ``LE.le 4 then
      let some v ← variableName? (left.getArg! 3) | return none
      return some s!"{v}_range"
    return none
  if type.isForall then
    /- `∀ key, lookup … = Option.map erase (contents key)`: the family's
    representation, named after its contents binder. -/
    let body := type.bindingBody!
    if body.isAppOfArity ``Eq 3 then
      let rhs := body.getArg! 2
      if rhs.isAppOfArity ``Option.map 4 then
        let read := rhs.getArg! 3
        if read.isApp then
          if let .fvar id := read.appFn! then
            let contents := (← id.getUserName).eraseMacroScopes.toString
            let family := if contents.endsWith "_contents" then
              contents.dropEnd "_contents".length |>.toString else contents
            return some s!"{family}_represented"
    return none
  return none

/-- Name every proposition in context that a shape claims, and every
inaccessible one — those no shape claims are the spec's own clauses,
numbered in order.  A hypothesis the harness named and no shape claims is
left alone. -/
elab "leaner_name_facts" : tactic => do
  Lean.Elab.Tactic.liftMetaTactic1 fun goal => do
    let mut goal := goal
    let mut clause := 0
    let mut taken : Array String := #[]
    let declarations ← goal.withContext do
      pure ((← Lean.getLCtx).decls.toList.filterMap id).toArray
    let userNames := declarations.map (·.userName)
    for (declaration, index) in declarations.zipIdx do
      if declaration.isImplementationDetail then continue
      let isFact ← goal.withContext do Lean.Meta.isProp declaration.type
      unless isFact do continue
      let shadowed := (userNames.extract (index + 1) userNames.size).contains
        declaration.userName
      let unnamed := declaration.userName.hasMacroScopes || shadowed
      let named? ← goal.withContext do factName? declaration.type
      let name ← match named? with
        | some name => pure name
        | none =>
            unless unnamed do continue
            let name := s!"requires_{clause}"
            clause := clause + 1
            pure name
      let name := if taken.contains name then s!"{name}_{taken.size}" else name
      taken := taken.push name
      goal ← goal.rename declaration.fvarId (Lean.Name.mkSimple name)
    return goal

/-- Name the local reads the drive introduced and has not yet named: each
`row[i]?.join = some v` (or `readLocal? … = some v`) becomes
`rowReadEq<k>` with its value `rowRead<k>`, numbered in context order from
zero.  Shape-directed, so the drive introducing further binders after a
read (a bare local as a block's value) cannot displace them. -/
elab "leaner_name_reads" : tactic => do
  Lean.Elab.Tactic.liftMetaTactic1 fun goal => do
    let mut goal := goal
    let mut index := 0
    let declarations ← goal.withContext do
      pure ((← Lean.getLCtx).decls.toList.filterMap id).toArray
    for declaration in declarations do
      if declaration.isImplementationDetail then continue
      unless declaration.userName.hasMacroScopes do continue
      let type ← goal.withContext do Lean.instantiateMVars declaration.type
      unless type.isAppOfArity ``Eq 3 do continue
      let lhs := type.getArg! 1
      let rhs := type.getArg! 2
      unless lhs.isAppOf ``Option.join || lhs.isAppOf ``SemanticOperations.readLocal? do continue
      unless rhs.isAppOfArity ``Option.some 2 do continue
      let .fvar valueId := rhs.getArg! 1 | continue
      goal ← goal.rename valueId (Lean.Name.mkSimple s!"rowRead{index}")
      goal ← goal.rename declaration.fvarId (Lean.Name.mkSimple s!"rowReadEq{index}")
      index := index + 1
    return goal

/-- Bind a route's own vocabulary to a hypothesis by name. -/
elab "leaner_rename" old:ident " => " new:ident : tactic => do
  Lean.Elab.Tactic.liftMetaTactic1 fun goal => do
    let declaration ← goal.withContext do
      Lean.Meta.getLocalDeclFromUserName old.getId
    return some (← goal.rename declaration.fvarId new.getId)

/-- Destruct one hypothesis through its existentials and conjunctions,
substituting the equations this exposes and leaving the remaining facts in
context.  This is how a proof enters a generated contract's precondition:
the logical binders become locals, the row equation rewrites the argument
row, and facts like hole-freedom stay available to `leaner_wp!`. -/
elab "leaner_cases" hypothesis:ident : tactic => do
  /- Substitutions remap the free variables of sibling hypotheses, so the
  worklist tracks hypotheses by fresh user names, which survive. -/
  /- A disjunction — a `Bool` argument's contract enumerates its values —
  opens into one goal per alternative, each destructured on its own; the
  generated proof runs its tail on every goal that results. -/
  let rec destruct (goal : Lean.MVarId) (userName : Lean.Name)
      (fuel : Nat) : Lean.MetaM (List Lean.MVarId) := do
    match fuel with
    | 0 => return [goal]
    | fuel + 1 =>
      let fvarId? ← goal.withContext do
        try pure (some (← Lean.Meta.getLocalDeclFromUserName userName).fvarId)
        catch _ => pure none
      let some fvarId := fvarId? | return [goal]
      /- Expose the head only when that uncovers structure: a contract
      field applied to its arguments unfolds to the quantified proposition,
      but a leaf fact keeps its authored form — reducing `0 ≤ x` to its
      `Int.NonNeg` unfolding would only obscure it. -/
      let isStructural (type : Lean.Expr) : Bool :=
        type.getAppFn.constName? == some ``Exists ||
          type.getAppFn.constName? == some ``And ||
          type.getAppFn.constName? == some ``Or
      let goal ← goal.withContext do
        let type ← Lean.instantiateMVars (← fvarId.getType)
        if isStructural type then pure goal else
        let reduced ← Lean.Meta.whnf type
        if reduced == type || !isStructural reduced then pure goal
        else goal.changeLocalDecl fvarId reduced
      let fvarId? ← goal.withContext do
        try pure (some (← Lean.Meta.getLocalDeclFromUserName userName).fvarId)
        catch _ => pure none
      let some fvarId := fvarId? | return [goal]
      let structural ← goal.withContext do
        let type ← Lean.instantiateMVars (← fvarId.getType)
        pure (isStructural type)
      let disjunction ← goal.withContext do
        let type ← Lean.instantiateMVars (← fvarId.getType)
        pure (type.getAppFn.constName? == some ``Or)
      if disjunction then
        let subgoals ← goal.cases fvarId
        let mut goals := []
        for subgoal in subgoals do
          let mut current := subgoal.mvarId
          let mut facts : Array Lean.Name := #[]
          for field in subgoal.fields do
            if let .fvar fieldId := field then
              let fresh ← Lean.mkFreshUserName `fact
              current ← current.rename fieldId fresh
              facts := facts.push fresh
          let mut branch := [current]
          for factName in facts do
            let mut next := []
            for g in branch do
              next := next ++ (← destruct g factName fuel)
            branch := next
          goals := goals ++ branch
        return goals
      else if structural then
        /- An existential's witness takes the contract's binder name, made
        accessible: generated scripts refer to it.  (`cases` would name it
        after `Exists.intro`'s field.) -/
        let binder? ← goal.withContext do
          let type ← Lean.instantiateMVars (← fvarId.getType)
          if type.isAppOfArity ``Exists 2 then
            let predicate := type.getArg! 1
            if predicate.isLambda then
              pure (some predicate.bindingName!.eraseMacroScopes)
            else pure none
          else pure none
        let subgoals ← goal.cases fvarId
        match subgoals with
        | #[subgoal] =>
            /- Only proposition fields are destructured further; a witness
            binder is data. -/
            let (goal, facts) ← do
              let mut goal := subgoal.mvarId
              let mut facts : Array Lean.Name := #[]
              for field in subgoal.fields do
                if let .fvar fieldId := field then
                  let isFact ← goal.withContext do
                    Lean.Meta.isProp (← fieldId.getType)
                  if isFact then
                    let fresh ← Lean.mkFreshUserName `fact
                    goal ← goal.rename fieldId fresh
                    facts := facts.push fresh
                  else
                    /- A family's contents binder is named after its twin,
                    from its type (`StorageKey → Option T`): the binder the
                    contract gave it does not survive normalization. -/
                    let contents? ← goal.withContext do
                      let type ← Lean.instantiateMVars (← fieldId.getType)
                      if type.isForall then
                        let body := type.bindingBody!
                        if body.isAppOfArity ``Option 1 then
                          if let .const twin _ := (body.getArg! 0).getAppFn then
                            pure (some s!"{twin.getString!}_contents")
                          else pure none
                        else pure none
                      else pure none
                    if let some name := contents? then
                      goal ← goal.rename fieldId (Lean.Name.mkSimple name)
                    else if let some binder := binder? then
                      goal ← goal.rename fieldId (Lean.Name.mkSimple binder.toString)
              pure (goal, facts)
            let mut goals := [goal]
            for factName in facts do
              let mut next := []
              for g in goals do
                next := next ++ (← destruct g factName fuel)
              goals := next
            return goals
        | _ => return [goal]
      else
        match ← substituteHypothesis goal fvarId fuel with
        | some next => return [next]
        | none => return []
  Lean.Elab.Tactic.liftMetaTactic fun goal =>
    destruct goal hypothesis.getId 64

/-- Destructure every conjunctive or existential hypothesis, substituting
the equations this exposes.  The closing attempt of a generated proof runs
it so a result or state equation buried in a stepping hypothesis reaches
the goal before the simp and arithmetic finish. -/
private partial def flattenHypotheses (goal : Lean.MVarId) (fuel : Nat) :
    Lean.MetaM (Option Lean.MVarId) := do
  match fuel with
  | 0 => return goal
  | fuel + 1 =>
    let structural? ← goal.withContext do
      (← Lean.getLCtx).findDeclM? fun declaration => do
        if declaration.isImplementationDetail then return none
        let type ← Lean.instantiateMVars declaration.type
        if type.getAppFn.constName? == some ``Exists ||
            type.getAppFn.constName? == some ``And then
          return some declaration.fvarId
        else return none
    match structural? with
    | none => return goal
    | some fvarId =>
        match ← goal.cases fvarId with
        | #[subgoal] =>
            let mut current := subgoal.mvarId
            for field in subgoal.fields do
              if let .fvar fieldId := field then
                match ← substituteHypothesis current fieldId 8 with
                | some next => current := next
                | none => return none
            flattenHypotheses current fuel
        | _ => return some goal

end LeanerIR.Proofs
