-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean

/-!
# `simp_all`, corrected

A copy of Lean 4.32's `Lean.Meta.SimpAll` with two corrections, and a
context without branch conditions. In `SimpAll.loop`, when a hypothesis is
modified, the theorem set drops the hypothesis's *current* id rather than
only its original one: otherwise a hypothesis modified in two rounds keeps
a stale copy of its first form in the set, and that copy rewrites the
hypothesis's own conjuncts to `True` in the next round, silently dropping
facts a leaf needs. And an equation whose left side occurs in its right
side is no rewrite rule, as rewriting with it would not terminate; for the
same reason a conditional's condition is not a rewrite rule in its branches,
and neither is an equation that closes a cycle with the admitted ones.
Original authors: Leonardo de Moura (Microsoft Corporation, Apache 2.0).
-/

namespace LeanerIR.Proofs.Denote

open Lean Meta
open Simp (Stats SimprocsArray)

/-- Whether an equation's left side occurs in its right side, including
conditional and quantified equations: rewriting with it would not terminate,
so it stays a fact but is no rewrite rule. -/
def selfReferential (type : Expr) : MetaM Bool := do
  forallTelescope type fun _ body => do
    let some (_, left, right) := body.eq? | return false
    -- An instance as rewriting finds one, up to reducible unfolding.
    return (← kabstract right left).hasLooseBVars

/-- Whether rewriting `left` to `right` closes a cycle with the admitted
equations (origin, left side, right side): `left` is reached from `right`
through equations whose left sides occur in the right sides met. Ground
equations without such a cycle rewrite to an end. -/
partial def closesCycle (rules : Array (Origin × Expr × Expr)) (left right : Expr) :
    MetaM Bool := do
  -- What a rule's right side reaches does not depend on the path to it, so
  -- each rule is explored once.
  let explored ← IO.mkRef (∅ : Std.HashSet Nat)
  let rec reaches (term : Expr) : MetaM Bool := do
    if (← kabstract term left).hasLooseBVars then return true
    for h : index in [0:rules.size] do
      if (← explored.get).contains index then continue
      let (_, ruleLeft, ruleRight) := rules[index]
      if (← kabstract term ruleLeft).hasLooseBVars then
        explored.modify (·.insert index)
        if ← reaches ruleRight then return true
    return false
  reaches right

/-- The congruences without the ones that take a conditional's condition as
a rewrite rule into its branches: a condition such as `x = x / k * k`
would rewrite without end, and a leaf splits its conditionals anyway. -/
def withoutBranchConditions (congruences : SimpCongrTheorems) : SimpCongrTheorems :=
  { lemmas := (congruences.lemmas.insert ``ite []).insert ``dite [] }

namespace SimpAll

structure Entry where
  fvarId   : FVarId -- original fvarId
  userName : Name
  id       : Origin -- id of the theorem at `SimpTheorems`
  origType : Expr
  type     : Expr
  proof    : Expr
  deriving Inhabited

structure State where
  modified     : Bool := false
  mvarId       : MVarId
  entries      : Array Entry := #[]
  ctx          : Simp.Context
  simprocs     : SimprocsArray
  usedTheorems : Simp.UsedSimps := {}
  diag         : Simp.Diagnostics := {}
  /-- The admitted equations, by origin, left and right side. -/
  rules        : Array (Origin × Expr × Expr) := #[]

abbrev M := StateRefT State MetaM

/-- Whether a hypothesis of this type is admitted as a rewrite rule: not an
equation whose left side its right side mentions, nor one closing a cycle
with the admitted equations; an admitted equation is recorded. -/
private def admit (id : Origin) (type : Expr) : M Bool := do
  if ← selfReferential type then return false
  let some (_, left, right) := type.eq? | return true
  if ← closesCycle (← get).rules left right then return false
  modify fun s => { s with rules := s.rules.push (id, left, right) }
  return true

/-- Forget an equation no longer in the rule set. -/
private def retire (id : Origin) : M Unit :=
  modify fun s => { s with rules := s.rules.filter (·.1 != id) }

private def initEntries : M Unit := do
  let hs ←  (← get).mvarId.withContext do getPropHyps
  let hsNonDeps ← (← get).mvarId.getNondepPropHyps
  let mut simpThms := (← get).ctx.simpTheorems
  for h in hs do
    unless simpThms.isErased (.fvar h) do
      let localDecl ← h.getDecl
      let proof  := localDecl.toExpr
      let ctx := (← get).ctx
      if ← admit (.fvar h) (← instantiateMVars localDecl.type) then
        simpThms ← simpThms.addTheorem (.fvar h) proof (config := ctx.indexConfig)
        modify fun s => { s with ctx := s.ctx.setSimpTheorems simpThms }
      if hsNonDeps.contains h then
        -- We only simplify nondependent hypotheses
        let type ← instantiateMVars localDecl.type
        let entry : Entry := { fvarId := h, userName := localDecl.userName, id := .fvar h, origType := type, type, proof }
        modify fun s => { s with entries := s.entries.push entry }

private abbrev getSimpTheorems : M SimpTheoremsArray :=
  return (← get).ctx.simpTheorems

