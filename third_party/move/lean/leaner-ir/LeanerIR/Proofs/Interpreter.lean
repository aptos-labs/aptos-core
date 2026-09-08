-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Interpreter.Interpreter

/-!
# Structured interpreter soundness

Successful executable evaluation constructs a derivation in the independent,
fuel-free big-step semantics.  Locations are erased only at this theorem
boundary: they remain attached to the executable result returned to tooling.
-/

namespace LeanerIR.Proofs.Interpreter

open LeanerIR.Interpreter
open LeanerIR.Validation
open SemanticOperations

attribute [local simp] bind Except.bind

private def valuesResult : Internal.ValuesEvaluation → BigStep.ValuesResult
  | .values state frame values => .values state frame values
  | .control state frame control => .control state frame control.value

private def statementsResult : Internal.StatementsEvaluation → BigStep.StatementsResult
  | .done state frame => .done state frame
  | .control state frame control => .control state frame control.value

private structure SoundAt (fuel : Nat) : Prop where
  function : ∀ executable handle state arguments result,
    Internal.evalFunction fuel executable handle state arguments = .ok result →
      BigStep.EvalFunction executable handle state arguments result.state result.outcome.value
  expression : ∀ executable namespaceId frame state exprId result,
    Internal.evalExpr fuel executable namespaceId frame state exprId = .ok result →
      BigStep.EvalExpr executable namespaceId frame state exprId
        result.frame result.state result.control.value
  values : ∀ executable namespaceId frame state expressions result,
    Internal.evalValues fuel executable namespaceId frame state expressions = .ok result →
      BigStep.EvalValues executable namespaceId frame state expressions (valuesResult result)
  statements : ∀ executable namespaceId frame state statements result,
    Internal.evalStatements fuel executable namespaceId frame state statements = .ok result →
      BigStep.EvalStatements executable namespaceId frame state statements
        (statementsResult result)
  arms : ∀ executable namespaceId ns ownerLoc frame state value arms result,
    Internal.evalArms fuel executable namespaceId ns ownerLoc frame state value arms = .ok result →
      BigStep.EvalArms executable namespaceId ns frame state value arms
        result.frame result.state result.control.value

