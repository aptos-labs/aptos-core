-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Replacing a reference field settles the old lender separately. The first two
-- clauses verify; the last two remain false.

spec caller where
  ensures spec.result[1] == 9
  ensures spec.result[0] == old(a)
  ensures spec.result[0] == 7
  ensures false
