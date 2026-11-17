-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

-- A table written back across a branch is read back by the write-back law,
-- whose entry round trip holds for every reader pair: a key or value read as
-- an integer reads back where the entry's value fits, and a key or value held
-- as it is reads back by reflexivity. The branch makes the intermediate table
-- a written-back value that the key test and the clause both read.
verex module 0x42::written_back_round_trip where
  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address
  native fun host_new {K} {V}() -> Table<K, V>
  native fun host_add {K} {V}(t : &mut Table<K, V>, k : K, v : V) -> Unit
  @[map_new (Table)]
  public fun new {K} {V}() -> Table<K, V> := host_new()
  @[map_add_no_override (Table)]
  public fun add {K} {V}(t : &mut Table<K, V>, k : K, v : V) -> Unit := host_add(t, k, v)
  @[map_spec_has_key (Table)]
  opaque spec fun spec_has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_spec_get (Table)]
  opaque spec fun spec_get {K} {V}(t : Table<K, V>, k : K) : V
  @[map_spec_len (Table)]
  opaque spec fun spec_len {K} {V}(t : Table<K, V>) : Int

  public fun vector_keys(k1 : Vector<u8>, k2 : Vector<u8>) -> Table<Vector<u8>, u64> := do
    let mut t := new::<Vector<u8>, u64>()
    add(&mut t, k1, 1u64)
    add(&mut t, k2, 2u64)
    t
  spec vector_keys where
    requires k1 != k2
    aborts_if false
    ensures spec_has(result, k1) && spec_has(result, k2)
  verify vector_keys

  public fun integer_keys(k1 : u8, k2 : u8) -> Table<u8, u64> := do
    let mut t := new::<u8, u64>()
    add(&mut t, k1, 1u64)
    add(&mut t, k2, 2u64)
    t
  spec integer_keys where
    requires k1 != k2
    aborts_if false
    ensures spec_has(result, k1) && spec_has(result, k2)
  verify integer_keys

  -- The fixture's own shape: the second update is conditional, so the map
  -- after the first update is materialised as a table and read back.
  public fun vector_keys_branch(k1 : Vector<u8>, k2 : Vector<u8>) -> Table<Vector<u8>, u64> := do
    let mut t := new::<Vector<u8>, u64>()
    add(&mut t, k1, 1u64)
    if k1 != k2 then
      add(&mut t, k2, 2u64)
    t
  spec vector_keys_branch where
    aborts_if false
    ensures spec_has(result, k1) && spec_has(result, k2)
  verify vector_keys_branch

  public fun integer_keys_branch(k1 : u8, k2 : u8) -> Table<u8, u64> := do
    let mut t := new::<u8, u64>()
    add(&mut t, k1, 1u64)
    if k1 != k2 then
      add(&mut t, k2, 2u64)
    t
  spec integer_keys_branch where
    aborts_if false
    ensures spec_has(result, k1) && spec_has(result, k2)
  verify integer_keys_branch
