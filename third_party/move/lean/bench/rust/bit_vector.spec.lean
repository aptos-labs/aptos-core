-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

impl BitVector where
  spec new where
    panics_if length == 0
    panics_if length >= 1024
    ensures result.bits.length == length
    ensures ∀ (k in 0 .. length), !result.bits[k]

  spec set where
    panics_if index >= old(self).bits.length
    ensures self.bits[index]
    ensures self.bits.length == old(self).bits.length
    ensures ∀ (k in 0 .. self.bits.length), k != index ==> self.bits[k] == old(self).bits[k]

  spec unset where
    panics_if index >= old(self).bits.length
    ensures !self.bits[index]
    ensures self.bits.length == old(self).bits.length
    ensures ∀ (k in 0 .. self.bits.length), k != index ==> self.bits[k] == old(self).bits[k]

  spec is_index_set where
    panics_if index >= self.bits.length
    ensures result == self.bits[index]

  spec length where
    ensures result == self.bits.length

  spec shift_left where
    ensures self.bits.length == old(self).bits.length
    ensures amount >= self.bits.length ==> ∀ (k in 0 .. self.bits.length), !self.bits[k]
    ensures amount < self.bits.length ==>
      ∀ (k in self.bits.length - amount .. self.bits.length), !self.bits[k]
    ensures amount < self.bits.length ==>
      ∀ (k in 0 .. self.bits.length - amount), self.bits[k] == old(self).bits[k + amount]
    loop 1 where
      invariant iter.start <= iter.end && iter.end == length
      invariant amount <= iter.start
      invariant self.bits.length == length && length == old(self).bits.length
      invariant ∀ (k in 0 .. iter.start - amount), self.bits[k] == old(self).bits[k + amount]
      invariant ∀ (k in iter.start - amount .. length), self.bits[k] == old(self).bits[k]
    loop 2 where
      invariant iter'.start <= iter'.end && iter'.end == length
      invariant length - amount <= iter'.start
      invariant self.bits.length == length && length == old(self).bits.length
      invariant ∀ (k in 0 .. length - amount), self.bits[k] == old(self).bits[k + amount]
      invariant ∀ (k in length - amount .. iter'.start), !self.bits[k]

  spec longest_set_sequence_starting_at where
    panics_if start >= self.bits.length
    ensures ∀ (k in start .. start + result), self.bits[k]
    ensures start + result <= self.bits.length
    loop 1 where
      invariant start <= index && index <= self.bits.length
      invariant ∀ (k in start .. index), self.bits[k]
