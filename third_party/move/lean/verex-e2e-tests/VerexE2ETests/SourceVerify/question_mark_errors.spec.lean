-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Each is false: the wrong branch of the `?`, or the value it passes.

spec option_add where
  ensures result is Some

spec option_two where
  ensures (result is Some) == (x is Some || y is Some)

spec result_add where
  ensures x is Err ==> result is Ok

spec result_two where
  ensures x is Ok && y is Err ==> core.data.selectVariants[core::result::Result<u32, u64>, Err.0](result) == core.data.selectVariants[core::result::Result<u32, u64>, Ok.0](x) + 1

spec result_to_other_ok where
  ensures x is Err ==> core.data.selectVariants[core::result::Result<Bool, u64>, Err.0](result) != core.data.selectVariants[core::result::Result<u32, u64>, Err.0](x)

spec option_increment where
  ensures (result is Some) == (x is Some)
