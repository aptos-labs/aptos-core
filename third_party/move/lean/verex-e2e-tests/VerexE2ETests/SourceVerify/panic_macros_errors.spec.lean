-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Each is false: a failure no clause permits, at its source location, a condition
-- that does not hold, or a result that does not hold.

spec plain_assert where
  ensures result == c

spec message_assert where
  panics_if c == 1
  ensures result == c

spec literal_message_assert where
  ensures result == c

spec literal_panic where
  ensures result == c

spec format_panic where
  panics_if c == 0
  ensures result == c + 1

spec unreachable_arm where
  ensures result == c

spec debug_only where
  ensures result == c

spec expect_option where
  ensures result == value.0

spec unwrap_option where
  ensures result == value.0

spec expect_result where
  ensures result == core.data.selectVariants[core::result::Result<u64, u64>, Ok.0](value)

spec unwrap_result where
  ensures result == core.data.selectVariants[core::result::Result<u64, u64>, Ok.0](value)

spec static_read where
  ensures result == 8

spec const_limit where
  panics_if c >= 101
  ensures result == 100 - c

spec maximum_guard where
  ensures result == c + 1

-- A panic on every path does not prove `ensures false` by itself.
spec always where
  ensures false

-- The path past the panic holds no such result.
spec past_the_panic where
  panics_if c == 0
  ensures false

-- A clause permitting more than the panic does not hold on the path past it.
spec not_always where
  panics_if true
  ensures false

-- The panic after a join is reached after what dominates it, not one of the arms.
spec join_assert where
  ensures a ==> result == b / 2
  ensures !a ==> result == b / 3

spec join_panic where
  ensures result > 100

-- No guard: reached after the function's own span.
spec unguarded where
  ensures true
