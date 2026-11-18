-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! # The validity of a call's arguments and of its result

A call owes the data invariants of the callee's arguments and receives those
of its result. The validity condition is normalized to one conjunct per row
position, split by argument, and each remaining fact is opened at the
argument it belongs to; the result's validity stays a folded fact of the
continuation, and a contract stating none leaves no premise.

The controls run on a cache hit (a second call of the same callee with the
same argument pattern), on first use (the first such call), and with
`verex.contractLemmas = false`, where the strict and the general rule apply
in place of the rule taking the contract's equations.
-/

namespace VerexSL.Tests.CallArgumentValidity

set_option Elab.async false
set_option maxHeartbeats 1000000
set_option verex.verifyHeartbeats 1000000

verex module 0x42::call_argument_validity where
  pragma verify = false

  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address
  native fun host_borrow {K} {V}(t : &Table<K, V>, k : K) -> &V
  @[map_borrow (Table)]
  public fun borrow {K} {V}(t : &Table<K, V>, k : K) -> &V := host_borrow(t, k)
  @[map_spec_has_key (Table)]
  opaque spec fun has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_spec_get (Table)]
  opaque spec fun get {K} {V}(t : Table<K, V>, k : K) : V

  struct Percent has Copy, Drop, Store where
    value : u64
  spec Percent where
    invariant value <= 100

  -- A holder without an invariant of its own: a field of it is an argument
  -- the context states no fact of, facts being per binder and never per
  -- projection.
  struct Holder has Copy, Drop, Store where
    inner : Percent
    tag : u64

  -- A row without invariants.
  fun plain(x : u64, y : u64) -> u64 := x
  spec plain where
    pragma opaque = true
    aborts_if false
    ensures result == x

  -- A struct argument after an invariant-free one.
  fun by_value(x : u64, p : Percent) -> u64 := p.value
  spec by_value where
    pragma opaque = true
    aborts_if false
    ensures result <= 100

  -- A `&mut` argument, whose fact is held for its referent.
  fun by_mut(p : &mut Percent) -> Unit := ()
  spec by_mut where
    pragma opaque = true
    aborts_if false
    ensures p.value == old(p.value)

  -- A tuple argument, whose reading nests inside its position.
  fun by_tuple(pair : (u64, Percent)) -> u64 := pair[0u32]
  spec by_tuple where
    pragma opaque = true
    aborts_if false

  -- A generic argument, which the caller knows at its own family.
  fun generic {T}(x : T) -> T := x
  spec generic where
    pragma opaque = true
    aborts_if false
    ensures result == x

  -- A callee whose failure conditions do not coincide: the general rule.
  fun may_abort(x : u64, p : Percent) -> u64 := x / p.value
  spec may_abort where
    pragma opaque = true
    aborts_if p.value == 0
    ensures result <= x

  fun use_plain(x : u64) -> u64 := do
    let a := plain(x, x)
    let b := plain(a, x)
    plain(b, x)
  spec use_plain where
    aborts_if false
    ensures result == x

  fun use_by_value(p : Percent) -> u64 := do
    let a := by_value(1, p)
    let b := by_value(a, p)
    by_value(b, p)
  spec use_by_value where
    aborts_if false
    ensures result <= 100

  fun use_by_mut(p : &mut Percent) -> Unit := do
    by_mut(p)
    by_mut(p)
  spec use_by_mut where
    aborts_if false

  fun use_by_tuple(p : Percent) -> u64 := do
    let a := by_tuple((1, p))
    by_tuple((a, p))
  spec use_by_tuple where
    aborts_if false

  fun use_generic(p : Percent) -> Percent := do
    let q := generic(p)
    generic(q)
  spec use_generic where
    aborts_if false
    ensures result.value <= 100

  -- The generic `borrow`, whose row and value transport between the callee's
  -- family and the caller's.
  fun use_borrow(t : &Table<u64, Percent>, a : u64, b : u64) -> u64 := do
    let first := borrow(t, a).value
    let second := borrow(t, b).value
    first + second
  spec use_borrow where
    requires has(t, a)
    requires has(t, b)
    aborts_if false
    ensures result <= 200

  fun use_may_abort(x : u64, p : Percent) -> u64 := do
    let a := may_abort(x, p)
    may_abort(a, p)
  spec use_may_abort where
    requires p.value > 0
    aborts_if false
    ensures result <= x

  fun by_field(h : Holder) -> u64 := by_value(1, h.inner)
  spec by_field where
    aborts_if false

  fun by_field_after_plain(h : Holder, v : u64) -> u64 := do
    let a := plain(v, v)
    by_value(a, h.inner)
  spec by_field_after_plain where
    aborts_if false

  -- A tuple argument whose struct component the context states no fact of:
  -- the reading's inner atoms are each the opener's.
  fun by_tuple_field(h : Holder) -> u64 := by_tuple((1, h.inner))
  spec by_tuple_field where
    aborts_if false

  -- A structured selector takes an argument's validity in an authored proof.
  -- Both calls take the same argument pattern, so the first meets it and the
  -- second reads the contract's normal forms from the cache.
  fun selected(h : Holder) -> u64 := do
    let a := by_value(1, h.inner)
    by_value(a, h.inner)
  spec selected where
    aborts_if false
  verify selected by lean =>
    verex_obligations
    verex_on
    | argument 2 of by_value at call 1 => verex_solve
    | argument 2 of by_value at call 2 => verex_solve
    | _ => verex_solve

  fun selected_generic(h : Holder) -> Percent := generic(h.inner)
  spec selected_generic where
    aborts_if false
    ensures result.value <= 100
  verify selected_generic by lean =>
    verex_obligations
    verex_on
    | argument 1 of generic at call 1 => verex_solve
    | _ => verex_solve

  -- A callee whose result is a tuple of two values with invariants: its
  -- result validity enters the context as one fact per component.
  fun two_percents(x : u64) -> (Percent, Percent) :=
    (new Percent { value := 1 }, new Percent { value := 2 })
  spec two_percents where
    pragma opaque = true
    aborts_if false

  -- Each component's fact is found by an exact match, so neither call's
  -- argument validity leaves an obligation; a step placed at one would run.
  fun use_two(x : u64) -> u64 := do
    let pair := two_percents(x)
    let a := by_value(1, pair[0u32])
    by_value(a, pair[1u32])
  spec use_two where
    aborts_if false
  verify use_two by lean =>
    verex_obligations
    verex_on
    | argument 2 of by_value at call 1 => fail
    | argument 2 of by_value at call 2 => fail
    | _ => verex_solve

  -- A claim about one component the invariant does not give.
  fun wrong_component(x : u64) -> u64 := do
    let pair := two_percents(x)
    pair[0u32].value
  spec wrong_component where
    aborts_if false
    ensures result <= 50

  -- A struct argument whose data invariant the caller does not establish.
  fun unchecked(v : u64) -> u64 := by_value(1, new Percent { value := v })
  spec unchecked where
    aborts_if false

  -- The same value where it reaches the call through a mutable reference,
  -- and where the call takes that reference.
  fun unchecked_copy(p : &mut Percent, v : u64) -> u64 := do
    p.value := v
    by_value(1, *p)
  spec unchecked_copy where
    aborts_if false

  fun unchecked_handed(p : &mut Percent, v : u64) -> Unit := do
    p.value := v
    by_mut(p)
  spec unchecked_handed where
    aborts_if false

