-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Typed
import LeanerLang.Perf
import LeanerLang.NativeExpression
import LeanerIR.Proofs.NativeBoundaryAgreement
import LeanerIR.Proofs.NativeMutationAgreement
import LeanerIR.Proofs.NativeUpdateAgreement
import LeanerIR.Proofs.Certify

/-! Native ownership-output emission. Authored clauses are translated by
Contract before this emitter runs; no contract is inferred from the body. -/

namespace LeanerLang.NativeMutable

open Lean Meta Elab Command
open LeanerIR.Proofs.Denotation
set_option quotPrecheck false

private def rooted (name : Name) : Ident := mkIdent (rootNamespace ++ name)

/-- Recognize the mutation operation, never a source function's name or its
contract. Unsupported ownership shapes remain explicit generation gaps. -/
def eligible (signature : Typed.SignatureInfo) (body : Lean.Expr) : Bool :=
  signature.typeParameterCount == 0 && signature.arguments.size == 1 &&
  signature.locals.size == 1 && signature.results.isEmpty &&
  signature.arguments[0]!.kind == .mutable &&
  (match signature.arguments[0]!.rep with | .int (.bits width) _ => width > 0 | _ => false) &&
  body.isAppOfArity ``nativeReferenceOperation 2 &&
  (body.getArg! 0).isConstOf ``ReferenceLocationOperation.mutate

