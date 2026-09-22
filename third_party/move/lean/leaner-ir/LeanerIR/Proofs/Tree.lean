-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.LoopSpec

/-!
# The body tree

A generated body is one literal tree: the native combinators a body is
built from, as data.  Its relational denotation `denote` is the combinator
term the generator used to emit directly; its computation `compute` is the
same tree read through the `RowSpec` combinators.  Agreement between the
two is proved once, by induction over the tree, so a generated function
carries no agreement proof of its own — the theorem instantiated at its
literal tree is the proof, and the tree's support is decided by `rfl`.

Calls retain their relation at a modular boundary; the composition route
consumes a proved callee contract there. Loops retain their finite execution
relation at an invariant boundary; their bodies normalize through the same
computation as straight-line code. Return, break, continue, and assignment
propagate control compositionally.
-/

namespace LeanerIR.Proofs.Denotation

open LeanerIR LeanerIR.Validation LeanerIR.SemanticOperations

mutual
/-- A body, as the combinators that denote it. -/
inductive Tree where
  | value (runtimeValue : RuntimeValue)
  | localVar (localId : LocalId)
  | primitive (operation : PrimitiveLocationOperation) (operands : Operands)
  | global (operation : GlobalLocationOperation) (operands : Operands)
  | local_ (operation : LocalLocationOperation) (operands : Operands)
  | derefLocalBorrow (operation : DerefLocalBorrowOperation) (operands : Operands)
  | indexedLocalBorrow (operation : IndexedLocalBorrowOperation) (operands : Operands)
  | indexedLocalFieldBorrow (operation : IndexedLocalFieldBorrowOperation)
      (operands : Operands)
  | reference (operation : ReferenceLocationOperation) (operands : Operands)
  | field (location : NominalFieldLocation) (operands : Operands)
  | variantField (location : NominalVariantFieldLocation) (operands : Operands)
  | variantTest (test : NominalVariantTest) (operands : Operands)
  | constructor (constructor : NominalConstructor) (operands : Operands)
  | call (handle : FunctionHandle) (lexical : Option Nat) (callee : FunctionDenotation)
      (operands : Operands)
  | callAt (handle : FunctionHandle) (lexical : Option Nat)
      (callee : Array (TypeId × TypeId) → FunctionDenotation) (operands : Operands)
  | branchNone (condition thenBranch : Tree)
  | branchSome (condition thenBranch elseBranch : Tree)
  | return_ (values : Operands)
  | throw_ (kind : ThrowKind) (arguments : Operands)
  | loop (site : ExprId) (body : Tree)
  | break_ (nest : Nat)
  | continue_ (nest : Nat)
  | assignLocal (localId : LocalId) (value : Tree)
  | assignLocalIndex (base index : LocalId) (value : Tree)
  | spec
  | blockUnit (statements : Statements)
  | blockResult (statements : Statements) (result : Tree)
  | letNoValue (body : Tree)
  | letValue (binder : NativePatternBinder) (initializer body : Tree)

/-- An operand row. -/
inductive Operands where
  | nil
  | cons (head : Tree) (tail : Operands)

/-- A statement row. -/
inductive Statements where
  | nil
  | cons (head : Tree) (tail : Statements)
end

/-! ## The relational reading -/

