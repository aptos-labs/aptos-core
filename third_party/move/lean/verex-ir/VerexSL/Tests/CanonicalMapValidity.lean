-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

-- Map validity observes keys. Generic calls and concrete owners must keep
-- one validity predicate when the specification reads u64 values as Int.
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
