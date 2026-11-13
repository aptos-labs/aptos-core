-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace signed_division where
  fun signed_div_rem(dividend : i8, divisor : i8) -> (i8, i8) := do
    let _3 := core.prim.checkedDividePanic(dividend, divisor)
    (_3, core.prim.checkedModuloPanic(dividend, divisor))
