-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! Optional storage observations retain the raw memory equation while converting
its found value. An unrelated resource invariant must not make that option
normalization recur at a concrete call of a generic function. -/
verex module 0x42::generic_storage_invariants where
  pragma verify = false

  struct Counter has Key where
    value : u64

  struct Coin {C : phantom type} has Key where
    value : u64

  struct Bounded has Key where
    value : u64

  spec Bounded where
    invariant value <= 100

  struct A where
    dummy_field : Bool

  struct B where
    dummy_field : Bool

  public fun deposit {C}(addr : Address, amount : u64) -> Unit := do
    let coin := &mut Coin<C>[addr]
    coin.value := coin.value + amount

  spec deposit where
    aborts_if !exists<Coin<C> >(addr)
    aborts_if global<Coin<C> >(addr).value + amount > MAX_U64
    ensures global<Coin<C> >(addr).value
        == old(global<Coin<C> >(addr).value) + amount
    modifies global<Coin<C> >(addr), *

  public fun deposit_a(addr : Address, amount : u64) -> Unit := do
    deposit::<A>(addr, amount)

  spec deposit_a where
    aborts_if !exists<Coin<A> >(addr)
    aborts_if global<Coin<A> >(addr).value + amount > MAX_U64
    ensures global<Coin<A> >(addr).value
        == old(global<Coin<A> >(addr).value) + amount
    ensures exists<Coin<B> >(addr) == old(exists<Coin<B> >(addr))
    ensures global<Counter>(addr) == old(global<Counter>(addr))
    modifies *

  public fun break_bounded(addr : Address) -> Unit := do
    let value := &mut Bounded[addr].value
    *value := 101
  spec break_bounded where
    requires exists<Bounded>(addr)
    modifies global<Bounded>(addr)
    aborts_if false

verify 0x42::generic_storage_invariants::deposit
verify 0x42::generic_storage_invariants::deposit_a

/--
error: the data invariant `invariant value <= 100` does not hold where a mutation ends
---
error: verex verification failed: the automatic verification of `break_bounded` failed; provide a proof in the module:
  verify break_bounded by lean =>
    verex_obligations
    verex_on
    | data_invariant_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::generic_storage_invariants::break_bounded

/-! A declaration's data invariant moves to the frame its type arguments
induce at any instance row, not only at the declaration's own parameters: the
one statement (`Skolems.encode_reframeAtInstance`) at an instance over another
parameter and at a nested one, which the kernel checks here, and which the
solver's one move applies on the goal and on a hypothesis alike
(`reframedOwnAtInstance`).  The row that move is stated at is the frame's
resolutions of the instance's arguments, and the row a statement carries is
their normal form; the controls below state that equation, the inhabitability
the solver builds of such a row, and the negative where the row is not those
resolutions. -/
namespace ReframeAtInstance

open Lean Elab Tactic Meta VerexIR VerexIR.Proofs.Denote


/-- A declaration of two parameters holding an address. -/
def owner : NTy :=
  .struct ⟨⟨0⟩, 1⟩ (.cons (.param 0) (.cons (.param 1) .nil)) (.cons .address .nil)

/-- A declaration of one parameter holding it. -/
def tagged : NTy :=
  .struct ⟨⟨0⟩, 0⟩ (.cons (.param 0) .nil) (.cons (.param 0) .nil)

/-- The instance over another parameter: `owner<u64, tagged<T>>`. -/
def instanceRow : NRow := .cons (.int 64 false) (.cons (tagged.subst (.cons (.param 0) .nil)) .nil)

/-- A nested instance: `owner<u64, tagged<tagged<T>>>`. -/
def nestedRow : NRow :=
  .cons (.int 64 false)
    (.cons (tagged.subst (.cons (tagged.subst (.cons (.param 0) .nil)) .nil)) .nil)