/-! ## The arguments' and the result's validity on each path -/

verify 0x42::call_argument_validity::use_plain
verify 0x42::call_argument_validity::use_by_value
verify 0x42::call_argument_validity::use_by_mut
verify 0x42::call_argument_validity::use_by_tuple
verify 0x42::call_argument_validity::use_generic
verify 0x42::call_argument_validity::use_borrow
verify 0x42::call_argument_validity::use_may_abort
verify 0x42::call_argument_validity::by_field
verify 0x42::call_argument_validity::by_field_after_plain
verify 0x42::call_argument_validity::by_tuple_field
verify 0x42::call_argument_validity::use_two
set_option verex.contractLemmas false in
verify 0x42::call_argument_validity::use_plain

set_option verex.contractLemmas false in
verify 0x42::call_argument_validity::use_by_value

set_option verex.contractLemmas false in
verify 0x42::call_argument_validity::use_by_mut

set_option verex.contractLemmas false in
verify 0x42::call_argument_validity::use_by_tuple

set_option verex.contractLemmas false in
verify 0x42::call_argument_validity::use_generic

set_option verex.contractLemmas false in
verify 0x42::call_argument_validity::use_borrow

set_option verex.contractLemmas false in
verify 0x42::call_argument_validity::use_may_abort

set_option verex.contractLemmas false in
verify 0x42::call_argument_validity::by_field

