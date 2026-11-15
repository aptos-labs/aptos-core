-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace tuple_callback_borrows where
  -- Tuple support must not erase sharing inside the returned aggregate.
  @[method_evidence = 1]
  fun apply {a : lifetime} {F} {l2 : lifetime}(
    callback : &[l2] mut F,
    __trait_FnMut_call_mut : Fn(&mut F) -> (&[a] u64, Bool) has Copy, Drop
  ) -> (&[a] u64, Bool) := invoke(__trait_FnMut_call_mut, &mut *callback)

  fun borrowed(value : &u64) -> (&u64, Bool) := do
    let mut callback :=
      closure «borrowed::{closure#0}» {u0 : lifetime} {u1 : lifetime} has Copy (
        value : &[u0] &[u1] u64 := &value
      )
        fun call(
          self : &«borrowed::{closure#0}»<lifetime _, lifetime _>
        ) -> (&u64, Bool) := do
          let _3 := copy((*self).value)
          (copy(*_3), true)
    let _4 := &mut callback
    apply(
      _4,
      function[Fn(&mut «borrowed::{closure#0}»<lifetime _, lifetime _>) -> (&u64, Bool) has Copy, Drop](«__fnmut_adapter_borrowed::{closure#0}»)
    )

  @[closure_method_adapter = "borrowed::{closure#0}::call"]
  fun «__fnmut_adapter_borrowed::{closure#0}»(
    self : &mut «borrowed::{closure#0}»<lifetime _, lifetime _>
  ) -> (&u64, Bool) := «borrowed::{closure#0}»::call(&*self)

-- no denotation for `borrowed`: a closure whose rows are not its target's is not carried by the denotation
