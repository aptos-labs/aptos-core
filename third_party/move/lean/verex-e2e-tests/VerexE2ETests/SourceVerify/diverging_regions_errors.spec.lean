-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Each is false: a panic no clause permits, under the site of its region, or a
-- clause or a result that does not hold.

spec unpermitted where
  ensures result == c

spec wrong_condition where
  panics_if c == 1
  ensures result == c

spec wrong_result where
  panics_if c == 0
  ensures result == c + 1

spec unpermitted_abort where
  ensures result == c

spec spinning where
  ensures result == c

spec two_regions where
  panics_if c == 0
  ensures result == c

spec only_exit where
  ensures false
  loop 1 where
    invariant true
