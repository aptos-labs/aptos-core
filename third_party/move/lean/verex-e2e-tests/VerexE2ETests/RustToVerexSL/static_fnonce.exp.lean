-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace static_fnonce where
  struct Token where
    value : u64

  fun take(token : Token) -> u64 := token.value

  @[method_evidence = 2]
  fun apply_once {F} {A} {R}(
    callback : F, argument : A,
    __trait_FnOnce_call_once : Fn(F, A) -> R has Copy, Drop
  ) -> R := invoke(__trait_FnOnce_call_once, callback, argument)

  @[method_evidence = 2]
  fun forward {F} {A} {R}(
    callback : F, argument : A,
    __trait_FnOnce_call_once : Fn(F, A) -> R has Copy, Drop
  ) -> R := apply_once(callback, argument, __trait_FnOnce_call_once)

  fun scalar(value : u64) -> u64 := do
    let token := new Token { value }
    forward(
      closure «scalar::{closure#0}» (token : Token := token)
        fun call_once(self : «scalar::{closure#0}», x : u64) -> u64 :=
          take(move(self.token)) + x,
      1u64,
      function[Fn(«scalar::{closure#0}», u64) -> u64 has Copy, Drop](«scalar::{closure#0}»::call_once)
    )

  fun pair(value : u64) -> (u64, Bool) := do
    let token := new Token { value }
    forward(
      closure «pair::{closure#0}» (token : Token := token)
        fun call_once(self : «pair::{closure#0}», x : u64) -> (u64, Bool) :=
          (take(move(self.token)) + x, true),
      1u64,
      function[Fn(«pair::{closure#0}», u64) -> (u64, Bool) has Copy, Drop](«pair::{closure#0}»::call_once)
    )

  @[method_evidence = 1]
  fun apply_unit {F}(
    callback : F, __trait_FnOnce_call_once : Fn(F) -> Unit has Copy, Drop
  ) -> Unit := do
    invoke(__trait_FnOnce_call_once, callback)

  fun unit(value : u64) -> Unit := do
    let token := new Token { value }
    apply_unit(
      closure «unit::{closure#0}» (token : Token := token)
        fun call_once(self : «unit::{closure#0}») -> Unit := do
          let _2 := take(move(self.token)),
      function[Fn(«unit::{closure#0}») -> Unit has Copy, Drop](«unit::{closure#0}»::call_once)
    )
