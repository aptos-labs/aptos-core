-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec roundtrip where
  ensures result is Ok

spec decode where
  ensures (result is Ok) == (exists (text : string),
    text.length == bytes.length && spec.stringBytes(text) == bytes)

spec success_bytes where
  ensures result is Ok ==> core.data.selectVariants[core::result::Result<Vector<u8>, alloc::string::FromUtf8Error>, Ok.0](result) == bytes

spec success_length where
  ensures result is Ok ==> core.data.selectVariants[core::result::Result<Bool, alloc::string::FromUtf8Error>, Ok.0](result)