mutual
/-- The relational denotation of a tree: the combinator term itself. -/
def Tree.denote : Tree → ExprDenotation
  | .value runtimeValue => value runtimeValue
  | .localVar localId => localVar localId
  | .primitive operation operands => nativePrimitiveOperation operation operands.denote
  | .global operation operands => nativeGlobalOperation operation operands.denote
  | .local_ operation operands => nativeLocalOperation operation operands.denote
  | .derefLocalBorrow operation operands =>
      nativeDerefLocalBorrowOperation operation operands.denote
  | .indexedLocalBorrow operation operands =>
      nativeIndexedLocalBorrowOperation operation operands.denote
  | .indexedLocalFieldBorrow operation operands =>
      nativeIndexedLocalFieldBorrowOperation operation operands.denote
  | .reference operation operands => nativeReferenceOperation operation operands.denote
  | .field location operands => nativeOperation location.evaluateSelect? operands.denote
  | .variantField location operands => nativeOperation location.evaluateSelect? operands.denote
  | .variantTest test operands => nativeOperation test.evaluate? operands.denote
  | .constructor constructor operands => nativeOperation constructor.evaluate? operands.denote
  | .call handle lexical callee operands => nativeCall handle lexical callee operands.denote
  | .callAt handle lexical callee operands => nativeCallAt handle lexical callee operands.denote
  | .branchNone condition thenBranch =>
      nativeBranch condition.denote thenBranch.denote none
  | .branchSome condition thenBranch elseBranch =>
      nativeBranch condition.denote thenBranch.denote (some elseBranch.denote)
  | .return_ values => nativeReturn values.denote
  | .throw_ kind arguments => nativeThrow kind arguments.denote
  | .loop site body => nativeLoop site body.denote
  | .break_ nest => nativeBreak nest none
  | .continue_ nest => nativeContinue nest
  | .assignLocal localId value => nativeAssignLocal localId value.denote
  | .assignLocalIndex base index value => nativeAssignLocalIndex base index value.denote
  | .spec => nativeSpec
  | .blockUnit statements => blockUnit statements.denote
  | .blockResult statements result => blockResult statements.denote result.denote
  | .letNoValue body => letNoValue body.denote
  | .letValue binder initializer body =>
      letNativeValue binder initializer.denote body.denote

def Operands.denote : Operands → ValuesDenotation
  | .nil => valuesNil
  | .cons head tail => valuesCons head.denote tail.denote

def Statements.denote : Statements → StatementsDenotation
  | .nil => statementsNil
  | .cons head tail => statementsCons head.denote tail.denote
end

/-! ## The computational reading -/

mutual
/-- The computation of a tree. Loops are discharged by their authored
invariants, never by unfolding the finite execution relation. -/
def Tree.compute : Tree → RowSpec Control
  | .value runtimeValue => RowSpec.value runtimeValue
  | .localVar localId => RowSpec.localVar localId
  | .primitive operation operands => RowSpec.operation operation.evaluate? operands.compute
  | .global operation operands => RowSpec.operation operation.evaluate? operands.compute
  | .local_ operation operands => RowSpec.operation operation.evaluate? operands.compute
  | .derefLocalBorrow operation operands =>
      RowSpec.operation operation.evaluate? operands.compute
  | .indexedLocalBorrow operation operands =>
      RowSpec.operation operation.evaluate? operands.compute
  | .indexedLocalFieldBorrow operation operands =>
      RowSpec.operation operation.evaluate? operands.compute
  | .reference operation operands => RowSpec.operation operation.evaluate? operands.compute
  | .field location operands => RowSpec.operation location.evaluateSelect? operands.compute
  | .variantField location operands => RowSpec.operation location.evaluateSelect? operands.compute
  | .variantTest test operands => RowSpec.operation test.evaluate? operands.compute
  | .constructor constructor operands =>
      RowSpec.operation constructor.evaluate? operands.compute
  | .call _ lexical callee operands => RowSpec.call lexical callee operands.compute
  | .callAt _ lexical callee operands => RowSpec.callAt lexical callee operands.compute
  | .branchNone condition thenBranch =>
      RowSpec.branch condition.compute thenBranch.compute none
  | .branchSome condition thenBranch elseBranch =>
      RowSpec.branch condition.compute thenBranch.compute (some elseBranch.compute)
  | .return_ arguments => RowSpec.return_ arguments.compute
  | .throw_ kind arguments => RowSpec.throw_ kind arguments.compute
  | .loop site body => RowSpec.loop site body.compute
  | .break_ nest => RowSpec.break_ nest
  | .continue_ nest => RowSpec.continue_ nest
  | .assignLocal localId value => RowSpec.assignLocal localId value.compute
  | .assignLocalIndex base index value => RowSpec.assignLocalIndex base index value.compute
  | .spec => RowSpec.value .unit
  | .blockUnit statements => RowSpec.blockUnit statements.compute
  | .blockResult statements result => RowSpec.blockResult statements.compute result.compute
  | .letNoValue body => body.compute
  | .letValue binder initializer body =>
      RowSpec.letValue binder initializer.compute body.compute

