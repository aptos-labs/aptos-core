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
- `move_data` unfolds the data level shared by both proof styles —
  references, stores, vectors, and the monad shells;
- `move_spec` unfolds the raw relational semantics through projection
  lemmas; together with `move_data` it is the brute-force inventory behind
  `simp [move_spec, move_data]`, and user proofs or project-specific
  summaries can extend it with `@[move_spec]`.
-/

/-! ## Numerals and the `ofNat` constructor

`Move.UInt.ofNat` is what the unsigned view lemmas produce; a numeral is what a
specification is written with.  The two spellings must meet, or every concrete
postcondition is left one defeq step short — but neither equation can be a simp
lemma.  Read numeral-to-`ofNat`, the key *must* be `no_index (OfNat.ofNat n)`
— a discrimination-tree wildcard this repo has already paid for once — because
the tree collapses numerals to literal keys, so a precisely keyed pattern never
matches at all.  Read `ofNat`-to-numeral, a rewrite rule would also strip the
`ofNat` shape off the compound `ofNat (a.toNat + b.toNat)` results that the
collapse lemmas and the `grind` patterns are keyed on.

Restricting to *literal* arguments is exactly the distinction a rewrite rule
cannot make and a simproc can, and it settles the canonical form: once the
argument is a literal all the arithmetic is done, so the value is the numeral.
The simproc below carries that choice through, keyed on the `UInt.ofNat` head
constant rather than on a numeral. -/

/-- `ofNat` of a literal *is* that literal. -/
simproc [simp, move_norm, wp_norm] uintOfNatLit (@Move.UInt.ofNat _ _ _) := fun e => do
  let_expr Move.UInt.ofNat w inst n := e | return .continue
  let some k := n.nat? | return .continue
  return .done
    { expr := ← Lean.Meta.mkAppOptM ``OfNat.ofNat
        #[← Lean.Meta.inferType e, Lean.mkRawNatLit k, none]
      proof? := Lean.mkApp3 (Lean.mkConst ``Move.UInt.ofNat_eq_numeral) w inst n }

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
  -- The width-directed literal constructors are named specification surface
  -- for the one `UInt.ofNat` / `SInt.ofInt`; unfolding them keeps a single
  -- discrimination-tree key, so which spelling a spec used cannot change
  -- whether a view lemma fires.
  Move.UInt.toNat_ofNat_numeral
  Move.UInt.toNat_zero
  Move.UInt.toNat_one
  Move.U8.ofNat
  Move.U16.ofNat
  Move.U32.ofNat
  Move.U64.ofNat
  Move.U128.ofNat
  Move.U256.ofNat
  Move.I8.ofInt
  Move.I16.ofInt
  Move.I32.ofInt
  Move.I64.ofInt
  Move.I128.ofInt
  Move.I256.ofInt
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
  Move.UInt.toNat_ofNat
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
  Move.SInt.add_def
  Move.SInt.sub_def
  Move.SInt.mul_def
  Move.SInt.div_def
  Move.SInt.mod_def
  Move.SInt.neg_def
  Move.SInt.cast_def
  Move.UInt.land_def
  Move.UInt.land_eq_ofNat
  Move.UInt.lor_eq_ofNat
  Move.UInt.lxor_eq_ofNat
  Move.UInt.shl_eq_ofNat
  Move.UInt.shr_eq_ofNat
  Move.UInt.lor_def
  Move.UInt.lxor_def
  Move.UInt.shl_def
  Move.UInt.shr_def
  Move.UInt.and_lt_size
  Move.UInt.or_lt_size
  Move.UInt.xor_lt_size
  Move.UInt.shiftRight_lt_size
  Move.Semantics.Checked.inRange_cast
  Move.Semantics.Checked.toInt_eq_zero_iff
  Move.Semantics.Checked.toInt_ne_zero_iff
  Move.Semantics.Checked.shlSpec_eq_pure
  Move.Semantics.Checked.shrSpec_eq_pure
  Move.Semantics.Vector.borrowElemSpec_eq_pure
  Move.Verify.withBorrowElemMutSpec_write_eq_pure

