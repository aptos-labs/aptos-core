-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.Operations

/-!
# Declarative big-step semantics for structured LIR

These rules are the authoritative, fuel-free meaning of the M1 executable
subset.  They mention syntax, runtime state, and the pure leaf operations,
but never the interpreter.  Nontermination is the absence of a finite
derivation.
-/

namespace LeanerIR
namespace BigStep

open Validation

open SemanticOperations

/-- Control which must propagate through ordinary expression sequencing. -/
inductive Abrupt : Control → Prop where
  | break_ (nest : Nat) (value : Option RuntimeValue) : Abrupt (.break_ nest value)
  | continue_ (nest : Nat) : Abrupt (.continue_ nest)
  | return_ (values : Array RuntimeValue) : Abrupt (.return_ values)
  | throw_ (kind : ThrowKind) (arguments : Array RuntimeValue) :
      Abrupt (.throw_ kind arguments)

/-- Result of evaluating an ordered expression list as operands. -/
inductive ValuesResult where
  | values (state : RuntimeState) (frame : RuntimeFrame) (values : List RuntimeValue)
  | control (state : RuntimeState) (frame : RuntimeFrame) (control : Control)

/-- Result of evaluating an ordered statement list. -/
inductive StatementsResult where
  | done (state : RuntimeState) (frame : RuntimeFrame)
  | control (state : RuntimeState) (frame : RuntimeFrame) (control : Control)

/-- The callee oracle of the open semantics: what a direct or closure call
means before the call graph is tied.  The closed semantics `EvalFunction`
is the least fixed point of the rules over this oracle, which is what makes
a recursive function's native denotation provable by the same induction
the rules themselves admit (`Proofs.Recursion`). -/
abbrev CalleeRelation :=
  FunctionHandle → Array (TypeId × TypeId) → RuntimeState →
    Array RuntimeValue → RuntimeState → Outcome → Prop

