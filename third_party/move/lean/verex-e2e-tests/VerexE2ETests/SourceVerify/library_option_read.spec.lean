-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec or_panic where
  panics_if value is None
  ensures result == value.0

spec view where
  ensures (result is Some) == (value is Some)
  ensures value is Some ==> result.0 == value.0

spec or_error where
  ensures (result is Ok) == (value is Some)
  ensures value is None ==> core.data.selectVariants[Result<u32, u8>, Err.0](result) == error

spec taken where
  ensures result == old(value)
  ensures value is None

spec replaced where
  ensures result == old(value)
  ensures value is Some && value.0 == next

spec defaulted where
  ensures value is Some ==> result == value.0
  ensures value is None ==> result == 0
