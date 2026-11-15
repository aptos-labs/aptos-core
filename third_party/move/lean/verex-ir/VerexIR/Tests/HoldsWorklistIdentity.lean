-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! # Whole-proposition identity when opening and observing `NTy.Holds`

Two nominal declarations with the same row share the same plain value type, so
one plain value expression can satisfy `NTy.Holds` for distinct handles or
carrier instances. Worklist deduplication when opening `NTy.Holds` hypotheses
(`verex_certified_clauses`) and when instantiating element clauses
(`verex_observed_elements`) keys on the complete proposition (`carriers`, type,
and value after instantiating metavariables and stripping outer metadata),
opening distinct nominal propositions in either hypothesis order while still
opening identical propositions once.
-/

namespace VerexIR.Tests.HoldsWorklistIdentity

open VerexIR VerexIR.Proofs.Denote
open Lean Meta Elab Tactic

private def isHiddenHyp (name : Name) : TacticM Bool := withMainContext do
  for decl in ← getLCtx do
    if decl.userName == name then
      return decl.isImplementationDetail
  throwError "missing local declaration {name}"

private def countVisibleOwn : TacticM Nat := withMainContext do
  let mut ownCount := 0
  for decl in ← getLCtx do
    if decl.isImplementationDetail || decl.isLet then continue
    if (← instantiateMVars decl.type).consumeMData.isAppOfArity ``Carriers.own 4 then
      ownCount := ownCount + 1
  return ownCount

-- Negative control first: `NTy.Holds` on handle `s` never supplies `Carriers.own` for
-- an unrelated handle `t` sharing the same plain carrier.
set_option linter.unusedVariables false in
example [Carriers] (s t : StructHandle)
    (v : (NTy.struct s .nil .nil).carrier)
    (hs : NTy.Holds (.struct s .nil .nil) v) : True := by
  fail_if_success
    have : Carriers.own t .nil (NTy.encode (.struct t .nil .nil) v) := by
      verex_certified_clauses
      assumption
  trivial

-- Negative observation control: `NTy.Holds` on `vector (.struct s ...)` never supplies
-- `Carriers.own` for handle `t` on an observed element `v.values[0]? = some x`.
set_option linter.unusedVariables false in
example [Carriers] (s t : StructHandle)
    (v : (NTy.vector (.struct s .nil .nil)).carrier)
    (x : (NTy.struct s .nil .nil).carrier)
    (hs : NTy.Holds (.vector (.struct s .nil .nil)) v)
    (found : v.values[0]? = some x) : True := by
  fail_if_success
    have : Carriers.own t .nil (NTy.encode (.struct t .nil .nil) x) := by
      verex_observed_elements
      assumption
  trivial

-- Early-opening pair: two handles on one value, `(hs, ht)` order, proving the second.
set_option linter.unusedVariables false in
example [Carriers] (s t : StructHandle)
    (v : (NTy.struct s .nil .nil).carrier)
    (hs : NTy.Holds (.struct s .nil .nil) v)
    (ht : NTy.Holds (.struct t .nil .nil) v) :
    Carriers.own t .nil (NTy.encode (.struct t .nil .nil) v) := by
  verex_certified_clauses
  assumption

-- Early-opening pair: the second hypothesis alone.
example [Carriers] (t : StructHandle)
    (v : (NTy.struct t .nil .nil).carrier)
    (ht : NTy.Holds (.struct t .nil .nil) v) :
    Carriers.own t .nil (NTy.encode (.struct t .nil .nil) v) := by
  verex_certified_clauses
  assumption

-- Early-opening pair in the reverse hypothesis order `(ht, hs)`, proving both handles.
set_option linter.unusedVariables false in
example [Carriers] (s t : StructHandle)
    (v : (NTy.struct s .nil .nil).carrier)
    (ht : NTy.Holds (.struct t .nil .nil) v)
    (hs : NTy.Holds (.struct s .nil .nil) v) :
    Carriers.own s .nil (NTy.encode (.struct s .nil .nil) v) ∧
      Carriers.own t .nil (NTy.encode (.struct t .nil .nil) v) := by
  verex_certified_clauses
  exact ⟨by assumption, by assumption⟩

-- Container hypotheses on the same value with distinct element handles stay visible (neither is hidden as a duplicate of the other).
set_option linter.unusedVariables false in
example [Carriers] (s t : StructHandle)
    (v : (NTy.vector (.struct s .nil .nil)).carrier)
    (hs : NTy.Holds (.vector (.struct s .nil .nil)) v)
    (ht : NTy.Holds (.vector (.struct t .nil .nil)) v) :
    NTy.Holds (.vector (.struct t .nil .nil)) v := by
  verex_certified_clauses
  run_tac do
    if ← isHiddenHyp `hs then throwError "hs should stay visible"
    if ← isHiddenHyp `ht then throwError "ht should not be hidden as a duplicate of hs"
  assumption