mutual
  /-- Big-step evaluation of one structured expression. -/
  inductive EvalExprWith (unit : ExecutableUnit) (callee : CalleeRelation) :
      NamespaceId → RuntimeFrame → RuntimeState → ExprId → RuntimeFrame →
      RuntimeState → Control → Prop where
    | value (namespaceId frame state exprId ns expression literal source runtimeValue)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .value literal source)
        (value_eq : constValue? literal = some runtimeValue) :
        EvalExprWith unit callee namespaceId frame state exprId frame state (.value runtimeValue)
    | constantValue (namespaceId frame state exprId ns expression reference handle
        targetNs declaration targetFrame finalState runtimeValue)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .constant reference)
        (resolve_eq : resolveConstant? unit.unit namespaceId reference = some handle)
        (target_namespace_eq : unit.unit.namespaces[handle.namespaceId.index]? = some targetNs)
        (declaration_eq : targetNs.constants[handle.constantId]? = some declaration)
        (initializer : EvalExprWith unit callee handle.namespaceId { locals := #[] } state
          declaration.value targetFrame finalState (.value runtimeValue)) :
        EvalExprWith unit callee namespaceId frame state exprId
          frame finalState (.value runtimeValue)
    | constantControl (namespaceId frame state exprId ns expression reference handle
        targetNs declaration targetFrame finalState control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .constant reference)
        (resolve_eq : resolveConstant? unit.unit namespaceId reference = some handle)
        (target_namespace_eq : unit.unit.namespaces[handle.namespaceId.index]? = some targetNs)
        (declaration_eq : targetNs.constants[handle.constantId]? = some declaration)
        (initializer : EvalExprWith unit callee handle.namespaceId { locals := #[] } state
          declaration.value targetFrame finalState control)
        (abrupt : Abrupt control) :
        EvalExprWith unit callee namespaceId frame state exprId frame finalState control
    | localVar (namespaceId frame state exprId ns expression localId runtimeValue)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .localVar localId)
        (local_eq : readLocal? frame localId = some runtimeValue) :
        EvalExprWith unit callee namespaceId frame state exprId frame state (.value runtimeValue)
    | callArgumentsControl (namespaceId frame state exprId ns expression reference
        instantiations arguments surface finalState finalFrame control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call (.function reference))
          instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.control finalState finalFrame control)) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | callReturned (namespaceId frame state exprId ns expression reference instantiations
        arguments surface argumentState argumentFrame values handle finalState results)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call (.function reference))
          instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.values argumentState argumentFrame values))
        (resolve_eq : resolveFunction? unit.unit namespaceId reference = some handle)
        (calleeStep : callee handle
          (callTypeInstantiation unit.unit handle argumentFrame.typeInstantiation instantiations)
          argumentState values.toArray finalState
          (.returned results)) :
        EvalExprWith unit callee namespaceId frame state exprId
          (registerReturnedLoan
            (certificateLoanId? unit.unit namespaceId exprId) results
            (applyPendingFrom argumentState.pending argumentFrame finalState).1)
          (applyPendingFrom argumentState.pending argumentFrame finalState).2
          (.value (packResults results))
    | callThrew (namespaceId frame state exprId ns expression reference instantiations
        arguments surface argumentState argumentFrame values handle finalState kind thrown)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call (.function reference))
          instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.values argumentState argumentFrame values))
        (resolve_eq : resolveFunction? unit.unit namespaceId reference = some handle)
        (calleeStep : callee handle
          (callTypeInstantiation unit.unit handle argumentFrame.typeInstantiation instantiations)
          argumentState values.toArray finalState
          (.threw kind thrown)) :
        EvalExprWith unit callee namespaceId frame state exprId
          (applyPendingFrom argumentState.pending argumentFrame finalState).1
          (applyPendingFrom argumentState.pending argumentFrame finalState).2
          (.throw_ kind thrown)
    | constructorArgumentsControl (namespaceId frame state exprId ns expression reference variant
        instantiations arguments surface finalState finalFrame control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call (.constructor reference variant))
          instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.control finalState finalFrame control)) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | constructorValue (namespaceId frame state exprId ns expression reference variant
        instantiations arguments surface finalState finalFrame values runtimeValue)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call (.constructor reference variant))
          instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.values finalState finalFrame values))
        (construct_eq : constructNominal? unit.unit namespaceId reference variant values.toArray =
          some runtimeValue) :
        EvalExprWith unit callee namespaceId frame state exprId
          finalFrame finalState (.value runtimeValue)
    | destructorArgumentsControl (namespaceId frame state exprId ns expression reference variant
        instantiations arguments surface finalState finalFrame control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call (.destructor reference variant))
          instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.control finalState finalFrame control)) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | destructorValue (namespaceId frame state exprId ns expression reference variant
        instantiations arguments surface finalState finalFrame value fields)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call (.destructor reference variant))
          instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.values finalState finalFrame [value]))
        (destruct_eq : destructNominal? unit.unit namespaceId reference variant value = some fields) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState
          (.value (packResults fields))
    | closureArgumentsControl (namespaceId frame state exprId ns expression reference
        instantiations captures surface finalState finalFrame control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call (.closure reference))
          instantiations captures surface)
        (operands : EvalValuesWith unit callee namespaceId frame state captures.toList
          (.control finalState finalFrame control)) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | closureValue (namespaceId frame state exprId ns expression reference instantiations captures
        surface finalState finalFrame values handle)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call (.closure reference))
          instantiations captures surface)
        (operands : EvalValuesWith unit callee namespaceId frame state captures.toList
          (.values finalState finalFrame values))
        (resolve_eq : resolveFunction? unit.unit namespaceId reference = some handle) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState
          (.value (.closure handle values.toArray))
    | invokeArgumentsControl (namespaceId frame state exprId ns expression instantiations arguments
        surface finalState finalFrame control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call .invoke) instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.control finalState finalFrame control)) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | invokeReturned (namespaceId frame state exprId ns expression instantiations arguments surface
        argumentState argumentFrame handle captures values finalState results)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call .invoke) instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.values argumentState argumentFrame (.closure handle captures :: values)))
        (calleeStep : callee handle argumentFrame.typeInstantiation argumentState
          (captures ++ values.toArray) finalState
          (.returned results)) :
        EvalExprWith unit callee namespaceId frame state exprId
          (applyPendingFrom argumentState.pending argumentFrame finalState).1
          (applyPendingFrom argumentState.pending argumentFrame finalState).2
          (.value (packResults results))
    | invokeThrew (namespaceId frame state exprId ns expression instantiations arguments surface
        argumentState argumentFrame handle captures values finalState kind thrown)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call .invoke) instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.values argumentState argumentFrame (.closure handle captures :: values)))
        (calleeStep : callee handle argumentFrame.typeInstantiation argumentState
          (captures ++ values.toArray) finalState
          (.threw kind thrown)) :
        EvalExprWith unit callee namespaceId frame state exprId
          (applyPendingFrom argumentState.pending argumentFrame finalState).1
          (applyPendingFrom argumentState.pending argumentFrame finalState).2
          (.throw_ kind thrown)
    | profileArgumentsControl (namespaceId frame state exprId ns expression operation targets
        instantiations arguments surface finalState finalFrame control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.profile operation targets)
          instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.control finalState finalFrame control)) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | profileValue (namespaceId frame state exprId ns expression operation targets instantiations
        arguments surface finalState finalFrame values runtimeValue)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.profile operation targets)
          instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.values finalState finalFrame values))
        (evaluate_eq : evaluateProfileOperation? unit ns expression.typeId operation values.toArray =
          some (.ok runtimeValue)) :
        EvalExprWith unit callee namespaceId frame state exprId
          finalFrame finalState (.value runtimeValue)
    | profileThrow (namespaceId frame state exprId ns expression operation targets instantiations
        arguments surface finalState finalFrame values kind thrown)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.profile operation targets)
          instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.values finalState finalFrame values))
        (evaluate_eq : evaluateProfileOperation? unit ns expression.typeId operation values.toArray =
          some (.error (kind, thrown))) :
        EvalExprWith unit callee namespaceId frame state exprId
          finalFrame finalState (.throw_ kind thrown)
    | primitiveArgumentsControl (namespaceId frame state exprId ns expression operation
        instantiations arguments surface finalState finalFrame control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.primitive operation)
          instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.control finalState finalFrame control)) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | primitiveValue (namespaceId frame state exprId ns expression operation instantiations
        arguments surface finalState finalFrame values runtimeValue)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.primitive operation)
          instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.values finalState finalFrame values))
        (evaluate_eq : evaluatePrimitiveOperation? ns expression.typeId operation values.toArray
          unit.targetPointerWidth =
          some (.ok runtimeValue)) :
        EvalExprWith unit callee namespaceId frame state exprId
          finalFrame finalState (.value runtimeValue)
    | primitiveThrow (namespaceId frame state exprId ns expression operation instantiations
        arguments surface finalState finalFrame values kind thrown)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.primitive operation)
          instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.values finalState finalFrame values))
        (evaluate_eq : evaluatePrimitiveOperation? ns expression.typeId operation values.toArray
          unit.targetPointerWidth =
          some (.error (kind, thrown))) :
        EvalExprWith unit callee namespaceId frame state exprId
          finalFrame finalState (.throw_ kind thrown)
    | globalArgumentsControl (namespaceId frame state exprId ns expression kind instantiations
        arguments surface finalState finalFrame control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.global kind) instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.control finalState finalFrame control)) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | globalValue (namespaceId frame state exprId ns expression kind instantiations arguments
        surface argumentState argumentFrame values finalFrame finalState runtimeValue)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.global kind) instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.values argumentState argumentFrame values))
        (evaluate_eq : evaluateGlobalOperation? unit.unit ns expression.typeId exprId kind
          instantiations values.toArray argumentFrame argumentState =
            some (.value finalFrame finalState runtimeValue)) :
        EvalExprWith unit callee namespaceId frame state exprId
          finalFrame finalState (.value runtimeValue)
    | globalThrow (namespaceId frame state exprId ns expression globalKind instantiations arguments
        surface argumentState argumentFrame values finalFrame finalState throwKind thrown)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.global globalKind) instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.values argumentState argumentFrame values))
        (evaluate_eq : evaluateGlobalOperation? unit.unit ns expression.typeId exprId globalKind
          instantiations values.toArray argumentFrame argumentState =
            some (.throw_ finalFrame finalState throwKind thrown)) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState
          (.throw_ throwKind thrown)
    | assertArgumentsControl (namespaceId frame state exprId ns expression instantiations arguments
        surface finalState finalFrame control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation .assert instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.control finalState finalFrame control)) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | assertTrue (namespaceId frame state exprId ns expression instantiations arguments surface
        finalState finalFrame)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation .assert instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.values finalState finalFrame [.bool true])) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState (.value .unit)
    | assertFalse (namespaceId frame state exprId ns expression instantiations arguments surface
        finalState finalFrame)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation .assert instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.values finalState finalFrame [.bool false])) :
        EvalExprWith unit callee namespaceId frame state exprId
          finalFrame finalState (.throw_ .abort #[])
    | operationArgumentsControl (namespaceId frame state exprId ns expression operation
        instantiations arguments surface finalState finalFrame control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation operation instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.control finalState finalFrame control)) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | operationValue (namespaceId frame state exprId ns expression operation instantiations
        arguments surface argumentState argumentFrame values finalFrame finalState runtimeValue)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation operation instantiations arguments surface)
        (operands : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.values argumentState argumentFrame values))
        (evaluate_eq : evaluatePlaceOperation? unit.unit ns expression.typeId exprId operation
          values.toArray argumentFrame argumentState = some (finalFrame, finalState, runtimeValue)) :
        EvalExprWith unit callee namespaceId frame state exprId
          finalFrame finalState (.value runtimeValue)
    | blockControl (namespaceId frame state exprId ns expression statements result
        finalState finalFrame control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .block statements result)
        (steps : EvalStatementsWith unit callee namespaceId frame state statements.toList
          (.control finalState finalFrame control)) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | blockUnit (namespaceId frame state exprId ns expression statements finalState finalFrame)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .block statements none)
        (steps : EvalStatementsWith unit callee namespaceId frame state statements.toList
          (.done finalState finalFrame)) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState (.value .unit)
    | blockResult (namespaceId frame state exprId ns expression statements result
        statementState statementFrame finalState finalFrame control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .block statements (some result))
        (steps : EvalStatementsWith unit callee namespaceId frame state statements.toList
          (.done statementState statementFrame))
        (result_step : EvalExprWith unit callee namespaceId statementFrame statementState result
          finalFrame finalState control) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | letNoValue (namespaceId frame state exprId ns expression pattern body
        finalFrame finalState control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .letDecl pattern none body)
        (body_step : EvalExprWith unit callee namespaceId frame state body finalFrame finalState control) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | letValueControl (namespaceId frame state exprId ns expression pattern initializer body
        finalFrame finalState control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .letDecl pattern (some initializer) body)
        (initializer_step : EvalExprWith unit callee namespaceId frame state initializer
          finalFrame finalState control)
        (abrupt : Abrupt control) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | letValue (namespaceId frame state exprId ns expression pattern initializer body
        initializedFrame initializedState runtimeValue boundFrame finalFrame finalState control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .letDecl pattern (some initializer) body)
        (initializer_step : EvalExprWith unit callee namespaceId frame state initializer
          initializedFrame initializedState (.value runtimeValue))
        (bind_eq : bindPattern unit.unit ns initializedFrame pattern runtimeValue = some boundFrame)
        (body_step : EvalExprWith unit callee namespaceId boundFrame initializedState body
          finalFrame finalState control) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | ifControl (namespaceId frame state exprId ns expression condition thenBranch elseBranch
        finalFrame finalState control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .ifElse condition thenBranch elseBranch)
        (condition_step : EvalExprWith unit callee namespaceId frame state condition
          finalFrame finalState control)
        (abrupt : Abrupt control) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | ifTrue (namespaceId frame state exprId ns expression condition thenBranch elseBranch
        conditionFrame conditionState finalFrame finalState control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .ifElse condition thenBranch elseBranch)
        (condition_step : EvalExprWith unit callee namespaceId frame state condition
          conditionFrame conditionState (.value (.bool true)))
        (branch_step : EvalExprWith unit callee namespaceId conditionFrame conditionState thenBranch
          finalFrame finalState control) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | ifFalseUnit (namespaceId frame state exprId ns expression condition thenBranch
        conditionFrame conditionState)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .ifElse condition thenBranch none)
        (condition_step : EvalExprWith unit callee namespaceId frame state condition
          conditionFrame conditionState (.value (.bool false))) :
        EvalExprWith unit callee namespaceId frame state exprId conditionFrame conditionState
          (.value .unit)
    | ifFalse (namespaceId frame state exprId ns expression condition thenBranch elseBranch
        conditionFrame conditionState finalFrame finalState control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .ifElse condition thenBranch (some elseBranch))
        (condition_step : EvalExprWith unit callee namespaceId frame state condition
          conditionFrame conditionState (.value (.bool false)))
        (branch_step : EvalExprWith unit callee namespaceId conditionFrame conditionState elseBranch
          finalFrame finalState control) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | matchControl (namespaceId frame state exprId ns expression scrutinee arms
        finalFrame finalState control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .match_ scrutinee arms)
        (scrutinee_step : EvalExprWith unit callee namespaceId frame state scrutinee
          finalFrame finalState control)
        (abrupt : Abrupt control) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | matchValue (namespaceId frame state exprId ns expression scrutinee arms
        scrutineeFrame scrutineeState runtimeValue finalFrame finalState control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .match_ scrutinee arms)
        (scrutinee_step : EvalExprWith unit callee namespaceId frame state scrutinee
          scrutineeFrame scrutineeState (.value runtimeValue))
        (arm_step : EvalArmsWith unit callee namespaceId ns scrutineeFrame scrutineeState runtimeValue
          arms.toList finalFrame finalState control) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | loopRepeatValue (namespaceId frame state exprId ns expression label body
        bodyFrame bodyState runtimeValue finalFrame finalState control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .loop label body)
        (body_step : EvalExprWith unit callee namespaceId frame state body bodyFrame bodyState
          (.value runtimeValue))
        (repeat_step : EvalExprWith unit callee namespaceId bodyFrame bodyState exprId
          finalFrame finalState control) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | loopRepeatContinue (namespaceId frame state exprId ns expression label body
        bodyFrame bodyState finalFrame finalState control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .loop label body)
        (body_step : EvalExprWith unit callee namespaceId frame state body bodyFrame bodyState
          (.continue_ 0))
        (repeat_step : EvalExprWith unit callee namespaceId bodyFrame bodyState exprId
          finalFrame finalState control) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | loopBreak (namespaceId frame state exprId ns expression label body
        finalFrame finalState value)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .loop label body)
        (body_step : EvalExprWith unit callee namespaceId frame state body finalFrame finalState
          (.break_ 0 value)) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState
          (.value (value.getD .unit))
    | loopOuterBreak (namespaceId frame state exprId ns expression label body
        finalFrame finalState nest value)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .loop label body)
        (body_step : EvalExprWith unit callee namespaceId frame state body finalFrame finalState
          (.break_ (nest + 1) value)) :
        EvalExprWith unit callee namespaceId frame state exprId
          finalFrame finalState (.break_ nest value)
    | loopOuterContinue (namespaceId frame state exprId ns expression label body
        finalFrame finalState nest)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .loop label body)
        (body_step : EvalExprWith unit callee namespaceId frame state body finalFrame finalState
          (.continue_ (nest + 1))) :
        EvalExprWith unit callee namespaceId frame state exprId
          finalFrame finalState (.continue_ nest)
    | loopReturn (namespaceId frame state exprId ns expression label body
        finalFrame finalState values)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .loop label body)
        (body_step : EvalExprWith unit callee namespaceId frame state body finalFrame finalState
          (.return_ values)) :
        EvalExprWith unit callee namespaceId frame state exprId
          finalFrame finalState (.return_ values)
    | loopThrow (namespaceId frame state exprId ns expression label body
        finalFrame finalState kind arguments)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .loop label body)
        (body_step : EvalExprWith unit callee namespaceId frame state body finalFrame finalState
          (.throw_ kind arguments)) :
        EvalExprWith unit callee namespaceId frame state exprId
          finalFrame finalState (.throw_ kind arguments)
    | breakNone (namespaceId frame state exprId ns expression nest)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .break_ nest none) :
        EvalExprWith unit callee namespaceId frame state exprId frame state (.break_ nest none)
    | breakControl (namespaceId frame state exprId ns expression nest child
        finalFrame finalState control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .break_ nest (some child))
        (child_step : EvalExprWith unit callee namespaceId frame state child finalFrame finalState control)
        (abrupt : Abrupt control) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | breakValue (namespaceId frame state exprId ns expression nest child
        finalFrame finalState runtimeValue)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .break_ nest (some child))
        (child_step : EvalExprWith unit callee namespaceId frame state child finalFrame finalState
          (.value runtimeValue)) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState
          (.break_ nest (some runtimeValue))
    | continue_ (namespaceId frame state exprId ns expression nest)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .continue_ nest) :
        EvalExprWith unit callee namespaceId frame state exprId frame state (.continue_ nest)
    | returnValues (namespaceId frame state exprId ns expression values
        finalFrame finalState runtimeValues)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .return_ values)
        (values_step : EvalValuesWith unit callee namespaceId frame state values.toList
          (.values finalState finalFrame runtimeValues)) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState
          (.return_ runtimeValues.toArray)
    | returnControl (namespaceId frame state exprId ns expression values
        finalFrame finalState control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .return_ values)
        (values_step : EvalValuesWith unit callee namespaceId frame state values.toList
          (.control finalState finalFrame control)) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | throwValues (namespaceId frame state exprId ns expression kind arguments
        finalFrame finalState runtimeValues)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .throw_ kind arguments)
        (values_step : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.values finalState finalFrame runtimeValues)) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState
          (.throw_ kind runtimeValues.toArray)
    | throwControl (namespaceId frame state exprId ns expression kind arguments
        finalFrame finalState control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .throw_ kind arguments)
        (values_step : EvalValuesWith unit callee namespaceId frame state arguments.toList
          (.control finalState finalFrame control)) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | assignControl (namespaceId frame state exprId ns expression place child
        finalFrame finalState control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .assign place child)
        (child_step : EvalExprWith unit callee namespaceId frame state child finalFrame finalState control)
        (abrupt : Abrupt control) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | assignValue (namespaceId frame state exprId ns expression place child
        childFrame childState resolved finalFrame finalState runtimeValue)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .assign place child)
        (child_step : EvalExprWith unit callee namespaceId frame state child childFrame childState
          (.value runtimeValue))
        (resolve_eq : resolvePlace? unit.unit ns childFrame childState place = some resolved)
        (write_eq : writeRuntimePlace? childFrame childState resolved runtimeValue =
          some (finalFrame, finalState)) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState (.value .unit)
    | assignPatternControl (namespaceId frame state exprId ns expression pattern child
        finalFrame finalState control)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .assignPattern pattern child)
        (child_step : EvalExprWith unit callee namespaceId frame state child finalFrame finalState control)
        (abrupt : Abrupt control) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState control
    | assignPatternValue (namespaceId frame state exprId ns expression pattern child
        childFrame finalFrame finalState runtimeValue)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .assignPattern pattern child)
        (child_step : EvalExprWith unit callee namespaceId frame state child childFrame finalState
          (.value runtimeValue))
        (bind_eq : bindPattern unit.unit ns childFrame pattern runtimeValue = some finalFrame) :
        EvalExprWith unit callee namespaceId frame state exprId finalFrame finalState (.value .unit)
    | spec (namespaceId frame state exprId ns expression block)
        (namespace_eq : unit.unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .spec block) :
        EvalExprWith unit callee namespaceId frame state exprId frame state (.value .unit)

  /-- Left-to-right evaluation of expression operands. -/
  inductive EvalValuesWith (unit : ExecutableUnit) (callee : CalleeRelation) :
      NamespaceId → RuntimeFrame → RuntimeState → List ExprId → ValuesResult → Prop where
    | nil (namespaceId frame state) :
        EvalValuesWith unit callee namespaceId frame state [] (.values state frame [])
    | headControl (namespaceId frame state expression expressions finalFrame finalState control)
        (head_step : EvalExprWith unit callee namespaceId frame state expression
          finalFrame finalState control)
        (abrupt : Abrupt control) :
        EvalValuesWith unit callee namespaceId frame state (expression :: expressions)
          (.control finalState finalFrame control)
    | tailValues (namespaceId frame state expression expressions headFrame headState value
        finalFrame finalState values)
        (head_step : EvalExprWith unit callee namespaceId frame state expression
          headFrame headState (.value value))
        (tail_step : EvalValuesWith unit callee namespaceId headFrame headState expressions
          (.values finalState finalFrame values)) :
        EvalValuesWith unit callee namespaceId frame state (expression :: expressions)
          (.values finalState finalFrame (value :: values))
    | tailControl (namespaceId frame state expression expressions headFrame headState value
        finalFrame finalState control)
        (head_step : EvalExprWith unit callee namespaceId frame state expression
          headFrame headState (.value value))
        (tail_step : EvalValuesWith unit callee namespaceId headFrame headState expressions
          (.control finalState finalFrame control)) :
        EvalValuesWith unit callee namespaceId frame state (expression :: expressions)
          (.control finalState finalFrame control)

  /-- Left-to-right evaluation of block statements. -/
  inductive EvalStatementsWith (unit : ExecutableUnit) (callee : CalleeRelation) :
      NamespaceId → RuntimeFrame → RuntimeState → List ExprId → StatementsResult →
      Prop where
    | nil (namespaceId frame state) :
        EvalStatementsWith unit callee namespaceId frame state [] (.done state frame)
    | headControl (namespaceId frame state statement statements finalFrame finalState control)
        (head_step : EvalExprWith unit callee namespaceId frame state statement
          finalFrame finalState control)
        (abrupt : Abrupt control) :
        EvalStatementsWith unit callee namespaceId frame state (statement :: statements)
          (.control finalState finalFrame control)
    | cons (namespaceId frame state statement statements headFrame headState value result)
        (head_step : EvalExprWith unit callee namespaceId frame state statement
          headFrame headState (.value value))
        (tail_step : EvalStatementsWith unit callee namespaceId headFrame headState statements result) :
        EvalStatementsWith unit callee namespaceId frame state (statement :: statements) result

  /-- First matching arm, including guard evaluation. -/
  inductive EvalArmsWith (unit : ExecutableUnit) (callee : CalleeRelation) :
      NamespaceId → ValidatedNamespace → RuntimeFrame → RuntimeState → RuntimeValue →
      List MatchArm → RuntimeFrame → RuntimeState → Control → Prop where
    | reject (namespaceId ns frame state value arm arms finalFrame finalState control)
        (bind_eq : bindPattern unit.unit ns frame arm.pattern value = none)
        (tail_step : EvalArmsWith unit callee namespaceId ns frame state value arms
          finalFrame finalState control) :
        EvalArmsWith unit callee namespaceId ns frame
          state value (arm :: arms) finalFrame finalState control
    | noGuard (namespaceId ns frame state value arm arms armFrame finalFrame finalState control)
        (guard_eq : arm.guard = none)
        (bind_eq : bindPattern unit.unit ns frame arm.pattern value = some armFrame)
        (body_step : EvalExprWith unit callee namespaceId armFrame state arm.body
          finalFrame finalState control) :
        EvalArmsWith unit callee namespaceId ns frame
          state value (arm :: arms) finalFrame finalState control
    | guardControl (namespaceId ns frame state value arm arms guard armFrame
        finalFrame finalState control)
        (guard_eq : arm.guard = some guard)
        (bind_eq : bindPattern unit.unit ns frame arm.pattern value = some armFrame)
        (guard_step : EvalExprWith unit callee namespaceId armFrame state guard
          finalFrame finalState control)
        (abrupt : Abrupt control) :
        EvalArmsWith unit callee namespaceId ns frame
          state value (arm :: arms) finalFrame finalState control
    | guardTrue (namespaceId ns frame state value arm arms guard armFrame guardFrame guardState
        finalFrame finalState control)
        (guard_eq : arm.guard = some guard)
        (bind_eq : bindPattern unit.unit ns frame arm.pattern value = some armFrame)
        (guard_step : EvalExprWith unit callee namespaceId armFrame state guard
          guardFrame guardState (.value (.bool true)))
        (body_step : EvalExprWith unit callee namespaceId guardFrame guardState arm.body
          finalFrame finalState control) :
        EvalArmsWith unit callee namespaceId ns frame
          state value (arm :: arms) finalFrame finalState control
    | guardFalse (namespaceId ns frame state value arm arms guard armFrame guardFrame guardState
        finalFrame finalState control)
        (guard_eq : arm.guard = some guard)
        (bind_eq : bindPattern unit.unit ns frame arm.pattern value = some armFrame)
        (guard_step : EvalExprWith unit callee namespaceId armFrame state guard
          guardFrame guardState (.value (.bool false)))
        (tail_step : EvalArmsWith unit callee namespaceId ns frame state value arms
          finalFrame finalState control) :
        EvalArmsWith unit callee namespaceId ns frame
          state value (arm :: arms) finalFrame finalState control
