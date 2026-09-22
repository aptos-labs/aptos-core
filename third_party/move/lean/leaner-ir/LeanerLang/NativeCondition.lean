-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.NativeComparison
import LeanerLang.NativeEffects
import LeanerLang.NativeData
import LeanerIR.Proofs.NativeConditionAgreement

/-! Arithmetic conditions evaluate typed operands before comparing them.
The computational condition can abort; its agreement preserves that behavior. -/

namespace LeanerLang.NativeCondition
open Lean Elab Command
open LeanerIR.Proofs.Denotation
set_option quotPrecheck false

def logical (expression : Lean.Expr) : Bool :=
  expression.isAppOfArity ``nativePrimitiveOperation 2 &&
    [``PrimitiveLocationOperation.logicalAnd, ``PrimitiveLocationOperation.logicalOr,
      ``PrimitiveLocationOperation.logicalNot].contains (expression.getArg! 0).getAppFn.constName!

def supported (expression : Lean.Expr) : Bool :=
  logical expression || (NativeComparison.supported expression && NativeEffects.required expression) ||
    (expression.isAppOfArity ``nativeBranch 3 && (expression.getArg! 2).isAppOf ``Option.some)

private def choice (test yes no : NativeExpression.Emitted) :
    CommandElabM NativeExpression.Emitted := do
  return {
    computation := ← ``(LeanerIR.Proofs.Spec.bind $(test.computation)
      (fun condition : Bool => if condition then $(yes.computation) else $(no.computation)))
    verifyWith := fun next => do
      let yesProof ← yes.verifyWith next
      let noProof ← no.verifyWith next
      let branches ← `(tactic|
        (rw [LeanerIR.Proofs.wp_branch]
         constructor
         · intro $(mkIdent `branchTrue):ident
           leaner_native_guard $(mkIdent `branchTrue):ident <;> $yesProof:tactic
         · intro $(mkIdent `branchFalse):ident
           leaner_native_guard $(mkIdent `branchFalse):ident <;> $noProof:tactic))
      let proof ← test.verifyWith branches
      `(tactic| (rw [LeanerIR.Proofs.wp_bind]; $proof:tactic))
    preserves := ← `(tactic|
      (apply LeanerIR.Proofs.StatePreserving.bind
       · $(test.preserves):tactic
       · intro condition
         apply LeanerIR.Proofs.StatePreserving.branch condition
         · $(yes.preserves):tactic
         · $(no.preserves):tactic))
    agreement := ← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.scalar_branch
       · $(test.agreement):tactic
       · $(yes.agreement):tactic
       · $(no.agreement):tactic)) }

partial def emit (twins : Array SpecTypes.TwinInfo) (slots : Array (Option Term))
    (pureValue : Typed.ValueRep → Lean.Expr → CommandElabM (Term × TSyntax `tactic))
    (integerRep : Typed.ValueRep) (expression : Lean.Expr) (resultName : Option Ident := none) :
    CommandElabM NativeExpression.Emitted := do
  if expression.isAppOfArity ``nativeBranch 3 then
    unless (expression.getArg! 2).isAppOf ``Option.some do
      throwError "native Boolean condition requires an else value"
    return ← choice (← emit twins slots pureValue integerRep (expression.getArg! 0))
      (← emit twins slots pureValue integerRep (expression.getArg! 1) resultName)
      (← emit twins slots pureValue integerRep ((expression.getArg! 2).getArg! 1) resultName)
  unless supported expression do
    return ← NativeEffects.emit twins slots pureValue .bool expression resultName
  let args := mkIdent `operands
  let operator := (expression.getArg! 0).getAppFn.constName!
  let (operands, value) ← if logical expression then do
      let unary := operator == ``PrimitiveLocationOperation.logicalNot
      let fields := if unary then #[Typed.ValueRep.bool] else #[.bool, .bool]
      -- Core Boolean operations are eager: evaluate every operand before
      -- applying the Boolean function, even when its answer is determined.
      let operands ← NativeEffects.emitOperands fields (expression.getArg! 1)
        (fun _ value => emit twins slots pureValue integerRep value)
      let value ← if unary then ``(!$args.1)
        else if operator == ``PrimitiveLocationOperation.logicalAnd then ``($args.1 && $args.2.1)
        else ``($args.1 || $args.2.1)
      pure (operands, value)
    else do
      let .int .. := integerRep | throwError "native arithmetic condition requires typed integer operands"
      let operands ← NativeEffects.emitOperands #[integerRep, integerRep] (expression.getArg! 1)
        (fun rep value => NativeEffects.emit twins slots pureValue rep value)
      let left ← ``(($args.1).val)
      let right ← ``(($args.2.1).val)
      pure (operands, ← NativeComparison.emitValue operator left right)
  return {
    computation := ← ``(LeanerIR.Proofs.Spec.bind $(operands.computation)
      (fun $args : $(operands.type) => LeanerIR.Proofs.Spec.pure $value))
    verifyWith := fun next => do
      let result ← if let some name := resultName then `(tactic|
          (rw [LeanerIR.Proofs.wp_pure_value]
           intro $name:ident $(mkIdent `valueEquation):ident
           $next:tactic))
        else `(tactic| (rw [LeanerIR.Proofs.wp_pure]; $next:tactic))
      let proof ← operands.verifyWith result
      `(tactic| (rw [LeanerIR.Proofs.wp_bind]; $proof:tactic))
    preserves := ← `(tactic|
      (apply LeanerIR.Proofs.StatePreserving.bind
       · $(operands.preserves):tactic
       · intro operands; exact LeanerIR.Proofs.StatePreserving.pure _))
    agreement := ← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.scalar_operation_value
         (encodeArgs := $(operands.encode)) (encodeResult := LeanerIR.RuntimeValue.bool)
         (value := fun $args : $(operands.type) => $value)
       · $(operands.agreement):tactic
       · intro operands state
         simp [LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.bool,
           LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.evaluate?,
           LeanerIR.Proofs.Denotation.liftPrimitiveEvaluator,
           LeanerIR.SemanticOperations.compareOrdered,
           LeanerIR.SemanticOperations.equalValues?, LeanerIR.SemanticOperations.notEqualValues?])) }

