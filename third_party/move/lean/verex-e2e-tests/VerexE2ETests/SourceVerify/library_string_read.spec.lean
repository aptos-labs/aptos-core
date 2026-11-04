-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec bytes where
  ensures result == text.length

spec blank where
  ensures result == (text.length == 0)

spec view where
  ensures result == text

spec blank_view where
  ensures result == (text.length == 0)
