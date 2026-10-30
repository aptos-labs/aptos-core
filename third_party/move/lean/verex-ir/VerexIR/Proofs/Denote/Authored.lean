-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.TypedMaps
import VerexIR.Proofs.Denote.NativeMaps

/-! Structured authored proofs. Preparation acts on shared program contexts;
clause alternatives contain ordinary Lean proofs in source order. -/

namespace VerexIR.Proofs.Denote

open Lean Meta Elab Tactic

/-- A fact a proof step gave, by its name (`doc/plan/proof-language.md`,
"Closers"): a hidden mark beside the hypothesis, so that the block's normal
form takes the hypothesis of that name as a rule. The mark depends on
nothing: a rewriting, a substitution or a case split carries it along as
they carry the hypothesis's name. -/
structure BlockFact (name : String) : Prop where
  intro ::

/-- A path's context prepared once for the obligations a proof's `on` steps
prove (`verex_step_prepare`): a hidden mark the leaves split from the path
carry, so that each leaf prepares only what its clause adds. -/
structure PathPrepared : Prop where
  intro ::

/-- Prepare a completed path's context once, before its clauses split
(`verex_prepare values`), where a proof's `on` step takes one of its clauses,
and mark the goals `PathPrepared`. -/
elab "verex_step_prepare" : tactic => focusGoal do
  -- Only a path with a clause a proof's `on` step takes.
  unless ← (← targetState).pathRouted.get do return
  evalTactic (← `(tactic| verex_prepare values))
  let marked ← (← getGoals).mapM fun goal => goal.withContext do
    let (mark, goal) ← (← goal.assert `pathPrepared (mkConst ``PathPrepared)
      (mkConst ``PathPrepared.intro)).intro1P
    goal.withContext do goal.setFVarKind mark .implDetail
    return goal
  setGoals marked

/-- Whether a goal's path was prepared (`PathPrepared`). -/
def pathPrepared (goal : MVarId) : MetaM Bool := goal.withContext do
  (← getLCtx).anyM fun decl => return decl.type.isConstOf ``PathPrepared

/-- The cases of an optional value held as a vector, the layout of a struct
with one vector field (`SemanticOperations.optionalLayout?`): no element,
one, or more, each an equation of the value. -/
theorem optionVector_cases {α : Type} (x : SpecVector α × Unit) :
    (∃ bounded, x = (⟨[].toArray, bounded⟩, ())) ∨
    (∃ value bounded, x = (⟨[value].toArray, bounded⟩, ())) ∨
    (∃ first second rest bounded, x = (⟨(first :: second :: rest).toArray, bounded⟩, ())) := by
  obtain ⟨⟨⟨values⟩, bounded⟩, ⟨⟩⟩ := x
  match values with
  | [] => exact Or.inl ⟨bounded, rfl⟩
  | [value] => exact Or.inr (Or.inl ⟨value, bounded, rfl⟩)
  | first :: second :: rest => exact Or.inr (Or.inr ⟨first, second, rest, bounded, rfl⟩)

/-- Apply each top-level preparation step to every surviving case. Nested
blocks retain Lean's ordinary tactic semantics. -/
elab "verex_each" steps:tacticSeq : tactic => do
  let sequence := steps.raw[0]
  let items := if sequence.isOfKind ``Lean.Parser.Tactic.tacticSeqBracketed then
      sequence[1].getSepArgs else sequence[0].getSepArgs
  for step in items do
    evalTactic (← `(tactic| all_goals $(⟨step⟩):tactic))

/-- Prepare selected source values as they become available. Normalize after
one case split before considering the next, so contradictory tags do not
multiply the remaining cases. Nested variant payloads stay intact. -/
elab "verex_prepare" &"cases" " [" values:ident,* "]" : tactic => focusGoal do
  for value in values.getElems do
    let mut remaining := []
    for goal in ← getGoals do
      if ← goal.isAssigned then continue
      setGoals [goal]
      let present ← goal.withContext do
        return ((← getLCtx).findFromUserName? value.getId).isSome
      if present then
        evalTactic (← `(tactic| verex_cases $value:ident))
        evalTactic (← `(tactic| all_goals verex_canonical_families))
        evalTactic (← `(tactic| all_goals verex_normalize_context))
      remaining := remaining ++ (← getUnsolvedGoals)
    setGoals remaining

