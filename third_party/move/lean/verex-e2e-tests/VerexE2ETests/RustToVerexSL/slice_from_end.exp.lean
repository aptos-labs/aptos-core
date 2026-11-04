-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace slice_from_end where
  fun last_or_zero(values : &Vector<u32>) -> u32 :=
    if values.length >= 1usize then values[values.length - 1usize] else 0u32

-- no denotation for `last_or_zero`: the type of an expression is not carried by the denotation
