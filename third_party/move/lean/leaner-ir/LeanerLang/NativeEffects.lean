-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.NativeAggregate
import LeanerLang.NativeExpression
import LeanerLang.NativeCallValue
import LeanerLang.NativeIndex
import LeanerIR.Proofs.NativeValueAgreement
import LeanerIR.Proofs.NativeValue
import LeanerIR.Proofs.NativeControlAgreement

/-! Typed construction with effectful operands. Products collect native fields
in source order; failures propagate through `Spec.bind` before construction.
Runtime encodings occur only in the independently checked agreement. -/

namespace LeanerLang.NativeEffects
open Lean Elab Command
open LeanerIR.Proofs.Denotation
set_option quotPrecheck false

private def rooted (name : Name) : Ident := mkIdent (rootNamespace ++ name)

def required (expression : Lean.Expr) : Bool :=
  expression.getUsedConstants.any fun name =>
    [``nativeCall, ``PrimitiveLocationOperation.checkedAdd,
      ``PrimitiveLocationOperation.checkedSubtract,
      ``PrimitiveLocationOperation.checkedMultiply,
      ``PrimitiveLocationOperation.checkedDivide, ``PrimitiveLocationOperation.checkedModulo,
      ``PrimitiveLocationOperation.bitwiseAnd, ``PrimitiveLocationOperation.bitwiseOr,
      ``PrimitiveLocationOperation.bitwiseXor,
      ``PrimitiveLocationOperation.checkedShiftLeft, ``PrimitiveLocationOperation.checkedShiftRight].contains name

abbrev Operands := NativeOperands.Emitted

/-- Native left-to-right operands, shared by constructors and comparisons. -/
def emitOperands (fields : Array Typed.ValueRep) (expressions : Lean.Expr)
    (emitValue : Typed.ValueRep → Lean.Expr → CommandElabM NativeExpression.Emitted) :
    CommandElabM Operands := do
  let mut remaining := expressions
  let mut operands : Array NativeOperands.Operand := #[]
  for field in fields do
    unless remaining.isAppOfArity ``valuesCons 2 do
      throwError "native effectful operation is missing a field"
    let value ← emitValue field (remaining.getArg! 0)
    let type ← field.typeSyntax (mkIdent `Carrier)
    let codec ← field.codecSyntax (mkIdent `codecs)
    operands := operands.push ⟨type, ← ``(fun value : $type => ($codec).encode value), value⟩
    remaining := remaining.getArg! 1
  unless remaining.isConstOf ``valuesNil do
    throwError "native effectful operation has extra fields"
  NativeOperands.emit operands

/-- Throw payloads cross the existing failure ABI only after their typed
operands complete. No normal result or continuation is manufactured. -/
def emitThrow (rep : Typed.ValueRep) (expression : Lean.Expr)
    (emitValue : Typed.ValueRep → Lean.Expr → CommandElabM NativeExpression.Emitted) :
    CommandElabM NativeExpression.Emitted := do
  unless expression.isAppOfArity ``nativeThrow 2 &&
      (expression.getArg! 0).isConstOf ``LeanerIR.ThrowKind.abort do
    throwError "native explicit throw requires the scalar abort payload ABI"
  let operands ← emitOperands #[.int (.bits 64) false] (expression.getArg! 1) emitValue
  let args := mkIdent `abortOperands
  let type ← rep.typeSyntax (mkIdent `Carrier)
  let codec ← rep.codecSyntax (mkIdent `codecs)
  return {
    computation := ← ``(LeanerIR.Proofs.Spec.bind $(operands.computation)
      (fun $args : $(operands.type) => LeanerIR.Proofs.Spec.abort
        (LeanerIR.Proofs.NativeArithmetic.runtimeFailure LeanerIR.ThrowKind.abort $args.1.val)))
    verifyWith := fun _ => do
      let proof ← operands.verifyWith (← `(tactic|
        (rw [LeanerIR.Proofs.wp_abort]
         simp (config := { failIfUnchanged := false }) only
           [LeanerIR.Proofs.NativeArithmetic.runtimeFailure, Prod.mk.injEq,
             Array.mk.injEq, List.cons.injEq, LeanerIR.RuntimeValue.integer.injEq,
             and_true, true_and] <;> leaner_certified_close!)))
      `(tactic| (rw [LeanerIR.Proofs.wp_bind]; $proof:tactic))
    preserves := ← `(tactic|
      (apply LeanerIR.Proofs.StatePreserving.bind
       · $(operands.preserves):tactic
       · intro abortOperands initial result final impossible; cases impossible))
    agreement := ← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.scalar_throw
         (encodeArgs := $(operands.encode)) (encode := fun result : $type => ($codec).encode result)
       $(operands.agreement):tactic)) }

