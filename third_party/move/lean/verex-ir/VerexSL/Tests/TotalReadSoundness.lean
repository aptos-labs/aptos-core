-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# Total-read off-domain soundness controls (`doc/plan/plain-values-semantics.md` §2)

Every total specification read is tested across the 7 rows of §2:
1. `nativeSlot` (absent global resource `T[a]`)
2. `typedVariantField?` (field selection from an inactive variant)
3. `SpecMap.get` / `valueAt` / `keyAt` (missing map key or out-of-bounds position)
4. `indexVector` (out-of-bounds vector index `v[i]`)
5. `ResultOf` (`result_of<f>(x)` when no run returns)
6. `nativeArbitrary` / `outsideValue` (aborting spec path or out-of-domain recursive spec fun)
7. `unspecifiedGhost` (ghost slot left unspecified by spec construction)

For each row:
- **Positive guarded controls** (`exists<Item>(a)`, `c is Present`, `has(t, k)` / `i < size(t)`,
  `0 <= i && i < v.length`, `!aborts_of<f>(x)`, caller domain `requires`, explicit ghost
  initialization / `iter_new`) **verify cleanly**.
- **Prepared `plainFallback` uninterpretedness controls** verify directly that `verex_decide`
  cannot equate `plainFallback` / `NTy.plainFallbackOf` to `0`, `false`, or any `Holds` property.
- **Rows 6–7 (`nativeArbitrary` / `outsideValue` and `unspecifiedGhost`)** negative controls
  are **already rejected in Phase 1**.
- **Rows 1–5 (`nativeSlot`, `typedVariantField?`, `SpecMap.get`/`keyAt`, `indexVector`,
  `ResultOf`)** off-domain structural-zero and `Holds` negative controls are defined and staged
  below to flip to `assertRejected` in Phase 2d when all off-domain total specification reads
  switch uniformly to `plainFallback`.
-/

namespace VerexSL.Tests.TotalReadSoundness

open Lean Elab Command VerexSL.Verify

private def forbiddenSubstrings : List String := [
  "exceeded its budget", "exceeded its heartbeat budget", "maxHeartbeats",
  "maximum number of heartbeats", "heartbeats", "stopped by the budget",
  "deterministic timeout", "(deterministic) timeout",
  "maximum recursion depth", "deep recursion",
  "unknown identifier", "unknown constant", "unknown function",
  "no Verex module declares", "no denotation for",
  "given verification is unsupported", "given verification concrete transfer failed",
  "pragma verify = manual",
  "type mismatch", "application type mismatch", "failed to synthesize",
  "unsolved goals", "no goals to be solved",
  "kernel", "PANIC at", "internal exception"
]

