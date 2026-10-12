-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! A stored invariant equating Tables must keep identity equality. Until
stored declaration callbacks carry the required observations, reject this
invariant instead of silently strengthening it to equality of cached lengths.
The attempted postcondition below does not follow from allocation identity. -/

set_option maxHeartbeats 25000
set_option leaner.verifyHeartbeats 25000

/-- error: the stored collection invariant of `Pair` is not carried: a physical Table observation needs a specification memory -/
#guard_msgs in
leaner module 0x42::stored_table_identity where
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
  struct Pair has Store where
    left : Table<u64, u64>
    right : Table<u64, u64>
  spec Pair where
    invariant left == right
  public fun wrong(t : &Table<u64, Pair>, k : u64) -> u64 := borrow(t, k).left.length
  spec wrong where
    requires has(t, k)
    aborts_if false
    ensures result == get(t, k).right.length
  verify wrong
