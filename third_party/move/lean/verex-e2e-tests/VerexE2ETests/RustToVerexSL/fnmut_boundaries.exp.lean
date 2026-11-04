-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace fnmut_boundaries where
  @[method_evidence = 2]
  fun argument {F}(
    callback : &mut F, value : u64,
    __trait_FnMut_call_mut : Fn(&mut F, u64) -> u64 has Copy, Drop
  ) -> u64 := invoke(__trait_FnMut_call_mut, &mut *callback, value)

  @[method_evidence = 1]
  fun result_parameter {F} {R}(
    callback : &mut F,
    __trait_FnMut_call_mut : Fn(&mut F) -> R has Copy, Drop
  ) -> R := invoke(__trait_FnMut_call_mut, &mut *callback)

  @[method_evidence = 1]
  fun result_pair {F}(
    callback : &mut F,
    __trait_FnMut_call_mut : Fn(&mut F) -> (u64, u64) has Copy, Drop
  ) -> (u64, u64) := invoke(__trait_FnMut_call_mut, &mut *callback)

  @[method_evidence = 1]
  fun result_reference {F} {l1 : lifetime}(
    callback : &[l1] mut F,
    __trait_FnMut_call_mut : Fn(&mut F) -> &[static] u64 has Copy, Drop
  ) -> &[static] u64 := invoke(__trait_FnMut_call_mut, &mut *callback)

  @[method_evidence = 1]
  fun shared {F}(
    callback : &F, __trait_Fn_call : Fn(&F) -> u64 has Copy, Drop
  ) -> u64 := invoke(__trait_Fn_call, callback)

  @[method_evidence = 1]
  fun once {F}(
    callback : F, __trait_FnOnce_call_once : Fn(F) -> u64 has Copy, Drop
  ) -> u64 := invoke(__trait_FnOnce_call_once, callback)

  @[method_evidence = 1]
  fun apply {F}(
    callback : &mut F,
    __trait_FnMut_call_mut : Fn(&mut F) -> u64 has Copy, Drop
  ) -> u64 := invoke(__trait_FnMut_call_mut, &mut *callback)

  -- This Fn closure uses the checked shared-receiver adapter for FnMut.
  fun adapter(value : u64) -> u64 := do
    let mut callback :=
      closure «adapter::{closure#0}» {u0 : lifetime} has Copy (
        value : &[u0] u64 := &value
      )
        fun call(self : &«adapter::{closure#0}»<lifetime _>) -> u64 := do
          let _2 := copy((*self).value)
          *_2
    let _4 := &mut callback
    apply(
      _4,
      function[Fn(&mut «adapter::{closure#0}»<lifetime _>) -> u64 has Copy, Drop](«__fnmut_adapter_adapter::{closure#0}»)
    )

  @[closure_method_adapter = "adapter::{closure#0}::call"]
  fun «__fnmut_adapter_adapter::{closure#0}»(
    self : &mut «adapter::{closure#0}»<lifetime _>
  ) -> u64 := «adapter::{closure#0}»::call(&*self)