private def assertRejected (base : Name) (command : TSyntax `command) (label : String) :
    CommandElabM Unit := do
  let saved ← get
  let priorCount := saved.messages.toList.length
  let command := Id.run <| command.raw.replaceM fun stx =>
    pure <| if stx.isIdent then some (mkIdentFrom stx stx.getId.eraseMacroScopes) else none
  let thrown ← try
    elabCommand command
    pure none
  catch error =>
    pure (some (error.isMaxHeartbeat || error.isMaxRecDepth, ← error.toMessageData.toString))
  let attempted ← get
  let curNs ← getCurrNamespace
  let fresh := attempted.messages.toList.drop priorCount
  let freshErrors := fresh.filter (·.severity == .error)
  let hasHeartbeatTag := fresh.any (·.data.hasTag (· == `runtime.maxHeartbeats))
  let errorTexts := (← freshErrors.mapM fun message => message.data.toString) ++
    (thrown.map (·.2)).toList
  let noProof := !attempted.env.contains (base ++ `verified) &&
    !attempted.env.contains (curNs ++ base ++ `verified) &&
    !attempted.env.contains (base ++ `givenTypedVerified) &&
    !attempted.env.contains (curNs ++ base ++ `givenTypedVerified) &&
    !attempted.env.contains (base ++ `typedVerified) &&
    !attempted.env.contains (curNs ++ base ++ `typedVerified) &&
    !attempted.env.contains (base ++ `givenTransfer) &&
    !attempted.env.contains (curNs ++ base ++ `givenTransfer) &&
    ((givenProofArtifacts.getState attempted.env).find? base).isNone &&
    ((givenProofArtifacts.getState attempted.env).find? (curNs ++ base)).isNone
  modify fun _ => saved
  unless noProof do
    throwError "expected `{label}` ({base}) not to publish any proof artifact"
  if thrown.any (·.1) || hasHeartbeatTag then
    throwError "expected semantic rejection for `{label}` ({base}), got heartbeat/recursion limit: {errorTexts}"
  if let some bad := errorTexts.find? (fun text => forbiddenSubstrings.any (text.contains ·)) then
    throwError "expected semantic rejection for `{label}` ({base}), got budget/infrastructure diagnostic: {bad}"
  let fn := base.getString!
  let summaryPrefix := s!"verex verification failed: the automatic verification of `{fn}` failed;"
  let isSemanticClause (text : String) : Bool :=
    (text.startsWith "the specification clause " && text.endsWith " is not established") ||
    (text.startsWith "the data invariant " && text.contains " does not hold ")
  let hasSummary := errorTexts.any (·.startsWith summaryPrefix)
  let hasClause := errorTexts.any isSemanticClause
  let allExpected := errorTexts.all fun text =>
    text.startsWith summaryPrefix || isSemanticClause text
  unless !errorTexts.isEmpty && hasSummary && hasClause && allExpected do
    throwError "expected `{label}` ({base}) to fail with a semantic clause diagnostic, got: {errorTexts}"

/-! ## Row 1: `nativeSlot` (absent resource `T[a]`) -/

verex module 0x42::row1_slot where
  pragma verify = false

  struct Counter has Key where
    n : u64

  struct Item has Copy, Drop, Store, Key where
    count : u64
  spec Item where
    invariant count > 0

  fun pos_value(a : Address) -> Unit := ()
  spec pos_value where
    requires exists<Counter>(a) && Counter[a].n == 7
    aborts_if false
    ensures Counter[a].n == 7

  fun pos_holds(a : Address) -> u64 := (&Item[a]).count
  spec pos_holds where
    requires exists<Item>(a)
    aborts_if false
    ensures result > 0 && Item[a].count > 0

  fun neg_absent_zero(a : Address) -> Unit := ()
  spec neg_absent_zero where
    requires !exists<Counter>(a)
    aborts_if false
    ensures Counter[a].n == 0

  -- Staged for Phase 2 (`2b`/`2d`): `nativeSlot` currently falls back via
  -- `typedInhabitant` (`NTy.inhabitant` on the certified subtype `Item.carrier`),
  -- and flips to `plainFallbackValue` in Phase 2 so this off-domain `Holds`
  -- claim becomes rejected.
  fun staged_absent_holds(a : Address) -> Unit := ()
  spec staged_absent_holds where
    requires !exists<Item>(a)
    aborts_if false
    ensures Item[a].count > 0

verify 0x42::row1_slot::pos_value
verify 0x42::row1_slot::pos_holds

-- `neg_absent_zero` and `staged_absent_holds` flip to `assertRejected` in Phase 2d
-- when `nativeSlot` switches its off-domain fallback to `plainFallbackValue`.

/-! ## Row 2: `typedVariantField?` (inactive variant field selection) -/

