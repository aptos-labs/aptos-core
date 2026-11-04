-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec unguarded where
  ensures result == values[position]

spec wrong_element where
  requires values.length > 0
  ensures result == 0

spec wrong_lookup where
  ensures result == 1

-- Indexing by a range matches no shape of the contract.
spec from_range where
  ensures true
