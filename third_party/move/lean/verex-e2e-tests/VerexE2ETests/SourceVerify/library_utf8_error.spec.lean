-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec recover where
  ensures result == bytes

spec error_length where
  ensures result

spec roundtrip where
  ensures result

spec discard_error where
  ensures (result is Some) == (exists (text : string),
    text.length == bytes.length && spec.stringBytes(text) == bytes)
  ensures result is Some ==> result.0 == bytes