end

/-- The closed big-step semantics of a whole function: the body rules with
every call resolved by `EvalFunction` itself.  Nontermination is the absence
of a finite derivation, exactly as before; the oracle only names the knot. -/
inductive EvalFunction (unit : ExecutableUnit) : FunctionHandle →
    Array (TypeId × TypeId) → RuntimeState →
    Array RuntimeValue → RuntimeState → Outcome → Prop where
  | body (handle typeInstantiation initialState arguments ns declaration frame root
      finalFrame evaluatedState
      finalState control outcome)
      (namespace_eq : unit.unit.namespaces[handle.namespaceId.index]? = some ns)
      (declaration_eq : ns.functions[handle.functionId.index]? = some declaration)
      (frame_eq : initialFrame? declaration arguments typeInstantiation = some frame)
      (body_eq : declaration.body = .structured root)
      (body_step : EvalExprWith unit (EvalFunction unit) handle.namespaceId frame initialState root
        finalFrame evaluatedState control)
      (outcome_eq : finishControl? declaration.signature.results.size control = some outcome)
      (finalize_eq : finalizeFunctionState unit declaration.profile initialState evaluatedState
        finalFrame outcome = finalState) :
      EvalFunction unit handle typeInstantiation initialState arguments finalState outcome


/-- Big-step evaluation of one structured expression, calls resolved by the
closed semantics. -/
abbrev EvalExpr (unit : ExecutableUnit) := EvalExprWith unit (EvalFunction unit)

