-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace fnonce_adapters where
  @[method_evidence = 1]
  fun shared_call {F} {R}(
    callback : &F, __trait_Fn_call : Fn(&F) -> R has Copy, Drop
  ) -> R := invoke(__trait_Fn_call, callback)

  @[method_evidence = 1]
  fun mutable_call {F} {R}(
    callback : &mut F,
    __trait_FnMut_call_mut : Fn(&mut F) -> R has Copy, Drop
  ) -> R := invoke(__trait_FnMut_call_mut, &mut *callback)

  @[method_evidence = 1]
  fun once {F} {R}(
    callback : F, __trait_FnOnce_call_once : Fn(F) -> R has Copy, Drop
  ) -> R := invoke(__trait_FnOnce_call_once, callback)

  fun shared(value : u64) -> u64 := do
    let callback :=
      closure «shared::{closure#0}» has Copy (value : u64 := value)
        fun call(self : &«shared::{closure#0}») -> u64 := (*self).value
    let _3 :=
      shared_call(
        &callback,
        function[Fn(&«shared::{closure#0}») -> u64 has Copy, Drop](«shared::{closure#0}»::call)
      )
    once(
      callback,
      function[Fn(«shared::{closure#0}») -> u64 has Copy, Drop](«__fnonce_adapter_shared::{closure#0}»)
    )

  @[closure_method_adapter = "shared::{closure#0}::call"]
  fun «__fnonce_adapter_shared::{closure#0}»(
    self : «shared::{closure#0}»
  ) -> u64 := do
    let result := «shared::{closure#0}»::call(&self)
    drop(self)
    result

  fun mutable(value : u64) -> u64 := do
    let mut callback :=
      closure «mutable::{closure#0}» has Copy (state : u64 := value)
        fun call_mut(self : &mut «mutable::{closure#0}») -> u64 := do
          (*self).state := (*self).state + 1u64
          (*self).state
    let _4 := &mut callback
    let _3 :=
      mutable_call(
        _4,
        function[Fn(&mut «mutable::{closure#0}») -> u64 has Copy, Drop](«mutable::{closure#0}»::call_mut)
      )
    once(
      callback,
      function[Fn(«mutable::{closure#0}») -> u64 has Copy, Drop](«__fnonce_adapter_mutable::{closure#0}»)
    )

  @[closure_method_adapter = "mutable::{closure#0}::call_mut"]
  fun «__fnonce_adapter_mutable::{closure#0}»(
    mut self : «mutable::{closure#0}»
  ) -> u64 := do
    let result := «mutable::{closure#0}»::call_mut(&mut self)
    drop(self)
    result

  fun pair(value : u64) -> (u64, Bool) := do
    let mut callback :=
      closure «pair::{closure#0}» has Copy (state : u64 := value)
        fun call_mut(self : &mut «pair::{closure#0}») -> (u64, Bool) := do
          (*self).state := (*self).state + 1u64
          ((*self).state, true)
    let _4 := &mut callback
    let _3 :=
      mutable_call(
        _4,
        function[Fn(&mut «pair::{closure#0}») -> (u64, Bool) has Copy, Drop](«pair::{closure#0}»::call_mut)
      )
    once(
      callback,
      function[Fn(«pair::{closure#0}») -> (u64, Bool) has Copy, Drop](«__fnonce_adapter_pair::{closure#0}»)
    )

  @[closure_method_adapter = "pair::{closure#0}::call_mut"]
  fun «__fnonce_adapter_pair::{closure#0}»(
    mut self : «pair::{closure#0}»
  ) -> (u64, Bool) := do
    let result := «pair::{closure#0}»::call_mut(&mut self)
    drop(self)
    result

  fun unit(value : u64) -> Unit := do
    let mut callback :=
      closure «unit::{closure#0}» has Copy (state : u64 := value)
        fun call_mut(self : &mut «unit::{closure#0}») -> Unit :=
          (*self).state := (*self).state + 1u64
    let _4 := &mut callback
    mutable_call(
      _4,
      function[Fn(&mut «unit::{closure#0}») -> Unit has Copy, Drop](«unit::{closure#0}»::call_mut)
    )
    once(
      callback,
      function[Fn(«unit::{closure#0}») -> Unit has Copy, Drop](«__fnonce_adapter_unit::{closure#0}»)
    )

  @[closure_method_adapter = "unit::{closure#0}::call_mut"]
  fun «__fnonce_adapter_unit::{closure#0}»(
    mut self : «unit::{closure#0}»
  ) -> Unit := do
    let result := «unit::{closure#0}»::call_mut(&mut self)
    drop(self)
