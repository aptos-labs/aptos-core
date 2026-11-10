-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec valid_empty where
  ensures !result -- error: deliberately false postcondition

spec valid_ascii where
  ensures !result -- error: deliberately false postcondition

spec valid_two_byte where
  ensures !result -- error: deliberately false postcondition

spec valid_three_byte where
  ensures !result -- error: deliberately false postcondition

spec valid_four_byte where
  ensures !result -- error: deliberately false postcondition

spec invalid_ff where
  ensures !result -- error: deliberately false postcondition

spec invalid_continuation where
  ensures !result -- error: deliberately false postcondition

spec invalid_overlong where
  ensures !result -- error: deliberately false postcondition

spec invalid_truncated where
  ensures !result -- error: deliberately false postcondition

spec invalid_surrogate where
  ensures !result -- error: deliberately false postcondition

spec invalid_beyond_unicode where
  ensures !result -- error: deliberately false postcondition
