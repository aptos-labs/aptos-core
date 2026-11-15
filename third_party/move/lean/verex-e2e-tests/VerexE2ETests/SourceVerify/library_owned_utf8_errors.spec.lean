-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec roundtrip where
  ensures result is Err -- error: deliberately false postcondition

spec decode where
  ensures result is Err -- error: deliberately false postcondition

spec success_bytes where
  ensures result is Ok ==> (core.data.selectVariants[core::result::Result<Vector<u8>, alloc::string::FromUtf8Error>, Ok.0](result)).length == 0 -- error: deliberately false postcondition

spec success_length where
  ensures result is Ok ==> !core.data.selectVariants[core::result::Result<Bool, alloc::string::FromUtf8Error>, Ok.0](result) -- error: deliberately false postcondition

spec all_valid where
  ensures result is Ok -- error: deliberately false postcondition
