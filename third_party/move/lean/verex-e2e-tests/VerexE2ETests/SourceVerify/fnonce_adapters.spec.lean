-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec shared_call where
  pragma opaque
  ensures ensures_of<__trait_Fn_call>(callback, result)

spec mutable_call where
  pragma opaque
  ensures ensures_of<__trait_FnMut_call_mut>(old(callback), result, callback)

spec once where
  pragma opaque
  ensures ensures_of<__trait_FnOnce_call_once>(callback, result)

impl «shared::{closure#0}» where
  spec call where
    pragma opaque
    ensures result == self.value

spec shared where
  ensures result == value

impl «mutable::{closure#0}» where
  spec call_mut where
    pragma opaque
    ensures self.state == (old(self.state) + 1) % 18446744073709551616
    ensures result == self.state

spec mutable where
  ensures result == (value + 2) % 18446744073709551616

impl «pair::{closure#0}» where
  spec call_mut where
    pragma opaque
    ensures self.state == (old(self.state) + 1) % 18446744073709551616
    ensures spec.result[0] == self.state
    ensures spec.result[1]

spec pair where
  ensures spec.result[0] == (value + 2) % 18446744073709551616
  ensures spec.result[1]

impl «unit::{closure#0}» where
  spec call_mut where
    pragma opaque
    ensures self.state == (old(self.state) + 1) % 18446744073709551616

spec unit where
  ensures true
