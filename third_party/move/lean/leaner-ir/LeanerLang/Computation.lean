-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.Typed
import LeanerLang.Perf
import LeanerLang.Arithmetic
import LeanerLang.NativeCalls
import LeanerLang.NativeSequence
import LeanerIR.Proofs.ComputationAgreement
import LeanerIR.Proofs.Certify

/-!
# Certified native computation generation

Generated fragments include checked binary arithmetic, and value forwarding:
parameter reads/moves and direct calls on that parameter. Bodies have native
parameters/results; their exact execution certificates are assembled separately
from operation laws.
Calls consume a verified native result equation and still owe the callee's
authored precondition. No function or specification text is recognized by name.

The forwarding fragment has one plain argument/result; arithmetic accepts
fixed-width integer parameters and literals. Both forbid extra locals. Effects,
unsupported aggregate operations and type-map-dependent contracts fail closed until their native
operations and agreement laws are available. This module does not import
RowScript or emit a frame-based verification tactic.
-/

namespace LeanerLang.Computation

open Lean Meta Elab Command
open LeanerIR.Proofs.Denotation

-- The declarations below bind identifiers assembled by the generator.
set_option quotPrecheck false

private structure Entry where
  relation : Name
  base : Name
  artifacts : Typed.Artifacts
  rawContract : Name
  typedContract : Name

private initialize entries : SimplePersistentEnvExtension Entry (NameMap Entry) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun state entry => state.insert entry.relation entry
    addImportedFn := fun imported => mkStateFromImportedEntries
      (fun state entry => state.insert entry.relation entry) {} imported }

private def rooted (name : Name) : Ident := mkIdent (rootNamespace ++ name)
private def n (name : Name) : Ident := mkIdent name

private partial def index? (expression : Lean.Expr) : Option Nat :=
  (expression.nat? <|> expression.rawNatLit?).orElse fun _ =>
    if expression.getAppNumArgs == 1 then index? (expression.getArg! 0) else none

/-- Whether accessing the parameter consumes its availability. -/
private def parameter? (expression : Lean.Expr) : Option Bool := do
  if expression.isAppOfArity ``localVar 1 then
    guard (index? (expression.getArg! 0) == some 0)
    return false
  guard (expression.isAppOfArity ``nativeLocalOperation 2)
  guard ((expression.getArg! 1).isConstOf ``valuesNil)
  let operation := expression.getArg! 0
  guard (operation.isAppOfArity ``LocalLocationOperation.move 1)
  guard (index? (operation.getArg! 0) == some 0)
  return true

private def sameScalar : Typed.ValueRep → Typed.ValueRep → Bool
  | .int width signed, .int otherWidth otherSigned =>
      width == otherWidth && signed == otherSigned
  | .bool, .bool | .string, .string | .address, .address | .signer, .signer
  | .bytes, .bytes | .unit, .unit | .parameter 0, .parameter 0 => true
  | _, _ => false

