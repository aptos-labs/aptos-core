-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! Insertion-position roles expose sequence order, even when keys descend.
The returned map's implicit validity invariant also checks that these roles
have not accidentally selected the sorted-map discipline. -/
namespace LeanerLang.Tests.Check.Specifications.InsertionMap

leaner module 0x42::insertion_map where
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

  public fun descending() -> Map<u64, u64> :=
    new Map<u64, u64> { entries := vector<Entry<u64, u64> >[
      new Entry<u64, u64> { key := 20, value := 2 },
      new Entry<u64, u64> { key := 10, value := 1 }] }
  spec descending where
    aborts_if false
    ensures key_at(result, 0) == 20
    ensures key_at(result, 1) == 10
    ensures rank(result, 20) == 0
    ensures rank(result, 10) == 1

  verify descending by
    all_goals simp [LeanerIR.Maps.Valid, LeanerIR.Maps.Discipline.Valid,
      LeanerIR.Maps.Distinct, LeanerIR.Maps.entriesOf, LeanerIR.RuntimeValue.field]

end LeanerLang.Tests.Check.Specifications.InsertionMap