/-- Left-to-right evaluation of expression operands, closed. -/
abbrev EvalValues (unit : ExecutableUnit) := EvalValuesWith unit (EvalFunction unit)

/-- Left-to-right evaluation of block statements, closed. -/
abbrev EvalStatements (unit : ExecutableUnit) :=
  EvalStatementsWith unit (EvalFunction unit)

/-- Match-arm selection, closed. -/
abbrev EvalArms (unit : ExecutableUnit) := EvalArmsWith unit (EvalFunction unit)

namespace EvalExpr
export EvalExprWith (value constantValue constantControl localVar callArgumentsControl callReturned callThrew constructorArgumentsControl constructorValue destructorArgumentsControl destructorValue closureArgumentsControl closureValue invokeArgumentsControl invokeReturned invokeThrew profileArgumentsControl profileValue profileThrow primitiveArgumentsControl primitiveValue primitiveThrow globalArgumentsControl globalValue globalThrow assertArgumentsControl assertTrue assertFalse operationArgumentsControl operationValue blockControl blockUnit blockResult letNoValue letValueControl letValue ifControl ifTrue ifFalseUnit ifFalse matchControl matchValue loopRepeatValue loopRepeatContinue loopBreak loopOuterBreak loopOuterContinue loopReturn loopThrow breakNone breakControl breakValue continue_ returnValues returnControl throwValues throwControl assignControl assignValue assignPatternControl assignPatternValue spec)
end EvalExpr

