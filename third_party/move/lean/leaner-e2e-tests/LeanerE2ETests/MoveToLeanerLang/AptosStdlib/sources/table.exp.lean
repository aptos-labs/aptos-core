-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
Type of large-scale storage tables.
source: https://github.com/move-language/move/blob/1b6b7513dcc1a5c866f178ca5c1e74beb2ce181e/language/extensions/move-table-extension/sources/Table.move#L1

It implements the Table type which supports individual table items to be represented by
separate global state items. The number of items and a unique handle are tracked on the table
struct itself, while the operations are implemented as native functions. No traversal is provided.
-/
leaner module 0x1::table where
  friend aptos_std::storage_slots_allocator;
  friend aptos_std::table_with_length;

  /--
  Type of tables
  -/
  @[intrinsic_map]
  struct Table {K : phantom type has Copy, Drop} {V : phantom type} has Store where
    handle : Address

  spec Table where
    pragma intrinsic = map

  /--
  Create a new Table.
  -/
  @[map_new (Table)]
  public fun new {K has Copy, Drop} {V has Store}() -> Table<K, V> :=
    new Table<K, V> { handle := new_table_handle::<K, V>() }

  spec new where
    pragma intrinsic

  /--
  Add a new entry to the table. Aborts if an entry for this
  key already exists. The entry itself is not stored in the
  table, and cannot be discovered from it.
  -/
  @[map_add_no_override (Table)]
  public fun add {K has Copy, Drop} {V}(
    self : &mut Table<K, V>, key : K, val : V
  ) -> Unit := do
    add_box::<K, V, Box<V> >(self, key, new Box<V> { val })

  spec add where
    pragma intrinsic

  /--
  Acquire an immutable reference to the value which `key` maps to.
  Aborts if there is no entry for `key`.
  -/
  @[map_borrow (Table)]
  public fun borrow {K has Copy, Drop} {V}(
    self : &Table<K, V>, key : K
  ) -> &V :=
    &borrow_box::<K, V, Box<V> >(self, key).val

  spec borrow where
    pragma intrinsic

  /--
  Acquire an immutable reference to the value which `key` maps to.
  Returns specified default value if there is no entry for `key`.
  -/
  @[map_borrow_with_default (Table)]
  public fun borrow_with_default {K has Copy, Drop} {V}(
    self : &Table<K, V>, key : K, default : &V
  ) -> &V :=
    if !self.contains(core.prim.copyValue(key)) then default
    else self.borrow(core.prim.copyValue(key))

  spec borrow_with_default where
    pragma intrinsic

  /--
  Acquire a mutable reference to the value which `key` maps to.
  Aborts if there is no entry for `key`.
  -/
  @[map_borrow_mut (Table)]
  public fun borrow_mut {K has Copy, Drop} {V}(
    self : &mut Table<K, V>, key : K
  ) -> &mut V := &mut borrow_box_mut::<K, V, Box<V> >(self, key).val

  spec borrow_mut where
    pragma intrinsic

  /--
  Acquire a mutable reference to the value which `key` maps to.
  Insert the pair (`key`, `default`) first if there is no entry for `key`.
  -/
  @[map_borrow_mut_with_default (Table)]
  public fun borrow_mut_with_default {K has Copy, Drop} {V has Drop}(
    self : &mut Table<K, V>, key : K, default : V
  ) -> &mut V := do
    if !self.contains(core.prim.copyValue(key)) then
      self.add(core.prim.copyValue(key), default)
    return self.borrow_mut(key)

  spec borrow_mut_with_default where
    pragma intrinsic

  /--
  Insert the pair (`key`, `value`) if there is no entry for `key`.
  update the value of the entry for `key` to `value` otherwise
  -/
  @[map_add_override_if_exists (Table)]
  public fun upsert {K has Copy, Drop} {V has Drop}(
    self : &mut Table<K, V>, key : K, value : V
  ) -> Unit :=
    if !self.contains(core.prim.copyValue(key)) then
      self.add(core.prim.copyValue(key), value)
    else
      let ref := self.borrow_mut(key)
      *ref := value

  spec upsert where
    pragma intrinsic

  /--
  Remove from `self` and return the value which `key` maps to.
  Aborts if there is no entry for `key`.
  -/
  @[map_del_must_exist (Table)]
  public fun remove {K has Copy, Drop} {V}(
    self : &mut Table<K, V>, key : K
  ) -> V := do
    let Box<V> { val := val } := remove_box::<K, V, Box<V> >(self, key)
    return val

  spec remove where
    pragma intrinsic

  /--
  Returns true iff `self` contains an entry for `key`.
  -/
  @[map_has_key (Table)]
  public fun contains {K has Copy, Drop} {V}(
    self : &Table<K, V>, key : K
  ) -> Bool := contains_box::<K, V, Box<V> >(self, key)

  spec contains where
    pragma intrinsic

  /--
  Table cannot know if it is empty or not, so this method is not public,
  and can be used only in modules that know by themselves that table is empty.
  -/
  @[map_destroy_empty (Table)]
  friend fun destroy_known_empty_unsafe {K has Copy, Drop} {V}(
    self : Table<K, V>
  ) -> Unit := do
    destroy_empty_box::<K, V, Box<V> >(&self)
    drop_unchecked_box::<K, V, Box<V> >(self)

  spec destroy_known_empty_unsafe where
    pragma intrinsic

  -- ======================================================================================================
  -- Internal API
  /--
  Wrapper for values. Required for making values appear as resources in the implementation.
  -/
  struct Box {V} has Drop, Store, Key where
    val : V

  -- Primitives which take as an additional type parameter `Box<V>`, so the implementation
  -- can use this to determine serialization layout.
  native fun new_table_handle {K} {V}() -> Address

  native fun add_box {K has Copy, Drop} {V} {B}(
    table : &mut Table<K, V>, key : K, val : Box<V>
  ) -> Unit

  native fun borrow_box {K has Copy, Drop} {V} {B}(
    table : &Table<K, V>, key : K
  ) -> &Box<V>

  native fun borrow_box_mut {K has Copy, Drop} {V} {B}(
    table : &mut Table<K, V>, key : K
  ) -> &mut Box<V>

  native fun contains_box {K has Copy, Drop} {V} {B}(
    table : &Table<K, V>, key : K
  ) -> Bool

  native fun remove_box {K has Copy, Drop} {V} {B}(
    table : &mut Table<K, V>, key : K
  ) -> Box<V>

  native fun destroy_empty_box {K has Copy, Drop} {V} {B}(
    table : &Table<K, V>
  ) -> Unit

  native fun drop_unchecked_box {K has Copy, Drop} {V} {B}(
    table : Table<K, V>
  ) -> Unit

  -- Make most of the public API intrinsic. Those functions have custom specifications in the prover.
  -- Specification functions for tables
  @[map_spec_has_key (Table)]
  opaque spec fun spec_contains {K} {V}(t : Table<K, V>, k : K) : Bool

  @[map_spec_del (Table)]
  opaque spec fun spec_remove {K} {V}(t : Table<K, V>, k : K) : Table<K, V>

  @[map_spec_set (Table)]
  opaque spec fun spec_set {K} {V}(t : Table<K, V>, k : K, v : V) : Table<K, V>

  @[map_spec_get (Table)]
  opaque spec fun spec_get {K} {V}(t : Table<K, V>, k : K) : V