private theorem soundAt : ∀ fuel, SoundAt fuel
  | 0 => {
      function := by simp [Internal.evalFunction, failAt]
      expression := by simp [Internal.evalExpr, failAt]
      values := by
        intro executable namespaceId frame state expressions result h
        cases expressions with
        | nil =>
            change Except.ok (.values state frame []) = .ok result at h
            cases h
            exact .nil _ _ _
        | cons expression expressions => simp [Internal.evalValues, failAt] at h
      statements := by
        intro executable namespaceId frame state statements result h
        cases statements with
        | nil =>
            change Except.ok (.done state frame) = .ok result at h
            cases h
            exact .nil _ _ _
        | cons statement statements => simp [Internal.evalStatements, failAt] at h
      arms := by
        intro executable namespaceId ns ownerLoc frame state value arms result h
        cases arms <;> simp [Internal.evalArms, failAt] at h }
  | fuel + 1 =>
      let previous := soundAt fuel
      {
        function := by
          intro executable handle state arguments result h
          simp only [Internal.evalFunction] at h
          cases namespace_eq : executable.unit.namespaces[handle.namespaceId.index]? with
          | none => simp [namespace_eq, failAt] at h
          | some ns =>
              cases declaration_eq : ns.functions[handle.functionId.index]? with
              | none => simp [namespace_eq, declaration_eq, failAt] at h
              | some declaration =>
                  by_cases arity_ne : arguments.size != declaration.signature.parameters.size
                  · simp [namespace_eq, declaration_eq, arity_ne, failAt] at h
                  · cases frame_eq : initialFrame? declaration arguments with
                    | none =>
                        simp [namespace_eq, declaration_eq, arity_ne, frame_eq, failAt] at h
                    | some initialFrame =>
                        cases body_eq : declaration.body with
                        | absent =>
                            simp [namespace_eq, declaration_eq, arity_ne, frame_eq, body_eq,
                              failAt] at h
                        | structured root =>
                            cases evaluation_eq : Internal.evalExpr fuel executable
                                handle.namespaceId initialFrame state root with
                            | error error =>
                                simp [namespace_eq, declaration_eq, arity_ne, frame_eq, body_eq,
                                  evaluation_eq] at h
                            | ok evaluation =>
                                have body_sound := previous.expression executable handle.namespaceId
                                  initialFrame state root evaluation evaluation_eq
                                cases control_eq : evaluation.control.value with
                                | value value =>
                                    cases unpack_eq : unpackFallthrough
                                        declaration.signature.results.size value with
                                    | none =>
                                        simp [namespace_eq, declaration_eq, arity_ne, frame_eq,
                                          body_eq, evaluation_eq, control_eq, unpack_eq, failAt] at h
                                    | some values =>
                                        simp [namespace_eq, declaration_eq, arity_ne, frame_eq,
                                          body_eq, evaluation_eq, control_eq, unpack_eq] at h
                                        cases h
                                        exact BigStep.EvalFunction.body
                                          (evaluatedState := evaluation.state)
                                          (namespace_eq := namespace_eq)
                                          (declaration_eq := declaration_eq)
                                          (frame_eq := frame_eq) (body_eq := body_eq)
                                          (body_step := by simpa [control_eq] using body_sound)
                                          (outcome_eq := by
                                            simp [finishControl?, unpack_eq])
                                          (finalize_eq := rfl)
                                | return_ values =>
                                    by_cases result_arity_ne :
                                        values.size != declaration.signature.results.size
                                    · simp [namespace_eq, declaration_eq, arity_ne, frame_eq, body_eq,
                                        evaluation_eq, control_eq, result_arity_ne, failAt] at h
                                    · simp [namespace_eq, declaration_eq, arity_ne, frame_eq, body_eq,
                                        evaluation_eq, control_eq, result_arity_ne] at h
                                      have result_arity_eq :
                                          values.size = declaration.signature.results.size := by
                                        simpa using result_arity_ne
                                      cases h
                                      exact BigStep.EvalFunction.body
                                        (evaluatedState := evaluation.state)
                                        (namespace_eq := namespace_eq)
                                        (declaration_eq := declaration_eq)
                                        (frame_eq := frame_eq) (body_eq := body_eq)
                                        (body_step := by simpa [control_eq] using body_sound)
                                        (outcome_eq := by
                                          simp [finishControl?, result_arity_eq])
                                        (finalize_eq := rfl)
                                | throw_ kind thrown =>
                                    simp [namespace_eq, declaration_eq, arity_ne, frame_eq, body_eq,
                                      evaluation_eq, control_eq] at h
                                    cases h
                                    exact BigStep.EvalFunction.body
                                      (evaluatedState := evaluation.state)
                                      (namespace_eq := namespace_eq)
                                      (declaration_eq := declaration_eq)
                                      (frame_eq := frame_eq) (body_eq := body_eq)
                                      (body_step := by simpa [control_eq] using body_sound)
                                      (outcome_eq := by simp [finishControl?])
                                      (finalize_eq := rfl)
                                | break_ nest value =>
                                    simp [namespace_eq, declaration_eq, arity_ne, frame_eq, body_eq,
                                      evaluation_eq, control_eq, failAt] at h
                                | continue_ nest =>
                                    simp [namespace_eq, declaration_eq, arity_ne, frame_eq, body_eq,
                                      evaluation_eq, control_eq, failAt] at h
        expression := by
          intro executable namespaceId frame state exprId result h
          simp only [Internal.evalExpr] at h
          cases namespace_eq : executable.unit.namespaces[namespaceId.index]? with
          | none => simp [namespace_eq, failAt] at h
          | some ns =>
              cases expression_eq : ns.expressions[exprId.index]? with
              | none => simp [namespace_eq, expression_eq, failAt] at h
              | some expression =>
                  cases kind_eq : expression.kind with
                  | value literal source =>
                      cases value_eq : constValue? literal with
                      | none =>
                          simp [namespace_eq, expression_eq, kind_eq, value_eq, failAt] at h
                      | some runtimeValue =>
                          simp [namespace_eq, expression_eq, kind_eq, value_eq] at h
                          cases h
                          exact BigStep.EvalExpr.value
                            (frame := frame) (state := state)
                            (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                            (kind_eq := kind_eq) (value_eq := value_eq)
                  | constant reference =>
                      cases resolve_eq : resolveConstant? executable.unit namespaceId reference with
                      | none =>
                          simp [namespace_eq, expression_eq, kind_eq, resolve_eq, failAt] at h
                      | some handle =>
                          cases target_namespace_eq :
                              executable.unit.namespaces[handle.namespaceId.index]? with
                          | none =>
                              simp [namespace_eq, expression_eq, kind_eq, resolve_eq,
                                target_namespace_eq, failAt] at h
                          | some targetNs =>
                              cases declaration_eq : targetNs.constants[handle.constantId]? with
                              | none =>
                                  simp [namespace_eq, expression_eq, kind_eq, resolve_eq,
                                    target_namespace_eq, declaration_eq, failAt] at h
                              | some declaration =>
                                  cases initializer_eq : Internal.evalExpr fuel executable
                                      handle.namespaceId { locals := #[] } state declaration.value with
                                  | error error =>
                                      simp [namespace_eq, expression_eq, kind_eq, resolve_eq,
                                        target_namespace_eq, declaration_eq, initializer_eq] at h
                                  | ok initialized =>
                                      have initializer_sound := previous.expression executable
                                        handle.namespaceId { locals := #[] } state declaration.value
                                        initialized initializer_eq
                                      cases control_eq : initialized.control.value with
                                      | value runtimeValue =>
                                          simp [namespace_eq, expression_eq, kind_eq, resolve_eq,
                                            target_namespace_eq, declaration_eq, initializer_eq,
                                            control_eq] at h
                                          cases h
                                          exact BigStep.EvalExpr.constantValue
                                            (frame := frame)
                                            (namespace_eq := namespace_eq)
                                            (expression_eq := expression_eq) (kind_eq := kind_eq)
                                            (resolve_eq := resolve_eq)
                                            (target_namespace_eq := target_namespace_eq)
                                            (declaration_eq := declaration_eq)
                                            (initializer := by
                                              simpa [control_eq] using initializer_sound)
                                      | break_ nest value =>
                                          simp [namespace_eq, expression_eq, kind_eq, resolve_eq,
                                            target_namespace_eq, declaration_eq, initializer_eq,
                                            control_eq, Located.pushCaller] at h
                                          cases h
                                          exact BigStep.EvalExpr.constantControl
                                            (frame := frame)
                                            (namespace_eq := namespace_eq)
                                            (expression_eq := expression_eq) (kind_eq := kind_eq)
                                            (resolve_eq := resolve_eq)
                                            (target_namespace_eq := target_namespace_eq)
                                            (declaration_eq := declaration_eq)
                                            (initializer := by simpa [control_eq] using initializer_sound)
                                            (abrupt := .break_ nest value)
                                      | continue_ nest =>
                                          simp [namespace_eq, expression_eq, kind_eq, resolve_eq,
                                            target_namespace_eq, declaration_eq, initializer_eq,
                                            control_eq, Located.pushCaller] at h
                                          cases h
                                          exact BigStep.EvalExpr.constantControl
                                            (frame := frame)
                                            (namespace_eq := namespace_eq)
                                            (expression_eq := expression_eq) (kind_eq := kind_eq)
                                            (resolve_eq := resolve_eq)
                                            (target_namespace_eq := target_namespace_eq)
                                            (declaration_eq := declaration_eq)
                                            (initializer := by simpa [control_eq] using initializer_sound)
                                            (abrupt := .continue_ nest)
                                      | return_ values =>
                                          simp [namespace_eq, expression_eq, kind_eq, resolve_eq,
                                            target_namespace_eq, declaration_eq, initializer_eq,
                                            control_eq, Located.pushCaller] at h
                                          cases h
                                          exact BigStep.EvalExpr.constantControl
                                            (frame := frame)
                                            (namespace_eq := namespace_eq)
                                            (expression_eq := expression_eq) (kind_eq := kind_eq)
                                            (resolve_eq := resolve_eq)
                                            (target_namespace_eq := target_namespace_eq)
                                            (declaration_eq := declaration_eq)
                                            (initializer := by simpa [control_eq] using initializer_sound)
                                            (abrupt := .return_ values)
                                      | throw_ kind arguments =>
                                          simp [namespace_eq, expression_eq, kind_eq, resolve_eq,
                                            target_namespace_eq, declaration_eq, initializer_eq,
                                            control_eq, Located.pushCaller] at h
                                          cases h
                                          exact BigStep.EvalExpr.constantControl
                                            (frame := frame)
                                            (namespace_eq := namespace_eq)
                                            (expression_eq := expression_eq) (kind_eq := kind_eq)
                                            (resolve_eq := resolve_eq)
                                            (target_namespace_eq := target_namespace_eq)
                                            (declaration_eq := declaration_eq)
                                            (initializer := by simpa [control_eq] using initializer_sound)
                                            (abrupt := .throw_ kind arguments)
                  | localVar localId =>
                      cases local_eq : readLocal? frame localId with
                      | none =>
                          simp [namespace_eq, expression_eq, kind_eq, local_eq, failAt] at h
                      | some runtimeValue =>
                          simp [namespace_eq, expression_eq, kind_eq, local_eq] at h
                          cases h
                          exact BigStep.EvalExpr.localVar
                            (frame := frame) (state := state)
                            (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                            (kind_eq := kind_eq) (local_eq := local_eq)
                  | operation operation instantiations arguments surface =>
                      cases operation with
                      | call callKind =>
                          cases callKind with
                          | function reference =>
                              cases operands_eq : Internal.evalValues fuel executable namespaceId
                                  frame state arguments.toList with
                              | error error =>
                                  simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                              | ok operands =>
                                  have operands_sound := previous.values executable namespaceId frame
                                    state arguments.toList operands operands_eq
                                  cases operands with
                                  | control operandState operandFrame control =>
                                      simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                                      cases h
                                      exact BigStep.EvalExpr.callArgumentsControl
                                        (namespace_eq := namespace_eq)
                                        (expression_eq := expression_eq) (kind_eq := kind_eq)
                                        (operands := operands_sound)
                                  | values operandState operandFrame values =>
                                      cases resolve_eq : resolveFunction? executable.unit namespaceId
                                          reference with
                                      | none =>
                                          simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                            resolve_eq, failAt] at h
                                      | some handle =>
                                          cases callee_eq : Internal.evalFunction fuel executable handle
                                              operandState values.toArray with
                                          | error error =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                resolve_eq, callee_eq, Located.pushCaller] at h
                                          | ok calleeResult =>
                                              have callee_sound := previous.function executable handle
                                                operandState values.toArray calleeResult callee_eq
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                resolve_eq, callee_eq] at h
                                              cases h
                                              cases outcome_eq : calleeResult.outcome.value with
                                              | returned results =>
                                                  simpa [Internal.callResult, outcome_eq] using
                                                    (BigStep.EvalExpr.callReturned
                                                      (namespace_eq := namespace_eq)
                                                      (expression_eq := expression_eq)
                                                      (kind_eq := kind_eq) (operands := operands_sound)
                                                      (resolve_eq := resolve_eq)
                                                      (calleeStep := by
                                                        simpa [outcome_eq] using callee_sound))
                                              | threw kind thrown =>
                                                  simpa [Internal.callResult, outcome_eq] using
                                                    (BigStep.EvalExpr.callThrew
                                                      (namespace_eq := namespace_eq)
                                                      (expression_eq := expression_eq)
                                                      (kind_eq := kind_eq) (operands := operands_sound)
                                                      (resolve_eq := resolve_eq)
                                                      (calleeStep := by
                                                        simpa [outcome_eq] using callee_sound))
                          | constructor reference variant =>
                              cases operands_eq : Internal.evalValues fuel executable namespaceId
                                  frame state arguments.toList with
                              | error error =>
                                  simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                              | ok operands =>
                                  have operands_sound := previous.values executable namespaceId frame
                                    state arguments.toList operands operands_eq
                                  cases operands with
                                  | control operandState operandFrame control =>
                                      simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                                      cases h
                                      exact BigStep.EvalExpr.constructorArgumentsControl
                                        (namespace_eq := namespace_eq)
                                        (expression_eq := expression_eq) (kind_eq := kind_eq)
                                        (operands := operands_sound)
                                  | values operandState operandFrame values =>
                                      cases construct_eq : constructNominal? executable.unit namespaceId
                                          reference variant values.toArray with
                                      | none =>
                                          simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                            construct_eq, failAt] at h
                                      | some runtimeValue =>
                                          simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                            construct_eq] at h
                                          cases h
                                          exact BigStep.EvalExpr.constructorValue
                                            (namespace_eq := namespace_eq)
                                            (expression_eq := expression_eq) (kind_eq := kind_eq)
                                            (operands := operands_sound) (construct_eq := construct_eq)
                          | destructor reference variant =>
                              cases operands_eq : Internal.evalValues fuel executable namespaceId
                                  frame state arguments.toList with
                              | error error =>
                                  simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                              | ok operands =>
                                  have operands_sound := previous.values executable namespaceId frame
                                    state arguments.toList operands operands_eq
                                  cases operands with
                                  | control operandState operandFrame control =>
                                      simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                                      cases h
                                      exact BigStep.EvalExpr.destructorArgumentsControl
                                        (namespace_eq := namespace_eq)
                                        (expression_eq := expression_eq) (kind_eq := kind_eq)
                                        (operands := operands_sound)
                                  | values operandState operandFrame values =>
                                      cases values with
                                      | nil =>
                                          simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                            failAt] at h
                                      | cons value tail =>
                                          cases tail with
                                          | nil =>
                                              cases destruct_eq : destructNominal? executable.unit
                                                  namespaceId reference variant value with
                                              | none =>
                                                  simp [namespace_eq, expression_eq, kind_eq,
                                                    operands_eq, destruct_eq, failAt] at h
                                              | some fields =>
                                                  simp [namespace_eq, expression_eq, kind_eq,
                                                    operands_eq, destruct_eq] at h
                                                  cases h
                                                  exact BigStep.EvalExpr.destructorValue
                                                    (namespace_eq := namespace_eq)
                                                    (expression_eq := expression_eq)
                                                    (kind_eq := kind_eq)
                                                    (operands := operands_sound)
                                                    (destruct_eq := destruct_eq)
                                          | cons next rest =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                          | closure reference =>
                              cases operands_eq : Internal.evalValues fuel executable namespaceId
                                  frame state arguments.toList with
                              | error error =>
                                  simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                              | ok operands =>
                                  have operands_sound := previous.values executable namespaceId frame
                                    state arguments.toList operands operands_eq
                                  cases operands with
                                  | control operandState operandFrame control =>
                                      simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                                      cases h
                                      exact BigStep.EvalExpr.closureArgumentsControl
                                        (namespace_eq := namespace_eq)
                                        (expression_eq := expression_eq) (kind_eq := kind_eq)
                                        (operands := operands_sound)
                                  | values operandState operandFrame values =>
                                      cases resolve_eq : resolveFunction? executable.unit namespaceId
                                          reference with
                                      | none =>
                                          simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                            resolve_eq, failAt] at h
                                      | some handle =>
                                          simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                            resolve_eq] at h
                                          cases h
                                          exact BigStep.EvalExpr.closureValue
                                            (namespace_eq := namespace_eq)
                                            (expression_eq := expression_eq) (kind_eq := kind_eq)
                                            (operands := operands_sound) (resolve_eq := resolve_eq)
                          | invoke =>
                              cases operands_eq : Internal.evalValues fuel executable namespaceId
                                  frame state arguments.toList with
                              | error error =>
                                  simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                              | ok operands =>
                                  have operands_sound := previous.values executable namespaceId frame
                                    state arguments.toList operands operands_eq
                                  cases operands with
                                  | control operandState operandFrame control =>
                                      simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                                      cases h
                                      exact BigStep.EvalExpr.invokeArgumentsControl
                                        (namespace_eq := namespace_eq)
                                        (expression_eq := expression_eq) (kind_eq := kind_eq)
                                        (operands := operands_sound)
                                  | values operandState operandFrame values =>
                                      cases values with
                                      | nil =>
                                          simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                            failAt] at h
                                      | cons callable arguments =>
                                          cases callable with
                                          | closure handle captures =>
                                              cases callee_eq : Internal.evalFunction fuel executable
                                                  handle operandState (captures ++ arguments.toArray) with
                                              | error error =>
                                                  simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                    callee_eq, Located.pushCaller] at h
                                              | ok calleeResult =>
                                                  have callee_sound := previous.function executable handle
                                                    operandState (captures ++ arguments.toArray)
                                                    calleeResult callee_eq
                                                  simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                    callee_eq] at h
                                                  cases h
                                                  cases outcome_eq : calleeResult.outcome.value with
                                                  | returned results =>
                                                      simpa [Internal.callResult, outcome_eq] using
                                                        (BigStep.EvalExpr.invokeReturned
                                                          (namespace_eq := namespace_eq)
                                                          (expression_eq := expression_eq)
                                                          (kind_eq := kind_eq)
                                                          (operands := operands_sound)
                                                          (calleeStep := by
                                                            simpa [outcome_eq] using callee_sound))
                                                  | threw kind thrown =>
                                                      simpa [Internal.callResult, outcome_eq] using
                                                        (BigStep.EvalExpr.invokeThrew
                                                          (namespace_eq := namespace_eq)
                                                          (expression_eq := expression_eq)
                                                          (kind_eq := kind_eq)
                                                          (operands := operands_sound)
                                                          (calleeStep := by
                                                            simpa [outcome_eq] using callee_sound))
                                          | unit =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | bool _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | character _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | integer _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | address _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | signer _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | string _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | bytes _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | vector _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | tuple _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | nominal _ _ _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | borrow _ _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | loanHole _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                          | extension value targets =>
                              cases operands_eq : Internal.evalValues fuel executable namespaceId frame
                                  state arguments.toList with
                              | error error =>
                                  simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                              | ok operands =>
                                  have operands_sound := previous.values executable namespaceId frame
                                    state arguments.toList operands operands_eq
                                  cases operands with
                                  | control operandState operandFrame control =>
                                      simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                                      cases h
                                      exact BigStep.EvalExpr.operationArgumentsControl
                                        (namespace_eq := namespace_eq)
                                        (expression_eq := expression_eq) (kind_eq := kind_eq)
                                        (operands := operands_sound)
                                  | values operandState operandFrame values =>
                                      cases evaluate_eq : evaluatePlaceOperation? executable.unit ns
                                          expression.typeId exprId (.call (.extension value targets))
                                          values.toArray operandFrame operandState with
                                      | none =>
                                          simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                            evaluate_eq, failAt] at h
                                      | some evaluated =>
                                          rcases evaluated with ⟨finalFrame, finalState, runtimeValue⟩
                                          simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                            evaluate_eq] at h
                                          cases h
                                          exact BigStep.EvalExpr.operationValue
                                            (namespace_eq := namespace_eq)
                                            (expression_eq := expression_eq) (kind_eq := kind_eq)
                                            (operands := operands_sound) (evaluate_eq := evaluate_eq)
                      | profile operation targets =>
                          cases operands_eq : Internal.evalValues fuel executable namespaceId frame
                              state arguments.toList with
                          | error error =>
                              simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                          | ok operands =>
                              have operands_sound := previous.values executable namespaceId frame state
                                arguments.toList operands operands_eq
                              cases operands with
                              | control operandState operandFrame control =>
                                  simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                                  cases h
                                  exact BigStep.EvalExpr.profileArgumentsControl
                                    (namespace_eq := namespace_eq)
                                    (expression_eq := expression_eq) (kind_eq := kind_eq)
                                    (operands := operands_sound)
                              | values operandState operandFrame values =>
                                  cases evaluate_eq : evaluateProfileOperation? executable ns
                                      expression.typeId operation values.toArray with
                                  | none =>
                                      simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                        evaluate_eq, failAt] at h
                                  | some evaluated =>
                                      cases evaluated with
                                      | ok runtimeValue =>
                                          simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                            evaluate_eq] at h
                                          cases h
                                          exact BigStep.EvalExpr.profileValue
                                            (namespace_eq := namespace_eq)
                                            (expression_eq := expression_eq) (kind_eq := kind_eq)
                                            (operands := operands_sound) (evaluate_eq := evaluate_eq)
                                      | error thrown =>
                                          rcases thrown with ⟨throwKind, thrownValues⟩
                                          simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                            evaluate_eq] at h
                                          cases h
                                          exact BigStep.EvalExpr.profileThrow
                                            (namespace_eq := namespace_eq)
                                            (expression_eq := expression_eq) (kind_eq := kind_eq)
                                            (operands := operands_sound) (evaluate_eq := evaluate_eq)
                      | primitive operation =>
                          cases operands_eq : Internal.evalValues fuel executable namespaceId frame
                              state arguments.toList with
                          | error error =>
                              simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                          | ok operands =>
                              have operands_sound := previous.values executable namespaceId frame state
                                arguments.toList operands operands_eq
                              cases operands with
                              | control operandState operandFrame control =>
                                  simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                                  cases h
                                  exact BigStep.EvalExpr.primitiveArgumentsControl
                                    (namespace_eq := namespace_eq)
                                    (expression_eq := expression_eq) (kind_eq := kind_eq)
                                    (operands := operands_sound)
                              | values operandState operandFrame values =>
                                  cases evaluate_eq : evaluatePrimitiveOperation? ns expression.typeId
                                      operation values.toArray
                                      executable.targetPointerWidth with
                                  | none =>
                                      simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                        evaluate_eq, failAt] at h
                                  | some evaluated =>
                                      cases evaluated with
                                      | ok runtimeValue =>
                                          simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                            evaluate_eq] at h
                                          cases h
                                          exact BigStep.EvalExpr.primitiveValue
                                            (namespace_eq := namespace_eq)
                                            (expression_eq := expression_eq) (kind_eq := kind_eq)
                                            (operands := operands_sound) (evaluate_eq := evaluate_eq)
                                      | error thrown =>
                                          rcases thrown with ⟨throwKind, thrownValues⟩
                                          simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                            evaluate_eq] at h
                                          cases h
                                          exact BigStep.EvalExpr.primitiveThrow
                                            (namespace_eq := namespace_eq)
                                            (expression_eq := expression_eq) (kind_eq := kind_eq)
                                            (operands := operands_sound) (evaluate_eq := evaluate_eq)
                      | reference referenceOperation =>
                          cases operands_eq : Internal.evalValues fuel executable namespaceId frame
                              state arguments.toList with
                          | error error =>
                              simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                          | ok operands =>
                              have operands_sound := previous.values executable namespaceId frame state
                                arguments.toList operands operands_eq
                              cases operands with
                              | control operandState operandFrame control =>
                                  simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                                  cases h
                                  exact BigStep.EvalExpr.operationArgumentsControl
                                    (namespace_eq := namespace_eq)
                                    (expression_eq := expression_eq) (kind_eq := kind_eq)
                                    (operands := operands_sound)
                              | values operandState operandFrame values =>
                                  cases evaluate_eq : evaluatePlaceOperation? executable.unit ns
                                      expression.typeId exprId (.reference referenceOperation)
                                      values.toArray operandFrame operandState with
                                  | none =>
                                      simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                        evaluate_eq, failAt] at h
                                  | some evaluated =>
                                      rcases evaluated with ⟨finalFrame, finalState, runtimeValue⟩
                                      simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                        evaluate_eq] at h
                                      cases h
                                      exact BigStep.EvalExpr.operationValue
                                        (namespace_eq := namespace_eq)
                                        (expression_eq := expression_eq) (kind_eq := kind_eq)
                                        (operands := operands_sound) (evaluate_eq := evaluate_eq)
                      | data dataOperation =>
                          cases operands_eq : Internal.evalValues fuel executable namespaceId frame
                              state arguments.toList with
                          | error error =>
                              simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                          | ok operands =>
                              have operands_sound := previous.values executable namespaceId frame state
                                arguments.toList operands operands_eq
                              cases operands with
                              | control operandState operandFrame control =>
                                  simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                                  cases h
                                  exact BigStep.EvalExpr.operationArgumentsControl
                                    (namespace_eq := namespace_eq)
                                    (expression_eq := expression_eq) (kind_eq := kind_eq)
                                    (operands := operands_sound)
                              | values operandState operandFrame values =>
                                  cases evaluate_eq : evaluateDataOperation? executable.unit
                                      ns.identity dataOperation values.toArray with
                                  | none =>
                                      simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                        evaluate_eq, failAt] at h
                                  | some runtimeValue =>
                                      simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                        evaluate_eq] at h
                                      cases h
                                      exact BigStep.EvalExpr.operationValue
                                        (namespace_eq := namespace_eq)
                                        (expression_eq := expression_eq) (kind_eq := kind_eq)
                                        (operands := operands_sound) (evaluate_eq := by
                                          simp [evaluatePlaceOperation?, evaluate_eq])
                      | specification specificationOperation =>
                          cases operands_eq : Internal.evalValues fuel executable namespaceId frame
                              state arguments.toList with
                          | error error =>
                              simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                          | ok operands =>
                              have operands_sound := previous.values executable namespaceId frame state
                                arguments.toList operands operands_eq
                              cases operands with
                              | control operandState operandFrame control =>
                                  simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                                  cases h
                                  exact BigStep.EvalExpr.operationArgumentsControl
                                    (namespace_eq := namespace_eq)
                                    (expression_eq := expression_eq) (kind_eq := kind_eq)
                                    (operands := operands_sound)
                              | values operandState operandFrame values =>
                                  cases evaluate_eq : evaluatePlaceOperation? executable.unit ns
                                      expression.typeId exprId (.specification specificationOperation)
                                      values.toArray operandFrame operandState with
                                  | none =>
                                      simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                        evaluate_eq, failAt] at h
                                  | some evaluated =>
                                      rcases evaluated with ⟨finalFrame, finalState, runtimeValue⟩
                                      simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                        evaluate_eq] at h
                                      cases h
                                      exact BigStep.EvalExpr.operationValue
                                        (namespace_eq := namespace_eq)
                                        (expression_eq := expression_eq) (kind_eq := kind_eq)
                                        (operands := operands_sound) (evaluate_eq := evaluate_eq)
                      | global globalKind =>
                          cases operands_eq : Internal.evalValues fuel executable namespaceId frame
                              state arguments.toList with
                          | error error =>
                              simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                          | ok operands =>
                              have operands_sound := previous.values executable namespaceId frame state
                                arguments.toList operands operands_eq
                              cases operands with
                              | control operandState operandFrame control =>
                                  simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                                  cases h
                                  exact BigStep.EvalExpr.globalArgumentsControl
                                    (namespace_eq := namespace_eq)
                                    (expression_eq := expression_eq) (kind_eq := kind_eq)
                                    (operands := operands_sound)
                              | values operandState operandFrame values =>
                                  cases evaluate_eq : evaluateGlobalOperation? executable.unit ns expression.typeId exprId
                                      globalKind instantiations values.toArray operandFrame operandState with
                                  | none =>
                                      simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                        evaluate_eq, failAt] at h
                                  | some evaluated =>
                                      cases evaluated with
                                      | value finalFrame finalState runtimeValue =>
                                          simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                            evaluate_eq] at h
                                          cases h
                                          exact BigStep.EvalExpr.globalValue
                                            (namespace_eq := namespace_eq)
                                            (expression_eq := expression_eq) (kind_eq := kind_eq)
                                            (operands := operands_sound) (evaluate_eq := evaluate_eq)
                                      | throw_ finalFrame finalState throwKind thrown =>
                                          simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                            evaluate_eq] at h
                                          cases h
                                          exact BigStep.EvalExpr.globalThrow
                                            (namespace_eq := namespace_eq)
                                            (expression_eq := expression_eq) (kind_eq := kind_eq)
                                            (operands := operands_sound) (evaluate_eq := evaluate_eq)
                      | assert =>
                          cases operands_eq : Internal.evalValues fuel executable namespaceId frame
                              state arguments.toList with
                          | error error =>
                              simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                          | ok operands =>
                              have operands_sound := previous.values executable namespaceId frame state
                                arguments.toList operands operands_eq
                              cases operands with
                              | control operandState operandFrame control =>
                                  simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                                  cases h
                                  exact BigStep.EvalExpr.assertArgumentsControl
                                    (namespace_eq := namespace_eq)
                                    (expression_eq := expression_eq) (kind_eq := kind_eq)
                                    (operands := operands_sound)
                              | values operandState operandFrame values =>
                                  cases values with
                                  | nil =>
                                      simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                        failAt] at h
                                  | cons actual tail =>
                                      cases tail with
                                      | cons next rest =>
                                          simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                            failAt] at h
                                      | nil =>
                                          cases actual with
                                          | bool boolean =>
                                              cases boolean with
                                              | false =>
                                                  simp [namespace_eq, expression_eq, kind_eq,
                                                    operands_eq] at h
                                                  cases h
                                                  exact BigStep.EvalExpr.assertFalse
                                                    (namespace_eq := namespace_eq)
                                                    (expression_eq := expression_eq)
                                                    (kind_eq := kind_eq)
                                                    (operands := operands_sound)
                                              | true =>
                                                  simp [namespace_eq, expression_eq, kind_eq,
                                                    operands_eq] at h
                                                  cases h
                                                  exact BigStep.EvalExpr.assertTrue
                                                    (namespace_eq := namespace_eq)
                                                    (expression_eq := expression_eq)
                                                    (kind_eq := kind_eq)
                                                    (operands := operands_sound)
                                          | unit =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | character _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | integer _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | address _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | signer _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | string _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | bytes _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | vector _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | tuple _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | nominal _ _ _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | closure _ _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | borrow _ _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                                          | loanHole _ =>
                                              simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                                failAt] at h
                      | move place =>
                          cases operands_eq : Internal.evalValues fuel executable namespaceId frame
                              state arguments.toList with
                          | error error => simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                          | ok operands =>
                              have operands_sound := previous.values executable namespaceId frame state
                                arguments.toList operands operands_eq
                              cases operands with
                              | control operandState operandFrame control =>
                                  simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                                  cases h
                                  exact BigStep.EvalExpr.operationArgumentsControl
                                    (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                    (kind_eq := kind_eq) (operands := operands_sound)
                              | values operandState operandFrame values =>
                                  cases evaluate_eq : evaluatePlaceOperation? executable.unit ns
                                      expression.typeId exprId (.move place) values.toArray operandFrame
                                      operandState with
                                  | none => simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                      evaluate_eq, failAt] at h
                                  | some evaluated =>
                                      rcases evaluated with ⟨finalFrame, finalState, runtimeValue⟩
                                      simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                        evaluate_eq] at h
                                      cases h
                                      exact BigStep.EvalExpr.operationValue
                                        (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                        (kind_eq := kind_eq) (operands := operands_sound)
                                        (evaluate_eq := evaluate_eq)
                      | copy place =>
                          cases operands_eq : Internal.evalValues fuel executable namespaceId frame
                              state arguments.toList with
                          | error error => simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                          | ok operands =>
                              have operands_sound := previous.values executable namespaceId frame state
                                arguments.toList operands operands_eq
                              cases operands with
                              | control operandState operandFrame control =>
                                  simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                                  cases h
                                  exact BigStep.EvalExpr.operationArgumentsControl
                                    (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                    (kind_eq := kind_eq) (operands := operands_sound)
                              | values operandState operandFrame values =>
                                  cases evaluate_eq : evaluatePlaceOperation? executable.unit ns
                                      expression.typeId exprId (.copy place) values.toArray operandFrame
                                      operandState with
                                  | none => simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                      evaluate_eq, failAt] at h
                                  | some evaluated =>
                                      rcases evaluated with ⟨finalFrame, finalState, runtimeValue⟩
                                      simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                        evaluate_eq] at h
                                      cases h
                                      exact BigStep.EvalExpr.operationValue
                                        (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                        (kind_eq := kind_eq) (operands := operands_sound)
                                        (evaluate_eq := evaluate_eq)
                      | borrow borrowKind place =>
                          cases operands_eq : Internal.evalValues fuel executable namespaceId frame
                              state arguments.toList with
                          | error error => simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                          | ok operands =>
                              have operands_sound := previous.values executable namespaceId frame state
                                arguments.toList operands operands_eq
                              cases operands with
                              | control operandState operandFrame control =>
                                  simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                                  cases h
                                  exact BigStep.EvalExpr.operationArgumentsControl
                                    (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                    (kind_eq := kind_eq) (operands := operands_sound)
                              | values operandState operandFrame values =>
                                  cases evaluate_eq : evaluatePlaceOperation? executable.unit ns
                                      expression.typeId exprId (.borrow borrowKind place) values.toArray
                                      operandFrame operandState with
                                  | none => simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                      evaluate_eq, failAt] at h
                                  | some evaluated =>
                                      rcases evaluated with ⟨finalFrame, finalState, runtimeValue⟩
                                      simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                        evaluate_eq] at h
                                      cases h
                                      exact BigStep.EvalExpr.operationValue
                                        (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                        (kind_eq := kind_eq) (operands := operands_sound)
                                        (evaluate_eq := evaluate_eq)
                      | read place =>
                          cases operands_eq : Internal.evalValues fuel executable namespaceId frame
                              state arguments.toList with
                          | error error => simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                          | ok operands =>
                              have operands_sound := previous.values executable namespaceId frame state
                                arguments.toList operands operands_eq
                              cases operands with
                              | control operandState operandFrame control =>
                                  simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                                  cases h
                                  exact BigStep.EvalExpr.operationArgumentsControl
                                    (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                    (kind_eq := kind_eq) (operands := operands_sound)
                              | values operandState operandFrame values =>
                                  cases evaluate_eq : evaluatePlaceOperation? executable.unit ns
                                      expression.typeId exprId (.read place) values.toArray operandFrame
                                      operandState with
                                  | none => simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                      evaluate_eq, failAt] at h
                                  | some evaluated =>
                                      rcases evaluated with ⟨finalFrame, finalState, runtimeValue⟩
                                      simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                        evaluate_eq] at h
                                      cases h
                                      exact BigStep.EvalExpr.operationValue
                                        (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                        (kind_eq := kind_eq) (operands := operands_sound)
                                        (evaluate_eq := evaluate_eq)
                      | write place =>
                          cases operands_eq : Internal.evalValues fuel executable namespaceId frame
                              state arguments.toList with
                          | error error => simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                          | ok operands =>
                              have operands_sound := previous.values executable namespaceId frame state
                                arguments.toList operands operands_eq
                              cases operands with
                              | control operandState operandFrame control =>
                                  simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                                  cases h
                                  exact BigStep.EvalExpr.operationArgumentsControl
                                    (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                    (kind_eq := kind_eq) (operands := operands_sound)
                              | values operandState operandFrame values =>
                                  cases evaluate_eq : evaluatePlaceOperation? executable.unit ns
                                      expression.typeId exprId (.write place) values.toArray operandFrame
                                      operandState with
                                  | none => simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                      evaluate_eq, failAt] at h
                                  | some evaluated =>
                                      rcases evaluated with ⟨finalFrame, finalState, runtimeValue⟩
                                      simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                        evaluate_eq] at h
                                      cases h
                                      exact BigStep.EvalExpr.operationValue
                                        (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                        (kind_eq := kind_eq) (operands := operands_sound)
                                        (evaluate_eq := evaluate_eq)
                      | drop place =>
                          cases operands_eq : Internal.evalValues fuel executable namespaceId frame
                              state arguments.toList with
                          | error error => simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                          | ok operands =>
                              have operands_sound := previous.values executable namespaceId frame state
                                arguments.toList operands operands_eq
                              cases operands with
                              | control operandState operandFrame control =>
                                  simp [namespace_eq, expression_eq, kind_eq, operands_eq] at h
                                  cases h
                                  exact BigStep.EvalExpr.operationArgumentsControl
                                    (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                    (kind_eq := kind_eq) (operands := operands_sound)
                              | values operandState operandFrame values =>
                                  cases evaluate_eq : evaluatePlaceOperation? executable.unit ns
                                      expression.typeId exprId (.drop place) values.toArray operandFrame
                                      operandState with
                                  | none => simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                      evaluate_eq, failAt] at h
                                  | some evaluated =>
                                      rcases evaluated with ⟨finalFrame, finalState, runtimeValue⟩
                                      simp [namespace_eq, expression_eq, kind_eq, operands_eq,
                                        evaluate_eq] at h
                                      cases h
                                      exact BigStep.EvalExpr.operationValue
                                        (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                        (kind_eq := kind_eq) (operands := operands_sound)
                                        (evaluate_eq := evaluate_eq)
                  | block statements child =>
                      cases statements_eq : Internal.evalStatements fuel executable namespaceId
                          frame state statements.toList with
                      | error error =>
                          simp [namespace_eq, expression_eq, kind_eq, statements_eq] at h
                      | ok statementsResult =>
                          have statements_sound := previous.statements executable namespaceId frame
                            state statements.toList statementsResult statements_eq
                          cases statementsResult with
                          | control finalState finalFrame control =>
                              simp [namespace_eq, expression_eq, kind_eq, statements_eq] at h
                              cases h
                              exact BigStep.EvalExpr.blockControl
                                (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                (kind_eq := kind_eq) (steps := statements_sound)
                          | done statementState statementFrame =>
                              cases child with
                              | none =>
                                  simp [namespace_eq, expression_eq, kind_eq, statements_eq] at h
                                  cases h
                                  exact BigStep.EvalExpr.blockUnit
                                    (namespace_eq := namespace_eq)
                                    (expression_eq := expression_eq) (kind_eq := kind_eq)
                                    (steps := statements_sound)
                              | some child =>
                                  cases child_eq : Internal.evalExpr fuel executable namespaceId
                                      statementFrame statementState child with
                                  | error error =>
                                      simp [namespace_eq, expression_eq, kind_eq, statements_eq,
                                        child_eq] at h
                                  | ok childResult =>
                                      have child_sound := previous.expression executable namespaceId
                                        statementFrame statementState child childResult child_eq
                                      cases control_eq : childResult.control.value <;>
                                        simp [namespace_eq, expression_eq, kind_eq, statements_eq,
                                          child_eq, control_eq] at h <;> cases h <;>
                                        exact BigStep.EvalExpr.blockResult
                                          (namespace_eq := namespace_eq)
                                          (expression_eq := expression_eq) (kind_eq := kind_eq)
                                          (steps := statements_sound)
                                          (result_step := by simpa [control_eq] using child_sound)
                  | letDecl pattern initializer body =>
                      cases initializer with
                      | none =>
                          cases body_eq : Internal.evalExpr fuel executable namespaceId frame state body with
                          | error error =>
                              simp [namespace_eq, expression_eq, kind_eq, body_eq] at h
                          | ok bodyResult =>
                              have body_sound := previous.expression executable namespaceId frame state
                                body bodyResult body_eq
                              cases control_eq : bodyResult.control.value <;>
                                simp [namespace_eq, expression_eq, kind_eq, body_eq, control_eq] at h <;>
                                cases h <;> exact BigStep.EvalExpr.letNoValue
                                  (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                  (kind_eq := kind_eq)
                                  (body_step := by simpa [control_eq] using body_sound)
                      | some initializer =>
                          cases initializer_eq : Internal.evalExpr fuel executable namespaceId frame
                              state initializer with
                          | error error =>
                              simp [namespace_eq, expression_eq, kind_eq, initializer_eq] at h
                          | ok initialized =>
                              have initializer_sound := previous.expression executable namespaceId
                                frame state initializer initialized initializer_eq
                              cases control_eq : initialized.control.value with
                              | value runtimeValue =>
                                  cases bind_eq : bindPattern executable.unit ns initialized.frame pattern runtimeValue with
                                  | none =>
                                      simp [namespace_eq, expression_eq, kind_eq, initializer_eq,
                                        control_eq, bind_eq, failAt] at h
                                  | some boundFrame =>
                                      cases body_eq : Internal.evalExpr fuel executable namespaceId
                                          boundFrame initialized.state body with
                                      | error error =>
                                          simp [namespace_eq, expression_eq, kind_eq, initializer_eq,
                                            control_eq, bind_eq, body_eq] at h
                                      | ok bodyResult =>
                                          have body_sound := previous.expression executable namespaceId
                                            boundFrame initialized.state body bodyResult body_eq
                                          cases body_control_eq : bodyResult.control.value <;>
                                            simp [namespace_eq, expression_eq, kind_eq, initializer_eq,
                                              control_eq, bind_eq, body_eq, body_control_eq] at h <;>
                                            cases h <;> exact BigStep.EvalExpr.letValue
                                              (namespace_eq := namespace_eq)
                                              (expression_eq := expression_eq) (kind_eq := kind_eq)
                                              (initializer_step := by
                                                simpa [control_eq] using initializer_sound)
                                              (bind_eq := bind_eq)
                                              (body_step := by
                                                simpa [body_control_eq] using body_sound)
                              | break_ nest value =>
                                  simp [namespace_eq, expression_eq, kind_eq, initializer_eq,
                                    control_eq] at h
                                  cases h
                                  simpa [control_eq] using
                                    (BigStep.EvalExpr.letValueControl
                                      (control := .break_ nest value)
                                      (namespace_eq := namespace_eq)
                                      (expression_eq := expression_eq) (kind_eq := kind_eq)
                                      (initializer_step := by
                                        simpa [control_eq] using initializer_sound)
                                      (abrupt := BigStep.Abrupt.break_ nest value))
                              | continue_ nest =>
                                  simp [namespace_eq, expression_eq, kind_eq, initializer_eq,
                                    control_eq] at h
                                  cases h
                                  simpa [control_eq] using
                                    (BigStep.EvalExpr.letValueControl
                                      (control := .continue_ nest)
                                      (namespace_eq := namespace_eq)
                                      (expression_eq := expression_eq) (kind_eq := kind_eq)
                                      (initializer_step := by
                                        simpa [control_eq] using initializer_sound)
                                      (abrupt := BigStep.Abrupt.continue_ nest))
                              | return_ values =>
                                  simp [namespace_eq, expression_eq, kind_eq, initializer_eq,
                                    control_eq] at h
                                  cases h
                                  simpa [control_eq] using
                                    (BigStep.EvalExpr.letValueControl
                                      (control := .return_ values)
                                      (namespace_eq := namespace_eq)
                                      (expression_eq := expression_eq) (kind_eq := kind_eq)
                                      (initializer_step := by
                                        simpa [control_eq] using initializer_sound)
                                      (abrupt := BigStep.Abrupt.return_ values))
                              | throw_ kind arguments =>
                                  simp [namespace_eq, expression_eq, kind_eq, initializer_eq,
                                    control_eq] at h
                                  cases h
                                  simpa [control_eq] using
                                    (BigStep.EvalExpr.letValueControl
                                      (control := .throw_ kind arguments)
                                      (namespace_eq := namespace_eq)
                                      (expression_eq := expression_eq) (kind_eq := kind_eq)
                                      (initializer_step := by
                                        simpa [control_eq] using initializer_sound)
                                      (abrupt := BigStep.Abrupt.throw_ kind arguments))
                  | ifElse condition thenBranch elseBranch =>
                      cases condition_eq : Internal.evalExpr fuel executable namespaceId frame state
                          condition with
                      | error error =>
                          simp [namespace_eq, expression_eq, kind_eq, condition_eq] at h
                      | ok conditionResult =>
                          have condition_sound := previous.expression executable namespaceId frame state
                            condition conditionResult condition_eq
                          cases control_eq : conditionResult.control.value with
                          | value runtimeValue =>
                              cases runtimeValue with
                              | bool boolean =>
                                  cases boolean with
                                  | false =>
                                      cases elseBranch with
                                      | none =>
                                          simp [namespace_eq, expression_eq, kind_eq, condition_eq,
                                            control_eq] at h
                                          cases h
                                          exact BigStep.EvalExpr.ifFalseUnit
                                            (namespace_eq := namespace_eq)
                                            (expression_eq := expression_eq) (kind_eq := kind_eq)
                                            (condition_step := by
                                              simpa [control_eq] using condition_sound)
                                      | some branch =>
                                          cases branch_eq : Internal.evalExpr fuel executable namespaceId
                                              conditionResult.frame conditionResult.state branch with
                                          | error error =>
                                              simp [namespace_eq, expression_eq, kind_eq, condition_eq,
                                                control_eq, branch_eq] at h
                                          | ok branchResult =>
                                              have branch_sound := previous.expression executable
                                                namespaceId conditionResult.frame conditionResult.state
                                                branch branchResult branch_eq
                                              cases branch_control_eq : branchResult.control.value <;>
                                                simp [namespace_eq, expression_eq, kind_eq, condition_eq,
                                                  control_eq, branch_eq, branch_control_eq] at h <;>
                                                cases h <;> exact BigStep.EvalExpr.ifFalse
                                                  (namespace_eq := namespace_eq)
                                                  (expression_eq := expression_eq)
                                                  (kind_eq := kind_eq)
                                                  (condition_step := by
                                                    simpa [control_eq] using condition_sound)
                                                  (branch_step := by
                                                    simpa [branch_control_eq] using branch_sound)
                                  | true =>
                                      cases branch_eq : Internal.evalExpr fuel executable namespaceId
                                          conditionResult.frame conditionResult.state thenBranch with
                                      | error error =>
                                          simp [namespace_eq, expression_eq, kind_eq, condition_eq,
                                            control_eq, branch_eq] at h
                                      | ok branchResult =>
                                          have branch_sound := previous.expression executable namespaceId
                                            conditionResult.frame conditionResult.state thenBranch
                                            branchResult branch_eq
                                          cases branch_control_eq : branchResult.control.value <;>
                                            simp [namespace_eq, expression_eq, kind_eq, condition_eq,
                                              control_eq, branch_eq, branch_control_eq] at h <;>
                                            cases h <;> exact BigStep.EvalExpr.ifTrue
                                              (namespace_eq := namespace_eq)
                                              (expression_eq := expression_eq) (kind_eq := kind_eq)
                                              (condition_step := by
                                                simpa [control_eq] using condition_sound)
                                              (branch_step := by
                                                simpa [branch_control_eq] using branch_sound)
                              | unit =>
                                  simp [namespace_eq, expression_eq, kind_eq, condition_eq,
                                    control_eq, failAt] at h
                              | integer _ =>
                                  simp [namespace_eq, expression_eq, kind_eq, condition_eq,
                                    control_eq, failAt] at h
                              | address _ =>
                                  simp [namespace_eq, expression_eq, kind_eq, condition_eq,
                                    control_eq, failAt] at h
                              | character _ =>
                                  simp [namespace_eq, expression_eq, kind_eq, condition_eq,
                                    control_eq, failAt] at h
                              | signer _ =>
                                  simp [namespace_eq, expression_eq, kind_eq, condition_eq,
                                    control_eq, failAt] at h
                              | string _ =>
                                  simp [namespace_eq, expression_eq, kind_eq, condition_eq,
                                    control_eq, failAt] at h
                              | bytes _ =>
                                  simp [namespace_eq, expression_eq, kind_eq, condition_eq,
                                    control_eq, failAt] at h
                              | vector _ =>
                                  simp [namespace_eq, expression_eq, kind_eq, condition_eq,
                                    control_eq, failAt] at h
                              | tuple _ =>
                                  simp [namespace_eq, expression_eq, kind_eq, condition_eq,
                                    control_eq, failAt] at h
                              | nominal _ _ _ =>
                                  simp [namespace_eq, expression_eq, kind_eq, condition_eq,
                                    control_eq, failAt] at h
                              | closure _ _ =>
                                  simp [namespace_eq, expression_eq, kind_eq, condition_eq,
                                    control_eq, failAt] at h
                              | borrow _ _ =>
                                  simp [namespace_eq, expression_eq, kind_eq, condition_eq,
                                    control_eq, failAt] at h
                              | loanHole _ =>
                                  simp [namespace_eq, expression_eq, kind_eq, condition_eq,
                                    control_eq, failAt] at h
                          | break_ nest value =>
                              simp [namespace_eq, expression_eq, kind_eq, condition_eq, control_eq] at h
                              cases h
                              simpa [control_eq] using
                                (BigStep.EvalExpr.ifControl (control := .break_ nest value)
                                  (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                  (kind_eq := kind_eq)
                                  (condition_step := by simpa [control_eq] using condition_sound)
                                  (abrupt := BigStep.Abrupt.break_ nest value))
                          | continue_ nest =>
                              simp [namespace_eq, expression_eq, kind_eq, condition_eq, control_eq] at h
                              cases h
                              simpa [control_eq] using
                                (BigStep.EvalExpr.ifControl (control := .continue_ nest)
                                  (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                  (kind_eq := kind_eq)
                                  (condition_step := by simpa [control_eq] using condition_sound)
                                  (abrupt := BigStep.Abrupt.continue_ nest))
                          | return_ values =>
                              simp [namespace_eq, expression_eq, kind_eq, condition_eq, control_eq] at h
                              cases h
                              simpa [control_eq] using
                                (BigStep.EvalExpr.ifControl (control := .return_ values)
                                  (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                  (kind_eq := kind_eq)
                                  (condition_step := by simpa [control_eq] using condition_sound)
                                  (abrupt := BigStep.Abrupt.return_ values))
                          | throw_ kind arguments =>
                              simp [namespace_eq, expression_eq, kind_eq, condition_eq, control_eq] at h
                              cases h
                              simpa [control_eq] using
                                (BigStep.EvalExpr.ifControl (control := .throw_ kind arguments)
                                  (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                  (kind_eq := kind_eq)
                                  (condition_step := by simpa [control_eq] using condition_sound)
                                  (abrupt := BigStep.Abrupt.throw_ kind arguments))
                  | match_ scrutinee arms =>
                      cases scrutinee_eq : Internal.evalExpr fuel executable namespaceId frame state
                          scrutinee with
                      | error error =>
                          simp [namespace_eq, expression_eq, kind_eq, scrutinee_eq] at h
                      | ok scrutineeResult =>
                          have scrutinee_sound := previous.expression executable namespaceId frame state
                            scrutinee scrutineeResult scrutinee_eq
                          cases control_eq : scrutineeResult.control.value with
                          | value runtimeValue =>
                              cases arms_eq : Internal.evalArms fuel executable namespaceId ns
                                  expression.loc scrutineeResult.frame scrutineeResult.state
                                  runtimeValue arms.toList with
                              | error error =>
                                  simp [namespace_eq, expression_eq, kind_eq, scrutinee_eq,
                                    control_eq, arms_eq] at h
                              | ok armResult =>
                                  have arms_sound := previous.arms executable namespaceId ns
                                    expression.loc scrutineeResult.frame scrutineeResult.state
                                    runtimeValue arms.toList armResult arms_eq
                                  cases arm_control_eq : armResult.control.value <;>
                                    simp [namespace_eq, expression_eq, kind_eq, scrutinee_eq,
                                      control_eq, arms_eq, arm_control_eq] at h <;> cases h <;>
                                    exact BigStep.EvalExpr.matchValue
                                      (namespace_eq := namespace_eq)
                                      (expression_eq := expression_eq) (kind_eq := kind_eq)
                                      (scrutinee_step := by
                                        simpa [control_eq] using scrutinee_sound)
                                      (arm_step := by simpa [arm_control_eq] using arms_sound)
                          | break_ nest value =>
                              simp [namespace_eq, expression_eq, kind_eq, scrutinee_eq, control_eq] at h
                              cases h
                              simpa [control_eq] using
                                (BigStep.EvalExpr.matchControl (control := .break_ nest value)
                                  (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                  (kind_eq := kind_eq)
                                  (scrutinee_step := by simpa [control_eq] using scrutinee_sound)
                                  (abrupt := BigStep.Abrupt.break_ nest value))
                          | continue_ nest =>
                              simp [namespace_eq, expression_eq, kind_eq, scrutinee_eq, control_eq] at h
                              cases h
                              simpa [control_eq] using
                                (BigStep.EvalExpr.matchControl (control := .continue_ nest)
                                  (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                  (kind_eq := kind_eq)
                                  (scrutinee_step := by simpa [control_eq] using scrutinee_sound)
                                  (abrupt := BigStep.Abrupt.continue_ nest))
                          | return_ values =>
                              simp [namespace_eq, expression_eq, kind_eq, scrutinee_eq, control_eq] at h
                              cases h
                              simpa [control_eq] using
                                (BigStep.EvalExpr.matchControl (control := .return_ values)
                                  (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                  (kind_eq := kind_eq)
                                  (scrutinee_step := by simpa [control_eq] using scrutinee_sound)
                                  (abrupt := BigStep.Abrupt.return_ values))
                          | throw_ kind arguments =>
                              simp [namespace_eq, expression_eq, kind_eq, scrutinee_eq, control_eq] at h
                              cases h
                              simpa [control_eq] using
                                (BigStep.EvalExpr.matchControl (control := .throw_ kind arguments)
                                  (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                  (kind_eq := kind_eq)
                                  (scrutinee_step := by simpa [control_eq] using scrutinee_sound)
                                  (abrupt := BigStep.Abrupt.throw_ kind arguments))
                  | loop label body =>
                      cases body_eq : Internal.evalExpr fuel executable namespaceId frame state body with
                      | error error =>
                          simp [namespace_eq, expression_eq, kind_eq, body_eq] at h
                      | ok bodyResult =>
                          have body_sound := previous.expression executable namespaceId frame state body
                            bodyResult body_eq
                          cases control_eq : bodyResult.control.value with
                          | value runtimeValue =>
                              cases repeat_eq : Internal.evalExpr fuel executable namespaceId
                                  bodyResult.frame bodyResult.state exprId with
                              | error error =>
                                  simp [namespace_eq, expression_eq, kind_eq, body_eq, control_eq,
                                    repeat_eq] at h
                              | ok repeated =>
                                  have repeat_sound := previous.expression executable namespaceId
                                    bodyResult.frame bodyResult.state exprId repeated repeat_eq
                                  simp [namespace_eq, expression_eq, kind_eq, body_eq, control_eq,
                                    repeat_eq] at h
                                  cases h
                                  exact BigStep.EvalExpr.loopRepeatValue
                                    (namespace_eq := namespace_eq)
                                    (expression_eq := expression_eq) (kind_eq := kind_eq)
                                    (body_step := by simpa [control_eq] using body_sound)
                                    (repeat_step := repeat_sound)
                          | continue_ nest =>
                              cases nest with
                              | zero =>
                                  cases repeat_eq : Internal.evalExpr fuel executable namespaceId
                                      bodyResult.frame bodyResult.state exprId with
                                  | error error =>
                                      simp [namespace_eq, expression_eq, kind_eq, body_eq, control_eq,
                                        repeat_eq] at h
                                  | ok repeated =>
                                      have repeat_sound := previous.expression executable namespaceId
                                        bodyResult.frame bodyResult.state exprId repeated repeat_eq
                                      simp [namespace_eq, expression_eq, kind_eq, body_eq, control_eq,
                                        repeat_eq] at h
                                      cases h
                                      exact BigStep.EvalExpr.loopRepeatContinue
                                        (namespace_eq := namespace_eq)
                                        (expression_eq := expression_eq) (kind_eq := kind_eq)
                                        (body_step := by simpa [control_eq] using body_sound)
                                        (repeat_step := repeat_sound)
                              | succ nest =>
                                  simp [namespace_eq, expression_eq, kind_eq, body_eq, control_eq] at h
                                  cases h
                                  exact BigStep.EvalExpr.loopOuterContinue
                                    (namespace_eq := namespace_eq)
                                    (expression_eq := expression_eq) (kind_eq := kind_eq)
                                    (body_step := by simpa [control_eq] using body_sound)
                          | break_ nest value =>
                              cases nest with
                              | zero =>
                                  simp [namespace_eq, expression_eq, kind_eq, body_eq, control_eq] at h
                                  cases h
                                  exact BigStep.EvalExpr.loopBreak
                                    (namespace_eq := namespace_eq)
                                    (expression_eq := expression_eq) (kind_eq := kind_eq)
                                    (body_step := by simpa [control_eq] using body_sound)
                              | succ nest =>
                                  simp [namespace_eq, expression_eq, kind_eq, body_eq, control_eq] at h
                                  cases h
                                  exact BigStep.EvalExpr.loopOuterBreak
                                    (namespace_eq := namespace_eq)
                                    (expression_eq := expression_eq) (kind_eq := kind_eq)
                                    (body_step := by simpa [control_eq] using body_sound)
                          | return_ values =>
                              simp [namespace_eq, expression_eq, kind_eq, body_eq, control_eq] at h
                              cases h
                              simpa [control_eq] using
                                (BigStep.EvalExpr.loopReturn
                                  (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                  (kind_eq := kind_eq)
                                  (body_step := by simpa [control_eq] using body_sound))
                          | throw_ kind arguments =>
                              simp [namespace_eq, expression_eq, kind_eq, body_eq, control_eq] at h
                              cases h
                              simpa [control_eq] using
                                (BigStep.EvalExpr.loopThrow
                                  (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                  (kind_eq := kind_eq)
                                  (body_step := by simpa [control_eq] using body_sound))
                  | break_ nest child =>
                      cases child with
                      | none =>
                          simp [namespace_eq, expression_eq, kind_eq] at h
                          cases h
                          exact BigStep.EvalExpr.breakNone
                            (frame := frame) (state := state)
                            (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                            (kind_eq := kind_eq)
                      | some child =>
                          cases child_eq : Internal.evalExpr fuel executable namespaceId frame state child with
                          | error error =>
                              simp [namespace_eq, expression_eq, kind_eq, child_eq] at h
                          | ok childResult =>
                              have child_sound := previous.expression executable namespaceId frame state
                                child childResult child_eq
                              cases control_eq : childResult.control.value with
                              | value runtimeValue =>
                                  simp [namespace_eq, expression_eq, kind_eq, child_eq, control_eq] at h
                                  cases h
                                  exact BigStep.EvalExpr.breakValue
                                    (namespace_eq := namespace_eq)
                                    (expression_eq := expression_eq) (kind_eq := kind_eq)
                                    (child_step := by simpa [control_eq] using child_sound)
                              | break_ innerNest value =>
                                  simp [namespace_eq, expression_eq, kind_eq, child_eq, control_eq] at h
                                  cases h
                                  simpa [control_eq] using
                                    (BigStep.EvalExpr.breakControl
                                      (control := .break_ innerNest value)
                                      (namespace_eq := namespace_eq)
                                      (expression_eq := expression_eq) (kind_eq := kind_eq)
                                      (child_step := by simpa [control_eq] using child_sound)
                                      (abrupt := BigStep.Abrupt.break_ innerNest value))
                              | continue_ innerNest =>
                                  simp [namespace_eq, expression_eq, kind_eq, child_eq, control_eq] at h
                                  cases h
                                  simpa [control_eq] using
                                    (BigStep.EvalExpr.breakControl
                                      (control := .continue_ innerNest)
                                      (namespace_eq := namespace_eq)
                                      (expression_eq := expression_eq) (kind_eq := kind_eq)
                                      (child_step := by simpa [control_eq] using child_sound)
                                      (abrupt := BigStep.Abrupt.continue_ innerNest))
                              | return_ values =>
                                  simp [namespace_eq, expression_eq, kind_eq, child_eq, control_eq] at h
                                  cases h
                                  simpa [control_eq] using
                                    (BigStep.EvalExpr.breakControl (control := .return_ values)
                                      (namespace_eq := namespace_eq)
                                      (expression_eq := expression_eq) (kind_eq := kind_eq)
                                      (child_step := by simpa [control_eq] using child_sound)
                                      (abrupt := BigStep.Abrupt.return_ values))
                              | throw_ kind arguments =>
                                  simp [namespace_eq, expression_eq, kind_eq, child_eq, control_eq] at h
                                  cases h
                                  simpa [control_eq] using
                                    (BigStep.EvalExpr.breakControl
                                      (control := .throw_ kind arguments)
                                      (namespace_eq := namespace_eq)
                                      (expression_eq := expression_eq) (kind_eq := kind_eq)
                                      (child_step := by simpa [control_eq] using child_sound)
                                      (abrupt := BigStep.Abrupt.throw_ kind arguments))
                  | continue_ nest =>
                      simp [namespace_eq, expression_eq, kind_eq] at h
                      cases h
                      exact BigStep.EvalExpr.continue_
                        (frame := frame) (state := state)
                        (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                        (kind_eq := kind_eq)
                  | return_ values =>
                      cases values_eq : Internal.evalValues fuel executable namespaceId frame state
                          values.toList with
                      | error error =>
                          simp [namespace_eq, expression_eq, kind_eq, values_eq] at h
                      | ok evaluated =>
                          have values_sound := previous.values executable namespaceId frame state
                            values.toList evaluated values_eq
                          cases evaluated with
                          | values finalState finalFrame runtimeValues =>
                              simp [namespace_eq, expression_eq, kind_eq, values_eq] at h
                              cases h
                              exact BigStep.EvalExpr.returnValues
                                (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                (kind_eq := kind_eq) (values_step := values_sound)
                          | control finalState finalFrame control =>
                              simp [namespace_eq, expression_eq, kind_eq, values_eq] at h
                              cases h
                              exact BigStep.EvalExpr.returnControl
                                (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                (kind_eq := kind_eq) (values_step := values_sound)
                  | throw_ kind arguments =>
                      cases values_eq : Internal.evalValues fuel executable namespaceId frame state
                          arguments.toList with
                      | error error =>
                          simp [namespace_eq, expression_eq, kind_eq, values_eq] at h
                      | ok evaluated =>
                          have values_sound := previous.values executable namespaceId frame state
                            arguments.toList evaluated values_eq
                          cases evaluated with
                          | values finalState finalFrame runtimeValues =>
                              simp [namespace_eq, expression_eq, kind_eq, values_eq] at h
                              cases h
                              exact BigStep.EvalExpr.throwValues
                                (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                (kind_eq := kind_eq) (values_step := values_sound)
                          | control finalState finalFrame control =>
                              simp [namespace_eq, expression_eq, kind_eq, values_eq] at h
                              cases h
                              exact BigStep.EvalExpr.throwControl
                                (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                (kind_eq := kind_eq) (values_step := values_sound)
                  | assign place child =>
                      cases child_eq : Internal.evalExpr fuel executable namespaceId frame state child with
                      | error error =>
                          simp [namespace_eq, expression_eq, kind_eq, child_eq] at h
                      | ok childResult =>
                          have child_sound := previous.expression executable namespaceId frame state
                            child childResult child_eq
                          cases control_eq : childResult.control.value with
                          | value runtimeValue =>
                              cases resolve_eq : resolvePlace? executable.unit ns childResult.frame
                                  childResult.state place with
                              | none =>
                                  simp [namespace_eq, expression_eq, kind_eq, child_eq, control_eq,
                                    resolve_eq, failAt] at h
                              | some resolved =>
                                  cases write_eq : writeRuntimePlace? childResult.frame childResult.state
                                      resolved runtimeValue with
                                  | none =>
                                      simp [namespace_eq, expression_eq, kind_eq, child_eq, control_eq,
                                        resolve_eq, write_eq, failAt] at h
                                  | some final =>
                                      rcases final with ⟨finalFrame, finalState⟩
                                      simp [namespace_eq, expression_eq, kind_eq, child_eq, control_eq,
                                        resolve_eq, write_eq] at h
                                      cases h
                                      exact BigStep.EvalExpr.assignValue
                                        (namespace_eq := namespace_eq)
                                        (expression_eq := expression_eq) (kind_eq := kind_eq)
                                        (child_step := by simpa [control_eq] using child_sound)
                                        (resolve_eq := resolve_eq) (write_eq := write_eq)
                          | break_ nest value =>
                              simp [namespace_eq, expression_eq, kind_eq, child_eq, control_eq] at h
                              cases h
                              simpa [control_eq] using
                                (BigStep.EvalExpr.assignControl (control := .break_ nest value)
                                  (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                  (kind_eq := kind_eq)
                                  (child_step := by simpa [control_eq] using child_sound)
                                  (abrupt := BigStep.Abrupt.break_ nest value))
                          | continue_ nest =>
                              simp [namespace_eq, expression_eq, kind_eq, child_eq, control_eq] at h
                              cases h
                              simpa [control_eq] using
                                (BigStep.EvalExpr.assignControl (control := .continue_ nest)
                                  (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                  (kind_eq := kind_eq)
                                  (child_step := by simpa [control_eq] using child_sound)
                                  (abrupt := BigStep.Abrupt.continue_ nest))
                          | return_ values =>
                              simp [namespace_eq, expression_eq, kind_eq, child_eq, control_eq] at h
                              cases h
                              simpa [control_eq] using
                                (BigStep.EvalExpr.assignControl (control := .return_ values)
                                  (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                  (kind_eq := kind_eq)
                                  (child_step := by simpa [control_eq] using child_sound)
                                  (abrupt := BigStep.Abrupt.return_ values))
                          | throw_ kind arguments =>
                              simp [namespace_eq, expression_eq, kind_eq, child_eq, control_eq] at h
                              cases h
                              simpa [control_eq] using
                                (BigStep.EvalExpr.assignControl
                                  (control := .throw_ kind arguments)
                                  (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                  (kind_eq := kind_eq)
                                  (child_step := by simpa [control_eq] using child_sound)
                                  (abrupt := BigStep.Abrupt.throw_ kind arguments))
                  | assignPattern pattern child =>
                      cases child_eq : Internal.evalExpr fuel executable namespaceId frame state child with
                      | error error =>
                          simp [namespace_eq, expression_eq, kind_eq, child_eq] at h
                      | ok childResult =>
                          have child_sound := previous.expression executable namespaceId frame state
                            child childResult child_eq
                          cases control_eq : childResult.control.value with
                          | value runtimeValue =>
                              cases bind_eq : bindPattern executable.unit ns childResult.frame pattern runtimeValue with
                              | none =>
                                  simp [namespace_eq, expression_eq, kind_eq, child_eq, control_eq,
                                    bind_eq, failAt] at h
                              | some finalFrame =>
                                  simp [namespace_eq, expression_eq, kind_eq, child_eq, control_eq,
                                    bind_eq] at h
                                  cases h
                                  exact BigStep.EvalExpr.assignPatternValue
                                    (namespace_eq := namespace_eq)
                                    (expression_eq := expression_eq) (kind_eq := kind_eq)
                                    (child_step := by simpa [control_eq] using child_sound)
                                    (bind_eq := bind_eq)
                          | break_ nest value =>
                              simp [namespace_eq, expression_eq, kind_eq, child_eq, control_eq] at h
                              cases h
                              simpa [control_eq] using
                                (BigStep.EvalExpr.assignPatternControl
                                  (control := .break_ nest value)
                                  (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                  (kind_eq := kind_eq)
                                  (child_step := by simpa [control_eq] using child_sound)
                                  (abrupt := BigStep.Abrupt.break_ nest value))
                          | continue_ nest =>
                              simp [namespace_eq, expression_eq, kind_eq, child_eq, control_eq] at h
                              cases h
                              simpa [control_eq] using
                                (BigStep.EvalExpr.assignPatternControl
                                  (control := .continue_ nest)
                                  (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                  (kind_eq := kind_eq)
                                  (child_step := by simpa [control_eq] using child_sound)
                                  (abrupt := BigStep.Abrupt.continue_ nest))
                          | return_ values =>
                              simp [namespace_eq, expression_eq, kind_eq, child_eq, control_eq] at h
                              cases h
                              simpa [control_eq] using
                                (BigStep.EvalExpr.assignPatternControl
                                  (control := .return_ values)
                                  (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                  (kind_eq := kind_eq)
                                  (child_step := by simpa [control_eq] using child_sound)
                                  (abrupt := BigStep.Abrupt.return_ values))
                          | throw_ kind arguments =>
                              simp [namespace_eq, expression_eq, kind_eq, child_eq, control_eq] at h
                              cases h
                              simpa [control_eq] using
                                (BigStep.EvalExpr.assignPatternControl
                                  (control := .throw_ kind arguments)
                                  (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                                  (kind_eq := kind_eq)
                                  (child_step := by simpa [control_eq] using child_sound)
                                  (abrupt := BigStep.Abrupt.throw_ kind arguments))
                  | quantifier kind binders triggers condition body =>
                      simp [namespace_eq, expression_eq, kind_eq, failAt] at h
                  | spec block =>
                      simp [namespace_eq, expression_eq, kind_eq] at h
                      cases h
                      exact BigStep.EvalExpr.spec
                        (frame := frame) (state := state)
                        (namespace_eq := namespace_eq) (expression_eq := expression_eq)
                        (kind_eq := kind_eq)
        values := by
          intro executable namespaceId frame state expressions result h
          cases expressions with
          | nil =>
              change Except.ok (.values state frame []) = .ok result at h
              cases h
              exact .nil _ _ _
          | cons expression expressions =>
              simp only [Internal.evalValues] at h
              cases head_eq : Internal.evalExpr fuel executable namespaceId frame state expression with
              | error error => simp [head_eq, bind, Except.bind] at h
              | ok head =>
                  have head_sound := previous.expression executable namespaceId frame state
                    expression head head_eq
                  cases control_eq : head.control.value with
                  | value value =>
                      cases tail_eq : Internal.evalValues fuel executable namespaceId head.frame
                          head.state expressions with
                      | error error => simp [head_eq, control_eq, tail_eq] at h
                      | ok tail =>
                          have tail_sound := previous.values executable namespaceId head.frame
                            head.state expressions tail tail_eq
                          cases tail with
                          | values finalState finalFrame values =>
                              simp [head_eq, control_eq, tail_eq] at h
                              cases h
                              exact BigStep.EvalValues.tailValues
                                (head_step := by simpa [control_eq] using head_sound)
                                (tail_step := tail_sound)
                          | control finalState finalFrame control =>
                              simp [head_eq, control_eq, tail_eq] at h
                              cases h
                              exact BigStep.EvalValues.tailControl
                                (head_step := by simpa [control_eq] using head_sound)
                                (tail_step := tail_sound)
                  | break_ nest value =>
                      simp [head_eq, control_eq] at h
                      cases h
                      simpa [valuesResult, control_eq] using
                        (BigStep.EvalValues.headControl
                          (expressions := expressions)
                          (head_step := by simpa [control_eq] using head_sound)
                          (abrupt := BigStep.Abrupt.break_ nest value))
                  | continue_ nest =>
                      simp [head_eq, control_eq] at h
                      cases h
                      simpa [valuesResult, control_eq] using
                        (BigStep.EvalValues.headControl
                          (expressions := expressions)
                          (head_step := by simpa [control_eq] using head_sound)
                          (abrupt := BigStep.Abrupt.continue_ nest))
                  | return_ values =>
                      simp [head_eq, control_eq] at h
                      cases h
                      simpa [valuesResult, control_eq] using
                        (BigStep.EvalValues.headControl
                          (expressions := expressions)
                          (head_step := by simpa [control_eq] using head_sound)
                          (abrupt := BigStep.Abrupt.return_ values))
                  | throw_ kind arguments =>
                      simp [head_eq, control_eq] at h
                      cases h
                      simpa [valuesResult, control_eq] using
                        (BigStep.EvalValues.headControl
                          (expressions := expressions)
                          (head_step := by simpa [control_eq] using head_sound)
                          (abrupt := BigStep.Abrupt.throw_ kind arguments))
        statements := by
          intro executable namespaceId frame state statements result h
          cases statements with
          | nil =>
              change Except.ok (.done state frame) = .ok result at h
              cases h
              exact .nil _ _ _
          | cons statement statements =>
              simp only [Internal.evalStatements] at h
              cases head_eq : Internal.evalExpr fuel executable namespaceId frame state statement with
              | error error => simp [head_eq] at h
              | ok head =>
                  have head_sound := previous.expression executable namespaceId frame state
                    statement head head_eq
                  cases control_eq : head.control.value with
                  | value value =>
                      cases tail_eq : Internal.evalStatements fuel executable namespaceId head.frame
                          head.state statements with
                      | error error => simp [head_eq, control_eq, tail_eq] at h
                      | ok tail =>
                          have tail_sound := previous.statements executable namespaceId head.frame
                            head.state statements tail tail_eq
                          simp [head_eq, control_eq, tail_eq] at h
                          cases h
                          exact BigStep.EvalStatements.cons
                            (head_step := by simpa [control_eq] using head_sound)
                            (tail_step := tail_sound)
                  | break_ nest value =>
                      simp [head_eq, control_eq] at h
                      cases h
                      simpa [statementsResult, control_eq] using
                        (BigStep.EvalStatements.headControl
                          (statements := statements)
                          (head_step := by simpa [control_eq] using head_sound)
                          (abrupt := BigStep.Abrupt.break_ nest value))
                  | continue_ nest =>
                      simp [head_eq, control_eq] at h
                      cases h
                      simpa [statementsResult, control_eq] using
                        (BigStep.EvalStatements.headControl
                          (statements := statements)
                          (head_step := by simpa [control_eq] using head_sound)
                          (abrupt := BigStep.Abrupt.continue_ nest))
                  | return_ values =>
                      simp [head_eq, control_eq] at h
                      cases h
                      simpa [statementsResult, control_eq] using
                        (BigStep.EvalStatements.headControl
                          (statements := statements)
                          (head_step := by simpa [control_eq] using head_sound)
                          (abrupt := BigStep.Abrupt.return_ values))
                  | throw_ kind arguments =>
                      simp [head_eq, control_eq] at h
                      cases h
                      simpa [statementsResult, control_eq] using
                        (BigStep.EvalStatements.headControl
                          (statements := statements)
                          (head_step := by simpa [control_eq] using head_sound)
                          (abrupt := BigStep.Abrupt.throw_ kind arguments))
        arms := by
          intro executable namespaceId ns ownerLoc frame state value arms result h
          cases arms with
          | nil => simp [Internal.evalArms, failAt] at h
          | cons arm arms =>
              simp only [Internal.evalArms] at h
              cases bind_eq : bindPattern executable.unit ns frame arm.pattern value with
              | none =>
                  cases tail_eq : Internal.evalArms fuel executable namespaceId ns ownerLoc
                      frame state value arms with
                  | error error => simp [bind_eq, tail_eq] at h
                  | ok tail =>
                      have tail_sound := previous.arms executable namespaceId ns ownerLoc frame
                        state value arms tail tail_eq
                      simp [bind_eq, tail_eq] at h
                      cases h
                      exact BigStep.EvalArms.reject (bind_eq := bind_eq) (tail_step := tail_sound)
              | some armFrame =>
                  cases guard_eq : arm.guard with
                  | none =>
                      cases body_eq : Internal.evalExpr fuel executable namespaceId armFrame
                          state arm.body with
                      | error error => simp [bind_eq, guard_eq, body_eq] at h
                      | ok body =>
                          have body_sound := previous.expression executable namespaceId armFrame
                            state arm.body body body_eq
                          simp [bind_eq, guard_eq, body_eq] at h
                          cases h
                          exact BigStep.EvalArms.noGuard (guard_eq := guard_eq)
                            (arms := arms)
                            (bind_eq := bind_eq) (body_step := body_sound)
                  | some guard =>
                      cases guard_step_eq : Internal.evalExpr fuel executable namespaceId armFrame
                          state guard with
                      | error error => simp [bind_eq, guard_eq, guard_step_eq] at h
                      | ok guardResult =>
                          have guard_sound := previous.expression executable namespaceId armFrame
                            state guard guardResult guard_step_eq
                          cases control_eq : guardResult.control.value with
                          | value runtimeValue =>
                              cases runtimeValue with
                              | unit => simp [bind_eq, guard_eq, guard_step_eq, control_eq, failAt] at h
                              | integer integer =>
                                  simp [bind_eq, guard_eq, guard_step_eq, control_eq, failAt] at h
                              | character _ =>
                                  simp [bind_eq, guard_eq, guard_step_eq, control_eq, failAt] at h
                              | address _ =>
                                  simp [bind_eq, guard_eq, guard_step_eq, control_eq, failAt] at h
                              | signer _ =>
                                  simp [bind_eq, guard_eq, guard_step_eq, control_eq, failAt] at h
                              | tuple values =>
                                  simp [bind_eq, guard_eq, guard_step_eq, control_eq, failAt] at h
                              | string _ =>
                                  simp [bind_eq, guard_eq, guard_step_eq, control_eq, failAt] at h
                              | bytes _ =>
                                  simp [bind_eq, guard_eq, guard_step_eq, control_eq, failAt] at h
                              | vector _ =>
                                  simp [bind_eq, guard_eq, guard_step_eq, control_eq, failAt] at h
                              | nominal _ _ _ =>
                                  simp [bind_eq, guard_eq, guard_step_eq, control_eq, failAt] at h
                              | closure _ _ =>
                                  simp [bind_eq, guard_eq, guard_step_eq, control_eq, failAt] at h
                              | borrow _ _ =>
                                  simp [bind_eq, guard_eq, guard_step_eq, control_eq, failAt] at h
                              | loanHole _ =>
                                  simp [bind_eq, guard_eq, guard_step_eq, control_eq, failAt] at h
                              | bool boolean =>
                                  cases boolean with
                                  | false =>
                                      cases tail_eq : Internal.evalArms fuel executable namespaceId ns
                                          ownerLoc frame state value arms with
                                      | error error =>
                                          simp [bind_eq, guard_eq, guard_step_eq, control_eq, tail_eq] at h
                                      | ok tail =>
                                          have tail_sound := previous.arms executable namespaceId ns
                                            ownerLoc frame state value arms tail tail_eq
                                          simp [bind_eq, guard_eq, guard_step_eq, control_eq, tail_eq] at h
                                          cases h
                                          exact BigStep.EvalArms.guardFalse (guard_eq := guard_eq)
                                            (arms := arms)
                                            (bind_eq := bind_eq)
                                            (guard_step := by simpa [control_eq] using guard_sound)
                                            (tail_step := tail_sound)
                                  | true =>
                                      cases body_eq : Internal.evalExpr fuel executable namespaceId
                                          guardResult.frame guardResult.state arm.body with
                                      | error error =>
                                          simp [bind_eq, guard_eq, guard_step_eq, control_eq, body_eq] at h
                                      | ok body =>
                                          have body_sound := previous.expression executable namespaceId
                                            guardResult.frame guardResult.state arm.body body body_eq
                                          simp [bind_eq, guard_eq, guard_step_eq, control_eq, body_eq] at h
                                          cases h
                                          exact BigStep.EvalArms.guardTrue (guard_eq := guard_eq)
                                            (arms := arms)
                                            (bind_eq := bind_eq)
                                            (guard_step := by simpa [control_eq] using guard_sound)
                                            (body_step := body_sound)
                          | break_ nest value =>
                              simp [bind_eq, guard_eq, guard_step_eq, control_eq] at h
                              cases h
                              simpa [control_eq] using
                                (BigStep.EvalArms.guardControl (guard_eq := guard_eq)
                                  (arms := arms) (control := .break_ nest value)
                                  (bind_eq := bind_eq)
                                  (guard_step := by simpa [control_eq] using guard_sound)
                                  (abrupt := BigStep.Abrupt.break_ nest value))
                          | continue_ nest =>
                              simp [bind_eq, guard_eq, guard_step_eq, control_eq] at h
                              cases h
                              simpa [control_eq] using
                                (BigStep.EvalArms.guardControl (guard_eq := guard_eq)
                                  (arms := arms) (control := .continue_ nest)
                                  (bind_eq := bind_eq)
                                  (guard_step := by simpa [control_eq] using guard_sound)
                                  (abrupt := BigStep.Abrupt.continue_ nest))
                          | return_ values =>
                              simp [bind_eq, guard_eq, guard_step_eq, control_eq] at h
                              cases h
                              simpa [control_eq] using
                                (BigStep.EvalArms.guardControl (guard_eq := guard_eq)
                                  (arms := arms) (control := .return_ values)
                                  (bind_eq := bind_eq)
                                  (guard_step := by simpa [control_eq] using guard_sound)
                                  (abrupt := BigStep.Abrupt.return_ values))
                          | throw_ kind arguments =>
                              simp [bind_eq, guard_eq, guard_step_eq, control_eq] at h
                              cases h
                              simpa [control_eq] using
                                (BigStep.EvalArms.guardControl (guard_eq := guard_eq)
                                  (arms := arms) (control := .throw_ kind arguments)
                                  (bind_eq := bind_eq)
                                  (guard_step := by simpa [control_eq] using guard_sound)
                                  (abrupt := BigStep.Abrupt.throw_ kind arguments)) }

/-- Foundational M1 theorem: every successful fuelled invocation is related
by the authoritative, fuel-free function semantics. -/
theorem run_sound (executable : ExecutableUnit) (fuel : Nat) (function : FunctionHandle)
    (arguments : Array RuntimeValue) (state finalState : RuntimeState)
    (outcome : LocatedOutcome)
    (execution : run executable fuel function arguments state = .ok (finalState, outcome)) :
    BigStep.EvalFunction executable function state arguments finalState outcome.value := by
  simp only [run] at execution
  cases result_eq : Internal.evalFunction fuel executable function state arguments with
  | error error => simp [result_eq] at execution
  | ok result =>
      simp [result_eq] at execution
      cases execution
      exact (soundAt fuel).function executable function state arguments result result_eq

end LeanerIR.Proofs.Interpreter
