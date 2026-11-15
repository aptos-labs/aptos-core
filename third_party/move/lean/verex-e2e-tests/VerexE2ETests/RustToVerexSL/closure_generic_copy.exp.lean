-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace closure_generic_copy where
  -- Reading a `T: Copy` upvar: the closure body shows the bound.
  fun generic {T has Copy}(x : T) -> T := do
    let g :=
      closure «generic::{closure#0}» {T has Copy} {u0 : lifetime} has Copy (
        x : &[u0] T := &x
      )
        fun call {T has Copy}(
          self : &«generic::{closure#0}»<T, lifetime _>
        ) -> T := do
          let _2 := copy((*self).x)
          copy(*_2)
    «generic::{closure#0}»::call(&g)
