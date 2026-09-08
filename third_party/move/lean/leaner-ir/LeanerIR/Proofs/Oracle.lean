-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.BigStep

/-!
# The closed semantics as the least fixed point of the open rules

`EvalExprWith unit callee` evaluates a body with every call answered by the
oracle `callee`; `EvalFunction unit` ties the knot.  Two facts make that
knot usable for recursive native denotations:

* the open rules are monotone in the oracle (`EvalExprWith.mono`), and
* the closed semantics is below every oracle closed under one unfolding
  of the function boundary (`EvalFunction.induction`) — the induction
  principle of the rules themselves, stated once for every oracle.

Both are proved by mirroring every constructor, which is what a change to
the rules must keep in step.
-/

namespace LeanerIR.BigStep

open Validation

/-- Close one premise of a mirrored constructor: a sub-derivation is the
induction hypothesis in context, an oracle fact is carried across by
`lift`. -/
syntax "leaner_oracle_premise " term : tactic

macro_rules
  | `(tactic| leaner_oracle_premise $lift:term) =>
    `(tactic| first | assumption | exact $lift _ _ _ _ _ ‹_›)

/-- Close one mirror case: the goal is the conclusion of an open-semantics
constructor under a new oracle, and every premise is closed by
`leaner_oracle_premise`. -/
syntax "leaner_oracle_mirror " term : tactic

macro_rules
  | `(tactic| leaner_oracle_mirror $lift:term) =>
    `(tactic|
      first
      | (apply EvalExprWith.value <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.constantValue <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.constantControl <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.localVar <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.callArgumentsControl <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.callReturned <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.callThrew <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.constructorArgumentsControl <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.constructorValue <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.destructorArgumentsControl <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.destructorValue <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.closureArgumentsControl <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.closureValue <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.invokeArgumentsControl <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.invokeReturned <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.invokeThrew <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.profileArgumentsControl <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.profileValue <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.profileThrow <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.primitiveArgumentsControl <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.primitiveValue <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.primitiveThrow <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.globalArgumentsControl <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.globalValue <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.globalThrow <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.assertArgumentsControl <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.assertTrue <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.assertFalse <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.operationArgumentsControl <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.operationValue <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.blockControl <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.blockUnit <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.blockResult <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.letNoValue <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.letValueControl <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.letValue <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.ifControl <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.ifTrue <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.ifFalseUnit <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.ifFalse <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.matchControl <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.matchValue <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.loopRepeatValue <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.loopRepeatContinue <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.loopBreak <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.loopOuterBreak <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.loopOuterContinue <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.loopReturn <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.loopThrow <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.breakNone <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.breakControl <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.breakValue <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.continue_ <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.returnValues <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.returnControl <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.throwValues <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.throwControl <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.assignControl <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.assignValue <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.assignPatternControl <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.assignPatternValue <;> leaner_oracle_premise $lift)
      | (apply EvalExprWith.spec <;> leaner_oracle_premise $lift)
      | (apply EvalValuesWith.nil <;> leaner_oracle_premise $lift)
      | (apply EvalValuesWith.headControl <;> leaner_oracle_premise $lift)
      | (apply EvalValuesWith.tailValues <;> leaner_oracle_premise $lift)
      | (apply EvalValuesWith.tailControl <;> leaner_oracle_premise $lift)
      | (apply EvalStatementsWith.nil <;> leaner_oracle_premise $lift)
      | (apply EvalStatementsWith.headControl <;> leaner_oracle_premise $lift)
      | (apply EvalStatementsWith.cons <;> leaner_oracle_premise $lift)
      | (apply EvalArmsWith.reject <;> leaner_oracle_premise $lift)
      | (apply EvalArmsWith.noGuard <;> leaner_oracle_premise $lift)
      | (apply EvalArmsWith.guardControl <;> leaner_oracle_premise $lift)
      | (apply EvalArmsWith.guardTrue <;> leaner_oracle_premise $lift)
      | (apply EvalArmsWith.guardFalse <;> leaner_oracle_premise $lift))

section Monotone

