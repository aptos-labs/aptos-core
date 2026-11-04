-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

trait impl «<static_step_errors::Counter as static_step_errors::Step>» where
  spec step where
    pragma opaque
    requires self.value < 100
    ensures self.value == old(self.value) + 1

trait impl «<static_step_errors::Toggle as static_step_errors::Step>» where
  spec step where
    pragma opaque
    ensures self.value == old(self.value) -- false implementation contract

spec step_twice where
  pragma opaque
  ensures exists (middle : T),
    ensures_of<__trait_Step_step>(old(value), middle) &&
    ensures_of<__trait_Step_step>(middle, value)

spec forward where
  pragma opaque
  ensures value == old(value) -- a dictionary may change the receiver twice

spec counter where
  -- Its method contract requires a bound, so it cannot supply the dictionary's
  -- unconditional no-panic contract for arbitrary receivers.
  ensures result == (value + 2) % 18446744073709551616

spec toggle where
  ensures result == !value -- two steps restore the original value
