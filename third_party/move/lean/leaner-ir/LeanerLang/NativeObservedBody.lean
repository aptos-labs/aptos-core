-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.NativeLocalView

namespace LeanerLang.NativeObservedBody

open Lean Meta Elab Command
open LeanerIR.Proofs.Denotation
open NativeFlowBody (Kind Emitted Slots headerType pack)

set_option quotPrecheck false

private partial def index? (expression : Lean.Expr) : Option Nat :=
  (expression.nat? <|> expression.rawNatLit?).orElse fun _ =>
    if expression.getAppNumArgs == 1 then index? (expression.getArg! 0) else none

private def flow (kind : Kind) (value : Term) : CommandElabM Term :=
  match kind with
  | .normal => ``(LeanerIR.Proofs.NativeFlow.Flow.normal $value)
  | .continue_ => ``(LeanerIR.Proofs.NativeFlow.Flow.continue_ $value)
  | .break_ => ``(LeanerIR.Proofs.NativeFlow.Flow.break_ $value)

/-- Emit the loop's statement tree over a typed header product. Ordinary
branch joins share one native continuation; continue and break skip it. -/
partial def emit (fuel : Nat) (loop info : NativeLoopInfo.Loop)
    (view : NativeLocalView.View) (ghosts : Array (Option Term)) (slots : Slots)
    (scalar : Slots → Typed.ValueRep → Lean.Expr → Option Ident → CommandElabM NativeExpression.Emitted)
    (expression : Lean.Expr) : CommandElabM Emitted := do
  let fuel + 1 := fuel | throwError "native loop body exceeds its structural depth"
  let locals := view.locals
  let frame := view.frame ghosts
  let type ← headerType info
  let loopType ← headerType loop
  let packed ← pack info slots
  let loopPacked ← pack loop slots
  let finish (kind : Kind) (agreement : TSyntax `tactic) : CommandElabM Emitted := do
    let packed := match kind with | .normal => packed | _ => loopPacked
    return {
      computation := ← ``(LeanerIR.Proofs.Spec.pure
        ($(← flow kind packed) : LeanerIR.Proofs.NativeFlow.Flow $type $loopType))
      verifyWith := fun next => do
        `(tactic| (rw [LeanerIR.Proofs.wp_pure]; $(← next kind packed):tactic))
      preserves := ← `(tactic| exact LeanerIR.Proofs.StatePreserving.pure _)
      agreement := ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.observed_of_controlled
           (frame := fun _ => $(← frame slots))
         · $agreement:tactic
         · rintro initial value final ⟨sameValue, sameState⟩
           cases sameValue
           cases sameState
           exact ⟨$(← view.packGhosts ghosts slots), rfl⟩)) }
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
    let next ← emit fuel loop info view ghosts slots scalar body
    return { next with agreement := ← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.observed_blockUnit
       $(next.agreement):tactic)) }
  if expression.isAppOfArity ``nativeAssignLocal 2 then
    let body := mkApp (mkConst ``blockUnit)
      (mkApp2 (mkConst ``statementsCons) expression (mkConst ``statementsNil))
    let next ← emit fuel loop info view ghosts slots scalar body
    return { next with agreement := ← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.observed_assign
       $(next.agreement):tactic)) }
  if expression.isAppOfArity ``blockResult 2 then
    let statements := expression.getArg! 0
    let result := expression.getArg! 1
    if statements.isConstOf ``statementsNil then
      let next ← emit fuel loop info view ghosts slots scalar result
      return { next with agreement := ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.observed_blockNil
         $(next.agreement):tactic)) }
    unless statements.isAppOfArity ``statementsCons 2 do
      throwError "unsupported native loop statement list"
    let head := statements.getArg! 0
    let tail := mkApp2 (mkConst ``blockResult) (statements.getArg! 1) result
    if head.isAppOfArity ``nativeAssignLocal 2 then
      let binder := mkApp2 (mkConst ``LeanerIR.SemanticOperations.NativePatternBinder.mk)
        (mkNatLit 1) (mkApp (mkConst ``LeanerIR.SemanticOperations.NativePattern.variable) (head.getArg! 0))
      let rebound := mkApp3 (mkConst ``letNativeValue) binder (head.getArg! 1) tail
      let next ← emit fuel loop info view ghosts slots scalar rebound
      return { next with agreement := ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.observed_blockAssign
         $(next.agreement):tactic)) }
    let join := view.joinInfo loop slots
    let joinType ← headerType join
    let head ← emit fuel loop join view ghosts slots scalar head
    let joined := mkIdent (Name.mkSimple s!"loop_join_{fuel}")
    let dead := mkIdent (Name.mkSimple s!"loop_join_dead_{fuel}")
    let nextGhosts ← view.unpackGhosts ⟨dead.raw⟩
    let nextSlots ← view.slots join ⟨joined.raw⟩
    let next ← emit fuel loop info view nextGhosts nextSlots scalar tail
    return {
      computation := ← ``(LeanerIR.Proofs.NativeFlow.sequence $(head.computation)
        (fun $joined : $joinType => $(next.computation)))
      verifyWith := fun post => do
        let nextProof ← next.verifyWith post
        let proof ← head.verifyWith fun kind values => do
          match kind with
          | .normal => `(tactic|
              (let $joined : $joinType := $values
               change LeanerIR.Proofs.wp $(next.computation) _ _ _
               simp (config := { failIfUnchanged := false }) only [$joined:term, Prod.fst, Prod.snd]
               $nextProof:tactic))
          | _ => `(tactic|
              (change LeanerIR.Proofs.wp (LeanerIR.Proofs.Spec.pure
                 ($(← flow kind values) : LeanerIR.Proofs.NativeFlow.Flow $type $loopType)) _ _ _
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
        (apply LeanerIR.Proofs.ComputationAgreement.observed_sequence
           (joinFrames := $(← view.frames join))
           (frames := $(← view.frames info))
           (loopFrames := $(← view.frames loop))
         · $(head.agreement):tactic
         · intro $joined:ident observed_frame
           rintro ⟨$dead:ident, sameFrame⟩
           subst observed_frame
           $(next.agreement):tactic)) }
  if expression.isAppOfArity ``letNativeValue 3 then
    let binder := expression.getArg! 0
    let pattern := binder.getArg! 1
    unless pattern.isAppOfArity ``LeanerIR.SemanticOperations.NativePattern.variable 1 do
      throwError "native loop local binding requires a typed variable"
    let some index := index? (pattern.getArg! 0) | throwError "invalid native loop binder"
    unless locals[index]!.kind == .plain do
      throwError "native observed body-local binding requires an owned value"
    let rep := locals[index]!.rep
    let localType ← rep.typeSyntax (mkIdent `Carrier)
    let codec ← rep.codecSyntax (mkIdent `codecs)
    let name := mkIdent (Name.mkSimple s!"loop_local_{index}_{fuel}")
    let first ← scalar slots rep (expression.getArg! 1) (some name)
    let nextSlots := slots.set! index (some ⟨name.raw⟩)
    let next ← emit fuel loop info view ghosts nextSlots scalar (expression.getArg! 2)
    return {
      computation := ← ``(LeanerIR.Proofs.Spec.bind $(first.computation)
        (fun $name : $localType => $(next.computation)))
      verifyWith := fun post => do
        let mut after ← next.verifyWith post
        if let .int .. := rep then
          if let `(LeanerIR.Proofs.Spec.pure $value) := first.computation then
            after ← `(tactic|
              (have loopAlias : LeanerIR.SpecInt.val $name = LeanerIR.SpecInt.val $value := by rfl
               $after:tactic))
        let proof ← first.verifyWith after
        `(tactic| (rw [LeanerIR.Proofs.wp_bind]; $proof:tactic))
      preserves := ← `(tactic|
        (apply LeanerIR.Proofs.StatePreserving.bind
         · $(first.preserves):tactic
         · intro $name:ident; $(next.preserves):tactic))
      agreement := ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.observed_let
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
    let yes ← emit fuel loop info view ghosts slots scalar (expression.getArg! 1)
    let no ← emit fuel loop info view ghosts slots scalar no
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
        (apply LeanerIR.Proofs.ComputationAgreement.observed_branch
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
      (apply LeanerIR.Proofs.ComputationAgreement.observed_of_controlled
         (frame := fun _ => $(← frame slots))
       · apply LeanerIR.Proofs.ComputationAgreement.controlled_map
         · apply LeanerIR.Proofs.ComputationAgreement.controlled_scalar
           $(value.agreement):tactic
         · intro ignored; rfl
         · intro ignored; rfl
         · exact LeanerIR.Proofs.ComputationAgreement.flowControl_valid
       · rintro initial value final ⟨ignored, middle, ran, sameValue, sameState⟩
         cases sameValue
         cases sameState
         exact ⟨$(← view.packGhosts ghosts slots), rfl⟩)) }

end LeanerLang.NativeObservedBody