set_option verex.contractLemmas false in
verify 0x42::call_argument_validity::by_tuple_field

set_option verex.contractLemmas false in
verify 0x42::call_argument_validity::use_two

-- The same selectors take the same obligations where no normal form of the
-- contract is cached, so the three paths agree on the conjunct each argument
-- owes.
/-! ## The structured selector of an argument's validity

An authored proof selects the obligation by `argument k of f at call n`. The
proofs of `selected` and `selected_generic` place a step there, on the first
use of the contract's normal forms and on a cache hit, and verify.
-/

verify 0x42::call_argument_validity::selected
verify 0x42::call_argument_validity::selected_generic

set_option verex.contractLemmas false in
verify 0x42::call_argument_validity::selected

set_option verex.contractLemmas false in
verify 0x42::call_argument_validity::selected_generic

/-! ## The residual an argument's validity leaves

A step that fails where the selector placed it reports the obligation the
solver routed to it: the goal is the clause of the argument's data invariant,
the case is named by the obligation's key, and the position counts the
argument of the row — the second here, after one without invariants.
-/

open Lean Elab Command in
/-- Elaborating `command` reports an error containing each of `expected`. -/
elab "#guard_reports " expected:str+ " in " command:command : command => do
  let saved ← get
  let before := saved.messages.toList.length
  try elabCommand command catch _ => pure ()
  let fresh := ((← get).messages.toList.drop before).filter (·.severity == .error)
  let texts ← fresh.mapM fun message => message.data.toString
  modify fun _ => saved
  for text in expected do
    unless texts.any (fun reported => (reported.splitOn text.getString).length > 1) do
      throwError "no reported obligation at `{text.getString}`: {texts}"

#guard_reports "argument_2_of_by_value_at_call_1" "h_inner_value.val ≤ 100" in
verex module 0x42::call_argument_residual where
  pragma verify = false
  struct Percent has Copy, Drop, Store where
    value : u64
  spec Percent where
    invariant value <= 100
  struct Holder has Copy, Drop, Store where
    inner : Percent
    tag : u64
  fun by_value(x : u64, p : Percent) -> u64 := p.value
  spec by_value where
    pragma opaque = true
    aborts_if false
    ensures result <= 100
  fun routed(h : Holder) -> u64 := by_value(1, h.inner)
  spec routed where
    aborts_if false
  verify routed by lean =>
    verex_obligations
    verex_on
    | argument 2 of by_value at call 1 => fail
    | _ => verex_solve

/-! ## The provenance of an argument's validity

Its description names the argument's position in the callee's row, counting
from one, and its key spells the position with the callee and the call.
-/

open VerexIR.Proofs.Denote in
#guard (Provenance.callArgument "0x42::call_argument_validity::by_value" (some 1) 2).describe ==
  "argument 2 of call 1 of `0x42::call_argument_validity::by_value`"

