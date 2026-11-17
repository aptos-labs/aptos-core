-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport
import VerexSL.Tests.Shapes.Support

/-!
# The table (intrinsic map) operations, at each value shape

A read, an insert, a remove and a mutable borrow of an intrinsic map, over a
scalar value, an invariant-bearing struct, a generic struct at an instance,
a table held by a resource, and at the caller's own type parameters. A
positive states the fact the operation gives: the map's specification
functions, or the stored value's own data invariant; the negative asks for
more, or stores a value the invariant refuses.
-/

namespace VerexSL.Tests.Shapes.Tables

set_option Elab.async false

verex module 0x42::shapes_tables where
  pragma verify = false

  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address

  native fun host_new {K} {V}() -> Table<K, V>
  native fun host_add {K} {V}(t : &mut Table<K, V>, k : K, v : V) -> Unit
  native fun host_remove {K} {V}(t : &mut Table<K, V>, k : K) -> V
  native fun host_borrow {K} {V}(t : &Table<K, V>, k : K) -> &V
  native fun host_borrow_mut {K} {V}(t : &mut Table<K, V>, k : K) -> &mut V
  native fun host_contains {K} {V}(t : &Table<K, V>, k : K) -> Bool

  @[map_new (Table)]
  public fun new {K} {V}() -> Table<K, V> := host_new()
  @[map_add_no_override (Table)]
  public fun add {K} {V}(t : &mut Table<K, V>, k : K, v : V) -> Unit := host_add(t, k, v)
  @[map_del_must_exist (Table)]
  public fun remove {K} {V}(t : &mut Table<K, V>, k : K) -> V := host_remove(t, k)
  @[map_borrow (Table)]
  public fun borrow {K} {V}(t : &Table<K, V>, k : K) -> &V := host_borrow(t, k)
  @[map_borrow_mut (Table)]
  public fun borrow_mut {K} {V}(t : &mut Table<K, V>, k : K) -> &mut V := host_borrow_mut(t, k)
  @[map_has_key (Table)]
  public fun contains {K} {V}(t : &Table<K, V>, k : K) -> Bool := host_contains(t, k)

  @[map_spec_has_key (Table)]
  opaque spec fun has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_spec_get (Table)]
  opaque spec fun get {K} {V}(t : Table<K, V>, k : K) : V
  @[map_spec_len (Table)]
  opaque spec fun size {K} {V}(t : Table<K, V>) : Int

  struct Percent has Copy, Drop, Store where
    value : u64
  spec Percent where
    invariant value <= 100

  struct Box {T has Copy, Drop} has Copy, Drop, Store where
    item : T

  struct Registry has Key where
    slots : Table<Address, Percent>

  -- 7.7 a read at an instance of scalars
  fun read_scalar_bad(t : &Table<u64, u64>, k : u64) -> u64 := *borrow(t, k)
  spec read_scalar_bad where
    aborts_if !has(t, k)
    ensures result == get(t, k) + 1

  fun read_scalar_ok(t : &Table<u64, u64>, k : u64) -> u64 := *borrow(t, k)
  spec read_scalar_ok where
    aborts_if !has(t, k)
    ensures result == get(t, k)

  -- 7.8 a read of an invariant-bearing stored value
  fun read_struct_bad(t : &Table<u64, Percent>, k : u64) -> u64 := borrow(t, k).value
  spec read_struct_bad where
    aborts_if !has(t, k)
    ensures result <= 99

  fun read_struct_ok(t : &Table<u64, Percent>, k : u64) -> u64 := borrow(t, k).value
  spec read_struct_ok where
    aborts_if !has(t, k)
    ensures result <= 100

  -- 7.9 a read at a generic struct instance, and at the caller's own parameters
  fun read_instance_bad(t : &Table<u64, Box<Percent> >, k : u64) -> u64 :=
    borrow(t, k).item.value
  spec read_instance_bad where
    aborts_if !has(t, k)
    ensures result <= 99

  fun read_instance_ok(t : &Table<u64, Box<Percent> >, k : u64) -> u64 :=
    borrow(t, k).item.value
  spec read_instance_ok where
    aborts_if !has(t, k)
    ensures result <= 100

  fun read_param_bad {K has Copy, Drop} {V has Copy, Drop}(t : &Table<K, V>, k : K) -> V :=
    *borrow(t, k)
  spec read_param_bad where
    aborts_if !has(t, k)
    ensures !has(t, k)

  fun read_param_ok {K has Copy, Drop} {V has Copy, Drop}(t : &Table<K, V>, k : K) -> V :=
    *borrow(t, k)
  spec read_param_ok where
    aborts_if !has(t, k)
    ensures result == get(t, k)

  -- 7.10 insert
  fun insert_bad(t : &mut Table<u64, Percent>, k : u64) -> Unit :=
    add(t, k, new Percent { value := 101 })
  spec insert_bad where
    requires !has(t, k)
    aborts_if false

  fun insert_ok(t : &mut Table<u64, Percent>, k : u64, amount : u64) -> Unit :=
    add(t, k, new Percent { value := amount })
  spec insert_ok where
    requires !has(t, k) && amount <= 100
    aborts_if false
    ensures has(t, k)

  -- 7.11 remove: the taken value carries its invariant
  fun remove_bad(t : &mut Table<u64, Percent>, k : u64) -> u64 := remove(t, k).value
  spec remove_bad where
    aborts_if !has(t, k)
    ensures result <= 99

  fun remove_ok(t : &mut Table<u64, Percent>, k : u64) -> u64 := remove(t, k).value
  spec remove_ok where
    aborts_if !has(t, k)
    ensures result <= 100
    ensures !has(t, k)

  -- 7.12 a mutable borrow and a write back
  fun borrow_mut_bad(t : &mut Table<u64, Percent>, k : u64) -> Unit := do
    let slot := borrow_mut(t, k)
    slot.value := 101
  spec borrow_mut_bad where
    requires has(t, k)
    aborts_if false

  fun borrow_mut_ok(t : &mut Table<u64, Percent>, k : u64, amount : u64) -> Unit := do
    let slot := borrow_mut(t, k)
    slot.value := amount
  spec borrow_mut_ok where
    requires has(t, k) && amount <= 100
    aborts_if false
    ensures get(t, k).value == amount

  -- 7.13 presence
  fun contains_bad(t : &Table<u64, Percent>, k : u64) -> Bool := contains(t, k)
  spec contains_bad where
    aborts_if false
    ensures result == !has(t, k)

  fun contains_ok(t : &Table<u64, Percent>, k : u64) -> Bool := contains(t, k)
  spec contains_ok where
    aborts_if false
    ensures result == has(t, k)

  -- 7.14 a table held by a resource
  fun stored_read_bad(addr : Address, k : Address) -> u64 :=
    borrow(&Registry[addr].slots, k).value
  spec stored_read_bad where
    requires exists<Registry>(addr)
    aborts_if !has(global<Registry>(addr).slots, k)
    ensures result <= 99

  fun stored_read_ok(addr : Address, k : Address) -> u64 :=
    borrow(&Registry[addr].slots, k).value
  spec stored_read_ok where
    requires exists<Registry>(addr)
    aborts_if !has(global<Registry>(addr).slots, k)
    ensures result <= 100

  fun stored_write_bad(addr : Address, k : Address) -> Unit := do
    let slot := borrow_mut(&mut Registry[addr].slots, k)
    slot.value := 101
  spec stored_write_bad where
    requires exists<Registry>(addr) && has(global<Registry>(addr).slots, k)
    modifies global<Registry>(addr)
    aborts_if false

  fun stored_write_ok(addr : Address, k : Address, amount : u64) -> Unit := do
    let slot := borrow_mut(&mut Registry[addr].slots, k)
    slot.value := amount
  spec stored_write_ok where
    requires exists<Registry>(addr) && has(global<Registry>(addr).slots, k)
    requires amount <= 100
    modifies global<Registry>(addr)
    aborts_if false
    ensures get(global<Registry>(addr).slots, k).value == amount

