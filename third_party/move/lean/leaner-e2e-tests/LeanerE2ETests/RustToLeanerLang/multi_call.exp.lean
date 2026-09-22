-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace multi_call where
  fun transform(value : u32) -> u32 := value ^ 1u32

  fun apply(value : u32) -> u32 := transform(value)
