-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec append where
  ensures result == old(text).length -- error: it grows

spec append_char where
  ensures text == old(text) -- error: it grows

spec bytes_of where
  ensures result.length == 0 -- error: not of a non-empty string

spec same_length where
  ensures !result -- error: they are equal

spec view_bytes where
  ensures result == 0 -- error: not of a non-empty string
