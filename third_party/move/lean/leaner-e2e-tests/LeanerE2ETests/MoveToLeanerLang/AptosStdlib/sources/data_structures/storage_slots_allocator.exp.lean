-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
Abstraction to having "addressable" storage slots (i.e. items) in global storage.
Addresses are local u64 values (unique within a single StorageSlotsAllocator instance,
but can and do overlap across instances).

Allows optionally to initialize slots (and pay for them upfront), and then reuse them,
providing predictable storage costs.

If we need to mutate multiple slots at the same time, we can workaround borrow_mut preventing us from that,
via provided pair of `remove_and_reserve` and `fill_reserved_slot` methods, to do so in non-conflicting manner.

Similarly allows getting an address upfront via `reserve_slot`, for a slot created
later (i.e. if we need address to initialize the value itself).

In the future, more sophisticated strategies can be added, without breaking/modifying callers,
for example:
* inlining some nodes
* having a fee-payer for any storage creation operations
-/
leaner module 0x1::storage_slots_allocator where
  use 0x1::aptos_std::table_with_length
  use 0x1::aptos_std::table_with_length::TableWithLength
  use 0x1::std::error::invalid_argument
  use 0x1::std::option
  use 0x1::std::option::Option
  use 0x1::std::option::destroy_none
  use 0x1::std::option::destroy_some
  use 0x1::std::option::fill
  use 0x1::std::option::is_none
  use 0x1::std::option::is_some
  use 0x1::std::option::none

  const EINVALID_ARGUMENT : u64 := 1

  const ECANNOT_HAVE_SPARES_WITHOUT_REUSE : u64 := 2

  const EINTERNAL_INVARIANT_BROKEN : u64 := 7

  const NULL_INDEX : u64 := 0

  const FIRST_INDEX : u64 := 10

  -- keeping space for usecase-specific values
  /--
  Data stored in an individual slot
  -/
  enum Link {T has Store} has Store where
    | Occupied (value : T)
    | Vacant (next : u64)

  enum StorageSlotsAllocator {T has Store} has Store where
    | V1 (slots : Option<TableWithLength<u64, Link<T> > >,
      new_slot_index : u64,
      should_reuse : Bool,
      reuse_head_index : u64,
      reuse_spare_count : u32)

  -- V1 is sequential - any two operations on the StorageSlotsAllocator will conflict.
  -- In general, StorageSlotsAllocator is invoked on less frequent operations, so
  -- that shouldn't be a big issue.
  -- Lazily create slots table only when needed
  /--
  Handle to a reserved slot within a transaction.
  Not copy/drop/store-able, to guarantee reservation
  is used or released within the transaction.
  -/
  struct ReservedSlot where
    slot_index : u64

  /--
  Ownership handle to a slot.
  Not copy/drop-able to make sure slots are released when not needed,
  and there is unique owner for each slot.
  -/
  struct StoredSlot has Store where
    slot_index : u64

  public fun new {T has Store}(
    should_reuse : Bool
  ) -> StorageSlotsAllocator<T> :=
    new StorageSlotsAllocator<T>::V1 {
      slots := none::<TableWithLength<u64, Link<T> > >(),
      new_slot_index := FIRST_INDEX, should_reuse,
      reuse_head_index := NULL_INDEX, reuse_spare_count := 0u32
    }

  public fun allocate_spare_slots {T has Store}(
    self : &mut StorageSlotsAllocator<T>, num_to_allocate : u64
  ) -> Unit := do
    assert!(
      self.should_reuse, invalid_argument(
        ECANNOT_HAVE_SPARES_WITHOUT_REUSE
      )
    )
    for i in 0..num_to_allocate do
      let slot_index := self.next_slot_index()
      self.maybe_push_to_reuse_queue(slot_index)

  public fun get_num_spare_slot_count {T has Store}(
    self : &StorageSlotsAllocator<T>
  ) -> u32 := do
    assert!(
      self.should_reuse, invalid_argument(
        ECANNOT_HAVE_SPARES_WITHOUT_REUSE
      )
    )
    return self.reuse_spare_count

  public fun add {T has Store}(
    self : &mut StorageSlotsAllocator<T>, val : T
  ) -> StoredSlot := do
    let (stored_slot, reserved_slot) := self.reserve_slot()
    self.fill_reserved_slot(reserved_slot, val)
    return stored_slot

  public fun remove {T has Store}(
    self : &mut StorageSlotsAllocator<T>, slot : StoredSlot
  ) -> T := do
    let (reserved_slot, value) :=
      self.remove_and_reserve(slot.stored_to_index())
    self.free_reserved_slot(reserved_slot, slot)
    return value

  public fun destroy_empty {T has Store}(
    mut self : StorageSlotsAllocator<T>
  ) -> Unit := do
    loop do
      let reuse_index := self.maybe_pop_from_reuse_queue()
      if reuse_index == NULL_INDEX then break
    match self with
      | StorageSlotsAllocator<T>::V1 { slots := slots,
      new_slot_index := _,
      should_reuse := _,
      reuse_head_index := reuse_head_index,
      reuse_spare_count := _ } => do
        assert!(reuse_head_index == NULL_INDEX, EINTERNAL_INVARIANT_BROKEN)
        if is_some(&slots) then
          table_with_length::destroy_empty(destroy_some(slots))
        else destroy_none(slots)

  public fun borrow {T has Store}(
    self : &StorageSlotsAllocator<T>, slot_index : u64
  ) -> &T :=
    &table_with_length::borrow(option::borrow(&self.slots), slot_index).value

  public fun borrow_mut {T has Store}(
    self : &mut StorageSlotsAllocator<T>, slot_index : u64
  ) -> &mut T :=
    &mut table_with_length::borrow_mut(
      option::borrow_mut(&mut self.slots), slot_index
    ).value

  -- We also provide here operations where `add()` is split into `reserve_slot`,
  -- and then doing fill_reserved_slot later.
  -- Similarly we have `remove_and_reserve`, and then `fill_reserved_slot` later.
  public fun reserve_slot {T has Store}(
    self : &mut StorageSlotsAllocator<T>
  ) -> (StoredSlot, ReservedSlot) := do
    let slot_index := self.maybe_pop_from_reuse_queue()
    if slot_index == NULL_INDEX then slot_index := self.next_slot_index()
    return (new StoredSlot { slot_index }, new ReservedSlot { slot_index })

  public fun fill_reserved_slot {T has Store}(
    self : &mut StorageSlotsAllocator<T>, slot : ReservedSlot, val : T
  ) -> Unit := do
    let ReservedSlot { slot_index := slot_index } := slot
    self.add_link(slot_index, new Link<T>::Occupied { value := val })

  /--
  Remove storage slot, but reserve it for later.
  -/
  public fun remove_and_reserve {T has Store}(
    self : &mut StorageSlotsAllocator<T>, slot_index : u64
  ) -> (ReservedSlot, T) := do
    let Link<T>::Occupied { value := value } := self.remove_link(slot_index)
    return (new ReservedSlot { slot_index }, value)

  public fun free_reserved_slot {T has Store}(
    self : &mut StorageSlotsAllocator<T>, reserved_slot : ReservedSlot,
    stored_slot : StoredSlot
  ) -> Unit := do
    let ReservedSlot { slot_index := slot_index } := reserved_slot
    assert!(slot_index == stored_slot.slot_index, EINVALID_ARGUMENT)
    let StoredSlot { slot_index := _ } := stored_slot
    self.maybe_push_to_reuse_queue(slot_index)

  -- ========== Section for methods handling references ========
  public fun reserved_to_index(self : &ReservedSlot) -> u64 := self.slot_index

  public fun stored_to_index(self : &StoredSlot) -> u64 := self.slot_index

  public fun is_null_index(slot_index : u64) -> Bool := slot_index == NULL_INDEX

  public fun is_special_unused_index(slot_index : u64) -> Bool :=
    slot_index != NULL_INDEX && slot_index < FIRST_INDEX

  -- ========== Section for private internal utility methods ========
  fun maybe_pop_from_reuse_queue {T has Store}(
    self : &mut StorageSlotsAllocator<T>
  ) -> u64 := do
    let slot_index := self.reuse_head_index
    if slot_index != NULL_INDEX then
      let Link<T>::Vacant { next := next } := self.remove_link(slot_index)
      self.reuse_head_index := next
      let _t1 := &mut self.reuse_spare_count
      *_t1 := *_t1 - 1u32
    return slot_index

  fun maybe_push_to_reuse_queue {T has Store}(
    self : &mut StorageSlotsAllocator<T>, slot_index : u64
  ) -> Unit :=
    if self.should_reuse then
      let link := new Link<T>::Vacant { next := self.reuse_head_index }
      self.add_link(slot_index, link)
      self.reuse_head_index := slot_index
      let _t1 := &mut self.reuse_spare_count
      *_t1 := *_t1 + 1u32

  fun next_slot_index {T has Store}(
    self : &mut StorageSlotsAllocator<T>
  ) -> u64 := do
    let slot_index := self.new_slot_index
    let _t1 := &mut self.new_slot_index
    *_t1 := *_t1 + 1
    if is_none(&self.slots) then
      fill(&mut self.slots, table_with_length::new::<u64, Link<T> >())
    return slot_index

  fun add_link {T has Store}(
    self : &mut StorageSlotsAllocator<T>, slot_index : u64, link : Link<T>
  ) -> Unit := do
    table_with_length::add(
      option::borrow_mut(&mut self.slots), slot_index, link
    )

  fun remove_link {T has Store}(
    self : &mut StorageSlotsAllocator<T>, slot_index : u64
  ) -> Link<T> :=
    table_with_length::remove(option::borrow_mut(&mut self.slots), slot_index)
