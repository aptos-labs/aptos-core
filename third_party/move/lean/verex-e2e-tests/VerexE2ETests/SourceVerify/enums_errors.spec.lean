-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Each contract states another value.

spec size where
  ensures result == 0

spec read where
  ensures result == borrowed.value + 1

spec use_first where
  ensures result == 0
