-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace reference_composite where
  fun first_pair(left : &u32, right : &u32) -> u32 := do
    let pair := (left, right)
    let _4 := copy(pair[0u32])
    return *_4
