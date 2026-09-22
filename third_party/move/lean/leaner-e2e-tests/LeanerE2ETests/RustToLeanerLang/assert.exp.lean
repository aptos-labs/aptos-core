-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace «assert» where
  fun divide(dividend : u32, divisor : u32) -> u32 :=
    if divisor == 0u32 then panic() else dividend / divisor