def Operands.compute : Operands → RowSpec RowSpec.Values
  | .nil => RowSpec.valuesNil
  | .cons head tail => RowSpec.valuesCons head.compute tail.compute

def Statements.compute : Statements → RowSpec RowSpec.Statements
  | .nil => RowSpec.statementsNil
  | .cons head tail => RowSpec.statementsCons head.compute tail.compute
end

mutual
/-- Whether every node of the tree computes. -/
def Tree.supported : Tree → Bool
  | .value _ => true
  | .localVar _ => true
  | .primitive _ operands => operands.supported
  | .global _ operands => operands.supported
  | .local_ _ operands => operands.supported
  | .derefLocalBorrow _ operands => operands.supported
  | .indexedLocalBorrow _ operands => operands.supported
  | .indexedLocalFieldBorrow _ operands => operands.supported
  | .reference _ operands => operands.supported
  | .field _ operands => operands.supported
  | .variantField _ operands => operands.supported
  | .variantTest _ operands => operands.supported
  | .constructor _ operands => operands.supported
  | .call _ _ _ operands => operands.supported
  | .callAt _ _ _ operands => operands.supported
  | .branchNone condition thenBranch => condition.supported && thenBranch.supported
  | .branchSome condition thenBranch elseBranch =>
      condition.supported && thenBranch.supported && elseBranch.supported
  | .return_ arguments => arguments.supported
  | .throw_ _ arguments => arguments.supported
  | .loop _ body => body.supported
  | .break_ _ => true
  | .continue_ _ => true
  | .assignLocal _ value => value.supported
  | .assignLocalIndex _ _ value => value.supported
  | .spec => true
  | .blockUnit statements => statements.supported
  | .blockResult statements result => statements.supported && result.supported
  | .letNoValue body => body.supported
  | .letValue _ initializer body => initializer.supported && body.supported

def Operands.supported : Operands → Bool
  | .nil => true
  | .cons head tail => head.supported && tail.supported

def Statements.supported : Statements → Bool
  | .nil => true
  | .cons head tail => head.supported && tail.supported
end

/-! ## Agreement and totality, once -/

mutual
theorem Tree.compute_agrees : ∀ tree : Tree, tree.supported = true →
    RowSpec.ExprAgrees tree.compute tree.denote
  | .value runtimeValue, _ => RowSpec.value_agrees runtimeValue
  | .localVar localId, _ => RowSpec.localVar_agrees localId
  | .primitive _ operands, h => RowSpec.operation_agrees (operands.compute_agrees h)
  | .global _ operands, h => RowSpec.operation_agrees (operands.compute_agrees h)
  | .local_ _ operands, h => RowSpec.operation_agrees (operands.compute_agrees h)
  | .derefLocalBorrow _ operands, h => RowSpec.operation_agrees (operands.compute_agrees h)
  | .indexedLocalBorrow _ operands, h => RowSpec.operation_agrees (operands.compute_agrees h)
  | .indexedLocalFieldBorrow _ operands, h =>
      RowSpec.operation_agrees (operands.compute_agrees h)
  | .reference _ operands, h => RowSpec.operation_agrees (operands.compute_agrees h)
  | .field _ operands, h => RowSpec.operation_agrees (operands.compute_agrees h)
  | .variantField _ operands, h => RowSpec.operation_agrees (operands.compute_agrees h)
  | .variantTest _ operands, h => RowSpec.operation_agrees (operands.compute_agrees h)
  | .constructor _ operands, h => RowSpec.operation_agrees (operands.compute_agrees h)
  | .call _ _ _ operands, h => RowSpec.call_agrees (operands.compute_agrees h)
  | .callAt _ _ _ operands, h => RowSpec.callAt_agrees (operands.compute_agrees h)
  | .branchNone condition thenBranch, h => by
      simp only [Tree.supported, Bool.and_eq_true] at h
      exact RowSpec.branchNone_agrees (condition.compute_agrees h.1) (thenBranch.compute_agrees h.2)
  | .branchSome condition thenBranch elseBranch, h => by
      simp only [Tree.supported, Bool.and_eq_true] at h
      exact RowSpec.branchSome_agrees (condition.compute_agrees h.1.1)
        (thenBranch.compute_agrees h.1.2) (elseBranch.compute_agrees h.2)
  | .return_ arguments, h => RowSpec.return_agrees (arguments.compute_agrees h)
  | .throw_ _ arguments, h => RowSpec.throw_agrees (arguments.compute_agrees h)
  | .loop site body, h => RowSpec.loop_agrees site (body.compute_agrees h)
  | .break_ nest, _ => RowSpec.break_agrees nest
  | .continue_ nest, _ => RowSpec.continue_agrees nest
  | .assignLocal _ value, h => RowSpec.assignLocal_agrees (value.compute_agrees h)
  | .assignLocalIndex _ _ value, h =>
      RowSpec.assignLocalIndex_agrees (value.compute_agrees h)
  | .spec, _ => RowSpec.value_agrees .unit
  | .blockUnit statements, h => RowSpec.blockUnit_agrees (statements.compute_agrees h)
  | .blockResult statements result, h => by
      simp only [Tree.supported, Bool.and_eq_true] at h
      exact RowSpec.blockResult_agrees (statements.compute_agrees h.1) (result.compute_agrees h.2)
  | .letNoValue body, h => body.compute_agrees h
  | .letValue _ initializer body, h => by
      simp only [Tree.supported, Bool.and_eq_true] at h
      exact RowSpec.letValue_agrees (initializer.compute_agrees h.1) (body.compute_agrees h.2)

