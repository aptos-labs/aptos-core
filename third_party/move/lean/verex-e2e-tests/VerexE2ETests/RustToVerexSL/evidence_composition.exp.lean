-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace evidence_composition where
  @[method_evidence = 1]
  fun apply {F} {R}(
    callback : &mut F,
    __trait_FnMut_call_mut : Fn(&mut F) -> R has Copy, Drop
  ) -> R := invoke(__trait_FnMut_call_mut, &mut *callback)

  @[method_evidence = 2]
  fun apply_arg {F} {T} {R}(
    callback : &mut F, value : T,
    __trait_FnMut_call_mut : Fn(&mut F, T) -> R has Copy, Drop
  ) -> R := invoke(__trait_FnMut_call_mut, &mut *callback, value)

  @[method_evidence = 2, method_evidence = 3]
  fun two {A} {B}(
    a : &mut A, b : &mut B,
    __trait_FnMut_call_mut : Fn(&mut A) -> u64 has Copy, Drop,
    __trait_FnMut_call_mut_1 : Fn(&mut B) -> u64 has Copy, Drop
  ) -> u64 := do
    let _3 := invoke(__trait_FnMut_call_mut, &mut *a)
    _3 + invoke(__trait_FnMut_call_mut_1, &mut *b)

  @[method_evidence = 1]
  fun collision {F}(
    __trait_FnMut_call_mut : &mut F,
    __trait_FnMut_call_mut_1 : Fn(&mut F) -> u64 has Copy, Drop
  ) -> u64 := invoke(__trait_FnMut_call_mut_1, &mut *__trait_FnMut_call_mut)

  @[method_evidence = 1]
  fun outer {G}(
    g : &mut G, __trait_FnMut_call_mut : Fn(&mut G) -> u64 has Copy, Drop
  ) -> u64 := do
    let n := invoke(__trait_FnMut_call_mut, &mut *g)
    let mut c :=
      closure «outer::{closure#0}» {G} has Copy (n : u64 := n)
        @[method_evidence = 1]
        fun call_mut {G}(
          self : &mut «outer::{closure#0}»<G>,
          __trait_FnMut_call_mut : Fn(&mut G) -> u64 has Copy, Drop
        ) -> u64 := do
          (*self).n := (*self).n + 1u64
          (*self).n
    let _4 := &mut c
    apply(
      _4,
      function[Fn(&mut «outer::{closure#0}»<G>) -> u64 has Copy, Drop](«outer::{closure#0}»::call_mut::<G>, _,
      __trait_FnMut_call_mut)
    )

  fun nested(value : u64) -> u64 := do
    let mut c :=
      closure «nested::{closure#0}» has Copy (n : u64 := value)
        fun call_mut(self : &mut «nested::{closure#0}») -> u64 := do
          (*self).n := (*self).n + 1u64
          (*self).n
    let _3 := &mut c
    outer(
      _3,
      function[Fn(&mut «nested::{closure#0}») -> u64 has Copy, Drop](«nested::{closure#0}»::call_mut)
    )

  fun shared(value : u64) -> u64 := do
    let mut c :=
      closure «shared::{closure#0}» {u0 : lifetime} has Copy (
        value : &[u0] u64 := &value
      )
        fun call(self : &«shared::{closure#0}»<lifetime _>) -> u64 := do
          let _2 := copy((*self).value)
          *_2
    let _4 := &mut c
    apply(
      _4,
      function[Fn(&mut «shared::{closure#0}»<lifetime _>) -> u64 has Copy, Drop](«__fnmut_adapter_shared::{closure#0}»)
    )

  @[closure_method_adapter = "shared::{closure#0}::call"]
  fun «__fnmut_adapter_shared::{closure#0}»(
    self : &mut «shared::{closure#0}»<lifetime _>
  ) -> u64 := «shared::{closure#0}»::call(&*self)

  fun argument(value : u64) -> u64 := do
    let mut c :=
      closure «argument::{closure#0}» has Copy (n : u64 := 0u64)
        fun call_mut(self : &mut «argument::{closure#0}», x : u64) -> u64 := do
          (*self).n := (*self).n + x
          (*self).n
    let _3 := &mut c
    apply_arg(
      _3, value,
      function[Fn(&mut «argument::{closure#0}», u64) -> u64 has Copy, Drop](«argument::{closure#0}»::call_mut)
    )

  fun pair(value : u64) -> (u64, Bool) := do
    let mut c :=
      closure «pair::{closure#0}» has Copy (n : u64 := value)
        fun call_mut(self : &mut «pair::{closure#0}») -> (u64, Bool) := do
          (*self).n := (*self).n + 1u64
          ((*self).n, true)
    let _3 := &mut c
    apply(
      _3,
      function[Fn(&mut «pair::{closure#0}») -> (u64, Bool) has Copy, Drop](«pair::{closure#0}»::call_mut)
    )

  fun multiple(value : u64) -> u64 := do
    let _2 :=
      &mut closure «multiple::{closure#0}» has Copy (a : u64 := value)
        fun call_mut(self : &mut «multiple::{closure#0}») -> u64 := do
          (*self).a := (*self).a + 1u64
          (*self).a
    let _4 :=
      &mut closure «multiple::{closure#1}» has Copy (b : u64 := 0u64)
        fun call_mut(self : &mut «multiple::{closure#1}») -> u64 := do
          (*self).b := (*self).b + 1u64
          (*self).b
    two(
      _2, _4,
      function[Fn(&mut «multiple::{closure#0}») -> u64 has Copy, Drop](«multiple::{closure#0}»::call_mut),
      function[Fn(&mut «multiple::{closure#1}») -> u64 has Copy, Drop](«multiple::{closure#1}»::call_mut)
    )

  @[method_evidence = 2, method_evidence = 3]
  fun two_steps {A} {B}(
    a : &mut A, b : &mut B,
    __trait_Step_step : Fn(&mut A) -> Unit has Copy, Drop,
    __trait_Step_step_1 : Fn(&mut B) -> Unit has Copy, Drop
  ) -> Unit := do
    invoke(__trait_Step_step, &mut *a)
    invoke(__trait_Step_step_1, &mut *b)

  struct Pair where
    number : u64
    flag : Bool

  fun record(value : u64) -> Pair := do
    let mut c :=
      closure «record::{closure#0}» has Copy (n : u64 := value)
        fun call_mut(self : &mut «record::{closure#0}») -> Pair := do
          (*self).n := (*self).n + 1u64
          new Pair { number := (*self).n, flag := true }
    let _3 := &mut c
    apply(
      _3,
      function[Fn(&mut «record::{closure#0}») -> Pair has Copy, Drop](«record::{closure#0}»::call_mut)
    )

  @[method_evidence = 3]
  fun apply_two {F}(
    callback : &mut F, value : u64, enabled : Bool,
    __trait_FnMut_call_mut : Fn(&mut F, u64, Bool) -> u64 has Copy, Drop
  ) -> u64 := invoke(__trait_FnMut_call_mut, &mut *callback, value, enabled)

  fun arguments(value : u64) -> u64 := do
    let mut c :=
      closure «arguments::{closure#0}» has Copy (n : u64 := 0u64)
        fun call_mut(
          self : &mut «arguments::{closure#0}», x : u64, enabled : Bool
        ) -> u64 := do
          if enabled then (*self).n := (*self).n + x
          (*self).n
    let _3 := &mut c
    apply_two(
      _3, value, true,
      function[Fn(&mut «arguments::{closure#0}», u64, Bool) -> u64 has Copy, Drop](«arguments::{closure#0}»::call_mut)
    )