/-- Generate native artifacts for the supported fragment. Each declaration
is kernel checked before the entry is made available to callers. -/
private def generate (segments : Array String) (function : String)
    (generated : Denotation.Generated) (artifacts : Typed.Artifacts)
    (twins : Array SpecTypes.TwinInfo)
    (rawContract typedContract : Name) (budget : Nat) : CommandElabM Unit := do
  let moduleName := segments.foldl Name.str .anonymous
  let base := (← getCurrNamespace) ++ Name.str moduleName function
  let env ← getEnv
  if env.contains (base ++ `computation) then
    unless [ `computationVerified, `computationRepresents ].all
        (fun suffix => env.contains (base ++ suffix)) do
      throwError "incomplete native artifacts for `{base}`"
    return
  if ← NativeSequence.generate? segments function generated artifacts twins rawContract typedContract then
    return
  if ← Arithmetic.generate? segments function generated artifacts rawContract typedContract then
    return
  if ← NativeCalls.generate? segments function generated artifacts rawContract typedContract then
    return
  let signature := artifacts.signature
  unless signature.arguments.size == 1 && signature.results.size == 1 &&
      signature.locals.size == 1 && signature.typeParameterCount <= 1 do
    throwError "native generation requires one plain parameter/result and no extra locals; frame/row fallback is disabled"
  let argument := signature.arguments[0]!
  let result := signature.results[0]!
  unless (match argument.kind, result.kind with | .plain, .plain => true | _, _ => false) &&
      sameScalar argument.rep result.rep do
    throwError "native generation does not yet support this value representation; frame/row fallback is disabled"
  let generic := signature.typeParameterCount != 0
  if generic && !(match argument.rep with | .parameter 0 => true | _ => false) then
    throwError "native generic generation requires the parameter's abstract carrier"
  let some bodyValue := (← getEnv).find? generated.body |>.bind (·.value?)
    | throwError "native generation cannot find the lowered body"
  let body := bodyValue.bindingBody!
  let (moved, callee?) ← if let some moved := parameter? body then pure (moved, none)
    else do
      unless (body.isAppOfArity ``nativeCallAt 4 || body.isAppOfArity ``nativeCall 4) &&
          (body.getArg! 1).isAppOf ``Option.none do
        throwError "native generation has no operation law for this body; frame/row fallback is disabled"
      let operands := body.getArg! 3
      unless operands.isAppOfArity ``valuesCons 2 &&
          (operands.getArg! 1).isConstOf ``valuesNil do
        throwError "native calls currently require one parameter operand"
      let some moved := parameter? (operands.getArg! 0)
        | throwError "native calls currently require a parameter read or move"
      let available := entries.getState (← getEnv)
      let callees := (body.getArg! 2).getUsedConstants.filterMap fun name =>
        available.find? name
      let #[callee] := callees
        | throwError "native call requires a previously verified generated native callee"
      pure (moved, some callee)
  if generic && !moved then
    throwError "a generic value retained at exit requires a native loan-export law"
  -- A pure body may still have a storage-dependent contract. Do not pick an
  -- arbitrary invocation map when proving its call precondition.
  let some raw := (← getEnv).find? rawContract |>.bind (·.value?)
    | throwError "missing native contract input"
  if raw.getUsedConstants.contains ``LeanerIR.SemanticOperations.instantiatedTypeId then
    throwError "native forwarding does not yet support type-map-dependent contracts"
  let names := #[`computation, `computationPure, `pureVerified,
    `computationVerified, `computationRepresents].map (base ++ ·)
  let countErrors (messages : MessageLog) :=
    messages.toList.countP (·.severity == .error)
  let errorsBefore := countErrors (← get).messages
  Perf.measureArtifacts s!"{moduleName}::{function} typed" names do
    let carrier := n `Carrier
    let codecs := n `codecs
    let types := n `types
    let args := n `args
    let initial := n `initial
    let permitted := n `permitted
    let executable := n `executable
    let computation := rooted (base ++ `computation)
    let pureEq := rooted (base ++ `computationPure)
    let pureVerified := rooted (base ++ `pureVerified)
    let typeBinders ← if generic then pure #[← `(bracketedBinder| {$carrier : Nat → Type})]
      else pure #[]
    let codecBinders ← if generic then pure #[
        ← `(bracketedBinder| ($codecs : ∀ index,
          LeanerIR.Proofs.Codec ($carrier index) LeanerIR.RuntimeValue)),
        ← `(bracketedBinder| ($types : Array (LeanerIR.TypeId × LeanerIR.TypeId)))]
      else pure #[]
    let inhabitantBinders ← if generic then pure #[← `(bracketedBinder|
        [$(n `carrierInhabited) : ∀ index, Inhabited ($carrier index)])]
      else pure #[]
    let proofBinders := typeBinders ++ inhabitantBinders ++ codecBinders
    let agreementBinders := typeBinders ++ codecBinders
    let argumentType ← if generic then ``($(rooted artifacts.argumentsType) $carrier)
      else pure (⟨(rooted artifacts.argumentsType).raw⟩ : Term)
    let nativeType ← argument.rep.typeSyntax carrier
    let codec ← argument.rep.codecSyntax codecs
    let argumentValue ← ``($(rooted (artifacts.argumentsType ++ argument.name)) $args)
    let contract ← if generic then ``($(rooted typedContract) $types $codecs)
      else pure (⟨(rooted typedContract).raw⟩ : Term)
    let argumentCodec ← if generic then ``($(rooted artifacts.argumentsCodec) $codecs)
      else pure (⟨(rooted artifacts.argumentsCodec).raw⟩ : Term)
    let resultCodec ← if generic then ``($(rooted artifacts.resultsCodec) $codecs)
      else pure (⟨(rooted artifacts.resultsCodec).raw⟩ : Term)
    let invocation ← if generic then pure (⟨types.raw⟩ : Term) else ``(#[])
    let denotation ← if generic then ``($(rooted generated.denotation) $executable $types)
      else ``($(rooted generated.denotation) $executable)
    let expected ← ``(fun ($args : $argumentType) => $argumentValue)
    let summary ← ``(LeanerIR.Proofs.Contract.withStateFrame
      (LeanerIR.Proofs.Contract.withResult $contract $expected))
    let inhabitant ← match argument.rep with
      | .parameter index => ``($(n `carrierInhabited) $(Syntax.mkNatLit index))
      | .int .. => ``(({ default := ⟨0, by decide⟩ } : Inhabited $nativeType))
      | _ => ``((inferInstance : Inhabited $nativeType))
    let calleeApply (callee : Entry) (suffix : Name) : CommandElabM Term := do
      let name := rooted (callee.base ++ suffix)
      if callee.artifacts.signature.typeParameterCount == 0 then return name
      ``($name (Carrier := fun _ => $nativeType))
    let calleeVerified (callee : Entry) : CommandElabM Term := do
      let name := rooted (callee.base ++ `pureVerified)
      if callee.artifacts.signature.typeParameterCount == 0 then return name
      ``($name (Carrier := fun _ => $nativeType)
        (carrierInhabited := fun _ => $inhabitant) (fun _ => $codec) #[])
    let compute ← match callee? with
      | none => ``(LeanerIR.Proofs.Spec.pure $argumentValue)
      | some callee => ``($(← calleeApply callee `computation) ⟨$argumentValue⟩)
    elabCommand (← `(def $computation $typeBinders* ($args : $argumentType) :
        LeanerIR.Proofs.Spec LeanerIR.RuntimeState LeanerIR.Proofs.Failure $nativeType := $compute))
    let pureProof ← match callee? with
      | none => `(tactic| exact LeanerIR.Proofs.Spec.Equiv.refl _)
      | some callee => `(tactic| exact $(← calleeApply callee `computationPure) ⟨$argumentValue⟩)
    elabCommand (← `(theorem $pureEq $typeBinders* ($args : $argumentType) :
        LeanerIR.Proofs.Spec.Equiv ($computation $args)
          (LeanerIR.Proofs.Spec.pure $argumentValue) := by $pureProof:tactic))
    let normalization : Array (TSyntax `Lean.Parser.Tactic.simpLemma) := #[
      ← `(Lean.Parser.Tactic.simpLemma| LeanerIR.Proofs.Contract.withStateFrame),
      ← `(Lean.Parser.Tactic.simpLemma| LeanerIR.Proofs.Contract.withResult),
      ← `(Lean.Parser.Tactic.simpLemma| LeanerIR.Proofs.Contract.typed),
      ← `(Lean.Parser.Tactic.simpLemma| $(rooted typedContract):term),
      ← `(Lean.Parser.Tactic.simpLemma| $(rooted rawContract):term),
      ← `(Lean.Parser.Tactic.simpLemma| $(rooted artifacts.argumentsCodec):term),
      ← `(Lean.Parser.Tactic.simpLemma| $(rooted artifacts.resultsCodec):term)]
    let nativeProof ← match callee? with
      | none => `(tactic|
          (apply LeanerIR.Proofs.satisfies_of_wp
           intro $args:ident $initial:ident $permitted:ident
           simp only [$computation:term, LeanerIR.Proofs.wp_pure, $normalization,*]
             at $permitted:ident ⊢
           leaner_certified_close!))
      | some callee => do
        let witness ← match callee.artifacts.signature.arguments[0]!.rep with
          | .int .. => ``(($argumentValue).val)
          | _ => pure argumentValue
        let calleeNorm := #[
          ← `(Lean.Parser.Tactic.simpLemma| $(rooted callee.typedContract):term),
          ← `(Lean.Parser.Tactic.simpLemma| $(rooted callee.rawContract):term),
          ← `(Lean.Parser.Tactic.simpLemma| $(rooted callee.artifacts.argumentsCodec):term)] ++ normalization
        `(tactic|
          (apply LeanerIR.Proofs.satisfies_of_wp
           intro $args:ident $initial:ident $permitted:ident
           simp only [$normalization,*] at $permitted:ident
           leaner_cases $permitted:ident
           apply LeanerIR.Proofs.wp_mono
             (LeanerIR.Proofs.wp_of_satisfies
               (args := ⟨$argumentValue⟩) (initial := $initial)
               $(← calleeVerified callee)
               (by
                 simp only [$calleeNorm,*]
                 refine ⟨$witness, ?_⟩
                 leaner_certified_close!)
               (by
                 simp only [$calleeNorm,*]
                 leaner_certified_close!))
           · intro $(n `returned):ident $(n `final):ident $(n `established):ident
             obtain ⟨⟨$(n `sameValue):ident, _⟩, $(n `sameState):ident, _⟩ := $(n `established):ident
             change $(n `returned):ident = $argumentValue at $(n `sameValue):ident
             subst $(n `returned):ident $(n `final):ident
             simp only [$normalization,*]
             leaner_certified_close!
           · intro $(n `error):ident $(n `impossible):ident
             simp only [$calleeNorm,*, LeanerIR.Proofs.Obligation_iff] at $(n `impossible):ident <;>
               (leaner_cases $(n `impossible):ident
                leaner_certified_close!)))
    elabCommand (← `(set_option Elab.async false in
      set_option maxHeartbeats $(Syntax.mkNumLit (toString budget)):num in
      theorem $pureVerified $proofBinders* :
        LeanerIR.Proofs.Satisfies $computation $summary := by $nativeProof:tactic))
    let verifiedTerm ← if generic then ``($pureVerified $codecs $types) else pure (⟨pureVerified.raw⟩ : Term)
    elabCommand (← `(theorem $(rooted (base ++ `computationVerified)) $proofBinders* :
        LeanerIR.Proofs.Satisfies $computation $contract :=
      LeanerIR.Proofs.satisfies_of_result
        (LeanerIR.Proofs.satisfies_of_stateFrame $verifiedTerm)))
    let leafProof ← if moved then `(tactic|
        (apply LeanerIR.Proofs.ComputationAgreement.move
         · rfl
         · simp))
      else `(tactic| exact LeanerIR.Proofs.ComputationAgreement.read _ _ _ rfl)
    let expressionProof ← match callee? with
      | none => pure leafProof
      | some callee => do
        let callRule := rooted (if body.isAppOfArity ``nativeCall 4 then
          ``LeanerIR.Proofs.ComputationAgreement.callMonomorphic
          else ``LeanerIR.Proofs.ComputationAgreement.call)
        let calleeRepresents ← if callee.artifacts.signature.typeParameterCount == 0 then
            ``($(rooted (callee.base ++ `computationRepresents)) $executable)
          else ``($(← calleeApply callee `computationRepresents)
            (fun _ => $codec) _ $executable)
        `(tactic|
          (apply $callRule:term
           · apply LeanerIR.Proofs.ComputationAgreement.cons
             · $leafProof:tactic
             · exact LeanerIR.Proofs.ComputationAgreement.nil _
           · intro $(n `state):ident $(n `final):ident $(n `outcome):ident
             apply LeanerIR.Proofs.ComputationAgreement.relation
             exact ($calleeRepresents ⟨$argumentValue⟩).trans
               ((LeanerIR.Proofs.encodeSpec_congr _
                 ($(← calleeApply callee `computationPure) ⟨$argumentValue⟩)).trans
                   (LeanerIR.Proofs.encodeSpec_pure _ $argumentValue))))
    elabCommand (← `(theorem $(rooted (base ++ `computationRepresents)) $agreementBinders*
        ($executable : LeanerIR.Validation.ExecutableUnit) :
        LeanerIR.Proofs.Represents $argumentCodec $resultCodec $computation $denotation := by
      intro $args:ident
      apply LeanerIR.Proofs.Spec.Equiv.trans ?_
        ((LeanerIR.Proofs.encodeSpec_congr $resultCodec ($pureEq $args)).trans
          (LeanerIR.Proofs.encodeSpec_pure $resultCodec $argumentValue)).symm
      apply LeanerIR.Proofs.ComputationAgreement.function (entry := {
        locals := #[some (($codec).encode $argumentValue)]
        loanLocations := LeanerIR.SemanticOperations.parameterLoanLocations #[($codec).encode $argumentValue]
        typeInstantiation := $invocation })
      · simp [LeanerIR.SemanticOperations.nativeInitialFrame?,
          LeanerIR.SemanticOperations.initialLocals, $(rooted artifacts.argumentsCodec):term,
          $(rooted generated.shape):term]
        rfl
      · $expressionProof:tactic
      · rfl
      · simp [LeanerIR.SemanticOperations.frameBorrows, LeanerIR.Proofs.Codec.specInt,
          LeanerIR.Proofs.Codec.bool, LeanerIR.Proofs.Codec.string, LeanerIR.Proofs.Codec.address,
          LeanerIR.Proofs.Codec.signer, LeanerIR.Proofs.Codec.bytes, LeanerIR.Proofs.Codec.unit,
          LeanerIR.SemanticOperations.outermostBorrows, LeanerIR.SemanticOperations.collectPruned,
          LeanerIR.SemanticOperations.borrowEntry?]))
  if countErrors (← get).messages > errorsBefore then
    throwError "native computation generation failed for `{function}`"
  modifyEnv fun env => entries.addEntry env {
    relation := generated.relation, base, artifacts, rawContract, typedContract }

