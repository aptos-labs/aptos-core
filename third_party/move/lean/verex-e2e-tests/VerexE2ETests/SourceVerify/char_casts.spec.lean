-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec from_ascii where
  ensures true

spec code_of_ascii where
  ensures result == value

spec is_letter where
  ensures result == (value == 97)

spec bounded_code where
  ensures result <= 1114111

spec outside_surrogates where
  ensures result < 55296 || result > 57343
