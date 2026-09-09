-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace «enum» where
  enum Choice where
    | First (0 : u32) = 4
    | Second (0 : u32) = 9

  fun first(value : u32) -> Choice := new Choice::First { 0 := value }

  fun select(value : Choice) -> u32 :=
    match discriminant[Choice, isize](value) with
      | 4 => value.0
      | 9 => value.0
