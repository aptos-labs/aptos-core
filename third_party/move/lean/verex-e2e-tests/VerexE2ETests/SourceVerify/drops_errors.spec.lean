-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- The value kept is the other, and the dropped one stays dropped.

spec pick where
  ensures first ==> result == right

spec replace_once where
  ensures result == initial
  loop 1 where
    invariant index <= 1
    invariant index <= count
    invariant result == initial
