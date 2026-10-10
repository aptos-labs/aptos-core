-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace signed_division where
  fun signed_div_rem(dividend : i8, divisor : i8) -> (i8, i8) :=
    if divisor == 0i8 then panic()
    else
      if (divisor == -1i8) & (dividend == -128i8) then panic()
      else
        if divisor == 0i8 then panic()
        else
          if (divisor == -1i8) & (dividend == -128i8) then panic()
          else (dividend / divisor, dividend % divisor)
