-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec divide where
  requires b != 0
  ensures result == a / b

spec overflow_value where
  requires a + b <= 255
  ensures result == a + b
