-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport
/-! A table held by a stored resource is written in place: adding an entry
or updating one through a mutable reference writes the resource, which owes
the deep data invariants of its entries. A cached length counts the entries
added to a fresh table. -/

verex module 0x42::table_global_writes where
  pragma verify = false
  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address
    length : u64
  native fun host_new {K} {V}() -> Table<K, V>
  native fun host_add {K} {V}(t : &mut Table<K, V>, k : K, v : V) -> Unit
  native fun host_borrow {K} {V}(t : &Table<K, V>, k : K) -> &V
  native fun host_borrow_mut {K} {V}(t : &mut Table<K, V>, k : K) -> &mut V
  @[map_new (Table)]
  public fun new {K} {V}() -> Table<K, V> := host_new()
  @[map_add_no_override (Table)]
  public fun add {K} {V}(t : &mut Table<K, V>, k : K, v : V) -> Unit := host_add(t, k, v)
  @[map_borrow (Table)]
  public fun borrow {K} {V}(t : &Table<K, V>, k : K) -> &V := host_borrow(t, k)
  @[map_borrow_mut (Table)]
  public fun borrow_mut {K} {V}(t : &mut Table<K, V>, k : K) -> &mut V := host_borrow_mut(t, k)
  @[map_spec_has_key (Table)]
  opaque spec fun has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_spec_get (Table)]
  opaque spec fun get {K} {V}(t : Table<K, V>, k : K) : V
  @[map_spec_len (Table)]
  opaque spec fun size {K} {V}(t : Table<K, V>) : Int

  public fun three() -> Table<u8, u64> := do
    let mut t := new::<u8, u64>()
    add(&mut t, 1u8, 2u64)
    add(&mut t, 2u8, 3u64)
    add(&mut t, 3u8, 4u64)
    t
  spec three where
    ensures size(result) == 3
    ensures get(result, 1u8) == 2
  verify three by lean =>
    verex_obligations
    all_goals verex_prepare only
    all_goals verex_decide

  struct S has Store where
    value : u64
  spec S where
    invariant value != 0
  struct R has Key where
    map : Table<Address, S>

  public fun insert(a : Address) -> Unit := do
    add(&mut R[@0x42].map, a, new S { value := 1u64 })
  verify insert

  public fun bump(a : Address) -> Unit := do
    let s := borrow_mut(&mut R[@0x42].map, a)
    s.value := 5u64
  verify bump

  public fun zero(a : Address) -> Unit := do
    let s := borrow_mut(&mut R[@0x42].map, a)
    s.value := 0u64

  struct Holder has Key where
    s : S
  public fun plain_one() -> Unit := do
    Holder[@0x42].s.value := 1u64
  verify plain_one

  public fun plain_zero() -> Unit := do
    Holder[@0x42].s.value := 0u64

/--
error: the data invariant `invariant value != 0` does not hold for a value written to memory
---
error: verex verification failed: the automatic verification of `zero` failed; provide a proof in the module:
  verify zero by lean =>
    verex_obligations
    verex_on
    | stored_invariant => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::table_global_writes::zero

/--
error: the data invariant `invariant value != 0` does not hold for a value written to memory
---
error: verex verification failed: the automatic verification of `plain_zero` failed; provide a proof in the module:
  verify plain_zero by lean =>
    verex_obligations
    verex_on
    | stored_invariant => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::table_global_writes::plain_zero
