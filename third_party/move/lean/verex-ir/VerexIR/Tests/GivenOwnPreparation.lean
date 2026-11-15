-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Base
import VerexIR.Proofs.Denote.GivenOwn

open Lean Meta Elab Tactic VerexIR.Proofs.Denote
namespace VerexIR.Tests.GivenOwnPreparation

local notation "ownType" =>
  NTy.struct ⟨⟨0⟩, 0⟩ (NRow.cons (NTy.param 0) NRow.nil) (NRow.cons (NTy.param 0) NRow.nil)
local notation "otherOwnType" =>
  NTy.struct ⟨⟨0⟩, 1⟩ (NRow.cons (NTy.param 0) NRow.nil) (NRow.cons (NTy.param 0) NRow.nil)

def declarations (G : GivenFrame) (predicate : @NTy.carrier G.toCarriers ownType → Prop) :
    GivenOwnDeclarations G := fun _ _ => some ⟨ownType, predicate⟩

def twoDeclarations (G : GivenFrame)
    (predicate₁ predicate₂ : @NTy.carrier G.toCarriers ownType → Prop) :
    GivenOwnDeclarations G := fun handle _ =>
  if handle == ⟨⟨0⟩, 0⟩ then some ⟨ownType, predicate₁⟩
  else if handle == ⟨⟨0⟩, 1⟩ then some ⟨otherOwnType, predicate₂⟩
  else none

-- 1 (Negative): A plain value without `NTy.Holds` exposes no declaration invariant.
set_option linter.unusedVariables false in
example (G : GivenFrame) (predicate : @NTy.carrier G.toCarriers ownType → Prop)
    (value : @NTy.carrier (G.withDeclarations (declarations G predicate)).toCarriers ownType) :
    True := by
  fail_if_success
    have : predicate value := by
      run_tac assertCertifiedClauses pure none
      assumption
  trivial

-- 2 (Positive): Explicit `NTy.Holds` exposes the resolved `predicate value` before canonicalization,
--    so construction-time and invariant readers see resolved type arguments at the actual value.
example (G : GivenFrame) (predicate : @NTy.carrier G.toCarriers ownType → Prop)
    (value : @NTy.carrier (G.withDeclarations (declarations G predicate)).toCarriers ownType)
    (held : @NTy.Holds (G.withDeclarations (declarations G predicate)).toCarriers ownType value) :
    predicate value := by
  run_tac do
    let predicate := mkFVar (← getFVarId (mkIdent `predicate))
    assertCertifiedClauses (fun statement => do
      unless statement.getAppFn == predicate do
        throwError "selected clause did not expose its actual predicate"
      return statement) none
  assumption

-- 3 (Positive): Whole-proposition `NTy.Holds` identity opens distinct Given declarations sharing
--    the same plain value.
example (G : GivenFrame)
    (predicate₁ predicate₂ : @NTy.carrier G.toCarriers ownType → Prop)
    (value : @NTy.carrier (G.withDeclarations (twoDeclarations G predicate₁ predicate₂)).toCarriers ownType)
    (held₁ : @NTy.Holds (G.withDeclarations (twoDeclarations G predicate₁ predicate₂)).toCarriers ownType value)
    (held₂ : @NTy.Holds (G.withDeclarations (twoDeclarations G predicate₁ predicate₂)).toCarriers otherOwnType value) :
    predicate₁ value ∧ predicate₂ value := by
  run_tac assertCertifiedClauses pure none
  exact ⟨by assumption, by assumption⟩

-- 4 (Positive): Explicit `Carriers.own` on an encoding decodes to `predicate value` under `verex_split_hypotheses`.
example (G : GivenFrame) (predicate : @NTy.carrier G.toCarriers ownType → Prop)
    (value : @NTy.carrier G.toCarriers ownType)
    (held : @Carriers.own (G.withDeclarations (declarations G predicate)).toCarriers
      ⟨⟨0⟩, 0⟩ (.cons (.param 0) .nil)
      (@NTy.encode G.toCarriers ownType value)) : predicate value := by
  verex_split_hypotheses
  assumption

end VerexIR.Tests.GivenOwnPreparation
