-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

impl BitVector where
  spec new_unbounded where
    panics_if length == 0
    ensures result.bits.length == length

  spec new_longer where
    panics_if length == 0
    panics_if length >= 1024
    ensures result.bits.length == length + 1

  spec set where
    ensures self.bits[index]

  spec unset_other where
    panics_if index >= old(self).bits.length
    ensures ∀ (k in 0 .. self.bits.length), !self.bits[k]

  spec shift_left where
    ensures self.bits.length == old(self).bits.length
    ensures amount < self.bits.length ==>
      ∀ (k in 0 .. self.bits.length - amount), self.bits[k] == old(self).bits[k]
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
    ensures ∀ (k in start .. start + result + 1), self.bits[k]
    loop 1 where
      invariant start <= index && index <= self.bits.length
      invariant ∀ (k in start .. index), self.bits[k]