-- The move at an instance over another parameter, by the one statement.
example {unit : Validation.ValidatedUnit} {functions base : FunctionCarriers unit}
    (sameCarrier : functions.carrier = base.carrier)
    (sameFunctions : HEq functions.functions base.functions)
    (sameNominals : functions.nominals = base.nominals)
    (Θ : @Skolems unit functions)
    (inhabitable : (instanceRow.map Θ.resolve).inhabitable = true)
    (row : @NTy.carrier Θ.toCarriers (owner.subst instanceRow)) :
    @NTy.encode
        (Skolems.atArguments unit base (instanceRow.map Θ.resolve) inhabitable).toCarriers owner
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ instanceRow
          inhabitable (by decide) row) =
      @NTy.encode Θ.toCarriers (owner.subst instanceRow) row :=
  Skolems.encode_reframeAtInstance sameCarrier sameFunctions sameNominals Θ instanceRow
    inhabitable (by decide) row

-- The same at a nested instance.
example {unit : Validation.ValidatedUnit} {functions base : FunctionCarriers unit}
    (sameCarrier : functions.carrier = base.carrier)
    (sameFunctions : HEq functions.functions base.functions)
    (sameNominals : functions.nominals = base.nominals)
    (Θ : @Skolems unit functions)
    (inhabitable : (nestedRow.map Θ.resolve).inhabitable = true)
    (row : @NTy.carrier Θ.toCarriers (owner.subst nestedRow)) :
    @NTy.encode
        (Skolems.atArguments unit base (nestedRow.map Θ.resolve) inhabitable).toCarriers owner
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ nestedRow
          inhabitable (by decide) row) =
      @NTy.encode Θ.toCarriers (owner.subst nestedRow) row :=
  Skolems.encode_reframeAtInstance sameCarrier sameFunctions sameNominals Θ nestedRow
    inhabitable (by decide) row

-- A negative: the row's entries are not the declaration's parameters, so the
-- move at the parameter row does not state this.
example : NRow.isParamsFrom 0 instanceRow = false := by decide

/-- A closed instance of the declaration: `owner<u64, u64>`. -/
def closedRow : NRow := .cons (.int 64 false) (.cons (.int 64 false) .nil)

-- The third case, at a closed instance: the frame the row induces and the
-- leaf's frame read the declaration's type alike, so the value stands at both
-- and the equation of their encodings is its own. The kernel checks the
-- conversion here, as the solver checks it at its construction site.
example {unit : Validation.ValidatedUnit} {functions base : FunctionCarriers unit}
    (Θ : @Skolems unit functions)
    (inhabitable : closedRow.inhabitable = true)
    (row : @NTy.carrier Θ.toCarriers (owner.subst closedRow)) :
    @NTy.encode (Skolems.atArguments unit base closedRow inhabitable).toCarriers owner row =
      @NTy.encode Θ.toCarriers (owner.subst closedRow) row := rfl

-- The row a statement carries is the normal form of those resolutions: a
-- closed entry stands unresolved, an entry over a parameter resolved, and the
-- frame's own laws prove the equation the solver reads the row by.
example {unit : Validation.ValidatedUnit} {functions : FunctionCarriers unit}
    (Θ : @Skolems unit functions) :
    instanceRow.map Θ.resolve =
      .cons (.int 64 false) (.cons (Θ.resolve (tagged.subst (.cons (.param 0) .nil))) .nil) := by
  simp only [instanceRow, NRow.map, vir_denote_norm]

-- A negative: a row that is not those resolutions has no such equation, so no
-- statement is read at it.
example {unit : Validation.ValidatedUnit} {functions : FunctionCarriers unit}
    (Θ : @Skolems unit functions) : Θ.resolve .unit = Θ.resolve .unit := by
  fail_if_success
    (have : instanceRow.map Θ.resolve = .cons (.int 64 false) (.cons (.param 0) .nil) := by
      simp only [instanceRow, NRow.map, vir_denote_norm])
  rfl

/-- The inhabitability of a row of a frame's resolutions, built entry by entry
by the solver's builder (`rowInhabitableEntries?`) and handed to the kernel
here: a closed entry is decided, an entry the frame resolves holds by the
frame's own law. -/
elab "built_row_inhabitable" : tactic => do
  let goal ← getMainGoal
  let some (_, stated, _) := (← instantiateMVars (← goal.getType)).eq?
    | throwError "the control's goal states no row's inhabitability"
  let_expr VerexIR.Proofs.Denote.NRow.inhabitable row := stated
    | throwError "the control's goal states no row's inhabitability"
  let some built ← rowInhabitableEntries? row
    | throwError "the builder declined the row {row}"
  goal.assign built

