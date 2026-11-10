-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
import VerexSL
namespace VerexSL.Tests.GenericLiteralRanges
-- Generic insertion constructs entries in the callee's family. The concrete
-- callers preserve sortedness, whose only nonvacuous pair is indices 0 and 1.
-- The unconstrained helper is deliberately not verified for arbitrary maps.
verex module 0x42::generic_literal_ranges where
  pragma verify = false
  struct Entry {K has Copy, Drop, Store} {V has Copy, Drop, Store} has Copy, Drop, Store where
    key : K
    value : V
  struct Map {K has Copy, Drop, Store} {V has Copy, Drop, Store} has Copy, Drop, Store where
    entries : Vector<Entry<K, V> >
  spec Map where
    invariant sorted_keys(entries)
  spec fun sorted_keys {K has Copy, Drop, Store} {V has Copy, Drop, Store}(entries : Vector<Entry<K,V> >) : Bool :=
    forall (i : Int), forall (j : Int), 0 <= i && i < j && j < entries.length ==>
      core.prim.compare(entries[i].key, entries[j].key) < 0
  fun empty {K has Copy, Drop, Store} {V has Copy, Drop, Store}() -> Map<K, V> :=
    new Map<K, V> { entries := vector<Entry<K, V> >[] }
  fun add {K has Copy, Drop, Store} {V has Copy, Drop, Store}(map : &mut Map<K,V>, key : K, value : V) -> Unit := do
    let entries := &mut map.entries
    *entries := core.prim.insertVector(*entries, 0, new Entry<K,V> { key, value })
  fun direct_read() -> u64 := do
    let mut map := empty::<Bool,u64>()
    add::<Bool,u64>(&mut map, true, 10)
    map.entries[0].value
  spec direct_read where
    ensures result == 10
    aborts_if false
  verify direct_read

  fun two_read() -> u64 := do
    let mut map := empty::<Bool,u64>()
    add::<Bool,u64>(&mut map, true, 10)
    add::<Bool,u64>(&mut map, false, 20)
    map.entries[1].value
  spec two_read where
    ensures result == 10
    aborts_if false
  verify two_read

  -- The interior insertion needs all pairs of positions of a three-entry map.
  fun add_at {K has Copy, Drop, Store} {V has Copy, Drop, Store}
      (map : &mut Map<K,V>, key : K, value : V, index : u64) -> Unit := do
    let entries := &mut map.entries
    *entries := core.prim.insertVector(*entries, index, new Entry<K,V> { key, value })

  fun three_read() -> u64 := do
    let mut map := empty::<u64,u64>()
    add_at::<u64,u64>(&mut map, 30, 300, 0)
    add_at::<u64,u64>(&mut map, 10, 100, 0)
    add_at::<u64,u64>(&mut map, 20, 200, 1)
    map.entries[1].value
  spec three_read where
    ensures result == 200
    aborts_if false
  verify three_read

end VerexSL.Tests.GenericLiteralRanges
