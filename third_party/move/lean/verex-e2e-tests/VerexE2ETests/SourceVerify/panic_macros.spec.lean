-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Each macro's failure, permitted by a clause or ruled out by a precondition.

spec plain_assert where
  panics_if c == 0
  ensures result == c

spec message_assert where
  panics_if c == 0
  ensures result == c

spec literal_message_assert where
  requires c != 0
  ensures result == c

spec literal_panic where
  panics_if c == 0
  ensures result == c

spec format_panic where
  requires c != 0
  ensures result == c

spec unreachable_arm where
  requires c > 0
  ensures result == c

spec debug_only where
  panics_if c == 0
  ensures result == c

spec expect_option where
  panics_if value is None
  ensures result == value.0

spec unwrap_option where
  requires value is Some
  ensures result == value.0

spec expect_result where
  panics_if value is Err
  ensures result == core.data.selectVariants[core::result::Result<u64, u64>, Ok.0](value)

spec unwrap_result where
  requires value is Ok
  ensures result == core.data.selectVariants[core::result::Result<u64, u64>, Ok.0](value)

spec static_read where
  ensures result == 7

spec const_limit where
  panics_if c >= 100
  ensures result == 100 - c

spec maximum_guard where
  panics_if c >= 18446744073709551615
  ensures result == c + 1

-- Every path panics, so a result clause holds of no path, and only the clause
-- permitting the panic makes this one hold.
spec always where
  panics_if true
  ensures false

spec join_assert where
  panics_if c == 0
  ensures a ==> result == b / 2
  ensures !a ==> result == b / 3

spec join_panic where
  panics_if (if a then b / 2 else b / 3) <= 100
  ensures result > 100

spec unguarded where
  panics_if true