verex module 0x42::row2_variant where
  pragma verify = false

  struct Item has Copy, Drop, Store where
    count : u64
  spec Item where
    invariant count > 0

  enum ScalarChoice has Copy, Drop where
    | Empty
    | Present (n : u64)

  enum ItemChoice has Copy, Drop where
    | Empty
    | Present (item : Item)

  struct PlainZeroWrapper has Copy, Drop where
    choice : ItemChoice
  spec PlainZeroWrapper where
    invariant !(choice is Empty) ||
      (core.data.selectVariants[ItemChoice, Present.item](choice)).count == 0

  struct PlainHoldsWrapper has Copy, Drop where
    choice : ItemChoice
  spec PlainHoldsWrapper where
    invariant !(choice is Empty) ||
      (core.data.selectVariants[ItemChoice, Present.item](choice)).count > 0

  fun pos_scalar(c : ScalarChoice) -> Unit := ()
  spec pos_scalar where
    requires c is Present && core.data.selectVariants[ScalarChoice, Present.n](c) == 7
    aborts_if false
    ensures core.data.selectVariants[ScalarChoice, Present.n](c) == 7

  fun pos_holds(c : ItemChoice) -> Unit := ()
  spec pos_holds where
    requires c is Present
    aborts_if false
    ensures (core.data.selectVariants[ItemChoice, Present.item](c)).count > 0

  fun neg_scalar_zero(c : ScalarChoice) -> Unit := ()
  spec neg_scalar_zero where
    requires c is Empty
    aborts_if false
    ensures core.data.selectVariants[ScalarChoice, Present.n](c) == 0

  fun neg_plain_zero() -> PlainZeroWrapper :=
    new PlainZeroWrapper { choice := new ItemChoice::Empty {} }
  spec neg_plain_zero where
    aborts_if false

  fun neg_plain_holds() -> PlainHoldsWrapper :=
    new PlainHoldsWrapper { choice := new ItemChoice::Empty {} }
  spec neg_plain_holds where
    aborts_if false

  -- Staged for Phase 2 (`2b`/`2d`): certified `typedVariantField?` on `ItemChoice`
  -- switches from `NTy.inhabitant` to `plainFallbackValue` in Phase 2.
  fun staged_certified_holds(c : ItemChoice) -> Unit := ()
  spec staged_certified_holds where
    requires c is Empty
    aborts_if false
    ensures (core.data.selectVariants[ItemChoice, Present.item](c)).count > 0

verify 0x42::row2_variant::pos_scalar
verify 0x42::row2_variant::pos_holds

-- `neg_scalar_zero`, `neg_plain_zero`, `neg_plain_holds`, and `staged_certified_holds`
-- flip to `assertRejected` in Phase 2d when `typedVariantField?` switches to `plainFallbackValue`.

/-! ## Row 3: `SpecMap.get` / `valueAt` / `keyAt` (missing key or OOB position) -/

verex module 0x42::row3_map where
  pragma verify = false

  struct Item has Copy, Drop, Store where
    count : u64
  spec Item where
    invariant count > 0

  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address
    length : u64

  @[map_spec_has_key (Table)]
  opaque spec fun has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_spec_get (Table)]
  opaque spec fun get {K} {V}(t : Table<K, V>, k : K) : V
  @[map_spec_len (Table)]
  opaque spec fun size {K} {V}(t : Table<K, V>) : Int
  @[map_spec_key_at (Table)]
  opaque spec fun key_at {K} {V}(t : Table<K, V>, i : Int) : K

  fun pos_value(t : &Table<u64, u64>, k : u64) -> Unit := ()
  spec pos_value where
    requires has(*t, k) && get(*t, k) == 7
    aborts_if false
    ensures get(*t, k) == 7

  fun pos_holds(t : &Table<u64, Item>, k : u64) -> Unit := ()
  spec pos_holds where
    requires has(*t, k)
    aborts_if false
    ensures (get(*t, k)).count > 0

  fun pos_key_at(t : &Table<u64, u64>, i : u64) -> Unit := ()
  spec pos_key_at where
    requires i < size(*t) && key_at(*t, i) == 9
    aborts_if false
    ensures has(*t, 9) && key_at(*t, i) == 9

  fun neg_missing_value_zero(t : &Table<u64, u64>, k : u64) -> Unit := ()
  spec neg_missing_value_zero where
    requires !has(*t, k)
    aborts_if false
    ensures get(*t, k) == 0

  fun neg_oob_key_at_zero(t : &Table<u64, u64>, i : u64) -> Unit := ()
  spec neg_oob_key_at_zero where
    requires i >= size(*t)
    aborts_if false
    ensures key_at(*t, i) == 0

  fun neg_missing_bool_false(t : &Table<u64, Bool>, k : u64) -> Unit := ()
  spec neg_missing_bool_false where
    requires !has(*t, k)
    aborts_if false
    ensures !get(*t, k)

  -- Staged for Phase 2 (`2b`/`2d`): certified `Table<u64, Item>` view switches
  -- from certified `NTy.inhabitantOf` to plain carrier in Phase 2.
  fun staged_missing_holds(t : &Table<u64, Item>, k : u64) -> Unit := ()
  spec staged_missing_holds where
    requires !has(*t, k)
    aborts_if false
    ensures (get(*t, k)).count > 0

