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

/-- The runtime's implementation of a function without a body: the final
state and outcome of a native call, where the runtime provides one. It is
fixed but unknown, so a proof about a caller assumes the native's contract. -/
opaque nativeCall {unit : ValidatedUnit} : ExecutableUnit unit → FunctionHandle →
    Array (TypeId × TypeId) → RuntimeState → Array RuntimeValue → Option (RuntimeState × Outcome)

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
  /-- Big-step evaluation of one structured expression: the loans whose
  deaths the borrow certificates anchor before it end, its node runs, and
  once the node produced a value the loans anchored after it end. -/
  inductive EvalExprWith {unit : ValidatedUnit} (executable : ExecutableUnit unit)
      (callee : CalleeRelation) :
      NamespaceId → RuntimeFrame → RuntimeState → ExprId → RuntimeFrame →
      RuntimeState → Control → Prop where
    | node (namespaceId frame state exprId startFrame startState nodeFrame nodeState control
        finalFrame finalState)
        (before_eq : settleLoans (loanDeathsAt unit namespaceId exprId).before frame state =
          (startFrame, startState))
        (node_step : EvalNodeWith executable callee namespaceId startFrame startState exprId
          nodeFrame nodeState control)
        (after_eq : settleAfter (loanDeathsAt unit namespaceId exprId).after control
          nodeFrame nodeState = (finalFrame, finalState)) :
        EvalExprWith executable callee namespaceId frame state exprId finalFrame finalState control

  /-- Big-step evaluation of one node, the loan deaths at it aside. A loop
  repeats its node: the deaths at the loop end once, around all of its
  iterations. -/
  inductive EvalNodeWith {unit : ValidatedUnit} (executable : ExecutableUnit unit)
      (callee : CalleeRelation) :
      NamespaceId → RuntimeFrame → RuntimeState → ExprId → RuntimeFrame →
      RuntimeState → Control → Prop where
    | value (namespaceId frame state exprId ns expression literal source runtimeValue)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .value literal source)
        (value_eq : constValue? literal = some runtimeValue) :
        EvalNodeWith executable callee namespaceId frame state exprId frame state (.value runtimeValue)
    | constantValue (namespaceId frame state exprId ns expression reference handle
        targetNs declaration targetFrame finalState runtimeValue)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .constant reference)
        (resolve_eq : resolveConstant? unit namespaceId reference = some handle)
        (target_namespace_eq : unit.namespaces[handle.namespaceId.index]? = some targetNs)
        (declaration_eq : targetNs.constants[handle.constantId]? = some declaration)
        (initializer : EvalExprWith executable callee handle.namespaceId { locals := #[] } state
          declaration.value targetFrame finalState (.value runtimeValue)) :
        EvalNodeWith executable callee namespaceId frame state exprId
          frame finalState (.value runtimeValue)
    | constantControl (namespaceId frame state exprId ns expression reference handle
        targetNs declaration targetFrame finalState control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .constant reference)
        (resolve_eq : resolveConstant? unit namespaceId reference = some handle)
        (target_namespace_eq : unit.namespaces[handle.namespaceId.index]? = some targetNs)
        (declaration_eq : targetNs.constants[handle.constantId]? = some declaration)
        (initializer : EvalExprWith executable callee handle.namespaceId { locals := #[] } state
          declaration.value targetFrame finalState control)
        (abrupt : Abrupt control) :
        EvalNodeWith executable callee namespaceId frame state exprId frame finalState control
    | localVar (namespaceId frame state exprId ns expression localId runtimeValue)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .localVar localId)
        (local_eq : readLocal? frame localId = some runtimeValue) :
        EvalNodeWith executable callee namespaceId frame state exprId frame state (.value runtimeValue)
    | callArgumentsControl (namespaceId frame state exprId ns expression reference
        instantiations arguments surface finalState finalFrame control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call (.function reference))
          instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.control finalState finalFrame control)) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | callReturned (namespaceId frame state exprId ns expression reference instantiations
        arguments surface argumentState argumentFrame values handle finalState results)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call (.function reference))
          instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.values argumentState argumentFrame values))
        (resolve_eq : resolveFunction? unit namespaceId reference = some handle)
        (calleeStep : callee handle
          (callTypeInstantiation unit handle argumentFrame.typeInstantiation instantiations)
          argumentState values.toArray finalState
          (.returned results)) :
        EvalNodeWith executable callee namespaceId frame state exprId
          (registerReturnedLoan
            (certificateLoanId? unit namespaceId exprId) results
            (applyPendingFrom argumentState.pending argumentFrame finalState).1)
          (applyPendingFrom argumentState.pending argumentFrame finalState).2
          (.value (packResults results))
    | callThrew (namespaceId frame state exprId ns expression reference instantiations
        arguments surface argumentState argumentFrame values handle finalState kind thrown)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call (.function reference))
          instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.values argumentState argumentFrame values))
        (resolve_eq : resolveFunction? unit namespaceId reference = some handle)
        (calleeStep : callee handle
          (callTypeInstantiation unit handle argumentFrame.typeInstantiation instantiations)
          argumentState values.toArray finalState
          (.threw kind thrown)) :
        EvalNodeWith executable callee namespaceId frame state exprId
          (applyPendingFrom argumentState.pending argumentFrame finalState).1
          (applyPendingFrom argumentState.pending argumentFrame finalState).2
          (.throw_ kind thrown)
    | constructorArgumentsControl (namespaceId frame state exprId ns expression reference variant
        instantiations arguments surface finalState finalFrame control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call (.constructor reference variant))
          instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.control finalState finalFrame control)) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | constructorValue (namespaceId frame state exprId ns expression reference variant
        instantiations arguments surface finalState finalFrame values runtimeValue)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call (.constructor reference variant))
          instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.values finalState finalFrame values))
        (construct_eq : constructNominal? unit namespaceId reference variant values.toArray =
          some runtimeValue) :
        EvalNodeWith executable callee namespaceId frame state exprId
          finalFrame finalState (.value runtimeValue)
    | destructorArgumentsControl (namespaceId frame state exprId ns expression reference variant
        instantiations arguments surface finalState finalFrame control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call (.destructor reference variant))
          instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.control finalState finalFrame control)) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | destructorValue (namespaceId frame state exprId ns expression reference variant
        instantiations arguments surface finalState finalFrame value fields)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call (.destructor reference variant))
          instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.values finalState finalFrame [value]))
        (destruct_eq : destructNominal? unit namespaceId reference variant value = some fields) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState
          (.value (packResults fields))
    | closureArgumentsControl (namespaceId frame state exprId ns expression reference mask
        instantiations captures surface finalState finalFrame control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call (.closure reference mask))
          instantiations captures surface)
        (operands : EvalValuesWith executable callee namespaceId frame state captures.toList
          (.control finalState finalFrame control)) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    /-- A closure fixes its target's type instantiation where it is built. -/
    | closureValue (namespaceId frame state exprId ns expression reference mask instantiations
        captures surface finalState finalFrame values handle)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call (.closure reference mask))
          instantiations captures surface)
        (operands : EvalValuesWith executable callee namespaceId frame state captures.toList
          (.values finalState finalFrame values))
        (resolve_eq : resolveFunction? unit namespaceId reference = some handle) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState
          (.value (.closure handle mask
            (callTypeInstantiation unit handle finalFrame.typeInstantiation instantiations)
            values.toArray))
    | invokeArgumentsControl (namespaceId frame state exprId ns expression instantiations arguments
        surface finalState finalFrame control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call .invoke) instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.control finalState finalFrame control)) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    /-- An invocation calls the closure's target under the instantiation
    the closure fixed, with the captures and the supplied arguments
    composed by its mask, as a direct call does. -/
    | invokeReturned (namespaceId frame state exprId ns expression instantiations arguments surface
        argumentState argumentFrame handle mask typeInstantiation captures values composed
        finalState results)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call .invoke) instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.values argumentState argumentFrame
            (.closure handle mask typeInstantiation captures :: values)))
        (compose_eq : ClosureMask.compose mask captures.toList values = some composed)
        (calleeStep : callee handle typeInstantiation argumentState composed.toArray finalState
          (.returned results)) :
        EvalNodeWith executable callee namespaceId frame state exprId
          (registerReturnedLoan
            (certificateLoanId? unit namespaceId exprId) results
            (applyPendingFrom argumentState.pending argumentFrame finalState).1)
          (applyPendingFrom argumentState.pending argumentFrame finalState).2
          (.value (packResults results))
    | invokeThrew (namespaceId frame state exprId ns expression instantiations arguments surface
        argumentState argumentFrame handle mask typeInstantiation captures values composed
        finalState kind thrown)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.call .invoke) instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.values argumentState argumentFrame
            (.closure handle mask typeInstantiation captures :: values)))
        (compose_eq : ClosureMask.compose mask captures.toList values = some composed)
        (calleeStep : callee handle typeInstantiation argumentState composed.toArray finalState
          (.threw kind thrown)) :
        EvalNodeWith executable callee namespaceId frame state exprId
          (applyPendingFrom argumentState.pending argumentFrame finalState).1
          (applyPendingFrom argumentState.pending argumentFrame finalState).2
          (.throw_ kind thrown)
    | profileArgumentsControl (namespaceId frame state exprId ns expression operation targets
        instantiations arguments surface finalState finalFrame control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.profile operation targets)
          instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.control finalState finalFrame control)) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | profileValue (namespaceId frame state exprId ns expression operation targets instantiations
        arguments surface finalState finalFrame values runtimeValue)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.profile operation targets)
          instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.values finalState finalFrame values))
        (evaluate_eq : evaluateProfileOperation? executable ns expression.typeId operation values.toArray =
          some (.ok runtimeValue)) :
        EvalNodeWith executable callee namespaceId frame state exprId
          finalFrame finalState (.value runtimeValue)
    | profileThrow (namespaceId frame state exprId ns expression operation targets instantiations
        arguments surface finalState finalFrame values kind thrown)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.profile operation targets)
          instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.values finalState finalFrame values))
        (evaluate_eq : evaluateProfileOperation? executable ns expression.typeId operation values.toArray =
          some (.error (kind, thrown))) :
        EvalNodeWith executable callee namespaceId frame state exprId
          finalFrame finalState (.throw_ kind thrown)
    | primitiveArgumentsControl (namespaceId frame state exprId ns expression operation
        instantiations arguments surface finalState finalFrame control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.primitive operation)
          instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.control finalState finalFrame control)) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | primitiveValue (namespaceId frame state exprId ns expression operation instantiations
        arguments surface finalState finalFrame values runtimeValue)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.primitive operation)
          instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.values finalState finalFrame values))
        (evaluate_eq : evaluatePrimitiveOperation? ns expression.typeId operation values.toArray
          executable.targetPointerWidth =
          some (.ok runtimeValue)) :
        EvalNodeWith executable callee namespaceId frame state exprId
          finalFrame finalState (.value runtimeValue)
    | primitiveThrow (namespaceId frame state exprId ns expression operation instantiations
        arguments surface finalState finalFrame values kind thrown)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.primitive operation)
          instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.values finalState finalFrame values))
        (evaluate_eq : evaluatePrimitiveOperation? ns expression.typeId operation values.toArray
          executable.targetPointerWidth =
          some (.error (kind, thrown))) :
        EvalNodeWith executable callee namespaceId frame state exprId
          finalFrame finalState (.throw_ kind thrown)
    | globalArgumentsControl (namespaceId frame state exprId ns expression kind instantiations
        arguments surface finalState finalFrame control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.global kind) instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.control finalState finalFrame control)) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | globalValue (namespaceId frame state exprId ns expression kind instantiations arguments
        surface argumentState argumentFrame values finalFrame finalState runtimeValue)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.global kind) instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.values argumentState argumentFrame values))
        (evaluate_eq : evaluateGlobalOperation? unit ns expression.typeId exprId kind
          instantiations values.toArray argumentFrame argumentState =
            some (.value finalFrame finalState runtimeValue)) :
        EvalNodeWith executable callee namespaceId frame state exprId
          finalFrame finalState (.value runtimeValue)
    | globalThrow (namespaceId frame state exprId ns expression globalKind instantiations arguments
        surface argumentState argumentFrame values finalFrame finalState throwKind thrown)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation (.global globalKind) instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.values argumentState argumentFrame values))
        (evaluate_eq : evaluateGlobalOperation? unit ns expression.typeId exprId globalKind
          instantiations values.toArray argumentFrame argumentState =
            some (.throw_ finalFrame finalState throwKind thrown)) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState
          (.throw_ throwKind thrown)
    | assertArgumentsControl (namespaceId frame state exprId ns expression instantiations arguments
        surface finalState finalFrame control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation .assert instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.control finalState finalFrame control)) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | assertTrue (namespaceId frame state exprId ns expression instantiations arguments surface
        finalState finalFrame)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation .assert instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.values finalState finalFrame [.bool true])) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState (.value .unit)
    | assertFalse (namespaceId frame state exprId ns expression instantiations arguments surface
        finalState finalFrame)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation .assert instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.values finalState finalFrame [.bool false])) :
        EvalNodeWith executable callee namespaceId frame state exprId
          finalFrame finalState (.throw_ .abort #[])
    | operationArgumentsControl (namespaceId frame state exprId ns expression operation
        instantiations arguments surface finalState finalFrame control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation operation instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.control finalState finalFrame control)) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | operationValue (namespaceId frame state exprId ns expression operation instantiations
        arguments surface argumentState argumentFrame values finalFrame finalState runtimeValue)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation operation instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.values argumentState argumentFrame values))
        (evaluate_eq : evaluatePlaceOperation? unit ns expression.typeId exprId operation
          values.toArray argumentFrame argumentState = some (finalFrame, finalState, runtimeValue)) :
        EvalNodeWith executable callee namespaceId frame state exprId
          finalFrame finalState (.value runtimeValue)
    | operationMismatch (namespaceId frame state exprId ns expression operation instantiations
        arguments surface argumentState argumentFrame values kind thrown)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .operation operation instantiations arguments surface)
        (operands : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.values argumentState argumentFrame values))
        (evaluate_eq : evaluatePlaceOperation? unit ns expression.typeId exprId operation
          values.toArray argumentFrame argumentState = none)
        (mismatch : variantMismatch? unit ns operation values.toArray argumentFrame
          argumentState = true)
        (mismatch_eq : patternMismatchThrow? ns.profile = some (kind, thrown)) :
        EvalNodeWith executable callee namespaceId frame state exprId argumentFrame argumentState
          (.throw_ kind thrown)
    | blockControl (namespaceId frame state exprId ns expression statements result
        finalState finalFrame control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .block statements result)
        (steps : EvalStatementsWith executable callee namespaceId frame state statements.toList
          (.control finalState finalFrame control)) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | blockUnit (namespaceId frame state exprId ns expression statements finalState finalFrame)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .block statements none)
        (steps : EvalStatementsWith executable callee namespaceId frame state statements.toList
          (.done finalState finalFrame)) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState (.value .unit)
    | blockResult (namespaceId frame state exprId ns expression statements result
        statementState statementFrame finalState finalFrame control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .block statements (some result))
        (steps : EvalStatementsWith executable callee namespaceId frame state statements.toList
          (.done statementState statementFrame))
        (result_step : EvalExprWith executable callee namespaceId statementFrame statementState result
          finalFrame finalState control) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | letNoValue (namespaceId frame state exprId ns expression pattern body
        finalFrame finalState control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .letDecl pattern none body)
        (body_step : EvalExprWith executable callee namespaceId frame state body finalFrame finalState control) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | letValueControl (namespaceId frame state exprId ns expression pattern initializer body
        finalFrame finalState control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .letDecl pattern (some initializer) body)
        (initializer_step : EvalExprWith executable callee namespaceId frame state initializer
          finalFrame finalState control)
        (abrupt : Abrupt control) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | letValue (namespaceId frame state exprId ns expression pattern initializer body
        initializedFrame initializedState runtimeValue boundFrame finalFrame finalState control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .letDecl pattern (some initializer) body)
        (initializer_step : EvalExprWith executable callee namespaceId frame state initializer
          initializedFrame initializedState (.value runtimeValue))
        (bind_eq : bindPattern unit ns initializedFrame pattern runtimeValue = some boundFrame)
        (body_step : EvalExprWith executable callee namespaceId boundFrame initializedState body
          finalFrame finalState control) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | letMismatch (namespaceId frame state exprId ns expression pattern initializer body
        initializedFrame initializedState runtimeValue kind arguments)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .letDecl pattern (some initializer) body)
        (initializer_step : EvalExprWith executable callee namespaceId frame state initializer
          initializedFrame initializedState (.value runtimeValue))
        (bind_eq : bindPattern unit ns initializedFrame pattern runtimeValue = none)
        (mismatch_eq : patternMismatchThrow? ns.profile = some (kind, arguments)) :
        EvalNodeWith executable callee namespaceId frame state exprId initializedFrame initializedState
          (.throw_ kind arguments)
    | ifControl (namespaceId frame state exprId ns expression condition thenBranch elseBranch
        finalFrame finalState control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .ifElse condition thenBranch elseBranch)
        (condition_step : EvalExprWith executable callee namespaceId frame state condition
          finalFrame finalState control)
        (abrupt : Abrupt control) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | ifTrue (namespaceId frame state exprId ns expression condition thenBranch elseBranch
        conditionFrame conditionState finalFrame finalState control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .ifElse condition thenBranch elseBranch)
        (condition_step : EvalExprWith executable callee namespaceId frame state condition
          conditionFrame conditionState (.value (.bool true)))
        (branch_step : EvalExprWith executable callee namespaceId conditionFrame conditionState thenBranch
          finalFrame finalState control) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | ifFalseUnit (namespaceId frame state exprId ns expression condition thenBranch
        conditionFrame conditionState)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .ifElse condition thenBranch none)
        (condition_step : EvalExprWith executable callee namespaceId frame state condition
          conditionFrame conditionState (.value (.bool false))) :
        EvalNodeWith executable callee namespaceId frame state exprId conditionFrame conditionState
          (.value .unit)
    | ifFalse (namespaceId frame state exprId ns expression condition thenBranch elseBranch
        conditionFrame conditionState finalFrame finalState control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .ifElse condition thenBranch (some elseBranch))
        (condition_step : EvalExprWith executable callee namespaceId frame state condition
          conditionFrame conditionState (.value (.bool false)))
        (branch_step : EvalExprWith executable callee namespaceId conditionFrame conditionState elseBranch
          finalFrame finalState control) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | matchControl (namespaceId frame state exprId ns expression scrutinee arms
        finalFrame finalState control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .match_ scrutinee arms)
        (scrutinee_step : EvalExprWith executable callee namespaceId frame state scrutinee
          finalFrame finalState control)
        (abrupt : Abrupt control) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | matchValue (namespaceId frame state exprId ns expression scrutinee arms
        scrutineeFrame scrutineeState runtimeValue finalFrame finalState control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .match_ scrutinee arms)
        (scrutinee_step : EvalExprWith executable callee namespaceId frame state scrutinee
          scrutineeFrame scrutineeState (.value runtimeValue))
        (arm_step : EvalArmsWith executable callee namespaceId ns scrutineeFrame scrutineeState runtimeValue
          arms.toList finalFrame finalState control) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | loopRepeatValue (namespaceId frame state exprId ns expression label body
        bodyFrame bodyState runtimeValue finalFrame finalState control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .loop label body)
        (body_step : EvalExprWith executable callee namespaceId frame state body bodyFrame bodyState
          (.value runtimeValue))
        (repeat_step : EvalNodeWith executable callee namespaceId bodyFrame bodyState exprId
          finalFrame finalState control) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | loopRepeatContinue (namespaceId frame state exprId ns expression label body
        bodyFrame bodyState finalFrame finalState control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .loop label body)
        (body_step : EvalExprWith executable callee namespaceId frame state body bodyFrame bodyState
          (.continue_ 0))
        (repeat_step : EvalNodeWith executable callee namespaceId bodyFrame bodyState exprId
          finalFrame finalState control) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | loopBreak (namespaceId frame state exprId ns expression label body
        finalFrame finalState value)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .loop label body)
        (body_step : EvalExprWith executable callee namespaceId frame state body finalFrame finalState
          (.break_ 0 value)) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState
          (.value (value.getD .unit))
    | loopOuterBreak (namespaceId frame state exprId ns expression label body
        finalFrame finalState nest value)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .loop label body)
        (body_step : EvalExprWith executable callee namespaceId frame state body finalFrame finalState
          (.break_ (nest + 1) value)) :
        EvalNodeWith executable callee namespaceId frame state exprId
          finalFrame finalState (.break_ nest value)
    | loopOuterContinue (namespaceId frame state exprId ns expression label body
        finalFrame finalState nest)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .loop label body)
        (body_step : EvalExprWith executable callee namespaceId frame state body finalFrame finalState
          (.continue_ (nest + 1))) :
        EvalNodeWith executable callee namespaceId frame state exprId
          finalFrame finalState (.continue_ nest)
    | loopReturn (namespaceId frame state exprId ns expression label body
        finalFrame finalState values)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .loop label body)
        (body_step : EvalExprWith executable callee namespaceId frame state body finalFrame finalState
          (.return_ values)) :
        EvalNodeWith executable callee namespaceId frame state exprId
          finalFrame finalState (.return_ values)
    | loopThrow (namespaceId frame state exprId ns expression label body
        finalFrame finalState kind arguments)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .loop label body)
        (body_step : EvalExprWith executable callee namespaceId frame state body finalFrame finalState
          (.throw_ kind arguments)) :
        EvalNodeWith executable callee namespaceId frame state exprId
          finalFrame finalState (.throw_ kind arguments)
    | breakNone (namespaceId frame state exprId ns expression nest)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .break_ nest none) :
        EvalNodeWith executable callee namespaceId frame state exprId frame state (.break_ nest none)
    | breakControl (namespaceId frame state exprId ns expression nest child
        finalFrame finalState control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .break_ nest (some child))
        (child_step : EvalExprWith executable callee namespaceId frame state child finalFrame finalState control)
        (abrupt : Abrupt control) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | breakValue (namespaceId frame state exprId ns expression nest child
        finalFrame finalState runtimeValue)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .break_ nest (some child))
        (child_step : EvalExprWith executable callee namespaceId frame state child finalFrame finalState
          (.value runtimeValue)) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState
          (.break_ nest (some runtimeValue))
    | continue_ (namespaceId frame state exprId ns expression nest)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .continue_ nest) :
        EvalNodeWith executable callee namespaceId frame state exprId frame state (.continue_ nest)
    | returnValues (namespaceId frame state exprId ns expression values
        finalFrame finalState runtimeValues)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .return_ values)
        (values_step : EvalValuesWith executable callee namespaceId frame state values.toList
          (.values finalState finalFrame runtimeValues)) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState
          (.return_ runtimeValues.toArray)
    | returnControl (namespaceId frame state exprId ns expression values
        finalFrame finalState control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .return_ values)
        (values_step : EvalValuesWith executable callee namespaceId frame state values.toList
          (.control finalState finalFrame control)) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | throwValues (namespaceId frame state exprId ns expression kind arguments
        finalFrame finalState runtimeValues)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .throw_ kind arguments)
        (values_step : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.values finalState finalFrame runtimeValues)) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState
          (.throw_ kind runtimeValues.toArray)
    | throwControl (namespaceId frame state exprId ns expression kind arguments
        finalFrame finalState control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .throw_ kind arguments)
        (values_step : EvalValuesWith executable callee namespaceId frame state arguments.toList
          (.control finalState finalFrame control)) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | assignControl (namespaceId frame state exprId ns expression place child
        finalFrame finalState control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .assign place child)
        (child_step : EvalExprWith executable callee namespaceId frame state child finalFrame finalState control)
        (abrupt : Abrupt control) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | assignValue (namespaceId frame state exprId ns expression place child
        childFrame childState resolved finalFrame finalState runtimeValue)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .assign place child)
        (child_step : EvalExprWith executable callee namespaceId frame state child childFrame childState
          (.value runtimeValue))
        (resolve_eq : resolvePlace? unit ns childFrame childState place = some resolved)
        (write_eq : writeRuntimePlace? childFrame childState resolved runtimeValue =
          some (finalFrame, finalState)) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState (.value .unit)
    | assignMismatch (namespaceId frame state exprId ns expression place child
        childFrame childState runtimeValue kind thrown)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .assign place child)
        (child_step : EvalExprWith executable callee namespaceId frame state child childFrame childState
          (.value runtimeValue))
        (resolve_eq : resolvePlace? unit ns childFrame childState place = none)
        (mismatch : placeVariantMismatchFuel? unit ns childFrame childState
          (2 * ns.places.size + 3) place = true)
        (mismatch_eq : patternMismatchThrow? ns.profile = some (kind, thrown)) :
        EvalNodeWith executable callee namespaceId frame state exprId childFrame childState
          (.throw_ kind thrown)
    | assignPatternControl (namespaceId frame state exprId ns expression pattern child
        finalFrame finalState control)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .assignPattern pattern child)
        (child_step : EvalExprWith executable callee namespaceId frame state child finalFrame finalState control)
        (abrupt : Abrupt control) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState control
    | assignPatternValue (namespaceId frame state exprId ns expression pattern child
        childFrame finalFrame finalState runtimeValue)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .assignPattern pattern child)
        (child_step : EvalExprWith executable callee namespaceId frame state child childFrame finalState
          (.value runtimeValue))
        (bind_eq : bindPattern unit ns childFrame pattern runtimeValue = some finalFrame) :
        EvalNodeWith executable callee namespaceId frame state exprId finalFrame finalState (.value .unit)
    | assignPatternMismatch (namespaceId frame state exprId ns expression pattern child
        childFrame finalState runtimeValue kind arguments)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .assignPattern pattern child)
        (child_step : EvalExprWith executable callee namespaceId frame state child childFrame finalState
          (.value runtimeValue))
        (bind_eq : bindPattern unit ns childFrame pattern runtimeValue = none)
        (mismatch_eq : patternMismatchThrow? ns.profile = some (kind, arguments)) :
        EvalNodeWith executable callee namespaceId frame state exprId childFrame finalState
          (.throw_ kind arguments)
    | spec (namespaceId frame state exprId ns expression block)
        (namespace_eq : unit.namespaces[namespaceId.index]? = some ns)
        (expression_eq : ns.expressions[exprId.index]? = some expression)
        (kind_eq : expression.kind = .spec block) :
        EvalNodeWith executable callee namespaceId frame state exprId frame state (.value .unit)

  /-- Left-to-right evaluation of expression operands. -/
  inductive EvalValuesWith {unit : ValidatedUnit} (executable : ExecutableUnit unit)
      (callee : CalleeRelation) :
      NamespaceId → RuntimeFrame → RuntimeState → List ExprId → ValuesResult → Prop where
    | nil (namespaceId frame state) :
        EvalValuesWith executable callee namespaceId frame state [] (.values state frame [])
    | headControl (namespaceId frame state expression expressions finalFrame finalState control)
        (head_step : EvalExprWith executable callee namespaceId frame state expression
          finalFrame finalState control)
        (abrupt : Abrupt control) :
        EvalValuesWith executable callee namespaceId frame state (expression :: expressions)
          (.control finalState finalFrame control)
    | tailValues (namespaceId frame state expression expressions headFrame headState value
        finalFrame finalState values)
        (head_step : EvalExprWith executable callee namespaceId frame state expression
          headFrame headState (.value value))
        (tail_step : EvalValuesWith executable callee namespaceId headFrame headState expressions
          (.values finalState finalFrame values)) :
        EvalValuesWith executable callee namespaceId frame state (expression :: expressions)
          (.values finalState finalFrame (value :: values))
    | tailControl (namespaceId frame state expression expressions headFrame headState value
        finalFrame finalState control)
        (head_step : EvalExprWith executable callee namespaceId frame state expression
          headFrame headState (.value value))
        (tail_step : EvalValuesWith executable callee namespaceId headFrame headState expressions
          (.control finalState finalFrame control)) :
        EvalValuesWith executable callee namespaceId frame state (expression :: expressions)
          (.control finalState finalFrame control)

  /-- Left-to-right evaluation of block statements. -/
  inductive EvalStatementsWith {unit : ValidatedUnit} (executable : ExecutableUnit unit)
      (callee : CalleeRelation) :
      NamespaceId → RuntimeFrame → RuntimeState → List ExprId → StatementsResult →
      Prop where
    | nil (namespaceId frame state) :
        EvalStatementsWith executable callee namespaceId frame state [] (.done state frame)
    | headControl (namespaceId frame state statement statements finalFrame finalState control)
        (head_step : EvalExprWith executable callee namespaceId frame state statement
          finalFrame finalState control)
        (abrupt : Abrupt control) :
        EvalStatementsWith executable callee namespaceId frame state (statement :: statements)
          (.control finalState finalFrame control)
    | cons (namespaceId frame state statement statements headFrame headState value result)
        (head_step : EvalExprWith executable callee namespaceId frame state statement
          headFrame headState (.value value))
        (tail_step : EvalStatementsWith executable callee namespaceId headFrame headState statements result) :
        EvalStatementsWith executable callee namespaceId frame state (statement :: statements) result

  /-- First matching arm, including guard evaluation. -/
  inductive EvalArmsWith {unit : ValidatedUnit} (executable : ExecutableUnit unit)
      (callee : CalleeRelation) :
      NamespaceId → ValidatedNamespace → RuntimeFrame → RuntimeState → RuntimeValue →
      List MatchArm → RuntimeFrame → RuntimeState → Control → Prop where
    | exhausted (namespaceId ns frame state value kind arguments)
        (mismatch_eq : patternMismatchThrow? ns.profile = some (kind, arguments)) :
        EvalArmsWith executable callee namespaceId ns frame state value [] frame state
          (.throw_ kind arguments)
    | reject (namespaceId ns frame state value arm arms finalFrame finalState control)
        (bind_eq : bindPattern unit ns frame arm.pattern value = none)
        (tail_step : EvalArmsWith executable callee namespaceId ns frame state value arms
          finalFrame finalState control) :
        EvalArmsWith executable callee namespaceId ns frame
          state value (arm :: arms) finalFrame finalState control
    | noGuard (namespaceId ns frame state value arm arms armFrame finalFrame finalState control)
        (guard_eq : arm.guard = none)
        (bind_eq : bindPattern unit ns frame arm.pattern value = some armFrame)
        (body_step : EvalExprWith executable callee namespaceId armFrame state arm.body
          finalFrame finalState control) :
        EvalArmsWith executable callee namespaceId ns frame
          state value (arm :: arms) finalFrame finalState control
    | guardControl (namespaceId ns frame state value arm arms guard armFrame
        finalFrame finalState control)
        (guard_eq : arm.guard = some guard)
        (bind_eq : bindPattern unit ns frame arm.pattern value = some armFrame)
        (guard_step : EvalExprWith executable callee namespaceId armFrame state guard
          finalFrame finalState control)
        (abrupt : Abrupt control) :
        EvalArmsWith executable callee namespaceId ns frame
          state value (arm :: arms) finalFrame finalState control
    | guardTrue (namespaceId ns frame state value arm arms guard armFrame guardFrame guardState
        finalFrame finalState control)
        (guard_eq : arm.guard = some guard)
        (bind_eq : bindPattern unit ns frame arm.pattern value = some armFrame)
        (guard_step : EvalExprWith executable callee namespaceId armFrame state guard
          guardFrame guardState (.value (.bool true)))
        (body_step : EvalExprWith executable callee namespaceId guardFrame guardState arm.body
          finalFrame finalState control) :
        EvalArmsWith executable callee namespaceId ns frame
          state value (arm :: arms) finalFrame finalState control
    | guardFalse (namespaceId ns frame state value arm arms guard armFrame guardFrame guardState
        finalFrame finalState control)
        (guard_eq : arm.guard = some guard)
        (bind_eq : bindPattern unit ns frame arm.pattern value = some armFrame)
        (guard_step : EvalExprWith executable callee namespaceId armFrame state guard
          guardFrame guardState (.value (.bool false)))
        (tail_step : EvalArmsWith executable callee namespaceId ns frame state value arms
          finalFrame finalState control) :
        EvalArmsWith executable callee namespaceId ns frame
          state value (arm :: arms) finalFrame finalState control
