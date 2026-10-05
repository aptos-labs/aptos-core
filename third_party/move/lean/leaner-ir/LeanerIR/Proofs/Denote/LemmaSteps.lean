-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Contract

/-!
# Proof steps over lemmas

An `apply` step states a formula over lemma instances
(`LemmaApplication`, `LemmaInstance`) and a `split` step a `CaseSplit`.
Where such a formula is owed, `dischargeLemmaSteps` introduces its premises
and binders, reduces each application to the lemma's premise, and proves
each instance and split; where it holds, `lemmaFacts` turns each application
into the lemma's conclusion. The lemma's theorem is named by elaborating its
identifier: inside its own recursion group that is the recursive reference.
-/

namespace LeanerIR.Proofs.Denote

open Lean Elab Tactic Meta

/-- The lemma theorems established: verified without an error. A theorem
whose proof fails is declared all the same, over `sorry`, and is not used. -/
initialize establishedLemmas : TagDeclarationExtension ← mkTagDeclarationExtension

/-- Whether a lemma definition names a lemma's premise, conclusion, or step. -/
private def isLemmaDefinition : Name → Bool
  | .str _ part => part == "lemmaRequires" || part == "lemmaEnsures" ||
      part.startsWith "lemmaStep_"
  | _ => false

/-- A lemma's premise or conclusion, unfolded, without the markers of its
clauses: an unmet premise is reported where the lemma is applied. -/
def unfoldLemmaDefinitions (e : Expr) : MetaM Expr := do
  let expanded ← deltaExpand (← instantiateMVars e) isLemmaDefinition
  -- The bundle's components, where it is a literal tuple.
  Meta.transform expanded.headBeta (post := fun sub => do
    if sub.isAppOfArity ``Obligation 4 then return .done (sub.getArg! 3)
    if sub.isAppOfArity ``Prod.fst 3 && (sub.getArg! 2).isAppOfArity ``Prod.mk 4 then
      return .done ((sub.getArg! 2).getArg! 2)
    if sub.isAppOfArity ``Prod.snd 3 && (sub.getArg! 2).isAppOfArity ``Prod.mk 4 then
      return .done ((sub.getArg! 2).getArg! 3)
    return .continue)

/-- Whether a formula states a lemma instance or a case split. -/
def statesLemmaStep (e : Expr) : Bool :=
  (e.find? fun sub =>
    sub.isAppOfArity ``LemmaApplication 2 || sub.isAppOfArity ``LemmaInstance 2 ||
      sub.isAppOfArity ``CaseSplit 1).isSome

