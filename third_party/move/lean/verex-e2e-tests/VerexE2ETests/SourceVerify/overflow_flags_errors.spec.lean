-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A flag holds exactly where the exact result leaves the width, so these fail.

spec add_flag where
  ensures result == (a + b >= 255)

spec signed_add_flag where
  ensures result == (a + b > 127)

spec unguarded where
  ensures result == a + b
