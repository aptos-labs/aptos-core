-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec count where
  ensures result == n
  loop 1 where
    invariant iter.start <= iter.end && iter.end == n
    invariant total == iter.start

spec count_signed where
  requires n >= 0
  ensures result == n
  loop 1 where
    invariant 0 <= iter.start && iter.start <= iter.end && iter.end == n
    invariant total == iter.start

spec both where
  requires n <= 1000 && values.length <= 1000
  ensures result == values.length + n
  loop 1 where
    invariant iter.start <= iter.end && iter.end == values.length
    invariant total == iter.start
  loop 2 where
    invariant iter.start <= iter.end && iter.end == values.length
    invariant iter'.start <= iter'.end && iter'.end == n
    invariant total == values.length + iter'.start
