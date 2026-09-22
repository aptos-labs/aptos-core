-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace unary where
  fun negate(value : i32) -> i32 :=
    if value == -2147483648 then panic() else -value
