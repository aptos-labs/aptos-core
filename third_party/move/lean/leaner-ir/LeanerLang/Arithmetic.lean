-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Typed
import LeanerLang.Perf
import LeanerIR.Proofs.ArithmeticAgreement
import LeanerIR.Proofs.Certify

/-! Native checked binary arithmetic on typed parameters and integer literals.
This generator uses shared operation and agreement rules, never a frame-based
VC script. -/

namespace LeanerLang.Arithmetic

open Lean Meta Elab Command
open LeanerIR.Proofs.Denotation

set_option quotPrecheck false

private def rooted (name : Name) : Ident := mkIdent (rootNamespace ++ name)

private partial def index? (expression : Lean.Expr) : Option Nat :=
  (expression.nat? <|> expression.rawNatLit?).orElse fun _ =>
    if expression.getAppNumArgs == 1 then index? (expression.getArg! 0) else none

private inductive Operand where
  | parameter (index : Nat)
  | literal (value : Lean.Expr)

/-- Return false only when the body does not belong to this operation family.
Once recognized, unsupported operands and failed proofs are explicit errors. -/
def generate? (segments : Array String) (function : String)
    (generated : Denotation.Generated) (artifacts : Typed.Artifacts)
    (rawContract typedContract : Name) : CommandElabM Bool := do
  let some bodyValue := (← getEnv).find? generated.body |>.bind (·.value?)
    | return false
  let body := bodyValue.bindingBody!
  unless body.isAppOfArity ``nativePrimitiveOperation 2 do return false
  let operation := body.getArg! 0
  let operator := operation.getAppFn.constName!
  unless [``PrimitiveLocationOperation.checkedAdd,
      ``PrimitiveLocationOperation.checkedSubtract,
      ``PrimitiveLocationOperation.checkedMultiply,
      ``PrimitiveLocationOperation.checkedCast].contains operator do return false
  let isCast := operator == ``PrimitiveLocationOperation.checkedCast
  let signature := artifacts.signature
  unless signature.typeParameterCount == 0 && signature.results.size == 1 &&
      signature.locals.size == signature.arguments.size do
    throwError "native checked arithmetic requires plain parameters and no extra locals"
  let result := signature.results[0]!
  let .int (.bits width) signed := result.rep
    | throwError "native checked arithmetic requires a fixed-width integer result"
  unless width > 0 && result.kind == .plain && signature.arguments.all (fun argument =>
      argument.kind == .plain && match argument.rep with
        | .int (.bits otherWidth) otherSigned =>
            otherWidth > 0 && (isCast || (width == otherWidth && signed == otherSigned))
        | _ => false) do
    throwError "native checked arithmetic requires matching fixed-width integer parameters"
  let operands := body.getArg! 1
  unless operands.isAppOfArity ``valuesCons 2 &&
      (if isCast then (operands.getArg! 1).isConstOf ``valuesNil else
        (operands.getArg! 1).isAppOfArity ``valuesCons 2 &&
        ((operands.getArg! 1).getArg! 1).isConstOf ``valuesNil) do
    throwError "native checked arithmetic has an invalid operand count"
  let operand (expression : Lean.Expr) : CommandElabM Operand := do
    if expression.isAppOfArity ``LeanerIR.Proofs.Denotation.value 1 &&
        (expression.getArg! 0).isAppOfArity ``LeanerIR.RuntimeValue.integer 1 then
      let literal := (expression.getArg! 0).getArg! 0
      unless !literal.hasLooseBVars do throwError "native arithmetic literal is not closed"
      return .literal literal
    unless expression.isAppOfArity ``localVar 1 do
      throwError "native checked arithmetic currently requires parameter reads or integer literals"
    let some index := index? (expression.getArg! 0)
      | throwError "invalid native arithmetic parameter index"
    unless index < signature.arguments.size do
      throwError "native arithmetic operand is not a parameter"
    return .parameter index
  let left ← operand (operands.getArg! 0)
  let right ← if isCast then pure left else operand ((operands.getArg! 1).getArg! 0)
  let moduleName := segments.foldl Name.str .anonymous
  let base := (← getCurrNamespace) ++ Name.str moduleName function
  let computation := rooted (base ++ `computation)
  let verified := rooted (base ++ `computationVerified)
  let summary := rooted (base ++ `nativeSummary)
  let preserves := rooted (base ++ `computationState)
  let represents := rooted (base ++ `computationRepresents)
  let names := #[`computation, `nativeSummary, `computationState,
    `computationVerified, `computationRepresents].map (base ++ ·)
  Perf.measureArtifacts s!"{moduleName}::{function} typed" names do
    let args := mkIdent `args
    let executable := mkIdent `executable
    let argumentType := rooted artifacts.argumentsType
    let resultType ← result.rep.typeSyntax (mkIdent `Carrier)
    let argumentValue (index : Nat) : CommandElabM Term :=
      ``(($(rooted (artifacts.argumentsType ++ signature.arguments[index]!.name)) $args).val)
    let operandValue : Operand → CommandElabM Term
      | .parameter index => argumentValue index
      | .literal value => liftTermElabM <| PrettyPrinter.delab value
    let operandProof : Operand → CommandElabM (TSyntax `tactic)
      | .parameter _ => `(tactic| exact LeanerIR.Proofs.ComputationAgreement.read _ _ _ rfl)
      | .literal _ => `(tactic| exact LeanerIR.Proofs.ComputationAgreement.literal _ _)
    let nonnegative : Operand → CommandElabM Term
      | .parameter index => ``((LeanerIR.SpecInt.unsigned_bounds
          ($(rooted (artifacts.argumentsType ++ signature.arguments[index]!.name)) $args)).1)
      | .literal value => do
        let literal ← liftTermElabM <| PrettyPrinter.delab value
        ``((by decide : 0 ≤ ($literal : Int)))
    let leftValue ← operandValue left
    let rightValue ← operandValue right
    let value ← if isCast then pure leftValue
      else if operator == ``PrimitiveLocationOperation.checkedAdd then
        ``($leftValue + $rightValue)
      else if operator == ``PrimitiveLocationOperation.checkedSubtract then
        ``($leftValue - $rightValue)
      else ``($leftValue * $rightValue)
    let widthTerm ← ``(LeanerIR.IntWidth.bits $(Syntax.mkNatLit width))
    let signedTerm ← if signed then ``(true) else ``(false)
    let failure ← liftTermElabM <| PrettyPrinter.delab (operation.getArg! 0)
    let boundsProof ← if !signed && operator == ``PrimitiveLocationOperation.checkedMultiply then
        `(tactic| have $(mkIdent `_productNonnegative) : 0 ≤ $value :=
          Int.mul_nonneg $(← nonnegative left) $(← nonnegative right))
      else `(tactic| skip)
    let mut locals : Array Term := #[]
    for index in [:signature.arguments.size] do
      locals := locals.push (← ``(some (LeanerIR.RuntimeValue.integer $(← argumentValue index))))
    let operandTailProof ← if isCast then
        `(tactic| exact LeanerIR.Proofs.ComputationAgreement.nil _)
      else `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.cons
         · $(← operandProof right):tactic
         · exact LeanerIR.Proofs.ComputationAgreement.nil _))
    elabCommand (← `(def $computation ($args : $argumentType) :
        LeanerIR.Proofs.Spec LeanerIR.RuntimeState LeanerIR.Proofs.Failure $resultType :=
      LeanerIR.Proofs.NativeArithmetic.checkedInteger $widthTerm $signedTerm
        (LeanerIR.Proofs.NativeArithmetic.runtimeFailure $failure) $value))
    elabCommand (← `(theorem $summary : LeanerIR.Proofs.Satisfies $computation
        (LeanerIR.Proofs.Contract.withStateFrame $(rooted typedContract)) := by
      apply LeanerIR.Proofs.satisfies_of_wp
      intro $args:ident $(mkIdent `initial):ident $(mkIdent `permitted):ident
      simp only [$computation:term, LeanerIR.Proofs.NativeArithmetic.wp_checkedInteger,
        LeanerIR.Proofs.Contract.withStateFrame,
        $(rooted typedContract):term, LeanerIR.Proofs.Contract.typed,
        $(rooted rawContract):term, $(rooted artifacts.argumentsCodec):term,
        $(rooted artifacts.resultsCodec):term] at $(mkIdent `permitted):ident ⊢
      leaner_cases $(mkIdent `permitted):ident
      $boundsProof:tactic
      constructor
      · intro $(mkIdent `fits):ident
        simp only [LeanerIR.IntegerValueFits, LeanerIR.Ty.integerValueFits?,
          LeanerIR.Ty.integerBounds?] at $(mkIdent `fits):ident
        leaner_certified_close!
      · intro $(mkIdent `overflow):ident
        simp [LeanerIR.IntegerValueFits, LeanerIR.Ty.integerValueFits?,
          LeanerIR.Ty.integerBounds?] at $(mkIdent `overflow):ident
        leaner_certified_close!))
    elabCommand (← `(theorem $verified : LeanerIR.Proofs.Satisfies $computation
        $(rooted typedContract) := LeanerIR.Proofs.satisfies_of_stateFrame $summary))
    elabCommand (← `(theorem $preserves ($args : $argumentType) :
        LeanerIR.Proofs.StatePreserving ($computation $args) :=
      LeanerIR.Proofs.NativeArithmetic.checkedInteger_preserves _ _ _ _))
    elabCommand (← `(theorem $represents ($executable : LeanerIR.Validation.ExecutableUnit) :
        LeanerIR.Proofs.Represents $(rooted artifacts.argumentsCodec)
          $(rooted artifacts.resultsCodec) $computation
          ($(rooted generated.denotation) $executable) := by
      intro $args:ident
      apply LeanerIR.Proofs.ComputationAgreement.function_checkedInteger
        (entry := { locals := #[$locals,*] }) (exit := { locals := #[$locals,*] })
      · simp [LeanerIR.SemanticOperations.nativeInitialFrame?,
          LeanerIR.SemanticOperations.initialLocals, $(rooted artifacts.argumentsCodec):term,
          $(rooted generated.shape):term, LeanerIR.SemanticOperations.parameterLoanLocations,
          LeanerIR.Proofs.Codec.specInt] <;> rfl
      · apply LeanerIR.Proofs.ComputationAgreement.cons
        · $(← operandProof left):tactic
        · $operandTailProof:tactic
      · intro $(mkIdent `fits):ident $(mkIdent `state):ident
        simp only [LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.evaluate?,
          LeanerIR.SemanticOperations.checkedBinaryInteger,
          LeanerIR.Proofs.Denotation.liftPrimitiveEvaluator, List.toList_toArray]
        rw [LeanerIR.Proofs.ComputationAgreement.checkedInteger_fits
          _ _ _ _ _ _ rfl $(mkIdent `fits):ident]
        rfl
      · intro $(mkIdent `overflow):ident $(mkIdent `state):ident
        simp only [LeanerIR.Proofs.Denotation.PrimitiveLocationOperation.evaluate?,
          LeanerIR.SemanticOperations.checkedBinaryInteger,
          LeanerIR.Proofs.Denotation.liftPrimitiveEvaluator, List.toList_toArray]
        rw [LeanerIR.Proofs.ComputationAgreement.checkedInteger_overflow
          _ _ _ _ _ _ rfl $(mkIdent `overflow):ident]
        rfl
      · intro $(mkIdent `result):ident; rfl
      · simp [LeanerIR.SemanticOperations.frameBorrows,
          LeanerIR.SemanticOperations.outermostBorrows,
          LeanerIR.SemanticOperations.collectPruned, LeanerIR.SemanticOperations.borrowEntry?]))
  return true

end LeanerLang.Arithmetic
