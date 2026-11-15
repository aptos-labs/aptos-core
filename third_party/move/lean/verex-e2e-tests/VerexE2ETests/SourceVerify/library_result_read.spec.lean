-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec or_panic where
  panics_if value is Err
  ensures result == core.data.selectVariants[Result<u32, u8>, Ok.0](value)

spec success where
  ensures (result is Some) == (value is Ok)
  ensures value is Ok ==> result.0 == core.data.selectVariants[Result<u32, u8>, Ok.0](value)

spec failure where
  ensures (result is Some) == (value is Err)
  ensures value is Err ==> result.0 == core.data.selectVariants[Result<u32, u8>, Err.0](value)
