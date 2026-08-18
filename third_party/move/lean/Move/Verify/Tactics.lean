-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Verify.WP
import Move.Verify.Syntax
import Lean.Elab.Tactic.Location

/-!
# Small proof-language conveniences for source verification

These are deliberately syntax wrappers over the public proof lemmas. They
do not introduce a second verification semantics or hide generated proof
terms behind an opaque tactic.
-/

namespace Move.Verify

open Lean Elab Tactic

/-- Normalize value-level source semantics using the shared inventory. Side
conditions are discharged only by arithmetic, decidability, or an existing
hypothesis, keeping normalization predictable. -/
syntax "spec_norm" (Lean.Parser.Tactic.location)? : tactic

macro_rules
  | `(tactic| spec_norm $[$location]?) =>
      `(tactic| simp (disch := first | omega | decide | assumption)
        only [move_norm] $[$location]?)

/-- Normalize a WP goal using the primitive proof rules. -/
syntax "wp_norm" (Lean.Parser.Tactic.location)? : tactic
macro_rules
  | `(tactic| wp_norm $[$location]?) =>
      `(tactic| simp only [wp_norm] $[$location]?)

/-- Split a source condition, rewrite the corresponding conditional, and
normalize the named hypothesis in both branches. In particular,
`logicalLT_u64` and `logicalLE_u64` turn numeric Move conditions into
natural-number facts without simplifying unrelated hypotheses. For the common
`0 < n` loop guard, the false branch also receives a fact named `nZero`. -/
syntax "move_cases " ident " : " term : tactic
macro_rules
  | `(tactic| move_cases $hypothesis:ident :
        Move.Verify.Source.logicalLT 0 $value:ident) => do
      let zeroFact := mkIdentFrom value
        (Name.mkSimple s!"{value.getId.getString!}Zero")
      let zeroFactTerm : TSyntax `term := ⟨zeroFact.raw⟩
      let zeroFactLemma ←
        `(Lean.Parser.Tactic.simpLemma| $zeroFactTerm:term)
      let hypothesisTerm : TSyntax `term := ⟨hypothesis.raw⟩
      let hypothesisLocation ←
        `(Lean.Parser.Tactic.locationHyp| $hypothesisTerm:term)
      let hypothesisLemma ←
        `(Lean.Parser.Tactic.simpLemma| $hypothesisTerm:term)
      `(tactic|
        by_cases $hypothesis : Move.Verify.Source.logicalLT 0 $value <;>
          simp only [$hypothesisLemma, if_true, if_false,
            Move.Verify.Source.logicalLT_u64] <;>
          simp only [Move.Verify.Source.logicalLT_u64]
            at $hypothesisLocation <;>
          try have $zeroFact : $value = 0 :=
            Move.U64.eq_zero_of_not_pos $hypothesis <;>
          try simp only [$zeroFactLemma])
  | `(tactic| move_cases $hypothesis:ident : $condition:term) => do
      let hypothesisTerm : TSyntax `term := ⟨hypothesis.raw⟩
      let hypothesisLocation ←
        `(Lean.Parser.Tactic.locationHyp| $hypothesisTerm:term)
      let hypothesisLemma ←
        `(Lean.Parser.Tactic.simpLemma| $hypothesisTerm:term)
      `(tactic|
        by_cases $hypothesis : $condition <;>
          simp only [$hypothesisLemma, if_true, if_false,
            Move.Verify.Source.logicalLT_u64,
            Move.Verify.Source.logicalLE_u64] <;>
          simp only [Move.Verify.Source.logicalLT_u64,
            Move.Verify.Source.logicalLE_u64] at $hypothesisLocation)

/-- Finish a numeric source-value goal directly, or first reduce equality of
`U64` values to equality of their exposed natural values. -/
syntax "u64_omega" : tactic
macro_rules
  | `(tactic| u64_omega) =>
      `(tactic| spec_norm <;>
        first | rfl | omega | (apply Move.U64.ext; omega))

private partial def introUntilSatisfies : TacticM Unit := withMainContext do
  let target ← instantiateMVars (← getMainTarget)
  if target.getAppFn.constName? == some ``Move.Verify.Satisfies then
    return
  match target with
  | .forallE .. =>
      let goal ← getMainGoal
      let (_, next) ← goal.intro1P
      replaceMainGoal [next]
      introUntilSatisfies
  | _ =>
      throwError
        "expected generated contract context followed by `Move.Verify.Satisfies`, got {target}"

