-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace slice_from_end where
  fun last_or_zero(values : &Vector<u32>) -> u32 :=
    if values.length >= 1usize then values[values.length - 1usize] else 0u32