partial def emit (twins : Array SpecTypes.TwinInfo) (slots : Array (Option Term))
    (pureValue : Typed.ValueRep → Lean.Expr → CommandElabM (Term × TSyntax `tactic))
    (rep : Typed.ValueRep) (expression : Lean.Expr) (resultName : Option Ident := none) :
    CommandElabM NativeExpression.Emitted := do
  if expression.isAppOfArity ``nativeThrow 2 then
    return ← emitThrow rep expression (fun rep value => emit twins slots pureValue rep value)
  if NativeIndex.supported expression then
    return ← NativeIndex.emit slots rep expression resultName
  if NativeCopy.supported expression then
    return ← NativeCopy.wrap (← emit twins slots pureValue rep
      (← NativeCopy.operand expression) resultName)
  let type ← rep.typeSyntax (mkIdent `Carrier)
  let codec ← rep.codecSyntax (mkIdent `codecs)
  let encode ← ``(fun value : $type => ($codec).encode value)
  if expression.isAppOfArity ``nativeCall 4 then
    let name := resultName.getD (mkIdent `operandResult)
    let call ← NativeCallValue.emit twins rep pureValue expression
    return {
      computation := call.computation
      verifyWith := fun next => do
        let call ← NativeCallValue.emit twins rep pureValue expression (some (name, next))
        return call.verify
      preserves := call.preserves
      agreement := call.agreement }
  if NativeAggregate.supported expression && required expression then
    let (constructor, fields) ← NativeAggregate.constructorInfo twins rep expression
    let row ← emitOperands fields (expression.getArg! 1)
      (fun rep value => emit twins slots pureValue rep value)
    let args := mkIdent `operands
    let mut tail : Term := ⟨args.raw⟩
    let mut values : Array Term := #[]
    for _ in fields do
      values := values.push (← ``($tail.1))
      tail ← ``($tail.2)
    let value ← ``($constructor $values*)
    let finalRow := row
    return {
      computation := ← ``(LeanerIR.Proofs.Spec.bind $(finalRow.computation)
        (fun $args : $(finalRow.type) => LeanerIR.Proofs.Spec.pure $value))
      verifyWith := fun next => do
        let result ← if let some name := resultName then `(tactic|
            (rw [LeanerIR.Proofs.wp_pure_value]
             intro $name:ident $(mkIdent `valueEquation):ident
             $next:tactic))
          else `(tactic| (rw [LeanerIR.Proofs.wp_pure]; $next:tactic))
        let proof ← finalRow.verifyWith result
        `(tactic| (rw [LeanerIR.Proofs.wp_bind]; $proof:tactic))
      preserves := ← `(tactic|
        (apply LeanerIR.Proofs.StatePreserving.bind
         · $(finalRow.preserves):tactic
         · intro operands; exact LeanerIR.Proofs.StatePreserving.pure _))
      agreement := ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.scalar_operation_value
           (encodeArgs := $(finalRow.encode)) (encodeResult := $encode)
           (value := fun $args : $(finalRow.type) => $value)
         · $(finalRow.agreement):tactic
         · intro operands state; $(← NativeAggregate.constructorAgreement expression):tactic)) }
  if expression.isAppOfArity ``nativePrimitiveOperation 2 && required expression then
    let .int (.bits width) signed := rep
      | throwError "native checked operand requires a fixed-width integer type"
    let checked ← NativeExpression.emit ⟨type, width, signed, slots, #[]⟩ expression resultName
    if let some name := resultName then
      let operation := (expression.getArg! 0).getAppFn.constName!
      if [``PrimitiveLocationOperation.checkedAdd, ``PrimitiveLocationOperation.checkedSubtract,
          ``PrimitiveLocationOperation.checkedMultiply].contains operation then
        let bounds := mkIdent (if signed then ``LeanerIR.SpecInt.signed_bounds
          else ``LeanerIR.SpecInt.unsigned_bounds)
        return { checked with verifyWith := fun next => do
          checked.verifyWith (← `(tactic|
            (simp (config := { failIfUnchanged := false }) only
               [Prod.fst, Prod.snd, leaner_native_integer_choice] at $(mkIdent `valueEquation):ident
             have $(mkIdent `nativeResultBounds):ident := $bounds $name:ident
             leaner_cases $(mkIdent `nativeResultBounds):ident
             $next:tactic))) }
    return checked
  let (value, evaluates) ← pureValue rep expression
  return {
    computation := ← ``(LeanerIR.Proofs.Spec.pure $value)
    verifyWith := fun next =>
      if let some name := resultName then `(tactic|
        (rw [LeanerIR.Proofs.wp_pure]
         let $name : $type := $value
         $next:tactic))
      else `(tactic| (rw [LeanerIR.Proofs.wp_pure]; $next:tactic))
    preserves := ← `(tactic| exact LeanerIR.Proofs.StatePreserving.pure _)
    agreement := ← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.scalar_pure $encode $value
       $evaluates:tactic)) }

end LeanerLang.NativeEffects
