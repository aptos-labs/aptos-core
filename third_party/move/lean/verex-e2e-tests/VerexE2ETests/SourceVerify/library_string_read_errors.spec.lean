-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec bytes where
  ensures result == 0 -- error: not of a non-empty string

spec blank where
  ensures result -- error: not of a non-empty string

spec view where
  ensures result.length == 0 -- error: it is the whole string

spec blank_view where
  ensures !result -- error: not of an empty string
