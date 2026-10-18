-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-! A table is allocated at a fresh handle and filled through its roles: the
caller reads the contents it built, and its frame leaves the allocation
history open while keeping every other resource
(`designs/intrinsic-maps.md`, "Owned Table values"). -/

verex module 0x42::table_allocation where
  pragma verify = false
  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address
  native fun host_new {K} {V}() -> Table<K, V>
  native fun host_add {K} {V}(t : &mut Table<K, V>, k : K, v : V) -> Unit
  native fun host_remove {K} {V}(t : &mut Table<K, V>, k : K) -> V
  @[map_new (Table)]
  public fun new {K} {V}() -> Table<K, V> := host_new()
  @[map_add_no_override (Table)]
  public fun add {K} {V}(t : &mut Table<K, V>, k : K, v : V) -> Unit := host_add(t, k, v)
  @[map_del_must_exist (Table)]
  public fun remove {K} {V}(t : &mut Table<K, V>, k : K) -> V := host_remove(t, k)
  @[map_spec_has_key (Table)]
  opaque spec fun has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_spec_get (Table)]
  opaque spec fun get {K} {V}(t : Table<K, V>, k : K) : V
  @[map_spec_len (Table)]
  opaque spec fun size {K} {V}(t : Table<K, V>) : Int

  public fun filled() -> Table<u8, u64> := do
    let mut t := new::<u8, u64>()
    add(&mut t, 1u8, 3u64)
    add(&mut t, 2u8, 4u64)
    t
  spec filled where
    ensures has(result, 1u8) && has(result, 2u8)
    ensures get(result, 1u8) == 3
    ensures get(result, 2u8) == 4
    ensures size(result) == 2
  verify filled

  public fun removed() -> Table<u8, u64> := do
    let mut t := filled()
    remove(&mut t, 1u8)
    t
  spec removed where
    ensures !has(result, 1u8) && has(result, 2u8)
    ensures size(result) == 1
  verify removed

  public fun duplicate(k1 : u8, k2 : u8) -> Table<u8, u64> := do
    let mut t := new::<u8, u64>()
    add(&mut t, k1, 2u64)
    add(&mut t, k2, 3u64)
    t
  spec duplicate where
    aborts_if k1 == k2
  verify duplicate

  public fun nested_values(x : u8) -> Table<u8, Vector<u8> > := do
    let mut t := new::<u8, Vector<u8> >()
    add(&mut t, 1u8, vector<u8>[x & 1])
    t
  spec nested_values where
    aborts_if false
    ensures get(result, 1u8) == vector<u8>[x & 1]
  verify nested_values

  -- A table held by a struct, keyed by a struct: the owner's twin bridges
  -- the entries through the key's.
  struct Key has Copy, Drop where
    v : Vector<u8>
  struct Holder has Store where
    t : Table<Key, u64>
  public fun held() -> Holder := do
    let mut t := new::<Key, u64>()
    add(&mut t, new Key { v := vector<u8>[1u8, 2u8] }, 22u64)
    new Holder { t := t }
  spec held where
    ensures size(result.t) == 1
    ensures get(result.t, new Key { v := vector<u8>[1u8, 2u8] }) == 22
  verify held

  public fun wrong_value() -> Table<u8, u64> := do
    let mut t := new::<u8, u64>()
    add(&mut t, 1u8, 3u64)
    t
  spec wrong_value where
    ensures get(result, 1u8) == 4

/--
error: the specification clause `ensures get(result, 1u8) == 4` is not established
---
error: verex verification failed: the automatic verification of `wrong_value` failed; provide a proof: `verify wrong_value by …` in the module (`verify wrong_value by verex_obligations` lists its obligations)
-/
#guard_msgs in
verify 0x42::table_allocation::wrong_value
