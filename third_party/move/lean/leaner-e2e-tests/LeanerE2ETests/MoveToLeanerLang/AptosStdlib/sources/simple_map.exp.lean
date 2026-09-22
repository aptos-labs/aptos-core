-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
This module provides a solution for unsorted maps, that is it has the properties that
1) Keys point to Values
2) Each Key must be unique
3) A Key can be found within O(N) time
4) The keys are unsorted.
5) Adds and removals take O(N) time

DEPRECATED: since it's implementation is inneficient, it
has been deprecated in favor of `ordered_map.move`.
-/
leaner module 0x1::simple_map where
  use 0x1::std::error::invalid_argument
  use 0x1::std::option::Option
  use 0x1::std::option::extract
  use 0x1::std::option::is_none
  use 0x1::std::option::is_some
  use 0x1::std::option::none
  use 0x1::std::option::some
  use 0x1::std::option::spec_borrow
  use 0x1::std::vector
  use 0x1::std::vector::spec_contains

  /--
  Map key already exists
  -/
  const EKEY_ALREADY_EXISTS : u64 := 1

  /--
  Map key is not found
  -/
  const EKEY_NOT_FOUND : u64 := 2

  /--
  DEPRECATED: since it's implementation is inneficient, it
  has been deprecated in favor of `ordered_map.move`.
  -/
  @[intrinsic_map]
  struct SimpleMap {«Key»} {Value} has Copy, Drop, Store where
    data : Vector<Element<«Key», Value> >

  spec SimpleMap where
    pragma intrinsic = map

  struct Element {«Key»} {Value} has Copy, Drop, Store where
    key : «Key»
    value : Value

  @[map_len (SimpleMap)]
  public fun length {«Key» has Store} {Value has Store}(
    self : &SimpleMap<«Key», Value>
  ) -> u64 := self.data.length

  spec length where
    pragma intrinsic

  /--
  Create an empty SimpleMap.
  -/
  public fun new {«Key» has Store} {Value has Store}() -> SimpleMap<«Key», Value> :=
    new SimpleMap<«Key», Value> { data := vector<Element<«Key», Value> >[] }

  spec new where
    pragma opaque
    pragma intrinsic
    aborts_if [abstract] false
    ensures [abstract] spec_len(result) == 0
    ensures [abstract] ∀ (k : «Key»), !spec_contains_key(result, k)

  /--
  Create a SimpleMap from a vector of keys and values. The keys must be unique.
  -/
  public fun new_from {«Key» has Store} {Value has Store}(
    keys : Vector<«Key»>, values : Vector<Value>
  ) -> SimpleMap<«Key», Value> := do
    let mut map := new::<«Key», Value>()
    map.add_all(keys, values)
    return map

  spec new_from where
    pragma opaque
    pragma intrinsic
    aborts_if [abstract] false
    ensures [abstract] spec_len(result) == keys.length
    ensures [abstract] ∀ (k : «Key»),
        spec_contains_key(result, k) <==> spec_contains(keys, k)
    ensures [abstract] ∀ (i in 0 .. keys.length),
        spec_get(result, 0x1::std::vector::borrow(keys, i))
          == 0x1::std::vector::borrow(values, i)

  /--
  Create an empty SimpleMap.
  This function is deprecated, use `new` instead.
  -/
  @[map_new (SimpleMap)]
  public deprecated fun create {«Key» has Store} {Value has Store}() -> SimpleMap<«Key», Value> := new::<«Key», Value>()

  spec create where
    pragma intrinsic

  @[map_borrow (SimpleMap)]
  public fun borrow {«Key» has Store} {Value has Store}(
    self : &SimpleMap<«Key», Value>, key : &«Key»
  ) -> &Value := do
    let mut maybe_idx := self.find(key)
    assert!(is_some(&maybe_idx), invalid_argument(EKEY_NOT_FOUND))
    let idx := extract(&mut maybe_idx)
    return &self.data[idx].value

  spec borrow where
    pragma intrinsic

  @[map_borrow_mut (SimpleMap)]
  public fun borrow_mut {«Key» has Store} {Value has Store}(
    self : &mut SimpleMap<«Key», Value>, key : &«Key»
  ) -> &mut Value := do
    let mut maybe_idx := self.find(key)
    assert!(is_some(&maybe_idx), invalid_argument(EKEY_NOT_FOUND))
    let idx := extract(&mut maybe_idx)
    return &mut self.data[idx].value

  spec borrow_mut where
    pragma intrinsic

  @[map_has_key (SimpleMap)]
  public fun contains_key {«Key» has Store} {Value has Store}(
    self : &SimpleMap<«Key», Value>, key : &«Key»
  ) -> Bool := do
    let maybe_idx := self.find(key)
    return is_some(&maybe_idx)

  spec contains_key where
    pragma intrinsic

  @[map_destroy_empty (SimpleMap)]
  public fun destroy_empty {«Key» has Store} {Value has Store}(
    self : SimpleMap<«Key», Value>
  ) -> Unit := do
    let SimpleMap<«Key», Value> { data := data } := self
    data.destroy_empty()

  spec destroy_empty where
    pragma intrinsic

  /--
  Add a key/value pair to the map. The key must not already exist.
  -/
  @[map_add_no_override (SimpleMap)]
  public fun add {«Key» has Store} {Value has Store}(
    self : &mut SimpleMap<«Key», Value>, key : «Key», value : Value
  ) -> Unit := do
    let maybe_idx := self.find(&key)
    assert!(is_none(&maybe_idx), invalid_argument(EKEY_ALREADY_EXISTS))
    self.data := core.prim.pushVector(
      self.data, new Element<«Key», Value> { key, value }
    )

  spec add where
    pragma intrinsic

  /--
  Add multiple key/value pairs to the map. The keys must not already exist.
  -/
  public fun add_all {«Key» has Store} {Value has Store}(
    self : &mut SimpleMap<«Key», Value>, keys : Vector<«Key»>,
    values : Vector<Value>
  ) -> Unit := do
    let mut (self', v2) := (keys, values)
    self'.reverse()
    v2.reverse()
    let mut (self', v2) := (self', v2)
    spec assume folds_capture_anchor!(38)
    let len := self'.length
    assert!(len == v2.length, 131074)
    while len > 0 do
      let (e1, e2) := (self'.pop_back(), v2.pop_back())
      let (key, value) := (e1, e2)
      self.add(key, value)
      len := len - 1
    where
      invariant with_state_anchor!(38, old(self')).length >= len
      invariant len == self'.length
      invariant len == v2.length
      invariant with_state_anchor!(38, old(self')).length
        == with_state_anchor!(38, old(v2)).length
      invariant ∀ (j in 0 .. len),
        self'[j] == with_state_anchor!(38, old(self'))[j]
      invariant ∀ (j in 0 .. len), v2[j] == with_state_anchor!(38, old(v2))[j]
      invariant ∀ (j in len .. with_state_anchor!(38, old(self')).length), true
      invariant true
    self'.destroy_empty()
    v2.destroy_empty()

  spec add_all where
    pragma intrinsic

  /--
  Insert key/value pair or update an existing key to a new value
  -/
  public fun upsert {«Key» has Store} {Value has Store}(
    self : &mut SimpleMap<«Key», Value>, key : «Key», value : Value
  ) -> (Option<«Key»>, Option<Value>) := do
    let data := &mut self.data
    let len := data.length
    for i in 0..len do
      let element := &data[i]
      if &element.key == &key then
        *data := core.prim.pushVector(
          *data, new Element<«Key», Value> { key, value }
        )
        data.swap(i, len)
        let Element<«Key», Value> { key := key, value := value } :=
          data.pop_back()
        return (some(key), some(value));
    self.data := core.prim.pushVector(
      self.data, new Element<«Key», Value> { key, value }
    )
    return (none::<«Key»>(), none::<Value>())

  spec upsert where
    pragma opaque
    pragma intrinsic
    aborts_if [abstract] false
    ensures [abstract] self == spec_set(old(self), key, value)
    ensures [abstract] !spec_contains_key(old(self), key) ==> is_none(result)
    ensures [abstract] !spec_contains_key(old(self), key)
        ==> is_none(spec.result[1])
    ensures [abstract] spec_contains_key(self, key)
    ensures [abstract] spec_get(self, key) == value
    ensures [abstract] spec_contains_key(old(self), key)
        ==> is_some(result) && spec_borrow(result) == key
    ensures [abstract] spec_contains_key(old(self), key)
        ==> is_some(spec.result[1])
          && spec_borrow(spec.result[1]) == spec_get(old(self), key)

  /--
  Return all keys in the map. This requires keys to be copyable.
  -/
  public fun keys {«Key» has Copy} {Value}(
    self : &SimpleMap<«Key», Value>
  ) -> Vector<«Key»> := do
    let self := &self.data
    spec assume spec.inlineCallSummary(
      «spec_map_ref$lambda$0»(self, self.length),
      «spec_map_ref_aborts$lambda$1»(self, self.length)
    )
    let _inline_summary_result_46 :=
      do
        let mut result := vector<«Key»>[]
        let i := 0
        let len := self.length
        while i < len do
          result := core.prim.pushVector(
            result,
            do
              let e := &self[i]
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
    spec assert _inline_summary_result_46
      == «spec_map_ref$lambda$0»(self, self.length)
    return _inline_summary_result_46

  spec keys where
    pragma verify = false

  /--
  Return all values in the map. This requires values to be copyable.
  -/
  public fun values {«Key»} {Value has Copy}(
    self : &SimpleMap<«Key», Value>
  ) -> Vector<Value> := do
    let self := &self.data
    spec assume spec.inlineCallSummary(
      «spec_map_ref$lambda$2»(self, self.length),
      «spec_map_ref_aborts$lambda$3»(self, self.length)
    )
    let _inline_summary_result_51 :=
      do
        let mut result := vector<Value>[]
        let i := 0
        let len := self.length
        while i < len do
          result := core.prim.pushVector(
            result,
            do
              let e := &self[i]
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
    spec assert _inline_summary_result_51
      == «spec_map_ref$lambda$2»(self, self.length)
    return _inline_summary_result_51

  spec values where
    pragma verify = false

  /--
  Transform the map into two vectors with the keys and values respectively
  Primarily used to destroy a map
  -/
  public fun to_vec_pair {«Key» has Store} {Value has Store}(
    self : SimpleMap<«Key», Value>
  ) -> (Vector<«Key»>, Vector<Value>) := do
    let mut keys := vector<«Key»>[]
    let mut values := vector<Value>[]
    let SimpleMap<«Key», Value> { data := data } := self
    let mut self := data
    self.reverse()
    let mut self := self
    spec assume folds_capture_anchor!(53)
    spec assume folds_capture_anchor!(52)
    let len := self.length
    while len > 0 do
      let e := self.pop_back()
      let e := e
      let Element<«Key», Value> { key := key, value := value } := e
      keys := core.prim.pushVector(keys, key)
      values := core.prim.pushVector(values, value)
      len := len - 1
    where
      invariant with_state_anchor!(52, old(self)).length >= len
      invariant len == self.length
      invariant ∀ (j in 0 .. len),
        self[j] == with_state_anchor!(52, old(self))[j]
      invariant ∀ (j in len .. with_state_anchor!(52, old(self)).length), true
      invariant (keys, values)
        == «spec_fold$gen$1»(
          with_state_anchor!(
            53, old(keys)
          ), with_state_anchor!(53, old(values)),
          with_state_anchor!(52, old(self)).length - len,
          with_state_anchor!(52, old(self))
        )
    self.destroy_empty()
    return (keys, values)

  spec to_vec_pair where
    pragma opaque
    pragma intrinsic
    aborts_if [abstract] false
    ensures [abstract] ∀ (k : «Key»),
        spec_contains(result, k) <==> spec_contains_key(self, k)
    ensures [abstract] ∀ (i in 0 .. result.length),
        spec_get(self, 0x1::std::vector::borrow(result, i))
          == 0x1::std::vector::borrow(spec.result[1], i)

  /--
  Remove a key/value pair from the map. The key must exist.
  -/
  @[map_del_return_key (SimpleMap)]
  public fun remove {«Key» has Store} {Value has Store}(
    self : &mut SimpleMap<«Key», Value>, key : &«Key»
  ) -> («Key», Value) := do
    let mut maybe_idx := self.find(key)
    assert!(is_some(&maybe_idx), invalid_argument(EKEY_NOT_FOUND))
    let placement := extract(&mut maybe_idx)
    let Element<«Key», Value> { key := key, value := value } :=
      self.data.swap_remove(placement)
    return (key, value)

  spec remove where
    pragma intrinsic

  fun find {«Key» has Store} {Value has Store}(
    self : &SimpleMap<«Key», Value>, key : &«Key»
  ) -> Option<u64> := do
    let len := self.data.length
    for i in 0..len do
      let element := &self.data[i]
      if &element.key == key then return some(i);
    return none::<u64>()

  spec find where
    pragma verify = false

  -- test adding 3 elements using upsert
  -- change mapping 1->1 to 1->4
  -- Make most of the public API intrinsic. Those functions have custom specifications in the prover.
  -- Specification functions for tables
  @[map_spec_new (SimpleMap)]
  opaque spec fun spec_new {K} {V}() : SimpleMap<K, V>

  @[map_spec_len (SimpleMap)]
  opaque spec fun spec_len {K} {V}(t : SimpleMap<K, V>) : Int

  @[map_spec_has_key (SimpleMap)]
  opaque spec fun spec_contains_key {K} {V}(t : SimpleMap<K, V>, k : K) : Bool

  @[map_spec_set (SimpleMap)]
  opaque spec fun spec_set {K} {V}(
    t : SimpleMap<K, V>, k : K, v : V
  ) : SimpleMap<K, V>

  @[map_spec_del (SimpleMap)]
  opaque spec fun spec_remove {K} {V}(
    t : SimpleMap<K, V>, k : K
  ) : SimpleMap<K, V>

  @[map_spec_get (SimpleMap)]
  opaque spec fun spec_get {K} {V}(t : SimpleMap<K, V>, k : K) : V

  -- Abort-condition spec functions — mirror the abort guards in the Boogie template
  @[map_spec_aborts_destroy_empty (SimpleMap)]
  opaque spec fun spec_aborts_destroy_empty {K} {V}(m : SimpleMap<K, V>) : Bool

  @[map_spec_aborts_add (SimpleMap)]
  opaque spec fun spec_aborts_add {K} {V}(
    m : SimpleMap<K, V>, k : K, v : V
  ) : Bool

  @[map_spec_aborts_del (SimpleMap)]
  opaque spec fun spec_aborts_del {K} {V}(m : SimpleMap<K, V>, k : K) : Bool

  @[map_spec_aborts_borrow (SimpleMap)]
  opaque spec fun spec_aborts_borrow {K} {V}(m : SimpleMap<K, V>, k : K) : Bool

  -- Cannot call return in an inline function so we need to resort to break here.
  -- When we are close to the end, it is cheaper to not create
  -- a temporary vector, and swap directly
  -- i out of bounds; abort
  -- When we are close to the end, it is cheaper to not create
  -- a temporary vector, and swap directly
  -- This doesn't cost a O(2N) run time as index_of scans from left to right and stops when the element is found,
  -- while remove would continue from the identified index to the end of the vector.
  -- We need to reverse the vector to consume it efficiently
  spec fun «spec_fold$gen$1» {T0} {T1}(
    «keys$init» : Vector<T0>, «values$init» : Vector<T1>, _end : Int,
    _fold_anchor_ctx_0 : Vector<Element<T0, T1> >
  ) : (Vector<T0>, Vector<T1>) :=
    if _end == 0 then («keys$init», «values$init»)
    else
      let («keys$acc», «values$acc») :=
        «spec_fold$gen$1»(
          «keys$init», «values$init», _end
            - 1, _fold_anchor_ctx_0
        )
      return (concat(
          «keys$acc»,
          vec(
            _fold_anchor_ctx_0[_fold_anchor_ctx_0.length - 1
              - (_end - 1)].key
          )
        ),
        concat(
          «values$acc»,
          vec(
            _fold_anchor_ctx_0[_fold_anchor_ctx_0.length - 1
              - (_end - 1)].value
          )
        ))

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
    v : Vector<Element<T0, T1> >, end : Int
  ) : Vector<T0> :=
    if end == 0 then vec::<T0>()
    else concat(«spec_map_ref$lambda$0»(v, end - 1), vec(v[end - 1].key))

  spec fun «spec_map_ref$lambda$2» {T0} {T1}(
    v : Vector<Element<T0, T1> >, end : Int
  ) : Vector<T1> :=
    if end == 0 then vec::<T1>()
    else concat(«spec_map_ref$lambda$2»(v, end - 1), vec(v[end - 1].value))

  spec fun «spec_map_ref_aborts$lambda$1» {T0} {T1}(
    v : Vector<Element<T0, T1> >, end : Int
  ) : Bool :=
    end > 0 && («spec_map_ref_aborts$lambda$1»(v, end - 1) || false)

  spec fun «spec_map_ref_aborts$lambda$3» {T0} {T1}(
    v : Vector<Element<T0, T1> >, end : Int
  ) : Bool :=
    end > 0 && («spec_map_ref_aborts$lambda$3»(v, end - 1) || false)
