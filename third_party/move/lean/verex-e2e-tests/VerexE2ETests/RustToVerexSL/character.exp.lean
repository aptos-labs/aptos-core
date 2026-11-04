-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace character where
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

-- no denotation for `ordering`: the type of a local is not carried by the denotation
-- no denotation for `literal`: the type of the result is not carried by the denotation
-- no denotation for `classify`: the type of a local is not carried by the denotation
-- no denotation for `to_u32`: the type of a local is not carried by the denotation
-- no denotation for `from_ascii`: the type of the result is not carried by the denotation