end

/-- The closed big-step semantics of a whole function: the body rules with
every call resolved by `EvalFunction` itself.  Nontermination is the absence
of a finite derivation, exactly as before; the oracle only names the knot. -/
inductive EvalFunction {unit : ValidatedUnit} (executable : ExecutableUnit unit) : FunctionHandle →
    Array (TypeId × TypeId) → RuntimeState →
    Array RuntimeValue → RuntimeState → Outcome → Prop where
  | body (handle typeInstantiation initialState arguments ns declaration frame root
      finalFrame evaluatedState
      finalState control outcome)
      (namespace_eq : unit.namespaces[handle.namespaceId.index]? = some ns)
      (declaration_eq : ns.functions[handle.functionId.index]? = some declaration)
      (frame_eq : initialFrame? declaration arguments typeInstantiation = some frame)
      (body_eq : declaration.body = .structured root)
      (body_step : EvalExprWith executable (EvalFunction executable) handle.namespaceId frame initialState root
        finalFrame evaluatedState control)
      (outcome_eq : finishControl? declaration.signature.results.size control = some outcome)
      (hole_free : outcome.holeFree = true)
      (finalize_eq : finalizeFunctionState executable declaration.profile initialState evaluatedState
        finalFrame outcome = finalState) :
      EvalFunction executable handle typeInstantiation initialState arguments finalState outcome
  | native (handle typeInstantiation initialState arguments ns declaration finalState outcome)
      (namespace_eq : unit.namespaces[handle.namespaceId.index]? = some ns)
      (declaration_eq : ns.functions[handle.functionId.index]? = some declaration)
      (arity_eq : arguments.size = declaration.signature.parameters.size)
      (body_eq : declaration.body = .absent)
      (native_eq : nativeCall executable handle typeInstantiation initialState arguments =
        some (finalState, outcome))
      (hole_free : outcome.holeFree = true) :
      EvalFunction executable handle typeInstantiation initialState arguments finalState outcome


