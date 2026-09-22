-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Typed
import LeanerIR.Proofs.ArithmeticAgreement

/-! Native scalar comparisons shared by Boolean results and branch tests.
The emitted value is a Boolean over typed operands. Runtime evaluation occurs
only in its separate exact `Returns` certificate. -/

namespace LeanerLang.NativeComparison

open Lean Elab Command
open LeanerIR.Proofs.Denotation

set_option quotPrecheck false

def supported (expression : Lean.Expr) : Bool :=
  expression.isAppOfArity ``nativePrimitiveOperation 2 &&
    [``PrimitiveLocationOperation.less, ``PrimitiveLocationOperation.greater,
      ``PrimitiveLocationOperation.lessEqual, ``PrimitiveLocationOperation.greaterEqual,
      ``PrimitiveLocationOperation.equal, ``PrimitiveLocationOperation.notEqual].contains
        (expression.getArg! 0).getAppFn.constName!

/-- The descriptor's type is the result type, not the operand type.
The caller selects the equality representation from its typed operands. -/
def isEquality (expression : Lean.Expr) : Bool :=
  supported expression &&
    [``PrimitiveLocationOperation.equal, ``PrimitiveLocationOperation.notEqual].contains
      (expression.getArg! 0).getAppFn.constName!

/-- The same typed comparison is used with pure or effectful operands. -/
def emitValue (operator : Name) (left right : Term) (booleanOperands : Bool := false) :
    CommandElabM Term := do
  if booleanOperands then
    if operator == ``PrimitiveLocationOperation.equal then ``($left == $right)
    else ``($left != $right)
  else if operator == ``PrimitiveLocationOperation.less then ``(decide ($left < $right))
  else if operator == ``PrimitiveLocationOperation.greater then ``(decide ($left > $right))
  else if operator == ``PrimitiveLocationOperation.lessEqual then ``(decide ($left ≤ $right))
  else if operator == ``PrimitiveLocationOperation.greaterEqual then ``(decide ($left ≥ $right))
  else if operator == ``PrimitiveLocationOperation.equal then ``(decide ($left = $right))
  else ``(decide ($left ≠ $right))

def emit (operand : Lean.Expr → CommandElabM (Term × TSyntax `tactic))
    (expression : Lean.Expr) (booleanOperands : Bool := false) :
    CommandElabM (Term × TSyntax `tactic) := do
  unless supported expression do throwError "native comparison requires a supported scalar operation"
  let operator := (expression.getArg! 0).getAppFn.constName!
  let operands := expression.getArg! 1
  unless operands.isAppOfArity ``valuesCons 2 &&
      (operands.getArg! 1).isAppOfArity ``valuesCons 2 &&
      ((operands.getArg! 1).getArg! 1).isConstOf ``valuesNil do
    throwError "native comparison requires two operands"
  let (left, leftProof) ← operand (operands.getArg! 0)
  let (right, rightProof) ← operand ((operands.getArg! 1).getArg! 0)
  unless !booleanOperands || isEquality expression do
    throwError "native Boolean comparison requires equality or inequality"
  let value ← emitValue operator left right booleanOperands
  let agreement ← `(tactic|
    (apply LeanerIR.Proofs.ComputationAgreement.operation_value
     · apply LeanerIR.Proofs.ComputationAgreement.cons
       · $leftProof:tactic
       · apply LeanerIR.Proofs.ComputationAgreement.cons
         · $rightProof:tactic
         · exact LeanerIR.Proofs.ComputationAgreement.nil _
     · intro state
       simp [bne, LeanerIR.Proofs.Codec.specInt, LeanerIR.Proofs.Codec.bool,
         LeanerIR.RuntimeValue.beq_bool,
         LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.evaluate?,
         LeanerIR.Proofs.Denotation.liftPrimitiveEvaluator,
         LeanerIR.SemanticOperations.compareOrdered,
         LeanerIR.SemanticOperations.equalValues?, LeanerIR.SemanticOperations.notEqualValues?] <;> rfl))
  return (value, agreement)

end LeanerLang.NativeComparison
