-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace match_guard where
  enum Maybe where
    | None = 0
    | Some (0 : u32) = 1

  fun positive_or(value : Maybe, fallback : u32) -> u32 :=
    match discriminant[Maybe, isize](value) with
      | 1 => do
        let candidate' := &value.0
        return if *candidate' > 0u32 then value.0 else fallback
      | 0 => fallback
