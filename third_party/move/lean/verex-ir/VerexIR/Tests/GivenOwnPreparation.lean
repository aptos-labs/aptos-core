-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Base
import VerexIR.Proofs.Denote.GivenOwn

open Lean Meta Elab Tactic VerexIR.Proofs.Denote
namespace VerexIR.Tests.GivenOwnPreparation

abbrev ownType : NTy := .struct ⟨⟨0⟩, 0⟩ (.cons (.param 0) .nil) (.cons (.param 0) .nil)
def declarations (G : GivenFrame) (predicate : @NTy.plain G.toCarriers ownType → Prop) :
    GivenOwnDeclarations G := fun _ _ => some ⟨ownType, predicate⟩

example (G : GivenFrame) (predicate : @NTy.plain G.toCarriers ownType → Prop)
    (value : @NTy.carrier (G.withDeclarations (declarations G predicate)).toCarriers ownType) :
    predicate value.val := by
  run_tac do
    let value := mkFVar (← getFVarId (mkIdent `value))
    let some proof ← certifiedClauses? value
      | throwError "given certificate did not expose its declared predicate"
    (← getMainGoal).assign proof
    replaceMainGoal []
-- A mutable current supplies an explicit own hypothesis, independently of its
-- certified prophecy. Decode this hypothesis at the current's own row.
example (G : GivenFrame) (predicate : @NTy.plain G.toCarriers ownType → Prop)
    (value : @NTy.plain G.toCarriers ownType)
    (held : @Carriers.own (G.withDeclarations (declarations G predicate)).toCarriers
      ⟨⟨0⟩, 0⟩ (.cons (.param 0) .nil)
      (@NTy.plainEncode G.toCarriers ownType value)) : predicate value := by
  verex_split_hypotheses
  assumption

-- A plain value carries no declaration proof to expose.
example (G : GivenFrame) (value : @NTy.plain G.toCarriers ownType) : True := by
  run_tac do
    let value := mkFVar (← getFVarId (mkIdent `value))
    if (← certifiedClauses? value).isSome then
      throwError "a plain row unexpectedly supplied a nominal certificate"
  trivial

-- Observe the selected clause before its canonicalizer runs. The clause's
-- record projection and beta application must expose the predicate at the
-- actual value, so construction-time readers see resolved type arguments.
example (G : GivenFrame) (predicate : @NTy.plain G.toCarriers ownType → Prop)
    (value : @NTy.carrier (G.withDeclarations (declarations G predicate)).toCarriers ownType) :
    predicate value.val := by
  run_tac do
    let predicate := mkFVar (← getFVarId (mkIdent `predicate))
    let value := mkFVar (← getFVarId (mkIdent `value))
    assertCertifiedClauses (fun statement => do
      unless statement.getAppFn == predicate do
        throwError "selected clause did not expose its actual predicate"
      return statement) (some #[value])
  assumption

end VerexIR.Tests.GivenOwnPreparation
