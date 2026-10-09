-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! Read-only Table roles are explicit intrinsic-contract assumptions. Their
contents come from the selected memory, including nested Tables and caller-side
contracts; no synthetic entry field is added to the physical Table. -/

leaner module 0x42::table_reads where
  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address

  native fun host_borrow {K} {V}(t : &Table<K, V>, k : K) -> &V
  native fun host_contains {K} {V}(t : &Table<K, V>, k : K) -> Bool

  @[map_borrow (Table)]
  public fun borrow {K} {V}(t : &Table<K, V>, k : K) -> &V := host_borrow(t, k)
  @[map_has_key (Table)]
  public fun contains {K} {V}(t : &Table<K, V>, k : K) -> Bool := host_contains(t, k)

  @[map_spec_has_key (Table)]
  opaque spec fun has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_spec_get (Table)]
  opaque spec fun get {K} {V}(t : Table<K, V>, k : K) : V

  public fun membership(t : &Table<u64, u64>, k : u64) -> Bool := contains(t, k)
  spec membership where
    aborts_if false
    ensures result == has(t, k)
  verify membership

  public fun lookup(t : &Table<u64, u64>, k : u64) -> u64 := *borrow(t, k)
  spec lookup where
    pragma opaque
    aborts_if !has(t, k)
    ensures result == get(t, k)
  verify lookup

  public fun caller(t : &Table<u64, u64>, k : u64) -> u64 := lookup(t, k)
  spec caller where
    requires has(t, k)
    aborts_if false
    ensures result == old(get(t, k))
  verify caller

  public fun lookup_generic {K has Copy, Drop} {V has Copy, Drop}
      (t : &Table<K, V>, k : K) -> V := *borrow(t, k)
  spec lookup_generic where
    aborts_if !has(t, k)
    ensures result == get(t, k)
  verify lookup_generic

  public fun nested(t : &Table<u64, Table<u64, u64> >, outer : u64, inner : u64) -> u64 :=
    *borrow(borrow(t, outer), inner)
  spec nested where
    requires has(t, outer)
    requires has(get(t, outer), inner)
    aborts_if false
    ensures result == get(get(t, outer), inner)
  verify nested

  struct Transaction has Copy, Drop, Store where
    payload : Vector<Vector<u8> >
    sequence : u64

  -- A borrowed aggregate and the specification's snapshot must project to
  -- the same value, without assuming any invariant about its vector length.
  public fun payload(t : &Table<u64, Transaction>, k : u64) -> Vector<Vector<u8> > :=
    borrow(t, k).payload
  spec payload where
    aborts_if !has(t, k)
    ensures result == get(t, k).payload
    ensures result.length == get(t, k).payload.length
  verify payload

  public fun wrong_payload_bound(t : &Table<u64, Transaction>, k : u64) -> Vector<Vector<u8> > :=
    borrow(t, k).payload
  spec wrong_payload_bound where
    pragma verify = false
    requires has(t, k)
    aborts_if false
    ensures result.length <= 1

  public fun wrong_lookup(t : &Table<u64, u64>, k : u64) -> u64 := *borrow(t, k)
  spec wrong_lookup where
    pragma verify = false
    requires has(t, k)
    aborts_if false
    ensures result != get(t, k)

/--
error: the specification clause `ensures result != get(t, k)` is not established
---
error: leaner verification failed: the automatic verification of `wrong_lookup` failed; provide a proof: `verify wrong_lookup by …` in the module (`verify wrong_lookup by skip` shows the obligations it leaves)
-/
#guard_msgs in
verify 0x42::table_reads::wrong_lookup

/--
error: the specification clause `ensures result.length <= 1` is not established
---
error: leaner verification failed: the automatic verification of `wrong_payload_bound` failed; provide a proof: `verify wrong_payload_bound by …` in the module (`verify wrong_payload_bound by skip` shows the obligations it leaves)
-/
#guard_msgs in
verify 0x42::table_reads::wrong_payload_bound
