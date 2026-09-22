-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
This module provides an implementation for an big ordered map.
Big means that it is stored across multiple resources, and doesn't have an
upper limit on number of elements it can contain.

Keys point to values, and each key in the map must be unique.

Currently, one implementation is provided - BPlusTreeMap, backed by a B+Tree,
with each node being a separate resource, internally containing OrderedMap.

BPlusTreeMap is chosen since the biggest (performance and gas)
costs are reading resources, and it:
* reduces number of resource accesses
* reduces number of rebalancing operations, and makes each rebalancing
  operation touch only few resources
* it allows for parallelism for keys that are not close to each other,
  once it contains enough keys

Note: Default configuration (used in `new_with_config(0, 0, false)`) allows for keys and values of up to 5KB,
or 100 times the first (key, value), to satisfy general needs.
If you need larger, use other constructor methods.
Based on initial configuration, BigOrderedMap will always accept insertion of keys and values
up to the allowed size, and will abort with EKEY_BYTES_TOO_LARGE or EARGUMENT_BYTES_TOO_LARGE.

Warning: All iterator functions need to be carefully used, because they are just pointers into the
structure, and modification of the map invalidates them (without compiler being able to catch it).
Type is also named IteratorPtr, so that Iterator is free to use later.
Better guarantees would need future Move improvements that will allow references to be part of the struct,
allowing cleaner iterator APIs.

