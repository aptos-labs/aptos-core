-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec once where
  pragma opaque
  ensures ensures_of<__trait_FnOnce_call_once>(callback, value, result)

spec shared where
  pragma opaque
  ensures ensures_of<__trait_Fn_call>(callback, value, result)

spec mutable where
  pragma opaque
  ensures exists (final_callback : F), ensures_of<__trait_FnMut_call_mut>(callback, value, result, final_callback)

impl «scalar::{closure#0}» where
  spec call where
    pragma opaque
    ensures result == (self.value + x) % 18446744073709551616

spec scalar where
  ensures result == (value + 1) % 18446744073709551616

impl «counter::{closure#0}» where
  spec call_mut where
    pragma opaque
    ensures self.state == (old(self.state) + x) % 18446744073709551616
    ensures result == self.state

spec counter where
  ensures result == (value + 1) % 18446744073709551616

impl «pair::{closure#0}» where
  spec call_mut where
    pragma opaque
    ensures self.state == (old(self.state) + x) % 18446744073709551616
    ensures spec.result[0] == self.state
    ensures spec.result[1]

spec pair where
  ensures spec.result[0] == (value + 1) % 18446744073709551616
  ensures spec.result[1]

spec forward where
  pragma opaque
  ensures ensures_of<__trait_Fn_call>(callback, value, result)

spec direct where
  pragma opaque
  ensures ensures_of<__trait_Fn_call>(callback, value, result)

spec relay where
  pragma opaque
  ensures ensures_of<__trait_Fn_call>(callback, value, result)
