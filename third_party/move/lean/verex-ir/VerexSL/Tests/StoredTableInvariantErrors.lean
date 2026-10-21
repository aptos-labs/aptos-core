-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! A stored invariant equating Tables compares their allocation identity: it
does not equate their contents or cached lengths. -/

set_option maxHeartbeats 25000
set_option verex.verifyHeartbeats 25000

verex module 0x42::stored_table_identity where
  pragma verify = false
  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address
    length : u64
  @[map_spec_has_key (Table)]
  opaque spec fun has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_spec_get (Table)]
  opaque spec fun get {K} {V}(t : Table<K, V>, k : K) : V
  @[map_spec_len (Table)]
  opaque spec fun size {K} {V}(t : Table<K, V>) : Int
  struct Pair has Store where
    left : Table<u64, u64>
    right : Table<u64, u64>
  spec Pair where
    invariant left == right
  public fun same_handle(p : &Pair) -> Bool := p.left.handle == p.right.handle
  spec same_handle where
    aborts_if false
    ensures result
  verify same_handle
  public fun wrong(p : &Pair) -> u64 := p.left.length
  spec wrong where
    aborts_if false
    ensures result == p.right.length

/--
error: the specification clause `ensures result == p.right.length` is not established
---
error: verex verification failed: the automatic verification of `wrong` failed; provide a proof in the module:
  verify wrong by
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::stored_table_identity::wrong
