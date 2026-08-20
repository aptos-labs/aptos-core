-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Verify.WP
import Move.Verify.Borrow
import Move.Verify.Syntax
import Lean.Elab.Tactic.Location

/-!
# Small proof-language conveniences for source verification

These are deliberately syntax wrappers over the public proof lemmas. They
do not introduce a second verification semantics or hide generated proof
terms behind an opaque tactic.

This module is also the single inventory for the shared verification simp
sets registered in `Move.Verify.SimpAttrs`:

- `move_norm` normalizes value-level source semantics: `U64` numerals,
  logical comparison operators, `Spec` monad laws, and the conditional
  `*_eq_pure` equations whose side conditions a proof context can discharge.
- `wp_norm` (populated at the rule definitions in `Move.Verify.WP` and
  `Move.Verify.Borrow`) rewrites weakest-precondition obligations.
- `move_spec` unfolds the raw relational semantics; it is the lemma
  inventory of the automatic `verify f` command, and user proofs or
  project-specific summaries can extend it with `@[move_spec]`.
-/

attribute [move_norm]
  Nat.mod_eq_of_lt
  MoveModel.IR.IntWidth.size
  MoveModel.IR.IntWidth.bits
  Move.U8.size
  Move.U16.size
  Move.U32.size
  Move.U64.size
  Move.U128.size
  Move.U256.size
  Move.widthOf_W8
  Move.widthOf_W16
  Move.widthOf_W32
  Move.widthOf_W64
  Move.widthOf_W128
  Move.widthOf_W256
  Move.width_W8
  Move.width_W16
  Move.width_W32
  Move.width_W64
  Move.width_W128
  Move.width_W256
  Move.UInt.add_def
  Move.UInt.sub_def
  Move.UInt.mul_def
  Move.UInt.div_def
  Move.UInt.mod_def
  Move.UInt.land_def
  Move.UInt.lor_def
  Move.UInt.lxor_def
  Move.UInt.shl_def
  Move.UInt.shr_def
  Move.UInt.cast_def
  Move.UInt.toNat_ofNat_sub
  Move.UInt.toNat_ofNat_div
  Move.UInt.toNat_ofNat_mod
  Move.UInt.toNat_ofNat_land
  Move.UInt.toNat_ofNat_lor
  Move.UInt.toNat_ofNat_lxor
  Move.UInt.toNat_ofNat_shr
  Move.UInt.toNat_ofNat
  Move.UInt.toNat_ofNat_numeral
  Move.UInt.toNat_zero
  Move.UInt.toNat_one
  Move.UInt.lt_iff_toNat_lt
  Move.Vector.length_toNat
  Move.Vector.toList_empty
  Move.Vector.toList_push
  Move.Vector.toList_set
  Move.Vector.toList_ofList
  Move.Verify.Source.logicalLT_uint
  Move.Verify.Source.logicalLE_uint
  Move.Verify.Source.logicalBEq_uint
  Move.Verify.Source.logicalLT_move
  Move.Verify.Source.logicalBEq_move
  Move.Semantics.Spec.pure_bind
  Move.Semantics.Spec.bind_pure
  Move.Semantics.Spec.abort_bind
  Move.Semantics.Spec.fix_const
  Move.Semantics.Checked.addSpec_eq_pure
  Move.Semantics.Checked.subSpec_eq_pure
  Move.Semantics.Checked.mulSpec_eq_pure
  Move.Semantics.Checked.divSpec_eq_pure
  Move.Semantics.Checked.modSpec_eq_pure
  Move.Semantics.Checked.shlSpec_eq_pure
  Move.Semantics.Checked.shrSpec_eq_pure
  Move.Semantics.Checked.castSpec_eq_pure
  Move.Semantics.Vector.borrowElemSpec_eq_pure
  Move.Verify.withBorrowElemMutSpec_write_eq_pure

