-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Extends Table and provides functions such as length and the ability to be destroyed -/
leaner module 0x1::table_with_length where
  use 0x1::aptos_std::table
  use 0x1::aptos_std::table::Table
  use 0x1::aptos_std::table::destroy_known_empty_unsafe
  use 0x1::std::error::invalid_state

  -- native code raises this with error::invalid_arguments()
  const EALREADY_EXISTS : u64 := 100

  -- native code raises this with error::invalid_arguments()
  const ENOT_FOUND : u64 := 101

  const ENOT_EMPTY : u64 := 102

  /--
  Type of tables
  -/
  @[intrinsic_map]
  struct TableWithLength {K : phantom type has Copy, Drop} {V : phantom type} has Store where
    inner : Table<K, V>
    length : u64

  spec TableWithLength where
    pragma intrinsic = map

  /--
  Create a new Table.
  -/
  @[map_new (TableWithLength)]
  public fun new {K has Copy, Drop} {V has Store}() -> TableWithLength<K, V> :=
    new TableWithLength<K, V> { inner := table::new::<K, V>(), length := 0 }

  spec new where
    pragma intrinsic

  /--
  Destroy a table. The table must be empty to succeed.
  -/
  @[map_destroy_empty (TableWithLength)]
  public fun destroy_empty {K has Copy, Drop} {V}(
    self : TableWithLength<K, V>
  ) -> Unit := do
    assert!(self.length == 0, invalid_state(ENOT_EMPTY))
    let TableWithLength<K, V> { inner := inner, length := _ } := self
    destroy_known_empty_unsafe(inner)

  spec destroy_empty where
    pragma intrinsic

  /--
  Add a new entry to the table. Aborts if an entry for this
  key already exists. The entry itself is not stored in the
  table, and cannot be discovered from it.
  -/
  @[map_add_no_override (TableWithLength)]
  public fun add {K has Copy, Drop} {V}(
    self : &mut TableWithLength<K, V>, key : K, val : V
  ) -> Unit := do
    table::add(&mut self.inner, key, val)
    let _t1 := &mut self.length
    *_t1 := *_t1 + 1

  spec add where
    pragma intrinsic

  /--
  Acquire an immutable reference to the value which `key` maps to.
  Aborts if there is no entry for `key`.
  -/
  @[map_borrow (TableWithLength)]
  public fun borrow {K has Copy, Drop} {V}(
    self : &TableWithLength<K, V>, key : K
  ) -> &V := table::borrow(&self.inner, key)

  spec borrow where
    pragma intrinsic

  /--
  Acquire a mutable reference to the value which `key` maps to.
  Aborts if there is no entry for `key`.
  -/
  @[map_borrow_mut (TableWithLength)]
  public fun borrow_mut {K has Copy, Drop} {V}(
    self : &mut TableWithLength<K, V>, key : K
  ) -> &mut V := table::borrow_mut(&mut self.inner, key)

  spec borrow_mut where
    pragma intrinsic

  /--
  Returns the length of the table, i.e. the number of entries.
  -/
  @[map_len (TableWithLength)]
  public fun length {K has Copy, Drop} {V}(
    self : &TableWithLength<K, V>
  ) -> u64 :=
    self.length

  spec length where
    pragma intrinsic

  /--
  Returns true if this table is empty.
  -/
  @[map_is_empty (TableWithLength)]
  public fun empty {K has Copy, Drop} {V}(
    self : &TableWithLength<K, V>
  ) -> Bool :=
    self.length == 0

  spec empty where
    pragma intrinsic

  /--
  Acquire a mutable reference to the value which `key` maps to.
  Insert the pair (`key`, `default`) first if there is no entry for `key`.
  -/
  @[map_borrow_mut_with_default (TableWithLength)]
  public fun borrow_mut_with_default {K has Copy, Drop} {V has Drop}(
    self : &mut TableWithLength<K, V>, key : K, default : V
  ) -> &mut V :=
    if table::contains(&self.inner, key) then
      table::borrow_mut(&mut self.inner, key)
    else
      table::add(&mut self.inner, key, default)
      let _t1 := &mut self.length
      *_t1 := *_t1 + 1
      return table::borrow_mut(&mut self.inner, key)

  spec borrow_mut_with_default where
    pragma intrinsic
    aborts_if false

  /--
  Insert the pair (`key`, `value`) if there is no entry for `key`.
  update the value of the entry for `key` to `value` otherwise
  -/
  @[map_add_override_if_exists (TableWithLength)]
  public fun upsert {K has Copy, Drop} {V has Drop}(
    self : &mut TableWithLength<K, V>, key : K, value : V
  ) -> Unit :=
    if !table::contains(&self.inner, key) then
      self.add(core.prim.copyValue(key), value)
    else
      let ref := table::borrow_mut(&mut self.inner, key)
      *ref := value

  spec upsert where
    pragma intrinsic

  /--
  Remove from `table` and return the value which `key` maps to.
  Aborts if there is no entry for `key`.
  -/
  @[map_del_must_exist (TableWithLength)]
  public fun remove {K has Copy, Drop} {V}(
    self : &mut TableWithLength<K, V>, key : K
  ) -> V := do
    let val := table::remove(&mut self.inner, key)
    let _t1 := &mut self.length
    *_t1 := *_t1 - 1
    return val

  spec remove where
    pragma intrinsic

  /--
  Returns true iff `table` contains an entry for `key`.
  -/
  @[map_has_key (TableWithLength)]
  public fun contains {K has Copy, Drop} {V}(
    self : &TableWithLength<K, V>, key : K
  ) -> Bool := table::contains(&self.inner, key)

  spec contains where
    pragma intrinsic

  -- Unpack table with length, dropping length count but not
  -- inner table.
  -- Drop inner table.
  -- Declare new table.
  -- Add table entry.
  -- Drop table.
  -- Table should not have key 0 yet
  -- This should insert key 0, with value 10, and length should be 1
  -- Ensure the value is correctly set to 10
  -- Ensure the length is correctly set
  -- Lets upsert the value to something else, and verify it's correct
  -- Since key 0 already existed, the length should not have changed
  -- If we upsert a non-existing key, the length should increase
  -- Make most of the public API intrinsic. Those functions have custom specifications in the prover.
  -- Specification functions for tables
  @[map_spec_len (TableWithLength)]
  opaque spec fun spec_len {K} {V}(t : TableWithLength<K, V>) : Int

  @[map_spec_has_key (TableWithLength)]
  opaque spec fun spec_contains {K} {V}(t : TableWithLength<K, V>, k : K) : Bool

  @[map_spec_set (TableWithLength)]
  opaque spec fun spec_set {K} {V}(
    t : TableWithLength<K, V>, k : K, v : V
  ) : TableWithLength<K, V>

  @[map_spec_del (TableWithLength)]
  opaque spec fun spec_remove {K} {V}(
    t : TableWithLength<K, V>, k : K
  ) : TableWithLength<K, V>

  @[map_spec_get (TableWithLength)]
  opaque spec fun spec_get {K} {V}(t : TableWithLength<K, V>, k : K) : V
