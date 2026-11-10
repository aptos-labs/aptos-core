-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

mod string where
  @[model(string)] opaque struct String where

  -- Retain the input bytes. Error offsets are not exposed by this model yet.
  @[model] struct FromUtf8Error where
    bytes : Vector<u8>

  impl FromUtf8Error where
    spec into_bytes where
      ensures result == _1.bytes

    spec as_bytes where
      ensures result == _1.bytes

  impl String where
    spec from_utf8 where
      ensures (result is Ok) == (exists (text : string), text.length == _1.length && spec.stringBytes(text) == _1)
      ensures result is Ok ==> spec.stringBytes(core.data.selectVariants[core::result::Result<string, alloc::string::FromUtf8Error>, Ok.0](result)) == _1
      ensures result is Ok ==> (core.data.selectVariants[core::result::Result<string, alloc::string::FromUtf8Error>, Ok.0](result)).length == _1.length
      ensures result is Err ==> (core.data.selectVariants[core::result::Result<string, alloc::string::FromUtf8Error>, Err.0](result)).bytes == _1

    spec into_bytes where
      ensures result == spec.stringBytes(_1)
      ensures result.length == _1.length

    spec len where
      ensures result == _1.length

    spec is_empty where
      ensures result == (_1.length == 0)

    spec as_str where
      ensures result == _1

    spec as_bytes where
      ensures result == spec.stringBytes(_1)
      ensures result.length == _1.length

    -- Capacity and allocation are outside the model, as for `Vec::push`.
    spec push_str where
      ensures old(_1).length + _2.length <= MAX_U64
      ensures _1 == spec.stringConcat(old(_1), _2)
      ensures _1.length == old(_1).length + _2.length

    spec push where
      ensures old(_1).length + spec.charEncode(_2).length <= MAX_U64
      ensures _1.length == old(_1).length + spec.charEncode(_2).length
      ensures spec.stringBytes(_1) == concat(spec.stringBytes(old(_1)), spec.charEncode(_2))

  trait impl «<std::string::String as std::clone::Clone>» where
    spec clone where
      ensures result == _1

  -- A `str` displays as itself, a `String` as its `str`.
  trait impl «<T as std::string::ToString>» where
    spec to_string at {string} where
      ensures result == _1
