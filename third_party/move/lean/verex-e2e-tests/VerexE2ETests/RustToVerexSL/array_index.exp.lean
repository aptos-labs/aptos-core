-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace array_index where
  fun get(values : Vector<u32, const 4>, index : usize) -> u32 := values[index]

  fun get_third(values : Vector<u32, const 4>) -> u32 := values[2usize]

  fun destructure(values : Vector<u32, const 4>) -> u32 := values[0usize]

-- no denotation for `get`: the type of a local is not carried by the denotation
-- no denotation for `get_third`: the type of a local is not carried by the denotation
-- no denotation for `destructure`: the type of a local is not carried by the denotation
