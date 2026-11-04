-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- The abort is not permitted, and the value is another.

spec abort_now where
  ensures true

spec spin_unless where
  ensures result == value + 1
