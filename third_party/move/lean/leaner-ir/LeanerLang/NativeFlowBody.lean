-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.NativeCondition
import LeanerLang.NativeLoopInfo
import LeanerIR.Proofs.NativeFlowAgreement

namespace LeanerLang.NativeFlowBody

open Lean Meta Elab Command
open LeanerIR.Proofs.Denotation

set_option quotPrecheck false

inductive Kind where
  | normal | continue_ | break_

structure Emitted where
  computation : Term
  verifyWith : (Kind → Term → CommandElabM (TSyntax `tactic)) → CommandElabM (TSyntax `tactic)
  preserves : TSyntax `tactic
  agreement : TSyntax `tactic

abbrev Slots := Array (Option Term)

private partial def index? (expression : Lean.Expr) : Option Nat :=
  (expression.nat? <|> expression.rawNatLit?).orElse fun _ =>
    if expression.getAppNumArgs == 1 then index? (expression.getArg! 0) else none

def headerType (info : NativeLoopInfo.Loop) : CommandElabM Term := do
  let mut type ← ``(Unit)
  for rep in info.representations.reverse do
    type ← ``($(← rep.typeSyntax (mkIdent `Carrier)) × $type)
  return type

def pack (info : NativeLoopInfo.Loop) (slots : Slots) : CommandElabM Term := do
  let mut result ← ``(())
  for slot in info.slots.reverse do
    let some (some value) := slots[slot.index]? | throwError "uninitialized native loop header local {slot.index}"
    result ← ``(($value, $result))
  return result

def unpack (info : NativeLoopInfo.Loop) (slots : Slots) (value : Term) : CommandElabM Slots := do
  let mut slots := slots
  let mut rest := value
  for slot in info.slots do
    slots := slots.set! slot.index (some (← ``(($rest).1)))
    rest ← ``(($rest).2)
  return slots

private def flow (kind : Kind) (value : Term) : CommandElabM Term :=
  match kind with
  | .normal => ``(LeanerIR.Proofs.NativeFlow.Flow.normal $value)
  | .continue_ => ``(LeanerIR.Proofs.NativeFlow.Flow.continue_ $value)
  | .break_ => ``(LeanerIR.Proofs.NativeFlow.Flow.break_ $value)

/-- Emit the loop's statement tree over a typed header product. Ordinary
branch joins share one native continuation; continue and break skip it. -/
partial def emit (fuel : Nat) (info : NativeLoopInfo.Loop)
    (locals : Array Typed.LocalInfo) (slots : Slots)
    (frame : Slots → CommandElabM Term)
    (scalar : Slots → Typed.ValueRep → Lean.Expr → Option Ident → CommandElabM NativeExpression.Emitted)
    (expression : Lean.Expr) : CommandElabM Emitted := do
  let fuel + 1 := fuel | throwError "native loop body exceeds its structural depth"
  let type ← headerType info
  let packed ← pack info slots
  let finish (kind : Kind) (agreement : TSyntax `tactic) : CommandElabM Emitted := do
    return {
      computation := ← ``(LeanerIR.Proofs.Spec.pure $(← flow kind packed))
      verifyWith := fun next => do
        `(tactic| (rw [LeanerIR.Proofs.wp_pure]; $(← next kind packed):tactic))
      preserves := ← `(tactic| exact LeanerIR.Proofs.StatePreserving.pure _)
      agreement }
  if expression.isAppOfArity ``nativeBreak 2 then
    unless index? (expression.getArg! 0) == some 0 &&
        (expression.getArg! 1).isAppOf ``Option.none do
      throwError "native loop body does not yet carry labeled or value-bearing breaks"
    return ← finish .break_ (← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.controlled_pure
       · intro initial finalFrame finalState control; rfl
       · exact LeanerIR.Proofs.ComputationAgreement.flowControl_valid)))
  if expression.isAppOfArity ``nativeContinue 1 then
    unless index? (expression.getArg! 0) == some 0 do
      throwError "native loop body does not yet carry labeled continues"
    return ← finish .continue_ (← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.controlled_pure
       · intro initial finalFrame finalState control; rfl
       · exact LeanerIR.Proofs.ComputationAgreement.flowControl_valid)))
  if (expression.isAppOfArity ``LeanerIR.Proofs.Denotation.value 1 &&
      (expression.getArg! 0).isConstOf ``LeanerIR.RuntimeValue.unit) ||
      expression.isConstOf ``nativeSpec then
    return ← finish .normal (← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.controlled_pure
       · intro initial finalFrame finalState control; rfl
       · exact LeanerIR.Proofs.ComputationAgreement.flowControl_valid)))
  if expression.isAppOfArity ``blockUnit 1 then
    let body := mkApp2 (mkConst ``blockResult) (expression.getArg! 0)
      (mkApp (mkConst ``LeanerIR.Proofs.Denotation.value) (mkConst ``LeanerIR.RuntimeValue.unit))
    let next ← emit fuel info locals slots frame scalar body
    return { next with agreement := ← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.controlled_blockUnit
       $(next.agreement):tactic)) }
  if expression.isAppOfArity ``nativeAssignLocal 2 then
    let body := mkApp (mkConst ``blockUnit)
      (mkApp2 (mkConst ``statementsCons) expression (mkConst ``statementsNil))
    let next ← emit fuel info locals slots frame scalar body
    return { next with agreement := ← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.controlled_assign
       $(next.agreement):tactic)) }
  if expression.isAppOfArity ``blockResult 2 then
    let statements := expression.getArg! 0
    let result := expression.getArg! 1
    if statements.isConstOf ``statementsNil then
      let next ← emit fuel info locals slots frame scalar result
      return { next with agreement := ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.controlled_blockNil
         $(next.agreement):tactic)) }
    unless statements.isAppOfArity ``statementsCons 2 do
      throwError "unsupported native loop statement list"
    let head := statements.getArg! 0
    let tail := mkApp2 (mkConst ``blockResult) (statements.getArg! 1) result
    if head.isAppOfArity ``nativeAssignLocal 2 then
      let binder := mkApp2 (mkConst ``LeanerIR.SemanticOperations.NativePatternBinder.mk)
        (mkNatLit 1) (mkApp (mkConst ``LeanerIR.SemanticOperations.NativePattern.variable) (head.getArg! 0))
      let rebound := mkApp3 (mkConst ``letNativeValue) binder (head.getArg! 1) tail
      let next ← emit fuel info locals slots frame scalar rebound
      return { next with agreement := ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.controlled_blockAssign
         $(next.agreement):tactic)) }
    let head ← emit fuel info locals slots frame scalar head
    let joined := mkIdent (Name.mkSimple s!"loop_join_{fuel}")
    let nextSlots ← unpack info slots ⟨joined.raw⟩
    let next ← emit fuel info locals nextSlots frame scalar tail
    return {
      computation := ← ``(LeanerIR.Proofs.NativeFlow.sequence $(head.computation)
        (fun $joined : $type => $(next.computation)))
      verifyWith := fun post => do
        let nextProof ← next.verifyWith post
        let proof ← head.verifyWith fun kind values => do
          match kind with
          | .normal => `(tactic|
              (let $joined : $type := $values
               change LeanerIR.Proofs.wp $(next.computation) _ _ _
               simp (config := { failIfUnchanged := false }) only [$joined:term, Prod.fst, Prod.snd]
               $nextProof:tactic))
          | _ => `(tactic|
              (change LeanerIR.Proofs.wp (LeanerIR.Proofs.Spec.pure $(← flow kind values)) _ _ _
               rw [LeanerIR.Proofs.wp_pure]
               $(← post kind values):tactic))
        `(tactic|
          (rw [LeanerIR.Proofs.NativeFlow.sequence, LeanerIR.Proofs.wp_bind]
           $proof:tactic))
      preserves := ← `(tactic|
        (apply LeanerIR.Proofs.StatePreserving.bind
         · $(head.preserves):tactic
         · intro flow
           cases flow with
           | normal $joined:ident => $(next.preserves):tactic
           | continue_ _ => exact LeanerIR.Proofs.StatePreserving.pure _
           | break_ _ => exact LeanerIR.Proofs.StatePreserving.pure _))
      agreement := ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.controlled_sequence
           (frame := fun $joined : $type => $(← frame nextSlots))
         · $(head.agreement):tactic
         · intro $joined:ident; $(next.agreement):tactic)) }
  if expression.isAppOfArity ``letNativeValue 3 then
    let binder := expression.getArg! 0
    let pattern := binder.getArg! 1
    unless pattern.isAppOfArity ``LeanerIR.SemanticOperations.NativePattern.variable 1 do
      throwError "native loop local binding requires a typed variable"
    let some index := index? (pattern.getArg! 0) | throwError "invalid native loop binder"
    unless info.slots.any (·.index == index) do
      throwError "native loop body-local availability is not yet represented"
    let rep := locals[index]!.rep
    let localType ← rep.typeSyntax (mkIdent `Carrier)
    let codec ← rep.codecSyntax (mkIdent `codecs)
    let name := mkIdent (Name.mkSimple s!"loop_local_{index}_{fuel}")
    let first ← scalar slots rep (expression.getArg! 1) (some name)
    let nextSlots := slots.set! index (some ⟨name.raw⟩)
    let next ← emit fuel info locals nextSlots frame scalar (expression.getArg! 2)
    return {
      computation := ← ``(LeanerIR.Proofs.Spec.bind $(first.computation)
        (fun $name : $localType => $(next.computation)))
      verifyWith := fun post => do
        let proof ← first.verifyWith (← next.verifyWith post)
        `(tactic| (rw [LeanerIR.Proofs.wp_bind]; $proof:tactic))
      preserves := ← `(tactic|
        (apply LeanerIR.Proofs.StatePreserving.bind
         · $(first.preserves):tactic
         · intro $name:ident; $(next.preserves):tactic))
      agreement := ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.controlled_let
           (encodeHead := fun value : $localType => ($codec).encode value)
           (bound := fun $name : $localType => $(← frame nextSlots))
         · $(first.agreement):tactic
         · intro $name:ident; rfl
         · intro $name:ident; $(next.agreement):tactic
         · exact LeanerIR.Proofs.ComputationAgreement.flowControl_valid)) }
  if expression.isAppOfArity ``nativeBranch 3 then
    let no := expression.getArg! 2
    let no := if no.isAppOf ``Option.some then no.getArg! 1
      else mkApp (mkConst ``LeanerIR.Proofs.Denotation.value) (mkConst ``LeanerIR.RuntimeValue.unit)
    let test ← scalar slots .bool (expression.getArg! 0) none
    let yes ← emit fuel info locals slots frame scalar (expression.getArg! 1)
    let no ← emit fuel info locals slots frame scalar no
    return {
      computation := ← ``(LeanerIR.Proofs.Spec.bind $(test.computation)
        (fun condition : Bool => if condition then $(yes.computation) else $(no.computation)))
      verifyWith := fun post => do
        let yesProof ← yes.verifyWith post
        let noProof ← no.verifyWith post
        let proof ← test.verifyWith (← `(tactic|
          (rw [LeanerIR.Proofs.wp_branch]
           constructor
           · intro branchTrue
             leaner_native_guard branchTrue <;> $yesProof:tactic
           · intro branchFalse
             leaner_native_guard branchFalse <;> $noProof:tactic)))
        `(tactic| (rw [LeanerIR.Proofs.wp_bind]; $proof:tactic))
      preserves := ← `(tactic|
        (apply LeanerIR.Proofs.StatePreserving.bind
         · $(test.preserves):tactic
         · intro condition
           apply LeanerIR.Proofs.StatePreserving.branch condition
           · $(yes.preserves):tactic
           · $(no.preserves):tactic))
      agreement := ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.controlled_branch
         · $(test.agreement):tactic
         · $(yes.agreement):tactic
         · $(no.agreement):tactic)) }
  -- Unit effects include assertions, calls, and explicit aborts. Their scalar
  -- emitter still owns exact failure evaluation and modular call verification.
  let value ← scalar slots .unit expression none
  return {
    computation := ← ``(LeanerIR.Proofs.Spec.bind $(value.computation)
      (fun _ : Unit => LeanerIR.Proofs.Spec.pure $(← flow .normal packed)))
    verifyWith := fun post => do
      let proof ← value.verifyWith (← `(tactic|
        (rw [LeanerIR.Proofs.wp_pure]; $(← post .normal packed):tactic)))
      `(tactic| (rw [LeanerIR.Proofs.wp_bind]; $proof:tactic))
    preserves := ← `(tactic|
      (apply LeanerIR.Proofs.StatePreserving.bind
       · $(value.preserves):tactic
       · intro ignored; exact LeanerIR.Proofs.StatePreserving.pure _))
    agreement := ← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.controlled_map
       · apply LeanerIR.Proofs.ComputationAgreement.controlled_scalar
         $(value.agreement):tactic
       · intro ignored; rfl
       · intro ignored; rfl
       · exact LeanerIR.Proofs.ComputationAgreement.flowControl_valid)) }

end LeanerLang.NativeFlowBody
