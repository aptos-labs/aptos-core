-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace «drop» where
  @[destructor]
  struct Token where
    0 : u32

  trait impl «<drop::Token as std::ops::Drop>» where
    fun «drop»(self : &mut Token) -> Unit := ()

  fun consume(_token : Token) -> Unit := drop(_token)

-- no denotation for `consume`: a drop at a type with a destructor is not carried by the denotation