-- Exact duplicate hypotheses on `verex_certified_clauses` (including outer metadata on the duplicate): opened once.
set_option linter.unusedVariables false in
example [Carriers] (t : StructHandle)
    (v : (NTy.struct t .nil .nil).carrier)
    (h1 : NTy.Holds (.struct t .nil .nil) v)
    (h2 : NTy.Holds (.struct t .nil .nil) v) :
    Carriers.own t .nil (NTy.encode (.struct t .nil .nil) v) := by
  run_tac do
    withMainContext do
      let fvarId ← getFVarId (mkIdent `h2)
      let decl ← fvarId.getDecl
      let goal ← (← getMainGoal).replaceLocalDeclDefEq fvarId (mkMData {} decl.type)
      setGoals [goal]
  verex_certified_clauses
  run_tac do
    let ownCount ← countVisibleOwn
    unless ownCount == 1 do
      throwError "expected exactly 1 visible Carriers.own hypothesis, found {ownCount}"
    unless ← isHiddenHyp `h1 do throwError "expected h1 to be hidden after opening"
    unless ← isHiddenHyp `h2 do throwError "expected duplicate h2 to be hidden"
  assumption

set_option linter.unusedVariables false in
example [Carriers] (t : StructHandle)
    (v : (NTy.vector (.struct t .nil .nil)).carrier)
    (h1 : NTy.Holds (.vector (.struct t .nil .nil)) v)
    (h2 : NTy.Holds (.vector (.struct t .nil .nil)) v) :
    NTy.Holds (.vector (.struct t .nil .nil)) v := by
  verex_certified_clauses
  run_tac do
    if ← isHiddenHyp `h1 then throwError "expected first container hypothesis h1 to remain visible"
    unless ← isHiddenHyp `h2 do throwError "expected exact duplicate container hypothesis h2 to be hidden"
  assumption

-- Observation pair: vector + membership + both `Holds` in `(hs, ht)` order, proving the second.
set_option linter.unusedVariables false in
example [Carriers] (s t : StructHandle)
    (v : (NTy.vector (.struct s .nil .nil)).carrier)
    (x : (NTy.struct s .nil .nil).carrier)
    (hs : NTy.Holds (.vector (.struct s .nil .nil)) v)
    (ht : NTy.Holds (.vector (.struct t .nil .nil)) v)
    (mem : x ∈ v.values) :
    Carriers.own t .nil (NTy.encode (.struct t .nil .nil) x) := by
  verex_observed_elements
  assumption

-- Observation via `getElem? = some x` (`NTy.Holds.of_getElem?` arm of `observedElementFacts`).
example [Carriers] (t : StructHandle)
    (v : (NTy.vector (.struct t .nil .nil)).carrier)
    (x : (NTy.struct t .nil .nil).carrier)
    (ht : NTy.Holds (.vector (.struct t .nil .nil)) v)
    (found : v.values[0]? = some x) :
    Carriers.own t .nil (NTy.encode (.struct t .nil .nil) x) := by
  verex_observed_elements
  assumption

-- Observation pair: the second hypothesis alone.
example [Carriers] (t : StructHandle)
    (v : (NTy.vector (.struct t .nil .nil)).carrier)
    (x : (NTy.struct t .nil .nil).carrier)
    (ht : NTy.Holds (.vector (.struct t .nil .nil)) v)
    (mem : x ∈ v.values) :
    Carriers.own t .nil (NTy.encode (.struct t .nil .nil) x) := by
  verex_observed_elements
  assumption

-- Observation pair in the reverse hypothesis order `(ht, hs)`, proving both handles.
set_option linter.unusedVariables false in
example [Carriers] (s t : StructHandle)
    (v : (NTy.vector (.struct s .nil .nil)).carrier)
    (x : (NTy.struct s .nil .nil).carrier)
    (ht : NTy.Holds (.vector (.struct t .nil .nil)) v)
    (hs : NTy.Holds (.vector (.struct s .nil .nil)) v)
    (mem : x ∈ v.values) :
    Carriers.own s .nil (NTy.encode (.struct s .nil .nil) x) ∧
      Carriers.own t .nil (NTy.encode (.struct t .nil .nil) x) := by
  verex_observed_elements
  exact ⟨by assumption, by assumption⟩

-- Exact duplicate hypotheses on `verex_observed_elements` (including outer metadata on the duplicate): opened once.
set_option linter.unusedVariables false in
example [Carriers] (t : StructHandle)
    (v : (NTy.vector (.struct t .nil .nil)).carrier)
    (x : (NTy.struct t .nil .nil).carrier)
    (h1 : NTy.Holds (.vector (.struct t .nil .nil)) v)
    (h2 : NTy.Holds (.vector (.struct t .nil .nil)) v)
    (mem : x ∈ v.values) :
    Carriers.own t .nil (NTy.encode (.struct t .nil .nil) x) := by
  run_tac do
    withMainContext do
      let fvarId ← getFVarId (mkIdent `h2)
      let decl ← fvarId.getDecl
      let goal ← (← getMainGoal).replaceLocalDeclDefEq fvarId (mkMData {} decl.type)
      setGoals [goal]
  verex_observed_elements
  run_tac do
    let ownCount ← countVisibleOwn
    unless ownCount == 1 do
      throwError "expected exactly 1 visible Carriers.own hypothesis, found {ownCount}"
  assumption

end VerexIR.Tests.HoldsWorklistIdentity
