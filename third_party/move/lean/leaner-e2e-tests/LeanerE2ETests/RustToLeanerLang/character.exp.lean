-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace character where
  fun ordering(left : Char, right : Char) -> (Bool, Bool, Bool, Bool) :=
    (left < right, left <= right, left > right, left >= right)

  fun literal() -> Char := '🦀'

  fun classify(value : Char) -> u8 :=
    match value with
      | 'a' => 1u8
      | '🦀' => 2u8
      | _ => 0u8

  fun to_u32(value : Char) -> u32 := value as u32

  fun from_ascii(value : u8) -> Char := value as Char
