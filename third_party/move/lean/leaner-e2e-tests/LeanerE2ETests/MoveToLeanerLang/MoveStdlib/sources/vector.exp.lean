-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
A variable-sized container that can hold any type. Indexing is 0-based, and
vectors are growable. This module has many native functions.
Verification of modules that use this one uses model functions that are implemented
directly in Boogie. The specification language has built-in functions operations such
as `singleton_vector`. There are some helper functions defined here for specifications in other
modules as well.

>Note: We did not verify most of the
Move functions here because many have loops, requiring loop invariants to prove, and
the return on investment didn't seem worth it for these simple functions.
-/
leaner module 0x1::vector where
  use 0x1::std::mem

  /--
  The index into the vector is out of bounds
  -/
  const EINDEX_OUT_OF_BOUNDS : u64 := 131072

  /--
  The index into the vector is out of bounds
  -/
  const EINVALID_RANGE : u64 := 131073

  /--
  The length of the vectors are not equal.
  -/
  const EVECTORS_LENGTH_MISMATCH : u64 := 131074

  /--
  The step provided in `range` is invalid, must be greater than zero.
  -/
  const EINVALID_STEP : u64 := 131075

  /--
  The range in `slice` is invalid.
  -/
  const EINVALID_SLICE_RANGE : u64 := 131076

  /--
  Whether to utilize native vector::move_range
  Vector module cannot call features module, due to cyclic dependency,
  so this is a constant.
  -/
  const USE_MOVE_RANGE : Bool := true

  /--
  Create an empty vector.
  -/
  public native fun empty {Element}() -> Vector<Element>

  /--
  Return the length of the vector.
  -/
  public native fun length {Element}(self : &Vector<Element>) -> u64

  /--
  Acquire an immutable reference to the `i`th element of the vector `self`.
  Aborts if `i` is out of bounds.
  -/
  public native fun borrow {Element}(
    self : &Vector<Element>, i : u64
  ) -> &Element

  /--
  Add element `e` to the end of the vector `self`.
  -/
  public native fun push_back {Element}(
    self : &mut Vector<Element>, e : Element
  ) -> Unit

  /--
  Return a mutable reference to the `i`th element in the vector `self`.
  Aborts if `i` is out of bounds.
  -/
  public native fun borrow_mut {Element}(
    self : &mut Vector<Element>, i : u64
  ) -> &mut Element

  /--
  Pop an element from the end of vector `self`.
  Aborts if `self` is empty.
  -/
  public native fun pop_back {Element}(self : &mut Vector<Element>) -> Element

  /--
  Destroy the vector `self`.
  Aborts if `self` is not empty.
  -/
  public native fun destroy_empty {Element}(self : Vector<Element>) -> Unit

  /--
  Swaps the elements at the `i`th and `j`th indices in the vector `self`.
  Aborts if `i` or `j` is out of bounds.
  -/
  public native fun swap {Element}(
    self : &mut Vector<Element>, i : u64, j : u64
  ) -> Unit

  /--
  Moves range of elements `[removal_position, removal_position + length)` from vector `from`,
  to vector `to`, inserting them starting at the `insert_position`.
  In the `from` vector, elements after the selected range are moved left to fill the hole
  (i.e. range is removed, while the order of the rest of the elements is kept)
  In the `to` vector, elements after the `insert_position` are moved to the right to make
  space for new elements (i.e. range is inserted, while the order of the rest of the
   elements is kept).
  Move prevents from having two mutable references to the same value, so `from` and `to`
  vectors are always distinct.
  -/
  public native fun move_range {T}(
    from : &mut Vector<T>, removal_position : u64, length : u64,
    to : &mut Vector<T>, insert_position : u64
  ) -> Unit

  /--
  Return an vector of size one containing element `e`.
  -/
  public fun singleton {Element}(e : Element) -> Vector<Element> := do
    let mut v := vector<Element>[]
    v := core.prim.pushVector(v, e)
    return v

  spec singleton where
    aborts_if false
    ensures result == vec(e)

  /--
  Returns a reference to last element in the vector, or aborts if the vector is empty.
  -/
  public fun last {Element}(self : &Vector<Element>) -> &Element := do
    assert!(self.length > 0, EINDEX_OUT_OF_BOUNDS)
    return &self[self.length - 1]

  /--
  Returns a mutable reference to the last element in the vector, or aborts if the vector is empty.
  -/
  public fun last_mut {Element}(
    self : &mut Vector<Element>
  ) -> &mut Element := do
    assert!(self.length > 0, EINDEX_OUT_OF_BOUNDS)
    let len := self.length
    return &mut self[len - 1]

  /--
  Reverses the order of the elements in the vector `self` in place.
  -/
  public fun reverse {Element}(self : &mut Vector<Element>) -> Unit := do
    let len := self.length
    self.reverse_slice(0, len)

  spec reverse where
    pragma intrinsic

  /--
  Reverses the order of the elements [left, right) in the vector `self` in place.
  -/
  public fun reverse_slice {Element}(
    self : &mut Vector<Element>, left : u64, right : u64
  ) -> Unit := do
    assert!(left <= right, EINVALID_RANGE)
    if left == right then return ();
    right := right - 1
    while left < right do
      self.swap(left, right)
      left := left + 1
      right := right - 1

  spec reverse_slice where
    pragma intrinsic

  /--
  Pushes all of the elements of the `other` vector into the `self` vector.
  -/
  public fun append {Element}(
    self : &mut Vector<Element>, mut other : Vector<Element>
  ) -> Unit :=
    if USE_MOVE_RANGE then
      let self_length := self.length
      let other_length := other.length
      move_range(&mut other, 0, other_length, self, self_length)
      other.destroy_empty()
    else
      other.reverse()
      self.reverse_append(other)

  spec append where
    pragma intrinsic

  /--
  Pushes all of the elements of the `other` vector into the `self` vector.
  -/
  public fun reverse_append {Element}(
    self : &mut Vector<Element>, mut other : Vector<Element>
  ) -> Unit := do
    let len := other.length
    while len > 0 do
      *self := core.prim.pushVector(*self, other.pop_back())
      len := len - 1
    other.destroy_empty()

  spec reverse_append where
    pragma intrinsic

  /--
  Splits (trims) the collection into two at the given index.
  Returns a newly allocated vector containing the elements in the range [new_len, len).
  After the call, the original vector will be left containing the elements [0, new_len)
  with its previous capacity unchanged.
  In many languages this is also called `split_off`.
  -/
  public fun trim {Element}(
    self : &mut Vector<Element>, new_len : u64
  ) -> Vector<Element> := do
    let len := self.length
    assert!(new_len <= len, EINDEX_OUT_OF_BOUNDS)
    let mut other := vector<Element>[]
    if USE_MOVE_RANGE then
      move_range(self, new_len, len - new_len, &mut other, 0)
    else
      while len > new_len do
        other := core.prim.pushVector(other, self.pop_back())
        len := len - 1
      other.reverse()
    return other

  spec trim where
    pragma intrinsic

  /--
  Trim a vector to a smaller size, returning the evicted elements in reverse order
  -/
  public fun trim_reverse {Element}(
    self : &mut Vector<Element>, new_len : u64
  ) -> Vector<Element> := do
    let len := self.length
    assert!(new_len <= len, EINDEX_OUT_OF_BOUNDS)
    let mut result := vector<Element>[]
    while new_len < len do
      result := core.prim.pushVector(result, self.pop_back())
      len := len - 1
    return result

  spec trim_reverse where
    pragma intrinsic

  /--
  Return `true` if the vector `self` has no elements and `false` otherwise.
  -/
  public fun is_empty {Element}(self : &Vector<Element>) -> Bool :=
    self.length == 0

  spec is_empty where
    pragma intrinsic

  /--
  Return true if `e` is in the vector `self`.
  -/
  public fun contains {Element}(
    self : &Vector<Element>, e : &Element
  ) -> Bool := do
    let i := 0
    let len := self.length
    while i < len do
      if &self[i] == e then return true;
      i := i + 1
    return false

  spec contains where
    pragma intrinsic

  /--
  Return `(true, i)` if `e` is in the vector `self` at index `i`.
  Otherwise, returns `(false, 0)`.
  -/
  public fun index_of {Element}(
    self : &Vector<Element>, e : &Element
  ) -> (Bool, u64) := do
    let i := 0
    let len := self.length
    while i < len do
      if &self[i] == e then return (true, i);
      i := i + 1
    return (false, 0)

  spec index_of where
    pragma intrinsic

  /--
  Insert a new element at position 0 <= i <= length, using O(length - i) time.
  Aborts if out of bounds.
  -/
  public fun insert {Element}(
    self : &mut Vector<Element>, i : u64, e : Element
  ) -> Unit := do
    let len := self.length
    assert!(i <= len, EINDEX_OUT_OF_BOUNDS)
    if USE_MOVE_RANGE then
      if i + 2 >= len then
        *self := core.prim.pushVector(*self, e)
        while i < len do
          self.swap(i, len)
          i := i + 1
      else
        let mut other := singleton(e)
        move_range(&mut other, 0, 1, self, i)
        other.destroy_empty()
    else
      *self := core.prim.pushVector(*self, e)
      while i < len do
        self.swap(i, len)
        i := i + 1

  spec insert where
    pragma intrinsic

  -- When we are close to the end, it is cheaper to not create
  -- a temporary vector, and swap directly
  /--
  Remove the `i`th element of the vector `self`, shifting all subsequent elements.
  This is O(n) and preserves ordering of elements in the vector.
  Aborts if `i` is out of bounds.
  -/
  public fun remove {Element}(
    self : &mut Vector<Element>, i : u64
  ) -> Element := do
    let len := self.length
    if i >= len then abort(EINDEX_OUT_OF_BOUNDS)
    return if USE_MOVE_RANGE then
      if i + 3 >= len then
        len := len - 1
        while i < len do
          self.swap(
            i,
            do
              i := i + 1
              return i)
        return self.pop_back()
      else
        let mut other := vector<Element>[]
        move_range(self, i, 1, &mut other, 0)
        let result := other.pop_back()
        other.destroy_empty()
        return result
    else
      len := len - 1
      while i < len do
        self.swap(
          i,
          do
            i := i + 1
            return i)
      return self.pop_back()

  spec remove where
    pragma intrinsic

  -- i out of bounds; abort
  -- When we are close to the end, it is cheaper to not create
  -- a temporary vector, and swap directly
  /--
  Remove the first occurrence of a given value in the vector `self` and return it in a vector, shifting all
  subsequent elements.
  This is O(n) and preserves ordering of elements in the vector.
  This returns an empty vector if the value isn't present in the vector.
  Note that this cannot return an option as option uses vector and there'd be a circular dependency between option
  and vector.
  -/
  public fun remove_value {Element}(
    self : &mut Vector<Element>, val : &Element
  ) -> Vector<Element> := do
    let (found, index) := self.index_of(val)
    return if found then vector<Element>[self.remove(index)]
    else vector<Element>[]

  spec remove_value where
    pragma intrinsic

  -- This doesn't cost a O(2N) run time as index_of scans from left to right and stops when the element is found,
  -- while remove would continue from the identified index to the end of the vector.
  /--
  Swap the `i`th element of the vector `self` with the last element and then pop the vector.
  This is O(1), but does not preserve ordering of elements in the vector.
  Aborts if `i` is out of bounds.
  -/
  public fun swap_remove {Element}(
    self : &mut Vector<Element>, i : u64
  ) -> Element := do
    assert!(!self.is_empty(), EINDEX_OUT_OF_BOUNDS)
    let last_idx := self.length - 1
    self.swap(i, last_idx)
    return self.pop_back()

  spec swap_remove where
    pragma intrinsic

  /--
  Replace the `i`th element of the vector `self` with the given value, and return
  to the caller the value that was there before.
  Aborts if `i` is out of bounds.
  -/
  public fun replace {Element}(
    self : &mut Vector<Element>, i : u64, val : Element
  ) -> Element := do
    let last_idx := self.length
    assert!(i < last_idx, EINDEX_OUT_OF_BOUNDS)
    return if USE_MOVE_RANGE then mem::replace(&mut self[i], val)
    else
      *self := core.prim.pushVector(*self, val)
      self.swap(i, last_idx)
      return self.pop_back()

  /--
  rotate(&mut [1, 2, 3, 4, 5], 2) -> [3, 4, 5, 1, 2] in place, returns the split point
  ie. 3 in the example above
  -/
  public fun rotate {Element}(
    self : &mut Vector<Element>, rot : u64
  ) -> u64 := do
    let len := self.length
    return self.rotate_slice(0, rot, len)

  spec rotate where
    pragma intrinsic

  /--
  Same as above but on a sub-slice of an array [left, right) with left <= rot <= right
  returns the
  -/
  public fun rotate_slice {Element}(
    self : &mut Vector<Element>, left : u64, rot : u64, right : u64
  ) -> u64 := do
    self.reverse_slice(left, rot)
    self.reverse_slice(rot, right)
    self.reverse_slice(left, right)
    return left + (right - rot)

  spec rotate_slice where
    pragma intrinsic

  public fun range(start : u64, end : u64) -> Vector<u64> :=
    range_with_step(start, end, 1)

  public fun range_with_step(
    start : u64, end : u64, step : u64
  ) -> Vector<u64> := do
    assert!(step > 0, EINVALID_STEP)
    let mut vec := vector<u64>[]
    while start < end do
      vec := core.prim.pushVector(vec, start)
      start := start + step
    return vec

  public fun slice {Element has Copy}(
    self : &Vector<Element>, start : u64, end : u64
  ) -> Vector<Element> := do
    assert!(start <= end && self.length >= end, EINVALID_SLICE_RANGE)
    let mut vec := vector<Element>[]
    while start < end do
      vec := core.prim.pushVector(vec, self[start])
      start := start + 1
    return vec

  -- =================================================================
  -- Module Specification
  -- Switch to module documentation context
  /--
  Check if `self` is equal to the result of adding `e` at the end of `v2`
  -/
  spec fun eq_push_back {Element}(
    self : Vector<Element>, v2 : Vector<Element>, e : Element
  ) : Bool :=
    self.length == v2.length + 1 && self[self.length - 1] == e
      && self[0 .. self.length - 1] == v2[0 .. v2.length]

  /--
  Check if `self` is equal to the result of concatenating `v1` and `v2`
  -/
  spec fun eq_append {Element}(
    self : Vector<Element>, v1 : Vector<Element>, v2 : Vector<Element>
  ) : Bool :=
    self.length == v1.length + v2.length && self[0 .. v1.length] == v1
      && self[v1.length .. self.length] == v2

  /--
  Check `self` is equal to the result of removing the first element of `v2`
  -/
  spec fun eq_pop_front {Element}(
    self : Vector<Element>, v2 : Vector<Element>
  ) : Bool :=
    self.length + 1 == v2.length && self == v2[1 .. v2.length]

  /--
  Check that `v1` is equal to the result of removing the element at index `i` from `v2`.
  -/
  spec fun eq_remove_elem_at_index {Element}(
    i : Int, v1 : Vector<Element>, v2 : Vector<Element>
  ) : Bool :=
    v1.length + 1 == v2.length && v1[0 .. i] == v2[0 .. i]
      && v1[i .. v1.length] == v2[i + 1 .. v2.length]

  /--
  Check if `self` contains `e`.
  -/
  spec fun spec_contains {Element}(
    self : Vector<Element>, e : Element
  ) : Bool :=
    ∃ (x in self), x == e

  /--
  `f` folded over `v[0..end]`, starting from `init`.
  -/
  spec fun spec_fold {Element} {Acc}(
    f : Fn(Acc, &Element) -> Acc, v : Vector<Element>, init : Acc, end : Int
  ) : Acc :=
    if end == 0 then init
    else result_of<f>(spec_fold(f, v, init, end - 1), v[end - 1])

  /--
  `t` folded over indices `0..end`, starting from `init`.
  -/
  spec fun spec_fold_idx {Acc}(
    t : Fn(Acc, u64) -> Acc, init : Acc, end : Int
  ) : Acc :=
    if end == 0 then init
    else result_of<t>(spec_fold_idx(t, init, end - 1), end - 1)

  /--
  The result of mapping `f` over the prefix `v[0..end]`.
  -/
  spec fun spec_map_ref {Element} {NewElement}(
    f : Fn(&Element) -> NewElement, v : Vector<Element>, end : Int
  ) : Vector<NewElement> :=
    if end == 0 then vec::<NewElement>()
    else concat(spec_map_ref(f, v, end - 1), vec(result_of<f>(v[end - 1])))

  /--
  Whether mapping `f` over the prefix `v[0..end]` aborts.
  -/
  spec fun spec_map_ref_aborts {Element} {NewElement}(
    f : Fn(&Element) -> NewElement, v : Vector<Element>, end : Int
  ) : Bool :=
    end > 0 && (spec_map_ref_aborts(f, v, end - 1) || aborts_of<f>(v[end - 1]))
