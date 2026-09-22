-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! The core of an ordered map: entries kept sorted by key, binary search. -/
leaner module 0x42::ordered_map where
  use 0x1::std::vector

  struct Entry has Copy, Drop, Store where
    key : u64
    value : u64

  struct Map has Copy, Drop, Store where
    entries : Vector<Entry>

  const E_PRESENT : u64 := 1

  const E_ABSENT : u64 := 2

  public fun empty() -> Map := new Map { entries := vector<Entry>[] }

  public fun length(map : &Map) -> u64 := map.entries.length

  spec length where
    ensures result == map.entries.length
    aborts_if false

  /--
  The first index whose key is not below `key`.
  -/
  fun lower_bound(map : &Map, key : u64) -> u64 := do
    let low := 0
    let high := map.entries.length
    while low < high do
      let mid := low + (high - low) / 2
      if map.entries[mid].key < key then low := mid + 1 else high := mid
    return low

  public fun contains(map : &Map, key : u64) -> Bool := do
    let index := lower_bound(map, key)
    return map.entries.length > index && map.entries[index].key == key

  public fun get(map : &Map, key : u64) -> u64 := do
    let index := lower_bound(map, key)
    assert!(map.entries.length > index, E_ABSENT)
    let «entry» := &map.entries[index]
    assert!(«entry».key == key, E_ABSENT)
    return «entry».value

  public fun add(map : &mut Map, key : u64, value : u64) -> Unit := do
    let index := lower_bound(map, key)
    if map.entries.length > index then
      assert!(map.entries[index].key != key, E_PRESENT)
    map.entries := core.prim.pushVector(map.entries, new Entry { key, value })
    let i := map.entries.length - 1
    while i > index do
      map.entries.swap(i, i - 1)
      i := i - 1

  public fun remove(map : &mut Map, key : u64) -> u64 := do
    let index := lower_bound(map, key)
    assert!(map.entries.length > index, E_ABSENT)
    let «entry» := map.entries.remove(index)
    assert!(«entry».key == key, E_ABSENT)
    return «entry».value
