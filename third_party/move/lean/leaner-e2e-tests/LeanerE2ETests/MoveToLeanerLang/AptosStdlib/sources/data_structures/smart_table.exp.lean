-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
A smart table implementation based on linear hashing. (https://en.wikipedia.org/wiki/Linear_hashing)
Compare to Table, it uses less storage slots but has higher chance of collision, a trade-off between space and time.
Compare to other dynamic hashing implementation, linear hashing splits one bucket a time instead of doubling buckets
when expanding to avoid unexpected gas cost.
SmartTable uses faster hash function SipHash instead of cryptographically secure hash functions like sha3-256 since
it tolerates collisions.

DEPRECATED: since it's implementation is inneficient, it
has been deprecated in favor of `big_ordered_map.move`.
-/
leaner module 0x1::smart_table where
  use 0x1::aptos_std::aptos_hash::sip_hash_from_value
  use 0x1::aptos_std::math64::max
  use 0x1::aptos_std::simple_map
  use 0x1::aptos_std::simple_map::SimpleMap
  use 0x1::aptos_std::table_with_length
  use 0x1::aptos_std::table_with_length::TableWithLength
  use 0x1::aptos_std::type_info::size_of_val
  use 0x1::std::error::invalid_argument
  use 0x1::std::error::permission_denied
  use 0x1::std::option::Option
  use 0x1::std::option::none
  use 0x1::std::option::some
  use 0x1::std::vector

  /--
  Key not found in the smart table
  -/
  const ENOT_FOUND : u64 := 1

  /--
  Smart table capacity must be larger than 0
  -/
  const EZERO_CAPACITY : u64 := 2

  /--
  Cannot destroy non-empty hashmap
  -/
  const ENOT_EMPTY : u64 := 3

  /--
  Key already exists
  -/
  const EALREADY_EXIST : u64 := 4

  /--
  Invalid load threshold percent to trigger split.
  -/
  const EINVALID_LOAD_THRESHOLD_PERCENT : u64 := 5

  /--
  Invalid target bucket size.
  -/
  const EINVALID_TARGET_BUCKET_SIZE : u64 := 6

  /--
  Invalid target bucket size.
  -/
  const EEXCEED_MAX_BUCKET_SIZE : u64 := 7

  /--
  Invalid bucket index.
  -/
  const EINVALID_BUCKET_INDEX : u64 := 8

  /--
  Invalid vector index within a bucket.
  -/
  const EINVALID_VECTOR_INDEX : u64 := 9

  /--
  SmartTable entry contains both the key and value.
  -/
  struct Entry {K} {V} has Copy, Drop, Store where
    hash : u64
    key : K
    value : V

  @[intrinsic_map]
  struct SmartTable {K} {V} has Store where
    buckets : TableWithLength<u64, Vector<Entry<K, V> > >
    num_buckets : u64
    level : u8
    size : u64
    split_load_threshold : u8
    target_bucket_size : u64

  spec SmartTable where
    pragma intrinsic = map

  -- number of bits to represent num_buckets
  -- total number of items
  -- Split will be triggered when target load threshold in percentage is reached when adding a new entry.
  -- The target size of each bucket, which is NOT enforced so oversized buckets can exist.
  /--
  Create an empty SmartTable with default configurations.
  -/
  @[map_new (SmartTable)]
  public fun new {K has Copy, Drop, Store} {V has Store}() -> SmartTable<K, V> :=
    new_with_config::<K, V>(0, 0u8, 0)

  /--
  Create an empty SmartTable with customized configurations.
  `num_initial_buckets`: The number of buckets on initialization. 0 means using default value.
  `split_load_threshold`: The percent number which once reached, split will be triggered. 0 means using default
  value.
  `target_bucket_size`: The target number of entries per bucket, though not guaranteed. 0 means not set and will
  dynamically assgined by the contract code.
  -/
  public fun new_with_config {K has Copy, Drop, Store} {V has Store}(
    num_initial_buckets : u64, split_load_threshold : u8,
    target_bucket_size : u64
  ) -> SmartTable<K, V> := do
    assert!(
      split_load_threshold <= 100u8,
      invalid_argument(EINVALID_LOAD_THRESHOLD_PERCENT)
    )
    let mut buckets := table_with_length::new::<u64, Vector<Entry<K, V> > >()
    table_with_length::add(&mut buckets, 0, vector<Entry<K, V> >[])
    let mut table :=
      new SmartTable<K, V> {
        buckets, num_buckets := 1, level := 0u8, size := 0,
        split_load_threshold := if split_load_threshold == 0u8 then 75u8
        else split_load_threshold,
        target_bucket_size
      }
    if num_initial_buckets == 0 then num_initial_buckets := 2
    while num_initial_buckets > 1 do
      num_initial_buckets := num_initial_buckets - 1
      table.split_one_bucket()
    return table

  spec new_with_config where
    pragma verify = false

  -- The default split load threshold is 75%.
  -- The default number of initial buckets is 2.
  /--
  Destroy empty table.
  Aborts if it's not empty.
  -/
  @[map_destroy_empty (SmartTable)]
  public fun destroy_empty {K} {V}(self : SmartTable<K, V>) -> Unit := do
    assert!(self.size == 0, invalid_argument(ENOT_EMPTY))
    for i in 0..self.num_buckets do
      table_with_length::remove(&mut self.buckets, i).destroy_empty()
    let SmartTable<K, V> { buckets := buckets,
    num_buckets := _,
    level := _,
    size := _,
    split_load_threshold := _,
    target_bucket_size := _ } :=
      self
    table_with_length::destroy_empty(buckets)

  /--
  Destroy a table completely when V has `drop`.
  -/
  public fun destroy {K has Drop} {V has Drop}(
    mut self : SmartTable<K, V>
  ) -> Unit := do
    self.clear()
    self.destroy_empty()

  spec destroy where
    pragma opaque
    pragma verify = false

  /--
  Clear a table completely when T has `drop`.
  -/
  public fun clear {K has Drop} {V has Drop}(
    self : &mut SmartTable<K, V>
  ) -> Unit := do
    *table_with_length::borrow_mut(
      &mut self.buckets, 0
    ) := vector<Entry<K, V> >[]
    for i in 1..self.num_buckets do
      table_with_length::remove(&mut self.buckets, i)
    self.num_buckets := 1
    self.level := 0u8
    self.size := 0

  spec clear where
    pragma opaque
    pragma verify = false

  /--
  Add (key, value) pair in the hash map, it may grow one bucket if current load factor exceeds the threshold.
  Note it may not split the actual overflowed bucket. Instead, it was determined by `num_buckets` and `level`.
  For standard linear hash algorithm, it is stored as a variable but `num_buckets` here could be leveraged.
  Abort if `key` already exists.
  Note: This method may occasionally cost much more gas when triggering bucket split.
  -/
  @[map_add_no_override (SmartTable)]
  public fun add {K} {V}(
    self : &mut SmartTable<K, V>, key : K, value : V
  ) -> Unit := do
    let hash := sip_hash_from_value(&key)
    let index := bucket_index(self.level, self.num_buckets, hash)
    let bucket := table_with_length::borrow_mut(&mut self.buckets, index)
    assert!(bucket.length <= 10000, permission_denied(EEXCEED_MAX_BUCKET_SIZE))
    assert!(
      do
        let self := bucket
        let result := true
        let i := 0
        while self.length > i do
          result := do
            let «entry» := &self[i]
            let e := «entry»
            return &e.key != &key
          if !result then break
          i := i + 1
        where
          invariant self.length >= i
          invariant result
          invariant ∀ (j in 0 .. i), self[j].key != key
        spec assert result <==> (∀ (j in 0 .. self.length), self[j].key != key)
        return result, invalid_argument(EALREADY_EXIST))
    let e := new Entry<K, V> { hash, key, value }
    if self.target_bucket_size == 0 then
      let estimated_entry_size := max(size_of_val(&e), 1)
      self.target_bucket_size := max(1024 / estimated_entry_size, 1)
    *bucket := core.prim.pushVector(*bucket, e)
    let _t1 := &mut self.size
    *_t1 := *_t1 + 1
    if self.load_factor() >= self.split_load_threshold as u64 then
      self.split_one_bucket()

  -- We set a per-bucket limit here with a upper bound (10000) that nobody should normally reach.
  -- free_write_quota
  /--
  Add multiple key/value pairs to the smart table. The keys must not already exist.
  -/
  public fun add_all {K} {V}(
    self : &mut SmartTable<K, V>, keys : Vector<K>, values : Vector<V>
  ) -> Unit := do
    let mut (self', v2) := (keys, values)
    self'.reverse()
    v2.reverse()
    let mut (self', v2) := (self', v2)
    spec assume folds_capture_anchor!(60)
    let len := self'.length
    assert!(len == v2.length, 131074)
    while len > 0 do
      let (e1, e2) := (self'.pop_back(), v2.pop_back())
      let (key, value) := (e1, e2)
      self.add(key, value)
      len := len - 1
    where
      invariant with_state_anchor!(60, old(self')).length >= len
      invariant len == self'.length
      invariant len == v2.length
      invariant with_state_anchor!(60, old(self')).length
        == with_state_anchor!(60, old(v2)).length
      invariant ∀ (j in 0 .. len),
        self'[j] == with_state_anchor!(60, old(self'))[j]
      invariant ∀ (j in 0 .. len), v2[j] == with_state_anchor!(60, old(v2))[j]
      invariant ∀ (j in len .. with_state_anchor!(60, old(self')).length), true
      invariant true
    self'.destroy_empty()
    v2.destroy_empty()

  spec add_all where
    pragma verify = false

  /--
  Convert a smart table to a simple_map, which is supposed to be called mostly by view functions to get an atomic
  view of the whole table.
  Disclaimer: This function may be costly as the smart table may be huge in size. Use it at your own discretion.
  -/
  public fun to_simple_map {K has Copy, Drop, Store} {V has Copy, Store}(
    self : &SmartTable<K, V>
  ) -> SimpleMap<K, V> := do
    let mut res := simple_map::new::<K, V>()
    for i in 0..self.num_buckets do
      let mut (keys, values) :=
        do
          let entries := table_with_length::borrow(&self.buckets, i)
          let mut keys := vector<K>[]
          let mut values := vector<V>[]
          let self := entries
          spec assume folds_capture_anchor!(75)
          let i := 0
          let len := self.length
          while i < len do
            let e := &self[i]
            let «entry» := e
            keys := core.prim.pushVector(keys, «entry».key)
            values := core.prim.pushVector(values, «entry».value)
            i := i + 1
          where
            invariant i <= len
            invariant ∀ (j in 0 .. i), true
            invariant (keys, values)
              == «spec_fold$gen$0»(
                self, with_state_anchor!(75, old(keys)),
                with_state_anchor!(75, old(values)), i
              )
            invariant ∀ (j in i .. len), true
            invariant ∀ (x : Entry<K, V>),
              ∀ (j in 0 .. i), x != self[j] ==> true
          return (keys, values)
      simple_map::add_all(&mut res, keys, values)
    return res

  spec to_simple_map where
    pragma verify = false

  /--
  Get all keys in a smart table.

  For a large enough smart table this function will fail due to execution gas limits, and
  `keys_paginated` should be used instead.
  -/
  public fun keys {K has Copy, Drop, Store} {V has Copy, Store}(
    self : &SmartTable<K, V>
  ) -> Vector<K> := do
    let (keys, _, _) := self.keys_paginated(0, 0, self.length())
    return keys

  spec keys where
    pragma verify = false

  /--
  Get keys from a smart table, paginated.

  This function can be used to paginate all keys in a large smart table outside of runtime,
  e.g. through chained view function calls. The maximum `num_keys_to_get` before hitting gas
  limits depends on the data types in the smart table.

  When starting pagination, pass `starting_bucket_index` = `starting_vector_index` = 0.

  The function will then return a vector of keys, an optional bucket index, and an optional
  vector index. The unpacked return indices can then be used as inputs to another pagination
  call, which will return a vector of more keys. This process can be repeated until the
  returned bucket index and vector index value options are both none, which means that
  pagination is complete. For an example, see `test_keys()`.
  -/
  public fun keys_paginated {K has Copy, Drop, Store} {V has Copy, Store}(
    self : &SmartTable<K, V>, starting_bucket_index : u64,
    starting_vector_index : u64, num_keys_to_get : u64
  ) -> (Vector<K>, Option<u64>, Option<u64>) := do
    let num_buckets := self.num_buckets
    let buckets_ref := &self.buckets
    assert!(starting_bucket_index < num_buckets, EINVALID_BUCKET_INDEX)
    let bucket_ref :=
      table_with_length::borrow(buckets_ref, starting_bucket_index)
    let bucket_length := bucket_ref.length
    assert!(
      starting_vector_index < bucket_length || starting_vector_index == 0,
      EINVALID_VECTOR_INDEX
    )
    let mut keys := vector<K>[]
    if num_keys_to_get == 0 then
      return (keys, some(starting_bucket_index), some(starting_vector_index));
    for bucket_index in starting_bucket_index..num_buckets do
      bucket_ref := table_with_length::borrow(buckets_ref, bucket_index)
      bucket_length := bucket_ref.length
      for vector_index in starting_vector_index..bucket_length do
        keys := core.prim.pushVector(keys, bucket_ref[vector_index].key)
        num_keys_to_get := num_keys_to_get - 1
        if num_keys_to_get == 0 then
          vector_index := vector_index + 1
          return (if vector_index == bucket_length then
            bucket_index := bucket_index + 1
            return (if bucket_index
              < num_buckets then (keys, some(bucket_index), some(0))
            else (keys, none::<u64>(), none::<u64>()))
          else (keys, some(bucket_index), some(vector_index)));
      starting_vector_index := 0
    return (keys, none::<u64>(), none::<u64>())

  spec keys_paginated where
    pragma verify = false

  -- In the general case, starting vector index should never be equal to bucket length
  -- because then iteration will attempt to borrow a vector element that is out of bounds.
  -- However starting vector index can be equal to bucket length in the special case of
  -- starting iteration at the beginning of an empty bucket since buckets are never
  -- destroyed, only emptied.
  -- Start parsing the next bucket at vector index 0.
  /--
  Decide which is the next bucket to split and split it into two with the elements inside the bucket.
  -/
  fun split_one_bucket {K} {V}(self : &mut SmartTable<K, V>) -> Unit := do
    let new_bucket_index := self.num_buckets
    let to_split := new_bucket_index ^ 1 << self.level
    self.num_buckets := new_bucket_index + 1
    if to_split + 1 == 1 << self.level then
      let _t1 := &mut self.level
      *_t1 := *_t1 + 1u8
    let old_bucket := table_with_length::borrow_mut(&mut self.buckets, to_split)
    let p :=
      do
        let self' := old_bucket
        let i := 0
        let len := self'.length
        while i < len do
          if !(do
            let e := &self'[i]
            let «entry» := e
            return bucket_index(self.level, self.num_buckets, «entry».hash)
              != new_bucket_index) then
            break
          i := i + 1
        let p := i
        i := i + 1
        while i < len do
          if (do
            let e := &self'[i]
            let «entry» := e
            return bucket_index(self.level, self.num_buckets, «entry».hash)
              != new_bucket_index) then
            self'.swap(p, i)
            p := p + 1
          i := i + 1
        return p
    let new_bucket := old_bucket.trim_reverse(p)
    table_with_length::add(&mut self.buckets, new_bucket_index, new_bucket)

  spec split_one_bucket where
    pragma verify = false

  -- the next bucket to split is num_bucket without the most significant bit.
  -- if the whole level is splitted once, bump the level.
  -- partition the bucket, [0..p) stays in old bucket, [p..len) goes to new bucket
  -- Explicit type to satisfy compiler
  /--
  Return the expected bucket index to find the hash.
  Basically, it use different base `1 << level` vs `1 << (level + 1)` in modulo operation based on the target
  bucket index compared to the index of the next bucket to split.
  -/
  fun bucket_index(level : u8, num_buckets : u64, hash : u64) -> u64 := do
    let index := hash % (1 << level + 1u8)
    return if index < num_buckets then index else index % (1 << level)

  spec bucket_index where
    pragma verify = false

  -- in existing bucket
  -- in unsplitted bucket
  /--
  Acquire an immutable reference to the value which `key` maps to.
  Aborts if there is no entry for `key`.
  -/
  @[map_borrow (SmartTable)]
  public fun borrow {K has Drop} {V}(
    self : &SmartTable<K, V>, key : K
  ) -> &V := do
    let index :=
      bucket_index(self.level, self.num_buckets, sip_hash_from_value(&key))
    let bucket := table_with_length::borrow(&self.buckets, index)
    let len := bucket.length
    for i in 0..len do
      let «entry» := &bucket[i]
      if &«entry».key == &key then return &«entry».value;
    abort(invalid_argument(ENOT_FOUND))

  /--
  Acquire an immutable reference to the value which `key` maps to.
  Returns specified default value if there is no entry for `key`.
  -/
  @[map_borrow_with_default (SmartTable)]
  public fun borrow_with_default {K has Copy, Drop} {V}(
    self : &SmartTable<K, V>, key : K, default : &V
  ) -> &V :=
    if !self.contains(core.prim.copyValue(key)) then default
    else self.borrow(core.prim.copyValue(key))

  /--
  Acquire a mutable reference to the value which `key` maps to.
  Aborts if there is no entry for `key`.
  -/
  @[map_borrow_mut (SmartTable)]
  public fun borrow_mut {K has Drop} {V}(
    self : &mut SmartTable<K, V>, key : K
  ) -> &mut V := do
    let index :=
      bucket_index(self.level, self.num_buckets, sip_hash_from_value(&key))
    let bucket := table_with_length::borrow_mut(&mut self.buckets, index)
    let len := bucket.length
    for i in 0..len do
      let «entry» := &mut bucket[i]
      if &«entry».key == &key then return &mut «entry».value;
    abort(invalid_argument(ENOT_FOUND))

  /--
  Acquire a mutable reference to the value which `key` maps to.
  Insert the pair (`key`, `default`) first if there is no entry for `key`.
  -/
  @[map_borrow_mut_with_default (SmartTable)]
  public fun borrow_mut_with_default {K has Copy, Drop} {V has Drop}(
    self : &mut SmartTable<K, V>, key : K, default : V
  ) -> &mut V := do
    if !self.contains(core.prim.copyValue(key)) then
      self.add(core.prim.copyValue(key), default)
    return self.borrow_mut(key)

  /--
  Returns true iff `table` contains an entry for `key`.
  -/
  @[map_has_key (SmartTable)]
  public fun contains {K has Drop} {V}(
    self : &SmartTable<K, V>, key : K
  ) -> Bool := do
    let hash := sip_hash_from_value(&key)
    let index := bucket_index(self.level, self.num_buckets, hash)
    let bucket := table_with_length::borrow(&self.buckets, index)
    let self := bucket
    let result := false
    let i := 0
    while self.length > i do
      result := do
        let e := &self[i]
        return e.hash == hash && &e.key == &key
      if result then break
      i := i + 1
    where
      invariant self.length >= i
      invariant !result
      invariant ∀ (j in 0 .. i),
        !(if self[j].hash
          == hash then self[j].hash == hash && self[j].key == key
        else false)
    spec assert result
      <==> (∃ (j in 0 .. self.length),
        if self[j].hash == hash then self[j].hash == hash && self[j].key == key
        else false)
    return result

  /--
  Remove from `table` and return the value which `key` maps to.
  Aborts if there is no entry for `key`.
  -/
  @[map_del_must_exist (SmartTable)]
  public fun remove {K has Copy, Drop} {V}(
    self : &mut SmartTable<K, V>, key : K
  ) -> V := do
    let index :=
      bucket_index(self.level, self.num_buckets, sip_hash_from_value(&key))
    let bucket := table_with_length::borrow_mut(&mut self.buckets, index)
    let len := bucket.length
    for i in 0..len do
      let «entry» := &bucket[i]
      if &«entry».key == &key then
        let Entry<K, V> { hash := _, key := _, value := value } :=
          bucket.swap_remove(i)
        let _t1 := &mut self.size
        *_t1 := *_t1 - 1
        return value;
    abort(invalid_argument(ENOT_FOUND))

  /--
  Insert the pair (`key`, `value`) if there is no entry for `key`.
  update the value of the entry for `key` to `value` otherwise
  -/
  @[map_add_override_if_exists (SmartTable)]
  public fun upsert {K has Copy, Drop} {V has Drop}(
    self : &mut SmartTable<K, V>, key : K, value : V
  ) -> Unit :=
    if !self.contains(core.prim.copyValue(key)) then
      self.add(core.prim.copyValue(key), value)
    else
      let ref := self.borrow_mut(key)
      *ref := value

  /--
  Returns the length of the table, i.e. the number of entries.
  -/
  @[map_len (SmartTable)]
  public fun length {K} {V}(self : &SmartTable<K, V>) -> u64 := self.size

  /--
  Return the load factor of the hashtable.
  -/
  public fun load_factor {K} {V}(self : &SmartTable<K, V>) -> u64 :=
    self.size * 100 / self.num_buckets / self.target_bucket_size

  spec load_factor where
    pragma verify = false

  /--
  Update `split_load_threshold`.
  -/
  public fun update_split_load_threshold {K} {V}(
    self : &mut SmartTable<K, V>, split_load_threshold : u8
  ) -> Unit := do
    assert!(
      split_load_threshold <= 100u8 && split_load_threshold > 0u8,
      invalid_argument(EINVALID_LOAD_THRESHOLD_PERCENT)
    )
    self.split_load_threshold := split_load_threshold

  spec update_split_load_threshold where
    pragma verify = false

  /--
  Update `target_bucket_size`.
  -/
  public fun update_target_bucket_size {K} {V}(
    self : &mut SmartTable<K, V>, target_bucket_size : u64
  ) -> Unit := do
    assert!(
      target_bucket_size > 0, invalid_argument(
        EINVALID_TARGET_BUCKET_SIZE
      )
    )
    self.target_bucket_size := target_bucket_size

  spec update_target_bucket_size where
    pragma verify = false

  -- Helper functions to circumvent the scope issue of inline functions.
  public fun borrow_kv {K} {V}(self : &Entry<K, V>) -> (&K, &V) :=
    (&self.key, &self.value)

  spec borrow_kv where
    aborts_if false
    ensures result == self.key
    ensures spec.result[1] == self.value

  public fun borrow_kv_mut {K} {V}(
    self : &mut Entry<K, V>
  ) -> (&mut K, &mut V) :=
    (&mut self.key, &mut self.value)

  spec borrow_kv_mut where
    aborts_if false
    ensures result == old(self.key)
    ensures spec.result[1] == old(self.value)

  public fun num_buckets {K} {V}(self : &SmartTable<K, V>) -> u64 :=
    self.num_buckets

  spec num_buckets where
    pragma verify = false

  public fun borrow_buckets {K} {V}(
    self : &SmartTable<K, V>
  ) -> &TableWithLength<u64, Vector<Entry<K, V> > > := &self.buckets

  spec borrow_buckets where
    pragma verify = false

  public fun borrow_buckets_mut {K} {V}(
    self : &mut SmartTable<K, V>
  ) -> &mut TableWithLength<u64, Vector<Entry<K, V> > > := &mut self.buckets

  spec borrow_buckets_mut where
    pragma verify = false

  -- To eliminate erroneous compiler "unused" warning
  -- Both indices 0.
  -- Specification functions for tables
  @[map_spec_len (SmartTable)]
  opaque spec fun spec_len {K} {V}(t : SmartTable<K, V>) : Int

  @[map_spec_has_key (SmartTable)]
  opaque spec fun spec_contains {K} {V}(t : SmartTable<K, V>, k : K) : Bool

  @[map_spec_set (SmartTable)]
  opaque spec fun spec_set {K} {V}(
    t : SmartTable<K, V>, k : K, v : V
  ) : SmartTable<K, V>

  @[map_spec_del (SmartTable)]
  opaque spec fun spec_remove {K} {V}(
    t : SmartTable<K, V>, k : K
  ) : SmartTable<K, V>

  @[map_spec_get (SmartTable)]
  opaque spec fun spec_get {K} {V}(t : SmartTable<K, V>, k : K) : V

  -- Cannot call return in an inline function so we need to resort to break here.
  -- When we are close to the end, it is cheaper to not create
  -- a temporary vector, and swap directly
  -- i out of bounds; abort
  -- When we are close to the end, it is cheaper to not create
  -- a temporary vector, and swap directly
  -- This doesn't cost a O(2N) run time as index_of scans from left to right and stops when the element is found,
  -- while remove would continue from the identified index to the end of the vector.
  -- We need to reverse the vector to consume it efficiently
  spec fun «spec_fold$gen$0» {T0} {T1}(
    _v : Vector<Entry<T0, T1> >, «keys$init» : Vector<T0>,
    «values$init» : Vector<T1>, _end : Int
  ) : (Vector<T0>, Vector<T1>) :=
    if _end == 0 then («keys$init», «values$init»)
    else
      let («keys$acc», «values$acc») :=
        «spec_fold$gen$0»(_v, «keys$init», «values$init», _end - 1)
      return (concat(«keys$acc», vec(_v[_end - 1].key)),
        concat(«values$acc», vec(_v[_end - 1].value)))

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
