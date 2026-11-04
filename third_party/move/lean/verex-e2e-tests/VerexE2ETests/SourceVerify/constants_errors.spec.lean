-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Each contract states another value of the constant.

spec below_limit where
  ensures result == (value < 1023)

spec maximum where
  ensures result == 18446744073709551614

spec count where
  ensures result == 8
