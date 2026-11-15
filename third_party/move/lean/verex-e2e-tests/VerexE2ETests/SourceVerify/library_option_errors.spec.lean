-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec add_or_zero where
  ensures result == left + right

spec or_default where
  ensures result == 7

-- Panics at its call of `unwrap`.
spec must_none where
  ensures result == 0

spec must_unstated where
  ensures result == value.0

spec present where
  ensures result

spec must_err where
  ensures result == 0
