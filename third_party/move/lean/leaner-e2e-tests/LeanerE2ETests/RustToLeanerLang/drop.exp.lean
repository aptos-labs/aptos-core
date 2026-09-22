-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace «drop» where
  struct Token where
    0 : u32

  fun «drop»(self : &mut Token) -> Unit := ()

  fun consume(_token : Token) -> Unit := drop(_token)
