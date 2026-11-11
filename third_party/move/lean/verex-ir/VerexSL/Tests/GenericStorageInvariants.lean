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

/--
error: the specification clause `aborts_if !exists<Coin<A> >(addr)` is not established
---
error: the specification clause `aborts_if global<Coin<A> >(addr).value + amount > MAX_U64` is not established
---
error: the specification clause `ensures global<Coin<A> >(addr).value
        == old(global<Coin<A> >(addr).value) + amount` is not established
---
error: the specification clause `ensures exists<Coin<B> >(addr) == old(exists<Coin<B> >(addr))` is not established
---
error: verification failed with a residual obligation:
case some.left.continues.continues.continues.right.right.right
verexInhabitants : VerexIR.Proofs.Denote.CarriersNonempty (VerexIR.Proofs.Denote.Carriers.runtime unit)
addr : VerexIR.Proofs.Denote.NTy.address.carrier
amount : (VerexIR.Proofs.Denote.NTy.int 64 false).carrier
initialState✝ : VerexIR.Proofs.Denote.Memory unit
assumed permitted : True
verexAnchor : VerexIR.Proofs.Denote.AnchorSaved 14 (some addr, some amount, none, ()) initialState✝
val✝ :
  (0x42.generic_storage_invariants.unit.nominal.namespace.0.declaration✝¹⁻⁰
      0x42.generic_storage_invariants.unit.nominal.namespace.0.declaration✝³⁻⁰).carrier
heq :
  initialState✝
      {
        type :=
          0x42.generic_storage_invariants.unit.nominal.namespace.0.declaration✝¹⁻⁰
            0x42.generic_storage_invariants.unit.nominal.namespace.0.declaration✝³⁻⁰,
        arguments :=
          VerexIR.Proofs.Denote.NRow.cons
            (VerexIR.Proofs.Denote.NTy.argumentWith VerexIR.Proofs.Denote.Skolems.argument
              (VerexIR.Proofs.Denote.NTy.param 0))
            VerexIR.Proofs.Denote.NRow.nil }
      (VerexIR.StorageKey.address addr) =
    some val✝
value :
  (0x42.generic_storage_invariants.unit.nominal.namespace.0.declaration✝¹⁻⁰
      0x42.generic_storage_invariants.unit.nominal.namespace.0.declaration✝³⁻⁰).carrier
a✝¹ : 0 ≤ val✝.val.fst.val + amount.val
a✝ : val✝.val.fst.val + amount.val ≤ 18446744073709551615
invariantHolds✝² invariantHolds✝¹ : True
named_eq : value.val.fst.val = val✝.val.fst.val + amount.val
invariantHolds✝ : True
⊢ ¬(initialState✝
          {
            type :=
              0x42.generic_storage_invariants.unit.nominal.namespace.0.declaration✝¹⁻⁰
                0x42.generic_storage_invariants.unit.nominal.namespace.0.declaration✝³⁻⁰,
            arguments :=
              VerexIR.Proofs.Denote.NRow.cons
                (VerexIR.Proofs.Denote.NTy.argumentWith VerexIR.Proofs.Denote.Skolems.argument
                  0x42.generic_storage_invariants.unit.nominal.namespace.0.declaration✝³⁻⁰)
                VerexIR.Proofs.Denote.NRow.nil }
          (VerexIR.StorageKey.address addr) =
        none ∨
      18446744073709551615 <
        ((initialState✝
                      {
                        type :=
                          0x42.generic_storage_invariants.unit.nominal.namespace.0.declaration✝¹⁻⁰
                            0x42.generic_storage_invariants.unit.nominal.namespace.0.declaration✝³⁻⁰,
                        arguments :=
                          VerexIR.Proofs.Denote.NRow.cons
                            (VerexIR.Proofs.Denote.NTy.argumentWith VerexIR.Proofs.Denote.Skolems.argument
                              0x42.generic_storage_invariants.unit.nominal.namespace.0.declaration✝³⁻⁰)
                            VerexIR.Proofs.Denote.NRow.nil }
                      (VerexIR.StorageKey.address addr)).getD
                  ((0x42.generic_storage_invariants.unit.nominal.namespace.0.declaration✝¹⁻⁰
                        0x42.generic_storage_invariants.unit.nominal.namespace.0.declaration✝³⁻⁰).inhabitant
                    ⋯)).val.fst.val +
          amount.val)
---
error: verex verification failed: the automatic verification of `deposit_a` failed; provide a proof in the module:
  verify deposit_a by lean =>
    verex_obligations
    verex_on
    | aborts_if_1 => verex_prepare; sorry
    | ensures_1 => verex_prepare; sorry
    | ensures_2 => verex_prepare; sorry
    | leaf => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
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
