-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace tuple_callbacks where
  @[method_evidence = 1]
  fun apply {F}(
    callback : &mut F,
    __trait_FnMut_call_mut : Fn(&mut F) -> (u64, Bool) has Copy, Drop
  ) -> (u64, Bool) := invoke(__trait_FnMut_call_mut, &mut *callback)

  @[method_evidence = 1]
  fun forward {F}(
    callback : &mut F,
    __trait_FnMut_call_mut : Fn(&mut F) -> (u64, Bool) has Copy, Drop
  ) -> (u64, Bool) := apply(callback, __trait_FnMut_call_mut)

  fun pair(value : u64) -> (u64, Bool) := do
    let mut callback :=
      closure «pair::{closure#0}» has Copy (n : u64 := value)
        fun call_mut(self : &mut «pair::{closure#0}») -> (u64, Bool) := do
          (*self).n := (*self).n + 1u64
          ((*self).n, true)
    let _3 := &mut callback
    forward(
      _3,
      function[Fn(&mut «pair::{closure#0}») -> (u64, Bool) has Copy, Drop](«pair::{closure#0}»::call_mut)
    )

  @[method_evidence = 1]
  fun apply_generic {F} {R}(
    callback : &mut F,
    __trait_FnMut_call_mut : Fn(&mut F) -> R has Copy, Drop
  ) -> R := invoke(__trait_FnMut_call_mut, &mut *callback)

  fun generic_pair(value : u64) -> (u64, Bool) := do
    let mut callback :=
      closure «generic_pair::{closure#0}» has Copy (n : u64 := value)
        fun call_mut(
          self : &mut «generic_pair::{closure#0}»
        ) -> (u64, Bool) := do
          (*self).n := (*self).n + 1u64
          ((*self).n, true)
    let _3 := &mut callback
    apply_generic(
      _3,
      function[Fn(&mut «generic_pair::{closure#0}») -> (u64, Bool) has Copy, Drop](«generic_pair::{closure#0}»::call_mut)
    )

  fun singleton(value : u64) -> (u64) := do
    let mut callback :=
      closure «singleton::{closure#0}» has Copy (value : u64 := value)
        fun call(self : &«singleton::{closure#0}») -> (u64) := ((*self).value,)
    let _3 := &mut callback
    apply_generic(
      _3,
      function[Fn(&mut «singleton::{closure#0}») -> (u64) has Copy, Drop](«__fnmut_adapter_singleton::{closure#0}»)
    )

  @[closure_method_adapter = "singleton::{closure#0}::call"]
  fun «__fnmut_adapter_singleton::{closure#0}»(
    self : &mut «singleton::{closure#0}»
  ) -> (u64) := «singleton::{closure#0}»::call(&*self)

  @[method_evidence = 1]
  fun apply_nested {F}(
    callback : &mut F,
    __trait_FnMut_call_mut : Fn(&mut F) -> (u64, (Bool, u64)) has Copy, Drop
  ) -> (u64, (Bool, u64)) := invoke(__trait_FnMut_call_mut, &mut *callback)

  fun nested_tuple(value : u64) -> (u64, (Bool, u64)) := do
    let mut callback :=
      closure «nested_tuple::{closure#0}» has Copy (value : u64 := value)
        fun call(self : &«nested_tuple::{closure#0}») -> (u64, (Bool, u64)) :=
          ((*self).value, (true, (*self).value + 1u64))
    let _3 := &mut callback
    apply_nested(
      _3,
      function[Fn(&mut «nested_tuple::{closure#0}») -> (u64, (Bool, u64)) has Copy, Drop](«__fnmut_adapter_nested_tuple::{closure#0}»)
    )

  @[closure_method_adapter = "nested_tuple::{closure#0}::call"]
  fun «__fnmut_adapter_nested_tuple::{closure#0}»(
    self : &mut «nested_tuple::{closure#0}»
  ) -> (u64, (Bool, u64)) := «nested_tuple::{closure#0}»::call(&*self)
