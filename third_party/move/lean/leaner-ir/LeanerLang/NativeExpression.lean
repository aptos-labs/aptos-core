-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Typed
import LeanerLang.NativeOperands
import LeanerLang.NativeCopy
import LeanerIR.Proofs.OperandAgreement
import LeanerIR.Proofs.NativeDivisionAgreement
import LeanerIR.Proofs.NativeBitwiseAgreement
import LeanerIR.Proofs.NativeShiftAgreement
import LeanerIR.Proofs.NativeResultAgreement
import LeanerIR.Proofs.NativeMutationAgreement
import LeanerIR.Proofs.NativeValue
import LeanerLang.NativeData

/-! Recursive typed operand evaluation. Only agreement proofs mention runtime
rows; the native tree contains range-carrying integers and `Spec.bind`. -/

namespace LeanerLang.NativeExpression

open Lean Meta Elab Command
open LeanerIR.Proofs.Denotation

set_option quotPrecheck false

private partial def index? (expression : Lean.Expr) : Option Nat :=
  (expression.nat? <|> expression.rawNatLit?).orElse fun _ =>
    if expression.getAppNumArgs == 1 then index? (expression.getArg! 0) else none

def division (expression : Lean.Expr) : Bool :=
  expression.isAppOfArity ``nativePrimitiveOperation 2 &&
    [``PrimitiveLocationOperation.checkedDivide, ``PrimitiveLocationOperation.checkedModulo].contains
      (expression.getArg! 0).getAppFn.constName!

def multiplication (expression : Lean.Expr) : Bool :=
  expression.isAppOfArity ``nativePrimitiveOperation 2 &&
    (expression.getArg! 0).isAppOfArity ``PrimitiveLocationOperation.checkedMultiply 2

def bitwise (expression : Lean.Expr) : Bool :=
  expression.isAppOfArity ``nativePrimitiveOperation 2 &&
    [``PrimitiveLocationOperation.bitwiseAnd, ``PrimitiveLocationOperation.bitwiseOr,
      ``PrimitiveLocationOperation.bitwiseXor].contains (expression.getArg! 0).getAppFn.constName!

def shift (expression : Lean.Expr) : Bool :=
  expression.isAppOfArity ``nativePrimitiveOperation 2 &&
    [``PrimitiveLocationOperation.checkedShiftLeft, ``PrimitiveLocationOperation.checkedShiftRight].contains
      (expression.getArg! 0).getAppFn.constName!

def nested (expression : Lean.Expr) : Bool := Id.run do
  unless expression.isAppOfArity ``nativePrimitiveOperation 2 do return false
  let operands := expression.getArg! 1
  unless operands.isAppOfArity ``valuesCons 2 &&
      (operands.getArg! 1).isAppOfArity ``valuesCons 2 do return false
  return (operands.getArg! 0).isAppOfArity ``nativePrimitiveOperation 2 ||
    ((operands.getArg! 1).getArg! 0).isAppOfArity ``nativePrimitiveOperation 2

structure Context where
  nativeType : Term
  width : Nat
  signed : Bool
  slots : Array (Option Term)
  /-- Live mutable owners, separate from ordinary scalar slots. A raw owner
  read is not an integer operand; only its source dereference projects it. -/
  owners : Array (Option Term) := #[]

abbrev Emitted := NativeOperands.Value

