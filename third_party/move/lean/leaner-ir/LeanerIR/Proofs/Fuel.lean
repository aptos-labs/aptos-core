-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Interpreter.Interpreter

/-!
# Interpreter fuel monotonicity

A successful fuelled evaluation is stable under additional fuel. This is the
bridge between the fuel-free big-step semantics and the interpreter: combined
with soundness it yields completeness up to fuel, and — because the
interpreter is a function — determinism of the big-step relation.
-/

namespace LeanerIR.Proofs.Fuel

open LeanerIR.Interpreter
open LeanerIR.Validation
open SemanticOperations

private structure MonoAt (fuel : Nat) : Prop where
  function : ∀ executable handle state arguments result,
    Internal.evalFunction fuel executable handle state arguments = .ok result →
      Internal.evalFunction (fuel + 1) executable handle state arguments = .ok result
  expression : ∀ executable namespaceId frame state exprId result,
    Internal.evalExpr fuel executable namespaceId frame state exprId = .ok result →
      Internal.evalExpr (fuel + 1) executable namespaceId frame state exprId = .ok result
  values : ∀ executable namespaceId frame state expressions result,
    Internal.evalValues fuel executable namespaceId frame state expressions = .ok result →
      Internal.evalValues (fuel + 1) executable namespaceId frame state expressions = .ok result
  statements : ∀ executable namespaceId frame state statements result,
    Internal.evalStatements fuel executable namespaceId frame state statements = .ok result →
      Internal.evalStatements (fuel + 1) executable namespaceId frame state statements
        = .ok result
  arms : ∀ executable namespaceId ns ownerLoc frame state value arms result,
    Internal.evalArms fuel executable namespaceId ns ownerLoc frame state value arms
        = .ok result →
      Internal.evalArms (fuel + 1) executable namespaceId ns ownerLoc frame state value arms
        = .ok result