open VerexIR.Proofs.Denote in
#guard (Provenance.callArgument "0x42::call_argument_validity::by_value" none 2).describe ==
  "argument 2 of the call to `0x42::call_argument_validity::by_value`"

open VerexIR.Proofs.Denote in
#guard ({ kind := "argument", index := some 2, callee := "by_value", call := some 1 } :
  ObligationKey).text == "argument 2 of by_value at call 1"

/--
error: the specification clause `ensures result <= 50` is not established
---
error: verex verification failed: the automatic verification of `wrong_component` failed; provide a proof in the module:
  verify wrong_component by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::call_argument_validity::wrong_component

/-! ## Negative controls

A value reaching a call owes its data invariants where it comes about: a
construction, a copy out of a mutable reference, or the hand-over of that
reference. The obligation is reported there, so the argument's own position
appears in no source-level diagnostic of these programs.
-/

/--
error: the data invariant `invariant value <= 100` does not hold where a value is constructed
---
error: verex verification failed: the automatic verification of `unchecked` failed; provide a proof in the module:
  verify unchecked by lean =>
    verex_obligations
    verex_on
    | data_invariant_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::call_argument_validity::unchecked

/--
error: the data invariant `invariant value <= 100` does not hold where a value is copied out of a mutable reference
---
error: verex verification failed: the automatic verification of `unchecked_copy` failed; provide a proof in the module:
  verify unchecked_copy by lean =>
    verex_obligations
    verex_on
    | data_invariant_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::call_argument_validity::unchecked_copy

/--
error: the data invariant `invariant value <= 100` does not hold where a mutable reference is passed to a call
---
error: verex verification failed: the automatic verification of `unchecked_handed` failed; provide a proof in the module:
  verify unchecked_handed by lean =>
    verex_obligations
    verex_on
    | data_invariant_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::call_argument_validity::unchecked_handed

/-! A row's validity is split whether or not the row is written out as a pair.
Written out, the law of the constructor states the head and the tail
themselves (`HList.holds_cons_mk`), so each fact names its parameter; over a
value that is no constructor application the general law
(`HList.holds_cons`) states them as the row's projections, and the facts are
the projections. Both leave one hypothesis per position, in the row's order. -/

open Lean Meta Elab Command VerexIR.Proofs.Denote in
run_cmd liftTermElabM do
  let τ := mkApp2 (mkConst ``NTy.int) (mkNatLit 64) (mkConst ``Bool.false)
  let row := mkApp2 (mkConst ``NRow.cons) τ (mkApp2 (mkConst ``NRow.cons) τ (mkConst ``NRow.nil))
  withLocalDeclD `Θ (mkConst ``Carriers) fun carriers => do
    let sorts := VerexSL.Contract.carrierSorts carriers
    let rowType := mkApp2 (mkConst ``HList) sorts row
    let held (values : Lean.Expr) : Lean.Expr :=
      mkApp3 (mkConst ``HList.Holds) carriers row values
    let component (index : Nat) (values : Lean.Expr) : MetaM Lean.Expr :=
      mkAppM (if index == 0 then ``Prod.fst else ``Prod.snd) #[values]
    -- A row that is no constructor application: the general law.
    withLocalDeclD `values rowType fun values => do
      withLocalDeclD `valid (held values) fun valid => do
        let goal := (← mkFreshExprMVar (mkConst ``True)).mvarId!
        let (_, goal) ← goal.assertHypotheses
          #[{ userName := `valid, type := held values, value := valid }]
        let split ← splitEntryValidity goal `valid #[`first, `second]
        split.withContext do
          let lctx ← getLCtx
          let some head := lctx.findFromUserName? `first | throwError "no first fact"
          unless (← instantiateMVars head.type).isAppOfArity ``NTy.Holds 3 do
            throwError "the first fact of a row that is no pair is not a fact: {head.type}"
          unless ((← instantiateMVars head.type).getArg! 2).isAppOfArity ``Prod.fst 3 do
            throwError "the general law did not state the row's projection: {head.type}"
          let some tail := lctx.findFromUserName? `second | throwError "no second fact"
          unless ((← instantiateMVars tail.type).getArg! 2).isAppOf ``Prod.fst do
            throwError "the second fact is not of the tail's head: {tail.type}"
        discard (component 0 values)
    -- The same row written out as a pair: the facts name the components.
    withLocalDeclD `head (mkApp2 (mkConst ``NTy.carrier) sorts τ) fun first => do
    withLocalDeclD `tail (mkApp2 (mkConst ``HList) sorts
        (mkApp2 (mkConst ``NRow.cons) τ (mkConst ``NRow.nil))) fun rest => do
      let pair ← mkAppM ``Prod.mk #[first, rest]
      withLocalDeclD `valid (held pair) fun valid => do
        let goal := (← mkFreshExprMVar (mkConst ``True)).mvarId!
        let (_, goal) ← goal.assertHypotheses
          #[{ userName := `valid, type := held pair, value := valid }]
        let split ← splitEntryValidity goal `valid #[`first, `second]
        split.withContext do
          let some head := (← getLCtx).findFromUserName? `first | throwError "no first fact"
          unless ((← instantiateMVars head.type).getArg! 2) == first do
            throwError "the written-out row's first fact is not of its head: {head.type}"

