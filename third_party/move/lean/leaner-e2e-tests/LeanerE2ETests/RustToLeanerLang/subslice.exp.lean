-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace subslice where
  fun tail_len(values : &Vector<u32>) -> usize :=
    if values.length >= 1usize then
      slice(*values, 1usize, values.length).length
    else 0usize
