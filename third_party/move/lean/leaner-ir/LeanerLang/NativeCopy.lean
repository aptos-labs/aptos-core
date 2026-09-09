-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.NativeOperands
import LeanerIR.Proofs.NativeValueAgreement

/-! Shared references carry their observed native value. Source copies remain
in execution agreement but require no extra computation or decoding. -/

namespace LeanerLang.NativeCopy
open Lean Elab Command
open LeanerIR.Proofs.Denotation
set_option quotPrecheck false

def supported (expression : Lean.Expr) : Bool :=
  expression.isAppOfArity ``nativePrimitiveOperation 2 &&
    (expression.getArg! 0).isAppOfArity ``PrimitiveLocationOperation.copyValue 1

/-- The shared-local operation is also an observed value, but retains its
slot-bounds and reference-kind checks in execution agreement. -/
def localBorrow? (expression : Lean.Expr) : Option Nat := do
  guard (expression.isAppOfArity ``nativeLocalOperation 2 &&
    (expression.getArg! 1).isConstOf ``valuesNil)
  let operation := expression.getArg! 0
  guard (operation.isAppOfArity ``LocalLocationOperation.borrow 4 &&
    (operation.getArg! 2).isConstOf ``LeanerIR.BorrowKind.immutable)
  let location := operation.getArg! 0
  guard (location.isAppOfArity ``LocalLocation.mk 1)
  let localId := location.getArg! 0
  guard (localId.isAppOfArity ``LeanerIR.LocalId.mk 1)
  (localId.getArg! 0).nat? <|> (localId.getArg! 0).rawNatLit?

def borrowReturns (index count : Nat) : CommandElabM (TSyntax `tactic) :=
  `(tactic| exact LeanerIR.Proofs.ComputationAgreement.shared_local _ _ _ _ _ rfl rfl
    (by change $(Syntax.mkNatLit index) < $(Syntax.mkNatLit count); decide))

def operand (expression : Lean.Expr) : CommandElabM Lean.Expr := do
  unless supported expression do throwError "native copy requires a copy operation"
  let operands := expression.getArg! 1
  unless operands.isAppOfArity ``valuesCons 2 &&
      (operands.getArg! 1).isConstOf ``valuesNil do
    throwError "native copy requires exactly one operand"
  return operands.getArg! 0

/-- Inspect type metadata through copies; emission must retain their agreement. -/
partial def unwrap (expression : Lean.Expr) : CommandElabM Lean.Expr := do
  if supported expression then unwrap (← operand expression) else pure expression

def returns (proof : TSyntax `tactic) : CommandElabM (TSyntax `tactic) :=
  `(tactic|
    (apply LeanerIR.Proofs.ComputationAgreement.operation_value
     · apply LeanerIR.Proofs.ComputationAgreement.cons
       · $proof:tactic
       · exact LeanerIR.Proofs.ComputationAgreement.nil _
     · intro state; rfl))

partial def emitPure (emit : Lean.Expr → CommandElabM (Term × TSyntax `tactic))
    (expression : Lean.Expr) : CommandElabM (Term × TSyntax `tactic) := do
  if supported expression then
    let (value, proof) ← emitPure emit (← operand expression)
    return (value, ← returns proof)
  emit expression

def wrap (value : NativeOperands.Value) : CommandElabM NativeOperands.Value := do
  return { value with agreement := ← `(tactic|
    (apply LeanerIR.Proofs.ComputationAgreement.scalar_copy
     $(value.agreement):tactic)) }

end LeanerLang.NativeCopy