-- The operations whose result cannot leave the width's range collapse without
-- the `% size` that `toNat_ofNat` would introduce.  They must outrank it
-- *within this set*: a `@[simp high]` attribute sets the priority of the `simp`
-- set only, so registering them here at default priority let the general lemma
-- win and left every bitwise proof with a `% size` residue to discharge by
-- arithmetic that does not model bitwise operations.
attribute [move_norm high]
  Move.UInt.add_def
  Move.UInt.sub_def
  Move.UInt.mul_def
  Move.UInt.div_def
  Move.UInt.mod_def
  Move.UInt.cast_def
  Move.UInt.add_eq_ofNat
  Move.UInt.mul_eq_ofNat
  Move.UInt.div_eq_ofNat
  Move.UInt.mod_eq_ofNat
  Move.UInt.ofInt_add
  Move.UInt.ofInt_sub
  Move.UInt.ofInt_mul
  Move.UInt.ofInt_tdiv
  Move.UInt.ofInt_tmod
  Move.Semantics.Checked.inRange_add
  Move.Semantics.Checked.inRange_sub
  Move.Semantics.Checked.inRange_mul
  Move.Semantics.Checked.inRange_tdiv
  Move.Semantics.Checked.addSpec_eq_pure_unsigned
  Move.Semantics.Checked.subSpec_eq_pure_unsigned
  Move.Semantics.Checked.mulSpec_eq_pure_unsigned
  Move.Semantics.Checked.divSpec_eq_pure_unsigned
  Move.Semantics.Checked.modSpec_eq_pure_unsigned
  Move.Semantics.Checked.castSpec_eq_pure_unsigned
  Move.UInt.toNat_ofNat_sub
  Move.UInt.toNat_ofNat_div
  Move.UInt.toNat_ofNat_mod
  Move.UInt.toNat_ofNat_land
  Move.UInt.toNat_ofNat_lor
  Move.UInt.toNat_ofNat_lxor
  Move.UInt.toNat_ofNat_shr

attribute [move_data]
  Id.run
  Bind.bind
  Pure.pure
  Move.Semantics.ResourceStore.contains
  Move.Semantics.ResourceStore.get
  Move.Semantics.ResourceStore.descriptor
  Move.Semantics.Mutation.read
  Move.Semantics.Mutation.write
  Move.Vector.empty
  Move.Vector.push
  Move.Vector.set
  Move.Vector.length
  Move.Vector.ofList
  Move.Vector.toList
  Move.U64.size

-- Global-invariant re-establishment is discharged by the generated
-- `@[grind]` reestablishment lemmas, not by unfolding the quantifier here.

attribute [move_spec]
  Id.run
  Bind.bind
  Pure.pure
  Move.Semantics.ResourceStore.contains
  Move.Semantics.ResourceStore.get
  Move.Semantics.ResourceStore.descriptor
  Move.Semantics.Mutation.read
  Move.Semantics.Mutation.write
  Move.Vector.empty
  Move.Vector.push
  Move.Vector.set
  Move.Vector.length
  Move.Vector.ofList
  Move.Vector.toList
  Move.U64.size
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
  Move.Semantics.Checked.addSpec
  Move.Semantics.Checked.subSpec
  Move.Semantics.Checked.mulSpec
  Move.Semantics.Checked.divSpec
  Move.Semantics.Checked.modSpec
  Move.Semantics.Checked.shlSpec
  Move.Semantics.Checked.shrSpec
  Move.Semantics.Checked.castSpec

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

/-- Discharge the arithmetic side of an abort obligation, or refute a branch
that no declared clause admits. -/
syntax "abort_arith" : tactic
macro_rules
  | `(tactic| abort_arith) =>
    `(tactic| first
        | assumption
        | omega
        | (spec_norm; omega)
        | (uint_bounds; simp only [move_norm, Nat.reducePow, Nat.reduceMod,
             List.getElem?_eq_none_iff] at *
           omega)
        | (simp only [move_norm, Nat.reducePow, Nat.reduceMod,
            List.getElem?_eq_none_iff] at *; omega)
        | (simp_all [move_norm, Nat.reducePow, Nat.reduceMod,
            List.getElem?_eq_none_iff]; omega))

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

