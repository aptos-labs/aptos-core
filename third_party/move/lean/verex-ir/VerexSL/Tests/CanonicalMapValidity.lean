-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

-- Map validity observes keys, so an owner's validity clause reads a value of
-- a fixed-width integer type as the integer a specification states it at: one
-- spelling of the entries for the clause's reading and for the validity. A
-- value of any other type stays in its native representation, which a generic
-- clause and an abstract parameter's sort need, and the discipline is the same
-- predicate either way. A value's own invariant is its type's and is not
-- granted by the map's validity.
verex module 0x42::canonical_map_validity where
  pragma verify = false
  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address

  native fun host_has {K} {V}(table : &Table<K, V>, key : &K) -> Bool
  @[map_has_key (Table)]
  fun has {K} {V}(table : &Table<K, V>, key : &K) -> Bool := host_has(table, key)
  spec has where
    pragma intrinsic

  @[map_spec_has_key (Table)]
  opaque spec fun spec_has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_spec_get (Table)]
  opaque spec fun spec_get {K} {V}(t : Table<K, V>, k : K) : V

  fun contains(table : &mut Table<Address, u64>, key : Address) -> Unit := do
    has(table, &key)
    ()
  spec contains where
    aborts_if false
  verify contains

  struct Holder has Store where
    table : Table<Address, u64>
  fun contains_nested(holder : &mut Holder, key : Address) -> Unit := do
    has(&holder.table, &key)
    ()
  spec contains_nested where
    aborts_if false
  verify contains_nested

  -- A concrete integer key and value: both sides of the owner's validity and
  -- the clause's reading state the entries once.
  struct Counts has Store where
    table : Table<u64, u64>
  fun integer_entries(counts : &mut Counts, key : u64) -> Unit := do
    has(&counts.table, &key)
    ()
  spec integer_entries where
    aborts_if false
  verify integer_entries

  struct Payload has Copy, Drop, Store where
    value : u64
  spec Payload where
    invariant value <= 10

  -- An integer key with a nominal value: the value keeps its native
  -- representation, and its own invariant is available where the read is in
  -- domain.
  struct Payloads has Store where
    table : Table<u64, Payload>
  fun nominal_entries(payloads : &mut Payloads, key : u64) -> Unit := do
    has(&payloads.table, &key)
    ()
  spec nominal_entries where
    aborts_if false
  verify nominal_entries

  fun nominal_value_invariant(payloads : &Payloads, key : u64) -> u64 := 10
  spec nominal_value_invariant where
    requires spec_has(payloads.table, key)
    ensures spec_get(payloads.table, key).value <= 10
  verify nominal_value_invariant

  fun nominal_value_too_strong(payloads : &Payloads, key : u64) -> u64 := 10
  spec nominal_value_too_strong where
    requires spec_has(payloads.table, key)
    ensures spec_get(payloads.table, key).value < 10 -- error: the invariant permits 10

  -- A value at a type parameter, read as its family's sort, and the same owner
  -- at an instantiated value type.
  struct Bag {V} has Store where
    table : Table<u64, V>
  fun generic_entries {V}(bag : &mut Bag<V>, key : u64) -> Unit := do
    has(&bag.table, &key)
    ()
  spec generic_entries where
    aborts_if false
  verify generic_entries

  fun instantiated_entries(bag : &mut Bag<u64>, key : u64) -> Unit := do
    has(&bag.table, &key)
    ()
  spec instantiated_entries where
    aborts_if false
  verify instantiated_entries

/--
error: the specification clause `ensures spec_get(payloads.table, key).value < 10` is not established
---
error: verex verification failed: the automatic verification of `nominal_value_too_strong` failed; provide a proof in the module:
  verify nominal_value_too_strong by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::canonical_map_validity::nominal_value_too_strong