/-- Big-step evaluation of one structured expression, calls resolved by the
closed semantics. -/
abbrev EvalExpr {unit : ValidatedUnit}
    (executable : ExecutableUnit unit) := EvalExprWith executable (EvalFunction executable)

/-- Big-step evaluation of one node, closed. -/
abbrev EvalNode {unit : ValidatedUnit}
    (executable : ExecutableUnit unit) := EvalNodeWith executable (EvalFunction executable)

/-- Left-to-right evaluation of expression operands, closed. -/
abbrev EvalValues {unit : ValidatedUnit}
    (executable : ExecutableUnit unit) := EvalValuesWith executable (EvalFunction executable)

/-- Left-to-right evaluation of block statements, closed. -/
abbrev EvalStatements {unit : ValidatedUnit} (executable : ExecutableUnit unit) :=
  EvalStatementsWith executable (EvalFunction executable)

/-- Match-arm selection, closed. -/
abbrev EvalArms {unit : ValidatedUnit}
    (executable : ExecutableUnit unit) := EvalArmsWith executable (EvalFunction executable)

namespace EvalExpr
export EvalExprWith (node)
end EvalExpr

namespace EvalNode
export EvalNodeWith (value constantValue constantControl localVar callArgumentsControl callReturned callThrew constructorArgumentsControl constructorValue destructorArgumentsControl destructorValue closureArgumentsControl closureValue invokeArgumentsControl invokeReturned invokeThrew profileArgumentsControl profileValue profileThrow primitiveArgumentsControl primitiveValue primitiveThrow globalArgumentsControl globalValue globalThrow assertArgumentsControl assertTrue assertFalse operationArgumentsControl operationValue operationMismatch blockControl blockUnit blockResult letNoValue letValueControl letValue letMismatch ifControl ifTrue ifFalseUnit ifFalse matchControl matchValue loopRepeatValue loopRepeatContinue loopBreak loopOuterBreak loopOuterContinue loopReturn loopThrow breakNone breakControl breakValue continue_ returnValues returnControl throwValues throwControl assignControl assignValue assignMismatch assignPatternControl assignPatternValue assignPatternMismatch spec)
end EvalNode