/-- Normalize a branch hypothesis produced by splitting a source conditional:
the sealed comparison markers become facts about integer values, and numerals
reduce. -/
syntax "move_hyp " ident : tactic
macro_rules
  | `(tactic| move_hyp $h:ident) =>
      `(tactic| try simp only [Move.Verify.Source.logicalLT_uint,
          Move.Verify.Source.logicalLE_uint, Move.Verify.Source.logicalLT_move,
          Move.Verify.Source.logicalBEq_move, Move.UInt.lt_iff_toNat_lt,
          Move.UInt.toNat_ofNat, Move.UInt.toNat_ofNat_numeral,
          Move.UInt.toNat_zero, Move.UInt.toNat_one, Move.Vector.length_toNat,
          move_norm, Nat.reducePow, Nat.reduceMod, Nat.zero_mod,
          Bool.not_eq_true, not_lt, not_le] at $h:ident)

/-- One symbolic step of a manual proof, chosen by the shape of the goal:

- a source conditional at the head is split, its branch hypothesis named by
  the first identifier given (default `h`) and normalized with `move_hyp`;
- a checked operation is split into its success branch (its binders named
  by the identifiers given) and its abort branch, which is discharged
  against the declared abort clauses and otherwise left as the last goal;
- a recursive call is stepped through with the contract being established,
  leaving its precondition and the postcondition weakening;
- otherwise the pure plumbing a weakest-precondition rule leaves behind —
  `¬mayAbort` guards, a trivial frame, reconciliation equations — is cleared.

Each step rewrites only the leading construct, so the proof keeps the shape
of the source. -/
syntax (name := moveStep) "move_step" (ppSpace colGt ident)* : tactic

@[tactic moveStep]
private def elabMoveStep : Tactic := fun stx => withMainContext do
  let names : Array (TSyntax `ident) := stx[1].getArgs.map (⟨·⟩)
  let firstName : TSyntax `ident := names.getD 0 (mkIdent `h)
  -- Expose the leading construct.
  evalTactic (← `(tactic| try simp only [wp_norm, Move.Semantics.Spec.pure_bind,
    Move.Semantics.Spec.bind_pure, Move.Semantics.Spec.abort_bind,
    Move.Semantics.Spec.fix_const, Move.Semantics.Mutation.read,
    Move.Semantics.Mutation.write, Move.Semantics.Mutation.current_write,
    Move.Semantics.Mutation.read_mk, Move.Semantics.Mutation.read_write,
    forall_eq, forall_eq']))
  let target := (← instantiateMVars (← getMainTarget)).consumeMData
  let head := target.getAppFn
  if head.isConstOf ``ite || head.isConstOf ``dite then
    -- A source conditional: split, name, normalize.
    evalTactic (← `(tactic| split <;> rename_i $firstName:ident <;>
      move_hyp $firstName:ident))
  else if head.isConstOf ``And &&
      !(target.appArg!.isConstOf ``True || target.appArg!.isAppOf ``Eq) then
    -- A two-branch rule: success (named binders) and abort (discharged).
    evalTactic (← `(tactic| refine ⟨?_, ?_⟩))
    match ← getGoals with
    | success :: abortObligation :: rest =>
        setGoals [abortObligation]
        evalTactic (← `(tactic| try abort_clause))
        let remaining ← getGoals
        setGoals [success]
        evalTactic (← `(tactic| intros))
        for name in names.reverse do
          evalTactic (← `(tactic| try rename_i $name:ident))
        let successGoals ← getGoals
        setGoals (successGoals ++ remaining ++ rest)
    | _ => pure ()
  else if head.isConstOf ``Move.Verify.wp then
    -- A recursive call: use the contract being established.
    let action := target.getArg! 2
    if action.getAppFn.isFVar then
      evalTactic (← `(tactic| refine Move.Verify.wp_mono
        (Move.Verify.wp_of_satisfies recursiveVerified ?_) ?_ ?_))
    else
      throwError "move_step: no rule for the leading operation{indentExpr action}"
  else
    -- Plumbing left by a rule: guards, frames, reconciliation.
    evalTactic (← `(tactic| first
      | simp only [false_and, and_false, exists_false, exists_const,
          not_false_eq_true, true_implies, not_true_eq_false, false_implies,
          implies_true, exists_and_left, exists_eq, exists_eq', and_true,
          true_and, not_or, exists_or, and_imp, eq_self_iff_true, and_self,
          Classical.not_not]
      | rfl
      | trivial
      | fail "move_step: no step applies to this goal"))

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
