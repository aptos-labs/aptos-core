-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace generic_enum where
  enum Maybe {T} where
    | None = 0
    | Some (0 : T) = 1

  fun unwrap_or(value : Maybe<u32>, fallback : u32) -> u32 :=
    match discriminant[Maybe<u32>, isize](value) with
      | 0 => fallback
      | 1 => value.0
