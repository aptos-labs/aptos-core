-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec option_add where
  ensures (result is Some) == (x is Some)
  ensures x is Some ==> result.0 == x.0 / 2

spec option_two where
  ensures (result is Some) == (x is Some && y is Some)
  ensures result is Some ==> result.0 == x.0 / 2 + y.0 / 2

spec result_add where
  ensures (result is Ok) == (x is Ok)
  ensures x is Ok ==> core.data.selectVariants[core::result::Result<u32, u64>, Ok.0](result) == core.data.selectVariants[core::result::Result<u32, u64>, Ok.0](x) / 2
  ensures x is Err ==> core.data.selectVariants[core::result::Result<u32, u64>, Err.0](result) == core.data.selectVariants[core::result::Result<u32, u64>, Err.0](x)

spec result_two where
  ensures (result is Ok) == (x is Ok && y is Ok)
  ensures x is Err ==> core.data.selectVariants[core::result::Result<u32, u64>, Err.0](result) == core.data.selectVariants[core::result::Result<u32, u64>, Err.0](x)
  ensures x is Ok && y is Err ==> core.data.selectVariants[core::result::Result<u32, u64>, Err.0](result) == core.data.selectVariants[core::result::Result<u32, u64>, Err.0](y)
  ensures result is Ok ==> core.data.selectVariants[core::result::Result<u32, u64>, Ok.0](result) == core.data.selectVariants[core::result::Result<u32, u64>, Ok.0](x) / 2 + core.data.selectVariants[core::result::Result<u32, u64>, Ok.0](y) / 2

spec result_to_other_ok where
  ensures (result is Ok) == (x is Ok)
  ensures x is Err ==> core.data.selectVariants[core::result::Result<Bool, u64>, Err.0](result) == core.data.selectVariants[core::result::Result<u32, u64>, Err.0](x)

spec option_increment where
  panics_if x is Some && x.0 == 4294967295
  ensures (result is Some) == (x is Some)
  ensures x is Some ==> result.0 == x.0 + 1
