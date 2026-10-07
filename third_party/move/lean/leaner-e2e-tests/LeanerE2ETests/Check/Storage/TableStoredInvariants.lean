-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport
/-! External Table entries carry deep data invariants at their resolved type
arguments. Computed keys, nested Tables, generic and phantom parameters, and
specification-only reads all use the same memory-bound lookup rule. -/

set_option maxHeartbeats 25000
set_option leaner.verifyHeartbeats 25000
leaner module 0x42::table_stored_invariants where
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
  struct Payload has Copy, Drop, Store where
    value : u64
  spec Payload where
    invariant value <= 10
  public fun read(t : &Table<u64, Payload>, k : u64) -> u64 := borrow(t, k).value
  spec read where
    requires has(t, k)
    aborts_if false
    ensures result <= 10
  verify read

  struct Cell {T} has Copy, Drop, Store where
    entries : Vector<T>
  spec Cell where
    invariant entries.length <= 1
  public fun count {T has Copy, Drop, Store}(t : &Table<u64, Cell<T> >, k : u64) -> u64 :=
    borrow(t, k).entries.length
  spec count where
    requires has(t, k)
    aborts_if false
    ensures result <= 1
  verify count

  public fun nested(t : &Table<u64, Table<u64, Payload> >, a : u64, b : u64) -> u64 :=
    borrow(borrow(t, a), b).value
  spec nested where
    requires has(t, a)
    requires has(get(t, a), b)
    aborts_if false
    ensures result <= 10
  verify nested

  public fun direct(t : &Table<u64, Payload>, k : u64) -> u64 := 10
  spec direct where
    requires has(t, k)
    ensures get(t, k).value <= result
  verify direct

  opaque spec fun permitted {T}(value : u64) : Bool
  struct Tagged {T} has Copy, Drop, Store where
    value : u64
  spec Tagged where
    invariant permitted::<T>(value)
  public fun tagged {T}(t : &Table<u64, Tagged<T> >, k : u64) -> u64 := borrow(t, k).value
  spec tagged where
    requires has(t, k)
    aborts_if false
    ensures permitted::<T>(result)
  verify tagged

  public fun computed(t : &Table<u64, Payload>, k : u64) -> u64 := borrow(t, k + 1).value
  spec computed where
    requires k < 18446744073709551615
    requires has(t, k + 1)
    aborts_if false
    ensures result <= 10
  verify computed

  public fun too_strong(t : &Table<u64, Payload>, k : u64) -> u64 := borrow(t, k).value
  spec too_strong where
    requires has(t, k)
    aborts_if false
    ensures result < 10

  public fun wrong_type {T}(t : &Table<u64, Tagged<T> >, k : u64) -> u64 := borrow(t, k).value
  spec wrong_type where
    requires has(t, k)
    aborts_if false
    ensures permitted::<Bool>(result)

  public fun tagged_u8(t : &Table<u64, Tagged<u8> >, k : u64) -> u64 := borrow(t, k).value
  spec tagged_u8 where
    requires has(t, k)
    aborts_if false
    ensures permitted::<u8>(result)
  verify tagged_u8

/--
error: the specification clause `ensures result < 10` is not established
---
error: leaner verification failed: the automatic verification of `too_strong` failed; provide a proof: `verify too_strong by …` in the module (`verify too_strong by skip` shows the obligations it leaves)
-/
#guard_msgs in
verify 0x42::table_stored_invariants::too_strong

/--
error: the specification clause `ensures permitted::<Bool>(result)` is not established
---
error: leaner verification failed: the automatic verification of `wrong_type` failed; provide a proof: `verify wrong_type by …` in the module (`verify wrong_type by skip` shows the obligations it leaves)
-/
#guard_msgs in
verify 0x42::table_stored_invariants::wrong_type

leaner module 0x42::table_stored_cached_length where
  pragma verify = false
  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address
    length : u64
  native fun host_borrow {K} {V}(t : &Table<K, V>, k : K) -> &V
  @[map_borrow (Table)]
  public fun borrow {K} {V}(t : &Table<K, V>, k : K) -> &V := host_borrow(t, k)
  @[map_spec_has_key (Table)]
  opaque spec fun has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_spec_get (Table)]
  opaque spec fun get {K} {V}(t : Table<K, V>, k : K) : V
  struct Payload has Copy, Drop, Store where
    value : u64
  spec Payload where
    invariant value <= 10
  public fun read(t : &Table<u64, Payload>, k : u64) -> u64 := borrow(t, k).value
  spec read where
    requires has(t, k)
    aborts_if false
    ensures result <= 10
  verify read

  struct Cell {T} has Copy, Drop, Store where
    entries : Vector<T>
  spec Cell where
    invariant entries.length <= 1
  public fun count {T has Copy, Drop, Store}(t : &Table<u64, Cell<T> >, k : u64) -> u64 :=
    borrow(t, k).entries.length
  spec count where
    requires has(t, k)
    aborts_if false
    ensures result <= 1
  verify count

