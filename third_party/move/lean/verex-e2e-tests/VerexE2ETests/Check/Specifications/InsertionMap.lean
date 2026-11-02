-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-! Insertion-position roles expose sequence order, even when keys descend:
a map built by its roles holds its entries in insertion order. The returned
map's implicit validity invariant also checks that these roles have not
accidentally selected the sorted-map discipline. -/
namespace VerexSL.Tests.Check.Specifications.InsertionMap

verex module 0x42::insertion_map where
  struct Entry {K} {V} has Copy, Drop, Store where
    key : K
    value : V

  @[intrinsic_map]
  struct Map {K} {V} has Copy, Drop, Store where
    entries : Vector<Entry<K, V> >

  @[map_spec_get (Map)]
  opaque spec fun get {K} {V}(m : Map<K, V>, k : K) : V
  @[map_spec_set (Map)]
  opaque spec fun set {K} {V}(m : Map<K, V>, k : K, v : V) : Map<K, V>
  @[map_spec_del (Map)]
  opaque spec fun del {K} {V}(m : Map<K, V>, k : K) : Map<K, V>
  @[map_spec_has_key (Map)]
  opaque spec fun has {K} {V}(m : Map<K, V>, k : K) : Bool
  @[map_spec_insertion_key_at (Map)]
  opaque spec fun key_at {K} {V}(m : Map<K, V>, i : Int) : K
  @[map_spec_insertion_rank (Map)]
  opaque spec fun rank {K} {V}(m : Map<K, V>, k : K) : Int

  native fun host_add {K} {V}(m : &mut Map<K, V>, k : K, v : V) -> Unit
  @[map_new (Map)]
  public fun new {K} {V}() -> Map<K, V> := new Map<K, V> { entries := vector<Entry<K, V> >[] }
  @[map_add_no_override (Map)]
  public fun add {K} {V}(m : &mut Map<K, V>, k : K, v : V) -> Unit := host_add(m, k, v)

  public fun descending() -> Map<u64, u64> := do
    let mut m := new::<u64, u64>()
    add(&mut m, 20u64, 2u64)
    add(&mut m, 10u64, 1u64)
    m
  spec descending where
    aborts_if false
    ensures key_at(result, 0) == 20
    ensures key_at(result, 1) == 10
    ensures rank(result, 20) == 0
    ensures rank(result, 10) == 1
  verify descending

end VerexSL.Tests.Check.Specifications.InsertionMap