That's why all functions returning iterators are prefixed with "internal_", to clarify nuances needed to make
sure usage is correct.
A set of inline utility methods is provided instead, to provide guaranteed valid usage to iterators.
-/
leaner module 0x1::big_ordered_map where
  use 0x1::aptos_framework::ordered_map
  use 0x1::aptos_framework::ordered_map::OrderedMap
  use 0x1::aptos_framework::ordered_map::append_disjoint
  use 0x1::aptos_framework::ordered_map::iter_add
  use 0x1::aptos_framework::ordered_map::iter_is_begin_from_non_empty
  use 0x1::aptos_framework::ordered_map::iter_replace
  use 0x1::aptos_framework::ordered_map::length
  use 0x1::aptos_framework::ordered_map::replace_key_inplace
  use 0x1::aptos_framework::ordered_map::trim
  use 0x1::aptos_std::math64::max
  use 0x1::aptos_std::math64::min
  use 0x1::aptos_std::storage_slots_allocator
  use 0x1::aptos_std::storage_slots_allocator::ReservedSlot
  use 0x1::aptos_std::storage_slots_allocator::StorageSlotsAllocator
  use 0x1::aptos_std::storage_slots_allocator::StoredSlot
  use 0x1::aptos_std::storage_slots_allocator::fill_reserved_slot
  use 0x1::aptos_std::storage_slots_allocator::free_reserved_slot
  use 0x1::aptos_std::storage_slots_allocator::is_null_index
  use 0x1::aptos_std::storage_slots_allocator::is_special_unused_index
  use 0x1::aptos_std::storage_slots_allocator::remove_and_reserve
  use 0x1::aptos_std::storage_slots_allocator::reserve_slot
  use 0x1::aptos_std::storage_slots_allocator::reserved_to_index
  use 0x1::aptos_std::storage_slots_allocator::stored_to_index
  use 0x1::std::bcs::constant_serialized_size
  use 0x1::std::bcs::serialized_size
  use 0x1::std::cmp::Ordering
  use 0x1::std::cmp::compare
  use 0x1::std::cmp::is_lt
  use 0x1::std::error::invalid_argument
  use 0x1::std::error::invalid_state
  use 0x1::std::option
  use 0x1::std::option::Option
  use 0x1::std::option::destroy_none
  use 0x1::std::option::destroy_some
  use 0x1::std::option::is_none
  use 0x1::std::option::is_some
  use 0x1::std::option::none
  use 0x1::std::option::some
  use 0x1::std::option::spec_borrow
  use 0x1::std::option::spec_is_none
  use 0x1::std::option::spec_is_some
  use 0x1::std::vector
  use 0x1::std::vector::spec_contains

  pragma verify = false

  -- Error constants shared with ordered_map (so try using same values)
  /--
  Map key already exists
  -/
  const EKEY_ALREADY_EXISTS : u64 := 1

  /--
  Map key is not found
  -/
  const EKEY_NOT_FOUND : u64 := 2

  /--
  Trying to do an operation on an IteratorPtr that would go out of bounds
  -/
  const EITER_OUT_OF_BOUNDS : u64 := 3

  -- Error constants specific to big_ordered_map
  /--
  The provided configuration parameter is invalid.
  -/
  const EINVALID_CONFIG_PARAMETER : u64 := 11

  /--
  Map isn't empty
  -/
  const EMAP_NOT_EMPTY : u64 := 12

  /--
  Trying to insert too large of an (key, value) into the map.
  -/
  const EARGUMENT_BYTES_TOO_LARGE : u64 := 13

  /--
  borrow_mut requires that key and value types have constant size
  (otherwise it wouldn't be able to guarantee size requirements are not violated)
  Use remove() + add() combo instead.
  -/
  const EBORROW_MUT_REQUIRES_CONSTANT_VALUE_SIZE : u64 := 14

  /--
  Trying to insert too large of a key into the map.
  -/
  const EKEY_BYTES_TOO_LARGE : u64 := 15

  /--
  Cannot use new/new_with_reusable with variable-sized types.
  Use `new_with_type_size_hints()` or `new_with_config()` instead if your types have variable sizes.
  `new_with_config(0, 0, false)` tries to work reasonably well for variety of sizes
  (allows keys or values of at least 5KB and 100x larger than the first inserted)
  -/
  const ECANNOT_USE_NEW_WITH_VARIABLE_SIZED_TYPES : u64 := 16

  -- Errors that should never be thrown
  /--
  Internal errors.
  -/
  const EINTERNAL_INVARIANT_BROKEN : u64 := 20

  -- Internal constants.
  -- Bounds on degrees:
  /--
  Smallest allowed degree on inner nodes.
  -/
  const INNER_MIN_DEGREE : u16 := 4u16

  /--
  Smallest allowed degree on leaf nodes.

  We rely on 1 being valid size only for root node,
  so this cannot be below 3 (unless that is changed)
  -/
  const LEAF_MIN_DEGREE : u16 := 3u16

  /--
  Largest degree allowed (both for inner and leaf nodes)
  -/
  const MAX_DEGREE : u64 := 4096

  -- Bounds on serialized sizes:
  /--
  Largest size all keys for inner nodes or key-value pairs for leaf nodes can have.
  Node itself can be a bit larger, due to few other accounting fields.
  This is a bit conservative, a bit less than half of the resource limit (which is 1MB)
  -/
  const MAX_NODE_BYTES : u64 := 409600

  /--
  Target node size, from efficiency perspective.
  -/
  const DEFAULT_TARGET_NODE_SIZE : u64 := 4096

  /--
  When using default constructors (new() / new_with_reusable() / new_with_config(0, 0, _))
  making sure key or value of this size (5KB) will be accepted, which should satisfy most cases
  If you need keys/values that are larger, use other constructors.
  -/
  const DEFAULT_MAX_KEY_OR_VALUE_SIZE : u64 := 5120

  -- 5KB
  /--
  Target max node size, when using hints (via new_with_type_size_hints).
  Smaller than MAX_NODE_BYTES, to improve performence, as large nodes are innefficient.
  -/
  const HINT_MAX_NODE_BYTES : u64 := 131072

  -- Constants aligned with storage_slots_allocator
  const NULL_INDEX : u64 := 0

  const ROOT_INDEX : u64 := 1

  /--
  A node of the BigOrderedMap.

  Inner node will have all children be Child::Inner, pointing to the child nodes.
  Leaf node will have all children be Child::Leaf.
  Basically - Leaf node is a single-resource OrderedMap, containing as much key/value entries, as can fit.
  So Leaf node contains multiple values, not just one.
  -/
  enum Node {K has Store} {V has Store} has Store where
    | V1 (is_leaf : Bool,
      children : OrderedMap<K, Child<V> >,
      prev : u64,
      next : u64)

  -- Whether this node is a leaf node.
  -- The children of the nodes.
  -- When node is inner node, K represents max_key within the child subtree, and values are Child::Inner.
  -- When the node is leaf node, K represents key of the leaf, and values are Child::Leaf.
  -- The node index of its previous node at the same level, or `NULL_INDEX` if it doesn't have a previous node.
  -- The node index of its next node at the same level, or `NULL_INDEX` if it doesn't have a next node.
  /--
  Contents of a child node.
  -/
  enum Child {V has Store} has Store where
    | Inner (node_index : StoredSlot)
    | Leaf (value : V)

  -- The node index of it's child
  -- Value associated with the leaf node.
  /--
  An iterator to iterate all keys in the BigOrderedMap.

  TODO: Once fields can be (mutable) references, this class will be deprecated.
  -/
  enum IteratorPtr {K} has Copy, Drop where
    | End
    | Some (node_index : u64, child_iter : ordered_map::IteratorPtr, key : K)

  struct IteratorPtrWithPath {K} has Copy, Drop where
    iterator : IteratorPtr<K>
    path : Vector<u64>

  /--
  The BigOrderedMap data structure.
  -/
  @[intrinsic_map]
  enum BigOrderedMap {K has Store} {V has Store} has Store where
    | BPlusTreeMap (root : Node<K, V>,
      nodes : StorageSlotsAllocator<Node<K, V> >,
      min_leaf_index : u64,
      max_leaf_index : u64,
      constant_kv_size : Bool,
      inner_max_degree : u16,
      leaf_max_degree : u16)

  spec BigOrderedMap where
    pragma intrinsic = map

  -- ======================= Constructors && Destructors ====================
  /--
  Returns a new BigOrderedMap with the default configuration.

  Cannot be used with variable-sized types.
  Use `new_with_type_size_hints()` or `new_with_config()` instead if your types have variable sizes.
  `new_with_config(0, 0, false)` tries to work reasonably well for variety of sizes
  (allows keys or values of at least 5KB and 100x larger than the first inserted)
  -/
  @[map_new (BigOrderedMap)]
  public fun new {K has Store} {V has Store}() -> BigOrderedMap<K, V> := do
    assert!(
      is_some(&constant_serialized_size::<K>())
        && is_some(&constant_serialized_size::<V>()),
      invalid_argument(ECANNOT_USE_NEW_WITH_VARIABLE_SIZED_TYPES)
    )
    return new_with_config::<K, V>(0u16, 0u16, false)

  spec new where
    pragma intrinsic

  /--
  Returns a new BigOrderedMap with with reusable storage slots.

  Cannot be used with variable-sized types.
  Use `new_with_type_size_hints()` or `new_with_config()` instead if your types have variable sizes.
  `new_with_config(0, 0, false)` tries to work reasonably well for variety of sizes
  (allows keys or values of at least 5KB and 100x larger than the first inserted)
  -/
  public fun new_with_reusable {K has Store} {V has Store}() -> BigOrderedMap<K, V> := do
    assert!(
      is_some(&constant_serialized_size::<K>())
        && is_some(&constant_serialized_size::<V>()),
      invalid_argument(ECANNOT_USE_NEW_WITH_VARIABLE_SIZED_TYPES)
    )
    return new_with_config::<K, V>(0u16, 0u16, true)

  spec new_with_reusable where
    pragma opaque
    pragma verify = false
    aborts_if false
    ensures spec_len(result) == 0
    ensures ∀ (k : K), !spec_contains_key(result, k)

  /--
  Returns a new BigOrderedMap, configured based on passed key and value serialized size hints.
  -/
  public fun new_with_type_size_hints {K has Store} {V has Store}(
    avg_key_bytes : u64, max_key_bytes : u64, avg_value_bytes : u64,
    max_value_bytes : u64
  ) -> BigOrderedMap<K, V> := do
    assert!(
      avg_key_bytes <= max_key_bytes,
      invalid_argument(EINVALID_CONFIG_PARAMETER)
    )
    assert!(
      avg_value_bytes <= max_value_bytes,
      invalid_argument(EINVALID_CONFIG_PARAMETER)
    )
    let inner_max_degree_from_avg :=
      max(
        min(MAX_DEGREE, DEFAULT_TARGET_NODE_SIZE / avg_key_bytes),
        INNER_MIN_DEGREE as u64
      )
    let inner_max_degree_from_max := HINT_MAX_NODE_BYTES / max_key_bytes
    assert!(
      inner_max_degree_from_max >= INNER_MIN_DEGREE as u64,
      invalid_argument(EINVALID_CONFIG_PARAMETER)
    )
    let avg_entry_size := avg_key_bytes + avg_value_bytes
    let max_entry_size := max_key_bytes + max_value_bytes
    let leaf_max_degree_from_avg :=
      max(
        min(MAX_DEGREE, DEFAULT_TARGET_NODE_SIZE / avg_entry_size),
        LEAF_MIN_DEGREE as u64
      )
    let leaf_max_degree_from_max := HINT_MAX_NODE_BYTES / max_entry_size
    assert!(
      leaf_max_degree_from_max >= LEAF_MIN_DEGREE as u64,
      invalid_argument(EINVALID_CONFIG_PARAMETER)
    )
    return new_with_config::<K, V>(
      min(inner_max_degree_from_avg, inner_max_degree_from_max) as u16,
      min(leaf_max_degree_from_avg, leaf_max_degree_from_max) as u16, false
    )

  spec new_with_type_size_hints where
    pragma opaque
    pragma verify = false
    aborts_if avg_key_bytes > max_key_bytes
    aborts_if avg_value_bytes > max_value_bytes
    aborts_if avg_key_bytes == 0
    aborts_if max_key_bytes > 0
        && HINT_MAX_NODE_BYTES / max_key_bytes < INNER_MIN_DEGREE
    aborts_if avg_key_bytes + avg_value_bytes > 18446744073709551615
    aborts_if max_key_bytes + max_value_bytes > 18446744073709551615
    aborts_if max_key_bytes + max_value_bytes > 0
        && HINT_MAX_NODE_BYTES / (max_key_bytes + max_value_bytes)
          < LEAF_MIN_DEGREE
    ensures spec_len(result) == 0
    ensures ∀ (k : K), !spec_contains_key(result, k)

  /--
  Returns a new BigOrderedMap with the provided max degree consts (the maximum # of children a node can have, both inner and leaf).

  If 0 is passed, then it is dynamically computed based on size of first key and value.
  WIth 0 it is configured to accept keys and values up to 5KB in size,
  or as large as 100x the size of the first insert. (100 = MAX_NODE_BYTES / DEFAULT_TARGET_NODE_SIZE)

  Sizes of all elements must respect (or their additions will be rejected):
    `key_size * inner_max_degree <= MAX_NODE_BYTES`
    `entry_size * leaf_max_degree <= MAX_NODE_BYTES`
  If keys or values have variable size, and first element could be non-representative in size (i.e. smaller than future ones),
  it is important to compute and pass inner_max_degree and leaf_max_degree based on the largest element you want to be able to insert.

  `reuse_slots` means that removing elements from the map doesn't free the storage slots and returns the refund.
  Together with `allocate_spare_slots`, it allows to preallocate slots and have inserts have predictable gas costs.
  (otherwise, inserts that require map to add new nodes, cost significantly more, compared to the rest)
  -/
  @[map_new_with_config (BigOrderedMap)]
  public fun new_with_config {K has Store} {V has Store}(
    inner_max_degree : u16, leaf_max_degree : u16, reuse_slots : Bool
  ) -> BigOrderedMap<K, V> := do
    assert!(
      inner_max_degree == 0u16
        || inner_max_degree >= INNER_MIN_DEGREE
          && inner_max_degree as u64 <= MAX_DEGREE,
      invalid_argument(EINVALID_CONFIG_PARAMETER)
    )
    assert!(
      leaf_max_degree == 0u16
        || leaf_max_degree >= LEAF_MIN_DEGREE
          && leaf_max_degree as u64 <= MAX_DEGREE,
      invalid_argument(EINVALID_CONFIG_PARAMETER)
    )
    assert!(
      is_null_index(NULL_INDEX), invalid_state(
        EINTERNAL_INVARIANT_BROKEN
      )
    )
    assert!(
      is_special_unused_index(ROOT_INDEX),
      invalid_state(EINTERNAL_INVARIANT_BROKEN)
    )
    let nodes := storage_slots_allocator::new::<Node<K, V> >(reuse_slots)
    let mut self :=
      new BigOrderedMap<K, V>::BPlusTreeMap {
        root := new_node::<K, V>(true), nodes, min_leaf_index := ROOT_INDEX,
        max_leaf_index := ROOT_INDEX, constant_kv_size := false,
        inner_max_degree, leaf_max_degree
      }
    self.validate_static_size_and_init_max_degrees()
    return self

  spec new_with_config where
    pragma intrinsic

  -- Assert that storage_slots_allocator special indices are aligned:
  -- is_leaf=
  -- Will be initialized in validate_static_size_and_init_max_degrees below.
  /--
  Create a BigOrderedMap from a vector of keys and values, with default configuration.
  Aborts with EKEY_ALREADY_EXISTS if duplicate keys are passed in.
  -/
  @[map_new_from (BigOrderedMap)]
  public fun new_from {K has Copy, Drop, Store} {V has Store}(
    keys : Vector<K>, values : Vector<V>
  ) -> BigOrderedMap<K, V> := do
    let mut map := new::<K, V>()
    map.add_all(keys, values)
    return map

  spec new_from where
    pragma intrinsic

  /--
  Destroys the map if it's empty, otherwise aborts.
  -/
  @[map_destroy_empty (BigOrderedMap)]
  public fun destroy_empty {K has Store} {V has Store}(
    self : BigOrderedMap<K, V>
  ) -> Unit := do
    let BigOrderedMap<K, V>::BPlusTreeMap { root := root,
    nodes := nodes,
    min_leaf_index := _,
    max_leaf_index := _,
    constant_kv_size := _,
    inner_max_degree := _,
    leaf_max_degree := _ } :=
      self
    root.destroy_empty_node()
    storage_slots_allocator::destroy_empty(nodes)

  spec destroy_empty where
    pragma intrinsic

  -- If root node is empty, then we know that no storage slots are used,
  -- and so we can safely destroy all nodes.
  /--
  Map was created with reuse_slots=true, you can allocate spare slots, to pay storage fee now, to
  allow future insertions to not require any storage slot creation - making their gas more predictable
  and better bounded/fair.
  (otherwsie, unlucky inserts create new storage slots and are charge more for it)
  -/
  public fun allocate_spare_slots {K has Store} {V has Store}(
    self : &mut BigOrderedMap<K, V>, num_to_allocate : u64
  ) -> Unit := do
    storage_slots_allocator::allocate_spare_slots(
      &mut self.nodes, num_to_allocate
    )

  spec allocate_spare_slots where
    pragma opaque
    pragma verify = false
    ensures self == old(self)
    ensures spec_iter_preserved(self, old(self))

  /--
  Returns true iff the BigOrderedMap is empty.
  -/
  @[map_is_empty (BigOrderedMap)]
  public fun is_empty {K has Store} {V has Store}(
    self : &BigOrderedMap<K, V>
  ) -> Bool := self.root.is_leaf && ordered_map::is_empty(&(self.root).children)

  spec is_empty where
    pragma intrinsic

  /--
  Returns the number of elements in the BigOrderedMap.
  This is an expensive function, as it goes through all the leaves to compute it.
  -/
  @[map_len (BigOrderedMap)]
  public fun compute_length {K has Store} {V has Store}(
    self : &BigOrderedMap<K, V>
  ) -> u64 := do
    let size := 0
    let self := self
    let iter := self.internal_leaf_new_begin_iter()
    while !iter.internal_leaf_iter_is_end() do
      let (node, next_iter) :=
        iter.internal_leaf_iter_borrow_entries_and_next_leaf_index(self)
      let children := node
      let _t := length(children)
      size := size + _t
      iter := next_iter
    where
      invariant spec_leaf_iter_valid(iter, self)
    return size

  spec compute_length where
    pragma intrinsic

  -- ======================= Section with Modifiers =========================
  /--
  Inserts the key/value into the BigOrderedMap.
  Aborts if the key is already in the map.
  -/
  @[map_add_no_override (BigOrderedMap)]
  public fun add {K has Copy, Drop, Store} {V has Store}(
    self : &mut BigOrderedMap<K, V>, key : K, value : V
  ) -> Unit := do
    destroy_none(self.add_or_upsert_impl(key, value, false))

  spec add where
    pragma intrinsic

  /--
  If the key doesn't exist in the map, inserts the key/value, and returns none.
  Otherwise updates the value under the given key, and returns the old value.
  -/
  @[map_upsert (BigOrderedMap)]
  public fun upsert {K has Copy, Drop, Store} {V has Store}(
    self : &mut BigOrderedMap<K, V>, key : K, value : V
  ) -> Option<V> := do
    let result := self.add_or_upsert_impl(key, value, true)
    return if is_some(&result) then
      let Child<V>::Leaf { value := old_value } := destroy_some(result)
      return some(old_value)
    else
      destroy_none(result)
      return none::<V>()

  spec upsert where
    pragma intrinsic

  /--
  Removes the entry from BigOrderedMap and returns the value which `key` maps to.
  Aborts if there is no entry for `key`.
  -/
  @[map_del_must_exist (BigOrderedMap)]
  public fun remove {K has Copy, Drop, Store} {V has Store}(
    self : &mut BigOrderedMap<K, V>, key : &K
  ) -> V := do
    if self.root.is_leaf then
      let Child<V>::Leaf { value := value } :=
        ordered_map::remove(&mut (self.root).children, key)
      return value;
    let path_to_leaf := self.find_leaf_path(key)
    assert!(!path_to_leaf.is_empty(), invalid_argument(EKEY_NOT_FOUND))
    let old_leaf :=
      do
        let (self, path_to_node, key) := (self, path_to_leaf, key)
        return self.remove_at_with_iter_hint(
          path_to_node, key, none::<ordered_map::IteratorPtr>()
        )
    assert!(is_some(&old_leaf), invalid_argument(EKEY_NOT_FOUND))
    let Child<V>::Leaf { value := value } := destroy_some(old_leaf)
    return value

  spec remove where
    pragma intrinsic

  -- Optimize case where only root node exists
  -- (optimizes out borrowing and path creation in `find_leaf_path`)
  /--
  Removes the entry from BigOrderedMap and returns the value which `key` maps to.
  Returns none if there is no entry for `key`.
  -/
  @[map_remove_or_none (BigOrderedMap)]
  public fun remove_or_none {K has Copy, Drop, Store} {V has Store}(
    self : &mut BigOrderedMap<K, V>, key : &K
  ) -> Option<V> := do
    if self.root.is_leaf then
      let value_option :=
        ordered_map::remove_or_none(&mut (self.root).children, key)
      if is_some(&value_option) then
        let Child<V>::Leaf { value := value } := destroy_some(value_option)
        return some(value);
      else
        destroy_none(value_option)
        return none::<V>();
    let path_to_leaf := self.find_leaf_path(key)
    return if path_to_leaf.is_empty() then none::<V>()
    else
      let old_leaf :=
        do
          let (self, path_to_node, key) := (self, path_to_leaf, key)
          return self.remove_at_with_iter_hint(
            path_to_node, key, none::<ordered_map::IteratorPtr>()
          )
      let self := old_leaf
      return if is_some(&self) then
        some(
          do
            let child := destroy_some(self)
            spec assume save_state_anchor!(54)
            let Child<V>::Leaf { value := value } := child
            return value)
      else
        destroy_none(self)
        return none::<V>()

  spec remove_or_none where
    pragma intrinsic

  -- Optimize case where only root node exists
  -- (optimizes out borrowing and path creation in `find_leaf_path`)
  fun __lambda__1__test_verify_modify(v : &mut u64) -> Bool := do
    let v := v
    let v := v
    *v := 11
    return true

  -- /// If value exists, calls modify_f on it, which returns tuple (to_keep, result).
  -- /// If to_keep is false, value is deleted from the map, and option::some(result) is returned.
  -- /// This function cannot be inline, due to iter_modify requiring actual function value.
  -- /// This also is why we return a value
  -- public fun modify_or_remove_if_present_and_return<K: drop + copy + store, V: store, R>(self: &mut BigOrderedMap<K, V>, key: &K, modify_f: |&mut V|(R, bool) has drop): Option<R> {
  --     let iter = self.find(key);
  --     if (iter.iter_is_end(self)) {
  --         option::none()
  --     } else {
  --         let (result, keep) = iter.iter_modify(self, modify_f);
  --         if (!keep) {
  --             iter.iter_remove(self);
  --         };
  --         option::some(result)
  --     }
  -- }
  /--
  Add multiple key/value pairs to the map. The keys must not already exist.
  Aborts with EKEY_ALREADY_EXISTS if key already exist, or duplicate keys are passed in.
  -/
  @[map_add_all (BigOrderedMap)]
  public fun add_all {K has Copy, Drop, Store} {V has Store}(
    self : &mut BigOrderedMap<K, V>, keys : Vector<K>, values : Vector<V>
  ) -> Unit := do
    let mut (self', v2) := (keys, values)
    self'.reverse()
    v2.reverse()
    let mut (self', v2) := (self', v2)
    spec assume folds_capture_anchor!(50)
    let len := self'.length
    assert!(len == v2.length, 131074)
    while len > 0 do
      let (e1, e2) := (self'.pop_back(), v2.pop_back())
      let (key, value) := (e1, e2)
      self.add(key, value)
      len := len - 1
    where
      invariant with_state_anchor!(50, old(self')).length >= len
      invariant len == self'.length
      invariant len == v2.length
      invariant with_state_anchor!(50, old(self')).length
        == with_state_anchor!(50, old(v2)).length
      invariant ∀ (j in 0 .. len),
        self'[j] == with_state_anchor!(50, old(self'))[j]
      invariant ∀ (j in 0 .. len), v2[j] == with_state_anchor!(50, old(v2))[j]
      invariant ∀ (j in len .. with_state_anchor!(50, old(self')).length), true
      invariant true
    self'.destroy_empty()
    v2.destroy_empty()

  spec add_all where
    pragma intrinsic

  -- TODO: Can be optimized, both in insertion order (largest first, then from smallest),
  -- as well as on initializing inner_max_degree/leaf_max_degree better
  @[map_pop_front (BigOrderedMap)]
  public fun pop_front {K has Copy, Drop, Store} {V has Store}(
    self : &mut BigOrderedMap<K, V>
  ) -> (K, V) := do
    let it := self.internal_new_begin_iter()
    let k := *it.iter_borrow_key()
    let v := self.remove(&k)
    return (k, v)

  spec pop_front where
    pragma intrinsic

  @[map_pop_back (BigOrderedMap)]
  public fun pop_back {K has Copy, Drop, Store} {V has Store}(
    self : &mut BigOrderedMap<K, V>
  ) -> (K, V) := do
    let it := self.internal_new_end_iter().iter_prev(self)
    let k := *it.iter_borrow_key()
    let v := self.remove(&k)
    return (k, v)

  spec pop_back where
    pragma intrinsic

  -- ============================= Accessors ================================
  /--
  Warning: Marked as internal, as it is safer to utilize provided inline functions instead.
  For direct usage of this method, check Warning at the top of the file corresponding to iterators.

  Returns an iterator pointing to the first element that is greater or equal to the provided
  key, or an end iterator if such element doesn't exist.
  -/
  public fun internal_lower_bound {K has Copy, Drop, Store} {V has Store}(
    self : &BigOrderedMap<K, V>, key : &K
  ) -> IteratorPtr<K> := do
    let leaf := self.find_leaf(key)
    if leaf == NULL_INDEX then return self.internal_new_end_iter();
    let node :=
      do
        let (self, node_index) := (self, leaf)
        return (if node_index == ROOT_INDEX then &self.root
        else storage_slots_allocator::borrow(&self.nodes, node_index))
    assert!(node.is_leaf, invalid_state(EINTERNAL_INVARIANT_BROKEN))
    let children := &node.children
    let child_lower_bound := ordered_map::internal_lower_bound(children, key)
    return if ordered_map::iter_is_end(&child_lower_bound, children) then
      self.internal_new_end_iter()
    else
      let iter_key :=
        *ordered_map::iter_borrow_key(&child_lower_bound, children)
      return new_iter(leaf, child_lower_bound, iter_key)

  spec internal_lower_bound where
    pragma opaque
    pragma verify = false
    aborts_if false
    ensures spec_iter_current(result, self)
    ensures result.iter_is_end(self)
        <==> (∀ (k : K),
          spec_contains_key(self, k)
            ==> compare(k, key) == new Ordering::Less {})
    ensures !result.iter_is_end(self) ==> spec_contains_key(self, result.key)
    ensures !result.iter_is_end(self)
        ==> compare(result.key, key) != new Ordering::Less {}
    ensures !result.iter_is_end(self)
        ==> (∀ (k : K),
          spec_contains_key(self, k) && compare(k, key) != new Ordering::Less {}
            ==> compare(result.key, k) != new Ordering::Greater {})

  /--
  Warning: Marked as internal, as it is safer to utilize provided inline functions instead.
  For direct usage of this method, check Warning at the top of the file corresponding to iterators.

  Returns an iterator pointing to the element that equals to the provided key, or an end
  iterator if the key is not found.
  -/
  public fun internal_find {K has Copy, Drop, Store} {V has Store}(
    self : &BigOrderedMap<K, V>, key : &K
  ) -> IteratorPtr<K> := do
    let internal_lower_bound := self.internal_lower_bound(key)
    return if internal_lower_bound.iter_is_end(self) then internal_lower_bound
    else
      if &internal_lower_bound.key == key then internal_lower_bound
      else self.internal_new_end_iter()

  spec internal_find where
    pragma opaque
    pragma verify = false
    aborts_if false
    ensures spec_iter_current(result, self)
    ensures result.iter_is_end(self) <==> !spec_contains_key(self, key)
    ensures !result.iter_is_end(self) ==> result.key == key

  /--
  Warning: Marked as internal, as it is safer to utilize provided inline functions instead.
  For direct usage of this method, check Warning at the top of the file corresponding to iterators.
  -/
  public fun internal_find_with_path {K has Copy, Drop, Store} {V has Store}(
    self : &BigOrderedMap<K, V>, key : &K
  ) -> IteratorPtrWithPath<K> := do
    let leaf_path := self.find_leaf_path(key)
    if leaf_path.is_empty() then
      return new IteratorPtrWithPath<K> {
        iterator := self.internal_new_end_iter(), path := vector<u64>[]
      };
    let leaf := leaf_path[leaf_path.length - 1]
    let node :=
      do
        let (self, node_index) := (self, leaf)
        return (if node_index == ROOT_INDEX then &self.root
        else storage_slots_allocator::borrow(&self.nodes, node_index))
    assert!(node.is_leaf, invalid_state(EINTERNAL_INVARIANT_BROKEN))
    let child_lower_bound :=
      ordered_map::internal_lower_bound(&node.children, key)
    return if ordered_map::iter_is_end(&child_lower_bound, &node.children) then
      new IteratorPtrWithPath<K> {
        iterator := self.internal_new_end_iter(), path := vector<u64>[]
      }
    else
      let iter_key :=
        *ordered_map::iter_borrow_key(&child_lower_bound, &node.children)
      return if &iter_key == key then
        new IteratorPtrWithPath<K> {
          iterator := new_iter(leaf, child_lower_bound, iter_key),
          path := leaf_path
        }
      else
        new IteratorPtrWithPath<K> {
          iterator := self.internal_new_end_iter(), path := vector<u64>[]
        }

  spec internal_find_with_path where
    pragma opaque
    pragma verify = false
    aborts_if false
    ensures spec_iter_current(result.iterator, self)
    ensures result.iterator.iter_is_end(self) <==> !spec_contains_key(self, key)
    ensures !result.iterator.iter_is_end(self) ==> result.iterator.key == key

  public fun iter_with_path_get_iter {K has Copy, Drop, Store}(
    self : &IteratorPtrWithPath<K>
  ) -> IteratorPtr<K> := self.iterator

  spec iter_with_path_get_iter where
    aborts_if false
    ensures result == self.iterator

  /--
  Returns true iff the key exists in the map.
  -/
  @[map_has_key (BigOrderedMap)]
  public fun contains {K has Copy, Drop, Store} {V has Store}(
    self : &BigOrderedMap<K, V>, key : &K
  ) -> Bool := do
    let internal_lower_bound := self.internal_lower_bound(key)
    return if internal_lower_bound.iter_is_end(self) then false
    else &internal_lower_bound.key == key

  spec contains where
    pragma intrinsic

  /--
  Returns a reference to the element with its key, aborts if the key is not found.
  -/
  @[map_borrow (BigOrderedMap)]
  public fun borrow {K has Copy, Drop, Store} {V has Store}(
    self : &BigOrderedMap<K, V>, key : &K
  ) -> &V := do
    let iter := self.internal_find(key)
    assert!(!iter.iter_is_end(self), invalid_argument(EKEY_NOT_FOUND))
    return iter.iter_borrow(self)

  spec borrow where
    pragma intrinsic

  @[map_get (BigOrderedMap)]
  public fun get {K has Copy, Drop, Store} {V has Copy, Store}(
    self : &BigOrderedMap<K, V>, key : &K
  ) -> Option<V> := do
    let iter := self.internal_find(key)
    return if iter.iter_is_end(self) then none::<V>()
    else some(*iter.iter_borrow(self))

  spec get where
    pragma intrinsic

  /--
  Returns a mutable reference to the element with its key at the given index, aborts if the key is not found.
  Aborts with EBORROW_MUT_REQUIRES_CONSTANT_VALUE_SIZE if KV size doesn't have constant size,
  because if it doesn't we cannot assert invariants on the size.
  In case of variable size, use either `borrow`, `copy` then `upsert`, or `remove` and `add` instead of mutable borrow.
  -/
  @[map_borrow_mut (BigOrderedMap)]
  public fun borrow_mut {K has Copy, Drop, Store} {V has Store}(
    self : &mut BigOrderedMap<K, V>, key : &K
  ) -> &mut V := do
    let iter := self.internal_find(key)
    assert!(!iter.iter_is_end(self), invalid_argument(EKEY_NOT_FOUND))
    return iter.iter_borrow_mut(self)

  spec borrow_mut where
    pragma intrinsic

  @[map_borrow_front (BigOrderedMap)]
  public fun borrow_front {K has Copy, Drop, Store} {V has Store}(
    self : &BigOrderedMap<K, V>
  ) -> (K, &V) := do
    let it := self.internal_new_begin_iter()
    let key := *it.iter_borrow_key()
    return (key, it.iter_borrow(self))

  spec borrow_front where
    pragma intrinsic

  @[map_front_key (BigOrderedMap)]
  public fun front_key {K has Copy, Drop, Store} {V has Store}(
    self : &BigOrderedMap<K, V>
  ) -> K := do
    let it := self.internal_new_begin_iter()
    return *it.iter_borrow_key()

  spec front_key where
    pragma intrinsic

  @[map_borrow_back (BigOrderedMap)]
  public fun borrow_back {K has Copy, Drop, Store} {V has Store}(
    self : &BigOrderedMap<K, V>
  ) -> (K, &V) := do
    let it := self.internal_new_end_iter().iter_prev(self)
    let key := *it.iter_borrow_key()
    return (key, it.iter_borrow(self))

  spec borrow_back where
    pragma intrinsic

  @[map_back_key (BigOrderedMap)]
  public fun back_key {K has Copy, Drop, Store} {V has Store}(
    self : &BigOrderedMap<K, V>
  ) -> K := do
    let it := self.internal_new_end_iter().iter_prev(self)
    return *it.iter_borrow_key()

  spec back_key where
    pragma intrinsic

  @[map_prev_key (BigOrderedMap)]
  public fun prev_key {K has Copy, Drop, Store} {V has Store}(
    self : &BigOrderedMap<K, V>, key : &K
  ) -> Option<K> := do
    let it := self.internal_lower_bound(key)
    return if it.iter_is_begin(self) then none::<K>()
    else some(*it.iter_prev(self).iter_borrow_key())

  spec prev_key where
    pragma intrinsic

  @[map_next_key (BigOrderedMap)]
  public fun next_key {K has Copy, Drop, Store} {V has Store}(
    self : &BigOrderedMap<K, V>, key : &K
  ) -> Option<K> := do
    let it := self.internal_lower_bound(key)
    return if it.iter_is_end(self) then none::<K>()
    else
      let cur_key := it.iter_borrow_key()
      return if key == cur_key then
        let it := it.iter_next(self)
        return if it.iter_is_end(self) then none::<K>()
        else some(*it.iter_borrow_key())
      else some(*cur_key)

  spec next_key where
    pragma intrinsic

  -- =========================== Views and Traversals ==============================
  /--
  Convert a BigOrderedMap to an OrderedMap, which is supposed to be called mostly by view functions to get an atomic
  view of the whole map.
  Disclaimer: This function may be costly as the BigOrderedMap may be huge in size. Use it at your own discretion.
  -/
  @[map_to_ordered_map (BigOrderedMap)]
  public fun to_ordered_map {K has Copy, Drop, Store} {V has Copy, Store}(
    self : &BigOrderedMap<K, V>
  ) -> OrderedMap<K, V> := do
    let mut result := ordered_map::new::<K, V>()
    let self := self
    let self := self
    let iter := self.internal_leaf_new_begin_iter()
    while !iter.internal_leaf_iter_is_end() do
      let (node, next_iter) :=
        iter.internal_leaf_iter_borrow_entries_and_next_leaf_index(self)
      let children := node
      let self := children
      let iter' := ordered_map::internal_new_begin_iter(self)
      while !ordered_map::iter_is_end(&iter', self) do
        let (k, v) :=
          (ordered_map::iter_borrow_key(&iter', self),
            ordered_map::iter_borrow(iter', self))
        let (k, v) := (k, v.internal_leaf_borrow_value())
        iter_add(
          ordered_map::internal_new_end_iter(&result), &mut result, *k, *v
        )
        iter' := ordered_map::iter_next(iter', self)
      iter := next_iter
    where
      invariant spec_leaf_iter_valid(iter, self)
    return result

  spec to_ordered_map where
    pragma intrinsic

  /--
  Get all keys.

  For a large enough BigOrderedMap this function will fail due to execution gas limits,
  use iterartor or next_key/prev_key to iterate over across portion of the map.
  -/
  @[map_keys (BigOrderedMap)]
  public fun keys {K has Copy, Drop, Store} {V has Store}(
    self : &BigOrderedMap<K, V>
  ) -> Vector<K> := do
    let mut result := vector<K>[]
    let self := self
    let self := self
    let iter := self.internal_leaf_new_begin_iter()
    while !iter.internal_leaf_iter_is_end() do
      let (node, next_iter) :=
        iter.internal_leaf_iter_borrow_entries_and_next_leaf_index(self)
      let children := node
      let self := children
      let iter' := ordered_map::internal_new_begin_iter(self)
      while !ordered_map::iter_is_end(&iter', self) do
        let (k, v) :=
          (ordered_map::iter_borrow_key(&iter', self),
            ordered_map::iter_borrow(iter', self))
        let (k, _v) := (k, v.internal_leaf_borrow_value())
        result := core.prim.pushVector(result, *k)
        iter' := ordered_map::iter_next(iter', self)
      iter := next_iter
    where
      invariant spec_leaf_iter_valid(iter, self)
    return result

  spec keys where
    pragma intrinsic

  -- ========================= IteratorPtr functions ===========================
  /--
  Warning: Marked as internal, as it is safer to utilize provided inline functions instead.
  For direct usage of this method, check Warning at the top of the file corresponding to iterators.

  Returns the begin iterator.
  -/
  public fun internal_new_begin_iter {K has Copy, Store} {V has Store}(
    self : &BigOrderedMap<K, V>
  ) -> IteratorPtr<K> := do
    if self.is_empty() then return new IteratorPtr<K>::End {};
    let node :=
      do
        let (self, node_index) := (self, self.min_leaf_index)
        return (if node_index == ROOT_INDEX then &self.root
        else storage_slots_allocator::borrow(&self.nodes, node_index))
    assert!(
      !ordered_map::is_empty(&node.children),
      invalid_state(EINTERNAL_INVARIANT_BROKEN)
    )
    let begin_child_iter := ordered_map::internal_new_begin_iter(&node.children)
    let begin_child_key :=
      *ordered_map::iter_borrow_key(&begin_child_iter, &node.children)
    return new_iter(self.min_leaf_index, begin_child_iter, begin_child_key)

  spec internal_new_begin_iter where
    pragma opaque
    pragma verify = false
    aborts_if false
    ensures spec_iter_current(result, self)
    ensures result.iter_is_end(self) <==> spec_len(self) == 0
    ensures !result.iter_is_end(self) ==> spec_contains_key(self, result.key)
    ensures !result.iter_is_end(self)
        ==> (∀ (k : K),
          spec_contains_key(self, k) && k != result.key
            ==> compare(result.key, k) == new Ordering::Less {})
    ensures !result.iter_is_end(self) ==> spec_rank(self, result.key) == 0

  /--
  Warning: Marked as internal, as it is safer to utilize provided inline functions instead.
  For direct usage of this method, check Warning at the top of the file corresponding to iterators.

  Returns the end iterator.
  -/
  public fun internal_new_end_iter {K has Store} {V has Store}(
    self : &BigOrderedMap<K, V>
  ) -> IteratorPtr<K> := new IteratorPtr<K>::End {}

  spec internal_new_end_iter where
    pragma opaque
    pragma verify = false
    aborts_if false
    ensures result is End

  -- Returns true iff the iterator is a begin iterator.
  public fun iter_is_begin {K has Store} {V has Store}(
    self : &IteratorPtr<K>, map : &BigOrderedMap<K, V>
  ) -> Bool :=
    if self is End then map.is_empty()
    else
      self.node_index == map.min_leaf_index
        && iter_is_begin_from_non_empty(&self.child_iter)

  spec iter_is_begin where
    pragma opaque
    pragma verify = false
    requires spec_iter_valid(self, map)
    aborts_if false
    ensures result <==> spec_iter_is_begin(self, map)
    ensures result && !(self is End) ==> spec_rank(map, self.key) == 0

  -- Returns true iff the iterator is an end iterator.
  public fun iter_is_end {K has Store} {V has Store}(
    self : &IteratorPtr<K>, _map : &BigOrderedMap<K, V>
  ) -> Bool := self is End

  spec iter_is_end where
    pragma opaque
    pragma verify = false
    aborts_if false
    ensures result == (self is End)

  /--
  Borrows the key given iterator points to.
  Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the end.
  Note: Requires that the map is not changed after the input iterator is generated.
  -/
  public fun iter_borrow_key {K}(self : &IteratorPtr<K>) -> &K := do
    assert!(!(self is End), invalid_argument(EITER_OUT_OF_BOUNDS))
    return &self.key

  spec iter_borrow_key where
    pragma opaque
    pragma verify = false
    aborts_if self is End
    ensures result == self.key

  /--
  Borrows the value given iterator points to.
  Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the end.
  Note: Requires that the map is not changed after the input iterator is generated.
  -/
  public fun iter_borrow {K has Drop, Store} {V has Store}(
    self : IteratorPtr<K>, map : &BigOrderedMap<K, V>
  ) -> &V := do
    assert!(!self.iter_is_end(map), invalid_argument(EITER_OUT_OF_BOUNDS))
    let IteratorPtr<K>::Some { node_index := node_index,
    child_iter := child_iter,
    key := _ } :=
      self
    let children :=
      &(do
        let (self, node_index) := (map, node_index)
        return (if node_index == ROOT_INDEX then &self.root
        else storage_slots_allocator::borrow(&self.nodes, node_index))).children
    return &ordered_map::iter_borrow(child_iter, children).value

  spec iter_borrow where
    pragma opaque
    pragma verify = false
    requires spec_iter_valid(self, map)
    aborts_if self.iter_is_end(map)
    ensures result == spec_get(map, self.key)

  /--
  Mutably borrows the value iterator points to.
  Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the end.
  Aborts with EBORROW_MUT_REQUIRES_CONSTANT_VALUE_SIZE if KV size doesn't have constant size,
  because if it doesn't we cannot assert invariants on the size.
  In case of variable size, use either `borrow`, `copy` then `upsert`, or `remove` and `add` instead of mutable borrow.

  Note: Requires that the map is not changed after the input iterator is generated.
  -/
  @[map_iter_borrow_mut (BigOrderedMap)]
  public fun iter_borrow_mut {K has Drop, Store} {V has Store}(
    self : IteratorPtr<K>, map : &mut BigOrderedMap<K, V>
  ) -> &mut V := do
    assert!(
      map.constant_kv_size || is_some(&constant_serialized_size::<V>()),
      invalid_argument(EBORROW_MUT_REQUIRES_CONSTANT_VALUE_SIZE)
    )
    assert!(!self.iter_is_end(map), invalid_argument(EITER_OUT_OF_BOUNDS))
    let IteratorPtr<K>::Some { node_index := node_index,
    child_iter := child_iter,
    key := _ } :=
      self
    let children :=
      &mut (do
        let (self, node_index) := (map, node_index)
        return (if node_index == ROOT_INDEX then &mut self.root
        else storage_slots_allocator::borrow_mut(
          &mut self.nodes, node_index
        ))).children
    return &mut ordered_map::iter_borrow_mut(child_iter, children).value

  spec iter_borrow_mut where
    pragma intrinsic
    requires spec_iter_valid(self, map)

  public fun iter_modify {K has Drop, Store} {V has Store} {R}(
    self : IteratorPtr<K>, map : &mut BigOrderedMap<K, V>,
    f : Fn(&mut V) -> R
  ) -> R := do
    assert!(!self.iter_is_end(map), invalid_argument(EITER_OUT_OF_BOUNDS))
    let IteratorPtr<K>::Some { node_index := node_index,
    child_iter := child_iter,
    key := key } :=
      self
    let children :=
      &mut (do
        let (self, node_index) := (map, node_index)
        return (if node_index == ROOT_INDEX then &mut self.root
        else storage_slots_allocator::borrow_mut(
          &mut self.nodes, node_index
        ))).children
    let value_mut :=
      &mut ordered_map::iter_borrow_mut(child_iter, children).value
    let result := invoke(f, value_mut)
    if map.constant_kv_size then return result;
    let key_size := serialized_size(&key)
    let value_size := serialized_size(value_mut)
    map.validate_size_and_init_max_degrees(key_size, value_size)
    return result

  spec iter_modify where
    pragma opaque
    pragma verify = false
    requires self.iter_is_end(map) || requires_of<f>(spec_get(map, self.key))
    requires spec_iter_valid(self, map)
    aborts_if self.iter_is_end(map)
    aborts_if aborts_of<f>(spec_get(map, self.key))
    ensures spec_iter_preserved(map, old(map))
    ensures spec_contains_key(map, self.key)
    ensures spec_len(map) == spec_len(old(map))
    ensures spec_unchanged_except_at(map, self.key)
    ensures ensures_of<f>(
        old(spec_get(map, self.key)), result, spec_get(
          map, self.key
        )
      )
    ensures ∀ (i in 0 .. spec_len(map)),
        spec_key_at(map, i) == spec_key_at(old(map), i)
    ensures ∀ (k : K),
        spec_contains_key(old(map), k)
          ==> spec_rank(map, k) == spec_rank(old(map), k)

  -- validate that after modifications size invariants hold
  /--
  Removes the entry from BigOrderedMap and returns the value which `key` maps to.
  Aborts if there is no entry for `key`.
  -/
  public fun iter_remove {K has Copy, Drop, Store} {V has Store}(
    self : IteratorPtrWithPath<K>, map : &mut BigOrderedMap<K, V>
  ) -> V := do
    let IteratorPtrWithPath<K> { iterator := iter, path := path_to_leaf } :=
      self
    assert!(!iter.iter_is_end(map), invalid_argument(EITER_OUT_OF_BOUNDS))
    let IteratorPtr<K>::Some { node_index := _,
    child_iter := child_iter,
    key := key } :=
      iter
    if map.root.is_leaf then
      let Child<V>::Leaf { value := value } :=
        ordered_map::iter_remove(child_iter, &mut (map.root).children)
      return value;
    assert!(!path_to_leaf.is_empty(), invalid_argument(EKEY_NOT_FOUND))
    let old_leaf :=
      map.remove_at_with_iter_hint(path_to_leaf, &key, some(child_iter))
    assert!(is_some(&old_leaf), invalid_argument(EKEY_NOT_FOUND))
    let Child<V>::Leaf { value := value } := destroy_some(old_leaf)
    return value

  spec iter_remove where
    pragma opaque
    pragma verify = false
    requires spec_iter_valid(self.iterator, map)
    aborts_if self.iterator.iter_is_end(map)
    ensures result == spec_get(old(map), self.iterator.key)
    ensures !spec_contains_key(map, self.iterator.key)
    ensures spec_len(map) == spec_len(old(map)) - 1
    ensures spec_unchanged_except_at(map, self.iterator.key)
    ensures ∀ (i in 0 .. spec_rank(old(map), self.iterator.key)),
        spec_key_at(map, i) == spec_key_at(old(map), i)
    ensures ∀ (i in spec_rank(old(map), self.iterator.key) .. spec_len(map)),
        spec_key_at(map, i) == spec_key_at(old(map), i + 1)

  -- Optimize case where only root node exists
  -- (optimizes out borrowing and path creation in `find_leaf_path`)
  /--
  Returns the next iterator.
  Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the end.
  Requires the map is not changed after the input iterator is generated.
  -/
  public fun iter_next {K has Copy, Drop, Store} {V has Store}(
    self : IteratorPtr<K>, map : &BigOrderedMap<K, V>
  ) -> IteratorPtr<K> := do
    assert!(!(self is End), invalid_argument(EITER_OUT_OF_BOUNDS))
    let node_index := self.node_index
    let node :=
      do
        let (self, node_index) := (map, node_index)
        return (if node_index == ROOT_INDEX then &self.root
        else storage_slots_allocator::borrow(&self.nodes, node_index))
    let child_iter := ordered_map::iter_next(self.child_iter, &node.children)
    if !ordered_map::iter_is_end(&child_iter, &node.children) then
      let iter_key := *ordered_map::iter_borrow_key(&child_iter, &node.children)
      return new_iter(node_index, child_iter, iter_key);
    let next_index := node.next
    if next_index != NULL_INDEX then
      let next_node :=
        do
          let (self, node_index) := (map, next_index)
          return (if node_index == ROOT_INDEX then &self.root
          else storage_slots_allocator::borrow(&self.nodes, node_index))
      let child_iter :=
        ordered_map::internal_new_begin_iter(&next_node.children)
      assert!(
        !ordered_map::iter_is_end(&child_iter, &next_node.children),
        invalid_state(EINTERNAL_INVARIANT_BROKEN)
      )
      let iter_key :=
        *ordered_map::iter_borrow_key(&child_iter, &next_node.children)
      return new_iter(next_index, child_iter, iter_key);
    return map.internal_new_end_iter()

  spec iter_next where
    pragma opaque
    pragma verify = false
    requires spec_iter_valid(self, map)
    aborts_if self.iter_is_end(map)
    ensures spec_iter_current(result, map)
    ensures result is End
        <==> (∀ (k : K),
          spec_contains_key(map, k)
            ==> compare(k, self.key) != new Ordering::Greater {})
    ensures !(result is End) ==> spec_contains_key(map, result.key)
    ensures !(result is End)
        ==> compare(result.key, self.key) == new Ordering::Greater {}
    ensures !(result is End)
        ==> (∀ (k : K),
          spec_contains_key(map, k)
            && compare(k, self.key) == new Ordering::Greater {}
            ==> compare(result.key, k) != new Ordering::Greater {})
    ensures !(result is End) && spec_contains_key(map, self.key)
        ==> spec_rank(map, result.key) == spec_rank(map, self.key) + 1
    ensures result is End && spec_contains_key(map, self.key)
        ==> spec_rank(map, self.key) == spec_len(map) - 1

  -- next is in the same leaf node
  -- next is in a different leaf node
  /--
  Returns the previous iterator.
  Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the beginning.
  Requires the map is not changed after the input iterator is generated.
  -/
  public fun iter_prev {K has Copy, Drop, Store} {V has Store}(
    self : IteratorPtr<K>, map : &BigOrderedMap<K, V>
  ) -> IteratorPtr<K> := do
    let prev_index :=
      if self is End then map.max_leaf_index
      else
        let node_index := self.node_index
        let node :=
          do
            let (self, node_index) := (map, node_index)
            return (if node_index == ROOT_INDEX then &self.root
            else storage_slots_allocator::borrow(&self.nodes, node_index))
        if !ordered_map::iter_is_begin(&self.child_iter, &node.children) then
          let child_iter :=
            ordered_map::iter_prev(self.child_iter, &node.children)
          let key := *ordered_map::iter_borrow_key(&child_iter, &node.children)
          return new_iter(node_index, child_iter, key);
        return node.prev
    assert!(prev_index != NULL_INDEX, invalid_argument(EITER_OUT_OF_BOUNDS))
    let prev_node :=
      do
        let (self, node_index) := (map, prev_index)
        return (if node_index == ROOT_INDEX then &self.root
        else storage_slots_allocator::borrow(&self.nodes, node_index))
    let prev_children := &prev_node.children
    let child_iter :=
      ordered_map::iter_prev(
        ordered_map::internal_new_end_iter(prev_children), prev_children
      )
    let iter_key := *ordered_map::iter_borrow_key(&child_iter, prev_children)
    return new_iter(prev_index, child_iter, iter_key)

  spec iter_prev where
    pragma opaque
    pragma verify = false
    requires spec_iter_valid(self, map)
    aborts_if spec_iter_is_begin(self, map)
    ensures spec_iter_current(result, map)
    ensures !(result is End)
    ensures spec_contains_key(map, result.key)
    ensures self is End
        ==> (∀ (k : K),
          spec_contains_key(map, k) && k != result.key
            ==> compare(k, result.key) == new Ordering::Less {})
    ensures !(self is End)
        ==> compare(result.key, self.key) == new Ordering::Less {}
    ensures !(self is End)
        ==> (∀ (k : K),
          spec_contains_key(map, k)
            && compare(k, self.key) == new Ordering::Less {}
            ==> compare(k, result.key) != new Ordering::Greater {})
    ensures self is End ==> spec_rank(map, result.key) == spec_len(map) - 1
    ensures !(self is End) && spec_contains_key(map, self.key)
        ==> spec_rank(map, result.key) == spec_rank(map, self.key) - 1

  -- next is in the same leaf node
  -- next is in a different leaf node
  -- ====================== Internal Implementations ========================
  enum LeafNodeIteratorPtr has Copy, Drop where
    | NodeIndex (node_index : u64)

  public fun internal_leaf_new_begin_iter {K has Store} {V has Store}(
    self : &BigOrderedMap<K, V>
  ) -> LeafNodeIteratorPtr :=
    new LeafNodeIteratorPtr::NodeIndex { node_index := self.min_leaf_index }

  spec internal_leaf_new_begin_iter where
    pragma opaque
    pragma verify = false
    aborts_if false
    ensures spec_leaf_iter_valid(result, self)
    ensures !result.internal_leaf_iter_is_end()
    ensures spec_leaf_offset(result, self) == 0

  public fun internal_leaf_iter_is_end(self : &LeafNodeIteratorPtr) -> Bool :=
    self.node_index == NULL_INDEX

  spec internal_leaf_iter_is_end where
    pragma opaque
    aborts_if false
    ensures result == (self.node_index == NULL_INDEX)

  public fun internal_leaf_borrow_value {V has Store}(self : &Child<V>) -> &V :=
    &self.value

  spec internal_leaf_borrow_value where
    pragma opaque
    pragma verify = false
    aborts_if false
    ensures result == self.value

  public fun internal_leaf_iter_borrow_entries_and_next_leaf_index {K has Store} {V has Store}(
    mut self : LeafNodeIteratorPtr, map : &BigOrderedMap<K, V>
  ) -> (&OrderedMap<K, Child<V> >, LeafNodeIteratorPtr) := do
    assert!(self.node_index != NULL_INDEX, EITER_OUT_OF_BOUNDS)
    let node :=
      do
        let (self, node_index) := (map, self.node_index)
        return (if node_index == ROOT_INDEX then &self.root
        else storage_slots_allocator::borrow(&self.nodes, node_index))
    assert!(node.is_leaf, EINTERNAL_INVARIANT_BROKEN)
    self.node_index := node.next
    return (&node.children, self)

  spec internal_leaf_iter_borrow_entries_and_next_leaf_index where
    pragma opaque
    pragma verify = false
    requires spec_leaf_iter_valid(self, map)
    aborts_if self.internal_leaf_iter_is_end()
    ensures spec_leaf_iter_valid(spec.result[1], map)
    ensures ∀ (k : K),
        0x1::aptos_framework::ordered_map::spec_contains_key(result, k)
          ==> spec_contains_key(map, k)
    ensures ∀ (k : K),
        0x1::aptos_framework::ordered_map::spec_contains_key(result, k)
          ==> 0x1::aptos_framework::ordered_map::spec_get(result, k) is Leaf
            && 0x1::aptos_framework::ordered_map::spec_get(result, k).value
              == spec_get(map, k)
    ensures spec_len(map) > 0
        ==> 0x1::aptos_framework::ordered_map::spec_len(result) > 0
    ensures spec_leaf_offset(self, map)
        + 0x1::aptos_framework::ordered_map::spec_len(result)
        <= spec_len(map)
    ensures ∀ (j in 0 .. 0x1::aptos_framework::ordered_map::spec_len(result)),
        spec_key_at(map, spec_leaf_offset(self, map) + j)
          == 0x1::aptos_framework::ordered_map::spec_key_at(result, j)
    ensures ∀ (j in 0 .. 0x1::aptos_framework::ordered_map::spec_len(result)),
        0x1::aptos_framework::ordered_map::spec_get(
          result, 0x1::aptos_framework::ordered_map::spec_key_at(
            result, j
          )
        ) is Leaf
    ensures ∀ (j in 0 .. 0x1::aptos_framework::ordered_map::spec_len(result)),
        0x1::aptos_framework::ordered_map::spec_get(
          result, 0x1::aptos_framework::ordered_map::spec_key_at(
            result, j
          )
        ).value
          == spec_get(map, spec_key_at(map, spec_leaf_offset(self, map) + j))
    ensures spec_leaf_offset(spec.result[1], map)
        == spec_leaf_offset(self, map)
          + 0x1::aptos_framework::ordered_map::spec_len(result)
    ensures spec_leaf_offset(spec.result[1], map) >= 0
    ensures spec.result[1].internal_leaf_iter_is_end()
        ==> spec_leaf_offset(spec.result[1], map) == spec_len(map)

  fun add_or_upsert_impl {K has Copy, Drop, Store} {V has Store}(
    self : &mut BigOrderedMap<K, V>, key : K, value : V,
    allow_overwrite : Bool
  ) -> Option<Child<V> > := do
    if !self.constant_kv_size then
      self.validate_dynamic_size_and_init_max_degrees(&key, &value)
    if self.root.is_leaf then
      let children := &mut (self.root).children
      let degree := length(children)
      if degree < self.leaf_max_degree as u64 then
        let result := ordered_map::upsert(children, key, new_leaf_child(value))
        assert!(
          allow_overwrite || is_none(&result),
          invalid_argument(EKEY_ALREADY_EXISTS)
        )
        return result;
    let mut path_to_leaf := self.find_leaf_path(&key)
    if path_to_leaf.is_empty() then
      let current := ROOT_INDEX
      loop do
        path_to_leaf := core.prim.pushVector(path_to_leaf, current)
        let current_node :=
          do
            let (self, node_index) := (self, current)
            return (if node_index == ROOT_INDEX then &mut self.root
            else storage_slots_allocator::borrow_mut(
              &mut self.nodes, node_index
            ))
        if current_node.is_leaf then break
        let last_value :=
          ordered_map::iter_remove(
            ordered_map::iter_prev(
              ordered_map::internal_new_end_iter(&current_node.children),
              &current_node.children
            ),
            &mut current_node.children
          )
        current := stored_to_index(&last_value.node_index)
        ordered_map::add(&mut current_node.children, key, last_value)
    return self.add_at(
      path_to_leaf, key, new_leaf_child(
        value
      ), allow_overwrite
    )

  -- Optimize case where only root node exists
  -- (optimizes out borrowing and path creation in `find_leaf_path`)
  -- In this case, the key is greater than all keys in the map.
  -- So we need to update `key` in the pointers to the last (rightmost) child
  -- on every level, to maintain the invariant of `add_at`
  -- we also create a path_to_leaf to the rightmost leaf.
  fun validate_dynamic_size_and_init_max_degrees {K has Store} {V has Store}(
    self : &mut BigOrderedMap<K, V>, key : &K, value : &V
  ) -> Unit := do
    let key_size := serialized_size(key)
    let value_size := serialized_size(value)
    self.validate_size_and_init_max_degrees(key_size, value_size)

  spec validate_dynamic_size_and_init_max_degrees where
    pragma opaque
    pragma verify = false

  fun validate_static_size_and_init_max_degrees {K has Store} {V has Store}(
    self : &mut BigOrderedMap<K, V>
  ) -> Unit := do
    let key_size := constant_serialized_size::<K>()
    let value_size := constant_serialized_size::<V>()
    if is_some(&key_size) then
      let key_size := destroy_some(key_size)
      if self.inner_max_degree == 0u16 then
        self.inner_max_degree := max(
          min(MAX_DEGREE, DEFAULT_TARGET_NODE_SIZE / key_size),
          INNER_MIN_DEGREE as u64
        ) as u16
      assert!(
        key_size * (self.inner_max_degree as u64) <= MAX_NODE_BYTES,
        invalid_argument(EKEY_BYTES_TOO_LARGE)
      )
      if is_some(&value_size) then
        let value_size := destroy_some(value_size)
        let entry_size := key_size + value_size
        if self.leaf_max_degree == 0u16 then
          self.leaf_max_degree := max(
            min(MAX_DEGREE, DEFAULT_TARGET_NODE_SIZE / entry_size),
            LEAF_MIN_DEGREE as u64
          ) as u16
        assert!(
          entry_size * (self.leaf_max_degree as u64) <= MAX_NODE_BYTES,
          invalid_argument(EARGUMENT_BYTES_TOO_LARGE)
        )
        self.constant_kv_size := true

  spec validate_static_size_and_init_max_degrees where
    pragma opaque
    pragma verify = false

  fun validate_size_and_init_max_degrees {K has Store} {V has Store}(
    self : &mut BigOrderedMap<K, V>, key_size : u64, value_size : u64
  ) -> Unit := do
    let entry_size := key_size + value_size
    if self.inner_max_degree == 0u16 then
      let default_max_degree :=
        min(MAX_DEGREE, MAX_NODE_BYTES / DEFAULT_MAX_KEY_OR_VALUE_SIZE)
      self.inner_max_degree := max(
        min(default_max_degree, DEFAULT_TARGET_NODE_SIZE / key_size),
        INNER_MIN_DEGREE as u64
      ) as u16
    if self.leaf_max_degree == 0u16 then
      let default_max_degree :=
        min(MAX_DEGREE, MAX_NODE_BYTES / DEFAULT_MAX_KEY_OR_VALUE_SIZE / 2)
      self.leaf_max_degree := max(
        min(default_max_degree, DEFAULT_TARGET_NODE_SIZE / entry_size),
        LEAF_MIN_DEGREE as u64
      ) as u16
    assert!(
      key_size * (self.inner_max_degree as u64) <= MAX_NODE_BYTES,
      invalid_argument(EKEY_BYTES_TOO_LARGE)
    )
    assert!(
      entry_size * (self.leaf_max_degree as u64) <= MAX_NODE_BYTES,
      invalid_argument(EARGUMENT_BYTES_TOO_LARGE)
    )

  spec validate_size_and_init_max_degrees where
    pragma opaque
    pragma verify = false

  -- Make sure that no nodes can exceed the upper size limit.
  fun destroy_inner_child {V has Store}(self : Child<V>) -> StoredSlot := do
    let Child<V>::Inner { node_index := node_index } := self
    return node_index

  fun destroy_empty_node {K has Store} {V has Store}(
    self : Node<K, V>
  ) -> Unit := do
    let Node<K, V>::V1 { is_leaf := _,
    children := children,
    prev := _,
    next := _ } :=
      self
    assert!(ordered_map::is_empty(&children), invalid_argument(EMAP_NOT_EMPTY))
    ordered_map::destroy_empty(children)

  fun new_node {K has Store} {V has Store}(is_leaf : Bool) -> Node<K, V> :=
    new Node<K, V>::V1 {
      is_leaf, children := ordered_map::new::<K, Child<V> >(),
      prev := NULL_INDEX, next := NULL_INDEX
    }

  fun new_node_with_children {K has Store} {V has Store}(
    is_leaf : Bool, children : OrderedMap<K, Child<V> >
  ) -> Node<K, V> :=
    new Node<K, V>::V1 {
      is_leaf, children, prev := NULL_INDEX, next := NULL_INDEX
    }

  fun new_inner_child {V has Store}(node_index : StoredSlot) -> Child<V> :=
    new Child<V>::Inner { node_index }

  fun new_leaf_child {V has Store}(value : V) -> Child<V> :=
    new Child<V>::Leaf { value }

  fun new_iter {K}(
    node_index : u64, child_iter : ordered_map::IteratorPtr, key : K
  ) -> IteratorPtr<K> := new IteratorPtr<K>::Some {
    node_index, child_iter, key
  }

  /--
  Find leaf where the given key would fall in.
  So the largest leaf with its `max_key <= key`.
  return NULL_INDEX if `key` is larger than any key currently stored in the map.
  -/
  fun find_leaf {K has Store} {V has Store}(
    self : &BigOrderedMap<K, V>, key : &K
  ) -> u64 := do
    let current := ROOT_INDEX
    loop do
      let node :=
        do
          let (self, node_index) := (self, current)
          return (if node_index == ROOT_INDEX then &self.root
          else storage_slots_allocator::borrow(&self.nodes, node_index))
      if node.is_leaf then return current;
      let children := &node.children
      let child_iter := ordered_map::internal_lower_bound(children, key)
      if ordered_map::iter_is_end(&child_iter, children) then
        return NULL_INDEX;
      else
        current := stored_to_index(
          &ordered_map::iter_borrow(child_iter, children).node_index
        )

  /--
  Find leaf where the given key would fall in.
  So the largest leaf with it's `max_key <= key`.
  Returns the path from root to that leaf (including the leaf itself)
  Returns empty path if `key` is larger than any key currently stored in the map.
  -/
  fun find_leaf_path {K has Store} {V has Store}(
    self : &BigOrderedMap<K, V>, key : &K
  ) -> Vector<u64> := do
    let mut vec := vector<u64>[]
    let current := ROOT_INDEX
    loop do
      vec := core.prim.pushVector(vec, current)
      let node :=
        do
          let (self, node_index) := (self, current)
          return (if node_index == ROOT_INDEX then &self.root
          else storage_slots_allocator::borrow(&self.nodes, node_index))
      if node.is_leaf then return vec;
      let children := &node.children
      let child_iter := ordered_map::internal_lower_bound(children, key)
      if ordered_map::iter_is_end(&child_iter, children) then
        return vector<u64>[];
      else
        current := stored_to_index(
          &ordered_map::iter_borrow(child_iter, children).node_index
        )

  fun get_max_degree {K has Store} {V has Store}(
    self : &BigOrderedMap<K, V>, leaf : Bool
  ) -> u64 :=
    if leaf then self.leaf_max_degree as u64 else self.inner_max_degree as u64

  fun replace_root {K has Store} {V has Store}(
    self : &mut BigOrderedMap<K, V>, mut new_root : Node<K, V>
  ) -> Node<K, V> := do
    let root := &mut self.root
    let tmp_is_leaf := root.is_leaf
    root.is_leaf := new_root.is_leaf
    new_root.is_leaf := tmp_is_leaf
    assert!(root.prev == NULL_INDEX, invalid_state(EINTERNAL_INVARIANT_BROKEN))
    assert!(root.next == NULL_INDEX, invalid_state(EINTERNAL_INVARIANT_BROKEN))
    assert!(
      new_root.prev == NULL_INDEX, invalid_state(
        EINTERNAL_INVARIANT_BROKEN
      )
    )
    assert!(
      new_root.next == NULL_INDEX, invalid_state(
        EINTERNAL_INVARIANT_BROKEN
      )
    )
    let tmp_children := trim(&mut root.children, 0)
    append_disjoint(&mut root.children, trim(&mut new_root.children, 0))
    append_disjoint(&mut new_root.children, tmp_children)
    return new_root

  -- TODO: once mem::replace is made public/released, update to:
  -- mem::replace(&mut self.root, new_root_node)
  -- let tmp_prev = root.prev;
  -- root.prev = new_root.prev;
  -- new_root.prev = tmp_prev;
  -- let tmp_next = root.next;
  -- root.next = new_root.next;
  -- new_root.next = tmp_next;
  /--
  Add a given child to a given node (last in the `path_to_node`), and update/rebalance the tree as necessary.
  It is required that `key` pointers to the child node, on the `path_to_node` are greater or equal to the given key.
  That means if we are adding a `key` larger than any currently existing in the map - we needed
  to update `key` pointers on the `path_to_node` to include it, before calling this method.

  Returns Child previously associated with the given key.
  If `allow_overwrite` is not set, function will abort if `key` is already present.
  -/
  fun add_at {K has Copy, Drop, Store} {V has Store}(
    self : &mut BigOrderedMap<K, V>, mut path_to_node : Vector<u64>, key : K,
    child : Child<V>, allow_overwrite : Bool
  ) -> Option<Child<V> > := do
    let node_index := path_to_node.pop_back()
    let node :=
      do
        let (self, node_index) := (self, node_index)
        return (if node_index == ROOT_INDEX then &mut self.root
        else storage_slots_allocator::borrow_mut(&mut self.nodes, node_index))
    let children := &mut node.children
    let degree := length(children)
    let max_degree :=
      if node.is_leaf then self.leaf_max_degree as u64
      else self.inner_max_degree as u64
    if degree < max_degree then
      let old_child := ordered_map::upsert(children, key, child)
      if node.is_leaf then
        assert!(
          allow_overwrite || is_none(&old_child),
          invalid_argument(EKEY_ALREADY_EXISTS)
        )
        return old_child;
      else
        assert!(
          !allow_overwrite && is_none(&old_child),
          invalid_state(EINTERNAL_INVARIANT_BROKEN)
        )
        return old_child;
    let iter := ordered_map::internal_find(children, &key)
    if !ordered_map::iter_is_end(&iter, children) then
      assert!(node.is_leaf, invalid_state(EINTERNAL_INVARIANT_BROKEN))
      assert!(allow_overwrite, invalid_argument(EKEY_ALREADY_EXISTS))
      return some(iter_replace(iter, children, child));
    let (reserved_slot, node) :=
      if node_index == ROOT_INDEX then
        assert!(
          path_to_node.is_empty(), invalid_state(
            EINTERNAL_INVARIANT_BROKEN
          )
        )
        let new_root_node := new_node::<K, V>(false)
        let (replacement_node_slot, replacement_node_reserved_slot) :=
          reserve_slot(&mut self.nodes)
        let max_key :=
          do
            let root_children := &(self.root).children
            let max_key :=
              *ordered_map::iter_borrow_key(
                &ordered_map::iter_prev(
                  ordered_map::internal_new_end_iter(
                    root_children
                  ), root_children
                ),
                root_children
              )
            if is_lt(&compare(&max_key, &key)) then max_key := key
            return max_key
        ordered_map::add(
          &mut new_root_node.children, max_key,
          new_inner_child::<V>(replacement_node_slot)
        )
        let node := self.replace_root(new_root_node)
        path_to_node := core.prim.pushVector(path_to_node, ROOT_INDEX)
        let replacement_index :=
          reserved_to_index(&replacement_node_reserved_slot)
        if node.is_leaf then
          self.min_leaf_index := replacement_index
          self.max_leaf_index := replacement_index
        return (replacement_node_reserved_slot, node)
      else
        let (cur_node_reserved_slot, node) :=
          remove_and_reserve(&mut self.nodes, node_index)
        return (cur_node_reserved_slot, node)
    core.prim.moveValue(node_index)
    assert!(!path_to_node.is_empty(), invalid_state(EINTERNAL_INVARIANT_BROKEN))
    let right_node_reserved_slot := reserved_slot
    let left_node := node
    let is_leaf := left_node.is_leaf
    let left_children := &mut left_node.children
    let right_node_index := reserved_to_index(&right_node_reserved_slot)
    let left_next := &mut left_node.next
    let left_prev := &mut left_node.prev
    let max_degree :=
      if is_leaf then self.leaf_max_degree as u64
      else self.inner_max_degree as u64
    let target_size := (max_degree + 1) / 2
    ordered_map::add(left_children, key, child)
    let right_node_children := trim(left_children, target_size)
    assert!(
      length(left_children) <= max_degree,
      invalid_state(EINTERNAL_INVARIANT_BROKEN)
    )
    assert!(
      length(&right_node_children) <= max_degree,
      invalid_state(EINTERNAL_INVARIANT_BROKEN)
    )
    let mut right_node := new_node_with_children(is_leaf, right_node_children)
    let (left_node_slot, left_node_reserved_slot) :=
      reserve_slot(&mut self.nodes)
    let left_node_index := stored_to_index(&left_node_slot)
    right_node.next := *left_next
    *left_next := right_node_index
    right_node.prev := left_node_index
    if *left_prev != NULL_INDEX then
      storage_slots_allocator::borrow_mut(
        &mut self.nodes, *left_prev
      ).next := left_node_index
      assert!(
        right_node_index != self.min_leaf_index,
        invalid_state(EINTERNAL_INVARIANT_BROKEN)
      )
    else
      if right_node_index == self.min_leaf_index then
        assert!(is_leaf, invalid_state(EINTERNAL_INVARIANT_BROKEN))
        self.min_leaf_index := left_node_index
    let max_left_key :=
      *ordered_map::iter_borrow_key(
        &ordered_map::iter_prev(
          ordered_map::internal_new_end_iter(left_children), left_children
        ),
        left_children
      )
    fill_reserved_slot(&mut self.nodes, left_node_reserved_slot, left_node)
    fill_reserved_slot(&mut self.nodes, right_node_reserved_slot, right_node)
    destroy_none(
      self.add_at(
        path_to_node, max_left_key, new_inner_child::<V>(
          left_node_slot
        ), false
      )
    )
    return none::<Child<V> >()

  spec add_at where
    pragma opaque

  -- Last node in the path is one where we need to add the child to.
  -- First check if we can perform this operation, without changing structure of the tree (i.e. without adding any nodes).
  -- For that we can just borrow the single node
  -- Compute directly, as we cannot use get_max_degree(), as self is already mutably borrowed.
  -- Adding a child to a current node doesn't exceed the size, so we can just do that.
  -- If we cannot add more nodes without exceeding the size,
  -- but node with `key` already exists, we either need to replace or abort.
  -- # of children in the current node exceeds the threshold, need to split into two nodes.
  -- If we are at the root, we need to move root node to become a child and have a new root node,
  -- in order to be able to split the node on the level it is.
  -- Splitting root now, need to create a new root.
  -- Since root is stored direclty in the resource, we will swap-in the new node there.
  -- is_leaf=
  -- Reserve a slot where the current root will be moved to.
  -- need to check if key is largest, as invariant is that "parent's pointers" have been updated,
  -- but key itself can be larger than all previous ones.
  -- New root will have start with a single child - the existing root (which will be at replacement location).
  -- we moved the currently processing node one level down, so we need to update the path
  -- replacement node is the only leaf, so we update the pointers:
  -- In order to work on multiple nodes at the same time, we cannot borrow_mut, and need to be
  -- remove_and_reserve existing node.
  -- move node_index out of scope, to make sure we don't accidentally access it, as we are done with it.
  -- (i.e. we should be using `reserved_slot` instead).
  -- Now we can perform the split at the current level, as we know we are not at the root level.
  -- Parent has a reference under max key to the current node, so existing index
  -- needs to be the right node.
  -- Since ordered_map::trim moves from the end (i.e. smaller keys stay),
  -- we are going to put the contents of the current node on the left side,
  -- and create a new right node.
  -- So if we had before (node_index, node), we will change that to end up having:
  -- (new_left_node_index, node trimmed off) and (node_index, new node with trimmed off children)
  --
  -- So let's rename variables cleanly:
  -- Compute directly, as we cannot use get_max_degree(), as self is already mutably borrowed.
  -- compute the target size for the left node:
  -- Add child (which will exceed the size), and then trim off to create two sets of children of correct sizes.
  -- right nodes next is the node that was next of the left (previous) node, and next of left node is the right node.
  -- right node's prev becomes current left node
  -- Since the previously used index is going to the right node, `prev` pointer of the next node is correct,
  -- and we need to update next pointer of the previous node (if exists)
  -- Otherwise, if we were the smallest node on the level. if this is the leaf level, update the pointer.
  -- Largest left key is the split key.
  -- Add new Child (i.e. pointer to the left node) in the parent.
  /--
  Given a path to node (excluding the node itself), which is currently stored under "old_key", update "old_key" to "new_key".
  -/
  fun update_key {K has Copy, Drop, Store} {V has Store}(
    self : &mut BigOrderedMap<K, V>, mut path_to_node : Vector<u64>,
    old_key : &K, new_key : K
  ) -> Unit :=
    while !path_to_node.is_empty() do
      let node_index := path_to_node.pop_back()
      let node :=
        do
          let (self, node_index) := (self, node_index)
          return (if node_index == ROOT_INDEX then &mut self.root
          else storage_slots_allocator::borrow_mut(&mut self.nodes, node_index))
      let children := &mut node.children
      replace_key_inplace(children, old_key, new_key)
      if ordered_map::iter_borrow_key(
        &ordered_map::iter_prev(
          ordered_map::internal_new_end_iter(children), children
        ),
        children
      )
        != &new_key then
        return ();

  -- If we were not updating the largest child, we don't need to continue.
  fun remove_at_with_iter_hint {K has Copy, Drop, Store} {V has Store}(
    self : &mut BigOrderedMap<K, V>, mut path_to_node : Vector<u64>,
    key : &K, iter_hint : Option<ordered_map::IteratorPtr>
  ) -> Option<Child<V> > := do
    let node_index := path_to_node.pop_back()
    let old_child :=
      do
        let node :=
          do
            let (self, node_index) := (self, node_index)
            return (if node_index == ROOT_INDEX then &mut self.root
            else storage_slots_allocator::borrow_mut(
              &mut self.nodes, node_index
            ))
        let children := &mut node.children
        let is_leaf := node.is_leaf
        let old_child :=
          if is_some(&iter_hint) then
            let iter_hint := destroy_some(iter_hint)
            return (if ordered_map::iter_is_end(&iter_hint, children) then
              none::<Child<V> >()
            else
              assert!(
                ordered_map::iter_borrow_key(&iter_hint, children) == key,
                invalid_argument(EINTERNAL_INVARIANT_BROKEN)
              )
              return some(ordered_map::iter_remove(iter_hint, children)))
          else ordered_map::remove_or_none(children, key)
        if is_none(&old_child) then return old_child;
        if node_index == ROOT_INDEX then
          assert!(
            path_to_node.is_empty(), invalid_state(
              EINTERNAL_INVARIANT_BROKEN
            )
          )
          if !is_leaf && length(children) == 1 then
            let Child<V>::Inner { node_index := inner_child_index } :=
              ordered_map::iter_remove(
                ordered_map::iter_prev(
                  ordered_map::internal_new_end_iter(children), children
                ),
                children
              )
            let inner_child :=
              storage_slots_allocator::remove(
                &mut self.nodes, inner_child_index
              )
            if inner_child.is_leaf then
              self.min_leaf_index := ROOT_INDEX
              self.max_leaf_index := ROOT_INDEX
            self.replace_root(inner_child).destroy_empty_node()
          return old_child;
        let max_degree :=
          if is_leaf then self.leaf_max_degree as u64
          else self.inner_max_degree as u64
        let degree := length(children)
        let big_enough := degree * 2 >= max_degree
        let new_max_key :=
          *ordered_map::iter_borrow_key(
            &ordered_map::iter_prev(
              ordered_map::internal_new_end_iter(children), children
            ),
            children
          )
        let max_key_updated := is_lt(&compare(&new_max_key, key))
        if max_key_updated then
          assert!(degree >= 1, invalid_state(EINTERNAL_INVARIANT_BROKEN))
          self.update_key(path_to_node, key, new_max_key)
        if big_enough then return old_child;
        return old_child
    self.process_rebalance_after_child_removal(node_index, path_to_node)
    return old_child

  spec remove_at_with_iter_hint where
    pragma opaque

  -- Last node in the path is one where we need to remove the child from.
  -- First check if we can perform this operation, without changing structure of the tree (i.e. without rebalancing any nodes).
  -- For that we can just borrow the single node
  -- key not found, no need to rebalance
  -- If current node is root, lower limit of max_degree/2 nodes doesn't apply.
  -- So we can adjust internally
  -- If root is not leaf, but has a single child, promote only child to root,
  -- and drop current root. Since root is stored directly in the resource, we
  -- "move" the child into the root.
  -- Compute directly, as we cannot use get_max_degree(), as self is already mutably borrowed.
  -- See if the node is big enough, or we need to merge it with another node on this level.
  -- See if max key was updated for the current node, and if so - update it on the path.
  -- If node is big enough after removal, we are done.
  -- Children size is below threshold, we need to rebalance with a neighbor on the same level.
  fun process_rebalance_after_child_removal {K has Copy, Drop, Store} {V has Store}(
    self : &mut BigOrderedMap<K, V>, node_index : u64,
    path_to_node : Vector<u64>
  ) -> Unit := do
    let (node_slot, mut node) := remove_and_reserve(&mut self.nodes, node_index)
    let is_leaf := node.is_leaf
    let max_degree := self.get_max_degree(is_leaf)
    let prev := node.prev
    let next := node.next
    let sibling_index :=
      do
        let parent_children :=
          &(do
            let (self, node_index) :=
              (self, path_to_node[path_to_node.length - 1])
            return (if node_index == ROOT_INDEX then &self.root
            else storage_slots_allocator::borrow(
              &self.nodes, node_index
            ))).children
        assert!(
          length(parent_children)
            >= 2, invalid_state(EINTERNAL_INVARIANT_BROKEN)
        )
        return (if stored_to_index(
          &ordered_map::iter_borrow(
            ordered_map::iter_prev(
              ordered_map::internal_new_end_iter(
                parent_children
              ), parent_children
            ),
            parent_children
          ).node_index
        )
          == node_index then
          prev
        else next)
    let children := &mut node.children
    let (sibling_slot, mut sibling_node) :=
      remove_and_reserve(&mut self.nodes, sibling_index)
    assert!(
      is_leaf == sibling_node.is_leaf,
      invalid_state(EINTERNAL_INVARIANT_BROKEN)
    )
    let sibling_children := &mut sibling_node.children
    if (length(sibling_children) - 1) * 2 >= max_degree then
      if sibling_index == next then
        let old_max_key :=
          *ordered_map::iter_borrow_key(
            &ordered_map::iter_prev(
              ordered_map::internal_new_end_iter(children), children
            ),
            children
          )
        let sibling_begin_iter :=
          ordered_map::internal_new_begin_iter(sibling_children)
        let borrowed_max_key :=
          *ordered_map::iter_borrow_key(&sibling_begin_iter, sibling_children)
        let borrowed_element :=
          ordered_map::iter_remove(sibling_begin_iter, sibling_children)
        iter_add(
          ordered_map::internal_new_end_iter(
            children
          ), children, borrowed_max_key,
          borrowed_element
        )
        self.update_key(path_to_node, &old_max_key, borrowed_max_key)
      else
        let sibling_end_iter :=
          ordered_map::iter_prev(
            ordered_map::internal_new_end_iter(
              sibling_children
            ), sibling_children
          )
        let borrowed_max_key :=
          *ordered_map::iter_borrow_key(&sibling_end_iter, sibling_children)
        let borrowed_element :=
          ordered_map::iter_remove(sibling_end_iter, sibling_children)
        ordered_map::add(children, borrowed_max_key, borrowed_element)
        self.update_key(
          path_to_node, &borrowed_max_key,
          *ordered_map::iter_borrow_key(
            &ordered_map::iter_prev(
              ordered_map::internal_new_end_iter(
                sibling_children
              ), sibling_children
            ),
            sibling_children
          )
        )
      fill_reserved_slot(&mut self.nodes, node_slot, node)
      fill_reserved_slot(&mut self.nodes, sibling_slot, sibling_node)
      return ();
    let (key_to_remove, reserved_slot_to_remove) :=
      if sibling_index == next then
        let Node<K, V>::V1 { is_leaf := _,
        children := sibling_children,
        prev := _,
        next := sibling_next } :=
          sibling_node
        let key_to_remove :=
          *ordered_map::iter_borrow_key(
            &ordered_map::iter_prev(
              ordered_map::internal_new_end_iter(children), children
            ),
            children
          )
        append_disjoint(children, sibling_children)
        node.next := sibling_next
        if node.next != NULL_INDEX then
          assert!(
            storage_slots_allocator::borrow_mut(&mut self.nodes, node.next).prev
              == sibling_index,
            invalid_state(EINTERNAL_INVARIANT_BROKEN)
          )
        if node.prev != NULL_INDEX then
          storage_slots_allocator::borrow_mut(
            &mut self.nodes, node.prev
          ).next := sibling_index
        if self.min_leaf_index == node_index then
          assert!(is_leaf, invalid_state(EINTERNAL_INVARIANT_BROKEN))
          self.min_leaf_index := sibling_index
        fill_reserved_slot(&mut self.nodes, sibling_slot, node)
        return (key_to_remove, node_slot)
      else
        let Node<K, V>::V1 { is_leaf := _,
        children := node_children,
        prev := _,
        next := node_next } :=
          node
        let key_to_remove :=
          *ordered_map::iter_borrow_key(
            &ordered_map::iter_prev(
              ordered_map::internal_new_end_iter(
                sibling_children
              ), sibling_children
            ),
            sibling_children
          )
        append_disjoint(sibling_children, node_children)
        sibling_node.next := node_next
        if sibling_node.next != NULL_INDEX then
          assert!(
            storage_slots_allocator::borrow_mut(
              &mut self.nodes, sibling_node.next
            ).prev
              == node_index,
            invalid_state(EINTERNAL_INVARIANT_BROKEN)
          )
        if sibling_node.prev != NULL_INDEX then
          storage_slots_allocator::borrow_mut(
            &mut self.nodes, sibling_node.prev
          ).next := node_index
        if self.min_leaf_index == sibling_index then
          assert!(is_leaf, invalid_state(EINTERNAL_INVARIANT_BROKEN))
          self.min_leaf_index := node_index
        fill_reserved_slot(&mut self.nodes, node_slot, sibling_node)
        return (key_to_remove, sibling_slot)
    assert!(!path_to_node.is_empty(), invalid_state(EINTERNAL_INVARIANT_BROKEN))
    let slot_to_remove :=
      destroy_some(
        do
          let (self, path_to_node, key) := (self, path_to_node, &key_to_remove)
          return self.remove_at_with_iter_hint(
            path_to_node, key, none::<ordered_map::IteratorPtr>()
          )
      ).destroy_inner_child()
    free_reserved_slot(&mut self.nodes, reserved_slot_to_remove, slot_to_remove)

  -- In order to work on multiple nodes at the same time, we cannot borrow_mut, and need to be
  -- remove_and_reserve existing node.
  -- index of the node we will rebalance with.
  -- If we are the largest node from the parent, we merge with the `prev`
  -- (which is then guaranteed to have the same parent, as any node has >1 children),
  -- otherwise we merge with `next`.
  -- The sibling node has enough elements, we can just borrow an element from the sibling node.
  -- if sibling is the node with larger keys, we remove a child from the start
  -- max_key of the current node changed, so update
  -- if sibling is the node with smaller keys, we remove a child from the end
  -- max_key of the sibling node changed, so update
  -- The sibling node doesn't have enough elements to borrow, merge with the sibling node.
  -- Keep the slot of the node with larger keys of the two, to not require updating key on the parent nodes.
  -- But append to the node with smaller keys, as ordered_map::append is more efficient when adding to the end.
  -- destroying larger sibling node, keeping sibling_slot.
  -- we are removing node_index, which previous's node's next was pointing to,
  -- so update the pointer
  -- Otherwise, we were the smallest node on the level. if this is the leaf level, update the pointer.
  -- destroying larger current node, keeping node_slot
  -- we are removing sibling node_index, which previous's node's next was pointing to,
  -- so update the pointer
  -- Otherwise, sibling was the smallest node on the level. if this is the leaf level, update the pointer.
  -- we can destory_some() here, because inner node should always be present.
  -- ===== spec ===========
  -- recursive functions need to be marked opaque
  -- ============================= Tests ====================================
  -- uncomment to debug:
  -- aptos_std::debug::print(&std::string::utf8(b"print map"));
  -- aptos_std::debug::print(self);
  -- self.print_map_for_node(ROOT_INDEX, 0);
  -- ========== Verify only functions ==========
  -- Closure without `requires`: `iter_modify`'s precondition on the
  -- closure is trivially dischargeable.
  -- Constrained closure used within its precondition: the caller
  -- discharges `requires_of` from the map's current content and gets
  -- the closure's postcondition in return.
  fun __lambda__2__test_verify_modify(v : &mut u64) -> Bool := do
    *v := 12
    return true

  spec __lambda__2__test_verify_modify where
    requires v == 11
    ensures v == 12
    ensures result == true

  fun test_verify_borrow_front_key() -> Unit := do
    let keys := vector<u64>[1, 2, 3]
    let values := vector<u64>[4, 5, 6]
    let mut map := new_from(keys, values)
    let (_key, _value) := map.borrow_front()
    spec do
      assert keys[0] == 1
      assert spec_contains(keys, 1)
      assert spec_contains_key(map, _key)
      assert spec_get(map, _key) == _value
      assert _key == 1
    map.remove(&1)
    map.remove(&2)
    map.remove(&3)
    map.destroy_empty()

  spec test_verify_borrow_front_key where
    pragma verify

  fun test_verify_borrow_back_key() -> Unit := do
    let keys := vector<u64>[1, 2, 3]
    let values := vector<u64>[4, 5, 6]
    let mut map := new_from(keys, values)
    let (key, value) := map.borrow_back()
    spec do
      assert keys[2] == 3
      assert spec_contains(keys, 3)
      assert spec_contains_key(map, key)
      assert spec_get(map, key) == value
      assert key == 3
    map.remove(&1)
    map.remove(&2)
    map.remove(&3)
    map.destroy_empty()

  spec test_verify_borrow_back_key where
    pragma verify

  fun test_verify_upsert() -> Unit := do
    let keys := vector<u64>[1, 2, 3]
    let values := vector<u64>[4, 5, 6]
    let mut map := new_from(keys, values)
    let (_key, _value) := map.borrow_back()
    let result_1 := map.upsert(4, 5)
    spec do
      assert spec_contains_key(map, 4)
      assert spec_get(map, 4) == 5
      assert is_none(result_1)
    let result_2 := map.upsert(4, 6)
    spec do
      assert spec_contains_key(map, 4)
      assert spec_get(map, 4) == 6
      assert is_some(result_2)
      assert 0x1::std::option::borrow(result_2) == 5
      assert !spec_contains_key(map, 10)
    spec do
      assert keys[0] == 1
      assert spec_contains_key(map, 1)
      assert spec_get(map, 1) == 4
    let v := map.remove(&1)
    spec assert v == 4
    map.remove(&2)
    map.remove(&3)
    map.remove(&4)
    spec do
      assert !spec_contains_key(map, 1)
      assert !spec_contains_key(map, 2)
      assert !spec_contains_key(map, 3)
      assert !spec_contains_key(map, 4)
      assert spec_len(map) == 0
    map.destroy_empty()

  spec test_verify_upsert where
    pragma verify

  fun test_verify_iter_across_upsert() -> Unit := do
    let mut map := new_from(vector<u64>[1, 2], vector<u64>[10, 20])
    let iter := map.internal_find(&1)
    let old_value := map.upsert(2, 21)
    spec do
      assert is_some(old_value)
      assert spec_iter_valid(iter, map)
    let v := *iter.iter_borrow(&map)
    spec assert v == 10
    map.remove(&1)
    map.remove(&2)
    map.destroy_empty()

  spec test_verify_iter_across_upsert where
    pragma verify

  -- An existing-key upsert replaces the value in place: the iterator
  -- stays valid across it.
  fun test_verify_next_key() -> Unit := do
    let keys := vector<u64>[1, 2, 3]
    let values := vector<u64>[4, 5, 6]
    let mut map := new_from(keys, values)
    let result_1 := map.next_key(&3)
    spec assert is_none(result_1)
    let result_2 := map.next_key(&1)
    spec do
      assert keys[0] == 1
      assert spec_contains_key(map, 1)
      assert keys[1] == 2
      assert spec_contains_key(map, 2)
      assert is_some(result_2)
      assert 0x1::std::option::borrow(result_2) == 2
    map.remove(&1)
    map.remove(&2)
    map.remove(&3)
    map.destroy_empty()

  spec test_verify_next_key where
    pragma verify

  fun test_verify_prev_key() -> Unit := do
    let keys := vector<u64>[1, 2, 3]
    let values := vector<u64>[4, 5, 6]
    let mut map := new_from(keys, values)
    let result_1 := map.prev_key(&1)
    spec assert is_none(result_1)
    let result_2 := map.prev_key(&3)
    spec do
      assert keys[0] == 1
      assert spec_contains_key(map, 1)
      assert keys[1] == 2
      assert spec_contains_key(map, 2)
      assert is_some(result_2)
    map.remove(&1)
    map.remove(&2)
    map.remove(&3)
    map.destroy_empty()

  spec test_verify_prev_key where
    pragma verify

  fun test_verify_remove() -> Unit := do
    let keys := vector<u64>[1, 2, 3]
    let values := vector<u64>[4, 5, 6]
    let mut map := new_from(keys, values)
    spec do
      assert keys[1] == 2
      assert spec_contains(keys, 2)
      assert spec_contains_key(map, 2)
      assert spec_get(map, 2) == 5
      assert spec_len(map) == 3
    let v := map.remove(&1)
    spec do
      assert v == 4
      assert spec_contains_key(map, 2)
      assert spec_get(map, 2) == 5
      assert spec_len(map) == 2
      assert !spec_contains_key(map, 1)
    map.remove(&2)
    map.remove(&3)
    map.destroy_empty()

  spec test_verify_remove where
    pragma verify

  fun test_aborts_if_new_from_1() -> BigOrderedMap<u64, u64> := do
    let keys := vector<u64>[1, 2, 3, 1]
    let values := vector<u64>[4, 5, 6, 7]
    spec do
      assert keys[0] == 1
      assert keys[3] == 1
    let map := new_from(keys, values)
    return map

  spec test_aborts_if_new_from_1 where
    pragma verify
    aborts_if true

  fun test_aborts_if_new_from_2(
    keys : Vector<u64>, values : Vector<u64>
  ) -> BigOrderedMap<u64, u64> := do
    let map := new_from(keys, values)
    return map

  spec test_aborts_if_new_from_2 where
    pragma verify
    aborts_if ∃ (i in 0 .. keys.length; j in 0 .. keys.length),
        i != j && keys[i] == keys[j]
    aborts_if keys.length != values.length

  fun test_aborts_if_remove(map : &mut BigOrderedMap<u64, u64>) -> Unit := do
    map.remove(&1)

  spec test_aborts_if_remove where
    pragma verify
    aborts_if !spec_contains_key(map, 1)

  fun test_verify_iter_next() -> Unit := do
    let keys := vector<u64>[1, 2, 3]
    let mut map := new_from(keys, vector<u64>[4, 5, 6])
    let it := map.internal_find(&1)
    let k1 := *it.iter_borrow_key()
    let it2 := it.iter_next(&map)
    let k2 := *it2.iter_borrow_key()
    let it3 := it2.iter_next(&map)
    let k3 := *it3.iter_borrow_key()
    let it_end := it3.iter_next(&map)
    spec do
      assert keys[0] == 1
      assert keys[1] == 2
      assert keys[2] == 3
      assert spec_contains(keys, 1)
      assert spec_contains(keys, 2)
      assert spec_contains(keys, 3)
      assert spec_contains_key(map, 1)
      assert spec_contains_key(map, 2)
      assert spec_contains_key(map, 3)
      assert spec_len(map) == 3
    ground_enum_123(&map)
    spec do
      assert k1 == 1
      assert k2 == 2
      assert k3 == 3
      assert it_end.iter_is_end(map)
      assert spec_rank(map, k1) == 0
      assert spec_rank(map, k2) == 1
      assert spec_rank(map, k3) == 2
    map.remove(&1)
    map.remove(&2)
    map.remove(&3)
    map.destroy_empty()

  spec test_verify_iter_next where
    pragma verify

  -- Each step advances the rank; End follows the last rank.
  fun test_verify_iter_prev() -> Unit := do
    let keys := vector<u64>[1, 2, 3]
    let values := vector<u64>[4, 5, 6]
    let mut map := new_from(keys, values)
    let it_end := map.internal_new_end_iter()
    let it3 := it_end.iter_prev(&map)
    let k3 := *it3.iter_borrow_key()
    let it2 := it3.iter_prev(&map)
    let k2 := *it2.iter_borrow_key()
    spec do
      assert k3 == 3
      assert keys[1] == 2
      assert spec_contains(keys, 2)
      assert spec_contains_key(map, 2)
      assert k2 == 2
      assert spec_len(map) == 3
      assert spec_rank(map, k3) == 2
      assert spec_rank(map, k2) == 1
    map.remove(&1)
    map.remove(&2)
    map.remove(&3)
    map.destroy_empty()

  spec test_verify_iter_prev where
    pragma verify

  -- Materialize the ground fact so the maximality quantifier of
  -- iter_prev's contract instantiates at k == 2.
  -- Stepping back from End lands on the last rank, then decrements.
  fun __lambda__1__test_verify_iter_modify(v : &mut u64) -> u64 := do
    let o := *v
    *v := 50
    return o

  -- Materialize ground membership facts so the frame quantifier
  -- (spec_unchanged_except_at) instantiates at keys 1 and 3.
  -- Frame: other keys are untouched.
  fun test_verify_keys_sorted() -> Unit := do
    let mut map := new_from(vector<u64>[3, 1, 2], vector<u64>[6, 4, 5])
    let ks := map.keys()
    spec do
      assert ks.length == 3
      assert ks[0] < ks[1]
      assert ks[1] < ks[2]
      assert spec_contains_key(map, ks[0])
      assert spec_contains_key(map, ks[1])
      assert spec_contains_key(map, ks[2])
    map.remove(&1)
    map.remove(&2)
    map.remove(&3)
    map.destroy_empty()

  spec test_verify_keys_sorted where
    pragma verify

  -- Sortedness and membership come from the keys() contract.
  fun test_verify_leaf_iter() -> Unit := do
    let mut map := new_from(vector<u64>[1, 2, 3], vector<u64>[4, 5, 6])
    let lit := map.internal_leaf_new_begin_iter()
    let (entries, _next) :=
      lit.internal_leaf_iter_borrow_entries_and_next_leaf_index(&map)
    let ks := ordered_map::keys(entries)
    let k0 := ks[0]
    let child := ordered_map::borrow(entries, &k0)
    let v0 := *child.internal_leaf_borrow_value()
    spec do
      assert spec_contains_key(map, k0)
      assert v0 == spec_get(map, k0)
    map.remove(&1)
    map.remove(&2)
    map.remove(&3)
    map.destroy_empty()

  spec test_verify_leaf_iter where
    pragma verify

  -- The begin leaf iterator is never end, so this cannot abort.
  -- Leaves of a nonempty map are nonempty, so borrowing the first leaf
  -- key cannot abort; the entry is a real map entry with matching value.
  /--
  Witness-test support: derives the exact enumeration of a map holding
  keys {1, 2, 3}, walking the stepwise assert ladder once; callers get
  the facts from the ensures.
  -/
  fun ground_enum_123(map : &BigOrderedMap<u64, u64>) -> Unit := do
    let _fk := map.front_key()
    spec do
      assert spec_rank(map, 1) >= 0 && spec_rank(map, 1) < 3
        && spec_key_at(map, spec_rank(map, 1)) == 1
      assert spec_rank(map, 2) >= 0 && spec_rank(map, 2) < 3
        && spec_key_at(map, spec_rank(map, 2)) == 2
      assert spec_rank(map, 3) >= 0 && spec_rank(map, 3) < 3
        && spec_key_at(map, spec_rank(map, 3)) == 3
      assert spec_rank(map, 1) < spec_rank(map, 2)
      assert spec_rank(map, 2) < spec_rank(map, 3)
      assert spec_rank(map, 1) == 0
      assert spec_rank(map, 2) == 1
      assert spec_rank(map, 3) == 2
      assert spec_key_at(map, 0) == 1
      assert spec_key_at(map, 1) == 2
      assert spec_key_at(map, 2) == 3

  spec ground_enum_123 where
    pragma verify
    requires spec_len(map) == 3
    requires spec_contains_key(map, 1)
    requires spec_contains_key(map, 2)
    requires spec_contains_key(map, 3)
    ensures spec_rank(map, 1) == 0 && spec_rank(map, 2) == 1
        && spec_rank(map, 3) == 2
    ensures spec_key_at(map, 0) == 1 && spec_key_at(map, 1) == 2
        && spec_key_at(map, 2) == 3

  -- Also pulls in the key's cmp instantiation (gates the ascending axiom).
  -- Each contained key has an in-range rank that key_at inverts.
  -- Ranks follow the key order.
  -- Three distinct ordered positions in 0..3 pin the ranks exactly.
  -- ... and therefore the enumeration itself.
  fun test_verify_enumeration_view() -> Unit := do
    let keys := vector<u64>[1, 2, 3]
    let values := vector<u64>[4, 5, 6]
    let mut map := new_from(keys, values)
    let fk := map.front_key()
    spec do
      assert keys[0] == 1
      assert keys[1] == 2
      assert keys[2] == 3
      assert spec_contains(keys, 1)
      assert spec_contains(keys, 2)
      assert spec_contains(keys, 3)
      assert spec_contains_key(map, 1)
      assert spec_contains_key(map, 2)
      assert spec_contains_key(map, 3)
      assert spec_len(map) == 3
    ground_enum_123(&map)
    spec do
      assert fk == 1
      assert fk == spec_key_at(map, 0)
      assert spec_get(map, spec_key_at(map, 0)) == 4
    map.remove(&1)
    map.remove(&2)
    map.remove(&3)
    map.destroy_empty()

  spec test_verify_enumeration_view where
    pragma verify

  -- Materialize ground membership facts for the quantifiers.
  -- Cross-checks: the ordering API agrees, and values compose.
  fun test_verify_iter_rank_loop() -> Unit := do
    let mut map := new_from(vector<u64>[1, 2, 3], vector<u64>[4, 5, 6])
    let count := 0
    let it := map.internal_new_begin_iter()
    while !it.iter_is_end(&map) do
      count := count + 1
      it := it.iter_next(&map)
    where
      invariant spec_iter_valid(it, map)
      invariant !(it is End)
        ==> spec_contains_key(map, it.key) && count == spec_rank(map, it.key)
      invariant it is End ==> count == spec_len(map)
    spec do
      assert count == spec_len(map)
      assert count == 3
    map.remove(&1)
    map.remove(&2)
    map.remove(&3)
    map.destroy_empty()

  spec test_verify_iter_rank_loop where
    pragma verify

  -- Iterator loop counting entries; the invariant indexes the walk by rank.
  fun test_verify_pop_rank() -> Unit := do
    let keys := vector<u64>[1, 2, 3]
    let values := vector<u64>[10, 20, 30]
    let mut map := new_from(keys, values)
    spec do
      assert keys[0] == 1
      assert keys[1] == 2
      assert keys[2] == 3
      assert spec_contains(keys, 1)
      assert spec_contains(keys, 2)
      assert spec_contains(keys, 3)
      assert spec_contains_key(map, 1)
      assert spec_contains_key(map, 2)
      assert spec_contains_key(map, 3)
      assert spec_len(map) == 3
    ground_enum_123(&map)
    let (k, v) := map.pop_front()
    spec do
      assert k == 1
      assert v == 10
      assert spec_len(map) == 2
      assert spec_rank(map, 2) == 0
      assert spec_rank(map, 3) == 1
      assert spec_key_at(map, 0) == 2
      assert spec_key_at(map, 1) == 3
    let (k2, v2) := map.pop_back()
    spec do
      assert k2 == 3
      assert v2 == 30
      assert spec_len(map) == 1
      assert spec_key_at(map, 0) == 2
      assert spec_rank(map, 2) == 0
    let (k3, _v3) := map.pop_front()
    spec assert k3 == 2
    map.destroy_empty()

  spec test_verify_pop_rank where
    pragma verify

  -- Materialize ground membership facts for the quantifiers.
  -- Popped key had rank 0; survivors' ranks shift down by one.
  -- Back border: the popped key had the last rank.
  fun test_verify_drain_loop() -> u64 := do
    let keys := vector<u64>[1, 2, 3]
    let values := vector<u64>[10, 20, 30]
    let mut map := new_from(keys, values)
    spec do
      assert keys[0] == 1
      assert keys[1] == 2
      assert keys[2] == 3
      assert spec_contains(keys, 1)
      assert spec_contains(keys, 2)
      assert spec_contains(keys, 3)
      assert spec_contains_key(map, 1)
      assert spec_contains_key(map, 2)
      assert spec_contains_key(map, 3)
      assert spec_len(map) == 3
      assert spec_get(map, 1) == 10
      assert spec_get(map, 2) == 20
      assert spec_get(map, 3) == 30
    ground_enum_123(&map)
    let sum := 0
    while !map.is_empty() do
      let (_k, v) := map.pop_front()
      sum := sum + v
    where
      invariant spec_len(map) <= 3
      invariant ∀ (i in 0 .. spec_len(map)),
        spec_key_at(map, i) == i + 4 - spec_len(map)
      invariant ∀ (i in 0 .. spec_len(map)),
        spec_get(map, spec_key_at(map, i)) == 10 * spec_key_at(map, i)
      invariant sum == 10 * (3 - spec_len(map)) * (4 - spec_len(map)) / 2
    spec assert sum == 60
    map.destroy_empty()
    return sum

  spec test_verify_drain_loop where
    pragma verify
    ensures result == 60

  -- Materialize ground membership and value facts for the invariant.
  -- The shift axioms carry the enumeration across pops, letting the
  -- invariant characterize the map and the sum by length alone.
  /--
  Witness-test support: sum of the values at the first `n` keys.
  -/
  spec fun spec_test_sum_upto(m : BigOrderedMap<u64, u64>, n : Int) : Int :=
    if n <= 0 then 0
    else spec_test_sum_upto(m, n - 1) + spec_get(m, spec_key_at(m, n - 1))

  fun test_verify_iter_sum_symbolic(m : &BigOrderedMap<u64, u64>) -> u64 := do
    let sum := 0
    let it := m.internal_new_begin_iter()
    while !it.iter_is_end(m) do
      let _t := *it.iter_borrow(m)
      sum := sum + _t
      it := it.iter_next(m)
    where
      invariant spec_iter_valid(it, m)
      invariant !(it is End)
        ==> spec_contains_key(m, it.key)
          && sum == spec_test_sum_upto(m, spec_rank(m, it.key))
      invariant it is End ==> sum == spec_test_sum_upto(m, spec_len(m))
    return sum

  spec test_verify_iter_sum_symbolic where
    pragma verify
    ensures result == spec_test_sum_upto(m, spec_len(m))

  -- Symbolic map + iterator walk: exercises the general axiom chain,
  -- not a concrete model.
  -- Aborts unspecified: the running u64 addition can overflow.
  fun test_verify_drain_symbolic(m : &mut BigOrderedMap<u64, u64>) -> u64 := do
    let count := 0
    while !m.is_empty() do
      let (_k, _v) := m.pop_front()
      count := count + 1
    where
      invariant count + spec_len(m) == spec_len(old(m))
      invariant ∀ (i in 0 .. spec_len(m)),
        spec_key_at(m, i) == spec_key_at(old(m), i + count)
      invariant ∀ (i in 0 .. spec_len(m)),
        spec_get(m, spec_key_at(m, i)) == spec_get(old(m), spec_key_at(m, i))
    return count

  spec test_verify_drain_symbolic where
    pragma verify
    aborts_if false
    ensures result == spec_len(old(m))
    ensures spec_len(m) == 0

  -- Symbolic drain: the current map stays the suffix of old(m) past
  -- the popped prefix.
  fun test_verify_remove_shift_symbolic(
    m : &mut BigOrderedMap<u64, u64>, k : u64
  ) -> Unit := do
    m.remove(&k)

  spec test_verify_remove_shift_symbolic where
    pragma verify
    requires spec_contains_key(m, k)
    aborts_if false
    ensures spec_len(m) == spec_len(old(m)) - 1
    ensures !spec_contains_key(m, k)
    ensures ∀ (i in 0 .. spec_len(m)),
        spec_key_at(m, i)
          == spec_key_at(old(m), if i < spec_rank(old(m), k) then i else i + 1)
    ensures ∀ (i in 0 .. spec_len(m)),
        spec_get(m, spec_key_at(m, i)) == spec_get(old(m), spec_key_at(m, i))

  -- Removal at an ARBITRARY rank, not just a border: the surviving
  -- enumeration is the old one with that one position spliced out.
  fun test_verify_pop_back_drain_symbolic(
    m : &mut BigOrderedMap<u64, u64>
  ) -> u64 := do
    let count := 0
    while !m.is_empty() do
      let (_k, _v) := m.pop_back()
      count := count + 1
    where
      invariant count + spec_len(m) == spec_len(old(m))
      invariant ∀ (i in 0 .. spec_len(m)),
        spec_key_at(m, i) == spec_key_at(old(m), i)
      invariant ∀ (i in 0 .. spec_len(m)),
        spec_get(m, spec_key_at(m, i)) == spec_get(old(m), spec_key_at(m, i))
    return count

  spec test_verify_pop_back_drain_symbolic where
    pragma verify
    aborts_if false
    ensures result == spec_len(old(m))
    ensures spec_len(m) == 0

  -- Back-border mirror of the drain above: popping from the back keeps
  -- the map a PREFIX of the entry map.
  fun test_verify_iter_collect_symbolic(
    m : &BigOrderedMap<u64, u64>
  ) -> Vector<u64> := do
    let mut out := vector<u64>[]
    let it := m.internal_new_begin_iter()
    while !it.iter_is_end(m) do
      out := core.prim.pushVector(out, *it.iter_borrow_key())
      it := it.iter_next(m)
    where
      invariant spec_iter_valid(it, m)
      invariant out.length <= spec_len(m)
      invariant ∀ (i in 0 .. out.length), out[i] == spec_key_at(m, i)
      invariant !(it is End)
        ==> spec_contains_key(m, it.key) && out.length == spec_rank(m, it.key)
      invariant it is End ==> out.length == spec_len(m)
    return out

  spec test_verify_iter_collect_symbolic where
    pragma verify
    aborts_if false
    ensures result.length == spec_len(m)
    ensures ∀ (i in 0 .. spec_len(m)), result[i] == spec_key_at(m, i)

  -- A full traversal that COLLECTS: the result is the whole key set in
  -- ascending order, position by position.
  fun test_verify_iter_valid_after_insert_symbolic(
    m : &mut BigOrderedMap<u64, u64>, k : u64, v : u64
  ) -> Unit := do
    m.add(k, v)
    let it := m.internal_new_begin_iter()
    spec do
      assert spec_iter_valid(it, m)
      assert !(it is End)
      assert spec_contains_key(m, it.key)

  spec test_verify_iter_valid_after_insert_symbolic where
    pragma verify
    requires !spec_contains_key(m, k)
    aborts_if false
    ensures spec_contains_key(m, k)
    ensures spec_len(m) == spec_len(old(m)) + 1

  -- A structural change invalidates outstanding iterators, but an
  -- iterator taken afterwards is valid and lands on a real entry.
  fun test_verify_back_key_rank_symbolic(m : &BigOrderedMap<u64, u64>) -> u64 :=
    m.back_key()

  spec test_verify_back_key_rank_symbolic where
    pragma verify
    requires spec_len(m) > 0
    aborts_if false
    ensures spec_contains_key(m, result)
    ensures spec_rank(m, result) == spec_len(m) - 1
    ensures spec_key_at(m, spec_len(m) - 1) == result

  fun test_verify_iter_borrow_mut_symbolic(
    m : &mut BigOrderedMap<u64, u64>, k : u64, nv : u64
  ) -> Unit := do
    let it := m.internal_find(&k)
    let v_ref := it.iter_borrow_mut(m)
    *v_ref := nv

  spec test_verify_iter_borrow_mut_symbolic where
    pragma verify
    requires spec_contains_key(m, k)
    aborts_if false
    ensures spec_get(m, k) == nv
    ensures spec_len(m) == spec_len(old(m))
    ensures spec_contains_key(m, k)
    ensures ∀ (other : u64),
        other != k
          ==> spec_contains_key(m, other) == spec_contains_key(old(m), other)
    ensures ∀ (other : u64),
        other != k && spec_contains_key(old(m), other)
          ==> spec_get(m, other) == spec_get(old(m), other)
    ensures ∀ (i in 0 .. spec_len(m)),
        spec_key_at(m, i) == spec_key_at(old(m), i)
    ensures spec_rank(m, k) == spec_rank(old(m), k)

  -- Write through the iterator's `&mut V`: the edge carried by the
  -- reference lands the update on the abstract map at `self.key`.
  -- Frame: no other entry is touched.
  -- A value write is not a structural change, so positions are intact.
  fun test_verify_iter_remove_symbolic(
    m : &mut BigOrderedMap<u64, u64>, k : u64
  ) -> u64 := do
    let it := m.internal_find_with_path(&k)
    return it.iter_remove(m)

  spec test_verify_iter_remove_symbolic where
    pragma verify
    requires spec_contains_key(m, k)
    aborts_if false
    ensures result == spec_get(old(m), k)
    ensures spec_len(m) == spec_len(old(m)) - 1
    ensures !spec_contains_key(m, k)
    ensures ∀ (other : u64),
        other != k
          ==> spec_contains_key(m, other) == spec_contains_key(old(m), other)
    ensures ∀ (other : u64),
        other != k && spec_contains_key(old(m), other)
          ==> spec_get(m, other) == spec_get(old(m), other)

  -- Frame: every other entry survives with its value.
  fun test_verify_iter_prev_loop_symbolic(
    m : &BigOrderedMap<u64, u64>
  ) -> u64 := do
    let count := 0
    let it := m.internal_new_end_iter()
    while !it.iter_is_begin(m) do
      it := it.iter_prev(m)
      count := count + 1
    where
      invariant spec_iter_valid(it, m)
      invariant count <= spec_len(m)
      invariant it is End ==> count == 0
      invariant !(it is End)
        ==> spec_contains_key(m, it.key)
          && spec_rank(m, it.key) == spec_len(m) - count
    return count

  spec test_verify_iter_prev_loop_symbolic where
    pragma verify
    aborts_if false
    ensures result == spec_len(m)

  -- Backward traversal over a symbolic map: from End each step decrements
  -- the rank, so the step count measures the distance from the end, and
  -- reaching begin means the whole map was walked.
  fun test_verify_front_remove_drain_symbolic(
    m : &mut BigOrderedMap<u64, u64>
  ) -> u64 := do
    let count := 0
    while !m.is_empty() do
      let k := m.front_key()
      m.remove(&k)
      count := count + 1
    where
      invariant count + spec_len(m) == spec_len(old(m))
      invariant ∀ (i in 0 .. spec_len(m)),
        spec_key_at(m, i) == spec_key_at(old(m), i + count)
      invariant ∀ (i in 0 .. spec_len(m)),
        spec_get(m, spec_key_at(m, i)) == spec_get(old(m), spec_key_at(m, i))
    return count

  spec test_verify_front_remove_drain_symbolic where
    pragma verify
    aborts_if false
    ensures result == spec_len(old(m))
    ensures spec_len(m) == 0

  -- Peek-then-remove, the shape callers write when they need the key
  -- before deciding: front_key's border rank fact plus the removal splice
  -- keep the map a suffix of the entry map, without going through
  -- pop_front's template.
  fun test_verify_iter_write_loop_symbolic(
    m : &mut BigOrderedMap<u64, u64>, c : u64
  ) -> Unit := do
    let count := 0
    let it := m.internal_new_begin_iter()
    while !it.iter_is_end(m) do
      let v_ref := it.iter_borrow_mut(m)
      *v_ref := c
      it := it.iter_next(m)
      count := count + 1
    where
      invariant spec_iter_valid(it, m)
      invariant spec_len(m) == spec_len(old(m))
      invariant count <= spec_len(m)
      invariant ∀ (i in 0 .. spec_len(m)),
        spec_key_at(m, i) == spec_key_at(old(m), i)
      invariant !(it is End)
        ==> spec_contains_key(m, it.key) && spec_rank(m, it.key) == count
      invariant it is End ==> count == spec_len(m)
      invariant ∀ (i in 0 .. count), spec_get(m, spec_key_at(m, i)) == c

  spec test_verify_iter_write_loop_symbolic where
    pragma verify
    aborts_if false
    ensures spec_len(m) == spec_len(old(m))
    ensures ∀ (i in 0 .. spec_len(m)),
        spec_key_at(m, i) == spec_key_at(old(m), i)
    ensures ∀ (i in 0 .. spec_len(m)), spec_get(m, spec_key_at(m, i)) == c

  -- Writing values while traversing: the walk stays valid because a value
  -- write is not a structural mutation, and positions survive it, so the
  -- invariant can say "every position visited so far now holds c".
  fun test_verify_find_started_walk_symbolic(
    m : &BigOrderedMap<u64, u64>, k : u64
  ) -> u64 := do
    let count := 0
    let it := m.internal_find(&k)
    while !it.iter_is_end(m) do
      it := it.iter_next(m)
      count := count + 1
    where
      invariant spec_iter_valid(it, m)
      invariant count + spec_rank(m, k) <= spec_len(m)
      invariant !(it is End)
        ==> spec_contains_key(m, it.key)
          && spec_rank(m, it.key) == spec_rank(m, k) + count
      invariant it is End ==> count + spec_rank(m, k) == spec_len(m)
    return count

  spec test_verify_find_started_walk_symbolic where
    pragma verify
    requires spec_contains_key(m, k)
    aborts_if false
    ensures result == spec_len(m) - spec_rank(m, k)

  -- Entering the walk at a found key rather than at begin: `internal_find`
  -- pins the iterator to that key, so the step count measures distance
  -- from its rank. This is the shape callers write when resuming from a
  -- known position.
  fun test_verify_early_exit_walk_symbolic(
    m : &BigOrderedMap<u64, u64>, target : u64
  ) -> Bool := do
    let found := false
    let it := m.internal_new_begin_iter()
    let seen := 0
    while !found && !it.iter_is_end(m) do
      if *it.iter_borrow(m) == target then found := true
      else
        it := it.iter_next(m)
        seen := seen + 1
    where
      invariant spec_iter_valid(it, m)
      invariant seen <= spec_len(m)
      invariant !(it is End)
        ==> spec_contains_key(m, it.key) && spec_rank(m, it.key) == seen
      invariant it is End ==> seen == spec_len(m)
      invariant !found
        ==> (∀ (i in 0 .. seen), spec_get(m, spec_key_at(m, i)) != target)
    return found

  spec test_verify_early_exit_walk_symbolic where
    pragma verify
    aborts_if false
    ensures !result
        ==> (∀ (i in 0 .. spec_len(m)),
          spec_get(m, spec_key_at(m, i)) != target)

  -- Early exit on the first entry whose value meets a condition. What the
  -- prefix invariant buys: on a negative result every position was
  -- checked, so the answer is complete rather than merely sound.
  -- A false result is complete: no position holds the target.
  fun test_verify_bounded_walk_symbolic(
    m : &BigOrderedMap<u64, u64>, limit : u64
  ) -> Vector<u64> := do
    let mut out := vector<u64>[]
    let it := m.internal_new_begin_iter()
    while out.length < limit && !it.iter_is_end(m) do
      out := core.prim.pushVector(out, *it.iter_borrow_key())
      it := it.iter_next(m)
    where
      invariant spec_iter_valid(it, m)
      invariant out.length <= limit && out.length <= spec_len(m)
      invariant ∀ (i in 0 .. out.length), out[i] == spec_key_at(m, i)
      invariant !(it is End)
        ==> spec_contains_key(m, it.key) && spec_rank(m, it.key) == out.length
      invariant it is End ==> out.length == spec_len(m)
    return out

  spec test_verify_bounded_walk_symbolic where
    pragma verify
    aborts_if false
    ensures result.length
        == (if limit < spec_len(m) then limit else spec_len(m))
    ensures ∀ (i in 0 .. result.length), result[i] == spec_key_at(m, i)

  -- Collect at most `limit` keys — the take-ready shape. The result is the
  -- bounded prefix of the enumeration, not just some subset of the keys.
  fun test_verify_keys_for_loop_symbolic(
    m : &BigOrderedMap<u64, u64>
  ) -> u64 := do
    let ks := m.keys()
    let sum := 0
    let n := ks.length
    let i := 0
    while i < n do
      let _t := *m.borrow(&ks[i])
      sum := sum + _t
      i := i + 1
    where
      invariant n == spec_len(m)
      invariant i <= spec_len(m)
      invariant sum == spec_test_sum_upto(m, i)
    return sum

  spec test_verify_keys_for_loop_symbolic where
    pragma verify
    ensures result == spec_test_sum_upto(m, spec_len(m))

  -- Walking `keys()` with a `for` loop and summing the values it points
  -- at. The loop variable is a vector index, so this only closes if the
  -- returned vector agrees position-wise with the enumeration.
  -- Aborts unspecified: the running u64 addition can overflow.
  fun test_verify_next_key_scan_symbolic(
    m : &BigOrderedMap<u64, u64>
  ) -> u64 := do
    let count := 1
    let k := m.front_key()
    let nxt := m.next_key(&k)
    while is_some(&nxt) do
      k := *option::borrow(&nxt)
      nxt := m.next_key(&k)
      count := count + 1
    where
      invariant spec_contains_key(m, k)
      invariant spec_rank(m, k) == count - 1
      invariant count <= spec_len(m)
      invariant spec_is_some(nxt)
        ==> spec_contains_key(m, spec_borrow(nxt))
          && spec_rank(m, spec_borrow(nxt)) == count
      invariant spec_is_none(nxt) ==> count == spec_len(m)
    return count

  spec test_verify_next_key_scan_symbolic where
    pragma verify
    requires spec_len(m) > 0
    aborts_if false
    ensures result == spec_len(m)

  -- Key stepping rather than iterators: start at the front and follow
  -- `next_key` to the end, the shape of a full scan that holds no
  -- iterator across calls. Counting the steps requires knowing that a
  -- successor sits one position later.
  fun test_verify_insert_shift_symbolic(
    m : &mut BigOrderedMap<u64, u64>, k : u64, v : u64
  ) -> Unit := do
    m.add(k, v)

  spec test_verify_insert_shift_symbolic where
    pragma verify
    requires !spec_contains_key(m, k)
    ensures spec_contains_key(m, k)
    ensures spec_len(m) == spec_len(old(m)) + 1
    ensures spec_get(m, k) == v
    ensures ∀ (i in 0 .. spec_rank(m, k)),
        spec_key_at(m, i) == spec_key_at(old(m), i)
    ensures ∀ (i in spec_rank(m, k) + 1 .. spec_len(m)),
        spec_key_at(m, i) == spec_key_at(old(m), i - 1)

  -- An insertion splices a position in: everything before the new key
  -- keeps its position, everything after moves up by one. Enumeration
  -- facts surviving an insert is what lets a caller reason about a map it
  -- has just added to.
  fun test_verify_lower_bound_scan_start_symbolic(
    m : &BigOrderedMap<u64, u64>, k : u64
  ) -> IteratorPtr<u64> := m.internal_lower_bound(&k)

  spec test_verify_lower_bound_scan_start_symbolic where
    pragma verify
    aborts_if false
    ensures !result.iter_is_end(m)
        ==> (∀ (i in 0 .. spec_rank(m, result.key)),
          compare(spec_key_at(m, i), k) == new Ordering::Less {})
    ensures spec_contains_key(m, k)
        ==> !result.iter_is_end(m) && result.key == k

  -- Where a range scan begins. The search is characterized by comparison,
  -- so these are the facts that carry it into the enumeration; no clause of
  -- its own is needed, unlike the ordered_map case, because that
  -- characterization quantifies over keys and so instantiates at the key
  -- being searched for.
  -- A scan starting here has skipped only smaller keys.
  -- A key that is present is landed on, not skipped past.
  -- Writing a value through an iterator leaves the key set alone, so every
  -- position is untouched — what a traversal needs in order to keep a
  -- position-indexed invariant while updating as it goes.
  fun __lambda__1__test_verify_iter_modify_ranks_symbolic(
    v : &mut u64
  ) -> u64 := do
    *v := 7
    return *v

  fun test_verify_iter_remove_shift_symbolic(
    m : &mut BigOrderedMap<u64, u64>, k : u64
  ) -> u64 := do
    let it := m.internal_find_with_path(&k)
    return it.iter_remove(m)

  spec test_verify_iter_remove_shift_symbolic where
    pragma verify
    requires spec_contains_key(m, k)
    aborts_if false
    ensures result == spec_get(old(m), k)
    ensures !spec_contains_key(m, k)
    ensures spec_len(m) == spec_len(old(m)) - 1
    ensures ∀ (i in 0 .. spec_rank(old(m), k)),
        spec_key_at(m, i) == spec_key_at(old(m), i)
    ensures ∀ (i in spec_rank(old(m), k) .. spec_len(m)),
        spec_key_at(m, i) == spec_key_at(old(m), i + 1)

  -- The mirror of the insert splice: removing through an iterator closes
  -- the position up, so keys before the removed one stay put and keys
  -- after it move down by one.
  fun test_verify_leaf_walk_sum_symbolic(
    m : &BigOrderedMap<u64, u64>
  ) -> u64 := do
    let sum := 0
    let it := m.internal_leaf_new_begin_iter()
    while !it.internal_leaf_iter_is_end() do
      let (entries, next_it) :=
        it.internal_leaf_iter_borrow_entries_and_next_leaf_index(m)
      let oit := ordered_map::internal_new_begin_iter(entries)
      while !ordered_map::iter_is_end(&oit, entries) do
        let _t :=
          *ordered_map::iter_borrow(oit, entries).internal_leaf_borrow_value()
        sum := sum + _t
        oit := ordered_map::iter_next(oit, entries)
      where
        invariant !(oit is End)
          ==> oit.index < 0x1::aptos_framework::ordered_map::spec_len(entries)
        invariant sum
          == spec_test_sum_upto(
            m,
            spec_leaf_offset(it, m)
              + (if oit is End then 0x1::aptos_framework::ordered_map::spec_len(
                entries
              )
              else oit.index)
          )
      it := next_it
    where
      invariant spec_leaf_iter_valid(it, m)
      invariant 0 <= spec_leaf_offset(it, m)
        && spec_leaf_offset(it, m) <= spec_len(m)
      invariant it.internal_leaf_iter_is_end()
        ==> spec_leaf_offset(it, m) == spec_len(m)
      invariant sum == spec_test_sum_upto(m, spec_leaf_offset(it, m))
    return sum

  spec test_verify_leaf_walk_sum_symbolic where
    pragma verify
    ensures result == spec_test_sum_upto(m, spec_len(m))

  -- The shape `for_each_ref` expands into: an outer walk over leaves and
  -- an inner walk over each leaf's entries. The leaf offset carries the
  -- aggregate across the nesting, so the total is the sum over every key
  -- — which needs the leaves to tile the enumeration, not merely to hold
  -- real entries.
  -- Carried, not just stated at the producer: the loop head
  -- havocs `it`, so without this the walk could exit early.
  -- `it` does not move during the inner walk, so its offset is
  -- the stable base for the positions being consumed here.
  -- Aborts unspecified: the running u64 addition can overflow.
  -- The ordering bindings below (`map_borrow_front`/`back`, `map_pop_front`/`back`,
  -- `map_prev_key`/`next_key`) presume `cmp::compare<K>` is a strict total order on K.
  -- Built-in K types satisfy this; user-defined K types must too for this spec block
  -- to be sound.
  --
  -- Size presumption: BigOrderedMap validates K/V serialized sizes against node-size
  -- limits (`validate_static_size_and_init_max_degrees` and per-insert checks) and
  -- aborts when exceeded. These size-based aborts — including `borrow_mut`'s
  -- constant-value-size requirement — are presumed not to fire and are not
  -- modeled by the bindings below.
  --
  -- Structural presumption: the tree's internal invariants — node shapes,
  -- child kinds (leaf nodes hold only `Child::Leaf`), and index validity
  -- (the `EINTERNAL_INVARIANT_BROKEN` asserts and variant field accesses) —
  -- are maintained by construction by this module, presumed to hold, and
  -- not modeled. Traversal specs' abort conditions are exhaustive modulo
  -- this presumption.
  --
  -- Iterator staleness: the iterator overlay specs apply to an iterator only
  -- for the map state it was created from (the documented API contract: the
  -- map must not be mutated while iterators are held). All iterator specs
  -- model reads and writes at the iterator's cached key; at runtime a stale
  -- iterator navigates by its retained position, so it may abort, return
  -- arbitrary results, or read/mutate a DIFFERENT entry than modeled. The
  -- prover enforces this contract mechanically and per map OBJECT: the
  -- validity bindings below give the map and its iterator types hidden
  -- version slots (fresh at creation, havocked by every structural
  -- mutation, preserved by value writes, excluded from equality, not
  -- nameable in specs), and validity — the bound native predicates, defined
  -- by the prover as slot equality — is stated as ordinary
  -- `requires`/`ensures` on the iterator API below. Any use of a stale
  -- iterator, or of an iterator against a different map object (also a
  -- sibling of the same type, or another element of a vector of maps),
  -- fails verification. Loops that advance an iterator carry
  -- `invariant spec_iter_valid(it, map)`; loops that merely hold one while
  -- leaving the map unmutated need no invariant.
  -- The iterator-validity predicates; their definitions (hidden-slot
  -- equality: the iterator was created from this map object and no
  -- structural mutation has intervened) come from the role bindings above.
  @[map_spec_iter_valid (BigOrderedMap)]
  opaque spec fun spec_iter_current {K} {V}(
    it : IteratorPtr<K>, map : BigOrderedMap<K, V>
  ) : Bool

  @[map_spec_leaf_iter_valid (BigOrderedMap)]
  opaque spec fun spec_leaf_iter_valid {K} {V}(
    it : LeafNodeIteratorPtr, map : BigOrderedMap<K, V>
  ) : Bool

  -- Frame predicate: no structural mutation between the two states, so
  -- iterators valid for the old state stay valid for the new one.
  @[map_spec_iter_preserved (BigOrderedMap)]
  opaque spec fun spec_iter_preserved {K} {V}(
    m_new : BigOrderedMap<K, V>, m_old : BigOrderedMap<K, V>
  ) : Bool

  spec fun spec_iter_valid {K} {V}(
    it : IteratorPtr<K>, map : BigOrderedMap<K, V>
  ) : Bool :=
    it is End || spec_iter_current(it, map)

  @[map_spec_len (BigOrderedMap)]
  opaque spec fun spec_len {K} {V}(t : BigOrderedMap<K, V>) : Int

  @[map_spec_has_key (BigOrderedMap)]
  opaque spec fun spec_contains_key {K} {V}(
    t : BigOrderedMap<K, V>, k : K
  ) : Bool

  -- Enumeration view: `spec_key_at(t, i)` is the i-th smallest key under
  -- `cmp::compare` (0 <= i < spec_len(t)), `spec_rank(t, k)` its inverse on
  -- contained keys. Lets loop invariants index a traversal by position.
  @[map_spec_key_at (BigOrderedMap)]
  opaque spec fun spec_key_at {K} {V}(t : BigOrderedMap<K, V>, i : Int) : K

  @[map_spec_rank (BigOrderedMap)]
  opaque spec fun spec_rank {K} {V}(t : BigOrderedMap<K, V>, k : K) : Int

  @[map_spec_set (BigOrderedMap)]
  opaque spec fun spec_set {K} {V}(
    t : BigOrderedMap<K, V>, k : K, v : V
  ) : BigOrderedMap<K, V>

  @[map_spec_del (BigOrderedMap)]
  opaque spec fun spec_remove {K} {V}(
    t : BigOrderedMap<K, V>, k : K
  ) : BigOrderedMap<K, V>

  @[map_spec_get (BigOrderedMap)]
  opaque spec fun spec_get {K} {V}(t : BigOrderedMap<K, V>, k : K) : V

  @[map_spec_aborts_destroy_empty (BigOrderedMap)]
  opaque spec fun spec_aborts_destroy_empty {K} {V}(
    t : BigOrderedMap<K, V>
  ) : Bool

  @[map_spec_aborts_add (BigOrderedMap)]
  opaque spec fun spec_aborts_add {K} {V}(
    t : BigOrderedMap<K, V>, k : K, v : V
  ) : Bool

  @[map_spec_aborts_del (BigOrderedMap)]
  opaque spec fun spec_aborts_del {K} {V}(t : BigOrderedMap<K, V>, k : K) : Bool

  @[map_spec_aborts_borrow (BigOrderedMap)]
  opaque spec fun spec_aborts_borrow {K} {V}(
    t : BigOrderedMap<K, V>, k : K
  ) : Bool

  @[map_spec_aborts_empty (BigOrderedMap)]
  spec fun spec_aborts_empty {K} {V}(t : BigOrderedMap<K, V>) : Bool :=
    spec_len(t) == 0

  @[map_spec_aborts_add_all (BigOrderedMap)]
  spec fun spec_aborts_add_all {K} {V}(
    m : BigOrderedMap<K, V>, keys : Vector<K>, values : Vector<V>
  ) : Bool :=
    keys.length != values.length
      || (∃ (i in 0 .. keys.length), spec_contains_key(m, keys[i]))
      || (∃ (i in 0 .. keys.length; j in 0 .. keys.length),
        i != j && keys[i] == keys[j])

  @[map_spec_aborts_new_from (BigOrderedMap)]
  spec fun spec_aborts_new_from {K} {V}(
    keys : Vector<K>, values : Vector<V>
  ) : Bool :=
    keys.length != values.length
      || (∃ (i in 0 .. keys.length; j in 0 .. keys.length),
        i != j && keys[i] == keys[j])

  @[map_spec_aborts_new_with_config (BigOrderedMap)]
  spec fun spec_aborts_new_with_config {K} {V}(
    inner_max_degree : Int, leaf_max_degree : Int, _reuse_slots : Bool
  ) : Bool :=
    inner_max_degree != 0 && (inner_max_degree < 4 || inner_max_degree > 4096)
      || leaf_max_degree != 0 && (leaf_max_degree < 3 || leaf_max_degree > 4096)

  -- Exhaustive over the hint-validation aborts (parameter ordering,
  -- division by zero, u64 overflow of the entry-size sums, and the
  -- hint-derived degree thresholds). The internal storage-allocator
  -- alignment asserts fall under the structural presumption above, and
  -- the degrees passed on to `new_with_config` are within its bounds by
  -- construction (clamped between the *_MIN_DEGREE thresholds asserted
  -- here and `MAX_DEGREE`).
  spec fun spec_unchanged_except_at {K has Copy, Drop, Store} {V has Store}(
    self : BigOrderedMap<K, V>, key : K
  ) : Bool :=
    ∀ (k : K),
      (k != key
        ==> spec_contains_key(self, k) == spec_contains_key(old(self), k))
        && (∀ (k : K),
          k != key && spec_contains_key(old(self), k)
            ==> spec_get(self, k) == spec_get(old(self), k))

  -- Intrinsic (`map_iter_borrow_mut`): the returned `&mut V` carries a table
  -- index edge, so caller write-back updates the abstract map at `self.key`
  -- instead of traversing intrinsic internals. The body's constant-value-size
  -- assert is covered by the size presumption above.
  @[map_spec_aborts_iter_borrow_mut (BigOrderedMap)]
  spec fun spec_aborts_iter_borrow_mut {K} {V}(
    self : IteratorPtr<K>, map : BigOrderedMap<K, V>
  ) : Bool :=
    self is End || !spec_contains_key(map, self.key)

  -- Spec-level mirror of `iter_is_begin`. The Move body reads intrinsic map
  -- internals, so the function itself cannot appear in spec expressions.
  -- self is End: begin iff map is empty (End acts as both begin and end on []).
  -- self is Some: begin iff self.key is the smallest key currently in map.
  spec fun spec_iter_is_begin {K} {V}(
    self : IteratorPtr<K>, map : BigOrderedMap<K, V>
  ) : Bool :=
    if self is End then spec_len(map) == 0
    else
      spec_contains_key(map, self.key)
        && (∀ (k : K),
          spec_contains_key(map, k) && k != self.key
            ==> compare(self.key, k) == new Ordering::Less {})

  -- The smallest key occupies position 0, so a non-End begin iterator
  -- sits at rank 0. Stated here because the characterization above is by
  -- `cmp::compare` minimality, which does not by itself reach the
  -- enumeration; a backward traversal needs this to conclude at begin
  -- that it has walked the whole map.
  -- Returns the iterator pointing to the smallest key K in self with K >= input
  -- key (compare not Less), or End if no such key exists.
  -- End iff no key >= input exists (all keys are Less than input).
  -- Otherwise, result.key is in the map, >= input, and the smallest such.
  -- Allocates vacant storage slots only: map content and iterator
  -- navigation are untouched, so iterators stay valid.
  -- An existing-key upsert replaces the value in place (`add_at`
  -- overwrites before ever splitting): not a structural mutation. The
  -- intrinsic model preserves iterator validity on that branch, so no
  -- annotation is needed here (see `test_verify_iter_across_upsert`).
  -- result.key is the smallest key in the map.
  -- The first key has rank 0.
  -- End iff self.key has no strict successor in the map.
  -- Otherwise result.key is the smallest in-map key strictly greater than self.key.
  -- Rank increments per step; End means self.key had the last rank.
  -- A predecessor always exists when self is not begin; from End the result
  -- is the largest key. The result always points at an in-map key.
  -- From End: result.key is the largest key in the map.
  -- Otherwise result.key is the largest in-map key strictly less than self.key.
  -- From End: result has the last rank; otherwise the rank decrements.
  -- The closure's contract (`aborts_of`/`ensures_of` below) is only
  -- established for inputs satisfying its precondition — the closure is
  -- verified under `requires_of` — so importing it is sound only when
  -- the caller establishes that precondition on the current value. The
  -- end-iterator disjunct exempts calls that abort at the end-check
  -- before the closure is ever invoked (`self.key` does not exist
  -- there). Trivially true for closures without a `requires`. The
  -- body's post-callback size validation is covered by the size
  -- presumption above.
  -- A value modification is not a structural mutation: iterators stay valid.
  -- iter_modify mutates the value at self.key via the closure. Containment is
  -- unchanged for every key; values for keys other than self.key are preserved;
  -- the closure's contract relates the old value, the new value, and the result.
  -- A value write moves no keys, so every position survives. This has to
  -- be stated positionally: equality against `spec_set` would not do it,
  -- because equality on a map carrying ghosts is extensional (see
  -- `$IsEqual` in the prelude), so it never produces the write term the
  -- model's rank-preservation axiom triggers on. Same content as that
  -- axiom, so no new trust.
  -- TRANSPARENT (the body is a plain projection): equality in an opaque
  -- ensures would not carry the projected iterator's hidden validity slot;
  -- inlined value flow does.
  -- Removal closes the position up: keys before the removed one keep
  -- their place, keys after it move down by one. Positional for the same
  -- reason as in `iter_modify` — extensional equality against
  -- `spec_remove` cannot reach the enumeration.
  -- Position of a leaf in the walk: the number of keys held by the leaves
  -- before it. Uninterpreted — its meaning comes entirely from the clauses on
  -- the two leaf functions below, which say it starts at zero, advances by
  -- each leaf's size, and reaches the map's length when the walk ends. That
  -- is what lets a leaf walk carry a rank-indexed invariant, and what makes
  -- the walk COMPLETE rather than merely sound: the entries seen are the
  -- map's keys at positions `offset .. offset + leaf size`, and the offsets
  -- tile `0 .. spec_len(map)`.
  @[map_spec_leaf_offset (BigOrderedMap)]
  opaque spec fun spec_leaf_offset {K} {V}(
    leaf : LeafNodeIteratorPtr, map : BigOrderedMap<K, V>
  ) : Int

  -- Points at `min_leaf_index`, which is never NULL_INDEX: an empty map's
  -- leaf walk visits the (empty) root leaf once.
  -- Nothing precedes the first leaf.
  -- Every entry in the returned leaf is a real map entry (a Leaf child
  -- with contained key and matching value).
  -- Leaves of a nonempty map are nonempty.
  -- The other direction, which the soundness clauses above do not give:
  -- this leaf holds exactly the map's keys at positions
  -- `offset .. offset + leaf size`, in that order. Leaves are visited in
  -- ascending key order and each leaf's entries are ascending, so the
  -- leaf's own enumeration lines up with the map's at that offset.
  -- The same correspondence for values, stated positionally. The
  -- key-guarded clauses above cannot be used by a walk that has just read
  -- position `j`: that would first require the key at `j` to be known
  -- contained, which nothing about an opaque result gives. Leaf-ness is
  -- part of it — `.value` is a variant selector, so without it the value
  -- equality says nothing about the child a walk actually reads.
  -- Advancing consumes exactly this leaf's keys, and the walk ends only
  -- once every key has been consumed.
  -- An offset counts keys, so it never goes backwards past the start.
  -- Without this a walk could sit at a negative position, where a
  -- position-indexed aggregate is trivially zero.
