-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec add_or_zero where
  ensures left + right <= MAX_U32 ==> result == left + right
  ensures left + right > MAX_U32 ==> result == 0

spec or_default where
  ensures value is Some ==> result == value.0
  ensures value is None ==> result == 7

spec or_default_some where
  ensures result == 3

spec must where
  panics_if value is None
  ensures result == value.0

spec must_some where
  ensures result == 3

spec present where
  ensures result == (value is Some)

spec absent where
  ensures result == (value is None)

spec ok_or_default where
  ensures value is Ok ==> result == core.data.selectVariants[Result<u32, u8>, Ok.0](value)
  ensures value is Err ==> result == 7

spec must_ok where
  panics_if value is Err
  ensures result == core.data.selectVariants[Result<u32, u8>, Ok.0](value)

spec succeeded where
  ensures result == (value is Ok)

spec failed where
  ensures result == (value is Err)
