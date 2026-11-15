-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec recover where
  ensures result.length == 0 -- error: deliberately false postcondition

spec error_length where
  ensures !result -- error: deliberately false postcondition

spec roundtrip where
  ensures !result -- error: deliberately false postcondition

spec discard_error where
  ensures result is Some -- error: deliberately false postcondition
