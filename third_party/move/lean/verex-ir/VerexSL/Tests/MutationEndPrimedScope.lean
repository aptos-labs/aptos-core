-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# Primed-name scope at mutation-end `data invariant` leaves

Checks that at a mutation-end `data invariant` leaf, the checked owner's primed
name `x'` denotes the value checked at that mutation end (`CheckedEntry`), while
`old(x)` denotes the entry value, a moved reference holder (`let r := x`)
preserves the `x'` association by prophecy identity, unrelated primed names
`other'` keep their exit prophecies (`FinalEntry`), `ensures` leaves keep their
exit prophecies, and a `checkingAt` step whose continuation does not start with
`Spec.assume (checked = prophecy)` asserts no `CheckedEntry` and keeps today's
`FinalEntry` binding.
-/

namespace VerexSL.Tests.MutationEndPrimedScope

open Lean Meta Elab Tactic
open VerexIR.Proofs.Denote

set_option Elab.async false

verex module 0x42::mutation_end_primed_scope where
  struct Counter has Copy, Drop where
    val : u64
    shadow : u64
  spec Counter where
    invariant val == shadow

  struct Boxed has Copy, Drop where
    val : u64
  spec Boxed where
    invariant val > 0

  -- Controls 1, 2, 3, and 5:
  -- - Control 1: `s'` at `data invariant` denotes the checked value `(old(s).val + 1, old(s).shadow + 1)`.
  -- - Control 2: unrelated `&mut` parameter `other'` at `s`'s `data invariant` leaf
  --   retains its `FinalEntry` exit prophecy.
  -- - Control 3: `old(s)` at `data invariant` denotes the entry value of `s`.
  -- - Control 5: `s'` and `other'` at `ensures` denote the settled final values.
  fun bump_with_other(other : &mut u64, s : &mut Counter) -> Unit := do
    *other := 7
    let v := (*s).val
    let sh := (*s).shadow
    (*s).val := v + 1
    (*s).shadow := sh + 1
  spec bump_with_other where
    requires s.val < 100
    aborts_if false
    ensures s.val == old(s).val + 1
    ensures other == 7

  verify bump_with_other by
    on data invariant {
      assert s'.val == old(s).val + 1
      assert s'.shadow == old(s).shadow + 1
      assert other' == 7
    }
    on ensures 1 {
      assert s'.val == old(s).val + 1
    }
    on ensures 2 {
      assert other' == 7
    }
    on _ { }

  -- Control 4: moving the `&mut` parameter into a local (`let r := s`) still
  -- associates `s'` at `data invariant` with the checked value via `s`'s
  -- prophecy identity, including on a 1-field struct (`Boxed`) where
  -- `Prod.mk.injEq` simplifies `(v, ()) = (v_final, ())` to `v = v_final`.
  fun bump_moved(s : &mut Boxed) -> Unit := do
    let r := s
    let v := (*r).val
    (*r).val := v + 1
  spec bump_moved where
    requires s.val < 100
    aborts_if false
    ensures s.val == old(s).val + 1

  verify bump_moved by
    on data invariant {
      assert s'.val == old(s).val + 1
    }
    on _ { }

-- Control 6: fallback when a `checkingAt` continuation does not start with
-- `Spec.assume (checked = prophecy)` — `settledProphecyOf?` returns `none`,
-- no `CheckedEntry` is asserted, and `bindFinals goal true` keeps the existing
-- `FinalEntry` binding untouched.
set_option linter.unusedVariables false in
private theorem fallback_without_leading_assume_keeps_final_entry
    (prophecy checked : Int)
    (_final : FinalEntry "s" prophecy) :
    True := by
  run_tac withMainContext do
    let state ← targetState
    state.scopeNames.set (some (["s", "s'"], ["s"]))
    state.scopeLocals.set #["s"]
    let prophecyFVar := (← getLocalDeclFromUserName `prophecy).toExpr
    let checkedFVar := (← getLocalDeclFromUserName `checked).toExpr
    let pureCont := mkAppN (mkConst ``VerexIR.Proofs.Spec.pure) #[mkConst ``Unit, mkConst ``Unit, mkConst ``Unit, mkConst ``Unit.unit]
    if (← settledProphecyOf? checkedFVar pureCont).isSome then
      throwError "expected `settledProphecyOf?` to return `none` when continuation is `Spec.pure`"
    -- Without a `CheckedEntry` asserted from a leading `Spec.assume (checked = prophecy)`,
    -- `bindFinals goal true` must keep `FinalEntry`'s `prophecy` for `"s'"`.
    let goal ← bindFinals (← getMainGoal) true
    replaceMainGoal [goal]
    withMainContext do
      let mut found : Option Lean.Expr := none
      for decl in ← getLCtx do
        if let some (name, _, val) := scopeEntry? (← instantiateMVars decl.type) then
          if name == "s'" then found := some val
      unless found == some prophecyFVar && found != some checkedFVar do
        throwError m!"fallback without CheckedEntry bound `s'` to `{found}` instead of `{prophecyFVar}`"
  trivial

-- Control 7: when `CheckedEntry` overrides `"s'"` to `sChecked`, an unrelated
-- `&mut` owner `"other'"` preserves its distinct `FinalEntry` value `otherProphecy`
-- (neither `sChecked` nor `sProphecy`).
set_option linter.unusedVariables false in
private theorem unrelated_final_entry_preserved_alongside_checked_entry
    (sProphecy sChecked otherProphecy : Int)
    (_finalS : FinalEntry "s" sProphecy)
    (_finalOther : FinalEntry "other" otherProphecy)
    (_checkedS : CheckedEntry sProphecy sChecked) :
    True := by
  run_tac withMainContext do
    let state ← targetState
    state.scopeNames.set (some (["s", "s'", "other", "other'"], ["s", "other"]))
    state.scopeLocals.set #["s", "other"]
    let sProphecyFVar := (← getLocalDeclFromUserName `sProphecy).toExpr
    let sCheckedFVar := (← getLocalDeclFromUserName `sChecked).toExpr
    let otherProphecyFVar := (← getLocalDeclFromUserName `otherProphecy).toExpr
    let goal ← bindFinals (← getMainGoal) true
    replaceMainGoal [goal]
    withMainContext do
      let mut foundS : Option Lean.Expr := none
      let mut foundOther : Option Lean.Expr := none
      for decl in ← getLCtx do
        if let some (name, _, val) := scopeEntry? (← instantiateMVars decl.type) then
          if name == "s'" then foundS := some val
          if name == "other'" then foundOther := some val
      unless foundS == some sCheckedFVar && foundS != some sProphecyFVar do
        throwError m!"expected `s'` bound to `{sCheckedFVar}`, got `{foundS}`"
      unless foundOther == some otherProphecyFVar &&
          foundOther != some sCheckedFVar && foundOther != some sProphecyFVar do
        throwError m!"expected unrelated `other'` bound to `{otherProphecyFVar}`, got `{foundOther}`"
  trivial

end VerexSL.Tests.MutationEndPrimedScope
