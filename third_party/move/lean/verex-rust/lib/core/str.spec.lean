-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

mod primitive where
  impl str where
    spec is_empty where
      ensures result == (_1.length == 0)

    spec as_bytes where
      ensures result == spec.stringBytes(_1)
      ensures result.length == _1.length

mod str where
  mod converts where
    -- Length equality bounds the encoding witness: stringBytes falls back to
    -- an empty vector for logical strings too large for a runtime vector.
    spec fun valid_utf8(bytes : Vector<u8>) : Bool :=
      exists (text : string), text.length == bytes.length && spec.stringBytes(text) == bytes

    spec from_utf8 where
      ensures (result is Ok) == valid_utf8(_1)
      ensures result is Ok ==> spec.stringBytes(core.data.selectVariants[result::Result<&[_] string, str::error::Utf8Error>, Ok.0](result)) == _1
      ensures result is Ok ==> (core.data.selectVariants[result::Result<&[_] string, str::error::Utf8Error>, Ok.0](result)).length == _1.length
