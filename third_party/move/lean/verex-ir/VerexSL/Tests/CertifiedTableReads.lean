-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! A specification-only table lookup exposes the stored value's or key's declared
invariant when guarded in domain (`has(t, k)` / `0 < size(t)`), and refuses
off-domain or overly strong claims (`key_read_empty`, `too_strong`, `wrong_type`). -/

verex module 0x42::certified_table_reads where
  pragma verify = false

  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address

  @[map_spec_has_key (Table)]
  opaque spec fun has {K} {V}(t : Table<K, V>, k : K) : Bool

  @[map_spec_get (Table)]
  opaque spec fun get {K} {V}(t : Table<K, V>, k : K) : V

  @[map_spec_len (Table)]
  opaque spec fun size {K} {V}(t : Table<K, V>) : Int

  @[map_spec_insertion_key_at (Table)]
  opaque spec fun key_at {K} {V}(t : Table<K, V>, index : Int) : K

  struct Payload has Copy, Drop, Store where
    value : u64

  spec Payload where
    invariant value <= 10

  public fun direct(t : &Table<u64, Payload>, k : u64) -> u64 := 10
  spec direct where
    requires has(t, k)
    ensures get(t, k).value <= 10

  public fun key_read(t : &Table<Payload, u64>) -> u64 := 10
  spec key_read where
    requires 0 < size(t)
    ensures key_at(t, 0).value <= 10

  public fun key_read_empty(t : &Table<Payload, u64>) -> u64 := 10
  spec key_read_empty where
    ensures key_at(t, 0).value <= 10

  opaque spec fun permitted {T}(value : u64) : Bool

  struct Tagged {T} has Copy, Drop, Store where
    value : u64

  spec Tagged where
    invariant permitted::<T>(value)

  public fun concrete(t : &Table<u64, Tagged<u8> >, k : u64) -> u64 := 10
  spec concrete where
    requires has(t, k)
    ensures permitted::<u8>(get(t, k).value)

  public fun wrong_type(t : &Table<u64, Tagged<u8> >, k : u64) -> u64 := 10
  spec wrong_type where
    requires has(t, k)
    ensures permitted::<Bool>(get(t, k).value)

  public fun too_strong(t : &Table<u64, Payload>, k : u64) -> u64 := 10
  spec too_strong where
    requires has(t, k)
    ensures get(t, k).value < 10

/--
error: the specification clause `ensures key_at(t, 0).value <= 10` is not established
---
error: verex verification failed: the automatic verification of `key_read_empty` failed; provide a proof in the module:
  verify key_read_empty by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::certified_table_reads::key_read_empty

/--
error: the specification clause `ensures get(t, k).value < 10` is not established
---
error: verex verification failed: the automatic verification of `too_strong` failed; provide a proof in the module:
  verify too_strong by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::certified_table_reads::too_strong

/--
error: the specification clause `ensures permitted::<Bool>(get(t, k).value)` is not established
---
error: verex verification failed: the automatic verification of `wrong_type` failed; provide a proof in the module:
  verify wrong_type by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::certified_table_reads::wrong_type

verify 0x42::certified_table_reads::direct
verify 0x42::certified_table_reads::key_read
verify 0x42::certified_table_reads::concrete
