-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace static_step where
  @[method_evidence = 1]
  fun step_twice {T}(
    value : &mut T, __trait_Step_step : Fn(&mut T) -> Unit has Copy, Drop
  ) -> Unit := do
    invoke(__trait_Step_step, &mut *value)
    invoke(__trait_Step_step, &mut *value)

  @[method_evidence = 1]
  fun forward {T}(
    value : &mut T, __trait_Step_step : Fn(&mut T) -> Unit has Copy, Drop
  ) -> Unit := do
    step_twice(value, __trait_Step_step)

  struct Counter where
    value : u64

  trait impl «<static_step::Counter as static_step::Step>» where
    fun step(
      self : &mut Counter
    ) -> Unit := (*self).value := (*self).value + 1u64

  struct Toggle where
    value : Bool

  trait impl «<static_step::Toggle as static_step::Step>» where
    fun step(self : &mut Toggle) -> Unit := (*self).value := !(*self).value

  fun counter(value : u64) -> u64 := do
    let mut state := new Counter { value }
    let _4 := &mut state
    step_twice(
      _4,
      function[Fn(&mut Counter) -> Unit has Copy, Drop](«<static_step::Counter as static_step::Step>»::step)
    )
    state.value

  fun toggle(value : Bool) -> Bool := do
    let mut state := new Toggle { value }
    let _4 := &mut state
    forward(
      _4,
      function[Fn(&mut Toggle) -> Unit has Copy, Drop](«<static_step::Toggle as static_step::Step>»::step)
    )
    state.value
