-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.NativeLocalView
import LeanerIR.Proofs.NativeReturnAgreement

namespace LeanerLang.NativeNestedBody

open Lean Meta Elab Command
open LeanerIR.Proofs.Denotation
open NativeFlowBody (Slots headerType pack)

inductive Kind where
  | normal | continue_ (depth : Nat) | break_ (depth : Nat) | return_

structure Emitted where
  computation : Term
  verifyWith : (Kind → Term → CommandElabM (TSyntax `tactic)) → CommandElabM (TSyntax `tactic)
  preserves : TSyntax `tactic
  agreement : TSyntax `tactic

/-- Return payloads stay native; encoding and borrow checks are proof-only. -/
structure Returns where
  rep : Typed.ValueRep
  codec : Term
  borrowFree : Array (Option Term) → Slots → CommandElabM (TSyntax `tactic)

set_option quotPrecheck false

private partial def index? (expression : Lean.Expr) : Option Nat :=
  (expression.nat? <|> expression.rawNatLit?).orElse fun _ =>
    if expression.getAppNumArgs == 1 then index? (expression.getArg! 0) else none

private def targetType (returns : Option Returns) : List NativeLoopInfo.Loop → CommandElabM Term
  | [] => match returns with
    | some result => result.rep.typeSyntax (mkIdent `Carrier)
    | none => throwError "native control requires an enclosing loop"
  | [loop] => if returns.isNone then headerType loop else do
    ``(Sum $(← headerType loop) $(← targetType returns []))
  | loop :: rest => do
    ``(Sum $(← headerType loop) $(← targetType returns rest))

private def targetRoute (returns : Option Returns) : List NativeLoopInfo.Loop → CommandElabM Term
  | [] => match returns with
    | some result => ``(LeanerIR.Proofs.ComputationAgreement.ControlRoute.returned ($(result.codec)).encode)
    | none => throwError "native control requires an enclosing loop"
  | loop :: rest => do
    if rest.isEmpty && returns.isNone then
      ``((LeanerIR.Proofs.ComputationAgreement.ControlRoute.root :
        LeanerIR.Proofs.ComputationAgreement.ControlRoute $(← headerType loop)))
    else
      ``((($(← targetRoute returns rest)).push :
        LeanerIR.Proofs.ComputationAgreement.ControlRoute
          (Sum $(← headerType loop) $(← targetType returns rest))))

private def targetFrames (returns : Option Returns) (view : NativeLocalView.View) :
    List NativeLoopInfo.Loop → CommandElabM Term
  | [] => match returns with
    | some result => do
      ``(fun (_ : $(← result.rep.typeSyntax (mkIdent `Carrier))) (frame : LeanerIR.RuntimeFrame) =>
        LeanerIR.SemanticOperations.frameBorrows frame = #[])
    | none => throwError "native control requires an enclosing loop"
  | loop :: rest => do
    if rest.isEmpty && returns.isNone then view.frames loop
    else ``(LeanerIR.Proofs.ComputationAgreement.sumFrames
      $(← view.frames loop) $(← targetFrames returns view rest))

private def inject (returns : Option Returns) (stack : List NativeLoopInfo.Loop)
    (depth : Nat) (value : Term) : CommandElabM Term := do
  match stack, depth with
  | [], 0 => if returns.isSome then pure value else
      throwError "native control target exceeds the enclosing loops"
  | [], _ => throwError "native control target exceeds the enclosing loops"
  | [_], 0 => if returns.isNone then pure value else ``(Sum.inl $value)
  | _ :: _, 0 => ``(Sum.inl $value)
  | _ :: rest, depth + 1 => ``(Sum.inr $(← inject returns rest depth value))

private def flow (returns : Option Returns) (stack : List NativeLoopInfo.Loop)
    (kind : Kind) (value : Term) : CommandElabM Term := do
  match kind with
  | .normal => ``(LeanerIR.Proofs.NativeFlow.Flow.normal $value)
  | .continue_ depth => ``(LeanerIR.Proofs.NativeFlow.Flow.continue_ $(← inject returns stack depth value))
  | .break_ depth => ``(LeanerIR.Proofs.NativeFlow.Flow.break_ $(← inject returns stack depth value))
  | .return_ => ``(LeanerIR.Proofs.NativeFlow.Flow.break_ $(← inject returns stack stack.length value))

/-- Emit the loop's statement tree over a typed header product. Ordinary
branch joins share one native continuation; continue and break skip it. -/
partial def emit (returns : Option Returns) (fuel : Nat) (loops : Array NativeLoopInfo.Loop) (args : Term)
    (stack : List NativeLoopInfo.Loop) (info : NativeLoopInfo.Loop)
    (view : NativeLocalView.View) (ghosts : Array (Option Term)) (slots : Slots)
    (scalar : Slots → Typed.ValueRep → Lean.Expr → Option Ident → CommandElabM NativeExpression.Emitted)
    (expression : Lean.Expr) : CommandElabM Emitted := do
  let fuel + 1 := fuel | throwError "native loop body exceeds its structural depth"
  let loop := stack.headD info
  let route ← targetRoute returns stack
  let locals := view.locals
  let frame := view.frame ghosts
  let type ← headerType info
  let loopType ← targetType returns stack
  let packed ← pack info slots
  let finish (kind : Kind) (agreement : TSyntax `tactic) : CommandElabM Emitted := do
    let packed ← match kind with
      | .normal => pure packed
      | .return_ => throwError "return payload must be evaluated before control emission"
      | .continue_ depth | .break_ depth =>
        let some target := stack[depth]? | throwError "native control target is out of range"
        pack target slots
    return {
      computation := ← ``(LeanerIR.Proofs.Spec.pure
        ($(← flow returns stack kind packed) : LeanerIR.Proofs.NativeFlow.Flow $type $loopType))
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
  if expression.isAppOfArity ``nativeReturn 1 then
    let some returns := returns | throwError "native return requires a function boundary"
    let resultType ← returns.rep.typeSyntax (mkIdent `Carrier)
    let operands := expression.getArg! 0
    let name := mkIdent (Name.mkSimple s!"returned_value_{fuel}")
    let value ← if operands.isConstOf ``valuesNil then do
        unless returns.rep == .unit do throwError "native return is missing its result"
        pure ({ computation := ← ``(LeanerIR.Proofs.Spec.pure ())
                verifyWith := fun next => `(tactic|
                  (rw [LeanerIR.Proofs.wp_pure]; let $name : Unit := (); $next:tactic))
                preserves := ← `(tactic| exact LeanerIR.Proofs.StatePreserving.pure _)
                agreement := ← `(tactic| exact LeanerIR.Proofs.ComputationAgreement.operands_nil _) } :
          NativeExpression.Emitted)
      else do
        unless operands.isAppOfArity ``valuesCons 2 && (operands.getArg! 1).isConstOf ``valuesNil do
          throwError "native return requires a single owned result"
        let value ← scalar slots returns.rep (operands.getArg! 0) (some name)
        pure { value with agreement := ← `(tactic|
          (apply LeanerIR.Proofs.ComputationAgreement.operands_single
           $(value.agreement):tactic)) }
    let returned ← flow (some returns) stack .return_ ⟨name.raw⟩
    return {
      computation := ← ``(LeanerIR.Proofs.Spec.bind $(value.computation)
        (fun $name : $resultType => LeanerIR.Proofs.Spec.pure
          ($returned : LeanerIR.Proofs.NativeFlow.Flow $type $loopType)))
      verifyWith := fun post => do
        if let `(LeanerIR.Proofs.Spec.pure $original) := value.computation then
          return ← `(tactic|
            (rw [LeanerIR.Proofs.wp_bind, LeanerIR.Proofs.wp_pure, LeanerIR.Proofs.wp_pure]
             $(← post .return_ original):tactic))
        let after ← `(tactic|
          (rw [LeanerIR.Proofs.wp_pure]; $(← post .return_ ⟨name.raw⟩):tactic))
        let proof ← value.verifyWith after
        `(tactic| (rw [LeanerIR.Proofs.wp_bind]; $proof:tactic))
      preserves := ← `(tactic|
        (apply LeanerIR.Proofs.StatePreserving.bind
         · $(value.preserves):tactic
         · intro $name:ident; exact LeanerIR.Proofs.StatePreserving.pure _))
      agreement := ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.observed_of_controlled
           (frame := fun _ => $(← frame slots))
         · apply LeanerIR.Proofs.ComputationAgreement.controlled_map
           · apply LeanerIR.Proofs.ComputationAgreement.controlled_return
             $(value.agreement):tactic
           · intro ignored; rfl
           · intro ignored; rfl
           · exact ($route).valid
         · rintro initial value final ⟨returned, middle, ran, sameValue, sameState⟩
           cases sameValue
           cases sameState
           change LeanerIR.SemanticOperations.frameBorrows $(← frame slots) = #[]
           $(← returns.borrowFree ghosts slots):tactic)) }
  if expression.isAppOfArity ``nativeLoop 2 then
    let some site := index? (expression.getArg! 0) | throwError "invalid nested loop site"
    let some inner := loops.find? (·.site.index == site)
      | throwError "nested loop has no typed invariant"
    let innerType ← headerType inner
    let initial ← pack inner slots
    let current := mkIdent (Name.mkSimple s!"nested_locals_{fuel}")
    let currentDead := mkIdent (Name.mkSimple s!"nested_dead_{fuel}")
    let currentSlots ← view.slots inner current
    let currentGhosts ← view.unpackGhosts currentDead
    let body ← emit returns fuel loops args (inner :: stack) inner view currentGhosts currentSlots scalar
      (expression.getArg! 1)
    let invariant ← ``($(mkIdent inner.predicate) $args $initial)
    let iteration ← ``(fun $current : $innerType =>
      LeanerIR.Proofs.NativeNestedFlow.iteration $(body.computation))
    let computation ← ``(LeanerIR.Proofs.NativeLoop.run $iteration $initial $invariant)
    let exited := mkIdent (Name.mkSimple s!"nested_exit_{fuel}")
    let exitDead := mkIdent (Name.mkSimple s!"nested_exit_dead_{fuel}")
    let exitSlots ← view.slots inner exited
    let projected ← pack info exitSlots
    let exitGhosts ← view.unpackGhosts exitDead
    let mapping ← ``(fun result : LeanerIR.Proofs.NativeFlow.Flow $innerType $loopType =>
      (match result with
      | .normal $exited => LeanerIR.Proofs.NativeFlow.Flow.normal $projected
      | .continue_ target => LeanerIR.Proofs.NativeFlow.Flow.continue_ target
      | .break_ target => LeanerIR.Proofs.NativeFlow.Flow.break_ target :
        LeanerIR.Proofs.NativeFlow.Flow $type $loopType))
    let recursive := mkIdent (Name.mkSimple s!"nested_recursive_{fuel}")
    let hypothesis := mkIdent (Name.mkSimple s!"nested_hypothesis_{fuel}")
    let holds := mkIdent (Name.mkSimple s!"nested_invariant_{fuel}")
    return {
      computation := ← ``(LeanerIR.Proofs.Spec.bind $computation
        (fun result => LeanerIR.Proofs.Spec.pure ($mapping result)))
      verifyWith := fun post => do
        let stepProof ← body.verifyWith fun kind values => do
          match kind with
          | .normal | .continue_ 0 => `(tactic|
              (change LeanerIR.Proofs.wp ($recursive $values) _ _ _
               apply $hypothesis
               simp (config := { failIfUnchanged := false }) only
                 [$(mkIdent inner.predicate):term, Prod.fst, Prod.snd]
               leaner_certified_close!))
          | .break_ 0 => do
            let projected ← pack info (← view.slots inner values)
            `(tactic|
              (change LeanerIR.Proofs.wp (LeanerIR.Proofs.Spec.pure
                 (LeanerIR.Proofs.NativeFlow.Flow.normal $values)) _ _ _
               rw [LeanerIR.Proofs.wp_pure]
               change LeanerIR.Proofs.wp (LeanerIR.Proofs.Spec.pure
                 ($(← flow returns stack .normal projected) : LeanerIR.Proofs.NativeFlow.Flow $type $loopType)) _ _ _
               rw [LeanerIR.Proofs.wp_pure]
               $(← post .normal projected):tactic))
          | .continue_ (_ + 1) | .break_ (_ + 1) | .return_ => do
            let outer := match kind with
              | .continue_ (depth + 1) => Kind.continue_ depth
              | .break_ (depth + 1) => Kind.break_ depth
              | _ => Kind.return_
            `(tactic|
              (change LeanerIR.Proofs.wp (LeanerIR.Proofs.Spec.pure
                 ($(← flow returns stack outer values) : LeanerIR.Proofs.NativeFlow.Flow $innerType $loopType)) _ _ _
               rw [LeanerIR.Proofs.wp_pure]
               change LeanerIR.Proofs.wp (LeanerIR.Proofs.Spec.pure
                 ($(← flow returns stack outer values) : LeanerIR.Proofs.NativeFlow.Flow $type $loopType)) _ _ _
               rw [LeanerIR.Proofs.wp_pure]
               $(← post outer values):tactic))
        let mut iterationProof ← `(tactic|
          (rw [LeanerIR.Proofs.NativeLoop.body, LeanerIR.Proofs.wp_bind,
             LeanerIR.Proofs.NativeNestedFlow.wp_iteration]
           $stepProof:tactic))
        for (slot, rep) in (inner.slots.zip inner.representations).reverse do
          if let .int _ signed := rep then
            let some (some value) := currentSlots[slot.index]? | throwError "missing nested loop local"
            let rule := mkIdent (if signed then ``LeanerIR.IntegerValueFits.signed_bounds
              else ``LeanerIR.IntegerValueFits.unsigned_bounds)
            iterationProof ← `(tactic|
              (have loopBounds := $rule (LeanerIR.SpecInt.fits $value)
               leaner_cases loopBounds
               $iterationProof:tactic))
        `(tactic|
          (rw [LeanerIR.Proofs.wp_bind]
           apply LeanerIR.Proofs.wp_withInvariant_fix_frame
           · simp (config := { failIfUnchanged := false }) only
               [$(mkIdent inner.predicate):term, Prod.fst, Prod.snd]
             leaner_certified_close!
           · intro $recursive:ident $hypothesis:ident $current:ident $holds:ident
             simp (config := { failIfUnchanged := false }) only
               [$(mkIdent inner.predicate):term, Prod.fst, Prod.snd] at $holds:ident
             leaner_cases $holds:ident
             $iterationProof:tactic))
      preserves := ← `(tactic|
        (apply LeanerIR.Proofs.StatePreserving.bind
         · apply LeanerIR.Proofs.NativeLoop.preserves
           intro $current:ident
           apply LeanerIR.Proofs.NativeNestedFlow.iteration_preserves
           $(body.preserves):tactic
         · intro result; exact LeanerIR.Proofs.StatePreserving.pure _))
      agreement := ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.observed_map $mapping
           (computation := $computation)
           (frames := LeanerIR.Proofs.ComputationAgreement.flowFrames
             $(← view.frames inner) $(← targetFrames returns view stack))
           (encode := ($route).encode)
         · apply LeanerIR.Proofs.ComputationAgreement.observed_nested_loop
             (route := $route)
           · exact ⟨$(← view.packGhosts ghosts slots), rfl⟩
           · intro $current:ident observed_frame
             rintro ⟨$currentDead:ident, sameFrame⟩
             subst observed_frame
             $(body.agreement):tactic
         · intro result; cases result <;> rfl
         · intro result observed_frame related
           cases result with
           | normal $exited:ident =>
             obtain ⟨$exitDead:ident, sameFrame⟩ := related
             subst observed_frame
             exact ⟨$(← view.packGhosts exitGhosts exitSlots), rfl⟩
           | continue_ _ => exact related
           | break_ _ => exact related
         · exact ($route).valid)) }
  if expression.isAppOfArity ``nativeBreak 2 then
    let some depth := index? (expression.getArg! 0) | throwError "invalid native break depth"
    unless (expression.getArg! 1).isAppOf ``Option.none do
      throwError "native loop body does not yet carry value-bearing breaks"
    return ← finish (.break_ depth) (← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.controlled_pure
       · intro initial finalFrame finalState control; rfl
       · exact ($route).valid)))
  if expression.isAppOfArity ``nativeContinue 1 then
    let some depth := index? (expression.getArg! 0) | throwError "invalid native continue depth"
    return ← finish (.continue_ depth) (← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.controlled_pure
       · intro initial finalFrame finalState control; rfl
       · exact ($route).valid)))
  if (expression.isAppOfArity ``LeanerIR.Proofs.Denotation.value 1 &&
      (expression.getArg! 0).isConstOf ``LeanerIR.RuntimeValue.unit) ||
      expression.isConstOf ``nativeSpec then
    return ← finish .normal (← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.controlled_pure
       · intro initial finalFrame finalState control; rfl
       · exact ($route).valid)))
  if expression.isAppOfArity ``blockUnit 1 then
    let body := mkApp2 (mkConst ``blockResult) (expression.getArg! 0)
      (mkApp (mkConst ``LeanerIR.Proofs.Denotation.value) (mkConst ``LeanerIR.RuntimeValue.unit))
    let next ← emit returns fuel loops args stack info view ghosts slots scalar body
    return { next with agreement := ← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.observed_blockUnit
       $(next.agreement):tactic)) }
  if expression.isAppOfArity ``nativeAssignLocal 2 then
    let body := mkApp (mkConst ``blockUnit)
      (mkApp2 (mkConst ``statementsCons) expression (mkConst ``statementsNil))
    let next ← emit returns fuel loops args stack info view ghosts slots scalar body
    return { next with agreement := ← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.observed_assign
       $(next.agreement):tactic)) }
  if expression.isAppOfArity ``blockResult 2 then
    let statements := expression.getArg! 0
    let result := expression.getArg! 1
    if statements.isConstOf ``statementsNil then
      let next ← emit returns fuel loops args stack info view ghosts slots scalar result
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
      let next ← emit returns fuel loops args stack info view ghosts slots scalar rebound
      return { next with agreement := ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.observed_blockAssign
         $(next.agreement):tactic)) }
    let join := view.joinInfo loop slots
    let joinType ← headerType join
    let head ← emit returns fuel loops args stack join view ghosts slots scalar head
    let joined := mkIdent (Name.mkSimple s!"loop_join_{fuel}")
    let dead := mkIdent (Name.mkSimple s!"loop_join_dead_{fuel}")
    let nextGhosts ← view.unpackGhosts ⟨dead.raw⟩
    let nextSlots ← view.slots join ⟨joined.raw⟩
    let next ← emit returns fuel loops args stack info view nextGhosts nextSlots scalar tail
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
                 ($(← flow returns stack kind values) : LeanerIR.Proofs.NativeFlow.Flow $type $loopType)) _ _ _
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
        (apply LeanerIR.Proofs.ComputationAgreement.observed_routed_sequence $route
           (joinFrames := $(← view.frames join))
           (frames := $(← view.frames info))
           (loopFrames := $(← targetFrames returns view stack))
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
    let next ← emit returns fuel loops args stack info view ghosts nextSlots scalar (expression.getArg! 2)
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
         · exact ($route).valid)) }
  if expression.isAppOfArity ``nativeBranch 3 then
    let no := expression.getArg! 2
    let no := if no.isAppOf ``Option.some then no.getArg! 1
      else mkApp (mkConst ``LeanerIR.Proofs.Denotation.value) (mkConst ``LeanerIR.RuntimeValue.unit)
    let test ← scalar slots .bool (expression.getArg! 0) none
    let yes ← emit returns fuel loops args stack info view ghosts slots scalar (expression.getArg! 1)
    let no ← emit returns fuel loops args stack info view ghosts slots scalar no
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
      (fun _ : Unit => LeanerIR.Proofs.Spec.pure $(← flow returns stack .normal packed)))
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
         · exact ($route).valid
       · rintro initial value final ⟨ignored, middle, ran, sameValue, sameState⟩
         cases sameValue
         cases sameState
         exact ⟨$(← view.packGhosts ghosts slots), rfl⟩)) }

/-- The outermost loop retains the existing compact native iteration and
continuation interface. Only nested bodies need a target sum. -/
def emitRoot (fuel : Nat) (loops : Array NativeLoopInfo.Loop) (args : Term)
    (info : NativeLoopInfo.Loop) (view : NativeLocalView.View)
    (ghosts : Array (Option Term)) (slots : Slots)
    (scalar : Slots → Typed.ValueRep → Lean.Expr → Option Ident → CommandElabM NativeExpression.Emitted)
    (expression : Lean.Expr) : CommandElabM NativeFlowBody.Emitted := do
  let result ← emit none fuel loops args [info] info view ghosts slots scalar expression
  return {
    computation := result.computation
    preserves := result.preserves
    agreement := ← `(tactic|
      (rw [← LeanerIR.Proofs.ComputationAgreement.ControlRoute.root_encode]
       $(result.agreement):tactic))
    verifyWith := fun post => result.verifyWith fun kind value =>
      match kind with
      | .normal => post .normal value
      | .continue_ 0 => post .continue_ value
      | .break_ 0 => post .break_ value
      | _ => throwError "native loop control escapes the outermost loop" }

end LeanerLang.NativeNestedBody