variable {unit : ExecutableUnit} {callee callee' : CalleeRelation}

/-- Every open derivation survives enlarging the oracle. -/
theorem EvalExprWith.mono
    (le : ∀ handle state arguments final outcome,
      callee handle state arguments final outcome →
        callee' handle state arguments final outcome)
    {namespaceId : NamespaceId} {frame finalFrame : RuntimeFrame}
    {state finalState : RuntimeState} {exprId : ExprId} {control : Control}
    (step : EvalExprWith unit callee namespaceId frame state exprId finalFrame
      finalState control) :
    EvalExprWith unit callee' namespaceId frame state exprId finalFrame
      finalState control := by
  apply EvalExprWith.rec
    (motive_1 := fun namespaceId frame state exprId finalFrame finalState control _ =>
      EvalExprWith unit callee' namespaceId frame state exprId finalFrame finalState
        control)
    (motive_2 := fun namespaceId frame state expressions result _ =>
      EvalValuesWith unit callee' namespaceId frame state expressions result)
    (motive_3 := fun namespaceId frame state statements result _ =>
      EvalStatementsWith unit callee' namespaceId frame state statements result)
    (motive_4 := fun namespaceId ns frame state value arms finalFrame finalState
        control _ =>
      EvalArmsWith unit callee' namespaceId ns frame state value arms finalFrame
        finalState control)
    (t := step)
  all_goals intros
  all_goals leaner_oracle_mirror le

theorem EvalFunctionWith.mono
    (le : ∀ handle state arguments final outcome,
      callee handle state arguments final outcome →
        callee' handle state arguments final outcome)
    {handle : FunctionHandle} {initialState finalState : RuntimeState}
    {arguments : Array RuntimeValue} {outcome : Outcome}
    (step : EvalFunctionWith unit callee handle initialState arguments finalState
      outcome) :
    EvalFunctionWith unit callee' handle initialState arguments finalState outcome := by
  obtain ⟨ns, declaration, frame, root, finalFrame, evaluatedState, control,
    namespace_eq, declaration_eq, frame_eq, body_eq, body_step, outcome_eq,
    finalize_eq⟩ := step
  exact ⟨ns, declaration, frame, root, finalFrame, evaluatedState, control,
    namespace_eq, declaration_eq, frame_eq, body_eq, EvalExprWith.mono le body_step,
    outcome_eq, finalize_eq⟩

end Monotone

section Induction

variable {unit : ExecutableUnit}

/-- Least-fixed-point induction: the closed semantics is contained in every
oracle closed under one unfolding of the function boundary. -/
theorem EvalFunction.induction (motive : CalleeRelation)
    (closed : ∀ handle initialState arguments finalState outcome,
      EvalFunctionWith unit motive handle initialState arguments finalState outcome →
        motive handle initialState arguments finalState outcome)
    {handle : FunctionHandle} {initialState finalState : RuntimeState}
    {arguments : Array RuntimeValue} {outcome : Outcome}
    (step : EvalFunction unit handle initialState arguments finalState outcome) :
    motive handle initialState arguments finalState outcome := by
  apply EvalFunction.rec
    (motive_1 := fun handle initialState arguments finalState outcome _ =>
      motive handle initialState arguments finalState outcome)
    (motive_2 := fun namespaceId frame state exprId finalFrame finalState control _ =>
      EvalExprWith unit motive namespaceId frame state exprId finalFrame finalState
        control)
    (motive_3 := fun namespaceId frame state expressions result _ =>
      EvalValuesWith unit motive namespaceId frame state expressions result)
    (motive_4 := fun namespaceId frame state statements result _ =>
      EvalStatementsWith unit motive namespaceId frame state statements result)
    (motive_5 := fun namespaceId ns frame state value arms finalFrame finalState
        control _ =>
      EvalArmsWith unit motive namespaceId ns frame state value arms finalFrame
        finalState control)
    (t := step)
  case body =>
    intros
    apply closed
    exact ⟨_, _, _, _, _, _, _, ‹_›, ‹_›, ‹_›, ‹_›, ‹_›, ‹_›, ‹_›⟩
  all_goals intros
  all_goals leaner_oracle_mirror (fun _ _ _ _ _ fact => fact)

/-- The closed semantics unfolds to the open boundary over itself. -/
theorem EvalFunction.unfold {handle : FunctionHandle}
    {initialState finalState : RuntimeState} {arguments : Array RuntimeValue}
    {outcome : Outcome}
    (step : EvalFunction unit handle initialState arguments finalState outcome) :
    EvalFunctionWith unit (EvalFunction unit) handle initialState arguments finalState
      outcome :=
  (EvalFunction_iff unit handle initialState arguments finalState outcome).mp step

theorem EvalFunction.fold {handle : FunctionHandle}
    {initialState finalState : RuntimeState} {arguments : Array RuntimeValue}
    {outcome : Outcome}
    (step : EvalFunctionWith unit (EvalFunction unit) handle initialState arguments
      finalState outcome) :
    EvalFunction unit handle initialState arguments finalState outcome :=
  (EvalFunction_iff unit handle initialState arguments finalState outcome).mpr step

end Induction

end LeanerIR.BigStep