/-! ## Negative controls -/

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_tables.read_scalar_bad
    ["ensures result == get(t, k) + 1"]
    (← `(command| verify 0x42::shapes_tables::read_scalar_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_tables.read_struct_bad ["ensures result <= 99"]
    (← `(command| verify 0x42::shapes_tables::read_struct_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_tables.read_instance_bad ["ensures result <= 99"]
    (← `(command| verify 0x42::shapes_tables::read_instance_bad))

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  expectCuts `«0x42».shapes_tables.insert_bad [(percent, construction)]
    (← `(command| verify 0x42::shapes_tables::insert_bad)) (some [])

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_tables.remove_bad ["ensures result <= 99"]
    (← `(command| verify 0x42::shapes_tables::remove_bad))

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  expectCuts `«0x42».shapes_tables.borrow_mut_bad [(percent, mutationEnd)]
    (← `(command| verify 0x42::shapes_tables::borrow_mut_bad)) (some [])

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_tables.contains_bad ["ensures result == !has(t, k)"]
    (← `(command| verify 0x42::shapes_tables::contains_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_tables.stored_read_bad ["ensures result <= 99"]
    (← `(command| verify 0x42::shapes_tables::stored_read_bad))

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  expectCuts `«0x42».shapes_tables.stored_write_bad [(percent, mutationEnd)]
    (← `(command| verify 0x42::shapes_tables::stored_write_bad)) (some [])

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_tables.read_param_bad ["ensures !has(t, k)"]
    (← `(command| verify 0x42::shapes_tables::read_param_bad))

/-! ## Positive controls -/

verify 0x42::shapes_tables::read_scalar_ok
verify 0x42::shapes_tables::read_struct_ok
verify 0x42::shapes_tables::read_instance_ok
verify 0x42::shapes_tables::insert_ok
verify 0x42::shapes_tables::remove_ok
verify 0x42::shapes_tables::contains_ok
verify 0x42::shapes_tables::stored_read_ok
verify 0x42::shapes_tables::read_param_ok

-- Open cells
--
-- `borrow_mut_ok`, `stored_write_ok`: after a write through a mutable map
-- borrow, of a table in hand and of a table held by a resource, the
-- specification's read of the same key does not meet the written value.

run_cmd do
  expectOpen `«0x42».shapes_tables.borrow_mut_ok
    "the specification clause `ensures get(t, k).value == amount` is not established"
    (← `(command| verify 0x42::shapes_tables::borrow_mut_ok))

run_cmd do
  expectOpen `«0x42».shapes_tables.stored_write_ok
    "the specification clause `ensures get(global<Registry>(addr).slots, k).value == amount` is not established"
    (← `(command| verify 0x42::shapes_tables::stored_write_ok))

end VerexSL.Tests.Shapes.Tables
