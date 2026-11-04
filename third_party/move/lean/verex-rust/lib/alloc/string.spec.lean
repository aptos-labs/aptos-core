-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

mod string where
  @[model(string)] opaque struct String where

  impl String where
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
