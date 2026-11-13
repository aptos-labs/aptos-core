-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-! The runtime provides a table's entry roles on its value: a mutable borrow
of the entry an iterator names, by key or by position, and a conversion to a
table of another owner (`doc/designs/intrinsic-maps.md`, "Tables in the runtime
semantics"). -/

verex module 0x42::keyed_cursors where
  pragma verify = false
  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address
  enum IteratorPtr {K} has Copy, Drop where
    | End
    | Some (key : K)
  native fun host_new {K} {V}() -> Table<K, V>
  native fun host_add {K} {V}(t : &mut Table<K, V>, k : K, v : V) -> Unit
  native fun host_borrow {K} {V}(t : &Table<K, V>, k : K) -> &V
  native fun host_iter_borrow_mut {K} {V}(it : IteratorPtr<K>, t : &mut Table<K, V>) -> &mut V
  @[map_new (Table)]
  public fun new {K} {V}() -> Table<K, V> := host_new()
  @[map_add_no_override (Table)]
  public fun add {K} {V}(t : &mut Table<K, V>, k : K, v : V) -> Unit := host_add(t, k, v)
  @[map_borrow (Table)]
  public fun borrow {K} {V}(t : &Table<K, V>, k : K) -> &V := host_borrow(t, k)
  @[map_iter_borrow_mut (Table)]
  public fun iter_borrow_mut {K} {V}(it : IteratorPtr<K>, t : &mut Table<K, V>) -> &mut V :=
    host_iter_borrow_mut(it, t)
  @[map_spec_has_key (Table)]
  opaque spec fun has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_spec_get (Table)]
  opaque spec fun get {K} {V}(t : Table<K, V>, k : K) : V
  @[map_spec_set (Table)]
  opaque spec fun set {K} {V}(t : Table<K, V>, k : K, v : V) : Table<K, V>

  -- The entry a keyed iterator names is written through; an end iterator
  -- and an absent key are a missing entry.
  public fun written(k : u8) -> u64 := do
    let mut t := new::<u8, u64>()
    add(&mut t, 1u8, 3u64)
    *iter_borrow_mut(new IteratorPtr<u8>::Some { key := k }, &mut t) := 9
    *borrow(&t, 1u8)

  public fun ended() -> u64 := do
    let mut t := new::<u8, u64>()
    add(&mut t, 1u8, 3u64)
    *iter_borrow_mut(new IteratorPtr<u8>::End {}, &mut t) := 9
    0

open VerexIR VerexE2ETests.CheckSupport in
run_cmd do
  assertRuns `«0x42».keyed_cursors #[
    ⟨"written", #[.integer 1], .returned #[.integer 9], {}⟩,
    ⟨"written", #[.integer 2], .threw .abort #[.integer 25863], {}⟩,
    ⟨"ended", #[], .threw .abort #[.integer 25863], {}⟩]

verex module 0x42::positioned_cursors where
  pragma verify = false
  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address
  enum IteratorPtr has Copy, Drop where
    | End
    | Some (position : u64)
  native fun host_new {K} {V}() -> Table<K, V>
  native fun host_add {K} {V}(t : &mut Table<K, V>, k : K, v : V) -> Unit
  native fun host_borrow {K} {V}(t : &Table<K, V>, k : K) -> &V
  native fun host_iter_borrow_mut {K} {V}(it : IteratorPtr, t : &mut Table<K, V>) -> &mut V
  @[map_new (Table)]
  public fun new {K} {V}() -> Table<K, V> := host_new()
  @[map_add_no_override (Table)]
  public fun add {K} {V}(t : &mut Table<K, V>, k : K, v : V) -> Unit := host_add(t, k, v)
  @[map_borrow (Table)]
  public fun borrow {K} {V}(t : &Table<K, V>, k : K) -> &V := host_borrow(t, k)
  @[map_iter_borrow_mut (Table)]
  public fun iter_borrow_mut {K} {V}(it : IteratorPtr, t : &mut Table<K, V>) -> &mut V :=
    host_iter_borrow_mut(it, t)
  @[map_spec_has_key (Table)]
  opaque spec fun has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_spec_get (Table)]
  opaque spec fun get {K} {V}(t : Table<K, V>, k : K) : V
  @[map_spec_set (Table)]
  opaque spec fun set {K} {V}(t : Table<K, V>, k : K, v : V) : Table<K, V>

  -- A position names the entry at that index of the table's entries, in
  -- insertion order for an owner binding no ordering role; one past the
  -- end is a missing entry.
  public fun written(position : u64) -> u64 := do
    let mut t := new::<u8, u64>()
    add(&mut t, 5u8, 3u64)
    add(&mut t, 2u8, 4u64)
    *iter_borrow_mut(new IteratorPtr::Some { position := position }, &mut t) := 9
    *borrow(&t, 2u8)

open VerexIR VerexE2ETests.CheckSupport in
run_cmd do
  assertRuns `«0x42».positioned_cursors #[
    ⟨"written", #[.integer 1], .returned #[.integer 9], {}⟩,
    ⟨"written", #[.integer 0], .returned #[.integer 4], {}⟩,
    ⟨"written", #[.integer 2], .threw .abort #[.integer 25863], {}⟩]

