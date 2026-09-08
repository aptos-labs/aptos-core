-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace reference where
  fun read(value : &u32) -> u32 := *value