attribute [move_spec]
  Id.run
  Bind.bind
  Pure.pure
  Move.Semantics.ResourceStore.contains
  Move.Semantics.ResourceStore.get
  Move.Semantics.ResourceStore.descriptor
  Move.Semantics.Resource.withBorrowMutFocusSpec
  Move.Semantics.Resource.withBorrowMutSpec_ok
  Move.Semantics.Resource.withBorrowMutSpec_aborts
  Move.Semantics.Resource.withBorrowMutSpec_undefined
  Move.Semantics.Vector.borrowElemSpec
  Move.Semantics.Vector.withBorrowElemMutSpec
  Move.Semantics.Vector.insertSpec
  Move.Semantics.Vector.removeSpec
  Move.Semantics.withMutation_ok
  Move.Semantics.withMutation_aborts
  Move.Semantics.withMutation_undefined
  Move.Semantics.Spec.bind_ok
  Move.Semantics.Spec.bind_aborts
  Move.Semantics.Spec.bind_undefined
  Move.Semantics.Spec.pure_ok
  Move.Semantics.Spec.pure_aborts
  Move.Semantics.Spec.abort_ok
  Move.Semantics.Spec.abort_aborts
  Move.Semantics.Spec.certified_ok
  Move.Semantics.Spec.certified_aborts
  Move.Semantics.Spec.ok_ite
  Move.Semantics.Spec.aborts_ite
  Move.Semantics.Spec.undefined_ite
  Move.Semantics.Spec.ok_dite
  Move.Semantics.Spec.aborts_dite
  Move.Semantics.Spec.undefined_dite
  Move.Semantics.Spec.pure_undefined
  Move.Semantics.Spec.abort_undefined
  Move.Semantics.Mutation.read
  Move.Semantics.Mutation.write
  Move.Vector.empty
  Move.Vector.push
  Move.Vector.set
  Move.Vector.length
  Move.Vector.ofList
  Move.Vector.toList
  Move.Semantics.Checked.addSpec
  Move.Semantics.Checked.subSpec
  Move.Semantics.Checked.mulSpec
  Move.Semantics.Checked.divSpec
  Move.Semantics.Checked.modSpec
  Move.Semantics.Checked.shlSpec
  Move.Semantics.Checked.shrSpec
  Move.Semantics.Checked.castSpec
  Move.U64.size

namespace Move.Verify

open Lean Elab Tactic

/-- Normalize value-level source semantics using the shared inventory. Side
conditions are discharged only by arithmetic, decidability, or an existing
hypothesis, keeping normalization predictable. -/
syntax "spec_norm" (Lean.Parser.Tactic.location)? : tactic

