-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.Rendered.Support

/-!
# `static_step` as a frontend renders it

The namespaces below are the rendering of
`VerexE2ETests/SourceVerify/static_step.rs`, the items of its
`static_step.spec.lean` and the contracts its calls reach spliced in, which
the verification of the source renders and elaborates; its `verex sources at`
command is dropped and the diagnostics are those of `static_step.exp`. Neither
is edited here: `scripts/prover-smoke-renderings.sh --write` writes this file
again, and without `--write` tells whether it is what the frontend renders.
The crate's own namespace, which the diagnostics are of, is the last command;
the namespaces before it are what it uses, and report nothing.
-/

#guard_rendering in
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

  -- Copyright © Aptos Foundation
  -- SPDX-License-Identifier: Apache-2.0

  trait impl «<static_step::Counter as static_step::Step>» where
    spec step where
      pragma opaque
      ensures self.value == (old(self.value) + 1) % 18446744073709551616

  trait impl «<static_step::Toggle as static_step::Step>» where
    spec step where
      pragma opaque
      ensures self.value == !old(self.value)

  spec step_twice where
    pragma opaque
    ensures exists (middle : T),
      ensures_of<__trait_Step_step>(old(value), middle) &&
      ensures_of<__trait_Step_step>(middle, value)

  spec forward where
    pragma opaque
    ensures exists (middle : T),
      ensures_of<__trait_Step_step>(old(value), middle) &&
      ensures_of<__trait_Step_step>(middle, value)

  spec counter where
    ensures result == (value + 2) % 18446744073709551616

  spec toggle where
    ensures result == value
