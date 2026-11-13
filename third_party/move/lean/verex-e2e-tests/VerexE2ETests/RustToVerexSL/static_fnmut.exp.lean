-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace static_fnmut where
  @[method_evidence = 1]
  fun apply_mut {F}(
    callback : &mut F,
    __trait_FnMut_call_mut : Fn(&mut F) -> u64 has Copy, Drop
  ) -> u64 := do
    let _2 := invoke(__trait_FnMut_call_mut, &mut *callback)
    invoke(__trait_FnMut_call_mut, &mut *callback)

  @[method_evidence = 1]
  fun forward {F}(
    callback : &mut F,
    __trait_FnMut_call_mut : Fn(&mut F) -> u64 has Copy, Drop
  ) -> u64 := apply_mut(callback, __trait_FnMut_call_mut)

  @[method_evidence = 1]
  fun apply_bool {F}(
    callback : &mut F,
    __trait_FnMut_call_mut : Fn(&mut F) -> Bool has Copy, Drop
  ) -> Bool := do
    let _2 := invoke(__trait_FnMut_call_mut, &mut *callback)
    invoke(__trait_FnMut_call_mut, &mut *callback)

  fun counter(value : u64) -> u64 := do
    let mut callback :=
      closure «counter::{closure#0}» has Copy (count : u64 := value)
        fun call_mut(self : &mut «counter::{closure#0}») -> u64 := do
          (*self).count := (*self).count + 1u64
          (*self).count
    let _3 := &mut callback
    forward(
      _3,
      function[Fn(&mut «counter::{closure#0}») -> u64 has Copy, Drop](«counter::{closure#0}»::call_mut)
    )

  fun toggle(value : Bool) -> Bool := do
    let mut callback :=
      closure «toggle::{closure#0}» has Copy (state : Bool := value)
        fun call_mut(self : &mut «toggle::{closure#0}») -> Bool := do
          (*self).state := !(*self).state
          (*self).state
    let _3 := &mut callback
    apply_bool(
      _3,
      function[Fn(&mut «toggle::{closure#0}») -> Bool has Copy, Drop](«toggle::{closure#0}»::call_mut)
    )

  @[method_evidence = 1]
  fun apply_unit {F}(
    callback : &mut F,
    __trait_FnMut_call_mut : Fn(&mut F) -> Unit has Copy, Drop
  ) -> Unit := do
    invoke(__trait_FnMut_call_mut, &mut *callback)

  fun unit(value : u64) -> Unit := do
    let mut callback :=
      closure «unit::{closure#0}» has Copy (count : u64 := value)
        fun call_mut(self : &mut «unit::{closure#0}») -> Unit :=
          (*self).count := (*self).count + 1u64
    let _4 := &mut callback
    apply_unit(
      _4,
      function[Fn(&mut «unit::{closure#0}») -> Unit has Copy, Drop](«unit::{closure#0}»::call_mut)
    )
