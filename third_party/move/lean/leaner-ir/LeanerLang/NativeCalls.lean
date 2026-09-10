-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.NativeRegistry
import LeanerLang.Perf
import LeanerIR.Proofs.CallAgreement
import LeanerIR.Proofs.Certify

/-! Modular native calls, including declared aborts. Runtime agreement uses
separate semantic certificates; caller VCs consume only verified contracts. -/

namespace LeanerLang.NativeCalls

open Lean Meta Elab Command
open LeanerIR.Proofs.Denotation

set_option quotPrecheck false

private def rooted (name : Name) : Ident := mkIdent (rootNamespace ++ name)

private partial def index? (expression : Lean.Expr) : Option Nat :=
  (expression.nat? <|> expression.rawNatLit?).orElse fun _ =>
    if expression.getAppNumArgs == 1 then index? (expression.getArg! 0) else none

def generate? (segments : Array String) (function : String)
    (generated : Denotation.Generated) (artifacts : Typed.Artifacts)
    (rawContract typedContract : Name) : CommandElabM Bool := do
  let some bodyValue := (← getEnv).find? generated.body |>.bind (·.value?) | return false
  let body := bodyValue.bindingBody!
  unless body.isAppOfArity ``nativeCall 4 && (body.getArg! 1).isAppOf ``Option.none do
    return false
  let available := NativeRegistry.entries.getState (← getEnv)
  let callees := (body.getArg! 2).getUsedConstants.filterMap (available.find? ·)
  let #[callee] := callees | return false
  let signature := artifacts.signature
  unless signature.typeParameterCount == 0 && signature.results.size == 1 &&
      signature.arguments.size > 0 && signature.locals.size == signature.arguments.size do
    throwError "native aborting calls require plain parameters and no extra locals"
  let .int width signed := signature.results[0]!.rep
    | throwError "native aborting calls currently require integer results"
  let sameRep : Typed.ValueRep → Bool
    | .int otherWidth otherSigned => width == otherWidth && signed == otherSigned
    | _ => false
  unless signature.results[0]!.kind == .plain &&
      sameRep callee.artifacts.signature.results[0]!.rep &&
      (signature.arguments ++ callee.artifacts.signature.arguments).all
        (fun argument => argument.kind == .plain && sameRep argument.rep) do
    throwError "native aborting calls require matching integer representations"
  let mut remaining := body.getArg! 3
  let mut indices : Array Nat := #[]
  while remaining.isAppOfArity ``valuesCons 2 do
    let expression := remaining.getArg! 0
    unless expression.isAppOfArity ``localVar 1 do
      throwError "native aborting calls currently require parameter reads"
    let some index := index? (expression.getArg! 0)
      | throwError "invalid native call parameter index"
    unless index < signature.arguments.size do throwError "native call operand is not a parameter"
    indices := indices.push index
    remaining := remaining.getArg! 1
  unless remaining.isConstOf ``valuesNil && indices.size == callee.artifacts.signature.arguments.size do
    throwError "native call argument shape does not match its callee"
  let moduleName := segments.foldl Name.str .anonymous
  let base := (← getCurrNamespace) ++ Name.str moduleName function
  let computation := rooted (base ++ `computation)
  let summary := rooted (base ++ `nativeSummary)
  let verified := rooted (base ++ `computationVerified)
  let preserves := rooted (base ++ `computationState)
  let represents := rooted (base ++ `computationRepresents)
  let names := #[`computation, `nativeSummary, `computationVerified,
    `computationState, `computationRepresents].map (base ++ ·)
  Perf.measureArtifacts s!"{moduleName}::{function} typed" names do
    let args := mkIdent `args
    let initial := mkIdent `initial
    let executable := mkIdent `executable
    let argumentType := rooted artifacts.argumentsType
    let resultType ← signature.results[0]!.rep.typeSyntax (mkIdent `Carrier)
    let field (index : Nat) : CommandElabM Term :=
      ``($(rooted (artifacts.argumentsType ++ signature.arguments[index]!.name)) $args)
    let argumentValues ← indices.mapM field
    let witnesses ← argumentValues.mapM fun value => ``(($value).val)
    let calleeArgs ← ``((⟨$argumentValues,*⟩ : $(rooted callee.artifacts.argumentsType)))
    let mut locals : Array Term := #[]
    for index in [:signature.arguments.size] do
      locals := locals.push (← ``(some (LeanerIR.RuntimeValue.integer ($(← field index)).val)))
    let mut operandsProof ← `(tactic| exact LeanerIR.Proofs.ComputationAgreement.nil _)
    for _ in indices.reverse do
      operandsProof ← `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.cons
         · exact LeanerIR.Proofs.ComputationAgreement.read _ _ _ rfl
         · $operandsProof:tactic))
    let norms : Array (TSyntax `Lean.Parser.Tactic.simpLemma) := #[
      ← `(Lean.Parser.Tactic.simpLemma| LeanerIR.Proofs.Contract.withStateFrame),
      ← `(Lean.Parser.Tactic.simpLemma| LeanerIR.Proofs.Contract.typed),
      ← `(Lean.Parser.Tactic.simpLemma| LeanerIR.Proofs.Codec.specInt)] ++
      (← #[typedContract, rawContract, artifacts.argumentsCodec, artifacts.resultsCodec,
          callee.typedContract, callee.rawContract, callee.artifacts.argumentsCodec,
          callee.artifacts.resultsCodec].mapM fun name =>
        `(Lean.Parser.Tactic.simpLemma| $(rooted name):term))
    elabCommand (← `(def $computation ($args : $argumentType) :
        LeanerIR.Proofs.Spec LeanerIR.RuntimeState LeanerIR.Proofs.Failure $resultType :=
      $(rooted (callee.base ++ `computation)) $calleeArgs))
    elabCommand (← `(theorem $summary : LeanerIR.Proofs.Satisfies $computation
        (LeanerIR.Proofs.Contract.withStateFrame $(rooted typedContract)) := by
      apply LeanerIR.Proofs.satisfies_of_wp
      intro $args:ident $initial:ident $(mkIdent `permitted):ident
      simp only [$norms,*] at $(mkIdent `permitted):ident
      leaner_cases $(mkIdent `permitted):ident
      apply LeanerIR.Proofs.wp_of_stateFrame_satisfies
        $(rooted (callee.base ++ `nativeSummary))
          $calleeArgs $initial (by
            simp only [$norms,*]
            refine ⟨$witnesses,*, ?_⟩
            leaner_certified_close!) (fun possible => possible)
      · intro $(mkIdent `result):ident $(mkIdent `output):ident $(mkIdent `normal):ident
        simp only [$norms,*] at $(mkIdent `output):ident $(mkIdent `normal):ident ⊢
        simp only [LeanerIR.Proofs.Obligation_iff] at $(mkIdent `output):ident
        leaner_cases $(mkIdent `output):ident
        leaner_certified_close!
      · intro $(mkIdent `error):ident $(mkIdent `aborted):ident
        simp only [$norms,*] at $(mkIdent `aborted):ident ⊢ <;>
          (simp only [LeanerIR.Proofs.Obligation_iff] at $(mkIdent `aborted):ident
           leaner_cases $(mkIdent `aborted):ident
           leaner_certified_close!)))
    elabCommand (← `(theorem $verified : LeanerIR.Proofs.Satisfies $computation
        $(rooted typedContract) := LeanerIR.Proofs.satisfies_of_stateFrame $summary))
    elabCommand (← `(theorem $preserves ($args : $argumentType) :
        LeanerIR.Proofs.StatePreserving ($computation $args) :=
      $(rooted (callee.base ++ `computationState)) $calleeArgs))
    elabCommand (← `(theorem $represents ($executable : LeanerIR.Validation.ExecutableUnit) :
        LeanerIR.Proofs.Represents $(rooted artifacts.argumentsCodec)
          $(rooted artifacts.resultsCodec) $computation
          ($(rooted generated.denotation) $executable) := by
      intro $args:ident
      apply LeanerIR.Proofs.ComputationAgreement.function_call_represents
        (entry := { locals := #[$locals,*] }) (exit := { locals := #[$locals,*] })
      · rfl
      · $operandsProof:tactic
      · exact (LeanerIR.Proofs.ComputationAgreement.relationSpec_native _ _ _ _ _).trans
          ($(rooted (callee.base ++ `computationRepresents)) $executable $calleeArgs)
      · exact $(rooted (callee.base ++ `computationState)) $calleeArgs
      · intro $(mkIdent `result):ident; rfl
      · simp [LeanerIR.SemanticOperations.frameBorrows,
          LeanerIR.SemanticOperations.outermostBorrows, LeanerIR.SemanticOperations.collectPruned,
          LeanerIR.SemanticOperations.borrowEntry?]))
  return true

end LeanerLang.NativeCalls
