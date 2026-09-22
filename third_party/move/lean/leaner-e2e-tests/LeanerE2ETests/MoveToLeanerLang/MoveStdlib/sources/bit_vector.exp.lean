-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner module 0x1::bit_vector where
  /--
  The provided index is out of bounds
  -/
  const EINDEX : u64 := 131072

  /--
  An invalid length of bitvector was given
  -/
  const ELENGTH : u64 := 131073

  const WORD_SIZE : u64 := 1

  /--
  The maximum allowed bitvector size
  -/
  const MAX_SIZE : u64 := 1024

  struct BitVector has Copy, Drop, Store where
    length : u64
    bit_field : Vector<Bool>

  spec BitVector where
    invariant length == bit_field.length

  public fun new(length : u64) -> BitVector := do
    assert!(length > 0, ELENGTH)
    assert!(length < MAX_SIZE, ELENGTH)
    let counter := 0
    let mut bit_field := vector<Bool>[]
    while counter < length do
      bit_field := core.prim.pushVector(bit_field, false)
      counter := counter + 1
    where
      invariant counter <= length
      invariant bit_field.length == counter
    spec do
      assert counter == length
      assert bit_field.length == length
    return new BitVector { length, bit_field }

  spec new where
    aborts_if length <= 0 with ELENGTH
    aborts_if length >= MAX_SIZE with ELENGTH
    ensures result.length == length
    ensures result.bit_field.length == length

  /--
  Set the bit at `bit_index` in the `self` regardless of its previous state.
  -/
  public fun set(self : &mut BitVector, bit_index : u64) -> Unit := do
    assert!(self.bit_field.length > bit_index, EINDEX)
    self.bit_field[bit_index] := true

  spec set where
    aborts_if bit_index >= self.length() with EINDEX
    ensures self.bit_field[bit_index]
    ensures self.length == old(self).length
    ensures ∀ (k in 0 .. self.length),
        k != bit_index ==> self.bit_field[k] == old(self).bit_field[k]

  /--
  Unset the bit at `bit_index` in the `self` regardless of its previous state.
  -/
  public fun unset(self : &mut BitVector, bit_index : u64) -> Unit := do
    assert!(self.bit_field.length > bit_index, EINDEX)
    self.bit_field[bit_index] := false

  spec unset where
    aborts_if bit_index >= self.length() with EINDEX
    ensures !self.bit_field[bit_index]
    ensures self.length == old(self).length
    ensures ∀ (k in 0 .. self.length),
        k != bit_index ==> self.bit_field[k] == old(self).bit_field[k]

  /--
  Shift the `self` left by `amount`. If `amount` is greater than the
  bitvector's length the bitvector will be zeroed out.
  -/
  public fun shift_left(self : &mut BitVector, amount : u64) -> Unit :=
    if self.length <= amount then
      let self := &mut self.bit_field
      spec assume folds_capture_anchor!(31)
      let i := 0
      let len := self.length
      while i < len do
        let elem := &mut self[i]
        *elem := false
        i := i + 1
      where
        invariant i <= len
        invariant self.length == with_state_anchor!(31, old(self)).length
        invariant len == self.length
        invariant ∀ (j in 0 .. i), !self[j]
        invariant ∀ (j in 0 .. i), !false
        invariant ∀ (j in i .. len),
          self[j] == with_state_anchor!(31, old(self))[j]
    else
      let i := amount
      while self.length > i do
        if self.is_index_set(i) then self.set(i - amount)
        else self.unset(i - amount)
        i := i + 1
      i := self.length - amount
      while self.length > i do
        self.unset(i)
        i := i + 1

  spec shift_left where
    pragma verify = false

  -- TODO: set to false because data invariant cannot be proved with inline function. Will remove it once inline is supported
  /--
  Return the value of the bit at `bit_index` in the `self`. `true`
  represents "1" and `false` represents a 0
  -/
  public fun is_index_set(self : &BitVector, bit_index : u64) -> Bool := do
    assert!(self.bit_field.length > bit_index, EINDEX)
    return self.bit_field[bit_index]

  spec is_index_set where
    aborts_if bit_index >= self.length() with EINDEX
    ensures result == self.bit_field[bit_index]

  spec fun spec_is_index_set(self : BitVector, bit_index : Int) : Bool :=
    if bit_index >= self.length() then false else self.bit_field[bit_index]

  /--
  Return the length (number of usable bits) of this bitvector
  -/
  public fun length(self : &BitVector) -> u64 := self.bit_field.length

  spec length where
    pragma opaque
    ensures [inferred] self.length == self.bit_field.length
        ==> result == self.bit_field.length
    aborts_if [inferred] false

  /--
  Returns the length of the longest sequence of set bits starting at (and
  including) `start_index` in the `bitvector`. If there is no such
  sequence, then `0` is returned.
  -/
  public fun longest_set_sequence_starting_at(
    self : &BitVector, start_index : u64
  ) -> u64 := do
    assert!(self.length > start_index, EINDEX)
    let index := start_index
    while self.length > index do
      if !self.is_index_set(index) then break
      index := index + 1
    where
      invariant index >= start_index
      invariant index == start_index || self.is_index_set(index - 1)
      invariant index == start_index || self.bit_field.length > index - 1
      invariant ∀ (j in start_index .. index), self.is_index_set(j)
      invariant ∀ (j in start_index .. index), self.bit_field.length > j
    return index - start_index

  spec longest_set_sequence_starting_at where
    aborts_if self.length <= start_index
    ensures ∀ (i in start_index .. result), self.is_index_set(i)

  -- Find the greatest index in the vector such that all indices less than it are set.
  public fun shift_left_for_verification_only(
    self : &mut BitVector, amount : u64
  ) -> Unit :=
    if self.length <= amount then
      let len := self.bit_field.length
      let i := 0
      while i < len do
        let elem := &mut self.bit_field[i]
        *elem := false
        i := i + 1
      where
        invariant len == self.length
        invariant ∀ (k in 0 .. i), !self.bit_field[k]
        invariant ∀ (k in i .. self.length),
          self.bit_field[k] == old(self).bit_field[k]
    else
      let i := amount
      while self.length > i do
        if self.is_index_set(i) then self.set(i - amount)
        else self.unset(i - amount)
        i := i + 1
      where
        invariant i >= amount
        invariant self.length == old(self).length
        invariant ∀ (j in amount .. i),
          old(self).bit_field[j] == self.bit_field[j - amount]
        invariant ∀ (j in i - amount .. self.length),
          old(self).bit_field[j] == self.bit_field[j]
        invariant ∀ (k in 0 .. i - amount),
          self.bit_field[k] == old(self).bit_field[k + amount]
      i := self.length - amount
      while self.length > i do
        self.unset(i)
        i := i + 1
      where
        invariant self.length == old(self).length
        invariant ∀ (j in self.length - amount .. i), !self.bit_field[j]
        invariant ∀ (k in 0 .. self.length - amount),
          self.bit_field[k] == old(self).bit_field[k + amount]
        invariant i >= self.length - amount

  spec shift_left_for_verification_only where
    aborts_if false
    ensures self.length <= amount
        ==> (∀ (k in 0 .. self.length), !self.bit_field[k])
    ensures self.length > amount
        ==> (∀ (i in self.length - amount .. self.length), !self.bit_field[i])
    ensures self.length > amount
        ==> (∀ (i in 0 .. self.length - amount),
          self.bit_field[i] == old(self).bit_field[i + amount])
