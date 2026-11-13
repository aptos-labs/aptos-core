-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace fnonce_forward where
  @[method_evidence = 2]
  fun once {F} {R}(
    callback : F, value : u64,
    __trait_FnOnce_call_once : Fn(F, u64) -> R has Copy, Drop
  ) -> R := invoke(__trait_FnOnce_call_once, callback, value)

  @[method_evidence = 2, scoped_method_evidence = 2]
  fun shared {F} {R}(
    callback : F, value : u64,
    __trait_Fn_call : Fn(&F, u64) -> R has Copy, Drop
  ) -> R :=
    once(
      callback, value,
      function[Fn(F, u64) -> R has Copy, Drop](__borrowed_to_fnonce_shared_0::<F, R>, _,
      _,
      __trait_Fn_call)
    )

  @[method_evidence = 2, scoped_method_evidence = 2, consuming_method_adapter = 2]
  fun __borrowed_to_fnonce_shared_0 {F} {R}(
    receiver : F, arg_1 : u64,
    __trait_Fn_call : Fn(&F, u64) -> R has Copy, Drop
  ) -> R := do
    let result := invoke(__trait_Fn_call, &receiver, arg_1)
    drop(receiver)
    result

  @[method_evidence = 2, scoped_method_evidence = 2]
  fun mutable {F} {R}(
    callback : F, value : u64,
    __trait_FnMut_call_mut : Fn(&mut F, u64) -> R has Copy, Drop
  ) -> R :=
    once(
      callback, value,
      function[Fn(F, u64) -> R has Copy, Drop](__borrowed_to_fnonce_mutable_0::<F, R>, _,
      _,
      __trait_FnMut_call_mut)
    )

  @[method_evidence = 2, scoped_method_evidence = 2, consuming_method_adapter = 2]
  fun __borrowed_to_fnonce_mutable_0 {F} {R}(
    mut receiver : F, arg_1 : u64,
    __trait_FnMut_call_mut : Fn(&mut F, u64) -> R has Copy, Drop
  ) -> R := do
    let result := invoke(__trait_FnMut_call_mut, &mut receiver, arg_1)
    drop(receiver)
    result

  -- Two forwarding hops require the original Fn dictionary to be scoped.
  @[method_evidence = 2, scoped_method_evidence = 2]
  fun forward {F} {R}(
    callback : F, value : u64,
    __trait_Fn_call : Fn(&F, u64) -> R has Copy, Drop
  ) -> R :=
    mutable(
      callback, value,
      function[Fn(&mut F, u64) -> R has Copy, Drop](__fn_to_fnmut_forward_0::<F, R>, _,
      _,
      __trait_Fn_call)
    )

  @[method_evidence = 2, scoped_method_evidence = 2, shared_method_adapter = 2]
  fun __fn_to_fnmut_forward_0 {F} {R}(
    receiver : &mut F, arg_1 : u64,
    __trait_Fn_call : Fn(&F, u64) -> R has Copy, Drop
  ) -> R := invoke(__trait_Fn_call, &*receiver, arg_1)

  @[method_evidence = 2, scoped_method_evidence = 2]
  fun relay {F} {R}(
    callback : F, value : u64,
    __trait_Fn_call : Fn(&F, u64) -> R has Copy, Drop
  ) -> R := forward(callback, value, __trait_Fn_call)

  -- A borrowed dictionary not used by consuming adapters remains unrestricted.
  @[method_evidence = 2]
  fun direct {F} {R}(
    callback : &F, value : u64,
    __trait_Fn_call : Fn(&F, u64) -> R has Copy, Drop
  ) -> R := invoke(__trait_Fn_call, callback, value)

  fun scalar(value : u64) -> u64 :=
    relay(
      closure «scalar::{closure#0}» has Copy (value : u64 := value)
        fun call(
          self : &«scalar::{closure#0}», x : u64
        ) -> u64 := (*self).value + x,
      1u64,
      function[Fn(&«scalar::{closure#0}», u64) -> u64 has Copy, Drop](«scalar::{closure#0}»::call)
    )

  fun counter(value : u64) -> u64 :=
    mutable(
      closure «counter::{closure#0}» has Copy (state : u64 := value)
        fun call_mut(self : &mut «counter::{closure#0}», x : u64) -> u64 := do
          (*self).state := (*self).state + x
          (*self).state,
      1u64,
      function[Fn(&mut «counter::{closure#0}», u64) -> u64 has Copy, Drop](«counter::{closure#0}»::call_mut)
    )

  fun pair(value : u64) -> (u64, Bool) :=
    mutable(
      closure «pair::{closure#0}» has Copy (state : u64 := value)
        fun call_mut(
          self : &mut «pair::{closure#0}», x : u64
        ) -> (u64, Bool) := do
          (*self).state := (*self).state + x
          ((*self).state, true),
      1u64,
      function[Fn(&mut «pair::{closure#0}», u64) -> (u64, Bool) has Copy, Drop](«pair::{closure#0}»::call_mut)
    )
