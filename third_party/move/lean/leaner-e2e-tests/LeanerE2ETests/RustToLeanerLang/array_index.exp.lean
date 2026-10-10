-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace array_index where
  fun get(values : Vector<u32, const 4>, index : usize) -> u32 :=
    if index < 4usize then values[index] else panic()

  fun get_third(values : Vector<u32, const 4>) -> u32 :=
    if 2usize < 4usize then values[2usize] else panic()

  fun destructure(values : Vector<u32, const 4>) -> u32 := values[0usize]
