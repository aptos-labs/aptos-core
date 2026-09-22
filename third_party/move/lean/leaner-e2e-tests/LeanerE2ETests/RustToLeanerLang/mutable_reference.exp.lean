-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace mutable_reference where
  fun replace(value : &mut u32, replacement : u32) -> u32 := do
    *value := replacement
    return *value
