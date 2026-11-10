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

spec counter where
  ensures result == (value + 2) % 18446744073709551616

spec toggle where
  ensures result == value

spec apply_unit where
  pragma opaque
  ensures ensures_of<__trait_FnMut_call_mut>(old(callback), callback)

impl «unit::{closure#0}» where
  spec call_mut where
    pragma opaque
    ensures self.count == (old(self.count) + 1) % 18446744073709551616

spec unit where
  ensures true
