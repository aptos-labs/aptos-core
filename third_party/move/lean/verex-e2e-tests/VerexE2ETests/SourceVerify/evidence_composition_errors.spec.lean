-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec apply where
  pragma opaque
  ensures ensures_of<__trait_FnMut_call_mut>(old(callback), result, callback)

spec apply_arg where
  pragma opaque
  ensures ensures_of<__trait_FnMut_call_mut>(old(callback), value, result, callback)

spec two where
  ensures exists (x : Int), exists (y : Int),
    ensures_of<__trait_FnMut_call_mut>(old(a), x, a) &&
    ensures_of<__trait_FnMut_call_mut_1>(old(b), y, b) &&
    result == (x + y) % 18446744073709551616

spec collision where
  ensures ensures_of<__trait_FnMut_call_mut_1>(old(__trait_FnMut_call_mut), result, __trait_FnMut_call_mut)

spec outer where
  pragma opaque
  ensures exists (x : Int), ensures_of<__trait_FnMut_call_mut>(old(g), x, g) &&
    result == (x + 1) % 18446744073709551616

impl «outer::{closure#0}» where
  spec call_mut where
    pragma opaque
    ensures self.n == (old(self.n) + 1) % 18446744073709551616
    ensures result == self.n

impl «nested::{closure#0}» where
  spec call_mut where
    pragma opaque
    ensures self.n == (old(self.n) + 1) % 18446744073709551616
    ensures result == self.n

spec nested where
  ensures result == (value + 3) % 18446744073709551616

impl «shared::{closure#0}» where
  spec call where
    ensures result == self.value

spec shared where
  ensures result != value

impl «argument::{closure#0}» where
  spec call_mut where
    pragma opaque
    ensures self.n == (old(self.n) + x) % 18446744073709551616
    ensures result == self.n

spec argument where
  ensures result != value

impl «multiple::{closure#0}» where
  spec call_mut where
    pragma opaque
    ensures self.a == (old(self.a) + 1) % 18446744073709551616
    ensures result == self.a

impl «multiple::{closure#1}» where
  spec call_mut where
    pragma opaque
    ensures self.b == (old(self.b) + 1) % 18446744073709551616
    ensures result == self.b

spec multiple where
  ensures result == (value + 3) % 18446744073709551616

impl «record::{closure#0}» where
  spec call_mut where
    pragma opaque
    ensures self.n == (old(self.n) + 1) % 18446744073709551616
    ensures result.number == self.n
    ensures result.flag

spec record where
  ensures result.number == (value + 1) % 18446744073709551616
  ensures !result.flag

spec apply_two where
  pragma opaque
  ensures ensures_of<__trait_FnMut_call_mut>(old(callback), value, enabled, result, callback)

impl «arguments::{closure#0}» where
  spec call_mut where
    pragma opaque
    ensures self.n == (if enabled then (old(self.n) + x) % 18446744073709551616 else old(self.n))
    ensures result == self.n

spec arguments where
  ensures result != value