private theorem monoAt : ∀ fuel, MonoAt fuel
  | 0 => {
      function := by simp [Internal.evalFunction, failAt]
      expression := by simp [Internal.evalExpr, failAt]
      values := by
        intro executable namespaceId frame state expressions result h
        cases expressions with
        | nil => exact h
        | cons expression expressions => simp [Internal.evalValues, failAt] at h
      statements := by
        intro executable namespaceId frame state statements result h
        cases statements with
        | nil => exact h
        | cons statement statements => simp [Internal.evalStatements, failAt] at h
      arms := by
        intro executable namespaceId ns ownerLoc frame state value arms result h
        cases arms with
        | nil => exact h
        | cons arm arms => simp [Internal.evalArms, failAt] at h }
  | fuel + 1 =>
      let previous := monoAt fuel
      {
        function := by
          intro executable handle state arguments result h
          simp only [Internal.evalFunction, bind, Except.bind] at h
          rw [Internal.evalFunction.eq_2]
          simp only [bind, Except.bind]
          cases ns_eq : executable.unit.namespaces[handle.namespaceId.index]? with
          | none => simp [ns_eq, failAt] at h
          | some ns =>
              simp only [ns_eq] at h ⊢
              cases declaration_eq : ns.functions[handle.functionId.index]? with
              | none => simp [declaration_eq, failAt] at h
              | some declaration =>
                  simp only [declaration_eq] at h ⊢
                  by_cases arity_ne : arguments.size != declaration.signature.parameters.size
                  · simp [arity_ne, failAt] at h
                  · simp only [arity_ne] at h ⊢
                    cases frame_eq : initialFrame? declaration arguments with
                    | none => simp [frame_eq, failAt] at h
                    | some initialFrame =>
                        simp only [frame_eq] at h ⊢
                        cases body_eq : declaration.body with
                        | absent => simp [body_eq, failAt] at h
                        | structured root =>
                            simp only [body_eq] at h ⊢
                            cases evaluation_eq : Internal.evalExpr fuel executable
                                handle.namespaceId initialFrame state root with
                            | error error => simp [evaluation_eq] at h
                            | ok evaluation =>
                                rw [previous.expression _ _ _ _ _ _ evaluation_eq]
                                simp only [evaluation_eq] at h
                                exact h
        expression := by
          intro executable namespaceId frame state exprId result h
          simp only [Internal.evalExpr, bind, Except.bind] at h
          rw [Internal.evalExpr.eq_2]
          simp only [bind, Except.bind]
          cases ns_eq : executable.unit.namespaces[namespaceId.index]? with
          | none => simp [ns_eq, failAt] at h
          | some ns =>
              simp only [ns_eq] at h ⊢
              cases expr_eq : ns.expressions[exprId.index]? with
              | none => simp [expr_eq, failAt] at h
              | some expression =>
                  simp only [expr_eq] at h ⊢
                  cases kind_eq : expression.kind with
                  | value value sourceConstant => simp only [kind_eq] at h ⊢; exact h
                  | localVar localId => simp only [kind_eq] at h ⊢; exact h
                  | continue_ nest => simp only [kind_eq] at h ⊢; exact h
                  | spec block => simp only [kind_eq] at h ⊢; exact h
                  | quantifier kind binders triggers condition body =>
                      simp only [kind_eq] at h ⊢; exact h
                  | constant reference =>
                      simp only [kind_eq] at h ⊢
                      cases resolve_eq : resolveConstant? executable.unit namespaceId
                          reference with
                      | none => simp [resolve_eq, failAt] at h
                      | some handle =>
                          simp only [resolve_eq] at h ⊢
                          cases target_eq :
                              executable.unit.namespaces[handle.namespaceId.index]? with
                          | none => simp [target_eq, failAt] at h
                          | some targetNs =>
                              simp only [target_eq] at h ⊢
                              cases declaration_eq :
                                  targetNs.constants[handle.constantId]? with
                              | none => simp [declaration_eq, failAt] at h
                              | some declaration =>
                                  simp only [declaration_eq] at h ⊢
                                  cases value_eq : Internal.evalExpr fuel executable
                                      handle.namespaceId { locals := #[] } state
                                      declaration.value with
                                  | error error => simp [value_eq] at h
                                  | ok valueResult =>
                                      rw [previous.expression _ _ _ _ _ _ value_eq]
                                      simp only [value_eq] at h
                                      exact h
                  | block statements result_ =>
                      simp only [kind_eq] at h ⊢
                      cases st_eq : Internal.evalStatements fuel executable namespaceId
                          frame state statements.toList with
                      | error error => simp [st_eq] at h
                      | ok statementsResult =>
                          rw [previous.statements _ _ _ _ _ _ st_eq]
                          simp only [st_eq] at h
                          cases statementsResult with
                          | control state frame control => exact h
                          | done doneState doneFrame =>
                              cases result_ with
                              | none => exact h
                              | some child =>
                                  dsimp only at h ⊢
                                  cases child_eq : Internal.evalExpr fuel executable
                                      namespaceId doneFrame doneState child with
                                  | error error => simp [child_eq] at h
                                  | ok evaluation =>
                                      rw [previous.expression _ _ _ _ _ _ child_eq]
                                      simp only [child_eq] at h
                                      exact h
                  | letDecl pattern value body =>
                      simp only [kind_eq] at h ⊢
                      cases value with
                      | none =>
                          dsimp only at h ⊢
                          cases body_eq : Internal.evalExpr fuel executable namespaceId
                              frame state body with
                          | error error => simp [body_eq] at h
                          | ok evaluation =>
                              rw [previous.expression _ _ _ _ _ _ body_eq]
                              simp only [body_eq] at h
                              exact h
                      | some initializer =>
                          dsimp only at h ⊢
                          cases init_eq : Internal.evalExpr fuel executable namespaceId
                              frame state initializer with
                          | error error => simp [init_eq] at h
                          | ok initialized =>
                              rw [previous.expression _ _ _ _ _ _ init_eq]
                              simp only [init_eq] at h
                              cases control_eq : initialized.control.value with
                              | value boundValue =>
                                  simp only [control_eq] at h ⊢
                                  cases bind_eq : bindPattern executable.unit ns initialized.frame pattern
                                      boundValue with
                                  | none => simp only [bind_eq] at h ⊢; exact h
                                  | some bound =>
                                      simp only [bind_eq] at h ⊢
                                      cases body_eq : Internal.evalExpr fuel executable
                                          namespaceId bound initialized.state body with
                                      | error error => simp [body_eq] at h
                                      | ok evaluation =>
                                          rw [previous.expression _ _ _ _ _ _ body_eq]
                                          simp only [body_eq] at h
                                          exact h
                              | «throw_» kind arguments => simp only [control_eq] at h ⊢; exact h
                              | «return_» values => simp only [control_eq] at h ⊢; exact h
                              | break_ nest value => simp only [control_eq] at h ⊢; exact h
                              | continue_ nest => simp only [control_eq] at h ⊢; exact h
                  | ifElse condition thenBranch elseBranch =>
                      simp only [kind_eq] at h ⊢
                      cases condition_eq : Internal.evalExpr fuel executable namespaceId
                          frame state condition with
                      | error error => simp [condition_eq] at h
                      | ok conditionResult =>
                          rw [previous.expression _ _ _ _ _ _ condition_eq]
                          simp only [condition_eq] at h
                          cases control_eq : conditionResult.control.value with
                          | value conditionValue =>
                              simp only [control_eq] at h ⊢
                              cases conditionValue
                              case bool b =>
                                  cases b with
                                  | true =>
                                      dsimp only at h ⊢
                                      cases then_eq : Internal.evalExpr fuel executable
                                          namespaceId conditionResult.frame
                                          conditionResult.state thenBranch with
                                      | error error => simp [then_eq] at h
                                      | ok thenResult =>
                                          rw [previous.expression _ _ _ _ _ _ then_eq]
                                          simp only [then_eq] at h
                                          exact h
                                  | false =>
                                      dsimp only at h ⊢
                                      cases elseBranch with
                                      | none => exact h
                                      | some branch =>
                                          dsimp only at h ⊢
                                          cases else_eq : Internal.evalExpr fuel executable
                                              namespaceId conditionResult.frame
                                              conditionResult.state branch with
                                          | error error => simp [else_eq] at h
                                          | ok elseResult =>
                                              rw [previous.expression _ _ _ _ _ _ else_eq]
                                              simp only [else_eq] at h
                                              exact h
                              all_goals exact h
                          | «throw_» kind arguments => simp only [control_eq] at h ⊢; exact h
                          | «return_» values => simp only [control_eq] at h ⊢; exact h
                          | break_ nest value => simp only [control_eq] at h ⊢; exact h
                          | continue_ nest => simp only [control_eq] at h ⊢; exact h
                  | match_ scrutinee arms =>
                      simp only [kind_eq] at h ⊢
                      cases scrutinee_eq : Internal.evalExpr fuel executable namespaceId
                          frame state scrutinee with
                      | error error => simp [scrutinee_eq] at h
                      | ok scrutineeResult =>
                          rw [previous.expression _ _ _ _ _ _ scrutinee_eq]
                          simp only [scrutinee_eq] at h
                          cases control_eq : scrutineeResult.control.value with
                          | value scrutineeValue =>
                              simp only [control_eq] at h ⊢
                              cases arms_eq : Internal.evalArms fuel executable namespaceId
                                  ns expression.loc scrutineeResult.frame
                                  scrutineeResult.state scrutineeValue arms.toList with
                              | error error => simp [arms_eq] at h
                              | ok armResult =>
                                  rw [previous.arms _ _ _ _ _ _ _ _ _ arms_eq]
                                  simp only [arms_eq] at h
                                  exact h
                          | «throw_» kind arguments => simp only [control_eq] at h ⊢; exact h
                          | «return_» values => simp only [control_eq] at h ⊢; exact h
                          | break_ nest value => simp only [control_eq] at h ⊢; exact h
                          | continue_ nest => simp only [control_eq] at h ⊢; exact h
                  | loop label body =>
                      simp only [kind_eq] at h ⊢
                      cases body_eq : Internal.evalExpr fuel executable namespaceId frame
                          state body with
                      | error error => simp [body_eq] at h
                      | ok bodyResult =>
                          rw [previous.expression _ _ _ _ _ _ body_eq]
                          simp only [body_eq] at h
                          cases control_eq : bodyResult.control.value with
                          | value value =>
                              simp only [control_eq] at h ⊢
                              cases again_eq : Internal.evalExpr fuel executable namespaceId
                                  bodyResult.frame bodyResult.state exprId with
                              | error error => simp [again_eq] at h
                              | ok again =>
                                  rw [previous.expression _ _ _ _ _ _ again_eq]
                                  simp only [again_eq] at h
                                  exact h
                          | continue_ nest =>
                              cases nest with
                              | zero =>
                                  simp only [control_eq] at h ⊢
                                  cases again_eq : Internal.evalExpr fuel executable
                                      namespaceId bodyResult.frame bodyResult.state
                                      exprId with
                                  | error error => simp [again_eq] at h
                                  | ok again =>
                                      rw [previous.expression _ _ _ _ _ _ again_eq]
                                      simp only [again_eq] at h
                                      exact h
                              | succ nest => simp only [control_eq] at h ⊢; exact h
                          | break_ nest value =>
                              simp only [control_eq] at h ⊢
                              cases nest with
                              | zero => exact h
                              | succ nest => exact h
                          | «throw_» kind arguments => simp only [control_eq] at h ⊢; exact h
                          | «return_» values => simp only [control_eq] at h ⊢; exact h
                  | break_ nest value =>
                      simp only [kind_eq] at h ⊢
                      cases value with
                      | none => exact h
                      | some child =>
                          dsimp only at h ⊢
                          cases child_eq : Internal.evalExpr fuel executable namespaceId
                              frame state child with
                          | error error => simp [child_eq] at h
                          | ok childResult =>
                              rw [previous.expression _ _ _ _ _ _ child_eq]
                              simp only [child_eq] at h
                              exact h
                  | «return_» values =>
                      simp only [kind_eq] at h ⊢
                      cases values_eq : Internal.evalValues fuel executable namespaceId
                          frame state values.toList with
                      | error error => simp [values_eq] at h
                      | ok operands =>
                          rw [previous.values _ _ _ _ _ _ values_eq]
                          simp only [values_eq] at h
                          exact h
                  | «throw_» kind arguments =>
                      simp only [kind_eq] at h ⊢
                      cases values_eq : Internal.evalValues fuel executable namespaceId
                          frame state arguments.toList with
                      | error error => simp [values_eq] at h
                      | ok operands =>
                          rw [previous.values _ _ _ _ _ _ values_eq]
                          simp only [values_eq] at h
                          exact h
                  | assign place value =>
                      simp only [kind_eq] at h ⊢
                      cases value_eq : Internal.evalExpr fuel executable namespaceId frame
                          state value with
                      | error error => simp [value_eq] at h
                      | ok valueResult =>
                          rw [previous.expression _ _ _ _ _ _ value_eq]
                          simp only [value_eq] at h
                          exact h
                  | assignPattern pattern value =>
                      simp only [kind_eq] at h ⊢
                      cases value_eq : Internal.evalExpr fuel executable namespaceId frame
                          state value with
                      | error error => simp [value_eq] at h
                      | ok valueResult =>
                          rw [previous.expression _ _ _ _ _ _ value_eq]
                          simp only [value_eq] at h
                          exact h
                  | operation operation instantiations arguments surface =>
                      simp only [kind_eq] at h ⊢
                      cases operation with
                      | call kind =>
                          cases kind with
                          | function callee =>
                              dsimp only at h ⊢
                              cases operands_eq : Internal.evalValues fuel executable
                                  namespaceId frame state arguments.toList with
                              | error error => simp [operands_eq] at h
                              | ok operands =>
                                  rw [previous.values _ _ _ _ _ _ operands_eq]
                                  simp only [operands_eq] at h
                                  cases operands with
                                  | control state frame control => exact h
                                  | values valuesState valuesFrame values =>
                                      dsimp only at h ⊢
                                      cases resolve_eq : resolveFunction? executable.unit
                                          namespaceId callee with
                                      | none => simp only [resolve_eq] at h ⊢; exact h
                                      | some handle =>
                                          simp only [resolve_eq] at h ⊢
                                          cases fn_eq : Internal.evalFunction fuel
                                              executable handle valuesState
                                              values.toArray with
                                          | error error => simp [fn_eq] at h
                                          | ok functionResult =>
                                              rw [previous.function _ _ _ _ _ fn_eq]
                                              simp only [fn_eq] at h
                                              exact h
                          | invoke =>
                              dsimp only at h ⊢
                              cases operands_eq : Internal.evalValues fuel executable
                                  namespaceId frame state arguments.toList with
                              | error error => simp [operands_eq] at h
                              | ok operands =>
                                  rw [previous.values _ _ _ _ _ _ operands_eq]
                                  simp only [operands_eq] at h
                                  cases operands with
                                  | control state frame control => exact h
                                  | values valuesState valuesFrame values =>
                                      dsimp only at h ⊢
                                      cases values with
                                      | nil => exact h
                                      | cons callable callArguments =>
                                          dsimp only at h ⊢
                                          cases callable
                                          case closure handle captures =>
                                              dsimp only at h ⊢
                                              cases fn_eq : Internal.evalFunction fuel
                                                  executable handle valuesState
                                                  (captures ++ callArguments.toArray) with
                                              | error error => simp [fn_eq] at h
                                              | ok functionResult =>
                                                  rw [previous.function _ _ _ _ _ fn_eq]
                                                  simp only [fn_eq] at h
                                                  exact h
                                          all_goals exact h
                          | constructor reference variant =>
                              dsimp only at h ⊢
                              cases operands_eq : Internal.evalValues fuel executable
                                  namespaceId frame state arguments.toList with
                              | error error => simp [operands_eq] at h
                              | ok operands =>
                                  rw [previous.values _ _ _ _ _ _ operands_eq]
                                  simp only [operands_eq] at h
                                  exact h
                          | destructor reference variant =>
                              dsimp only at h ⊢
                              cases operands_eq : Internal.evalValues fuel executable
                                  namespaceId frame state arguments.toList with
                              | error error => simp [operands_eq] at h
                              | ok operands =>
                                  rw [previous.values _ _ _ _ _ _ operands_eq]
                                  simp only [operands_eq] at h
                                  exact h
                          | closure reference =>
                              dsimp only at h ⊢
                              cases operands_eq : Internal.evalValues fuel executable
                                  namespaceId frame state arguments.toList with
                              | error error => simp [operands_eq] at h
                              | ok operands =>
                                  rw [previous.values _ _ _ _ _ _ operands_eq]
                                  simp only [operands_eq] at h
                                  exact h
                          | extension value targets =>
                              dsimp only at h ⊢
                              cases operands_eq : Internal.evalValues fuel executable
                                  namespaceId frame state arguments.toList with
                              | error error => simp [operands_eq] at h
                              | ok operands =>
                                  rw [previous.values _ _ _ _ _ _ operands_eq]
                                  simp only [operands_eq] at h
                                  exact h
                      | move place =>
                          dsimp only at h ⊢
                          cases operands_eq : Internal.evalValues fuel executable
                              namespaceId frame state arguments.toList with
                          | error error => simp [operands_eq] at h
                          | ok operands =>
                              rw [previous.values _ _ _ _ _ _ operands_eq]
                              simp only [operands_eq] at h
                              exact h
                      | copy place =>
                          dsimp only at h ⊢
                          cases operands_eq : Internal.evalValues fuel executable
                              namespaceId frame state arguments.toList with
                          | error error => simp [operands_eq] at h
                          | ok operands =>
                              rw [previous.values _ _ _ _ _ _ operands_eq]
                              simp only [operands_eq] at h
                              exact h
                      | borrow kind place =>
                          dsimp only at h ⊢
                          cases operands_eq : Internal.evalValues fuel executable
                              namespaceId frame state arguments.toList with
                          | error error => simp [operands_eq] at h
                          | ok operands =>
                              rw [previous.values _ _ _ _ _ _ operands_eq]
                              simp only [operands_eq] at h
                              exact h
                      | read place =>
                          dsimp only at h ⊢
                          cases operands_eq : Internal.evalValues fuel executable
                              namespaceId frame state arguments.toList with
                          | error error => simp [operands_eq] at h
                          | ok operands =>
                              rw [previous.values _ _ _ _ _ _ operands_eq]
                              simp only [operands_eq] at h
                              exact h
                      | write place =>
                          dsimp only at h ⊢
                          cases operands_eq : Internal.evalValues fuel executable
                              namespaceId frame state arguments.toList with
                          | error error => simp [operands_eq] at h
                          | ok operands =>
                              rw [previous.values _ _ _ _ _ _ operands_eq]
                              simp only [operands_eq] at h
                              exact h
                      | global kind =>
                          dsimp only at h ⊢
                          cases operands_eq : Internal.evalValues fuel executable
                              namespaceId frame state arguments.toList with
                          | error error => simp [operands_eq] at h
                          | ok operands =>
                              rw [previous.values _ _ _ _ _ _ operands_eq]
                              simp only [operands_eq] at h
                              exact h
                      | primitive kind =>
                          dsimp only at h ⊢
                          cases operands_eq : Internal.evalValues fuel executable
                              namespaceId frame state arguments.toList with
                          | error error => simp [operands_eq] at h
                          | ok operands =>
                              rw [previous.values _ _ _ _ _ _ operands_eq]
                              simp only [operands_eq] at h
                              exact h
                      | reference kind =>
                          dsimp only at h ⊢
                          cases operands_eq : Internal.evalValues fuel executable
                              namespaceId frame state arguments.toList with
                          | error error => simp [operands_eq] at h
                          | ok operands =>
                              rw [previous.values _ _ _ _ _ _ operands_eq]
                              simp only [operands_eq] at h
                              exact h
                      | data kind =>
                          dsimp only at h ⊢
                          cases operands_eq : Internal.evalValues fuel executable
                              namespaceId frame state arguments.toList with
                          | error error => simp [operands_eq] at h
                          | ok operands =>
                              rw [previous.values _ _ _ _ _ _ operands_eq]
                              simp only [operands_eq] at h
                              exact h
                      | specification kind =>
                          dsimp only at h ⊢
                          cases operands_eq : Internal.evalValues fuel executable
                              namespaceId frame state arguments.toList with
                          | error error => simp [operands_eq] at h
                          | ok operands =>
                              rw [previous.values _ _ _ _ _ _ operands_eq]
                              simp only [operands_eq] at h
                              exact h
                      | assert =>
                          dsimp only at h ⊢
                          cases operands_eq : Internal.evalValues fuel executable
                              namespaceId frame state arguments.toList with
                          | error error => simp [operands_eq] at h
                          | ok operands =>
                              rw [previous.values _ _ _ _ _ _ operands_eq]
                              simp only [operands_eq] at h
                              exact h
                      | drop place =>
                          dsimp only at h ⊢
                          cases operands_eq : Internal.evalValues fuel executable
                              namespaceId frame state arguments.toList with
                          | error error => simp [operands_eq] at h
                          | ok operands =>
                              rw [previous.values _ _ _ _ _ _ operands_eq]
                              simp only [operands_eq] at h
                              exact h
                      | profile value targets =>
                          dsimp only at h ⊢
                          cases operands_eq : Internal.evalValues fuel executable
                              namespaceId frame state arguments.toList with
                          | error error => simp [operands_eq] at h
                          | ok operands =>
                              rw [previous.values _ _ _ _ _ _ operands_eq]
                              simp only [operands_eq] at h
                              exact h
        values := by
          intro executable namespaceId frame state expressions result h
          cases expressions with
          | nil => exact h
          | cons expression expressions =>
              simp only [Internal.evalValues, bind, Except.bind] at h ⊢
              cases head_eq : Internal.evalExpr fuel executable namespaceId frame state
                  expression with
              | error error => simp [head_eq] at h
              | ok head =>
                  rw [previous.expression _ _ _ _ _ _ head_eq]
                  simp only [head_eq] at h
                  cases control_eq : head.control.value with
                  | value value =>
                      simp only [control_eq] at h ⊢
                      cases tail_eq : Internal.evalValues fuel executable namespaceId
                          head.frame head.state expressions with
                      | error error => simp [tail_eq] at h
                      | ok tail =>
                          rw [previous.values _ _ _ _ _ _ tail_eq]
                          simpa [tail_eq] using h
                  | «throw_» kind arguments => simpa [control_eq] using h
                  | «return_» values => simpa [control_eq] using h
                  | break_ nest value => simpa [control_eq] using h
                  | continue_ nest => simpa [control_eq] using h
        statements := by
          intro executable namespaceId frame state statements result h
          cases statements with
          | nil => exact h
          | cons statement statements =>
              simp only [Internal.evalStatements, bind, Except.bind] at h ⊢
              cases head_eq : Internal.evalExpr fuel executable namespaceId frame state
                  statement with
              | error error => simp [head_eq] at h
              | ok head =>
                  rw [previous.expression _ _ _ _ _ _ head_eq]
                  simp only [head_eq] at h
                  cases control_eq : head.control.value with
                  | value value =>
                      simp only [control_eq] at h ⊢
                      exact previous.statements _ _ _ _ _ _ h
                  | «throw_» kind arguments => simpa [control_eq] using h
                  | «return_» values => simpa [control_eq] using h
                  | break_ nest value => simpa [control_eq] using h
                  | continue_ nest => simpa [control_eq] using h
        arms := by
          intro executable namespaceId ns ownerLoc frame state value arms result h
          cases arms with
          | nil => exact h
          | cons arm arms =>
              simp only [Internal.evalArms, bind, Except.bind] at h ⊢
              cases bind_eq : bindPattern executable.unit ns frame arm.pattern value with
              | none =>
                  simp only [bind_eq] at h ⊢
                  exact previous.arms _ _ _ _ _ _ _ _ _ h
              | some armFrame =>
                  simp only [bind_eq] at h ⊢
                  cases guard_eq : arm.guard with
                  | none =>
                      simp only [guard_eq] at h ⊢
                      exact previous.expression _ _ _ _ _ _ h
                  | some guard =>
                      simp only [guard_eq] at h ⊢
                      cases guardResult_eq : Internal.evalExpr fuel executable namespaceId
                          armFrame state guard with
                      | error error => simp [guardResult_eq] at h
                      | ok guardResult =>
                          rw [previous.expression _ _ _ _ _ _ guardResult_eq]
                          simp only [guardResult_eq] at h
                          cases control_eq : guardResult.control.value with
                          | value guardValue =>
                              cases guardValue <;> simp only [control_eq] at h ⊢
                              case bool b =>
                                cases b with
                                | true => exact previous.expression _ _ _ _ _ _ h
                                | false => exact previous.arms _ _ _ _ _ _ _ _ _ h
                              all_goals simpa using h
                          | «throw_» kind arguments => simpa [control_eq] using h
                          | «return_» values => simpa [control_eq] using h
                          | break_ nest value => simpa [control_eq] using h
                          | continue_ nest => simpa [control_eq] using h
      }

/-- Success of the fuelled evaluator is stable under any additional fuel. -/
theorem evalFunction_mono {fuel fuel' : Nat} (le : fuel ≤ fuel')
    {executable handle state arguments result}
    (h : Internal.evalFunction fuel executable handle state arguments = .ok result) :
    Internal.evalFunction fuel' executable handle state arguments = .ok result := by
  induction le with
  | refl => exact h
  | step _ ih => exact (monoAt _).function _ _ _ _ _ ih

/-- Success of the fuelled expression evaluator is stable under any
additional fuel. -/
theorem evalExpr_mono {fuel fuel' : Nat} (le : fuel ≤ fuel')
    {executable namespaceId frame state exprId result}
    (h : Internal.evalExpr fuel executable namespaceId frame state exprId = .ok result) :
    Internal.evalExpr fuel' executable namespaceId frame state exprId = .ok result := by
  induction le with
  | refl => exact h
  | step _ ih => exact (monoAt _).expression _ _ _ _ _ _ ih

/-- Success of the fuelled operand-list evaluator is stable under any
additional fuel. -/
theorem evalValues_mono {fuel fuel' : Nat} (le : fuel ≤ fuel')
    {executable namespaceId frame state expressions result}
    (h : Internal.evalValues fuel executable namespaceId frame state expressions
      = .ok result) :
    Internal.evalValues fuel' executable namespaceId frame state expressions = .ok result := by
  induction le with
  | refl => exact h
  | step _ ih => exact (monoAt _).values _ _ _ _ _ _ ih

/-- Success of the fuelled statement-list evaluator is stable under any
additional fuel. -/
theorem evalStatements_mono {fuel fuel' : Nat} (le : fuel ≤ fuel')
    {executable namespaceId frame state statements result}
    (h : Internal.evalStatements fuel executable namespaceId frame state statements
      = .ok result) :
    Internal.evalStatements fuel' executable namespaceId frame state statements
      = .ok result := by
  induction le with
  | refl => exact h
  | step _ ih => exact (monoAt _).statements _ _ _ _ _ _ ih

/-- Success of the fuelled match-arm evaluator is stable under any additional
fuel. -/
theorem evalArms_mono {fuel fuel' : Nat} (le : fuel ≤ fuel')
    {executable namespaceId ns ownerLoc frame state value arms result}
    (h : Internal.evalArms fuel executable namespaceId ns ownerLoc frame state value arms
      = .ok result) :
    Internal.evalArms fuel' executable namespaceId ns ownerLoc frame state value arms
      = .ok result := by
  induction le with
  | refl => exact h
  | step _ ih => exact (monoAt _).arms _ _ _ _ _ _ _ _ _ ih

/-- Success of `Interpreter.run` is stable under any additional fuel. -/
theorem run_mono {fuel fuel' : Nat} (le : fuel ≤ fuel')
    {executable function arguments state result}
    (h : Interpreter.run executable fuel function arguments state = .ok result) :
    Interpreter.run executable fuel' function arguments state = .ok result := by
  simp only [Interpreter.run, bind, Except.bind] at h ⊢
  cases eval_eq : Internal.evalFunction fuel executable function state arguments with
  | error error => simp [eval_eq] at h
  | ok evaluation =>
      rw [evalFunction_mono le eval_eq]
      simp only [eval_eq] at h
      exact h

end LeanerIR.Proofs.Fuel
