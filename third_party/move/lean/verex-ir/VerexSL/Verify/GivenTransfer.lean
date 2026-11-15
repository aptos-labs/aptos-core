-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Verify.Contracts
import VerexIR.Proofs.RelationalTransfer

/-!
# Explicit concrete exports of given proofs

A given proof quantifies an interpretation and the contracts of its external
operations. Its concrete adapter asks for witnesses of those binders together
with a relational certificate. The generated assumption contains no proof of
the target function's concrete contract.
-/

namespace VerexSL.Verify

open Lean Meta Elab
open VerexIR.Proofs

private def satisfiesParts (type : Lean.Expr) : MetaM (Lean.Expr × Lean.Expr) := do
  let type := type.consumeMData.headBeta
  let_expr Satisfies _ _ _ _ meaning contract := type
    | throwError "expected a stated Satisfies conclusion, got {type}"
  return (meaning, contract)

private def givenWitnesses (concreteConclusion givenProof : Lean.Expr) : MetaM Lean.Expr := do
  let (concrete, concreteContract) ← satisfiesParts concreteConclusion
  forallTelescope (← inferType givenProof) fun binders conclusion => do
    let (given, givenContract) ← satisfiesParts conclusion
    let certificate ← mkAppM ``Contract.RelationalCertificate
      #[concrete, given, concreteContract, givenContract]
    let mut witnesses ← mkAppM ``Nonempty #[certificate]
    for binder in binders.reverse do
      witnesses ← mkAppM ``Exists #[← mkLambdaFVars #[binder] witnesses]
    return witnesses

/-- Eliminate only the generated witness package. At its leaf the actual
given theorem is applied to all witnessed parameters and operation contracts. -/
private partial def transferFromWitnesses (givenProof evidence : Lean.Expr)
    (witnesses : Array Lean.Expr := #[]) : MetaM Lean.Expr := do
  let type ← whnf (← inferType evidence)
  if type.isAppOfArity ``Exists 2 then
    let domain := type.getArg! 0
    let predicate := type.getArg! 1
    return ← withLocalDeclD `witness domain fun witness =>
      withLocalDeclD `remaining (mkApp predicate witness) fun remaining => do
        let body ← transferFromWitnesses givenProof remaining (witnesses.push witness)
        mkAppM ``Exists.elim #[evidence, ← mkLambdaFVars #[witness, remaining] body]
  mkAppM ``Contract.RelationalCertificate.transfer
    #[evidence, mkAppN givenProof witnesses]

/-- Declare a named propositional transfer assumption and its concrete adapter.
`concreteType` is the existing concrete theorem telescope and conclusion.
`givenProofAt` supplies the proved given theorem specialized to this unit's
fixed parameters; its remaining binders become existential witnesses. The
transfer binder is inserted at `transferIndex` to preserve the existing order
of native, transfer, and in-body-assumption hypotheses.

No witness or transfer evidence is synthesized. The adapter's only new
hypothesis is the named proposition whose definition records every obligation. -/
def declareGivenTransferAdapter (assumptionName adapterName : Name)
    (concreteType : Lean.Expr) (transferIndex : Nat)
    (givenProofAt : Array Lean.Expr → MetaM Lean.Expr)
    (binderName : Name := `givenTransfer) : TermElabM Unit := do
  let assumption ← forallTelescope concreteType fun binders conclusion => do
    if transferIndex > binders.size then
      throwError "transfer binder index exceeds the concrete theorem telescope"
    let witnesses ← givenWitnesses conclusion (← givenProofAt binders)
    mkForallFVars binders witnesses
  addAbbrev assumptionName assumption
  let proof ← forallTelescope concreteType fun binders _ => do
    withLocalDeclD binderName (mkConst assumptionName) fun transfer => do
      let proof ← transferFromWitnesses (← givenProofAt binders) (mkAppN transfer binders)
      let binders := binders.extract 0 transferIndex ++ #[transfer] ++
        binders.extract transferIndex binders.size
      mkLambdaFVars binders proof
  let proof ← instantiateMVars proof
  addDecl (.thmDecl {
    name := adapterName, levelParams := [], type := ← inferType proof, value := proof })

end VerexSL.Verify
