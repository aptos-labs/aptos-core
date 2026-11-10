-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec read where
  ensures result == x

spec get where
  ensures result == h.r

spec through where
  ensures result == x

spec lender_written where
  ensures result == 3

spec slot where
  ensures result == x