partial def emit (context : Context) (expression : Lean.Expr)
    (resultName : Option Ident := none) : CommandElabM Emitted := do
  if NativeCopy.supported expression then
    return ← NativeCopy.wrap (← emit context (← NativeCopy.operand expression) resultName)
  let nativeType := context.nativeType
  let widthTerm ← ``(LeanerIR.IntWidth.bits $(Syntax.mkNatLit context.width))
  let signedTerm := quote context.signed
  let encode ← ``(fun value : $nativeType => LeanerIR.RuntimeValue.integer value.val)
  if expression.isAppOfArity ``nativeReferenceOperation 2 &&
      (expression.getArg! 0).isConstOf ``ReferenceLocationOperation.dereference then
    let operands := expression.getArg! 1
    unless operands.isAppOfArity ``valuesCons 2 &&
        (operands.getArg! 1).isConstOf ``valuesNil &&
        (operands.getArg! 0).isAppOfArity ``localVar 1 do
      throwError "native integer dereference requires a live owner local"
    let some index := index? ((operands.getArg! 0).getArg! 0)
      | throwError "invalid native owner local"
    let some (some owner) := context.owners[index]?
      | throwError "native integer dereference has no live owner"
    return {
      computation := ← ``(LeanerIR.Proofs.Spec.pure ($owner).value)
      verifyWith := fun next => `(tactic| (rw [LeanerIR.Proofs.wp_pure]; $next:tactic))
      preserves := ← `(tactic| exact LeanerIR.Proofs.StatePreserving.pure _)
      agreement := ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.scalar_reference_read
          (LeanerIR.Proofs.Codec.specInt $widthTerm $signedTerm) $owner
         exact LeanerIR.Proofs.ComputationAgreement.read _ _ _ rfl)) }
  if expression.isAppOfArity ``localVar 1 ||
      expression.isAppOfArity ``LeanerIR.Proofs.Denotation.value 1 then
    let (value, returns) ← if expression.isAppOfArity ``localVar 1 then do
        let some index := index? (expression.getArg! 0) | throwError "invalid native operand local"
        if (context.owners[index]?.getD none).isSome then
          throwError "native integer operand must dereference its mutable owner"
        let some (some value) := context.slots[index]? | throwError "native operand local is unavailable"
        pure (value, ← `(tactic| exact LeanerIR.Proofs.ComputationAgreement.read _ _ _ rfl))
      else do
        unless (expression.getArg! 0).isAppOfArity ``LeanerIR.RuntimeValue.integer 1 do
          throwError "native arithmetic requires integer literals"
        let literal ← liftTermElabM <| PrettyPrinter.delab ((expression.getArg! 0).getArg! 0)
        pure (← ``((⟨$literal, by decide⟩ : $nativeType)),
          ← `(tactic| exact LeanerIR.Proofs.ComputationAgreement.literal _ _))
    return {
      computation := ← ``(LeanerIR.Proofs.Spec.pure $value)
      verifyWith := fun next => `(tactic| (rw [LeanerIR.Proofs.wp_pure]; $next:tactic))
      preserves := ← `(tactic| exact LeanerIR.Proofs.StatePreserving.pure _)
      agreement := ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.scalar_pure $encode $value
         $returns:tactic)) }
  unless expression.isAppOfArity ``nativePrimitiveOperation 2 do
    throwError "native arithmetic operand requires a local, integer literal, or checked operation"
  let operation := expression.getArg! 0
  let operator := operation.getAppFn.constName!
  unless [``PrimitiveLocationOperation.checkedAdd,
      ``PrimitiveLocationOperation.checkedSubtract,
      ``PrimitiveLocationOperation.checkedMultiply,
      ``PrimitiveLocationOperation.checkedDivide, ``PrimitiveLocationOperation.checkedModulo,
      ``PrimitiveLocationOperation.bitwiseAnd, ``PrimitiveLocationOperation.bitwiseOr,
      ``PrimitiveLocationOperation.bitwiseXor,
      ``PrimitiveLocationOperation.checkedShiftLeft, ``PrimitiveLocationOperation.checkedShiftRight].contains operator do
    throwError "nested native arithmetic requires a supported checked or bitwise operation"
  let isDivision := division expression
  let isMultiply := multiplication expression
  let isBitwise := bitwise expression
  let isShift := shift expression
  let shiftLeft := quote (operator == ``PrimitiveLocationOperation.checkedShiftLeft)
  let isRemainder := operator == ``PrimitiveLocationOperation.checkedModulo
  let operands := expression.getArg! 1
  unless operands.isAppOfArity ``valuesCons 2 &&
      (operands.getArg! 1).isAppOfArity ``valuesCons 2 &&
      ((operands.getArg! 1).getArg! 1).isConstOf ``valuesNil do
    throwError "nested native arithmetic requires two operands"
  let left ← emit context (operands.getArg! 0)
  let rightExpression := (operands.getArg! 1).getArg! 0
  let rightType ← if isShift then ``(Int) else pure nativeType
  let rightEncode ← if isShift then ``(fun value : Int => LeanerIR.RuntimeValue.integer value)
    else pure encode
  let right ← if !isShift then emit context rightExpression else do
    -- A shift distance is a mathematical integer. Parameters and locals keep
    -- their own fixed-width native types; only this operation's input projects
    -- their integer field. Nested arithmetic uses its resolved result type.
    if rightExpression.isAppOfArity ``localVar 1 ||
        rightExpression.isAppOfArity ``LeanerIR.Proofs.Denotation.value 1 then
      let (value, evaluates) ← if rightExpression.isAppOfArity ``localVar 1 then do
          let some index := index? (rightExpression.getArg! 0) | throwError "invalid native shift distance local"
          let some (some value) := context.slots[index]? | throwError "native shift distance is unavailable"
          pure (← ``(($value).val),
            ← `(tactic| exact LeanerIR.Proofs.ComputationAgreement.read _ _ _ rfl))
        else do
          unless (rightExpression.getArg! 0).isAppOfArity ``LeanerIR.RuntimeValue.integer 1 do
            throwError "native shift distance requires an integer literal"
          let value ← liftTermElabM <| PrettyPrinter.delab ((rightExpression.getArg! 0).getArg! 0)
          pure (value, ← `(tactic| exact LeanerIR.Proofs.ComputationAgreement.literal _ _))
      pure ({
        computation := ← ``(LeanerIR.Proofs.Spec.pure $value)
        verifyWith := fun next => `(tactic| (rw [LeanerIR.Proofs.wp_pure]; $next:tactic))
        preserves := ← `(tactic| exact LeanerIR.Proofs.StatePreserving.pure _)
        agreement := ← `(tactic|
          (apply LeanerIR.Proofs.ComputationAgreement.scalar_pure $rightEncode $value
           $evaluates:tactic)) } : Emitted)
    else
      unless rightExpression.isAppOfArity ``nativePrimitiveOperation 2 do
        throwError "native shift distance requires a local, literal, or integer operation"
      let distanceOperation := rightExpression.getArg! 0
      let ty := distanceOperation.getArg! (distanceOperation.getAppNumArgs - 1)
      unless ty.isAppOfArity ``LeanerIR.Ty.integer 2 &&
          (ty.getArg! 0).isAppOfArity ``LeanerIR.IntWidth.bits 1 do
        throwError "native shift distance operation requires a resolved fixed-width integer type"
      let some width := index? ((ty.getArg! 0).getArg! 0) | throwError "unresolved native shift distance width"
      let signed := (ty.getArg! 1).isConstOf ``Bool.true
      let type ← (Typed.ValueRep.int (.bits width) signed).typeSyntax (mkIdent `Carrier)
      let first ← emit { context with nativeType := type, width, signed } rightExpression
      pure ({
        computation := ← ``(LeanerIR.Proofs.Spec.bind $(first.computation)
          (fun value : $type => LeanerIR.Proofs.Spec.pure value.val))
        verifyWith := fun next => do
          let proof ← first.verifyWith (← `(tactic| (rw [LeanerIR.Proofs.wp_pure]; $next:tactic)))
          `(tactic| (rw [LeanerIR.Proofs.wp_bind]; $proof:tactic))
        preserves := ← `(tactic|
          (apply LeanerIR.Proofs.StatePreserving.bind
           · $(first.preserves):tactic
           · intro value; exact LeanerIR.Proofs.StatePreserving.pure _))
        agreement := ← `(tactic|
          (apply LeanerIR.Proofs.ComputationAgreement.scalar_map
             (encode := fun value : $type => LeanerIR.RuntimeValue.integer value.val)
           · $(first.agreement):tactic
           · intro value; rfl)) } : Emitted)
  let kind ← if isBitwise then ``(LeanerIR.ThrowKind.abort)
    else liftTermElabM <| PrettyPrinter.delab (operation.getArg! 0)
  let args := mkIdent `operands
  let leftValue ← ``(($args.1).val)
  let rightValue ← if isShift then ``($args.2.1) else ``(($args.2.1).val)
  let value ← if operator == ``PrimitiveLocationOperation.checkedAdd then
      ``($leftValue + $rightValue)
    else if isMultiply then ``($leftValue * $rightValue)
    else if isRemainder then ``(Int.tmod $leftValue $rightValue)
    else if isDivision then ``(Int.tdiv $leftValue $rightValue)
    else ``($leftValue - $rightValue)
  let operandProduct ← NativeOperands.emit #[⟨nativeType, encode, left⟩, ⟨rightType, rightEncode, right⟩]
  let argumentType := operandProduct.type
  let argumentComputation := operandProduct.computation
  let resultRule := mkIdent (if isRemainder then ``LeanerIR.Proofs.NativeArithmetic.remainderResult
    else ``LeanerIR.Proofs.NativeArithmetic.divisionResult)
  let nativeResult ← if isShift then
      ``(LeanerIR.Proofs.NativeArithmetic.shiftResult $(Syntax.mkNatLit context.width)
        (by decide) $signedTerm $shiftLeft (LeanerIR.Proofs.NativeArithmetic.runtimeFailure $kind)
        $args.1 $rightValue)
    else ``($resultRule $widthTerm $signedTerm
      (LeanerIR.Proofs.NativeArithmetic.emptyFailure $kind)
      (LeanerIR.Proofs.NativeArithmetic.runtimeFailure $kind) $leftValue $rightValue)
  let bitOperation ← if operator == ``PrimitiveLocationOperation.bitwiseAnd then
      ``(fun left right : Nat => left &&& right)
    else if operator == ``PrimitiveLocationOperation.bitwiseOr then
      ``(fun left right : Nat => left ||| right)
    else ``(fun left right : Nat => left ^^^ right)
  let bitResult ← ``(LeanerIR.Proofs.NativeArithmetic.bitwise $(Syntax.mkNatLit context.width)
    (by decide) $signedTerm $bitOperation $args.1 $args.2.1)
  let operationComputation ← if isBitwise then ``(LeanerIR.Proofs.Spec.pure $bitResult)
    else if isDivision || isShift then ``(LeanerIR.Proofs.Spec.ofExcept $nativeResult)
    else ``(LeanerIR.Proofs.NativeArithmetic.checkedInteger
      $widthTerm $signedTerm (LeanerIR.Proofs.NativeArithmetic.runtimeFailure $kind) $value)
  let computation ← ``(LeanerIR.Proofs.Spec.bind $argumentComputation
    (fun $args : $argumentType => $operationComputation))
  let verifyWith (next : TSyntax `tactic) : CommandElabM (TSyntax `tactic) := do
    let checked ← if let some name := resultName then `(tactic|
      (rw [LeanerIR.Proofs.NativeArithmetic.wp_checkedInteger_value]
       constructor
       · intro $name:ident $(mkIdent `valueEquation):ident
         $next:tactic
       · intro $(mkIdent `overflow):ident
         simp [LeanerIR.IntegerValueFits, LeanerIR.Ty.integerValueFits?, LeanerIR.Ty.integerBounds?]
           at $(mkIdent `overflow):ident
         all_goals simp (config := { failIfUnchanged := false }) only
           [LeanerIR.Proofs.NativeArithmetic.runtimeFailure, Prod.mk.injEq,
           Array.mk.injEq, List.cons.injEq, LeanerIR.RuntimeValue.integer.injEq,
           and_true, true_and] <;> leaner_certified_close!))
      else `(tactic|
      (rw [LeanerIR.Proofs.NativeArithmetic.wp_checkedInteger]
       constructor
       · intro $(mkIdent `fits):ident
         simp [LeanerIR.IntegerValueFits, LeanerIR.Ty.integerValueFits?, LeanerIR.Ty.integerBounds?]
           at $(mkIdent `fits):ident
         leaner_cases $(mkIdent `fits):ident
         $next:tactic
       · intro $(mkIdent `overflow):ident
         simp [LeanerIR.IntegerValueFits, LeanerIR.Ty.integerValueFits?, LeanerIR.Ty.integerBounds?]
           at $(mkIdent `overflow):ident
         all_goals simp (config := { failIfUnchanged := false }) only
           [LeanerIR.Proofs.NativeArithmetic.runtimeFailure, Prod.mk.injEq,
           Array.mk.injEq, List.cons.injEq, LeanerIR.RuntimeValue.integer.injEq,
           and_true, true_and] <;> leaner_certified_close!))
    let checked ← if isDivision then do
        -- Supply range facts directly from native operand certificates.
        -- No context search or decoding is needed to rediscover them.
        let bounds ← if context.signed then do
            let limit := Syntax.mkNatLit (2 ^ (context.width - 1))
            let quotient ← `(tactic| have $(mkIdent `nativeQuotientBounds):ident :=
              LeanerIR.Proofs.IntegerArithmetic.signed_quotient_bounds $limit:num
                $leftValue $rightValue
                (by have h := LeanerIR.SpecInt.signed_bounds $args.1; simp at h; omega)
                (by have h := LeanerIR.SpecInt.signed_bounds $args.1; simp at h; omega)
                $(mkIdent `divisorNonzero):ident)
            if isRemainder then `(tactic|
              ($quotient:tactic
               have $(mkIdent `nativeRemainderBounds):ident :=
                 LeanerIR.Proofs.IntegerArithmetic.signed_remainder_bounds $limit:num
                   $leftValue $rightValue
                   (by have h := LeanerIR.SpecInt.signed_bounds $args.2.1; simp at h; omega)
                   (by have h := LeanerIR.SpecInt.signed_bounds $args.2.1; simp at h; omega)
                   $(mkIdent `divisorNonzero):ident))
            else pure quotient
          else do
            let quotient ← `(tactic| have $(mkIdent `nativeQuotientBounds):ident :=
              LeanerIR.Proofs.IntegerArithmetic.unsigned_quotient_bounds
                (LeanerIR.SpecInt.fits $args.1) (LeanerIR.SpecInt.fits $args.2.1))
            if isRemainder then `(tactic|
              ($quotient:tactic
               have $(mkIdent `nativeRemainderBounds):ident :=
                 LeanerIR.Proofs.IntegerArithmetic.unsigned_remainder_bounds
                   (LeanerIR.SpecInt.fits $args.1) (LeanerIR.SpecInt.fits $args.2.1)
                   $(mkIdent `divisorNonzero):ident))
            else pure quotient
        let afterNonzero ← if isRemainder then `(tactic|
            (rw [LeanerIR.Proofs.wp_bind, LeanerIR.Proofs.NativeArithmetic.wp_checkedInteger]
             constructor
             · intro $(mkIdent `quotientFits):ident
               simp [LeanerIR.IntegerValueFits, LeanerIR.Ty.integerValueFits?, LeanerIR.Ty.integerBounds?]
                 at $(mkIdent `quotientFits):ident
               leaner_cases $(mkIdent `quotientFits):ident
               $checked:tactic
             · intro $(mkIdent `quotientOverflow):ident
               simp [LeanerIR.IntegerValueFits, LeanerIR.Ty.integerValueFits?, LeanerIR.Ty.integerBounds?]
                 at $(mkIdent `quotientOverflow):ident
               leaner_certified_close!))
          else pure checked
        let rule := mkIdent (if isRemainder then ``LeanerIR.Proofs.NativeArithmetic.wp_remainderResult
          else ``LeanerIR.Proofs.NativeArithmetic.wp_divisionResult)
        `(tactic|
          (rw [$rule:term]
           constructor
           · intro $(mkIdent `divisorZero):ident
             subst $args:ident
             leaner_certified_close!
           · intro $(mkIdent `divisorNonzero):ident
             $bounds:tactic
             subst $args:ident
             $afterNonzero:tactic))
      else pure checked
    let checked ← if isBitwise then do
        let bounds := mkIdent (if context.signed then ``LeanerIR.SpecInt.signed_bounds
          else ``LeanerIR.SpecInt.unsigned_bounds)
        let finish ← if let some name := resultName then `(tactic|
            (rw [LeanerIR.Proofs.wp_pure_value]
             intro $name:ident $(mkIdent `valueEquation):ident
             have $(mkIdent `integerValueEquation):ident :=
               congrArg LeanerIR.SpecInt.val $(mkIdent `valueEquation):ident
             simp (config := { failIfUnchanged := false }) only
               [LeanerIR.Proofs.NativeArithmetic.bitwise_and_unsigned]
               at $(mkIdent `integerValueEquation):ident
             $next:tactic))
          else `(tactic| (rw [LeanerIR.Proofs.wp_pure]; $next:tactic))
        `(tactic|
          (have $(mkIdent `bitwiseBounds):ident := $bounds $bitResult
           simp (config := { failIfUnchanged := false }) only
             [LeanerIR.Proofs.NativeArithmetic.bitwise_and_unsigned]
             at $(mkIdent `bitwiseBounds):ident
           leaner_cases $(mkIdent `bitwiseBounds):ident
           subst $args:ident
           $finish:tactic))
      else pure checked
    let checked ← if isShift then do
        let bounds := mkIdent (if context.signed then ``LeanerIR.SpecInt.signed_bounds
          else ``LeanerIR.SpecInt.unsigned_bounds)
        let result ← ``(LeanerIR.Proofs.NativeArithmetic.shiftValue $(Syntax.mkNatLit context.width)
          (by decide) $signedTerm $shiftLeft $args.1 $rightValue)
        let arithmetic ← if !context.signed && operator == ``PrimitiveLocationOperation.checkedShiftRight then
            `(tactic|
              (have $(mkIdent `shiftUpper):ident :=
                 LeanerIR.Proofs.NativeArithmetic.shiftValue_right_unsigned_upper
                   $(Syntax.mkNatLit context.width) (by decide) $args.1 $rightValue
                   (by have h := $(mkIdent `validDistance):ident; omega)
               simp [LeanerIR.Proofs.NativeArithmetic.shiftValue_right_unsigned]
                 at $(mkIdent `shiftUpper):ident))
          else `(tactic| skip)
        let specialize ← if !context.signed && operator == ``PrimitiveLocationOperation.checkedShiftRight then
            `(tactic| simp at $(mkIdent `shiftUpper):ident)
          else `(tactic| skip)
        let success ← if let some name := resultName then `(tactic|
            (intro $name:ident $(mkIdent `valueEquation):ident
             have $(mkIdent `integerValueEquation):ident :=
               congrArg LeanerIR.SpecInt.val $(mkIdent `valueEquation):ident
             simp (config := { failIfUnchanged := false }) only
               [LeanerIR.Proofs.NativeArithmetic.shiftValue_left_unsigned,
                 LeanerIR.Proofs.NativeArithmetic.shiftValue_right_unsigned]
               at $(mkIdent `integerValueEquation):ident
             have $(mkIdent `shiftBounds):ident := $bounds $name:ident
             $arithmetic:tactic
             subst $args:ident
             $specialize:tactic
             simp (config := { failIfUnchanged := false }) only [Int.reduceToNat]
               at $(mkIdent `integerValueEquation):ident
             simp at $(mkIdent `shiftBounds):ident
             leaner_cases $(mkIdent `shiftBounds):ident
             $next:tactic))
          else `(tactic|
            (have $(mkIdent `shiftBounds):ident := $bounds $result
             simp (config := { failIfUnchanged := false }) only
               [LeanerIR.Proofs.NativeArithmetic.shiftValue_left_unsigned,
                 LeanerIR.Proofs.NativeArithmetic.shiftValue_right_unsigned]
               at $(mkIdent `shiftBounds):ident
             $arithmetic:tactic
             subst $args:ident
             $specialize:tactic
             simp at $(mkIdent `shiftBounds):ident
             leaner_cases $(mkIdent `shiftBounds):ident
             $next:tactic))
        let rule := mkIdent (if resultName.isSome then
          ``LeanerIR.Proofs.NativeArithmetic.wp_shiftResult_value
          else ``LeanerIR.Proofs.NativeArithmetic.wp_shiftResult)
        `(tactic|
          (apply ($rule:term _ _ _ _ _ _ _ _ _ _).mpr
           constructor
           · intro $(mkIdent `invalidDistance):ident
             subst $args:ident
             simp at $(mkIdent `invalidDistance):ident <;> leaner_certified_close!
           · intro $(mkIdent `validDistance):ident
             $success:tactic))
      else pure checked
    let smallBound := Syntax.mkNatLit (2 ^ (context.width / 2) - 1)
    let checked ← if isMultiply && !context.signed then `(tactic|
        (have $(mkIdent `productNonnegative):ident : 0 ≤ $value :=
           Int.mul_nonneg (LeanerIR.SpecInt.unsigned_bounds $args.1).1
             (LeanerIR.SpecInt.unsigned_bounds $args.2.1).1
         -- An authored small-operand bound can establish no overflow without
         -- expanding multiplication into bit operations or enumerating values.
         try
           have $(mkIdent `productUpperBound):ident :
               $value ≤ ($smallBound:num : Int) * $smallBound:num :=
             Int.mul_le_mul_of_le_of_le_of_nonneg_of_nonneg
               (show $leftValue ≤ $smallBound:num by
                 subst $args:ident; simp only [Prod.fst, Prod.snd]; omega)
               (show $rightValue ≤ $smallBound:num by
                 subst $args:ident; simp only [Prod.fst, Prod.snd]; omega)
               (LeanerIR.SpecInt.unsigned_bounds $args.2.1).1 (by decide)
         subst $args:ident
         $checked:tactic))
      else pure checked
    let proof ← if isDivision || isBitwise || isShift || (isMultiply && !context.signed) then
        operandProduct.verifyNamedWith args checked
      else operandProduct.verifyWith checked
    `(tactic| (rw [LeanerIR.Proofs.wp_bind]; $proof:tactic))
  let operationPreserves ← if isBitwise then
      `(tactic| exact LeanerIR.Proofs.StatePreserving.pure _)
    else if isDivision || isShift then
      `(tactic| exact LeanerIR.Proofs.StatePreserving.ofExcept _)
    else `(tactic| exact LeanerIR.Proofs.NativeArithmetic.checkedInteger_preserves _ _ _ _)
  let preserves ← `(tactic|
    (apply LeanerIR.Proofs.StatePreserving.bind
     · $(operandProduct.preserves):tactic
     · intro operands
       $operationPreserves:tactic))
  let operandAgreement := operandProduct.agreement
  let agreement ← if isBitwise then `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.scalar_operation_value
         (encodeArgs := fun $args : $argumentType => [$encode $args.1, $encode $args.2.1])
         (encodeResult := $encode) (value := fun $args : $argumentType => $bitResult)
       · $operandAgreement:tactic
       · intro $args:ident $(mkIdent `state):ident
         simp only [LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.evaluate?,
           LeanerIR.Proofs.Denotation.liftPrimitiveEvaluator,
           LeanerIR.SemanticOperations.bitwiseBinary, List.toList_toArray,
           LeanerIR.Proofs.ComputationAgreement.bitwiseBinaryInteger_native
             $(Syntax.mkNatLit context.width) (by decide)]
         rfl))
    else if isShift then `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.scalar_operation_result
         (encodeArgs := $(operandProduct.encode)) (encodeResult := $encode)
         (result := fun $args : $argumentType => $nativeResult)
       · $operandAgreement:tactic
       · intro $args:ident $(mkIdent `state):ident
         exact LeanerIR.Proofs.ComputationAgreement.shift_evaluate_result
           $(Syntax.mkNatLit context.width) (by decide) $signedTerm $shiftLeft
           $kind $args.1 $rightValue _ _))
    else if isDivision then do
      let rule := mkIdent (if isRemainder then ``LeanerIR.Proofs.ComputationAgreement.remainder_evaluate_result
        else ``LeanerIR.Proofs.ComputationAgreement.division_evaluate_result)
      `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.scalar_operation_result
           (encodeArgs := fun $args : $argumentType => [$encode $args.1, $encode $args.2.1])
           (encodeResult := $encode) (result := fun $args : $argumentType => $nativeResult)
         · $operandAgreement:tactic
         · intro $args:ident $(mkIdent `state):ident
           exact $rule $(Syntax.mkNatLit context.width) $signedTerm $kind
             $leftValue $rightValue _ _ _ _ rfl))
    else `(tactic|
    (apply LeanerIR.Proofs.ComputationAgreement.scalar_checkedOperation
       (encode := fun $args : $argumentType => [$encode $args.1, $encode $args.2.1])
       (value := fun $args : $argumentType => $value)
     · $operandAgreement:tactic
     · intro $args:ident $(mkIdent `state):ident
       by_cases $(mkIdent `fits):ident : LeanerIR.IntegerValueFits
         $widthTerm $signedTerm $value
       · simp only [dif_pos $(mkIdent `fits):ident,
           LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.evaluate?,
           LeanerIR.SemanticOperations.checkedBinaryInteger]
         rw [LeanerIR.Proofs.ComputationAgreement.checkedInteger_fits
           _ _ _ _ _ _ rfl $(mkIdent `fits):ident]
         rfl
       · simp only [dif_neg $(mkIdent `fits):ident,
           LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.evaluate?,
           LeanerIR.SemanticOperations.checkedBinaryInteger]
         rw [LeanerIR.Proofs.ComputationAgreement.checkedInteger_overflow
           _ _ _ _ _ _ rfl $(mkIdent `fits):ident]
         rfl))
  return ⟨computation, verifyWith, preserves, agreement⟩

end LeanerLang.NativeExpression