private partial def pureChoice
    (pureValue : Typed.ValueRep → Lean.Expr → CommandElabM (Term × TSyntax `tactic))
    (rep : Typed.ValueRep) (expression : Lean.Expr) :
    CommandElabM (Term × TSyntax `tactic) := do
  let ty ← rep.typeSyntax (mkIdent `Carrier)
  let codec ← rep.codecSyntax (mkIdent `codecs)
  let encode ← ``(fun value : $ty => ($codec).encode value)
  if expression.isAppOfArity ``nativeBranch 3 &&
      (expression.getArg! 2).isAppOf ``Option.some then
    let (test, tested) ← pureChoice pureValue .bool (expression.getArg! 0)
    let (yes, left) ← pureChoice pureValue rep (expression.getArg! 1)
    let (no, right) ← pureChoice pureValue rep ((expression.getArg! 2).getArg! 1)
    return (← ``(if $test then $yes else $no), ← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.scalar_branch_pure
       · $tested:tactic
       · $left:tactic
       · $right:tactic)))
  let (value, evaluated) ← pureValue rep expression
  return (value, ← `(tactic|
    (apply LeanerIR.Proofs.ComputationAgreement.scalar_pure $encode $value
     $evaluated:tactic)))

/-- A typed branch result joins before the surrounding continuation. The
continuation is emitted once, independently of the number of branch arms. -/
partial def emitChoice (twins : Array SpecTypes.TwinInfo) (slots : Array (Option Term))
    (pureValue : Typed.ValueRep → Lean.Expr → CommandElabM (Term × TSyntax `tactic))
    (integerRep rep : Typed.ValueRep) (expression : Lean.Expr)
    (resultName : Option Ident := none) : CommandElabM NativeExpression.Emitted := do
  if expression.isAppOfArity ``nativeThrow 2 then
    return ← NativeEffects.emitThrow rep expression
      (fun rep value => emitChoice twins slots pureValue integerRep rep value)
  if expression.isAppOfArity ``blockUnit 1 then
    unless rep == .unit do throwError "native statement block must produce Unit"
    let result := Lean.mkApp (Lean.mkConst ``LeanerIR.Proofs.Denotation.value)
      (Lean.mkConst ``LeanerIR.RuntimeValue.unit)
    let next ← emitChoice twins slots pureValue integerRep rep
      (Lean.mkApp2 (Lean.mkConst ``blockResult) (expression.getArg! 0) result) resultName
    return { next with agreement := ← `(tactic|
      (rw [LeanerIR.Proofs.ComputationAgreement.block_unit_result]
       $(next.agreement):tactic)) }
  if expression.isAppOfArity ``blockResult 2 then
    let statements := expression.getArg! 0
    if statements.isConstOf ``statementsNil then
      let next ← emitChoice twins slots pureValue integerRep rep (expression.getArg! 1) resultName
      return { next with agreement := ← `(tactic|
        (rw [LeanerIR.Proofs.ComputationAgreement.block_nil_eq]
         $(next.agreement):tactic)) }
    unless statements.isAppOfArity ``statementsCons 2 do
      throwError "native statement block requires a lowered statement list"
    let head ← emitChoice twins slots pureValue integerRep .unit (statements.getArg! 0)
    let next ← emitChoice twins slots pureValue integerRep rep
      (Lean.mkApp2 (Lean.mkConst ``blockResult) (statements.getArg! 1) (expression.getArg! 1)) resultName
    return {
      computation := ← ``(LeanerIR.Proofs.Spec.bind $(head.computation) (fun _ : Unit => $(next.computation)))
      verifyWith := fun finish => do
        let proof ← head.verifyWith (← next.verifyWith finish)
        `(tactic| (rw [LeanerIR.Proofs.wp_bind]; $proof:tactic))
      preserves := ← `(tactic|
        (apply LeanerIR.Proofs.StatePreserving.bind
         · $(head.preserves):tactic
         · intro ignored; $(next.preserves):tactic))
      agreement := ← `(tactic|
        (rw [LeanerIR.Proofs.ComputationAgreement.block_discard]
         apply LeanerIR.Proofs.ComputationAgreement.scalar_discard
         · $(head.agreement):tactic
         · $(next.agreement):tactic)) }
  if expression.isAppOfArity ``nativeBranch 3 then
    if (expression.getArg! 2).isAppOf ``Option.none then
      unless rep == .unit do throwError "native typed choice requires an else value"
      let otherwise := Lean.mkApp (Lean.mkConst ``LeanerIR.Proofs.Denotation.value)
        (Lean.mkConst ``LeanerIR.RuntimeValue.unit)
      let next ← emitChoice twins slots pureValue integerRep rep
        (Lean.mkApp3 (Lean.mkConst ``nativeBranch) (expression.getArg! 0) (expression.getArg! 1)
          (Lean.mkApp2 (Lean.mkConst ``Option.some [Lean.Level.zero])
            (Lean.mkConst ``ExprDenotation) otherwise)) resultName
      return { next with agreement := ← `(tactic|
        (rw [LeanerIR.Proofs.ComputationAgreement.branch_none]
         $(next.agreement):tactic)) }
    if (match rep with | .int .. => true | _ => false) && !NativeEffects.required expression then
      -- Only pure rendering is speculative; both paths produce native
      -- computations with independently checked execution agreement.
      let pure? ← try pure (some (← pureChoice pureValue rep expression)) catch _ => pure none
      if let some (value, agreement) := pure? then
        let ty ← rep.typeSyntax (mkIdent `Carrier)
        return {
          computation := ← ``(LeanerIR.Proofs.Spec.pure $value)
          agreement
          preserves := ← `(tactic| exact LeanerIR.Proofs.StatePreserving.pure _)
          verifyWith := fun next => do
            if let some name := resultName then `(tactic|
              (rw [LeanerIR.Proofs.wp_pure]
               let $name : $ty := $value
               $next:tactic))
            else `(tactic| (rw [LeanerIR.Proofs.wp_pure]; $next:tactic)) }
    return ← choice (← emit twins slots pureValue integerRep (expression.getArg! 0))
      (← emitChoice twins slots pureValue integerRep rep (expression.getArg! 1) resultName)
      (← emitChoice twins slots pureValue integerRep rep ((expression.getArg! 2).getArg! 1) resultName)
  if rep == .bool then return ← emit twins slots pureValue integerRep expression resultName
  NativeEffects.emit twins slots pureValue rep expression resultName

end LeanerLang.NativeCondition
