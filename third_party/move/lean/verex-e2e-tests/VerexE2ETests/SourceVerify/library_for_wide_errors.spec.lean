-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec count where
  ensures result == n
  loop 1 where
    invariant iter.start <= iter.end && iter.end == n
    invariant total == iter.start + 1
