-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace lifetime_drop where
  @[destructor]
  struct Anonymous {a : lifetime} where
    r : &[a] mut u32

  trait impl «<lifetime_drop::Anonymous<'_> as std::ops::Drop>» where
    fun «drop» {l0 : lifetime}(
      self : &[l0] mut Anonymous<lifetime l0>
    ) -> Unit := do
      let _2 := &mut *((*self).r)
      *_2 := 9u32

  @[destructor]
  struct Named {a : lifetime} where
    r : &[a] mut u32

  trait impl «<lifetime_drop::Named<'a> as std::ops::Drop>» where
    fun «drop» {a : lifetime}(self : &[a] mut Named<lifetime a>) -> Unit := do
      let _2 := &mut *((*self).r)
      *_2 := 9u32