namespace EvalValues
export EvalValuesWith (nil headControl tailValues tailControl)
end EvalValues

namespace EvalStatements
export EvalStatementsWith (nil headControl cons)
end EvalStatements

namespace EvalArms
export EvalArmsWith (reject noGuard guardControl guardTrue guardFalse)
end EvalArms

/-- The open function boundary: a body evaluated under an arbitrary callee
oracle.  `EvalFunction` is its knot (`EvalFunction_iff`). -/
def EvalFunctionWith (unit : ExecutableUnit) (callee : CalleeRelation)
    (handle : FunctionHandle) (typeInstantiation : Array (TypeId × TypeId))
    (initialState : RuntimeState)
    (arguments : Array RuntimeValue) (finalState : RuntimeState)
    (outcome : Outcome) : Prop :=
  ∃ ns declaration frame root finalFrame evaluatedState control,
    unit.unit.namespaces[handle.namespaceId.index]? = some ns ∧
    ns.functions[handle.functionId.index]? = some declaration ∧
    initialFrame? declaration arguments typeInstantiation = some frame ∧
    declaration.body = .structured root ∧
    EvalExprWith unit callee handle.namespaceId frame initialState root
      finalFrame evaluatedState control ∧
    finishControl? declaration.signature.results.size control = some outcome ∧
    finalizeFunctionState unit declaration.profile initialState evaluatedState
      finalFrame outcome = finalState

