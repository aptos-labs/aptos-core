-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec apply_mut where
  pragma opaque
  ensures ensures_of<__trait_FnMut_call_mut>(old(callback), x, result, callback)

spec both where
  pragma opaque
  ensures a == old(a)
  ensures b == old(b)
  ensures exists (first : Int), exists (second : Int),
    ensures_of<__trait_Fn_call>(old(a), x, first) &&
    ensures_of<__trait_Fn_call_1>(old(b), x, second) &&
    result == (first + second) % 18446744073709551616

impl «multiple::{closure#0}» where
  spec call where
    pragma opaque
    ensures result == (self.value + x) % 18446744073709551616

impl «multiple::{closure#1}» where
  spec call where
    pragma opaque
    ensures result == x

spec multiple where
  ensures result == (value + 2) % 18446744073709551616
