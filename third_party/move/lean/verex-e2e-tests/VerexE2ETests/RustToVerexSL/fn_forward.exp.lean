-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace fn_forward where
  @[method_evidence = 2]
  fun apply_mut {F} {R}(
    callback : &mut F, x : u64,
    __trait_FnMut_call_mut : Fn(&mut F, u64) -> R has Copy, Drop
  ) -> R := invoke(__trait_FnMut_call_mut, &mut *callback, x)

  @[method_evidence = 2]
  fun forward {F} {R}(
    callback : &mut F, x : u64,
    __trait_Fn_call : Fn(&F, u64) -> R has Copy, Drop
  ) -> R :=
    apply_mut(
      callback, x,
      function[Fn(&mut F, u64) -> R has Copy, Drop](__fn_to_fnmut_forward_0::<F, R>, _,
      _,
      __trait_Fn_call)
    )

  @[method_evidence = 2, shared_method_adapter = 2]
  fun __fn_to_fnmut_forward_0 {F} {R}(
    receiver : &mut F, arg_1 : u64,
    __trait_Fn_call : Fn(&F, u64) -> R has Copy, Drop
  ) -> R := invoke(__trait_Fn_call, &*receiver, arg_1)

  @[method_evidence = 2]
  fun twice {F}(
    callback : &mut F, x : u64,
    __trait_Fn_call : Fn(&F, u64) -> u64 has Copy, Drop
  ) -> u64 := do
    let first := forward(callback, x, __trait_Fn_call)
    apply_mut(
      callback, first,
      function[Fn(&mut F, u64) -> u64 has Copy, Drop](__fn_to_fnmut_twice_0::<F>, _,
      _,
      __trait_Fn_call)
    )

  @[method_evidence = 2, shared_method_adapter = 2]
  fun __fn_to_fnmut_twice_0 {F}(
    receiver : &mut F, arg_1 : u64,
    __trait_Fn_call : Fn(&F, u64) -> u64 has Copy, Drop
  ) -> u64 := invoke(__trait_Fn_call, &*receiver, arg_1)

  fun scalar(value : u64) -> u64 := do
    let mut callback :=
      closure «scalar::{closure#0}» has Copy (value : u64 := value)
        fun call(
          self : &«scalar::{closure#0}», x : u64
        ) -> u64 := (*self).value + x
    let _3 := &mut callback
    twice(
      _3, 1u64,
      function[Fn(&«scalar::{closure#0}», u64) -> u64 has Copy, Drop](«scalar::{closure#0}»::call)
    )

  fun pair(value : u64) -> (u64, Bool) := do
    let mut callback :=
      closure «pair::{closure#0}» has Copy (value : u64 := value)
        fun call(self : &«pair::{closure#0}», x : u64) -> (u64, Bool) :=
          ((*self).value + x, true)
    let _3 := &mut callback
    forward(
      _3, 1u64,
      function[Fn(&«pair::{closure#0}», u64) -> (u64, Bool) has Copy, Drop](«pair::{closure#0}»::call)
    )

  @[method_evidence = 3, method_evidence = 4]
  fun both {A} {B}(
    a : &mut A, b : &mut B, x : u64,
    __trait_Fn_call : Fn(&A, u64) -> u64 has Copy, Drop,
    __trait_Fn_call_1 : Fn(&B, u64) -> u64 has Copy, Drop
  ) -> u64 := do
    let _4 :=
      apply_mut(
        a, x,
        function[Fn(&mut A, u64) -> u64 has Copy, Drop](__fn_to_fnmut_both_0::<A, B>, _,
        _,
        __trait_Fn_call)
      )
    _4
      + apply_mut(
        b, x,
        function[Fn(&mut B, u64) -> u64 has Copy, Drop](__fn_to_fnmut_both_1::<A, B>, _,
        _,
        __trait_Fn_call_1)
      )

  @[method_evidence = 2, shared_method_adapter = 2]
  fun __fn_to_fnmut_both_0 {A} {B}(
    receiver : &mut A, arg_1 : u64,
    __trait_Fn_call : Fn(&A, u64) -> u64 has Copy, Drop
  ) -> u64 := invoke(__trait_Fn_call, &*receiver, arg_1)

  @[method_evidence = 2, shared_method_adapter = 2]
  fun __fn_to_fnmut_both_1 {A} {B}(
    receiver : &mut B, arg_1 : u64,
    __trait_Fn_call_1 : Fn(&B, u64) -> u64 has Copy, Drop
  ) -> u64 := invoke(__trait_Fn_call_1, &*receiver, arg_1)

  fun multiple(value : u64) -> u64 := do
    let mut _3 :=
      closure «multiple::{closure#1}» has Copy ()
        fun call(self : &«multiple::{closure#1}», x : u64) -> u64 := x
    let mut a :=
      closure «multiple::{closure#0}» has Copy (value : u64 := value)
        fun call(
          self : &«multiple::{closure#0}», x : u64
        ) -> u64 := (*self).value + x
    let _4 := &mut a
    let _5 := &mut _3
    both(
      _4, _5, 1u64,
      function[Fn(&«multiple::{closure#0}», u64) -> u64 has Copy, Drop](«multiple::{closure#0}»::call),
      function[Fn(&«multiple::{closure#1}», u64) -> u64 has Copy, Drop](«multiple::{closure#1}»::call)
    )

  @[method_evidence = 1]
  fun unit_mut {F}(
    callback : &mut F,
    __trait_FnMut_call_mut : Fn(&mut F) -> Unit has Copy, Drop
  ) -> Unit := do
    invoke(__trait_FnMut_call_mut, &mut *callback)

  @[method_evidence = 1]
  fun unit_forward {F}(
    callback : &mut F, __trait_Fn_call : Fn(&F) -> Unit has Copy, Drop
  ) -> Unit := do
    unit_mut(
      callback,
      function[Fn(&mut F) -> Unit has Copy, Drop](__fn_to_fnmut_unit_forward_0::<F>, _,
      __trait_Fn_call)
    )
    unit_mut(
      callback,
      function[Fn(&mut F) -> Unit has Copy, Drop](__fn_to_fnmut_unit_forward_0::<F>, _,
      __trait_Fn_call)
    )

  @[method_evidence = 1, shared_method_adapter = 1]
  fun __fn_to_fnmut_unit_forward_0 {F}(
    receiver : &mut F, __trait_Fn_call : Fn(&F) -> Unit has Copy, Drop
  ) -> Unit := do
    invoke(__trait_Fn_call, &*receiver)

  fun unit() -> Unit := do
    let _2 :=
      &mut closure «unit::{closure#0}» has Copy ()
        fun call(self : &«unit::{closure#0}») -> Unit := ()
    unit_forward(
      _2,
      function[Fn(&«unit::{closure#0}») -> Unit has Copy, Drop](«unit::{closure#0}»::call)
    )

  @[method_evidence = 1]
  fun tuple_mut {F}(
    callback : &mut F,
    __trait_FnMut_call_mut : Fn(&mut F) -> (u64, Bool) has Copy, Drop
  ) -> (u64, Bool) := invoke(__trait_FnMut_call_mut, &mut *callback)

  @[method_evidence = 1]
  fun tuple_forward {F}(
    callback : &mut F,
    __trait_Fn_call : Fn(&F) -> (u64, Bool) has Copy, Drop
  ) -> (u64, Bool) :=
    tuple_mut(
      callback,
      function[Fn(&mut F) -> (u64, Bool) has Copy, Drop](__fn_to_fnmut_tuple_forward_0::<F>, _,
      __trait_Fn_call)
    )

  @[method_evidence = 1, shared_method_adapter = 1]
  fun __fn_to_fnmut_tuple_forward_0 {F}(
    receiver : &mut F,
    __trait_Fn_call : Fn(&F) -> (u64, Bool) has Copy, Drop
  ) -> (u64, Bool) := invoke(__trait_Fn_call, &*receiver)

  fun tuple_value(value : u64) -> (u64, Bool) := do
    let _2 :=
      &mut closure «tuple_value::{closure#0}» has Copy (value : u64 := value)
        fun call(self : &«tuple_value::{closure#0}») -> (u64, Bool) :=
          ((*self).value, true)
    tuple_forward(
      _2,
      function[Fn(&«tuple_value::{closure#0}») -> (u64, Bool) has Copy, Drop](«tuple_value::{closure#0}»::call)
    )
