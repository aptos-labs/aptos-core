-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec apply_mut where
  pragma opaque
  ensures ensures_of<__trait_FnMut_call_mut>(old(callback), x, result, callback)

spec forward where
  pragma opaque
  ensures callback == old(callback)
  ensures ensures_of<__trait_Fn_call>(old(callback), x, result)

spec twice where
  pragma opaque
  ensures callback == old(callback)
  ensures exists (first : Int),
    ensures_of<__trait_Fn_call>(old(callback), x, first) &&
    ensures_of<__trait_Fn_call>(old(callback), first, result)

impl «scalar::{closure#0}» where
  spec call where
    pragma opaque
    ensures result == (self.value + x) % 18446744073709551616

spec scalar where
  ensures result == (value + value + 1) % 18446744073709551616

impl «pair::{closure#0}» where
  spec call where
    pragma opaque
    ensures spec.result[0] == (self.value + x) % 18446744073709551616
    ensures spec.result[1]

spec pair where
  ensures spec.result[0] == (value + 1) % 18446744073709551616
  ensures spec.result[1]
