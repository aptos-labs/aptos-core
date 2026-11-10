-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec roundtrip where
  ensures !result -- error: bytes from a string are valid UTF-8

spec success_bytes where
  ensures result is Some ==> (result.0).length == 0 -- error: successful input need not be empty

spec success_length where
  ensures !result -- error: successful decoding preserves the byte length

spec decode where
  ensures result is Err -- error: valid UTF-8 succeeds

spec all_valid where
  ensures result -- error: arbitrary input need not be valid UTF-8
