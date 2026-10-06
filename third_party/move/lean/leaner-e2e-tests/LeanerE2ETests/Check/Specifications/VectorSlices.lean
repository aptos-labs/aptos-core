-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Specifications read vector literals, empty vectors, and slices: a slice
of a literal, an empty slice, a whole slice of a parameter, in either
orientation, and a literal whose integer elements have another width. -/

leaner module 0x42::vector_slices where
  fun three() -> Vector<u64> := vector<u64>[1, 2, 3]
  spec three where
    ensures result == vector<u64>[1, 2, 3]
    ensures result == vector<u256>[1, 2, 3]
    ensures result[0 .. 2] == vector<u64>[1, 2]
    ensures result[1 .. 1] == vector<u64>[]

  fun same(v : Vector<u64>) -> Vector<u64> := v
  spec same where
    ensures result == v[0 .. v.length]
    ensures result[0 .. v.length] == v
