-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec append where
  ensures text == spec.stringConcat(old(text), tail)
  ensures result == old(text).length + tail.length

spec append_char where
  ensures result == old(text).length + spec.charEncode(value).length
  ensures spec.stringBytes(text) == concat(spec.stringBytes(old(text)), spec.charEncode(value))

spec bytes_of where
  ensures result == spec.stringBytes(text)

spec same_length where
  ensures result

spec view_bytes where
  ensures result == text.length