verex module 0x42::table_conversion where
  pragma verify = false
  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address
  @[intrinsic_map]
  struct Ordered {K} {V} has Store where
    handle : Address
  native fun host_new {K} {V}() -> Table<K, V>
  native fun host_add {K} {V}(t : &mut Table<K, V>, k : K, v : V) -> Unit
  native fun host_to_ordered {K} {V}(t : &Table<K, V>) -> Ordered<K, V>
  native fun host_ordered_borrow {K} {V}(o : &Ordered<K, V>, k : K) -> &V
  @[map_new (Table)]
  public fun new {K} {V}() -> Table<K, V> := host_new()
  @[map_add_no_override (Table)]
  public fun add {K} {V}(t : &mut Table<K, V>, k : K, v : V) -> Unit := host_add(t, k, v)
  @[map_to_ordered_map (Table)]
  public fun to_ordered {K} {V}(t : &Table<K, V>) -> Ordered<K, V> := host_to_ordered(t)
  @[map_spec_has_key (Table)]
  opaque spec fun has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_spec_get (Table)]
  opaque spec fun get {K} {V}(t : Table<K, V>, k : K) : V
  @[map_spec_set (Table)]
  opaque spec fun set {K} {V}(t : Table<K, V>, k : K, v : V) : Table<K, V>
  @[map_spec_len (Table)]
  opaque spec fun size {K} {V}(t : Table<K, V>) : Int
  @[map_spec_key_at (Table)]
  opaque spec fun key_at {K} {V}(t : Table<K, V>, i : Int) : K
  @[map_borrow (Ordered)]
  public fun ordered_borrow {K} {V}(o : &Ordered<K, V>, k : K) -> &V := host_ordered_borrow(o, k)
  @[map_spec_has_key (Ordered)]
  opaque spec fun ordered_has {K} {V}(o : Ordered<K, V>, k : K) : Bool
  @[map_spec_get (Ordered)]
  opaque spec fun ordered_get {K} {V}(o : Ordered<K, V>, k : K) : V

  -- The conversion builds a table of the result's owner, at a fresh handle,
  -- holding the same entries.
  public fun converted() -> u64 := do
    let mut t := new::<u8, u64>()
    add(&mut t, 1u8, 3u64)
    let o := to_ordered(&t)
    *ordered_borrow(&o, 1u8)

open VerexIR VerexE2ETests.CheckSupport in
run_cmd do
  assertRuns `«0x42».table_conversion #[
    ⟨"converted", #[], .returned #[.integer 3], {}⟩]