def generate (base : Name) (generated : Denotation.Generated) (artifacts : Typed.Artifacts)
    (rawContract typedContract : Name) (body : Lean.Expr) : CommandElabM Unit := do
  let operands := body.getArg! 1
  unless operands.isAppOfArity ``valuesCons 2 &&
      (operands.getArg! 1).isAppOfArity ``valuesCons 2 &&
      ((operands.getArg! 1).getArg! 1).isConstOf ``valuesNil do
    throwError "native mutable assignment requires a reference and replacement"
  let reference := operands.getArg! 0
  unless reference.isAppOfArity ``localVar 1 &&
      (reference.getArg! 0).isAppOfArity ``LeanerIR.LocalId.mk 1 &&
      (((reference.getArg! 0).getArg! 0).nat? <|>
        ((reference.getArg! 0).getArg! 0).rawNatLit?) == some 0 do
    throwError "native mutable assignment requires its registered parameter"
  let replacement := (operands.getArg! 1).getArg! 0
  let literal? ← if replacement.isAppOfArity ``LeanerIR.Proofs.Denotation.value 1 &&
      (replacement.getArg! 0).isAppOfArity ``LeanerIR.RuntimeValue.integer 1 then do
    pure (some (← liftTermElabM <| PrettyPrinter.delab ((replacement.getArg! 0).getArg! 0)))
  else pure none
  let argument := artifacts.signature.arguments[0]!
  let valueType ← argument.rep.typeSyntax (mkIdent `Carrier)
  let codec ← argument.rep.codecSyntax (mkIdent `codecs)
  let args := mkIdent `args
  let output := mkIdent `output
  let initial := mkIdent `initial
  let middle := mkIdent `middle
  let executable := mkIdent `executable
  let permitted := mkIdent `permitted
  let post := mkIdent `post
  let noAbort := mkIdent `noAbort
  let sameLoan := mkIdent `sameLoan
  let clauses := mkIdent `clauses
  let owner ← ``(LeanerIR.Proofs.MutableArgument $valueType)
  let input ← ``($(rooted (artifacts.argumentsType ++ argument.name)) $args)
  let computation := rooted (base ++ `computation)
  let contract := rooted (base ++ `nativeContract)
  let exitFrame := rooted (base ++ `exitFrame)
  let project := rooted (base ++ `project)
  let commit := rooted (base ++ `commit)
  let commitEq := rooted (base ++ `commit_eq)
  let (nativeBody, nativeProof, executionProof) ← if let some literal := literal? then do
    let body ← ``(LeanerIR.Proofs.Spec.pure (LeanerIR.Proofs.NativeMutation.write $input ⟨$literal, by decide⟩))
    let proof ← `(tactic| leaner_certified_close!)
    let agreement ← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.controlled_pure
       · apply LeanerIR.Proofs.ComputationAgreement.nativeOperation_update
         · exact LeanerIR.Proofs.ComputationAgreement.cons
             (LeanerIR.Proofs.ComputationAgreement.read _ _ _ rfl)
             (LeanerIR.Proofs.ComputationAgreement.cons
               (LeanerIR.Proofs.ComputationAgreement.literal _ _) (LeanerIR.Proofs.ComputationAgreement.nil _))
         · intro state
           exact LeanerIR.Proofs.NativeMutation.write_registered $codec $input
             ⟨$literal, by decide⟩ ⟨0⟩ ($exitFrame $input) state
             (by simp [LeanerIR.SemanticOperations.localLoanPlace?, $exitFrame:term]) rfl
       · exact fun _ => trivial))
    pure (body, proof, agreement)
  else do
    let .int (.bits width) signed := argument.rep
      | throwError "native mutable arithmetic requires a fixed-width integer owner"
    let emitted ← NativeExpression.emit {
      nativeType := valueType, width, signed, slots := #[none], owners := #[some input] } replacement
    let row ← NativeOperands.emit #[{
      type := ← ``(Unit)
      encode := ← ``(fun _ : Unit => ($codec).mutable.encode $input)
      value := {
        computation := ← ``(LeanerIR.Proofs.Spec.pure ())
        verifyWith := fun next => `(tactic| (rw [LeanerIR.Proofs.wp_pure]; $next:tactic))
        preserves := ← `(tactic| exact LeanerIR.Proofs.StatePreserving.pure ())
        agreement := ← `(tactic|
          (apply LeanerIR.Proofs.ComputationAgreement.scalar_pure
            (fun _ : Unit => ($codec).mutable.encode $input) ()
           exact LeanerIR.Proofs.ComputationAgreement.read _ _ _ rfl)) } }, {
      type := valueType, encode := ← ``(fun value : $valueType => ($codec).encode value), value := emitted }]
    let update ← ``(fun values : $(row.type) => LeanerIR.Proofs.NativeMutation.write $input values.2.1)
    let body ← ``(LeanerIR.Proofs.Spec.bind $(row.computation)
      (fun values => LeanerIR.Proofs.Spec.pure ($update values)))
    let after ← row.verifyWith (← `(tactic|
      (rw [LeanerIR.Proofs.wp_pure]
       simp only [LeanerIR.Proofs.NativeMutation.write]
       leaner_certified_close!)))
    let proof ← `(tactic| (rw [LeanerIR.Proofs.wp_bind]; $after:tactic))
    let agreement ← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.controlled_operation_update
         (encodeArgs := $(row.encode)) (value := $update) (encodeResult := fun _ => .unit)
       · $(row.agreement):tactic
       · intro values state
         exact LeanerIR.Proofs.NativeMutation.write_registered $codec $input
           values.2.1 ⟨0⟩ ($exitFrame $input) state
           (by simp [LeanerIR.SemanticOperations.localLoanPlace?, $exitFrame:term]) rfl))
    pure (body, proof, agreement)
  elabCommand (← `(def $computation ($args : $(rooted artifacts.argumentsType)) :
      LeanerIR.Proofs.Spec LeanerIR.RuntimeState LeanerIR.Proofs.Failure $owner :=
    $nativeBody))
  elabCommand (← `(theorem $(rooted (base ++ `computationVerified)) :
      LeanerIR.Proofs.Satisfies $computation $contract := by
    apply LeanerIR.Proofs.satisfies_of_wp
    intro $args:ident $initial:ident $permitted:ident
    simp only [$computation:term, $contract:term, LeanerIR.Proofs.wp_pure,
      $(rooted typedContract):term, LeanerIR.Proofs.Contract.typed,
      $(rooted rawContract):term, $(rooted artifacts.argumentsCodec):term,
      LeanerIR.Proofs.NativeMutation.write] at $permitted:ident ⊢
    leaner_cases $permitted:ident
    $nativeProof:tactic))
  elabCommand (← `(def $exitFrame ($output : $owner) : LeanerIR.RuntimeFrame := {
    locals := #[some (.borrow ($output).loan (.integer ($output).value.val))]
    loanLocations := #[(($output).loan, ⟨.local ⟨0⟩, #[], true⟩)] }))
  elabCommand (← `(def $project (_ : $owner) : Unit := ()))
  elabCommand (← `(def $commit ($output : $owner) (state : LeanerIR.RuntimeState) :
      LeanerIR.RuntimeState :=
    LeanerIR.SemanticOperations.exportReturnedFrameLoans #[] ($exitFrame $output) state))
  elabCommand (← `(theorem $commitEq ($output : $owner) (state : LeanerIR.RuntimeState)
      (noGlobal : LeanerIR.SemanticOperations.globalLoanKey? state ($output).loan = none) :
      $commit $output state = { state with
        pending := state.pending.push (($output).loan, .integer ($output).value.val) } := by
    rw [$commit:term, LeanerIR.SemanticOperations.exportReturnedFrameLoans_noReturnedBorrows _ _ _ rfl]
    exact LeanerIR.SemanticOperations.exportFrameLoans_singleInteger_state
      state ($output).loan ($output).value.val _ _ noGlobal))
  elabCommand (← `(theorem $(rooted (base ++ `computationBoundary)) :
      LeanerIR.Proofs.NativeBoundary.Transports $contract $(rooted typedContract) $project $commit := by
    constructor
    · exact fun _ _ permitted => permitted
    · intro $args:ident $initial:ident $output:ident $middle:ident $permitted:ident $post:ident frame $noAbort:ident
      have sameState : $middle = $initial := frame
      subst $middle:ident
      obtain ⟨$sameLoan:ident, $clauses:ident⟩ := $post $noAbort
      have noGlobal : LeanerIR.SemanticOperations.globalLoanKey? $initial ($input).loan = none := by
        simp only [$(rooted typedContract):term, LeanerIR.Proofs.Contract.typed,
          $(rooted rawContract):term, $(rooted artifacts.argumentsCodec):term] at $permitted:ident
        leaner_cases $permitted:ident
        leaner_certified_close!
      rw [$commitEq $output $initial ($sameLoan ▸ noGlobal)]
      constructor
      · intro _
        have bounds := ($output).value.fits
        simp only [LeanerIR.IntegerValueFits, LeanerIR.Ty.integerValueFits?,
          LeanerIR.Ty.integerBounds?] at bounds
        simp (config := { failIfUnchanged := false }) only [LeanerIR.Proofs.Obligation_iff] at $clauses:ident
        refine ⟨($input).loan, ($input).value.val, ($output).value.val,
          .integer ($output).value.val, ?_⟩
        simp only [$(rooted artifacts.argumentsCodec):term, $(rooted artifacts.resultsCodec):term,
          LeanerIR.Proofs.Codec.mutable, LeanerIR.Proofs.Codec.specInt,
          LeanerIR.SemanticOperations.resolveReturnedBorrows_empty, $project:term,
          $sameLoan:term] at *
        leaner_certified_close!
      · constructor
        · exact ⟨rfl, LeanerIR.SemanticOperations.LoanDiscipline.of_eq rfl (Nat.le_refl _)⟩
        · simp only [$contract:term] at $noAbort:ident
          simp only [$(rooted typedContract):term, LeanerIR.Proofs.Contract.typed,
            $(rooted rawContract):term, $(rooted artifacts.argumentsCodec):term]
          leaner_certified_close!
    · intro $args:ident $initial:ident error _ failed
      simp only [$contract:term, LeanerIR.Proofs.Obligation_iff] at failed
      all_goals simp only [$(rooted typedContract):term, LeanerIR.Proofs.Contract.typed,
        $(rooted rawContract):term, $(rooted artifacts.argumentsCodec):term]
      all_goals first
      | exact failed
      | (refine ⟨($input).loan, ($input).value.val, rfl, ?_⟩
         simpa only [LeanerIR.Proofs.Obligation_iff] using failed)))
  elabCommand (← `(theorem $(rooted (base ++ `computationRepresents))
      ($executable : LeanerIR.Validation.ExecutableUnit) :
      LeanerIR.Proofs.NativeBoundary.Represents $(rooted artifacts.argumentsCodec)
        $(rooted artifacts.resultsCodec) $project $commit $computation
        ($(rooted generated.denotation) $executable) := by
    intro $args:ident
    apply LeanerIR.Proofs.Spec.Equiv.trans
      (LeanerIR.Proofs.ComputationAgreement.function_fromFrame _ _ _ _ _ ($exitFrame $input) ?_) ?_
    · simp [LeanerIR.SemanticOperations.nativeInitialFrame?, LeanerIR.SemanticOperations.initialLocals,
        $(rooted artifacts.argumentsCodec):term, LeanerIR.Proofs.Codec.mutable, LeanerIR.Proofs.Codec.specInt,
        $(rooted generated.shape):term, $exitFrame:term,
        LeanerIR.SemanticOperations.parameterLoanLocations_singleBorrow]
      rfl
    · apply LeanerIR.Proofs.ComputationAgreement.fromFrame_boundary (exit := $exitFrame)
        (control := fun _ => .value .unit)
      · $executionProof:tactic
      · exact fun _ => rfl
      · exact fun _ _ => rfl))

end LeanerLang.NativeMutable
