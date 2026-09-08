-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.NativeOperands
import LeanerLang.Typed
import LeanerLang.NativeData
import LeanerLang.NativeCopy
import LeanerIR.Proofs.NativeVectorAgreement

/-! Shared checked indexing keeps the profile's bounds failure and accesses
native array elements. Only its exact agreement mentions runtime locations. -/

namespace LeanerLang.NativeIndex
open Lean Meta Elab Command
open LeanerIR.Proofs.Denotation
set_option quotPrecheck false

-- Normalize only data equations; native computations and callee bodies stay
-- folded. This also prunes impossible constant-index branches immediately.
open Lean Parser Tactic in
macro "leaner_native_index_data" : tactic =>
  `(tactic| simp_all (config := { failIfUnchanged := false }) only
    [lir_data_norm, LeanerIR.Proofs.NativeVector.field_encode,
      Array.size_map, List.length_cons, List.length_nil, List.size_toArray,
      List.getElem_toArray, List.getElem_cons_zero, List.getElem_cons_succ,
      Int.reduceToNat, Int.reduceLT, Int.reduceLE, Nat.reduceLT, Nat.reduceAdd,
      and_true, true_and, false_and, and_false, not_true_eq_false, not_false_eq_true])

-- A callee can describe a vector through its encoded-array equality. Project
-- that fact at this read's index once; do not decode the vector or inspect
-- the callee's computation. Other contract facts are not rewrite material.
elab "leaner_native_index_facts" index:term : tactic => do
  let mut goal ← Lean.Elab.Tactic.getMainGoal
  let tail := (← Lean.Elab.Tactic.getGoals).drop 1
  let candidates ← goal.withContext do
    return (← getLCtx).foldl (init := #[]) fun found declaration =>
      let type := declaration.type.consumeMData
      if !declaration.isImplementationDetail && type.isAppOfArity ``Eq 3 &&
          (type.getArg! 0).isAppOfArity ``Array 1 &&
          ((type.getArg! 0).getArg! 0).isConstOf ``LeanerIR.RuntimeValue then
        found.push declaration.fvarId
      else found
  for candidate in candidates do
    let (next, name) ← goal.withContext do
      let projection ← Lean.Elab.Term.elabTerm (← ``(fun values : Array LeanerIR.RuntimeValue =>
        values[Int.toNat $index]?.getD LeanerIR.RuntimeValue.unit)) none
      let proof ← mkAppM ``congrArg #[projection, mkFVar candidate]
      let name ← mkFreshUserName `nativeIndexFact
      let (_, next) ← (← goal.assert name (← inferType proof) proof).intro1P
      return (next, name)
    goal := next
    Lean.Elab.Tactic.setGoals (goal :: tail)
    Lean.Elab.Tactic.evalTactic (← `(tactic|
      simp (config := { failIfUnchanged := false })
        (disch := (simp only [Array.size_map, Int.ofNat_eq_natCast] <;> omega)) only
        [Array.getElem?_eq_getElem, Array.getElem_map, Option.getD_some,
          Array.size_map, List.getElem?_toArray, List.getElem?_cons_zero,
          List.getElem?_cons_succ, List.getElem?_nil,
          LeanerIR.RuntimeValue.integer.injEq, LeanerIR.RuntimeValue.bool.injEq,
          Int.reduceToNat, Nat.reduceLT] at $(mkIdent name):ident))
    if (← Lean.Elab.Tactic.getGoals).length == tail.length then return
    goal ← Lean.Elab.Tactic.getMainGoal
  Lean.Elab.Tactic.setGoals (goal :: tail)

private partial def index? (expression : Lean.Expr) : Option Nat :=
  (expression.nat? <|> expression.rawNatLit?).orElse fun _ =>
    if expression.getAppNumArgs == 1 then index? (expression.getArg! 0) else none

def supported (expression : Lean.Expr) : Bool :=
  expression.isAppOfArity ``letNativeValue 3 &&
    (expression.getArg! 1).isAppOfArity ``nativePrimitiveOperation 2 &&
    ((expression.getArg! 1).getArg! 0).isAppOfArity ``PrimitiveLocationOperation.checkVectorIndex 1 &&
    (expression.getArg! 2).isAppOfArity ``nativeIndexedLocalBorrowOperation 2

def emit (slots : Array (Option Term)) (rep : Typed.ValueRep) (expression : Lean.Expr)
    (resultName : Option Ident := none) : CommandElabM NativeOperands.Value := do
  unless supported expression do throwError "native indexed read requires a checked element borrow"
  let binder := expression.getArg! 0
  unless (binder.getArg! 1).isConstOf ``LeanerIR.SemanticOperations.NativePattern.wildcard do
    throwError "native index check must discard its unit result"
  let borrowed := expression.getArg! 2
  unless (borrowed.getArg! 1).isConstOf ``valuesNil do
    throwError "native indexed borrow has unexpected operands"
  let descriptor := borrowed.getArg! 0
  unless descriptor.isAppOfArity ``IndexedLocalBorrowOperation.mk 7 &&
      (descriptor.getArg! 1).isConstOf ``Bool.false &&
      (descriptor.getArg! 4).isConstOf ``LeanerIR.BorrowKind.immutable do
    throwError "native indexed read requires an owned/shared vector, not a mutable reborrow"
  let some localIndex := index? (descriptor.getArg! 0)
    | throwError "native indexed read has an unresolved vector local"
  let some (some values) := slots[localIndex]?
    | throwError "native indexed read vector is unavailable"
  let codec ← rep.codecSyntax (mkIdent `codecs)
  let vectorType ← (Typed.ValueRep.vector rep true).typeSyntax (mkIdent `Carrier)
  let values ← ``(($values : $vectorType))
  let indexLocal := descriptor.getArg! 6
  let (index, indexProof, indexedProof) ← if indexLocal.isAppOfArity ``Option.some 2 then do
      let some position := index? (indexLocal.getArg! 1)
        | throwError "native indexed read has an unresolved index local"
      let some (some indexValue) := slots[position]?
        | throwError "native indexed read index is unavailable"
      let index ← ``(($indexValue).val)
      pure (index,
        ← `(tactic| exact LeanerIR.Proofs.ComputationAgreement.read _ _ _ rfl),
        ← `(tactic| exact LeanerIR.Proofs.NativeVector.indexed_dynamic_shared $codec $values
          _ _ _ $index _ _ _ rfl rfl rfl
          (by change $(Syntax.mkNatLit localIndex) < $(Syntax.mkNatLit slots.size); decide)
          $(mkIdent `indexValid)))
    else do
      unless indexLocal.isAppOfArity ``Option.none 1 do throwError "unresolved native index form"
      let some index := index? (descriptor.getArg! 2) | throwError "unresolved native literal index"
      pure (← ``(($(Syntax.mkNatLit index) : Int)),
        ← `(tactic| exact LeanerIR.Proofs.ComputationAgreement.literal _ _),
        ← `(tactic| exact LeanerIR.Proofs.NativeVector.indexed_shared $codec $values
          _ _ $(Syntax.mkNatLit index) _ _ rfl rfl
          (by change $(Syntax.mkNatLit localIndex) < $(Syntax.mkNatLit slots.size); decide)
          (by
            have h : 0 ≤ ($(Syntax.mkNatLit index) : Int) ∧
              ($(Syntax.mkNatLit index) : Int) < ($values).values.size := $(mkIdent `indexValid):ident
            omega)))
  let check := expression.getArg! 1
  let failure ← liftTermElabM <| PrettyPrinter.delab ((check.getArg! 0).getArg! 0)
  let checkOperands := check.getArg! 1
  unless checkOperands.isAppOfArity ``valuesCons 2 do throwError "native index check is missing operands"
  let (_, vectorProof) ← NativeCopy.emitPure (fun expression => do
    let proof ← if expression.isAppOfArity ``localVar 1 then
        `(tactic| exact LeanerIR.Proofs.ComputationAgreement.read _ _ _ rfl)
      else `(tactic| exact LeanerIR.Proofs.NativeVector.local_read _ _ _ rfl
        (by change $(Syntax.mkNatLit localIndex) < $(Syntax.mkNatLit slots.size); decide))
    return (values, proof)) (checkOperands.getArg! 0)
  let operandsProof ← `(tactic|
    (apply LeanerIR.Proofs.ComputationAgreement.cons
     · $vectorProof:tactic
     · apply LeanerIR.Proofs.ComputationAgreement.cons
       · $indexProof:tactic
       · exact LeanerIR.Proofs.ComputationAgreement.nil _))
  return {
    computation := ← ``(LeanerIR.Proofs.NativeVector.get $values $index
      (LeanerIR.Proofs.NativeVector.indexFailure $failure))
    verifyWith := fun next => do
      let normal ← if let some name := resultName then `(tactic|
          (intro $(mkIdent `indexValid):ident
           have $(mkIdent `nativeIndexBounds):ident := $(mkIdent `indexValid):ident
           simp (config := { failIfUnchanged := false }) at $(mkIdent `nativeIndexBounds):ident
           all_goals
             (intro $name:ident $(mkIdent `valueEquation):ident
              leaner_native_index_facts $index
              leaner_native_index_data <;> $next:tactic)))
        else `(tactic|
          (intro $(mkIdent `indexValid):ident
           have $(mkIdent `nativeIndexBounds):ident := $(mkIdent `indexValid):ident
           simp (config := { failIfUnchanged := false }) at $(mkIdent `nativeIndexBounds):ident
           all_goals
             (leaner_native_index_facts $index
              leaner_native_index_data <;> $next:tactic)))
      let rule := mkIdent (if resultName.isSome then ``LeanerIR.Proofs.NativeVector.wp_get_value
        else ``LeanerIR.Proofs.NativeVector.wp_get)
      `(tactic|
        (rw [$rule:ident]
         constructor
         · $normal:tactic
         · intro $(mkIdent `indexInvalid):ident
           leaner_native_index_data <;>
             simp (config := { failIfUnchanged := false }) only
               [LeanerIR.Proofs.NativeVector.indexFailure] <;> leaner_certified_close!))
    preserves := ← `(tactic| exact LeanerIR.Proofs.NativeVector.get_state _ _ _)
    agreement := ← `(tactic|
      (unfold LeanerIR.Proofs.NativeVector.get
       apply LeanerIR.Proofs.ComputationAgreement.scalar_checked_value
       · intro $(mkIdent `indexValid):ident
         apply LeanerIR.Proofs.ComputationAgreement.operation_value
         · $operandsProof:tactic
         · intro state
           exact LeanerIR.Proofs.NativeVector.check_index_success $codec $values $index $failure _ _
             $(mkIdent `indexValid)
       · intro $(mkIdent `indexInvalid):ident
         apply LeanerIR.Proofs.ComputationAgreement.operation_abort
         · $operandsProof:tactic
         · intro state
           exact LeanerIR.Proofs.NativeVector.check_index_failure $codec $values $index $failure _ _
             $(mkIdent `indexInvalid)
       · rfl
       · intro $(mkIdent `indexValid):ident
         $indexedProof:tactic)) }

end LeanerLang.NativeIndex
