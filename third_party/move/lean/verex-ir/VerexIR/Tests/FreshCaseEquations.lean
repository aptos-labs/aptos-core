-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Base
open VerexIR.Proofs.Denote

-- A single field alias does not determine the other field.
example (record result : Nat × Nat)
    (_invariant : record.1 = record.2)
    (_alias : (record, result, ()).1.1 = (record, result, ()).2.1.1) : True := by
  fail_if_success
    have : result.1 = result.2 := by
      verex_split_hypotheses
      verex_subst_vars
      verex_assumption
  trivial

-- Splitting the result and frame premise produces the field aliases in the
-- same form consumed by substitution, which then closes the invariant goal directly.
example (record result : Nat × Nat)
    (invariant : record.1 = record.2)
    (alias : (record, result, ()).1.1 = (record, result, ()).2.1.1 ∧
      (record, result, ()).1.2 = (record, result, ()).2.1.2) :
    result.1 = result.2 := by
  verex_split_hypotheses
  verex_subst_vars

-- The same constructor aliases can arrive as primitive projections from cases.
open Lean Meta Elab Tactic in
example (record result : Nat × Nat)
    (invariant : record.1 = record.2)
    (alias : (record, result, ()).1.1 = (record, result, ()).2.1.1 ∧
      (record, result, ()).1.2 = (record, result, ()).2.1.2) :
    result.1 = result.2 := by
  run_tac withMainContext do
    let id ← getFVarId (mkIdent `alias)
    let declaration ← id.getDecl
    let type ← Core.transform declaration.type (post := fun e => do
      if e.isAppOfArity ``Prod.fst 3 then return .done (mkProj ``Prod 0 e.appArg!)
      if e.isAppOfArity ``Prod.snd 3 then return .done (mkProj ``Prod 1 e.appArg!)
      return .continue)
    setGoals [← (← getMainGoal).replaceLocalDeclDefEq id type]
  verex_split_hypotheses
  verex_subst_vars