/-- The components of a bundle: right-nested pairs closed by `()`. -/
private def bundleComponents (bundle : Expr) (count : Nat) : MetaM (Array Expr) := do
  let mut rest := bundle
  let mut components := #[]
  for _ in [0:count] do
    if rest.isAppOfArity ``Prod.mk 4 then
      components := components.push (rest.getArg! 2)
      rest := rest.getArg! 3
    else
      components := components.push (← mkAppM ``Prod.fst #[rest])
      rest ← mkAppM ``Prod.snd #[rest]
  return components

/-- The theorem of the lemma whose premise `owes` is, at what the premise is
applied to: a proof of the premise implying the conclusion. None for a lemma
not established, whose failure is reported at the lemma. -/
def lemmaTheoremAt? (owes : Expr) : TacticM (Option Expr) := do
  let owes ← instantiateMVars owes
  let .const requires _ := owes.getAppFn
    | throwError m!"a lemma instance has no premise definition: {owes}"
  let .str base "lemmaRequires" := requires
    | throwError m!"a lemma instance's premise is not a lemma's: {owes}"
  let arguments := owes.getAppArgs
  let some bundle := arguments.back?
    | throwError m!"a lemma's premise is not applied to a bundle: {owes}"
  let reads := arguments.pop
  let some theoremTerm ← try
      some <$> Term.withoutErrToSorry
        (Lean.Elab.Tactic.elabTerm (mkIdent (Name.str base "lemma")) none)
    catch _ => pure none
    | return none
  -- Inside its recursion group the theorem is the recursive reference.
  if let .const name _ := theoremTerm.getAppFn then
    unless establishedLemmas.isTagged (← getEnv) name do return none
  -- What the lemma assumes, from the hypotheses that state it.
  let mut applied := theoremTerm
  repeat
    let .forallE _ domain _ _ := ← whnfR (← inferType applied) | break
    let .const name _ := domain.getAppFn | break
    let .str _ part := name | break
    unless part.startsWith "lemmaTrusted_" do break
    let some hypothesis ← findLocalDeclWithType? domain
      | throwError m!"the assumption `{name}` of a lemma is not a hypothesis here"
    applied := mkApp applied (mkFVar hypothesis)
  let atReads := mkAppN applied reads
  -- The parameters, then the premise.
  let parameters ← forallTelescopeReducing (← inferType atReads) fun binders _ =>
    pure (binders.size - 1)
  return some (mkAppN atReads (← bundleComponents bundle parameters))

/-- Replace a goal by one of a definitionally equal type. -/
private def retype (goal : MVarId) (type : Expr) : MetaM MVarId := do
  let replacement ← mkFreshExprSyntheticOpaqueMVar type (← goal.getTag)
  goal.assign replacement
  return replacement.mvarId!

/-- Reduce what an owed formula states of lemmas: introduce its premises and
binders, reduce each application to the lemma's premise, and prove each
instance by the lemma's theorem and each case split by the excluded middle;
an instance of a lemma not established is owed as its implication. The
remaining goals, each with whether it is a lemma's premise; an `Obligation`
marker stays on what it covers. -/
partial def dischargeLemmaSteps (goal : MVarId) : TacticM (Array (MVarId × Bool)) :=
  goal.withContext do
    let target ← whnfR (← instantiateMVars (← goal.getType))
    unless statesLemmaStep target do return #[(goal, false)]
    let goal ← retype goal target
    if target.isAppOfArity ``Obligation 4 then
      let parts ← dischargeLemmaSteps (← retype goal (target.getArg! 3))
      parts.mapM fun (part, premise) => part.withContext do
        let type ← instantiateMVars (← part.getType)
        pure (← retype part (mkAppN (target.getAppFn) (target.getAppArgs.pop.push type)),
          premise)
    else if target.isAppOfArity ``And 2 then
      let [left, right] ← goal.apply (mkConst ``And.intro)
        | throwError "a conjunction did not split"
      return (← dischargeLemmaSteps left) ++ (← dischargeLemmaSteps right)
    else if target.isAppOfArity ``LemmaApplication 2 then
      return #[(← retype goal (← unfoldLemmaDefinitions (target.getArg! 0)), true)]
    else if target.isAppOfArity ``LemmaInstance 2 then
      match ← lemmaTheoremAt? (target.getArg! 0) with
      | some proof =>
          unless ← isDefEq (← inferType proof) target do
            throwError m!"a lemma's theorem does not prove its instance {target}"
          goal.assign proof
          return #[]
      | none =>
          let implication ← mkArrow (← unfoldLemmaDefinitions (target.getArg! 0))
            (← unfoldLemmaDefinitions (target.getArg! 1))
          return #[(← retype goal implication, false)]
    else if target.isAppOfArity ``CaseSplit 1 then
      goal.assign (mkApp (mkConst ``CaseSplit.intro) (target.getArg! 0))
      return #[]
    else if target.isForall then
      let (_, inner) ← goal.intro1
      dischargeLemmaSteps inner
    else return #[(goal, false)]

/-- What an owed formula gives where it holds, from a proof of it: each
lemma application the lemma's conclusion, by its theorem, or only its
premise where the lemma is not established, and each instance its
implication; markers are dropped. The proof and its type. -/
partial def lemmaFacts (proof type : Expr) : TacticM (Expr × Expr) := do
  let type ← whnfR (← instantiateMVars type)
  unless statesLemmaStep type do return (proof, type)
  if type.isAppOfArity ``Obligation 4 then
    lemmaFacts proof (type.getArg! 3)
  else if type.isAppOfArity ``And 2 then
    let (left, leftType) ← lemmaFacts (mkProj ``And 0 proof) (type.getArg! 0)
    let (right, rightType) ← lemmaFacts (mkProj ``And 1 proof) (type.getArg! 1)
    return (mkApp4 (mkConst ``And.intro) leftType rightType left right,
      mkApp2 (mkConst ``And) leftType rightType)
  else if type.isAppOfArity ``LemmaApplication 2 then
    match ← lemmaTheoremAt? (type.getArg! 0) with
    | some theoremTerm =>
        return (mkApp theoremTerm proof, ← unfoldLemmaDefinitions (type.getArg! 1))
    | none => return (proof, ← unfoldLemmaDefinitions (type.getArg! 0))
  else if type.isAppOfArity ``LemmaInstance 2 then
    return (proof, ← mkArrow (← unfoldLemmaDefinitions (type.getArg! 0))
      (← unfoldLemmaDefinitions (type.getArg! 1)))
  else if type.isForall then
    forallBoundedTelescope type (some 1) fun binders body => do
      let (inner, innerType) ← lemmaFacts (mkAppN proof binders) body
      return (← mkLambdaFVars binders inner, ← mkForallFVars binders innerType)
  else return (proof, type)

/-- Replace a hypothesis stating an owed formula by the facts it gives. -/
def replaceByLemmaFacts (goal : MVarId) (hypothesis : FVarId) : TacticM MVarId :=
  goal.withContext do
    let type ← instantiateMVars (← hypothesis.getType)
    unless statesLemmaStep type do return goal
    let (proof, factType) ← lemmaFacts (mkFVar hypothesis) type
    return (← goal.replace hypothesis proof factType).mvarId

/-- Split each case split a hypothesis states: the proposition, then its
negation. -/
partial def splitCases (goal : MVarId) : TacticM (List MVarId) := goal.withContext do
  let found ← (← getLCtx).findDeclM? fun decl => do
    if decl.isImplementationDetail then return none
    let type ← instantiateMVars decl.type
    if type.isAppOfArity ``CaseSplit 1 then return some (decl.fvarId, type.getArg! 0)
    return none
  let some (hypothesis, proposition) := found | return [goal]
  let disjunction := mkApp2 (mkConst ``Or) proposition (mkNot proposition)
  let goal ← goal.replaceLocalDeclDefEq hypothesis disjunction
  let cases ← goal.cases hypothesis
  cases.toList.flatMapM fun subgoal => splitCases subgoal.mvarId

end LeanerIR.Proofs.Denote
