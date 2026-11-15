-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace closure_fn_holder where
  fun shared_view(x : &mut u32) -> u32 := do
    let read :=
      closure «shared_view::{closure#0}» {u0 : lifetime} (
        x : &[u0] mut u32 := x
      )
        fun call(
          self : &«shared_view::{closure#0}»<lifetime _>
        ) -> u32 := *(*self).x
    «shared_view::{closure#0}»::call(&read)
