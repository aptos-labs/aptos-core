-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Base

open Lean Meta Elab Tactic VerexIR.Proofs.Denote

namespace SplitOptionReadings

/-- True when `e` contains a full-arity `withSome` whose option argument is a
syntactic `Option.some` or `Option.none` application. -/
private def hasConstructorWithSome (e : Lean.Expr) : Bool :=
  (e.find? fun sub =>
    sub.isAppOfArity ``withSome 5 &&
      let opt := sub.getAppArgs[2]!
      opt.isAppOf ``Option.some || opt.isAppOf ``Option.none).isSome

private def assertNoConstructorWithSome : TacticM Unit := withMainContext do
  for decl in ← getLCtx do
    unless decl.isImplementationDetail do
      let ty ← instantiateMVars decl.type
      if hasConstructorWithSome ty then
        throwError "constructor withSome remained in hypothesis `{decl.userName}`: {ty}"

/-- A branch that consumes the equation its reading states. -/
private def readBack (option : Option Nat) : (a : Nat) → option = some a → Option Nat :=
  fun _ reads => some (option.get (by simp [reads]))

-- 1. `readingsAtValue?` reduces `withSome` at `some` (`withSome_some`) and strips `some`
-- (`Option.some.injEq`, which is in `readers`) without invoking `Prod.mk.injEq`: when the
-- right-hand side is not a pair constructor, the rewritten hypothesis is the pair equality
-- `(w, 1) = pairOut ()`, not a conjunction.
private theorem decided_reading_reduces_without_tuple_injection [Carriers]
    (option : Option Nat) (pairOut : Unit → Nat × Nat) (nonEmpty : ¬ option = none)
    (h : withSome option (fun a _ => some (a, 1)) none = some (pairOut ())) :
    ∃ w, option = some w ∧ (w, 1) = pairOut () := by
  verex_split_hypotheses
  run_tac withMainContext do
    assertNoConstructorWithSome
    let hTy ← instantiateMVars (← getLocalDeclFromUserName `h).type
    let some (_, lhs, _) := hTy.eq? | throwError "expected pair equality in `h`, got: {hTy}"
    unless lhs.isAppOfArity ``Prod.mk 4 do
      throwError "expected `Prod.mk` on lhs of `h` before tuple injection, got: {lhs}"
  exact ⟨_, rfl, h⟩

-- 2. When both sides of the reduced equation are `Prod.mk`, `splitHypotheses`'s separate
-- constructor-equation injection (`Lean.Meta.injection`) decomposes `(w, 1) = (x, y)` into
-- component equations that `verex_subst_vars` consumes.
private theorem decided_tuple_reading_injects_components [Carriers]
    (option : Option Nat) (x y : Nat) (nonEmpty : ¬ option = none)
    (h : withSome option (fun a _ => some (a, 1)) none = some (x, y)) :
    y = 1 := by
  verex_split_hypotheses
  run_tac assertNoConstructorWithSome
  verex_subst_vars

-- 3. Proof-dependent branch: `found` consumes `reads : option = some a`, transported by
-- `withSome_of_some` and reduced at `rfl` by `withSome_some`.
private theorem decided_dependent_reading_reduces [Carriers]
    (option : Option Nat) (x : Nat) (nonEmpty : ¬ option = none)
    (h : withSome option (readBack option) none = some x) :
    ∃ w, option = some w ∧ (some w : Option Nat).get (by simp) = x := by
  verex_split_hypotheses
  run_tac assertNoConstructorWithSome
  refine ⟨_, rfl, ?_⟩
  simpa [readBack] using h

-- 4. Empty branch and `withSome_none`: the empty case (`option = none`) is refuted through
-- `h` alone with no separate `¬ option = none` hypothesis, and a nested reading whose option
-- simplifies to `none` under the split is reduced by `withSome_none` in `readers`.
private theorem empty_branch_and_none_reading_reduce [Carriers]
    (option : Option Nat) (x y : Nat)
    (h : withSome option (fun a _ => some (a + 1)) none = some x)
    (h_none : withSome (withSome option (fun _ _ => (none : Option Nat)) (some 0))
      (fun b _ => b + 1) 7 = y) :
    ∃ w, option = some w ∧ w + 1 = x ∧ y = 7 := by
  verex_split_hypotheses
  run_tac assertNoConstructorWithSome
  verex_subst_vars
  exact ⟨_, rfl, rfl, rfl⟩

-- 5. Negative: an undecided symbolic option whose empty case is consistent with the context
-- is not resolved by `readingsAtValue?`.
private theorem undecided_symbolic_option_unchanged [Carriers]
    (option : Option Nat) (x : Nat)
    (h : withSome option (fun a _ => a + 1) 0 = x) :
    withSome option (fun a _ => a + 1) 0 = x := by
  verex_split_hypotheses
  run_tac withMainContext do
    let hTy ← instantiateMVars (← getLocalDeclFromUserName `h).type
    let some (_, lhs, _) := hTy.eq? | throwError "expected equality in `h`, got: {hTy}"
    unless lhs.isAppOfArity ``withSome 5 && lhs.getAppArgs[2]!.isFVar do
      throwError "undecided symbolic option reading was spuriously rewritten: {hTy}"
  exact h

-- 6. Negative: splitting a decided option preserves unrelated `Slot.Holds` hypotheses and
-- does not synthesize missing `NTy.Holds` validity for the extracted value.
private theorem missing_validity_still_owed [Carriers]
    (option otherSlot : Option (NTy.param 0).carrier) (x : (NTy.param 0).carrier)
    (nonEmpty : ¬ option = none)
    (_h : withSome option (fun a _ => some a) none = some x)
    (slotValid : Slot.Holds (NTy.param 0) otherSlot) :
    Slot.Holds (NTy.param 0) otherSlot := by
  verex_split_hypotheses
  run_tac assertNoConstructorWithSome
  fail_if_success
    have : (NTy.param 0).Holds x := by
      verex_assumption
  exact slotValid

open Lean Elab Command in
run_cmd do
  for name in [``decided_reading_reduces_without_tuple_injection,
      ``decided_tuple_reading_injects_components,
      ``decided_dependent_reading_reduces,
      ``empty_branch_and_none_reading_reduce,
      ``undecided_symbolic_option_unchanged,
      ``missing_validity_still_owed] do
    let axioms ← collectAxioms name
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {name}: {axioms}"

end SplitOptionReadings