/-- Failed elaboration must not leave an admitted summary available to a
later verification command (including commands following `#guard_msgs`). -/
def ensure (segments : Array String) (function : String)
    (generated : Denotation.Generated) (artifacts : Typed.Artifacts)
    (twins : Array SpecTypes.TwinInfo)
    (rawContract typedContract : Name) (budget : Nat) : CommandElabM Unit := do
  let saved ← get
  let boundedOptions := maxHeartbeats.set ((← getOptions).setBool `Elab.async false) budget
  try
    withScope (fun scope => { scope with opts := boundedOptions }) do
      generate segments function generated artifacts twins rawContract typedContract budget
      if (← get).messages.toList.countP (·.severity == .error) >
          saved.messages.toList.countP (·.severity == .error) then
        throwError "native computation generation failed for `{function}`"
      let moduleName := segments.foldl Name.str .anonymous
      let base := (← getCurrNamespace) ++ Name.str moduleName function
      if artifacts.signature.typeParameterCount == 0 &&
          (← getEnv).contains (base ++ `nativeSummary) &&
          (← getEnv).contains (base ++ `computationState) then
        modifyEnv fun env => NativeRegistry.entries.addEntry env {
          relation := generated.relation, base, artifacts, rawContract, typedContract }
  catch error =>
    modify fun state => { saved with messages := state.messages }
    throw error

end LeanerLang.Computation