verify 0x42::row3_map::pos_value
verify 0x42::row3_map::pos_holds
verify 0x42::row3_map::pos_key_at

-- `neg_missing_value_zero`, `neg_oob_key_at_zero`, `neg_missing_bool_false`, and
-- `staged_missing_holds` flip to `assertRejected` in Phase 2d when `SpecMap` views
-- switch their off-domain fallback to `plainFallback`.

/-! ## Row 4: `indexVector` (out-of-bounds vector read `v[i]`) -/

verex module 0x42::row4_vector where
  pragma verify = false

  struct Item has Copy, Drop, Store where
    count : u64
  spec Item where
    invariant count > 0

  struct PlainVecZero has Copy, Drop where
    items : Vector<Item>
  spec PlainVecZero where
    invariant items.length > 0 || (items[0]).count == 0

  struct PlainVecHolds has Copy, Drop where
    items : Vector<Item>
  spec PlainVecHolds where
    invariant items.length > 0 || (items[0]).count > 0

  fun pos_scalar(v : Vector<u64>, i : u64) -> Unit := ()
  spec pos_scalar where
    requires i < v.length && v[i] == 7
    aborts_if false
    ensures v[i] == 7

  fun pos_holds(v : &Vector<Item>, i : u64) -> u64 := (&v[i]).count
  spec pos_holds where
    requires i < v.length
    aborts_if false
    ensures result > 0 && (v[i]).count > 0

  fun neg_oob_u64_zero(v : Vector<u64>, i : u64) -> Unit := ()
  spec neg_oob_u64_zero where
    requires i >= v.length
    aborts_if false
    ensures v[i] == 0

  fun neg_oob_bool_false(v : Vector<Bool>, i : u64) -> Unit := ()
  spec neg_oob_bool_false where
    requires i >= v.length
    aborts_if false
    ensures !v[i]

  fun neg_plain_vec_zero() -> PlainVecZero :=
    new PlainVecZero { items := #[] }
  spec neg_plain_vec_zero where
    aborts_if false

  fun neg_plain_vec_holds() -> PlainVecHolds :=
    new PlainVecHolds { items := #[] }
  spec neg_plain_vec_holds where
    aborts_if false

  -- Staged for Phase 2 (`2b`/`2d`): certified `Vector<Item>` OOB fallback flips
  -- from `NTy.inhabitant` to `plainFallbackValue` in Phase 2.
  fun staged_certified_oob_holds(v : Vector<Item>, i : u64) -> Unit := ()
  spec staged_certified_oob_holds where
    requires i >= v.length
    aborts_if false
    ensures (v[i]).count > 0

verify 0x42::row4_vector::pos_scalar
verify 0x42::row4_vector::pos_holds

-- `neg_oob_u64_zero`, `neg_oob_bool_false`, `neg_plain_vec_zero`, `neg_plain_vec_holds`,
-- and `staged_certified_oob_holds` flip to `assertRejected` in Phase 2d when `indexVector`
-- switches its off-domain fallback to `plainFallbackValue`.

/-! ## Row 5: `ResultOf` (`result_of<f>(x)` when no run returns) -/

verex module 0x42::row5_result_of where
  pragma verify = false

  struct Item has Copy, Drop, Store where
    count : u64
  spec Item where
    invariant count > 0

  fun inc(x : u64) -> u64 := x + 1
  spec inc where
    aborts_if x + 1 > 18446744073709551615
    ensures result == x + 1
  verify inc

  fun build_item(x : u64) -> Item :=
    if x > 0 then new Item { count := x } else abort(1)
  spec build_item where
    aborts_if x == 0
    ensures result.count == x
  verify build_item

  fun apply_u64(f : Fn(u64) -> u64 has Copy, Drop, x : u64) -> u64 := invoke(f, x)
  spec apply_u64 where
    pragma opaque
    pragma verify = false
    aborts_if aborts_of<f>(x)
    ensures result == result_of<f>(x)

  fun apply_item(f : Fn(u64) -> Item has Copy, Drop, x : u64) -> Item := invoke(f, x)
  spec apply_item where
    pragma opaque
    pragma verify = false
    aborts_if aborts_of<f>(x)
    ensures result == result_of<f>(x)

  fun pos_returning(x : u64) -> u64 :=
    apply_u64(function[Fn(u64) -> u64 has Copy, Drop](inc, _), x)
  spec pos_returning where
    requires x < 100
    aborts_if false
    ensures result == x + 1

  fun pos_returning_holds(f : Fn(u64) -> Item has Copy, Drop, x : u64) -> u64 :=
    (apply_item(f, x)).count
  spec pos_returning_holds where
    requires !aborts_of<f>(x)
    aborts_if false
    ensures result > 0 && (result_of<f>(x)).count > 0

  fun neg_aborting_zero(f : Fn(u64) -> u64 has Copy, Drop, x : u64) -> Unit := ()
  spec neg_aborting_zero where
    requires aborts_of<f>(x)
    aborts_if false
    ensures result_of<f>(x) == 0

  -- Staged for Phase 2 (`2b`/`2d`): `ResultOf` default for `Item` switches from
  -- certified `typedInhabitant` to plain carrier in Phase 2.
  fun staged_aborting_holds(f : Fn(u64) -> Item has Copy, Drop, x : u64) -> Unit := ()
  spec staged_aborting_holds where
    requires aborts_of<f>(x)
    aborts_if false
    ensures (result_of<f>(x)).count > 0

verify 0x42::row5_result_of::pos_returning
verify 0x42::row5_result_of::pos_returning_holds

-- `neg_aborting_zero` and `staged_aborting_holds` flip to `assertRejected` in Phase 2d
-- when `ResultOf` switches its off-domain fallback to `plainFallback`.

/-! ## Prepared `plainFallback` decider-uninterpretedness controls -/

section PreparedPlainFallbackControls
open VerexIR VerexIR.Proofs.Denote Lean Elab Tactic

private def assertTacticFails (goalType : Lean.Expr) (tac : TSyntax `tactic) (label : String) :
    TermElabM Unit := do
  let mvar ← Meta.mkFreshExprMVar goalType
  let succeeded ← try
    let remaining ← Tactic.run mvar.mvarId! (Tactic.evalTactic tac)
    pure remaining.isEmpty
  catch _ => pure false
  if succeeded then
    throwError "expected `{label}` to remain unprovable for `{goalType}`"

run_cmd liftTermElabM do
  let zeroGoal ← Elab.Term.elabTerm
    (← `((@NTy.plainFallback Carriers.ground (.int 64 false) rfl).val = (0 : Int)))
    (some (.sort .zero))
  Elab.Term.synthesizeSyntheticMVarsNoPostponing
  assertTacticFails (← instantiateMVars zeroGoal) (← `(tactic| verex_decide))
    "prepared plainFallbackOf u64 == 0"
  let boolGoal ← Elab.Term.elabTerm
    (← `(@NTy.plainFallback Carriers.ground .bool rfl = false))
    (some (.sort .zero))
  Elab.Term.synthesizeSyntheticMVarsNoPostponing
  assertTacticFails (← instantiateMVars boolGoal) (← `(tactic| verex_decide))
    "prepared plainFallbackOf Bool == false"

end PreparedPlainFallbackControls

/-! ## Row 6: `nativeArbitrary` / `outsideValue` -/

verex module 0x42::row6_outside where
  pragma verify = false

  struct Item has Copy, Drop, Store where
    count : u64
  spec Item where
    invariant count > 0

  -- Derived spec function has `nativeArbitrary` on the `x == 0` abort branch.
  fun positive_only(x : u64) -> u64 :=
    if x > 0 then x else abort(1)

  -- Recursive fixed-width spec functions evaluate to `outsideValue` outside `u8`.
  spec fun count_u64(n : u8) : u64 :=
    if n == 0 then 5 else count_u64(n - 1)

  spec fun count_item(n : u8) : Item :=
    if n == 0 then new Item { count := 5 } else count_item(n - 1)

  fun item_or_abort(x : u64) -> Item :=
    if x > 0 then new Item { count := x } else abort(1)

  fun pos_arbitrary(x : u64) -> Unit := ()
  spec pos_arbitrary where
    requires x > 0
    aborts_if false
    ensures positive_only(x) == x

  fun pos_in_domain() -> Unit := ()
  spec pos_in_domain where
    aborts_if false
    ensures count_u64(0) == 5 && (count_item(0)).count == 5

  fun neg_arbitrary_zero() -> Unit := ()
  spec neg_arbitrary_zero where
    aborts_if false
    ensures positive_only(0) == 0

  fun neg_outside_u64_zero() -> Unit := ()
  spec neg_outside_u64_zero where
    aborts_if false
    ensures count_u64(300) == 0

  fun neg_outside_item_zero() -> Unit := ()
  spec neg_outside_item_zero where
    aborts_if false
    ensures (count_item(300)).count == 0

  fun neg_outside_item_holds() -> Unit := ()
  spec neg_outside_item_holds where
    aborts_if false
    ensures (count_item(300)).count > 0

  -- Staged for Phase 2 (`2b`/`2d`): `nativeArbitrary` on a nominal result type
  -- currently inhabits `Item.carrier` until Phase 2 switches carriers to plain.
  fun staged_arbitrary_holds() -> Unit := ()
  spec staged_arbitrary_holds where
    aborts_if false
    ensures (item_or_abort(0)).count > 0

verify 0x42::row6_outside::pos_arbitrary
verify 0x42::row6_outside::pos_in_domain

run_cmd do
  assertRejected `«0x42».row6_outside.neg_arbitrary_zero
    (← `(command| verify 0x42::row6_outside::neg_arbitrary_zero))
    "Row 6 nativeArbitrary == 0"
  assertRejected `«0x42».row6_outside.neg_outside_u64_zero
    (← `(command| verify 0x42::row6_outside::neg_outside_u64_zero))
    "Row 6 outsideValue u64 == 0"
  assertRejected `«0x42».row6_outside.neg_outside_item_zero
    (← `(command| verify 0x42::row6_outside::neg_outside_item_zero))
    "Row 6 outsideValue Item .count == 0"
  assertRejected `«0x42».row6_outside.neg_outside_item_holds
    (← `(command| verify 0x42::row6_outside::neg_outside_item_holds))
    "Row 6 outsideValue Item .count > 0"

/-! ## Row 7: `unspecifiedGhost` (unspecified ghost slots & iterator validity) -/

verex module 0x42::row7_ghost where
  pragma verify = false

  struct Item has Copy, Drop, Store where
    count : u64
  spec Item where
    invariant count > 0

  struct Tagged has Copy, Drop where
    x : u64
  spec Tagged where
    ghost tag : u64

  struct InitTagged has Copy, Drop where
    x : u64
  spec InitTagged where
    ghost tag : u64 := x + 1

  struct ItemTagged has Copy, Drop where
    x : u64
  spec ItemTagged where
    ghost tag : Item

  struct MapState has Copy, Drop where
    len : u64
  spec MapState where
    ghost validity : u64

  struct Iter has Copy, Drop where
    pos : u64
  spec Iter where
    ghost validity : u64

  fun iter_new(m : &MapState) -> Iter := do
    let mut it := new Iter { pos := 0 }
    spec do
      update it.validity := m.validity
    it
  spec iter_new where
    aborts_if false
    ensures result.validity == m.validity

  fun pos_initialized(x : u64) -> InitTagged :=
    new InitTagged { x := x }
  spec pos_initialized where
    requires x < 100
    aborts_if false
    ensures result.tag == x + 1

  fun pos_iter_new(m : &MapState) -> Iter :=
    iter_new(m)
  spec pos_iter_new where
    aborts_if false
    ensures result.validity == m.validity

  fun neg_spec_pack_zero() -> Unit := ()
  spec neg_spec_pack_zero where
    aborts_if false
    ensures (new Tagged { x := 1 }).tag == 0

  fun neg_iter_new_zero(m : &MapState) -> Iter :=
    iter_new(m)
  spec neg_iter_new_zero where
    aborts_if false
    ensures (new Iter { pos := 0 }).validity == 0

  fun neg_item_ghost_zero() -> Unit := ()
  spec neg_item_ghost_zero where
    aborts_if false
    ensures ((new ItemTagged { x := 1 }).tag).count == 0

  -- Staged for Phase 2 (`2b`/`2d`): `ghostValues` currently types `unspecifiedGhost`
  -- at `Item.carrier` until Phase 2 switches carriers to plain.
  fun staged_item_ghost_holds() -> Unit := ()
  spec staged_item_ghost_holds where
    aborts_if false
    ensures ((new ItemTagged { x := 1 }).tag).count > 0

verify 0x42::row7_ghost::iter_new
verify 0x42::row7_ghost::pos_initialized
verify 0x42::row7_ghost::pos_iter_new

run_cmd do
  assertRejected `«0x42».row7_ghost.neg_spec_pack_zero
    (← `(command| verify 0x42::row7_ghost::neg_spec_pack_zero))
    "Row 7 unspecifiedGhost == 0"
  assertRejected `«0x42».row7_ghost.neg_iter_new_zero
    (← `(command| verify 0x42::row7_ghost::neg_iter_new_zero))
    "Row 7 uninitialized iterator ghost validity == 0"
  assertRejected `«0x42».row7_ghost.neg_item_ghost_zero
    (← `(command| verify 0x42::row7_ghost::neg_item_ghost_zero))
    "Row 7 nominal unspecifiedGhost .count == 0"

/-! ## Rejection-helper budget-exhaustion and infrastructure-failure controls -/

verex module 0x42::rejection_helper_control where
  pragma verify = false

  fun pos_branch(b : Bool) -> u64 :=
    if b then 11 else 12
  spec pos_branch where
    aborts_if false
    ensures result == (if b then 11 else 12)

run_cmd do
  let before ← get
  let refusedBudget ← try
    assertRejected `«0x42».rejection_helper_control.pos_branch
      (← `(command| set_option verex.verifyHeartbeats 1 in
        verify 0x42::rejection_helper_control::pos_branch))
      "budget-exhausted positive control"
    pure none
  catch error =>
    pure (some (← error.toMessageData.toString))
  let afterBudget ← get
  unless afterBudget.messages.toList.length == before.messages.toList.length do
    modify fun _ => before
    throwError "assertRejected leaked diagnostics when refusing a budget-exhausted run"
  let some budgetRefusal := refusedBudget
    | throwError "assertRejected silently accepted a budget-exhausted run on a valid function"
  unless budgetRefusal.contains "exceeded its budget" ||
      budgetRefusal.contains "maximum number of heartbeats" do
    throwError "assertRejected refused budget exhaustion for the wrong reason: {budgetRefusal}"

  let refusedMissing ← try
    assertRejected `«0x42».rejection_helper_control.missing_fn
      (← `(command| verify 0x42::rejection_helper_control::missing_fn))
      "unknown-function infrastructure control"
    pure none
  catch error =>
    pure (some (← error.toMessageData.toString))
  let afterMissing ← get
  unless afterMissing.messages.toList.length == before.messages.toList.length do
    modify fun _ => before
    throwError "assertRejected leaked diagnostics when refusing an unknown-function error"
  let some missingRefusal := refusedMissing
    | throwError "assertRejected silently accepted an unknown-function elaboration error"
  unless missingRefusal.contains "unknown function" do
    throwError "assertRejected refused unknown function for the wrong reason: {missingRefusal}"

verify 0x42::rejection_helper_control::pos_branch

end VerexSL.Tests.TotalReadSoundness
