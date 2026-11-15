-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- The write through the yielded reference reaches the slice, so claiming the
-- element is preserved is refuted. Were the contract still to carry the
-- premise that the payload's prophecy is its current value, this would verify
-- vacuously instead.
spec preserved where
  ensures ∀ (k in 0 .. values.length), k != i ==> values[k] == old(values)[k]
  ensures i < old(values).length ==>
    values[i] == old(values)[i] -- error: the write through the reference lands

-- Reading through the yielded reference before writing is not yet related to
-- the element automatically: the untouched positions are derived, the written
-- one is not.
spec increment where
  ensures ∀ (k in 0 .. values.length), k != i ==> values[k] == old(values)[k]
  ensures i < old(values).length ==>
    values[i] == (old(values)[i] + 1) % 4294967296 -- error: not yet derived
