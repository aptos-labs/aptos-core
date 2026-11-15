-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec count where
  ensures result == n
  loop 1 where
    invariant iter.start <= iter.end && iter.end == n
    invariant total == iter.start

spec sum where
  requires n <= 1000
  ensures 2 * result == n * (n - 1)
  loop 1 where
    invariant iter.start <= iter.end && iter.end == n
    invariant total <= iter.start * 1000
    invariant 2 * total == iter.start * (iter.start - 1)

spec find_vec where
  ensures result == contains(values, target)
  loop 1 where
    invariant iter.start <= iter.end && iter.end == values.length
    invariant ∀ (k in 0 .. iter.start), values[k] != target

spec find_slice where
  ensures result == contains(values, target)
  loop 1 where
    invariant iter.start <= iter.end && iter.end == values.length
    invariant ∀ (k in 0 .. iter.start), values[k] != target

spec nested where
  requires n <= 1000 && m <= 1000
  ensures result == n * m
  loop 1 where
    invariant iter.start <= iter.end && iter.end == n
    invariant total == iter.start * m
  loop 2 where
    invariant iter.start <= iter.end && iter.end == n
    invariant iter'.start <= iter'.end && iter'.end == m
    invariant total == (iter.start - 1) * m + iter'.start