macro_rules
  | `(tactic| spec_norm $[$location]?) => do
      let core ← `(tactic| simp
        (disch := first
          | omega
          | (simp only [move_norm, Nat.reducePow, Nat.reduceMod]; omega)
          | (simp only [move_norm, Move.U64.size, Nat.reducePow,
              Nat.reduceMod]; omega)
          | decide
          | assumption
          | (simp (disch := omega) only [move_norm, Nat.mod_eq_of_lt,
              Nat.reducePow, Nat.reduceMod]
             assumption))
        only [move_norm, Nat.mod_eq_of_lt, Nat.reducePow, Nat.reduceMod]
        $[$location]?)
      `(tactic| (uint_bounds; $core))

/-- Normalize a WP goal using the primitive proof rules. -/
syntax "wp_norm" (Lean.Parser.Tactic.location)? : tactic
macro_rules
  | `(tactic| wp_norm $[$location]?) =>
      `(tactic| simp only [wp_norm] $[$location]?)

/-- Split a source condition, rewrite the corresponding conditional, and
normalize the named hypothesis in both branches. In particular,
`logicalLT_uint` and `logicalLE_uint` turn numeric Move conditions into
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
            Move.Verify.Source.logicalLT_uint] <;>
          simp only [Move.Verify.Source.logicalLT_uint,
            Move.UInt.toNat_ofNat, Move.UInt.toNat_ofNat_numeral,
            move_norm, Nat.reducePow, Nat.reduceMod, Nat.zero_mod]
            at $hypothesisLocation <;>
          try have $zeroFact : $value = 0 :=
            Move.UInt.eq_zero_of_not_pos $hypothesis <;>
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
            Move.Verify.Source.logicalLT_uint,
            Move.Verify.Source.logicalLE_uint] <;>
          simp only [Move.Verify.Source.logicalLT_uint,
            Move.Verify.Source.logicalLE_uint,
            Move.Verify.Source.logicalLT_move,
            Move.Verify.Source.logicalBEq_move,
            Move.UInt.toNat_ofNat, Move.UInt.toNat_ofNat_numeral,
            move_norm, Nat.reducePow, Nat.reduceMod, Nat.zero_mod]
            at $hypothesisLocation)

/-- Finish a numeric source-value goal directly, or first reduce equality of
`U64` values to equality of their exposed natural values. -/
syntax "u64_omega" : tactic
macro_rules
  | `(tactic| u64_omega) =>
      `(tactic| first
          | rfl
          | (uint_bounds; omega)
          | (spec_norm <;> omega)
          | (apply Move.UInt.ext <;> spec_norm <;> omega)
          | (apply Move.UInt.ext <;> uint_bounds <;> omega))

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

/-- Normalize the semantic `¬ mayAbort → ...` guard on the postcondition into
one negated hypothesis per declared abort condition — and none at all when no
abort condition is declared or it is `False`. `contract_intro` applies this
automatically; use it directly after a manual
`satisfies_of_wp`/`satisfies_fix_of_wp`. -/
syntax "abort_norm" : tactic
macro_rules
  | `(tactic| abort_norm) =>
    `(tactic|
      try simp only [false_and, and_false, exists_false, exists_const,
        not_false_eq_true, true_implies, not_true_eq_false, false_implies,
        implies_true, exists_and_left, exists_eq, exists_eq', and_true,
        not_or, exists_or, and_imp])

/-- Discharge the arithmetic side of an abort obligation, or refute a branch
that no declared clause admits. -/
syntax "abort_arith" : tactic
macro_rules
  | `(tactic| abort_arith) =>
    `(tactic| first
        | assumption
        | omega
        | (spec_norm; omega)
        | (uint_bounds; simp only [move_norm, Nat.reducePow, Nat.reduceMod] at *
           omega)
        | (simp only [move_norm, Nat.reducePow, Nat.reduceMod] at *; omega)
        | (simp_all [move_norm, Nat.reducePow, Nat.reduceMod]; omega))

/-- Prove the abort code of the clause under consideration. Selecting the
matching clause is what makes the surrounding search deterministic. -/
syntax "abort_code" : tactic
macro_rules
  | `(tactic| abort_code) =>
    `(tactic| first
        | rfl
        | trivial
        | (simp only [move_norm, Nat.reducePow, Nat.reduceMod]; done)
        | (simp [move_norm, Nat.reducePow, Nat.reduceMod]; done))

/-- Close an abort obligation against the contract's declared abort clauses.
The observed abort code selects the clause, and its condition is discharged
by arithmetic; a condition needing a semantic argument is left as the only
remaining goal. A branch that no clause admits is refuted instead. -/
syntax "abort_clause" : tactic
macro_rules
  | `(tactic| abort_clause) =>
    `(tactic| first
        | done
        | (refine Or.inl ?_; abort_clause)
        | (refine Or.inr ?_; abort_clause)
        | (refine ⟨?_, ?_⟩
           rotate_left
           abort_code
           try abort_arith)
        | trivial
        | abort_arith)

/-- Split the leading checked operation of a weakest-precondition goal into
its success and abort branches, naming the branch hypothesis, and discharge
the abort branch against the contract's declared abort clauses. Every checked
operation — arithmetic, casts, shifts, element borrows, and vector insert or
remove through a mutable borrow — has the same two-branch weakest
precondition, so one tactic covers them all. A remaining abort obligation is
left as the last goal. -/
syntax (name := checkedCases) "checked_cases " ident : tactic

@[tactic checkedCases]
private def elabCheckedCases : Tactic := fun stx => withMainContext do
  let branch : TSyntax `ident := ⟨stx[1]⟩
  evalTactic (← `(tactic| try wp_norm))
  evalTactic (← `(tactic| refine ⟨fun $branch => ?_, fun $branch => ?_⟩))
  match ← getGoals with
  | success :: abortObligation :: rest =>
      setGoals [abortObligation]
      evalTactic (← `(tactic| try abort_clause))
      setGoals (success :: (← getGoals) ++ rest)
  | _ => pure ()

/-- Open the generated contract at the current goal and switch to weakest-
precondition reasoning. The source function is recovered from a goal of the
form `f.contract`. Nonrecursive functions use `satisfies_of_wp`; recursive
functions unfold `f.sourceSpec`, use `satisfies_fix_of_wp`, and expose
`recursive` and `recursiveVerified`. In both cases the authored source body is
unfolded and the remaining binders are named `args`, `initial`, and
`permitted`. -/
syntax (name := contractIntro) "contract_intro" : tactic

private def normalizeMayAbort : TacticM Unit := do
  evalTactic (← `(tactic| abort_norm))

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
      normalizeMayAbort
    else
      evalTactic (← `(tactic| apply Move.Verify.satisfies_of_wp))
      introNamed `args
      introNamed `initial
      introNamed `permitted
      normalizeMayAbort

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