theorem EvalFunction_iff (unit : ExecutableUnit) (handle : FunctionHandle)
    (typeInstantiation : Array (TypeId × TypeId))
    (initialState : RuntimeState) (arguments : Array RuntimeValue)
    (finalState : RuntimeState) (outcome : Outcome) :
    EvalFunction unit handle typeInstantiation initialState arguments finalState outcome ↔
      EvalFunctionWith unit (EvalFunction unit) handle typeInstantiation initialState arguments
        finalState outcome := by
  constructor
  · intro step
    cases step
    exact ⟨_, _, _, _, _, _, _, ‹_›, ‹_›, ‹_›, ‹_›, ‹_›, ‹_›, ‹_›⟩
  · rintro ⟨ns, declaration, frame, root, finalFrame, evaluatedState, control,
      namespace_eq, declaration_eq, frame_eq, body_eq, body_step, outcome_eq,
      finalize_eq⟩
    exact .body handle typeInstantiation initialState arguments ns declaration frame root finalFrame
      evaluatedState finalState control outcome namespace_eq declaration_eq
      frame_eq body_eq body_step outcome_eq finalize_eq


/-- A source-independent function meaning over the declarative relation. -/
structure FunctionMeaning (unit : ExecutableUnit) (function : FunctionHandle) where
  relates : RuntimeState → Array RuntimeValue → RuntimeState → Outcome → Prop :=
    EvalFunction unit function #[]

/-- Canonical M1 meaning of a validated function. -/
def meaning (unit : ExecutableUnit) (function : FunctionHandle) : FunctionMeaning unit function :=
  { relates := EvalFunction unit function #[] }

end BigStep
end LeanerIR
