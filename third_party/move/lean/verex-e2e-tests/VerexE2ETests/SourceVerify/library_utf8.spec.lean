-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec roundtrip where
  ensures result

spec success_bytes where
  ensures result is Some ==> result.0 == bytes

spec success_length where
  ensures result

spec decode where
  ensures (result is Ok) == (exists (text : string),
    text.length == bytes.length && spec.stringBytes(text) == bytes)
