-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec apply_mut where
  pragma opaque
  ensures exists (middle : F), exists (first : Int),
    ensures_of<__trait_FnMut_call_mut>(old(callback), first, middle) &&
    ensures_of<__trait_FnMut_call_mut>(middle, result, callback)

spec forward where
  pragma opaque
  ensures exists (middle : F), exists (first : Int),
    ensures_of<__trait_FnMut_call_mut>(old(callback), first, middle) &&
    ensures_of<__trait_FnMut_call_mut>(middle, result, callback)

spec apply_bool where
  pragma opaque
  ensures exists (middle : F), exists (first : Bool),
    ensures_of<__trait_FnMut_call_mut>(old(callback), first, middle) &&
    ensures_of<__trait_FnMut_call_mut>(middle, result, callback)

impl «counter::{closure#0}» where
  spec call_mut where
    pragma opaque
    ensures self.count == (old(self.count) + 1) % 18446744073709551616
    ensures result == self.count

impl «toggle::{closure#0}» where
  spec call_mut where
    pragma opaque
    ensures self.state == !old(self.state)
    ensures result == self.state

-- The caller must account for both state transitions.
spec counter where
  ensures result == value

spec toggle where
  ensures result == !value

-- A potentially panicking closure cannot supply MethodContract.
impl «panicking::{closure#0}» where
  spec call_mut where
    pragma opaque
    ensures self.count == old(self.count) + 1
    ensures result == self.count

spec panicking where
  ensures result == value + 2

-- A captured mutable loan must retain the existing generic-contract refusal.
impl «captured_loan::{closure#0}» where
  spec call_mut where
    pragma opaque
    ensures self.count == (old(self.count) + 1) % 18446744073709551616
    ensures result == self.count

spec captured_loan where
  ensures result == (value + 2) % 18446744073709551616

-- An incorrect implementation contract is rejected at the closure body.
impl «wrong_body::{closure#0}» where
  spec call_mut where
    pragma opaque
    ensures self.count == old(self.count)
    ensures result == self.count

spec wrong_body where
  ensures result == value
