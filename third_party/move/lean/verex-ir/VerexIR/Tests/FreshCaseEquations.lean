-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Base
open VerexIR.Proofs.Denote

-- Splitting the result and frame premise produces the field aliases in the
-- same form consumed by substitution; the certified fact then closes directly.
example (record : { row : Nat × Nat // row.1 = row.2 })
    (result : Nat × Nat)
    (alias : (record, result, ()).1.val.1 = (record, result, ()).2.1.1 ∧
      (record, result, ()).1.val.2 = (record, result, ()).2.1.2) :
    result.1 = result.2 := by
  have invariant := record.property
  verex_split_hypotheses
  verex_subst_vars
  verex_assumption

-- A single field alias does not determine the other field.
example (record : { row : Nat × Nat // row.1 = row.2 })
    (result : Nat × Nat)
    (_alias : (record, result, ()).1.val.1 = (record, result, ()).2.1.1) : True := by
  fail_if_success
    have same : result.1 = result.2 := by
      have invariant := record.property
      verex_split_hypotheses
      verex_subst_vars
      verex_assumption
  trivial

-- The same constructor aliases can arrive as primitive projections from cases.
open Lean Meta Elab Tactic in
example (record : { row : Nat × Nat // row.1 = row.2 })
    (result : Nat × Nat)
    (alias : (record, result, ()).1.val.1 = (record, result, ()).2.1.1 ∧
      (record, result, ()).1.val.2 = (record, result, ()).2.1.2) :
    result.1 = result.2 := by
  have invariant := record.property
  run_tac withMainContext do
    let id ← getFVarId (mkIdent `alias)
    let declaration ← id.getDecl
    let type ← Core.transform declaration.type (post := fun e => do
      if e.isAppOfArity ``Prod.fst 3 then return .done (mkProj ``Prod 0 e.appArg!)
      if e.isAppOfArity ``Prod.snd 3 then return .done (mkProj ``Prod 1 e.appArg!)
      if e.isAppOfArity ``Subtype.val 3 then return .done (mkProj ``Subtype 0 e.appArg!)
      return .continue)
    setGoals [← (← getMainGoal).replaceLocalDeclDefEq id type]
  verex_split_hypotheses
  verex_subst_vars
  verex_assumption