private partial def loop : M Bool := do
  modify fun s => { s with modified := false }
  let simprocs := (← get).simprocs
  -- simplify entries
  let entries := (← get).entries
  for h : i in *...entries.size do
    let entry := entries[i]
    let ctx := (← get).ctx
    -- We disable the current entry to prevent it to be simplified to `True`
    let simpThmsWithoutEntry := (← getSimpTheorems).eraseTheorem entry.id
    let ctx := ctx.setSimpTheorems simpThmsWithoutEntry
    let (r, stats) ← simpStep (← get).mvarId entry.proof entry.type ctx simprocs (stats := { (← get) with })
    modify fun s => { s with usedTheorems := stats.usedTheorems, diag := stats.diag }
    match r with
    | none => return true -- closed the goal
    | some (proofNew, typeNew) =>
      unless typeNew == entry.type do
        /- We must erase the `id` for the simplified theorem. Otherwise,
           the previous versions can be used to self-simplify the new version. For example, suppose we have
           ```
            x : Nat
            h : x ≠ 0
            ⊢ Unit
           ```
           In the first round, `h : x ≠ 0` is simplified to `h : ¬ x = 0`.

           It is also important for avoiding identical hypotheses to simplify each other to `True`.
           Example
           ```
           ...
           h₁ : p a
           h₂ : p a
           ⊢ q a
           ```
           `h₁` is first simplified to `True`. If we don't remove `h₁` from the set of simp theorems, it will
           be used to simplify `h₂` to `True` and information is lost.

           We must use `mkExpectedTypeHint` because `inferType proofNew` may not be equal to `typeNew` when
           we have theorems marked with `rfl`.
        -/
        trace[Meta.Tactic.simp.all] "entry.id: {← ppOrigin entry.id}, {entry.type} => {typeNew}"
        -- Erase the entry's current id, not only its original fvar: the
        -- version of a hypothesis modified in an earlier round otherwise
        -- stays in the set and rewrites the hypothesis's own conjuncts to
        -- `True` in a later round.
        let mut simpThmsNew := (← getSimpTheorems).eraseTheorem entry.id
        retire entry.id
        let idNew ← mkFreshId
        if ← admit (.other idNew) typeNew then
          simpThmsNew ← simpThmsNew.addTheorem (.other idNew) (← mkExpectedTypeHint proofNew typeNew) (config := ctx.indexConfig)
        modify fun s => { s with
          modified         := true
          ctx              := ctx.setSimpTheorems simpThmsNew
          entries[i]       := { entry with type := typeNew, proof := proofNew, id := .other idNew }
        }
  -- simplify target
  let mvarId := (← get).mvarId
  let (r, stats) ← simpTarget mvarId (← get).ctx simprocs (stats := { (← get) with })
  modify fun s => { s with usedTheorems := stats.usedTheorems, diag := stats.diag }
  match r with
  | none => return true
  | some mvarIdNew =>
    unless mvarId == mvarIdNew do
      modify fun s => { s with
        modified := true
        mvarId   := mvarIdNew
      }
  if (← get).modified then
    loop
  else
    return false

def main : M (Option MVarId) := do
  initEntries
  if (← loop) then
    return none -- close the goal
  else
    let mvarId := (← get).mvarId
    -- Prior to #2334, the logic here was to re-assert all hypotheses and call `tryClearMany` on them all.
    -- This had the effect that the order of hypotheses was sometimes modified, whether or not any where simplified.
    -- Now we only re-assert the first modified hypothesis,
    -- along with all subsequent hypotheses, so as to preserve the order of hypotheses.
    let mut toAssert := #[]
    let mut toClear := #[]
    let mut modified := false
    for e in (← get).entries do
      if e.type.isTrue then
        -- Do not assert `True` hypotheses
        toClear := toClear.push e.fvarId
      else if modified || e.type != e.origType then
        toClear := toClear.push e.fvarId
        toAssert := toAssert.push { userName := e.userName, type := e.type, value := e.proof }
        modified := true
    let (_, mvarId) ← mvarId.assertHypotheses toAssert
    mvarId.tryClearMany toClear

end SimpAll

def simpAllFixed (mvarId : MVarId) (ctx : Simp.Context) (simprocs : SimprocsArray := #[]) (stats : Stats := {}) : MetaM (Option MVarId × Stats) := do
  mvarId.withContext do
    let (r, s) ← SimpAll.main.run { stats with mvarId, ctx, simprocs }
    if let .some mvarIdNew := r then
      if ctx.config.failIfUnchanged && mvarId == mvarIdNew then
        throwError "simp_all made no progress"
    return (r, { s with })

open Lean.Elab Lean.Elab.Tactic in
/-- `simp_all` with the round bookkeeping corrected (see `SimpAll.loop`). -/
syntax (name := leanerSimpAll) "leaner_simp_all" optConfig (Lean.Parser.Tactic.discharger)? (&" only")?
  (" [" withoutPosition((Lean.Parser.Tactic.simpErase <|> Lean.Parser.Tactic.simpLemma),*,?) "]")? : tactic

open Lean.Elab Lean.Elab.Tactic in
@[tactic leanerSimpAll] def evalLeanerSimpAll : Tactic := fun stx => withMainContext do
  let { ctx, simprocs, .. } ← mkSimpContext stx (eraseLocal := true) (kind := .simpAll) (ignoreStarArg := true)
  let ctx ← Simp.mkContext ctx.config ctx.simpTheorems (withoutBranchConditions ctx.congrTheorems)
  let (result?, _) ← simpAllFixed (← getMainGoal) ctx (simprocs := simprocs)
  match result? with
  | none => replaceMainGoal []
  | some mvarId => replaceMainGoal [mvarId]

end LeanerIR.Proofs.Denote