macro "verex_prepare" &"values" &"splitting" " [" values:ident,* "]" : tactic =>
  `(tactic| (
    verex_prepare values
    all_goals verex_prepare cases [$values,*]
    all_goals verex_normalize_dependent
    all_goals verex_normalize_families))

macro "verex_prepare" &"typed" &"splitting" " [" values:ident,* "]" : tactic =>
  `(tactic| (verex_prepare values splitting [$values,*]; all_goals verex_name_maps))

/-- An integer as the literal the normal form spells: a numeral, negated
where negative. -/
def integerLiteral (value : Int) : MetaM Lean.Expr := do
  let numeral ← mkNumeral (mkConst ``Int) value.natAbs
  if value < 0 then mkAppM ``Neg.neg #[numeral] else pure numeral

/-- Reduce closed map observations directly, avoiding a rewrite proof for
every list cell. Leave symbolic observations available to mathematical lemmas.
These procedures are opt-in: they are not in the global simp inventory. -/
private def evaluateClosedMap : Simp.DSimproc := fun expression => do
  if expression.hasFVar || expression.hasExprMVar || expression.hasLooseBVars then
    return .continue
  let result ← withTransparency .all (whnf expression)
  if result == expression then return .continue
  -- A closed integer the observation computes is a literal.
  if result.isAppOfArity ``Int.ofNat 1 then
    if let some n := (← withTransparency .all (whnf result.appArg!)).rawNatLit? then
      return .done (← integerLiteral (Int.ofNat n))
  return .done result

dsimproc_decl evaluateMapSize (SpecMap.size _) := evaluateClosedMap
dsimproc_decl evaluateMapHasKey (SpecMap.hasKey _ _) := evaluateClosedMap
dsimproc_decl evaluateMapContains (SpecMap.containsB _ _) := evaluateClosedMap
dsimproc_decl evaluateMapValueAt (SpecMap.valueAt _ _) := evaluateClosedMap
dsimproc_decl evaluateMapGet (SpecMap.get _ _) := evaluateClosedMap

/-- A bitwise operation on two integer literals, computed: `eval` decides
ground arithmetic, Move's bitwise operations among it. -/
private def evaluateBitwise (operation : Int → Int → Int) : Simp.DSimproc := fun e => do
  let arguments := e.getAppArgs
  unless arguments.size == 2 do return .continue
  let some left ← getIntValue? arguments[0]! | return .continue
  let some right ← getIntValue? arguments[1]! | return .continue
  return .done (← integerLiteral (operation left right))

dsimproc_decl evaluateBitwiseAnd (VerexIR.Proofs.IntegerArithmetic.bitwiseAnd _ _) :=
  evaluateBitwise VerexIR.Proofs.IntegerArithmetic.bitwiseAnd
dsimproc_decl evaluateBitwiseOr (VerexIR.Proofs.IntegerArithmetic.bitwiseOr _ _) :=
  evaluateBitwise VerexIR.Proofs.IntegerArithmetic.bitwiseOr
dsimproc_decl evaluateBitwiseXor (VerexIR.Proofs.IntegerArithmetic.bitwiseXor _ _) :=
  evaluateBitwise VerexIR.Proofs.IntegerArithmetic.bitwiseXor

/-- Simplify finite maps in the goal and its assumptions. Evaluate closed
observations before unfolding map construction and again after substitutions
expose literal arguments. Symbolic observations keep their `SpecMap` form. -/
macro "verex_map_simp" : tactic => `(tactic| simp_all
  [↓evaluateMapSize, evaluateMapSize, ↓evaluateMapHasKey, evaluateMapHasKey,
    ↓evaluateMapContains, evaluateMapContains, ↓evaluateMapValueAt, evaluateMapValueAt,
    ↓evaluateMapGet, evaluateMapGet,
    SpecMap.update, SpecMap.Discipline.set, SpecMap.ordSet, SpecMap.remove,
    SpecMap.Discipline.del, SpecMap.ordDel, SpecMap.Valid, SpecMap.Ascending,
    SpecMap.intOrder, compare, compareOfLessAndEq])

/-- The preparation a path of typed map facts needs:
projections of returned rows reduced, family spellings made canonical, and
clause markers unwrapped. -/
macro "verex_typed_prepare" : tactic => `(tactic| (
  verex_reduce_projections
  all_goals verex_canonical_families
  all_goals verex_reduce_projections
  all_goals try simp only [VerexIR.Proofs.Obligation_eq] at *))