/-! ## The argument validity of an invocation of a function value

An invocation of a function value the proof does not see owes the data
invariants of the arguments it passes.  The condition is the row's, so it is
read by the reader every call's condition has: the conditions below arrive
as the entry's row validity, which preparation splits, and as a value
certified where it is copied out of a mutable reference.
-/

verex module 0x42::invocation_argument_validity where
  -- The argument is the caller's own parameter at a type parameter: its
  -- fact is a conjunct of the entry's row validity until preparation splits
  -- it, and the atom the invocation owes is stated at the caller's family.
  fun pass {T} {R}(f : Fn(T) -> R, x : T) -> R := invoke(f, x)
  spec pass where
    ensures ensures_of<f>(x, result)

  -- Two positions of one row: each argument's atom is read at its own
  -- position of the condition.
  fun pass_two {T} {R}(f : Fn(T, T) -> R, x : T, y : T) -> R := invoke(f, x, y)
  spec pass_two where
    ensures ensures_of<f>(x, y, result)

  -- The argument is copied out of a mutable reference's current, which is
  -- certified where it is copied; the invocation reads that fact.
  fun pass_current {T} {R}(f : Fn(T) -> R, r : &mut T) -> R := invoke(f, *r)
  spec pass_current where
    ensures ensures_of<f>(old(*r), result)

/-! ## An invocation the rule does not take

A function value whose rows are a declared type, or a tuple, is outside the
rule's domain: the rule requires the rows to resolve to scalars at the frame,
which the frontend states of a type parameter's row and of the result row
(`wp_closureMeaning_unseen_at`).  The invocation is
not taken and the leaf reports what it reports of any undecided leaf; the
argument's validity is never reached.  In the source shapes examined here an
invocation's arguments are the caller's own, whose validity its entry states,
so none of them leaves an atom unpaid; that is a reading of these examples
and not a property of every program or of a later rule.  An unpaid atom is
exercised where the rule a dictionary's contract gives takes the invocation
(`VerexIR/Tests/UnseenArgumentValidity.lean`).
-/

#guard_reports "the automatic verification of `pass_percent` failed"
    "verification failed with a residual obligation" in
verex module 0x42::invocation_argument_domain where
  struct Percent has Copy, Drop, Store where
    value : u64
  spec Percent where
    invariant value <= 100

  fun pass_percent {R}(f : Fn(Percent) -> R, p : Percent) -> R := invoke(f, p)
  spec pass_percent where
    ensures ensures_of<f>(p, result)

end VerexSL.Tests.CallArgumentValidity