private def introNamed (name : Name) : TacticM Unit := withMainContext do
  let goal ← getMainGoal
  let (_, next) ← goal.intro name
  replaceMainGoal [next]

private def hasLastName (name : Name) (suffix : String) : Bool :=
  match name with
  | .str _ last => last == suffix
  | _ => false

private partial def hasFixHead : Expr → Bool
  | .lam _ _ body _ => hasFixHead body
  | .letE _ _ _ body _ => hasFixHead body
  | expression =>
      expression.getAppFn.constName? == some ``Move.Semantics.Spec.fix

private def targetUsesFix : TacticM Bool := withMainContext do
  let target ← instantiateMVars (← getMainTarget)
  unless target.getAppFn.constName? == some ``Move.Verify.Satisfies do
    throwError "expected a `Move.Verify.Satisfies` goal, got {target}"
  let arguments := target.getAppArgs
  if h : 2 ≤ arguments.size then
    let function := arguments[arguments.size - 2]'(by omega)
    return hasFixHead function
  return false

/-- Open the generated contract at the current goal and switch to weakest-
precondition reasoning. The source function is recovered from a goal of the
form `f.contract`. Nonrecursive functions use `satisfies_of_wp`; recursive
functions unfold `f.sourceSpec`, use `satisfies_fix_of_wp`, and expose
`recursive` and `recursiveVerified`. In both cases the authored source body is
unfolded and the remaining binders are named `args`, `initial`, and
`permitted`. -/
syntax (name := contractIntro) "contract_intro" : tactic

@[tactic contractIntro]
private def elabContractIntro : Tactic := fun stx => withMainContext do
  let target ← instantiateMVars (← getMainTarget)
  let some contractName := target.getAppFn.constName?
    | throwErrorAt stx
        "`contract_intro` must start on a generated goal of the form `f.contract`"
  let .str functionName "contract" := contractName
    | throwErrorAt stx
        "`contract_intro` expected a generated `f.contract` goal, got `{contractName}`"
  let sourceSpecName := functionName ++ `sourceSpec
  let bodySpecName := functionName ++ `bodySpec
  let env ← getEnv
  unless env.contains sourceSpecName do
    throwErrorAt stx
      "`contract_intro` supports effectful source contracts, but `{sourceSpecName}` is not defined"
  if let some info := env.find? sourceSpecName then
    if let some value := info.value? (allowOpaque := true) then
      for dependency in value.getUsedConstants do
        if hasLastName dependency "mutualSourceSpec" then
          throwErrorAt stx
            "`contract_intro` does not yet open mutually recursive contract families; use `satisfies_fixFamily` explicitly"
  let contract := mkIdentFrom stx contractName
  let sourceSpec := mkIdentFrom stx sourceSpecName
  evalTactic (← `(tactic| unfold $contract))
  introUntilSatisfies
  withMainContext do
    evalTactic (← `(tactic| unfold $sourceSpec))
    evalTactic (← `(tactic|
      try simp only [Move.Semantics.Spec.pure_bind]))
    if ← targetUsesFix then
      let bodySpec := mkIdentFrom stx bodySpecName
      evalTactic (← `(tactic| apply Move.Verify.satisfies_fix_of_wp))
      introNamed `recursive
      introNamed `recursiveVerified
      introNamed `args
      introNamed `initial
      introNamed `permitted
      if env.contains bodySpecName then
        evalTactic (← `(tactic| unfold $bodySpec))
    else
      evalTactic (← `(tactic| apply Move.Verify.satisfies_of_wp))
      introNamed `args
      introNamed `initial
      introNamed `permitted

/-- Step through a bound call using its established contract. This leaves the
normal-postcondition weakening and abort forwarding obligations as the two
remaining goals. -/
syntax "wp_call " term " using " term : tactic
macro_rules
  | `(tactic| wp_call $verified:term using $permitted:term) =>
      `(tactic| rw [Move.Verify.wp_bind] <;>
        apply Move.Verify.wp_mono
          (Move.Verify.wp_of_satisfies $verified $permitted))

end Move.Verify
