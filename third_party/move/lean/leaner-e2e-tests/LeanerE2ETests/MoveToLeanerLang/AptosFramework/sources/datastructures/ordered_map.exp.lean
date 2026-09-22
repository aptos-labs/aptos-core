-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
This module provides an implementation for an ordered map.

Keys point to values, and each key in the map must be unique.

Currently, one implementation is provided, backed by a single sorted vector.

That means that keys can be found within O(log N) time.
Adds and removals take O(N) time, but the constant factor is small,
as it does only O(log N) comparisons, and does efficient mem-copy with vector operations.

Additionally, it provides a way to lookup and iterate over sorted keys, making range query
take O(log N + R) time (where R is number of elements in the range).

Most methods operate with OrderedMap being `self`.
All methods that start with iter_*, operate with IteratorPtr being `self`.

Uses cmp::compare for ordering, which compares primitive types natively, and uses common
lexicographical sorting for complex types.

Warning: All iterator functions need to be carefully used, because they are just pointers into the
structure, and modification of the map invalidates them (without compiler being able to catch it).
Type is also named IteratorPtr, so that Iterator is free to use later.
Better guarantees would need future Move improvements that will allow references to be part of the struct,
allowing cleaner iterator APIs.

That's why all functions returning iterators are prefixed with "internal_", to clarify nuances needed to make
sure usage is correct.
A set of inline utility methods is provided instead, to provide guaranteed valid usage to iterators.
-/
leaner module 0x1::ordered_map where
  use 0x1::std::cmp::Ordering
  use 0x1::std::cmp::compare
  use 0x1::std::cmp::is_eq
  use 0x1::std::cmp::is_gt
  use 0x1::std::cmp::is_lt
  use 0x1::std::error::invalid_argument
  use 0x1::std::option
  use 0x1::std::option::Option
  use 0x1::std::option::is_none
  use 0x1::std::option::is_some
  use 0x1::std::option::none
  use 0x1::std::option::some
  use 0x1::std::option::spec_borrow
  use 0x1::std::option::spec_is_none
  use 0x1::std::option::spec_is_some
  use 0x1::std::vector
  use 0x1::std::vector::spec_contains

  friend aptos_framework::big_ordered_map;

  pragma verify

  /--
  Map key already exists
  -/
  const EKEY_ALREADY_EXISTS : u64 := 1

  /--
  Map key is not found
  -/
  const EKEY_NOT_FOUND : u64 := 2

  -- Trying to do an operation on an IteratorPtr that would go out of bounds
  const EITER_OUT_OF_BOUNDS : u64 := 3

  /--
  New key used in replace_key_inplace doesn't respect the order
  -/
  const ENEW_KEY_NOT_IN_ORDER : u64 := 4

  /--
  Individual entry holding (key, value) pair
  -/
  struct Entry {K} {V} has Copy, Drop, Store where
    key : K
    value : V

  /--
  The OrderedMap datastructure.
  -/
  @[intrinsic_map]
  enum OrderedMap {K} {V} has Copy, Drop, Store where
    | SortedVectorMap (entries : Vector<Entry<K, V> >)

  spec OrderedMap where
    pragma intrinsic = map

  /--
  An iterator pointing to a valid position in an ordered map, or to the end.

  TODO: Once fields can be (mutable) references, this class will be deprecated.
  -/
  enum IteratorPtr has Copy, Drop where
    | End
    | Position (index : u64)

  /--
  Create a new empty OrderedMap, using default (SortedVectorMap) implementation.
  -/
  @[map_new (OrderedMap)]
  public fun new {K} {V}() -> OrderedMap<K, V> :=
    new OrderedMap<K, V>::SortedVectorMap { entries := vector<Entry<K, V> >[] }

  spec new where
    pragma intrinsic

  /--
  Create a OrderedMap from a vector of keys and values.
  Aborts with EKEY_ALREADY_EXISTS if duplicate keys are passed in.
  -/
  @[map_new_from (OrderedMap)]
  public fun new_from {K} {V}(
    keys : Vector<K>, values : Vector<V>
  ) -> OrderedMap<K, V> := do
    let mut map := new::<K, V>()
    map.add_all(keys, values)
    return map

  spec new_from where
    pragma intrinsic

  /--
  Number of elements in the map.
  -/
  @[map_len (OrderedMap)]
  public fun length {K} {V}(self : &OrderedMap<K, V>) -> u64 :=
    self.entries.length

  spec length where
    pragma intrinsic

  /--
  Whether map is empty.
  -/
  @[map_is_empty (OrderedMap)]
  public fun is_empty {K} {V}(self : &OrderedMap<K, V>) -> Bool :=
    self.entries.is_empty()

  spec is_empty where
    pragma intrinsic

  /--
  Add a key/value pair to the map.
  Aborts with EKEY_ALREADY_EXISTS if key already exist.
  -/
  @[map_add_no_override (OrderedMap)]
  public fun add {K} {V}(
    self : &mut OrderedMap<K, V>, key : K, value : V
  ) -> Unit := do
    let len := self.entries.length
    let index := binary_search(&key, &self.entries, 0, len)
    assert!(
      index >= len || &self.entries[index].key != &key,
      invalid_argument(EKEY_ALREADY_EXISTS)
    )
    self.entries.insert(index, new Entry<K, V> { key, value })

  spec add where
    pragma intrinsic

  -- key must not already be inside.
  /--
  If the key doesn't exist in the map, inserts the key/value, and returns none.
  Otherwise, updates the value under the given key, and returns the old value.
  -/
  @[map_upsert (OrderedMap)]
  public fun upsert {K has Drop} {V}(
    self : &mut OrderedMap<K, V>, key : K, value : V
  ) -> Option<V> := do
    let len := self.entries.length
    let index := binary_search(&key, &self.entries, 0, len)
    return if index < len && &self.entries[index].key == &key then
      let Entry<K, V> { key := _, value := old_value } :=
        self.entries.replace(index, new Entry<K, V> { key, value })
      return some(old_value)
    else
      self.entries.insert(index, new Entry<K, V> { key, value })
      return none::<V>()

  spec upsert where
    pragma intrinsic

  /--
  Remove a key/value pair from the map.
  Aborts with EKEY_NOT_FOUND if `key` doesn't exist.
  -/
  @[map_del_must_exist (OrderedMap)]
  public fun remove {K has Drop} {V}(
    self : &mut OrderedMap<K, V>, key : &K
  ) -> V := do
    let len := self.entries.length
    let index := binary_search(key, &self.entries, 0, len)
    assert!(index < len, invalid_argument(EKEY_NOT_FOUND))
    let Entry<K, V> { key := old_key, value := value } :=
      self.entries.remove(index)
    assert!(key == &old_key, invalid_argument(EKEY_NOT_FOUND))
    return value

  spec remove where
    pragma intrinsic

  /--
  Remove a key/value pair from the map.
  Returns none if `key` doesn't exist.
  -/
  @[map_remove_or_none (OrderedMap)]
  public fun remove_or_none {K has Drop} {V}(
    self : &mut OrderedMap<K, V>, key : &K
  ) -> Option<V> := do
    let len := self.entries.length
    let index := binary_search(key, &self.entries, 0, len)
    return if index < len && key == &self.entries[index].key then
      let Entry<K, V> { key := _, value := value } := self.entries.remove(index)
      return some(value)
    else none::<V>()

  spec remove_or_none where
    pragma intrinsic

  /--
  Returns whether map contains a given key.
  -/
  @[map_has_key (OrderedMap)]
  public fun contains {K} {V}(self : &OrderedMap<K, V>, key : &K) -> Bool :=
    !self.internal_find(key).iter_is_end(self)

  spec contains where
    pragma intrinsic

  @[map_borrow (OrderedMap)]
  public fun borrow {K} {V}(self : &OrderedMap<K, V>, key : &K) -> &V :=
    self.internal_find(key).iter_borrow(self)

  spec borrow where
    pragma intrinsic

  @[map_borrow_mut (OrderedMap)]
  public fun borrow_mut {K} {V}(
    self : &mut OrderedMap<K, V>, key : &K
  ) -> &mut V := self.internal_find(key).iter_borrow_mut(self)

  spec borrow_mut where
    pragma intrinsic

  @[map_get (OrderedMap)]
  public fun get {K has Copy, Drop, Store} {V has Copy, Store}(
    self : &OrderedMap<K, V>, key : &K
  ) -> Option<V> := do
    let iter := self.internal_find(key)
    return if iter.iter_is_end(self) then none::<V>()
    else some(*iter.iter_borrow(self))

  spec get where
    pragma intrinsic

  /--
  Changes the key, while keeping the same value attached to it
  Aborts with EKEY_NOT_FOUND if `old_key` doesn't exist.
  Aborts with ENEW_KEY_NOT_IN_ORDER if `new_key` doesn't keep the order `old_key` was in.
  -/
  @[map_replace_key_inplace (OrderedMap)]
  friend fun replace_key_inplace {K has Drop} {V}(
    self : &mut OrderedMap<K, V>, old_key : &K, new_key : K
  ) -> Unit := do
    let len := self.entries.length
    let index := binary_search(old_key, &self.entries, 0, len)
    assert!(index < len, invalid_argument(EKEY_NOT_FOUND))
    assert!(
      old_key == &self.entries[index].key, invalid_argument(
        EKEY_NOT_FOUND
      )
    )
    if index > 0 then
      assert!(
        is_lt(&compare(&self.entries[index - 1].key, &new_key)),
        invalid_argument(ENEW_KEY_NOT_IN_ORDER)
      )
    if index + 1 < len then
      assert!(
        is_lt(&compare(&new_key, &self.entries[index + 1].key)),
        invalid_argument(ENEW_KEY_NOT_IN_ORDER)
      )
    self.entries[index].key := new_key

  spec replace_key_inplace where
    pragma intrinsic

  -- check that after we update the key, order is going to be respected
  /--
  Add multiple key/value pairs to the map. The keys must not already exist.
  Aborts with EKEY_ALREADY_EXISTS if key already exist, or duplicate keys are passed in.
  -/
  @[map_add_all (OrderedMap)]
  public fun add_all {K} {V}(
    self : &mut OrderedMap<K, V>, keys : Vector<K>, values : Vector<V>
  ) -> Unit := do
    let mut (self', v2) := (keys, values)
    self'.reverse()
    v2.reverse()
    let mut (self', v2) := (self', v2)
    spec assume folds_capture_anchor!(25)
    let len := self'.length
    assert!(len == v2.length, 131074)
    while len > 0 do
      let (e1, e2) := (self'.pop_back(), v2.pop_back())
      let (key, value) := (e1, e2)
      self.add(key, value)
      len := len - 1
    where
      invariant with_state_anchor!(25, old(self')).length >= len
      invariant len == self'.length
      invariant len == v2.length
      invariant with_state_anchor!(25, old(self')).length
        == with_state_anchor!(25, old(v2)).length
      invariant ∀ (j in 0 .. len),
        self'[j] == with_state_anchor!(25, old(self'))[j]
      invariant ∀ (j in 0 .. len), v2[j] == with_state_anchor!(25, old(v2))[j]
      invariant ∀ (j in len .. with_state_anchor!(25, old(self')).length), true
      invariant true
    self'.destroy_empty()
    v2.destroy_empty()

  spec add_all where
    pragma intrinsic

  -- TODO: Can be optimized, by sorting keys and values, and then creating map.
  /--
  Add multiple key/value pairs to the map, overwrites values if they exist already,
  or if duplicate keys are passed in.
  -/
  @[map_upsert_all (OrderedMap)]
  public fun upsert_all {K has Drop} {V has Drop}(
    self : &mut OrderedMap<K, V>, keys : Vector<K>, values : Vector<V>
  ) -> Unit := do
    let mut (self', v2) := (keys, values)
    self'.reverse()
    v2.reverse()
    let mut (self', v2) := (self', v2)
    spec assume folds_capture_anchor!(43)
    let len := self'.length
    assert!(len == v2.length, 131074)
    while len > 0 do
      let (e1, e2) := (self'.pop_back(), v2.pop_back())
      let (key, value) := (e1, e2)
      self.upsert(key, value)
      len := len - 1
    where
      invariant with_state_anchor!(43, old(self')).length >= len
      invariant len == self'.length
      invariant len == v2.length
      invariant with_state_anchor!(43, old(self')).length
        == with_state_anchor!(43, old(v2)).length
      invariant ∀ (j in 0 .. len),
        self'[j] == with_state_anchor!(43, old(self'))[j]
      invariant ∀ (j in 0 .. len), v2[j] == with_state_anchor!(43, old(v2))[j]
      invariant ∀ (j in len .. with_state_anchor!(43, old(self')).length), true
      invariant true
    self'.destroy_empty()
    v2.destroy_empty()

  spec upsert_all where
    pragma intrinsic

  -- TODO: Can be optimized, by sorting keys and values, and then creating map.
  /--
  Takes all elements from `other` and adds them to `self`,
  overwritting if any key is already present in self.
  -/
  @[map_append (OrderedMap)]
  public fun append {K has Drop} {V has Drop}(
    self : &mut OrderedMap<K, V>, other : OrderedMap<K, V>
  ) -> Unit := do
    self.append_impl(other)

  spec append where
    pragma intrinsic

  /--
  Takes all elements from `other` and adds them to `self`.
  Aborts with EKEY_ALREADY_EXISTS if `other` has a key already present in `self`.
  -/
  @[map_append_disjoint (OrderedMap)]
  public fun append_disjoint {K} {V}(
    self : &mut OrderedMap<K, V>, other : OrderedMap<K, V>
  ) -> Unit := do
    let overwritten := self.append_impl(other)
    assert!(overwritten.length == 0, invalid_argument(EKEY_ALREADY_EXISTS))
    overwritten.destroy_empty()

  spec append_disjoint where
    pragma intrinsic

  /--
  Takes all elements from `other` and adds them to `self`, returning list of entries in self that were overwritten.
  -/
  fun append_impl {K} {V}(
    self : &mut OrderedMap<K, V>, other : OrderedMap<K, V>
  ) -> Vector<Entry<K, V> > := do
    let mut OrderedMap<K, V>::SortedVectorMap { entries := other_entries } :=
      other
    let mut overwritten := vector<Entry<K, V> >[]
    if other_entries.is_empty() then
      other_entries.destroy_empty()
      return overwritten;
    if self.entries.is_empty() then
      self.entries.append(other_entries)
      return overwritten;
    if is_lt(
      &compare(
        &self.entries[self.entries.length
          - 1].key, &other_entries[0].key
      )
    ) then
      self.entries.append(other_entries)
      return overwritten;
    let mut reverse_result := vector<Entry<K, V> >[]
    let cur_i := self.entries.length - 1
    let other_i := other_entries.length - 1
    loop do
      let ord := compare(&self.entries[cur_i].key, &other_entries[other_i].key)
      if is_gt(&ord) then
        reverse_result := core.prim.pushVector(
          reverse_result, self.entries.pop_back()
        )
        if cur_i == 0 then
          self.entries.append(other_entries)
          break
        else cur_i := cur_i - 1
      else
        if is_eq(&ord) then
          overwritten := core.prim.pushVector(
            overwritten, self.entries.pop_back()
          )
          if cur_i == 0 then
            self.entries.append(other_entries)
            break
          else cur_i := cur_i - 1
        reverse_result := core.prim.pushVector(
          reverse_result, other_entries.pop_back()
        )
        if other_i == 0 then
          other_entries.destroy_empty()
          break
        else other_i := other_i - 1
    self.entries.reverse_append(reverse_result)
    return overwritten

  spec append_impl where
    pragma opaque
    pragma verify = false

  -- Optimization: if all elements in `other` are larger than all elements in `self`, we can just move them over.
  -- In O(n), traversing from the back, build reverse sorted result, and then reverse it back
  -- after the end of the loop, other_entries is empty, and any leftover is in entries
  -- make other_entries empty, and rest in entries.
  -- TODO cannot use mem::swap until it is public/released
  -- mem::swap(&mut self.entries, &mut other_entries);
  -- is_lt or is_eq
  -- we skip the entries one, and below put in the result one from other.
  -- make other_entries empty, and rest in entries.
  -- TODO cannot use mem::swap until it is public/released
  -- mem::swap(&mut self.entries, &mut other_entries);
  /--
  Splits the collection into two, such to leave `self` with `at` number of elements.
  Returns a newly allocated map containing the elements in the range [at, len).
  After the call, the original map will be left containing the elements [0, at).
  -/
  @[map_trim (OrderedMap)]
  public fun trim {K} {V}(
    self : &mut OrderedMap<K, V>, at : u64
  ) -> OrderedMap<K, V> := do
    let rest := self.entries.trim(at)
    return new OrderedMap<K, V>::SortedVectorMap { entries := rest }

  spec trim where
    pragma intrinsic

  @[map_borrow_front (OrderedMap)]
  public fun borrow_front {K} {V}(self : &OrderedMap<K, V>) -> (&K, &V) := do
    let «entry» := &self.entries[0]
    return (&«entry».key, &«entry».value)

  spec borrow_front where
    pragma intrinsic

  @[map_borrow_back (OrderedMap)]
  public fun borrow_back {K} {V}(self : &OrderedMap<K, V>) -> (&K, &V) := do
    let «entry» := &self.entries[self.entries.length - 1]
    return (&«entry».key, &«entry».value)

  spec borrow_back where
    pragma intrinsic

  @[map_pop_front (OrderedMap)]
  public fun pop_front {K} {V}(self : &mut OrderedMap<K, V>) -> (K, V) := do
    let Entry<K, V> { key := key, value := value } := self.entries.remove(0)
    return (key, value)

  spec pop_front where
    pragma intrinsic

  @[map_pop_back (OrderedMap)]
  public fun pop_back {K} {V}(self : &mut OrderedMap<K, V>) -> (K, V) := do
    let Entry<K, V> { key := key, value := value } := self.entries.pop_back()
    return (key, value)

  spec pop_back where
    pragma intrinsic

  @[map_prev_key (OrderedMap)]
  public fun prev_key {K has Copy} {V}(
    self : &OrderedMap<K, V>, key : &K
  ) -> Option<K> := do
    let it := self.internal_lower_bound(key)
    return if it.iter_is_begin(self) then none::<K>()
    else some(*it.iter_prev(self).iter_borrow_key(self))

  spec prev_key where
    pragma intrinsic

  @[map_next_key (OrderedMap)]
  public fun next_key {K has Copy} {V}(
    self : &OrderedMap<K, V>, key : &K
  ) -> Option<K> := do
    let it := self.internal_lower_bound(key)
    return if it.iter_is_end(self) then none::<K>()
    else
      let cur_key := it.iter_borrow_key(self)
      return if key == cur_key then
        let it := it.iter_next(self)
        return if it.iter_is_end(self) then none::<K>()
        else some(*it.iter_borrow_key(self))
      else some(*cur_key)

  spec next_key where
    pragma intrinsic

  -- TODO: see if it is more understandable if iterator points between elements,
  -- and there is iter_borrow_next and iter_borrow_prev, and provide iter_insert.
  -- This is called "cursor" in rust instead.
  /--
  Warning: Marked as internal, as it is safer to utilize provided inline functions instead.
  For direct usage of this method, check Warning at the top of the file corresponding to iterators.

  Returns an iterator pointing to the first element that is greater or equal to the provided
  key, or an end iterator if such element doesn't exist.
  -/
  public fun internal_lower_bound {K} {V}(
    self : &OrderedMap<K, V>, key : &K
  ) -> IteratorPtr := do
    let entries := &self.entries
    let len := entries.length
    let index := binary_search(key, entries, 0, len)
    return if index == len then self.internal_new_end_iter()
    else
      let index := index
      return new IteratorPtr::Position { index }

  spec internal_lower_bound where
    pragma opaque
    pragma verify = false
    aborts_if false
    ensures result is End
        <==> (∀ (i in 0 .. spec_len(self)),
          compare(spec_key_at(self, i), key) == new Ordering::Less {})
    ensures !(result is End) ==> result.index < spec_len(self)
    ensures !(result is End)
        ==> compare(spec_key_at(self, result.index), key)
          != new Ordering::Less {}
    ensures !(result is End)
        ==> (∀ (i in 0 .. result.index),
          compare(spec_key_at(self, i), key) == new Ordering::Less {})
    ensures spec_contains_key(self, key) ==> !(result is End)

  /--
  Warning: Marked as internal, as it is safer to utilize provided inline functions instead.
  For direct usage of this method, check Warning at the top of the file corresponding to iterators.

  Returns an iterator pointing to the element that equals to the provided key, or an end
  iterator if the key is not found.
  -/
  public fun internal_find {K} {V}(
    self : &OrderedMap<K, V>, key : &K
  ) -> IteratorPtr := do
    let internal_lower_bound := self.internal_lower_bound(key)
    return if internal_lower_bound.iter_is_end(self) then internal_lower_bound
    else
      if internal_lower_bound.iter_borrow_key(self) == key then
        internal_lower_bound
      else self.internal_new_end_iter()

  spec internal_find where
    pragma opaque
    pragma verify = false
    aborts_if false
    ensures result is End <==> !spec_contains_key(self, key)
    ensures !(result is End) ==> result.index == spec_rank(self, key)

  /--
  Warning: Marked as internal, as it is safer to utilize provided inline functions instead.
  For direct usage of this method, check Warning at the top of the file corresponding to iterators.

  Returns the begin iterator.
  -/
  public fun internal_new_begin_iter {K} {V}(
    self : &OrderedMap<K, V>
  ) -> IteratorPtr := do
    if self.is_empty() then return new IteratorPtr::End {};
    let index := 0
    return new IteratorPtr::Position { index }

  spec internal_new_begin_iter where
    pragma opaque
    pragma verify = false
    aborts_if false
    ensures result is End <==> spec_len(self) == 0
    ensures !(result is End) ==> result.index == 0

  /--
  Warning: Marked as internal, as it is safer to utilize provided inline functions instead.
  For direct usage of this method, check Warning at the top of the file corresponding to iterators.

  Returns the end iterator.
  -/
  public fun internal_new_end_iter {K} {V}(
    self : &OrderedMap<K, V>
  ) -> IteratorPtr := new IteratorPtr::End {}

  spec internal_new_end_iter where
    pragma opaque
    pragma verify = false
    aborts_if false
    ensures result is End

  -- ========== Section for methods opearting on iterators ========
  -- Note: After any modifications to the map, do not use any of the iterators obtained beforehand.
  -- Operations on iterators after map is modified are unexpected/incorrect.
  /--
  Returns the next iterator, or none if already at the end iterator.
  Note: Requires that the map is not changed after the input iterator is generated.
  -/
  public fun iter_next {K} {V}(
    self : IteratorPtr, map : &OrderedMap<K, V>
  ) -> IteratorPtr := do
    assert!(!self.iter_is_end(map), invalid_argument(EITER_OUT_OF_BOUNDS))
    let index := self.index + 1
    return if map.entries.length > index then
      let index := index
      return new IteratorPtr::Position { index }
    else map.internal_new_end_iter()

  spec iter_next where
    pragma opaque
    pragma verify = false
    aborts_if self is End
    ensures result is End <==> self.index + 1 >= spec_len(map)
    ensures !(result is End) ==> result.index == self.index + 1

  /--
  Returns the previous iterator, or none if already at the begin iterator.
  Note: Requires that the map is not changed after the input iterator is generated.
  -/
  public fun iter_prev {K} {V}(
    self : IteratorPtr, map : &OrderedMap<K, V>
  ) -> IteratorPtr := do
    assert!(!self.iter_is_begin(map), invalid_argument(EITER_OUT_OF_BOUNDS))
    let index := if self is End then map.entries.length - 1 else self.index - 1
    let index := index
    return new IteratorPtr::Position { index }

  spec iter_prev where
    pragma opaque
    pragma verify = false
    aborts_if if self is End then spec_len(map) == 0 else self.index == 0
    ensures !(result is End)
    ensures self is End ==> result.index == spec_len(map) - 1
    ensures !(self is End) ==> result.index == self.index - 1

  /--
  Returns whether the iterator is a begin iterator.
  -/
  public fun iter_is_begin {K} {V}(
    self : &IteratorPtr, map : &OrderedMap<K, V>
  ) -> Bool :=
    if self is End then map.is_empty() else self.index == 0

  spec iter_is_begin where
    pragma opaque
    pragma verify = false
    aborts_if false
    ensures result
        <==> (if self is End then spec_len(map) == 0 else self.index == 0)

  /--
  Returns true iff the iterator is a begin iterator from a non-empty collection.
  (I.e. if iterator points to a valid element)
  This method doesn't require having access to map, unlike iter_is_begin.
  -/
  public fun iter_is_begin_from_non_empty(self : &IteratorPtr) -> Bool :=
    if self is End then false else self.index == 0

  spec iter_is_begin_from_non_empty where
    pragma opaque
    pragma verify = false

  /--
  Returns whether the iterator is an end iterator.
  -/
  public fun iter_is_end {K} {V}(
    self : &IteratorPtr, _map : &OrderedMap<K, V>
  ) -> Bool := self is End

  spec iter_is_end where
    pragma opaque
    pragma verify = false
    aborts_if false
    ensures result <==> self is End

  /--
  Borrows the key given iterator points to.
  Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the end.
  Note: Requires that the map is not changed after the input iterator is generated.
  -/
  public fun iter_borrow_key {K} {V}(
    self : &IteratorPtr, map : &OrderedMap<K, V>
  ) -> &K := do
    assert!(!(self is End), invalid_argument(EITER_OUT_OF_BOUNDS))
    return &map.entries[self.index].key

  spec iter_borrow_key where
    pragma opaque
    pragma verify = false
    aborts_if self is End || self.index >= spec_len(map)
    ensures result == spec_key_at(map, self.index)

  /--
  Borrows the value given iterator points to.
  Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the end.
  Note: Requires that the map is not changed after the input iterator is generated.
  -/
  public fun iter_borrow {K} {V}(
    self : IteratorPtr, map : &OrderedMap<K, V>
  ) -> &V := do
    assert!(!(self is End), invalid_argument(EITER_OUT_OF_BOUNDS))
    return &map.entries[self.index].value

  spec iter_borrow where
    pragma opaque
    pragma verify = false
    aborts_if self is End || self.index >= spec_len(map)
    ensures result == spec_get(map, spec_key_at(map, self.index))

  /--
  Mutably borrows the value iterator points to.
  Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the end.
  Note: Requires that the map is not changed after the input iterator is generated.
  -/
  @[map_iter_borrow_mut (OrderedMap)]
  public fun iter_borrow_mut {K} {V}(
    self : IteratorPtr, map : &mut OrderedMap<K, V>
  ) -> &mut V := do
    assert!(!(self is End), invalid_argument(EITER_OUT_OF_BOUNDS))
    return &mut map.entries[self.index].value

  spec iter_borrow_mut where
    pragma intrinsic

  /--
  Removes (key, value) pair iterator points to, returning the previous value.
  Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the end.
  Note: Requires that the map is not changed after the input iterator is generated.
  -/
  public fun iter_remove {K has Drop} {V}(
    self : IteratorPtr, map : &mut OrderedMap<K, V>
  ) -> V := do
    assert!(!(self is End), invalid_argument(EITER_OUT_OF_BOUNDS))
    let Entry<K, V> { key := _, value := value } :=
      map.entries.remove(self.index)
    return value

  spec iter_remove where
    pragma opaque
    pragma verify = false
    aborts_if self is End || self.index >= spec_len(map)
    ensures result == old(spec_get(map, spec_key_at(map, self.index)))
    ensures spec_len(map) == spec_len(old(map)) - 1
    ensures !spec_contains_key(map, old(spec_key_at(map, self.index)))
    ensures ∀ (i in 0 .. self.index),
        spec_key_at(map, i) == spec_key_at(old(map), i)
    ensures ∀ (i in self.index .. spec_len(map)),
        spec_key_at(map, i) == spec_key_at(old(map), i + 1)
    ensures ∀ (k : K),
        k != old(spec_key_at(map, self.index)) && old(spec_contains_key(map, k))
          ==> spec_contains_key(map, k)
            && spec_get(map, k) == old(spec_get(map, k))

  /--
  Replaces the value iterator is pointing to, returning the previous value.
  Aborts with EITER_OUT_OF_BOUNDS if iterator is pointing to the end.
  Note: Requires that the map is not changed after the input iterator is generated.
  -/
  public fun iter_replace {K has Copy, Drop} {V}(
    self : IteratorPtr, map : &mut OrderedMap<K, V>, value : V
  ) -> V := do
    assert!(!(self is End), invalid_argument(EITER_OUT_OF_BOUNDS))
    let key := map.entries[self.index].key
    let Entry<K, V> { key := _, value := prev_value } :=
      map.entries.replace(self.index, new Entry<K, V> { key, value })
    return prev_value

  spec iter_replace where
    pragma opaque
    pragma verify = false
    aborts_if self is End || self.index >= spec_len(map)
    ensures result == old(spec_get(map, spec_key_at(map, self.index)))
    ensures spec_get(map, old(spec_key_at(map, self.index))) == value
    ensures spec_len(map) == spec_len(old(map))
    ensures ∀ (i in 0 .. spec_len(map)),
        spec_key_at(map, i) == spec_key_at(old(map), i)
    ensures ∀ (k : K),
        spec_contains_key(map, k) == old(spec_contains_key(map, k))
    ensures ∀ (k : K),
        k != old(spec_key_at(map, self.index)) && old(spec_contains_key(map, k))
          ==> spec_get(map, k) == old(spec_get(map, k))

  -- TODO once mem::replace is public/released, update to:
  -- let entry = map.entries.borrow_mut(self.index);
  -- mem::replace(&mut entry.value, value)
  /--
  Add key/value pair to the map, at the iterator position (before the element at the iterator position).
  Aborts with ENEW_KEY_NOT_IN_ORDER is key is not larger than the key before the iterator,
  or smaller than the key at the iterator position.
  -/
  public fun iter_add {K} {V}(
    self : IteratorPtr, map : &mut OrderedMap<K, V>, key : K, value : V
  ) -> Unit := do
    let len := map.entries.length
    let insert_index := if self is End then len else self.index
    if insert_index > 0 then
      assert!(
        is_lt(&compare(&map.entries[insert_index - 1].key, &key)),
        invalid_argument(ENEW_KEY_NOT_IN_ORDER)
      )
    if insert_index < len then
      assert!(
        is_lt(&compare(&key, &map.entries[insert_index].key)),
        invalid_argument(ENEW_KEY_NOT_IN_ORDER)
      )
    map.entries.insert(insert_index, new Entry<K, V> { key, value })

  spec iter_add where
    pragma opaque
    pragma verify = false
    aborts_if !(self is End) && self.index > spec_len(map)
    aborts_if spec_iter_add_index(self, map) > 0
        && compare(spec_key_at(map, spec_iter_add_index(self, map) - 1), key)
          != new Ordering::Less {}
    aborts_if spec_iter_add_index(self, map) < spec_len(map)
        && compare(key, spec_key_at(map, spec_iter_add_index(self, map)))
          != new Ordering::Less {}
    ensures spec_len(map) == spec_len(old(map)) + 1
    ensures spec_contains_key(map, key) && spec_get(map, key) == value
    ensures spec_rank(map, key) == spec_iter_add_index(self, old(map))
    ensures ∀ (i in 0 .. spec_iter_add_index(self, old(map))),
        spec_key_at(map, i) == spec_key_at(old(map), i)
    ensures ∀ (i in spec_iter_add_index(self, old(map)) + 1 .. spec_len(map)),
        spec_key_at(map, i) == spec_key_at(old(map), i - 1)
    ensures ∀ (k : K),
        k != key && old(spec_contains_key(map, k))
          ==> spec_contains_key(map, k)
            && spec_get(map, k) == old(spec_get(map, k))

  /--
  Destroys empty map.
  Aborts if `self` is not empty.
  -/
  @[map_destroy_empty (OrderedMap)]
  public fun destroy_empty {K} {V}(self : OrderedMap<K, V>) -> Unit := do
    let OrderedMap<K, V>::SortedVectorMap { entries := entries } := self
    entries.destroy_empty()

  spec destroy_empty where
    pragma intrinsic

  -- assert!(entries.is_empty(), E_NOT_EMPTY);
  -- ========= Section with views and inline for-loop methods =======
  /--
  Return all keys in the map. This requires keys to be copyable.
  -/
  @[map_keys (OrderedMap)]
  public fun keys {K has Copy} {V}(self : &OrderedMap<K, V>) -> Vector<K> := do
    let self := &self.entries
    spec assume spec.inlineCallSummary(
      «spec_map_ref$lambda$0»(self, self.length),
      «spec_map_ref_aborts$lambda$1»(self, self.length)
    )
    let _inline_summary_result_33 :=
      do
        let mut result := vector<K>[]
        let i := 0
        let len := self.length
        while i < len do
          result := core.prim.pushVector(
            result,
            do
              let e := &self[i]
              let e := e
              return e.key)
          i := i + 1
        where
          invariant i <= len
          invariant result.length == i
          invariant result == «spec_map_ref$lambda$0»(self, i)
          invariant !«spec_map_ref_aborts$lambda$1»(self, i)
          invariant ∀ (j in 0 .. i), result[j] == self[j].key
          invariant ∀ (j in 0 .. i), !false
        return result
    spec assert _inline_summary_result_33
      == «spec_map_ref$lambda$0»(self, self.length)
    return _inline_summary_result_33

  spec keys where
    pragma intrinsic

  /--
  Return all values in the map. This requires values to be copyable.
  -/
  @[map_values (OrderedMap)]
  public fun values {K} {V has Copy}(
    self : &OrderedMap<K, V>
  ) -> Vector<V> := do
    let self := &self.entries
    spec assume spec.inlineCallSummary(
      «spec_map_ref$lambda$2»(self, self.length),
      «spec_map_ref_aborts$lambda$3»(self, self.length)
    )
    let _inline_summary_result_38 :=
      do
        let mut result := vector<V>[]
        let i := 0
        let len := self.length
        while i < len do
          result := core.prim.pushVector(
            result,
            do
              let e := &self[i]
              let e := e
              return e.value)
          i := i + 1
        where
          invariant i <= len
          invariant result.length == i
          invariant result == «spec_map_ref$lambda$2»(self, i)
          invariant !«spec_map_ref_aborts$lambda$3»(self, i)
          invariant ∀ (j in 0 .. i), result[j] == self[j].value
          invariant ∀ (j in 0 .. i), !false
        return result
    spec assert _inline_summary_result_38
      == «spec_map_ref$lambda$2»(self, self.length)
    return _inline_summary_result_38

  spec values where
    pragma intrinsic

  /--
  Transform the map into two vectors with the keys and values respectively
  Primarily used to destroy a map
  -/
  @[map_to_vec_pair (OrderedMap)]
  public fun to_vec_pair {K} {V}(
    self : OrderedMap<K, V>
  ) -> (Vector<K>, Vector<V>) := do
    let mut keys := vector<K>[]
    let mut values := vector<V>[]
    let OrderedMap<K, V>::SortedVectorMap { entries := entries } := self
    let mut self := entries
    self.reverse()
    let mut self := self
    spec assume folds_capture_anchor!(39)
    let len := self.length
    while len > 0 do
      let e := self.pop_back()
      let e := e
      let Entry<K, V> { key := key, value := value } := e
      keys := core.prim.pushVector(keys, key)
      values := core.prim.pushVector(values, value)
      len := len - 1
    where
      invariant with_state_anchor!(39, old(self)).length >= len
      invariant len == self.length
      invariant ∀ (j in 0 .. len),
        self[j] == with_state_anchor!(39, old(self))[j]
      invariant ∀ (j in len .. with_state_anchor!(39, old(self)).length), true
      invariant true
    self.destroy_empty()
    return (keys, values)

  spec to_vec_pair where
    pragma intrinsic

  -- return index containing the key, or insert position.
  -- I.e. index of first element that has key larger or equal to the passed `key` argument.
  fun binary_search {K} {V}(
    key : &K, entries : &Vector<Entry<K, V> >, start : u64, end : u64
  ) -> u64 := do
    let l := start
    let r := end
    while l != r do
      let mid := l + (r - l >> 1u8)
      let comparison := compare(&entries[mid].key, key)
      if is_lt(&comparison) then l := mid + 1 else r := mid
    return l

  spec binary_search where
    pragma opaque
    pragma verify = false

  -- see if useful, and add
  --
  -- public fun iter_num_below<K, V>(self: IteratorPtr, map: &OrderedMap<K, V>): u64 {
  --     if (self.iter_is_end()) {
  --         map.entries.length()
  --     } else {
  --         self.index
  --     }
  -- }
  -- ================= Section for tests =====================
  fun test_verify_borrow_front_key() -> Unit := do
    let keys := vector<u64>[1, 2, 3]
    let values := vector<u64>[4, 5, 6]
    let map := new_from(keys, values)
    let (key, value) := map.borrow_front()
    spec do
      assert keys[0] == 1
      assert spec_contains(keys, 1)
      assert spec_contains_key(map, key)
      assert spec_get(map, key) == value
      assert key == 1

  fun test_verify_borrow_back_key() -> Unit := do
    let keys := vector<u64>[1, 2, 3]
    let values := vector<u64>[4, 5, 6]
    let map := new_from(keys, values)
    let (key, value) := map.borrow_back()
    spec do
      assert keys[2] == 3
      assert spec_contains(keys, 3)
      assert spec_contains_key(map, key)
      assert spec_get(map, key) == value
      assert key == 3

  fun test_verify_upsert() -> Unit := do
    let keys := vector<u64>[1, 2, 3]
    let values := vector<u64>[4, 5, 6]
    let mut map := new_from(keys, values)
    spec assert spec_len(map) == 3
    let (_key, _value) := map.borrow_back()
    let result_1 := map.upsert(4, 5)
    spec do
      assert spec_contains_key(map, 4)
      assert spec_get(map, 4) == 5
      assert is_none(result_1)
      assert spec_len(map) == 4
    let result_2 := map.upsert(4, 6)
    spec do
      assert spec_contains_key(map, 4)
      assert spec_get(map, 4) == 6
      assert is_some(result_2)
      assert 0x1::std::option::borrow(result_2) == 5
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

  fun test_verify_next_key() -> Unit := do
    let keys := vector<u64>[1, 2, 3]
    let values := vector<u64>[4, 5, 6]
    let map := new_from(keys, values)
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

  fun test_verify_prev_key() -> Unit := do
    let keys := vector<u64>[1, 2, 3]
    let values := vector<u64>[4, 5, 6]
    let map := new_from(keys, values)
    let result_1 := map.prev_key(&1)
    spec assert is_none(result_1)
    let result_2 := map.prev_key(&3)
    spec do
      assert keys[0] == 1
      assert spec_contains_key(map, 1)
      assert keys[1] == 2
      assert spec_contains_key(map, 2)
      assert is_some(result_2)

  fun test_aborts_if_new_from_1() -> OrderedMap<u64, u64> := do
    let keys := vector<u64>[1, 2, 3, 1]
    let values := vector<u64>[4, 5, 6, 7]
    spec do
      assert keys[0] == 1
      assert keys[3] == 1
    let map := new_from(keys, values)
    return map

  spec test_aborts_if_new_from_1 where
    aborts_if true

  fun test_aborts_if_new_from_2(
    keys : Vector<u64>, values : Vector<u64>
  ) -> OrderedMap<u64, u64> := do
    let map := new_from(keys, values)
    return map

  spec test_aborts_if_new_from_2 where
    aborts_if ∃ (i in 0 .. keys.length; j in 0 .. keys.length),
        i != j && keys[i] == keys[j]
    aborts_if keys.length != values.length

  fun test_aborts_if_remove(map : &mut OrderedMap<u64, u64>) -> Unit := do
    map.remove(&1)

  spec test_aborts_if_remove where
    aborts_if !spec_contains_key(map, 1)

  fun test_verify_remove_or_none() -> Unit := do
    let keys := vector<u64>[1, 2, 3]
    let values := vector<u64>[4, 5, 6]
    let mut map := new_from(keys, values)
    spec assert spec_len(map) == 3
    let (_key, _value) := map.borrow_back()
    spec do
      assert keys[0] == 1
      assert keys[1] == 2
      assert spec_contains_key(map, 1)
      assert spec_contains_key(map, 2)
    let result_1 := map.remove_or_none(&1)
    spec do
      assert spec_contains_key(map, 2)
      assert spec_get(map, 2) == 5
      assert spec_is_some(result_1)
      assert spec_borrow(result_1) == 4
      assert spec_len(map) == 2
      assert !spec_contains_key(map, 1)
      assert !spec_contains_key(map, 4)
    let result_2 := map.remove_or_none(&4)
    spec do
      assert spec_contains_key(map, 2)
      assert spec_get(map, 2) == 5
      assert spec_is_none(result_2)
      assert spec_len(map) == 2
      assert !spec_contains_key(map, 4)
    map.remove(&2)
    map.remove(&3)
    spec do
      assert !spec_contains_key(map, 1)
      assert !spec_contains_key(map, 2)
      assert !spec_contains_key(map, 3)
      assert spec_len(map) == 0
    map.destroy_empty()

  /--
  Witness-test support: derives the exact enumeration of a map holding
  keys {1, 2, 3}, walking the stepwise assert ladder once; callers get
  the facts from the ensures.
  -/
  fun ground_enum_123(map : &OrderedMap<u64, u64>) -> Unit := do
    let (_fk, _fv) := map.borrow_front()
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
  -- Ranks follow the key order; three distinct ordered positions
  -- in 0..3 pin them exactly, and with them the enumeration.
  fun test_verify_enumeration_view() -> Unit := do
    let keys := vector<u64>[1, 2, 3]
    let values := vector<u64>[4, 5, 6]
    let map := new_from(keys, values)
    let (fk, _fv) := map.borrow_front()
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

  -- Materialize ground membership facts for the quantifiers.
  -- Cross-checks: the ordering API agrees, and values compose.
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

  -- Materialize ground membership facts for the quantifiers.
  -- Popped key had rank 0; survivors' ranks shift down by one
  -- (mirrors the big_ordered_map test on the non-ghost template branch).
  -- Back border: the popped key had the last rank.
  fun test_verify_remove_shift_symbolic(
    m : &mut OrderedMap<u64, u64>, k : u64
  ) -> Unit := do
    m.remove(&k)

  spec test_verify_remove_shift_symbolic where
    requires spec_contains_key(m, k)
    aborts_if false
    ensures spec_len(m) == spec_len(old(m)) - 1
    ensures !spec_contains_key(m, k)
    ensures ∀ (i in 0 .. spec_len(m)),
        spec_key_at(m, i)
          == spec_key_at(old(m), if i < spec_rank(old(m), k) then i else i + 1)
    ensures ∀ (i in 0 .. spec_len(m)),
        spec_get(m, spec_key_at(m, i)) == spec_get(old(m), spec_key_at(m, i))

  -- Removal at an arbitrary rank, on the non-ghost template branch.
  fun test_verify_drain_symbolic(m : &mut OrderedMap<u64, u64>) -> u64 := do
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
    aborts_if false
    ensures result == spec_len(old(m))
    ensures spec_len(m) == 0

  fun test_verify_iter_collect_symbolic(
    m : &OrderedMap<u64, u64>
  ) -> Vector<u64> := do
    let mut out := vector<u64>[]
    let it := m.internal_new_begin_iter()
    while !it.iter_is_end(m) do
      out := core.prim.pushVector(out, *it.iter_borrow_key(m))
      it := it.iter_next(m)
    where
      invariant out.length <= spec_len(m)
      invariant ∀ (i in 0 .. out.length), out[i] == spec_key_at(m, i)
      invariant !(it is End)
        ==> it.index == out.length && it.index < spec_len(m)
      invariant it is End ==> out.length == spec_len(m)
    return out

  spec test_verify_iter_collect_symbolic where
    aborts_if false
    ensures result.length == spec_len(m)
    ensures ∀ (i in 0 .. spec_len(m)), result[i] == spec_key_at(m, i)

  -- A full iterator traversal of a symbolic map. OrderedMap's iterator is
  -- an index into the sorted entries, so the index is the position and the
  -- walk can be indexed by it directly.
  fun test_verify_iter_sum_symbolic(m : &OrderedMap<u64, u64>) -> u64 := do
    let sum := 0
    let it := m.internal_new_begin_iter()
    let count := 0
    while !it.iter_is_end(m) do
      sum := sum + *it.iter_borrow(m)
      it := it.iter_next(m)
      count := count + 1
    where
      invariant count <= spec_len(m)
      invariant !(it is End) ==> it.index == count && it.index < spec_len(m)
      invariant sum == spec_om_sum_upto(m, count)
      invariant it is End ==> count == spec_len(m)
    return sum

  spec test_verify_iter_sum_symbolic where
    ensures result == spec_om_sum_upto(m, spec_len(m))

  -- The value-side walk: reads through iter_borrow land on the entry at
  -- the current position.
  -- Aborts unspecified: the running u64 addition can overflow.
  fun test_verify_iter_borrow_mut_symbolic(
    m : &mut OrderedMap<u64, u64>, k : u64
  ) -> Unit := do
    let it := m.internal_find(&k)
    let v := it.iter_borrow_mut(m)
    *v := 7

  spec test_verify_iter_borrow_mut_symbolic where
    pragma verify
    requires spec_contains_key(m, k)
    aborts_if false
    ensures spec_get(m, k) == 7
    ensures spec_len(m) == spec_len(old(m))
    ensures ∀ (i in 0 .. spec_len(m)),
        spec_key_at(m, i) == spec_key_at(old(m), i)
    ensures ∀ (k2 : u64),
        k2 != k && spec_contains_key(old(m), k2)
          ==> spec_get(m, k2) == old(spec_get(m, k2))

  -- Writing through a position-based iterator: the borrow resolves the
  -- position to a key through the enumeration, so the write lands on that
  -- key and moves nothing.
  fun test_verify_iter_replace_symbolic(
    m : &mut OrderedMap<u64, u64>, i : u64
  ) -> u64 := do
    let it := new IteratorPtr::Position { index := i }
    return it.iter_replace(m, 7)

  spec test_verify_iter_replace_symbolic where
    pragma verify
    requires i < spec_len(m)
    aborts_if false
    ensures result == old(spec_get(m, spec_key_at(m, i)))
    ensures spec_get(m, old(spec_key_at(m, i))) == 7
    ensures spec_len(m) == spec_len(old(m))
    ensures ∀ (j in 0 .. spec_len(m)),
        spec_key_at(m, j) == spec_key_at(old(m), j)

  fun test_verify_iter_remove_shift_at_position(
    m : &mut OrderedMap<u64, u64>, i : u64
  ) -> u64 := do
    let it := new IteratorPtr::Position { index := i }
    return it.iter_remove(m)

  spec test_verify_iter_remove_shift_at_position where
    pragma verify
    requires i < spec_len(m)
    aborts_if false
    ensures result == old(spec_get(m, spec_key_at(m, i)))
    ensures spec_len(m) == spec_len(old(m)) - 1
    ensures !spec_contains_key(m, old(spec_key_at(m, i)))
    ensures ∀ (j in 0 .. i), spec_key_at(m, j) == spec_key_at(old(m), j)
    ensures ∀ (j in i .. spec_len(m)),
        spec_key_at(m, j) == spec_key_at(old(m), j + 1)

  fun test_verify_iter_add_append_symbolic(
    m : &mut OrderedMap<u64, u64>, k : u64
  ) -> Unit := do
    m.internal_new_end_iter().iter_add(m, k, 7)

  spec test_verify_iter_add_append_symbolic where
    pragma verify
    requires spec_len(m) == 0
        || compare(spec_key_at(m, spec_len(m) - 1), k) == new Ordering::Less {}
    aborts_if false
    ensures spec_len(m) == spec_len(old(m)) + 1
    ensures spec_get(m, k) == 7
    ensures spec_rank(m, k) == spec_len(old(m))
    ensures ∀ (j in 0 .. spec_len(old(m))),
        spec_key_at(m, j) == spec_key_at(old(m), j)

  -- Appending at the end iterator, the shape a queue uses: the new key
  -- takes the last position and every existing position is undisturbed.
  fun test_verify_lower_bound_rank_symbolic(
    m : &OrderedMap<u64, u64>, k : u64
  ) -> IteratorPtr := m.internal_lower_bound(&k)

  spec test_verify_lower_bound_rank_symbolic where
    pragma verify
    requires spec_contains_key(m, k)
    aborts_if false
    ensures !(result is End)
    ensures result.index == spec_rank(m, k)
    ensures spec_key_at(m, result.index) == k

  -- For a key that is present, the search lands exactly on its position:
  -- the characterization is by comparison, so reaching the enumeration
  -- needs the ascending order the model supplies.
  fun test_verify_lower_bound_gap_symbolic(
    m : &OrderedMap<u64, u64>, k : u64
  ) -> IteratorPtr := m.internal_lower_bound(&k)

  spec test_verify_lower_bound_gap_symbolic where
    pragma verify
    requires !spec_contains_key(m, k)
    aborts_if false
    ensures !(result is End)
        ==> compare(k, spec_key_at(m, result.index)) == new Ordering::Less {}
    ensures !(result is End) && result.index > 0
        ==> compare(spec_key_at(m, result.index - 1), k)
          == new Ordering::Less {}
    ensures result is End
        ==> (∀ (i in 0 .. spec_len(m)),
          compare(spec_key_at(m, i), k) == new Ordering::Less {})

  -- For a key that is absent, it lands on the first larger key, so an
  -- insert there keeps the order — the `iter_add` precondition.
  fun test_verify_iter_add_middle_symbolic(
    m : &mut OrderedMap<u64, u64>, i : u64, k : u64
  ) -> Unit := do
    let it := new IteratorPtr::Position { index := i }
    it.iter_add(m, k, 7)

  spec test_verify_iter_add_middle_symbolic where
    pragma verify
    requires i < spec_len(m)
    requires i == 0
        || compare(spec_key_at(m, i - 1), k) == new Ordering::Less {}
    requires compare(k, spec_key_at(m, i)) == new Ordering::Less {}
    aborts_if false
    ensures spec_len(m) == spec_len(old(m)) + 1
    ensures spec_get(m, k) == 7
    ensures spec_rank(m, k) == i
    ensures ∀ (j in 0 .. i), spec_key_at(m, j) == spec_key_at(old(m), j)
    ensures ∀ (j in i + 1 .. spec_len(m)),
        spec_key_at(m, j) == spec_key_at(old(m), j - 1)

  -- Inserting between two existing keys, which is the case an append
  -- cannot exercise: here the tail actually has to shift up.
  fun test_aborts_if_iter_add_out_of_order(
    m : &mut OrderedMap<u64, u64>, k : u64
  ) -> Unit := do
    m.internal_new_end_iter().iter_add(m, k, 7)

  spec test_aborts_if_iter_add_out_of_order where
    pragma verify
    requires spec_len(m) > 0
    requires compare(spec_key_at(m, spec_len(m) - 1), k)
        != new Ordering::Less {}
    aborts_if true

  -- A key that does not exceed the last one has no business at the end
  -- position; the body's ordering check rejects it, duplicates included.
  fun test_aborts_if_iter_remove_out_of_range(
    m : &mut OrderedMap<u64, u64>, i : u64
  ) -> u64 := do
    let it := new IteratorPtr::Position { index := i }
    return it.iter_remove(m)

  spec test_aborts_if_iter_remove_out_of_range where
    pragma verify
    requires i >= spec_len(m)
    aborts_if true

  fun test_verify_iter_borrow_mut_at_position(
    m : &mut OrderedMap<u64, u64>, i : u64
  ) -> Unit := do
    let it := new IteratorPtr::Position { index := i }
    *it.iter_borrow_mut(m) := 7

  spec test_verify_iter_borrow_mut_at_position where
    pragma verify
    requires i < spec_len(m)
    aborts_if false
    ensures spec_get(m, spec_key_at(old(m), i)) == 7
    ensures ∀ (j in 0 .. spec_len(m)),
        spec_key_at(m, j) == spec_key_at(old(m), j)
    ensures ∀ (j in 0 .. spec_len(m)),
        j != i
          ==> spec_get(m, spec_key_at(m, j))
            == old(spec_get(m, spec_key_at(m, j)))

  -- The borrow's positional meaning, stated directly: writing through the
  -- iterator at position `i` lands on the key sitting at `i`, and no other
  -- position is touched.
  fun test_aborts_if_iter_borrow_mut_end(
    m : &mut OrderedMap<u64, u64>
  ) -> Unit := do
    let it := m.internal_new_end_iter()
    *it.iter_borrow_mut(m) := 7

  spec test_aborts_if_iter_borrow_mut_end where
    pragma verify
    aborts_if true

  -- No value at the end iterator.
  fun test_aborts_if_iter_borrow_mut_out_of_range(
    m : &mut OrderedMap<u64, u64>, i : u64
  ) -> Unit := do
    let it := new IteratorPtr::Position { index := i }
    *it.iter_borrow_mut(m) := 7

  spec test_aborts_if_iter_borrow_mut_out_of_range where
    pragma verify
    requires i >= spec_len(m)
    aborts_if true

  -- Nor at a position past the last one — a stale iterator degrades to
  -- this once the map has shrunk under it.
  fun test_verify_iter_walk_mut_symbolic(
    m : &mut OrderedMap<u64, u64>
  ) -> Unit := do
    let it := m.internal_new_begin_iter()
    let count := 0
    while !it.iter_is_end(m) do
      *it.iter_borrow_mut(m) := 7
      it := it.iter_next(m)
      count := count + 1
    where
      invariant count <= spec_len(m)
      invariant spec_len(m) == spec_len(old(m))
      invariant ∀ (i in 0 .. spec_len(m)),
        spec_key_at(m, i) == spec_key_at(old(m), i)
      invariant !(it is End) ==> it.index == count && it.index < spec_len(m)
      invariant it is End ==> count == spec_len(m)
      invariant ∀ (i in 0 .. count), spec_get(m, spec_key_at(m, i)) == 7

  spec test_verify_iter_walk_mut_symbolic where
    pragma verify
    aborts_if false
    ensures spec_len(m) == spec_len(old(m))
    ensures ∀ (i in 0 .. spec_len(m)),
        spec_key_at(m, i) == spec_key_at(old(m), i)
    ensures ∀ (i in 0 .. spec_len(m)), spec_get(m, spec_key_at(m, i)) == 7

  -- The mutate-while-traversing shape: a walk that writes each value in
  -- place. Positions have to survive every write for the walk's own
  -- invariant to hold, which is what the enumeration-backed borrow gives.
  -- The ordering bindings below (`map_borrow_front`/`back`, `map_pop_front`/`back`,
  -- `map_prev_key`/`next_key`) presume `cmp::compare<K>` is a strict total order on K.
  -- Built-in K types satisfy this; user-defined K types must too for this spec block
  -- to be sound.
  @[map_spec_len (OrderedMap)]
  opaque spec fun spec_len {K} {V}(t : OrderedMap<K, V>) : Int

  @[map_spec_has_key (OrderedMap)]
  opaque spec fun spec_contains_key {K} {V}(t : OrderedMap<K, V>, k : K) : Bool

  -- Enumeration view (mirrors big_ordered_map): spec_key_at(t, i) is the
  -- i-th smallest key, spec_rank(t, k) its inverse on contained keys.
  @[map_spec_key_at (OrderedMap)]
  opaque spec fun spec_key_at {K} {V}(t : OrderedMap<K, V>, i : Int) : K

  spec fun spec_om_sum_upto(m : OrderedMap<u64, u64>, n : Int) : Int :=
    if n <= 0 then 0
    else spec_om_sum_upto(m, n - 1) + spec_get(m, spec_key_at(m, n - 1))

  @[map_spec_rank (OrderedMap)]
  opaque spec fun spec_rank {K} {V}(t : OrderedMap<K, V>, k : K) : Int

  @[map_spec_set (OrderedMap)]
  opaque spec fun spec_set {K} {V}(
    t : OrderedMap<K, V>, k : K, v : V
  ) : OrderedMap<K, V>

  @[map_spec_del (OrderedMap)]
  opaque spec fun spec_remove {K} {V}(
    t : OrderedMap<K, V>, k : K
  ) : OrderedMap<K, V>

  @[map_spec_get (OrderedMap)]
  opaque spec fun spec_get {K} {V}(t : OrderedMap<K, V>, k : K) : V

  @[map_spec_aborts_destroy_empty (OrderedMap)]
  opaque spec fun spec_aborts_destroy_empty {K} {V}(t : OrderedMap<K, V>) : Bool

  @[map_spec_aborts_add (OrderedMap)]
  opaque spec fun spec_aborts_add {K} {V}(
    t : OrderedMap<K, V>, k : K, v : V
  ) : Bool

  @[map_spec_aborts_del (OrderedMap)]
  opaque spec fun spec_aborts_del {K} {V}(t : OrderedMap<K, V>, k : K) : Bool

  @[map_spec_aborts_borrow (OrderedMap)]
  opaque spec fun spec_aborts_borrow {K} {V}(t : OrderedMap<K, V>, k : K) : Bool

  @[map_spec_aborts_empty (OrderedMap)]
  spec fun spec_aborts_empty {K} {V}(t : OrderedMap<K, V>) : Bool :=
    spec_len(t) == 0

  @[map_spec_aborts_add_all (OrderedMap)]
  spec fun spec_aborts_add_all {K} {V}(
    m : OrderedMap<K, V>, keys : Vector<K>, values : Vector<V>
  ) : Bool :=
    keys.length != values.length
      || (∃ (i in 0 .. keys.length), spec_contains_key(m, keys[i]))
      || (∃ (i in 0 .. keys.length; j in 0 .. keys.length),
        i != j && keys[i] == keys[j])

  @[map_spec_aborts_new_from (OrderedMap)]
  spec fun spec_aborts_new_from {K} {V}(
    keys : Vector<K>, values : Vector<V>
  ) : Bool :=
    keys.length != values.length
      || (∃ (i in 0 .. keys.length; j in 0 .. keys.length),
        i != j && keys[i] == keys[j])

  @[map_spec_aborts_append_disjoint (OrderedMap)]
  spec fun spec_aborts_append_disjoint {K} {V}(
    m : OrderedMap<K, V>, other : OrderedMap<K, V>
  ) : Bool :=
    ∃ (k : K), spec_contains_key(m, k) && spec_contains_key(other, k)

  @[map_spec_aborts_trim (OrderedMap)]
  spec fun spec_aborts_trim {K} {V}(m : OrderedMap<K, V>, at : Int) : Bool :=
    at > spec_len(m)

  @[map_spec_aborts_upsert_all (OrderedMap)]
  spec fun spec_aborts_upsert_all {K} {V}(
    _m : OrderedMap<K, V>, keys : Vector<K>, values : Vector<V>
  ) : Bool :=
    keys.length != values.length

  -- Over-approximates the template's cmp-order-violation abort path (modeled
  -- nondeterministically): when `old_key != new_key`, returns true even though
  -- the actual call may succeed if the order precondition holds.
  @[map_spec_aborts_replace_key_inplace (OrderedMap)]
  spec fun spec_aborts_replace_key_inplace {K} {V}(
    m : OrderedMap<K, V>, old_key : K, new_key : K
  ) : Bool :=
    !spec_contains_key(m, old_key) || old_key != new_key

  -- Where an add lands: the end iterator appends, any other iterator inserts
  -- at its own position.
  spec fun spec_iter_add_index {K} {V}(
    self : IteratorPtr, map : OrderedMap<K, V>
  ) : Int :=
    if self is End then spec_len(map) else self.index

  -- The insert splices a position in. The two ordering checks in the body say
  -- exactly that the position is the sorted one for this key — larger than the
  -- key before it and smaller than the key at it — so a duplicate key always
  -- aborts.
  -- A value write at the iterator's position: the key set, and so every
  -- position, is untouched.
  -- The mirror of the add: removing at a position closes it up, so keys after
  -- it move down one and keys before it stay put.
  -- Modelled by the intrinsic map: the borrow resolves this iterator's
  -- position to a key through the enumeration and hands back a mutation at
  -- that key, so a caller's write-back updates the abstract map instead of
  -- traversing the entry vector.
  -- Mirrors the borrow's abort behavior: no value at the end iterator, and
  -- none at a position past the last one.
  @[map_spec_aborts_iter_borrow_mut (OrderedMap)]
  spec fun spec_aborts_iter_borrow_mut {K} {V}(
    self : IteratorPtr, map : OrderedMap<K, V>
  ) : Bool :=
    self is End || self.index >= spec_len(map)

  -- The first position whose key is not less than the input — the end iterator
  -- when every key is smaller. Since positions are the enumeration, that index
  -- is where the input would sit, which is what lets a scan start here knowing
  -- it skipped only smaller keys.
  -- A key that is present is never skipped past. Implied by the End
  -- characterization above, but only after instantiating it at that key's
  -- own position, which a caller has no term to do with. Its position
  -- being the key's rank then follows from the comparison facts, so this
  -- is all that needs stating.
  -- The index is the position in ascending key order.
  -- One step forward, becoming End once past the last position.
  -- From End, one step back is the last position; otherwise one less.
  -- Cannot call return in an inline function so we need to resort to break here.
  -- When we are close to the end, it is cheaper to not create
  -- a temporary vector, and swap directly
  -- i out of bounds; abort
  -- When we are close to the end, it is cheaper to not create
  -- a temporary vector, and swap directly
  -- This doesn't cost a O(2N) run time as index_of scans from left to right and stops when the element is found,
  -- while remove would continue from the identified index to the end of the vector.
  -- We need to reverse the vector to consume it efficiently
  -- We need to reverse the vectors to consume it efficiently
  -- We can't use the constant EVECTORS_LENGTH_MISMATCH here as all calling code would then need to define it
  -- due to how inline functions work.
  -- We can't use the constant EVECTORS_LENGTH_MISMATCH here as all calling code would then need to define it
  -- due to how inline functions work.
  -- `_mut` HOFs use pointwise invariants because their inputs evolve.
  -- We can't use the constant EVECTORS_LENGTH_MISMATCH here as all calling code would then need to define it
  -- due to how inline functions work.
  -- We can't use the constant EVECTORS_LENGTH_MISMATCH here as all calling code would then need to define it
  -- due to how inline functions work.
  -- We can't use the constant EVECTORS_LENGTH_MISMATCH here as all calling code would then need to define it
  -- due to how inline functions work.
  -- =================================================================
  -- Module Specification
  -- Switch to module documentation context
  spec fun «spec_map_ref$lambda$0» {T0} {T1}(
    v : Vector<Entry<T0, T1> >, end : Int
  ) : Vector<T0> :=
    if end == 0 then vec::<T0>()
    else concat(«spec_map_ref$lambda$0»(v, end - 1), vec(v[end - 1].key))

  spec fun «spec_map_ref$lambda$2» {T0} {T1}(
    v : Vector<Entry<T0, T1> >, end : Int
  ) : Vector<T1> :=
    if end == 0 then vec::<T1>()
    else concat(«spec_map_ref$lambda$2»(v, end - 1), vec(v[end - 1].value))

  spec fun «spec_map_ref_aborts$lambda$1» {T0} {T1}(
    v : Vector<Entry<T0, T1> >, end : Int
  ) : Bool :=
    end > 0 && («spec_map_ref_aborts$lambda$1»(v, end - 1) || false)

  spec fun «spec_map_ref_aborts$lambda$3» {T0} {T1}(
    v : Vector<Entry<T0, T1> >, end : Int
  ) : Bool :=
    end > 0 && («spec_map_ref_aborts$lambda$3»(v, end - 1) || false)
