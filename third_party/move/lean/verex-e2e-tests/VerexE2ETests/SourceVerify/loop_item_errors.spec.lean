-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A function with two loops has no third.
spec two_loops where
  loop 3 where
    invariant i <= n
