-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec apply where
  pragma opaque
  ensures ensures_of<__trait_Fn_call>(callback, value, result)

spec forward where
  pragma opaque
  ensures ensures_of<__trait_Fn_call>(callback, value, result)

impl «scalar::{closure#0}» where
  spec call where
    pragma opaque
    ensures result == (self.value + x) % 18446744073709551616

spec scalar where
  ensures result == value

impl «pair::{closure#0}» where
  spec call where
    pragma opaque
    ensures spec.result[0] == (self.value + x) % 18446744073709551616
    ensures spec.result[1]

spec pair where
  ensures !spec.result[1]

spec two where
  pragma opaque
  ensures exists (x : Int), exists (y : Int),
    ensures_of<__trait_Fn_call>(a, x) &&
    ensures_of<__trait_Fn_call_1>(b, y) &&
    result == (x + y) % 18446744073709551616

impl «multiple::{closure#0}» where
  spec call where
    pragma opaque
    ensures result == self.value

impl «multiple::{closure#1}» where
  spec call where
    pragma opaque
    ensures result == 1

spec multiple where
  ensures result == value

spec apply_unit where
  pragma opaque
  ensures ensures_of<__trait_Fn_call>(callback)

impl «panicking::{closure#0}» where
  spec call where
    pragma opaque
    ensures result == self.value + x

spec panicking where
  ensures result == value + 1

-- Incorrect shared implementation contracts must be checked at the body.
impl «wrong_body::{closure#0}» where
  spec call where
    pragma opaque
    ensures result == self.value

spec wrong_body where
  ensures result == value
