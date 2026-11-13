-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- The index is read before the block of the second argument writes it.

spec pick where
  ensures result == x

spec index_then_write where
  requires r < 4
  ensures result == a[old(r)]
  ensures r == v

spec index_then_write_false where
  requires r < 4
  requires v < 4
  ensures result == a[v] -- error: the element is read before the write