theorem Operands.compute_agrees : ∀ operands : Operands, operands.supported = true →
    RowSpec.ValuesAgrees operands.compute operands.denote
  | .nil, _ => RowSpec.valuesNil_agrees
  | .cons head tail, h => by
      simp only [Operands.supported, Bool.and_eq_true] at h
      exact RowSpec.valuesCons_agrees (head.compute_agrees h.1) (tail.compute_agrees h.2)

theorem Statements.compute_agrees : ∀ statements : Statements, statements.supported = true →
    RowSpec.StatementsAgrees statements.compute statements.denote
  | .nil, _ => RowSpec.statementsNil_agrees
  | .cons head tail, h => by
      simp only [Statements.supported, Bool.and_eq_true] at h
      exact RowSpec.statementsCons_agrees (head.compute_agrees h.1) (tail.compute_agrees h.2)
end

mutual
theorem Tree.compute_total : ∀ tree : Tree, RowSpec.Total tree.compute
  | .value runtimeValue => RowSpec.total_value runtimeValue
  | .localVar localId => RowSpec.total_localVar localId
  | .primitive _ operands => RowSpec.total_operation operands.compute_total
  | .global _ operands => RowSpec.total_operation operands.compute_total
  | .local_ _ operands => RowSpec.total_operation operands.compute_total
  | .derefLocalBorrow _ operands => RowSpec.total_operation operands.compute_total
  | .indexedLocalBorrow _ operands => RowSpec.total_operation operands.compute_total
  | .indexedLocalFieldBorrow _ operands => RowSpec.total_operation operands.compute_total
  | .reference _ operands => RowSpec.total_operation operands.compute_total
  | .field _ operands => RowSpec.total_operation operands.compute_total
  | .variantField _ operands => RowSpec.total_operation operands.compute_total
  | .variantTest _ operands => RowSpec.total_operation operands.compute_total
  | .constructor _ operands => RowSpec.total_operation operands.compute_total
  | .call _ _ _ operands => RowSpec.total_call operands.compute_total
  | .callAt _ _ _ operands => RowSpec.total_callAt operands.compute_total
  | .branchNone condition thenBranch =>
      RowSpec.total_branchNone condition.compute_total thenBranch.compute_total
  | .branchSome condition thenBranch elseBranch =>
      RowSpec.total_branchSome condition.compute_total thenBranch.compute_total
        elseBranch.compute_total
  | .return_ arguments => RowSpec.total_return arguments.compute_total
  | .throw_ _ arguments => RowSpec.total_throw arguments.compute_total
  | .loop site body => RowSpec.total_loop site body.compute
  | .break_ _ => RowSpec.total_pure _
  | .continue_ _ => RowSpec.total_pure _
  | .assignLocal localId value => RowSpec.total_assignLocal localId value.compute_total
  | .assignLocalIndex _ _ value =>
      RowSpec.total_assignLocalIndex value.compute_total
  | .spec => RowSpec.total_value .unit
  | .blockUnit statements => RowSpec.total_blockUnit statements.compute_total
  | .blockResult statements result =>
      RowSpec.total_blockResult statements.compute_total result.compute_total
  | .letNoValue body => body.compute_total
  | .letValue _ initializer body =>
      RowSpec.total_letValue initializer.compute_total body.compute_total

