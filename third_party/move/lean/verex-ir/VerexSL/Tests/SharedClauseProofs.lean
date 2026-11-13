-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
open Lean Meta Elab Tactic
open VerexIR.Proofs.Denote.SharedContinuations
set_option Elab.async false
namespace VerexSL.Tests.SharedClauseProofs
private def boundary (α : Sort u) (left right : α) : Prop := left = right
attribute [irreducible] boundary
elab "test_shared_clause" : tactic => do
  let goal ← getMainGoal
  let next ← markSharedProof goal
  let some value ← getExprMVarAssignment? goal | throwError "missing shared proof assignment"
  unless sharedProof? value == some (mkMVar next) do throwError "shared proof marker lost"
  replaceMainGoal [next]

theorem authored_dep {α : Type} (x : α) : boundary α x x := by
  verex_proof authored_dep =>
    test_shared_clause
    unfold boundary
    rfl

theorem authored_let (x : Nat) : let y := x; let z := y; boundary Nat z x := by
  intro y z
  verex_proof authored_let =>
    test_shared_clause
    unfold boundary
    rfl

-- The retained let appears in a later binder's type, not only in values.
theorem dependentLetType (n : Nat) : let k := n + 1; ∀ (value : Fin k), value = value := by
  intro k value
  verex_proof dependentLetType =>
    run_tac
      withMainContext do
        let k ← getFVarFromUserName `k
        let value ← getFVarFromUserName `value
        unless (← k.fvarId!.getDecl).isLet &&
            (← inferType value).hasAnyFVar (· == k.fvarId!) do
          throwError "the fixture lost its dependent let binder"
    test_shared_clause
    rfl

theorem authored_siblings (x y : Nat) : boundary Nat x x ∧ boundary Nat y y := by
  verex_proof authored_siblings =>
    constructor
    all_goals
      test_shared_clause
      unfold boundary
      rfl

theorem authored_retry (x : Nat) : boundary Nat x x := by
  verex_proof authored_retry =>
    first
    | test_shared_clause
      fail "discard shared proof attempt"
    | test_shared_clause
      unfold boundary
      rfl

theorem authored_nested (x : Nat) : boundary Nat x x := by
  verex_proof authored_nested =>
    test_shared_clause
    have fresh : True := True.intro
    test_shared_clause
    clear fresh
    unfold boundary
    rfl

elab "test_clause_projections" : tactic => do
  let goal ← getMainGoal
  goal.withContext do
    let target ← instantiateMVars (← goal.getType)
    let some (left, right) := target.and? | throwError "expected a conjunction"
    let combined ← mkFreshExprSyntheticOpaqueMVar target (← goal.getTag)
    let pending ← markSharedProof combined.mvarId!
    goal.assign (mkApp4 (mkConst ``And.intro) left right
      (.proj ``And 0 combined) (.proj ``And 1 combined))
    replaceMainGoal [pending]

theorem sharedProjections (x : Nat) : x = x ∧ x = x := by
  verex_proof sharedProjections =>
    test_clause_projections
    constructor <;> rfl

-- Force Lean to rewrite the entire consumer assignment before finalization.
elab "instantiate_consumer " proof:tacticSeq : tactic => do
  let root ← getMainGoal
  evalTactic proof
  discard <| instantiateMVars (mkMVar root)

theorem earlyRoot (x : Nat) : x = x := by
  verex_proof earlyRoot =>
    instantiate_consumer
      test_shared_clause
      rfl

theorem earlyProjections (x : Nat) : x = x ∧ x = x := by
  verex_proof earlyProjections =>
    instantiate_consumer
      test_clause_projections
      constructor <;> rfl

theorem earlyLambda : ∀ x : Nat, x = x ∧ x = x := by
  verex_proof earlyLambda =>
    instantiate_consumer
      intro x
      test_clause_projections
      constructor <;> rfl

theorem earlyDependentLet : ∀ n : Nat, let k := n + 1;
    ∀ value : Fin k, value = value ∧ value = value := by
  verex_proof earlyDependentLet =>
    instantiate_consumer
      intro n k value
      test_clause_projections
      constructor <;> rfl

run_meta do
  for (name, count) in [( ``authored_dep, 1), (``authored_let, 1), (``authored_siblings, 2),
      (``authored_retry, 1), (``authored_nested, 2), (``sharedProjections, 1), (``dependentLetType, 1), (``earlyRoot, 1), (``earlyProjections, 1), (``earlyLambda, 1), (``earlyDependentLet, 1)] do
    unless (← collectAxioms name).isEmpty do throwError "shared proof acquired axioms: {name}"
    let constants := (← getEnv).constants.toList.map (·.1)
    let cuts := constants.filter fun n => name.isPrefixOf n && (n.toString.splitOn "_verexSharedProof").length > 1
    unless cuts.length == count do logError m!"{name}: expected {count} shared certificates, got {cuts}"

end VerexSL.Tests.SharedClauseProofs
