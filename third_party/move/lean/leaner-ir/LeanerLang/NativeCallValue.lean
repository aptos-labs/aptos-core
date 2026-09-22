-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang.NativeRegistry
import LeanerLang.NativeData
import LeanerIR.Proofs.ScalarAgreement
import LeanerIR.Proofs.Certify

/-! Modular native calls with pure typed arguments, including constructors
and selected payloads. Callee bodies stay out of caller verification. -/

namespace LeanerLang.NativeCallValue

open Lean Elab Command
open LeanerIR.Proofs.Denotation

set_option quotPrecheck false

private def rooted (name : Name) : Ident := mkIdent (rootNamespace ++ name)

structure Emitted where
  computation : Term
  verify : TSyntax `tactic
  preserves : TSyntax `tactic
  agreement : TSyntax `tactic

def resolve (expression : Lean.Expr) : CommandElabM NativeRegistry.Entry := do
  unless expression.isAppOfArity ``nativeCall 4 && (expression.getArg! 1).isAppOf ``Option.none do
    throwError "native value call requires an ordinary call"
  let available := NativeRegistry.entries.getState (← getEnv)
  let callees := (expression.getArg! 2).getUsedConstants.filterMap (available.find? ·)
  let #[callee] := callees | throwError "native value call requires a verified native callee"
  return callee

def emit (twins : Array SpecTypes.TwinInfo) (rep : Typed.ValueRep)
    (operand : Typed.ValueRep → Lean.Expr → CommandElabM (Term × TSyntax `tactic))
    (expression : Lean.Expr)
    (continuation : Option (Ident × TSyntax `tactic) := none) : CommandElabM Emitted := do
  let callee ← resolve expression
  let signature := callee.artifacts.signature
  let resultMatches := if signature.results.isEmpty then rep == .unit
    else signature.results.size == 1 && signature.results[0]!.kind == .plain && signature.results[0]!.rep == rep
  unless signature.typeParameterCount == 0 && resultMatches &&
      signature.arguments.all (fun argument => argument.kind == .plain || argument.kind == .shared) do
    throwError "native value call requires a monomorphic owned signature"
  let mut remaining := expression.getArg! 3
  let mut values : Array Term := #[]
  let mut witnesses : Array Term := #[]
  let mut proofs : Array (TSyntax `tactic) := #[]
  for argument in signature.arguments do
    unless remaining.isAppOfArity ``valuesCons 2 do throwError "missing native call argument"
    let (value, proof) ← operand argument.rep (remaining.getArg! 0)
    values := values.push value
    proofs := proofs.push proof
    witnesses := witnesses.push (← match argument.rep with
      | .int .. => ``(($value).val)
      | .bool => pure value
      | .twin name #[] => ``($(rooted (name ++ `erase)) $value)
      | .vector .. => do
          let codec ← argument.rep.codecSyntax (mkIdent `codecs)
          ``(($codec).encode $value)
      | _ => throwError "unsupported native call argument representation")
    remaining := remaining.getArg! 1
  unless remaining.isConstOf ``valuesNil do throwError "extra native call argument"
  let args ← ``((⟨$values,*⟩ : $(rooted callee.artifacts.argumentsType)))
  let computation ← ``($(rooted (callee.base ++ `computation)) $args)
  let preserves ← ``($(rooted (callee.base ++ `computationState)) $args)
  let norms : Array (TSyntax `Lean.Parser.Tactic.simpLemma) := #[
    ← `(Lean.Parser.Tactic.simpLemma| LeanerIR.Proofs.Contract.typed),
    ← `(Lean.Parser.Tactic.simpLemma| LeanerIR.Proofs.Codec.specInt),
    ← `(Lean.Parser.Tactic.simpLemma| LeanerIR.Proofs.Codec.bool),
    ← `(Lean.Parser.Tactic.simpLemma| LeanerIR.Proofs.Codec.unit),
    ← `(Lean.Parser.Tactic.simpLemma| LeanerIR.Proofs.Codec.boundedVector_encode),
    ← `(Lean.Parser.Tactic.simpLemma| LeanerIR.Proofs.Codec.vector_encode)] ++
    (← (#[callee.typedContract, callee.rawContract, callee.artifacts.argumentsCodec,
        callee.artifacts.resultsCodec] ++ twins.flatMap (fun twin =>
          #[twin.twin ++ `codec, twin.twin ++ `erase])).mapM fun name =>
      `(Lean.Parser.Tactic.simpLemma| $(rooted name):term))
  let pre ← if witnesses.isEmpty then `(tactic| leaner_certified_close!) else
    `(tactic|
      (refine ⟨$witnesses,*, ?_⟩
       simp only [$norms,*] <;>
         (leaner_native_data <;> leaner_certified_close!)))
  let result := continuation.map (·.1) |>.getD (mkIdent `result)
  let next ← match continuation with
    | some (_, proof) => pure proof
    | none => `(tactic| leaner_certified_close!)
  let normalize ← if continuation.isSome then
    `(tactic|
      simp only [$norms,*] at $(mkIdent `output):ident $(mkIdent `normal):ident)
    else `(tactic|
      simp only [$norms,*] at $(mkIdent `output):ident $(mkIdent `normal):ident ⊢)
  let data ← if continuation.isSome then `(tactic| leaner_native_hypotheses)
    else `(tactic| (leaner_native_hypotheses <;> leaner_native_data at *))
  let verify ← `(tactic|
    (leaner_native_lengths
     apply LeanerIR.Proofs.wp_of_stateFrame_satisfies
       $(rooted (callee.base ++ `nativeSummary)) $args _ (by
         simp only [$norms,*]
         $pre:tactic) (fun possible => possible)
     · intro $result:ident $(mkIdent `output):ident $(mkIdent `normal):ident
       $normalize:tactic
       simp (config := { failIfUnchanged := false }) only
         [Array.mk.injEq, List.cons.injEq, LeanerIR.RuntimeValue.bool.injEq,
           LeanerIR.RuntimeValue.integer.injEq, and_true, true_and, and_assoc,
           exists_and_left, exists_and_right, exists_eq_left, exists_eq_right,
           exists_eq_left', exists_eq_right', Classical.not_not] at $(mkIdent `normal):ident
       simp (config := { failIfUnchanged := false }) only
         [LeanerIR.Proofs.Obligation_iff, lir_data_norm, String.reduceBEq,
           beq_self_eq_true, Bool.false_eq_true, Bool.true_eq_false, ite_true, ite_false]
         at $(mkIdent `output):ident
       leaner_cases $(mkIdent `output):ident
       leaner_native_lengths
       $data:tactic <;> leaner_native_array_sizes <;> $next:tactic
     · intro $(mkIdent `error):ident $(mkIdent `aborted):ident
       simp only [$norms,*] at $(mkIdent `aborted):ident ⊢ <;>
         (simp only [LeanerIR.Proofs.Obligation_iff] at $(mkIdent `aborted):ident <;>
          (leaner_cases $(mkIdent `aborted):ident <;> leaner_certified_close!))))
  let mut operandsProof ← `(tactic| exact LeanerIR.Proofs.ComputationAgreement.nil _)
  for proof in proofs.reverse do
    operandsProof ← `(tactic|
      (apply LeanerIR.Proofs.ComputationAgreement.cons
       · $proof:tactic
       · $operandsProof:tactic))
  let resultType ← rep.typeSyntax (mkIdent `Carrier)
  let codec ← rep.codecSyntax (mkIdent `codecs)
  let agreement ← `(tactic|
    (apply LeanerIR.Proofs.ComputationAgreement.scalar_call
       (codec := $(rooted callee.artifacts.resultsCodec))
       (encode := fun result : $resultType => ($codec).encode result)
     · $operandsProof:tactic
     · exact (LeanerIR.Proofs.ComputationAgreement.relationSpec_native _ _ _ _ _).trans
         ($(rooted (callee.base ++ `computationRepresents)) $(mkIdent `executable) $args)
     · exact $preserves
     · intro result; rfl))
  return ⟨computation, verify, ← `(tactic| exact $preserves), agreement⟩

end LeanerLang.NativeCallValue