namespace EvalValues
export EvalValuesWith (nil headControl tailValues tailControl)
end EvalValues

namespace EvalStatements
export EvalStatementsWith (nil headControl cons)
end EvalStatements

namespace EvalArms
export EvalArmsWith (exhausted reject noGuard guardControl guardTrue guardFalse)
end EvalArms

/-- The open function boundary: a body evaluated under an arbitrary callee
oracle.  `EvalFunction` is its knot (`EvalFunction_iff`). -/
def EvalFunctionWith {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (callee : CalleeRelation)
    (handle : FunctionHandle) (typeInstantiation : Array (TypeId × TypeId))
    (initialState : RuntimeState)
    (arguments : Array RuntimeValue) (finalState : RuntimeState)
    (outcome : Outcome) : Prop :=
  (∃ ns declaration frame root finalFrame evaluatedState control,
    unit.namespaces[handle.namespaceId.index]? = some ns ∧
    ns.functions[handle.functionId.index]? = some declaration ∧
    initialFrame? declaration arguments typeInstantiation = some frame ∧
    declaration.body = .structured root ∧
    EvalExprWith executable callee handle.namespaceId frame initialState root
      finalFrame evaluatedState control ∧
    finishControl? declaration.signature.results.size control = some outcome ∧
    outcome.holeFree = true ∧
    finalizeFunctionState executable declaration.profile initialState evaluatedState
      finalFrame outcome = finalState) ∨
  (∃ ns declaration,
    unit.namespaces[handle.namespaceId.index]? = some ns ∧
    ns.functions[handle.functionId.index]? = some declaration ∧
    arguments.size = declaration.signature.parameters.size ∧
    declaration.body = .absent ∧
    nativeCall executable handle typeInstantiation initialState arguments = some (finalState, outcome) ∧
    outcome.holeFree = true)

theorem EvalFunction_iff {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (handle : FunctionHandle)
    (typeInstantiation : Array (TypeId × TypeId))
    (initialState : RuntimeState) (arguments : Array RuntimeValue)
    (finalState : RuntimeState) (outcome : Outcome) :
    EvalFunction executable handle typeInstantiation initialState arguments finalState outcome ↔
      EvalFunctionWith executable (EvalFunction executable) handle typeInstantiation initialState arguments
        finalState outcome := by
  constructor
  · intro step
    cases step
    · exact .inl ⟨_, _, _, _, _, _, _, ‹_›, ‹_›, ‹_›, ‹_›, ‹_›, ‹_›, ‹_›, ‹_›⟩
    · exact .inr ⟨_, _, ‹_›, ‹_›, ‹_›, ‹_›, ‹_›, ‹_›⟩
  · rintro (⟨ns, declaration, frame, root, finalFrame, evaluatedState, control,
      namespace_eq, declaration_eq, frame_eq, body_eq, body_step, outcome_eq, hole_free,
      finalize_eq⟩ | ⟨ns, declaration, namespace_eq, declaration_eq, arity_eq, body_eq, native_eq,
      hole_free⟩)
    · exact .body handle typeInstantiation initialState arguments ns declaration frame root finalFrame
        evaluatedState finalState control outcome namespace_eq declaration_eq
        frame_eq body_eq body_step outcome_eq hole_free finalize_eq
    · exact .native handle typeInstantiation initialState arguments ns declaration finalState outcome
        namespace_eq declaration_eq arity_eq body_eq native_eq hole_free


/-- A source-independent function meaning over the declarative relation. -/
structure FunctionMeaning {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (function : FunctionHandle) where
  relates : RuntimeState → Array RuntimeValue → RuntimeState → Outcome → Prop :=
    EvalFunction executable function #[]

/-- Canonical M1 meaning of a validated function. -/
def meaning {unit : ValidatedUnit} (executable : ExecutableUnit unit)
    (function : FunctionHandle) : FunctionMeaning executable function :=
  { relates := EvalFunction executable function #[] }

end BigStep
end LeanerIR
