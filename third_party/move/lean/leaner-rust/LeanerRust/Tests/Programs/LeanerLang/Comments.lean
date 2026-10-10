-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace comments where
  -- A source comment before the first declaration.
  fun mask_low(value : u32) -> u32 := value ^ 1u32

  -- A block comment between declarations.
  fun mask_high(value : u32) -> u32 := value ^ 2u32

  -- A trailing source comment.
