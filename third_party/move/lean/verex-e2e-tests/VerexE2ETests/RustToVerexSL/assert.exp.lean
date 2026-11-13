-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace «assert» where
  fun divide(dividend : u32, divisor : u32) -> u32 :=
    core.prim.checkedDividePanic(dividend, divisor)