example {unit : Validation.ValidatedUnit} {functions : FunctionCarriers unit}
    (Θ : @Skolems unit functions) :
    (NRow.cons (.int 64 false)
        (.cons (Θ.resolve (tagged.subst (.cons (.param 0) .nil))) .nil)).inhabitable = true := by
  built_row_inhabitable

example {unit : Validation.ValidatedUnit} {functions : FunctionCarriers unit}
    (Θ : @Skolems unit functions) :
    (NRow.cons (.int 64 false)
        (.cons (Θ.resolve (tagged.subst (.cons (tagged.subst (.cons (.param 0) .nil)) .nil)))
          .nil)).inhabitable = true := by
  built_row_inhabitable

end ReframeAtInstance

/-! A declaration's data invariant at an instance over another parameter, on
both sides of a leaf: the owner of a table of values of its own parameter owes
the table's invariant at `Table<u64, Tagged<T>>`, and a read of such a table
states it as a hypothesis, which the leaf of a failing clause reads — so that
leaf states no encoding (`doc/designs/native-verification.md`). -/
verex module 0x42::instance_row_invariants where
  pragma verify = false
  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address
  native fun host_contains {K} {V}(t : &Table<K, V>, k : K) -> Bool
  native fun host_borrow {K} {V}(t : &Table<K, V>, k : K) -> &V
  @[map_has_key (Table)]
  public fun contains {K} {V}(t : &Table<K, V>, k : K) -> Bool := host_contains(t, k)
  @[map_spec_has_key (Table)]
  opaque spec fun has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_borrow (Table)]
  public fun borrow {K} {V}(t : &Table<K, V>, k : K) -> &V := host_borrow(t, k)

  opaque spec fun permitted {T}(value : u64) : Bool
  struct Tagged {T} has Copy, Drop, Store where
    value : u64
  spec Tagged where
    invariant permitted::<T>(value)

  struct Slot {T} has Copy, Drop, Store where
    item : T
  struct Slots {T} has Store where
    slots : Table<u64, Slot<T> >

  -- The goal side: the argument's validity owes the table's invariant at an
  -- instance over the function's own parameter.
  public fun slots_has {T}(s : &Slots<T>, k : u64) -> Bool := contains(&s.slots, k)
  spec slots_has where
    aborts_if false
    ensures result == has(s.slots, k)
  verify slots_has

  -- The hypothesis side: the clause names another instance's predicate and is
  -- not established, and the leaf reported for it states no encoding.
  public fun wrong_instance {T}(t : &Table<u64, Tagged<T> >, k : u64) -> u64 :=
    borrow(t, k).value
  spec wrong_instance where
    requires has(t, k)
    aborts_if false
    ensures permitted::<Bool>(result) -- error: another instance's predicate

  -- A closed instance inside a generic function: the frame the arguments
  -- induce and the function's own frame read `Table<u64, u64>` alike, so the
  -- value stands at both and both sides read the same statement of it.
  struct Counts {T} has Store where
    counts : Table<u64, u64>
  public fun counts_has {T}(c : &Counts<T>, k : u64) -> Bool := contains(&c.counts, k)
  spec counts_has where
    aborts_if false
    ensures result == has(c.counts, k)
  verify counts_has

  public fun counts_wrong {T}(c : &Counts<T>, k : u64) -> Bool := contains(&c.counts, k)
  spec counts_wrong where
    aborts_if false
    ensures result -- error: the key may be absent

-- The reported leaf states no encoding: the clause it cannot establish is all
-- that is reported.
/--
error: the specification clause `ensures permitted::<Bool>(result)` is not established
---
error: verex verification failed: the automatic verification of `wrong_instance` failed; provide a proof in the module:
  verify wrong_instance by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::instance_row_invariants::wrong_instance

-- The closed instance's negative: a wrong claim about it is still reported,
-- and nothing else is.
/--
error: the specification clause `ensures result` is not established
---
error: verex verification failed: the automatic verification of `counts_wrong` failed; provide a proof in the module:
  verify counts_wrong by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::instance_row_invariants::counts_wrong