theorem Operands.compute_total : ∀ operands : Operands, RowSpec.Total operands.compute
  | .nil => RowSpec.total_valuesNil
  | .cons head tail => RowSpec.total_valuesCons head.compute_total tail.compute_total

theorem Statements.compute_total : ∀ statements : Statements, RowSpec.Total statements.compute
  | .nil => RowSpec.total_statementsNil
  | .cons head tail => RowSpec.total_statementsCons head.compute_total tail.compute_total
end

end LeanerIR.Proofs.Denotation

namespace LeanerIR.Proofs.Denotation.RowSpec

open LeanerIR LeanerIR.Validation LeanerIR.SemanticOperations

/-- The switch to the computation: a function whose body denotes a
supported tree has the weakest precondition of the tree's computation
over the row state, from the entry row. -/
theorem wp_nativeFunction_treeAt {unit : ExecutableUnit} {shape : FunctionShape}
    {typeInstantiation : Array (TypeId × TypeId)}
    {tree : Tree} {body : ExprDenotation} (denotes : tree.denote = body)
    (supported : tree.supported = true) {arguments : Array RuntimeValue}
    {initial : RuntimeState} {ensures : Array RuntimeValue → RuntimeState → Prop}
    {aborts : Failure → Prop} :
    wp (nativeFunctionAt unit shape typeInstantiation body arguments) ensures aborts initial ↔
      (arguments.size = shape.parameterCount → arguments.size ≤ shape.localCount →
        wp tree.compute
          (fun control final' =>
            ∀ outcome, finishControl? shape.resultCount control = some outcome →
              match outcome with
              | .returned results =>
                  ensures results
                    (finalizeFunctionState unit shape.profile initial final'.state
                      final'.frame outcome)
              | .threw kind thrown => aborts (kind, thrown))
          (fun _ => False)
          (entryStateAt shape arguments typeInstantiation initial)) := by
  subst denotes
  rw [← wp_congr_equiv
    (rowFunctionAt_equiv (typeInstantiation := typeInstantiation)
      (tree.compute_agrees supported) _)]
  exact wp_rowFunctionAt tree.compute_total

theorem wp_nativeFunction_tree {unit : ExecutableUnit} {shape : FunctionShape}
    {tree : Tree} {body : ExprDenotation} (denotes : tree.denote = body)
    (supported : tree.supported = true) {arguments : Array RuntimeValue}
    {initial : RuntimeState} {ensures : Array RuntimeValue → RuntimeState → Prop}
    {aborts : Failure → Prop} :
    wp (nativeFunction unit shape body arguments) ensures aborts initial ↔
      (arguments.size = shape.parameterCount → arguments.size ≤ shape.localCount →
        wp tree.compute
          (fun control final' =>
            ∀ outcome, finishControl? shape.resultCount control = some outcome →
              match outcome with
              | .returned results =>
                  ensures results
                    (finalizeFunctionState unit shape.profile initial final'.state
                      final'.frame outcome)
              | .threw kind thrown => aborts (kind, thrown))
          (fun _ => False)
          (entryState shape arguments initial)) := by
  simpa only [nativeFunction, entryState] using
    (wp_nativeFunction_treeAt (typeInstantiation := #[]) denotes supported)

end LeanerIR.Proofs.Denotation.RowSpec
