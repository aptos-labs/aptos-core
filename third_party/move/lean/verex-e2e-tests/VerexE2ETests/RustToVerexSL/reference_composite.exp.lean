-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace reference_composite where
  fun first_pair(left : &u32, right : &u32) -> u32 := do
    let pair := (left, right)
    let _4 := copy(pair[0u32])
    *_4

-- no denotation for `first_pair`: an element place on a non-vector is not carried by the denotation
