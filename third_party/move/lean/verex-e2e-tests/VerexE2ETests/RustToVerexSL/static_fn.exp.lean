-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace static_fn where
  @[method_evidence = 2]
  fun apply {F} {R}(
    callback : &F, value : u64,
    __trait_Fn_call : Fn(&F, u64) -> R has Copy, Drop
  ) -> R := invoke(__trait_Fn_call, callback, value)

  @[method_evidence = 2]
  fun forward {F} {R}(
    callback : &F, value : u64,
    __trait_Fn_call : Fn(&F, u64) -> R has Copy, Drop
  ) -> R := apply(callback, value, __trait_Fn_call)

  fun scalar(value : u64) -> u64 := do
    let callback :=
      closure «scalar::{closure#0}» has Copy (value : u64 := value)
        fun call(
          self : &«scalar::{closure#0}», x : u64
        ) -> u64 := (*self).value + x
    forward(
      &callback, 1u64,
      function[Fn(&«scalar::{closure#0}», u64) -> u64 has Copy, Drop](«scalar::{closure#0}»::call)
    )

  fun pair(value : u64) -> (u64, Bool) := do
    let callback :=
      closure «pair::{closure#0}» has Copy (value : u64 := value)
        fun call(self : &«pair::{closure#0}», x : u64) -> (u64, Bool) :=
          ((*self).value + x, true)
    forward(
      &callback, 1u64,
      function[Fn(&«pair::{closure#0}», u64) -> (u64, Bool) has Copy, Drop](«pair::{closure#0}»::call)
    )

  @[method_evidence = 2, method_evidence = 3]
  fun two {A} {B}(
    a : &A, b : &B, __trait_Fn_call : Fn(&A) -> u64 has Copy, Drop,
    __trait_Fn_call_1 : Fn(&B) -> u64 has Copy, Drop
  ) -> u64 := do
    let _3 := invoke(__trait_Fn_call, a)
    _3 + invoke(__trait_Fn_call_1, b)

  fun multiple(value : u64) -> u64 := do
    let _3 :=
      closure «multiple::{closure#1}» has Copy ()
        fun call(self : &«multiple::{closure#1}») -> u64 := 1u64
    let a :=
      closure «multiple::{closure#0}» has Copy (value : u64 := value)
        fun call(self : &«multiple::{closure#0}») -> u64 := (*self).value
    two(
      &a, &new «multiple::{closure#1}» {},
      function[Fn(&«multiple::{closure#0}») -> u64 has Copy, Drop](«multiple::{closure#0}»::call),
      function[Fn(&«multiple::{closure#1}») -> u64 has Copy, Drop](«multiple::{closure#1}»::call)
    )

  @[method_evidence = 1]
  fun apply_unit {F}(
    callback : &F, __trait_Fn_call : Fn(&F) -> Unit has Copy, Drop
  ) -> Unit := do
    invoke(__trait_Fn_call, callback)
    invoke(__trait_Fn_call, callback)

  fun unit() -> Unit := do
    closure «unit::{closure#0}» has Copy ()
      fun call(self : &«unit::{closure#0}») -> Unit := ()
    apply_unit(
      &new «unit::{closure#0}» {},
      function[Fn(&«unit::{closure#0}») -> Unit has Copy, Drop](«unit::{closure#0}»::call)
    )
