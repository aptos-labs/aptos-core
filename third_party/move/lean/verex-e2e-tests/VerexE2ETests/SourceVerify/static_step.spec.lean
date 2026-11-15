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