/-- An equation of an integer literal and a term, stated term first: the
literal is what the term computes to, the direction a rewrite by the
hypotheses takes it in (`verex_typed_close`). -/
elab "verex_orient_literals" : tactic => focusGoal do
  let mut goal ← getMainGoal
  for decl in ← goal.withContext getLCtx do
    if decl.isImplementationDetail then continue
    let type ← goal.withContext (instantiateMVars decl.type)
    let some (_, lhs, rhs) := type.eq? | continue
    let literal (e : Lean.Expr) : MetaM Bool := do
      return (← getIntValue? e).isSome || (← getNatValue? e).isSome
    unless ← goal.withContext (literal lhs) do continue
    if ← goal.withContext (literal rhs) then continue
    let next ← goal.withContext do
      let flipped ← mkEqSymm decl.toExpr
      let (fvarId, next) ← (← goal.assert decl.userName (← inferType flipped) flipped).intro1P
      let _ := fvarId
      next.tryClear decl.fvarId
    goal := next
  replaceMainGoal [goal]

/-- Close a leaf stated over typed maps: the variables equations identify
substituted; then substitute the equations that give maps their values,
evaluate the closed observations, and decide; a literal vector's membership
at its positions; the cheap automatic decision next; a general
simplification by the hypotheses last. -/
macro "verex_typed_close" : tactic => `(tactic| (
  (try verex_subst_vars)
  first
  | (simp only [*, ↓evaluateMapSize, evaluateMapSize, ↓evaluateMapHasKey, evaluateMapHasKey,
      ↓evaluateMapValueAt, evaluateMapValueAt, evaluateBitwiseAnd, evaluateBitwiseOr,
      evaluateBitwiseXor]; done)
  | (simp only [*]; decide)
  -- Observations the hypotheses state of closed maps, evaluated there too,
  -- so that the values they read are known.
  | ((try simp only [↓evaluateMapSize, evaluateMapSize, ↓evaluateMapHasKey, evaluateMapHasKey,
        ↓evaluateMapValueAt, evaluateMapValueAt] at *)
     verex_orient_literals
     simp only [*, ↓evaluateMapSize, evaluateMapSize, ↓evaluateMapHasKey, evaluateMapHasKey,
       ↓evaluateMapValueAt, evaluateMapValueAt, evaluateBitwiseAnd, evaluateBitwiseOr,
       evaluateBitwiseXor]
     done)
  | (verex_goal_witnesses; done)
  | (verex_decide; done)
  | (simp [*, ↓evaluateMapHasKey, evaluateMapHasKey, ↓evaluateMapValueAt, evaluateMapValueAt,
      evaluateBitwiseAnd, evaluateBitwiseOr, evaluateBitwiseXor,
      VerexIR.Proofs.Denote.NativeMaps.get_integers_asInt]; done)))

/-- Generate a function's obligations, prepare shared contexts, and route
source clauses to ordinary Lean proofs. Every top-level preparation step
runs on all surviving cases. Omitted clauses use only `verex_decide`.
No automatic solver runs before an explicitly selected clause argument. -/
syntax (name := verexProof) "verex_proof" (obligationArguments)?
  (&"preparing" tacticSeq)?
  withPosition((ppDedent(ppLine) colGe verexOnAlternative)*) : tactic

elab_rules : tactic
  | `(tactic| verex_proof $[$arguments]? $[preparing $preparation:tacticSeq]?
      $alternatives:verexOnAlternative*) => do
    match preparation with
    | some steps =>
      evalTactic (← `(tactic| verex_obligations $[$arguments]? preparing
        verex_paths
        all_goals verex_clear_computations
        all_goals try verex_trivial
        verex_each $steps:tacticSeq))
    | none => evalTactic (← `(tactic| verex_obligations $[$arguments]?))
    if alternatives.isEmpty then
      evalTactic (← `(tactic| all_goals (verex_paths; all_goals (verex_finish_prepare; verex_decide))))
    else
      let routed ← alternatives.mapM fun alternative => do
        let pattern : TSyntax `verexOnPattern := ⟨alternative.raw[1]⟩
        let steps : TSyntax ``Lean.Parser.Tactic.tacticSeq := ⟨alternative.raw[3]⟩
        `(verexOnAlternative| | $pattern:verexOnPattern =>
          verex_paths
          all_goals (verex_finish_prepare; ($steps:tacticSeq)))
      evalTactic (← `(tactic| verex_on $routed:verexOnAlternative*))

end VerexIR.Proofs.Denote
