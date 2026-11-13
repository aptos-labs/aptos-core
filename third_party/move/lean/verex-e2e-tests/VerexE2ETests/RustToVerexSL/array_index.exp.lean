-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace array_index where
  fun get(values : Vector<u32, const 4>, index : usize) -> u32 := values[index]

  fun get_third(values : Vector<u32, const 4>) -> u32 := values[2usize]

  fun destructure(values : Vector<u32, const 4>) -> u32 := values[0usize]
