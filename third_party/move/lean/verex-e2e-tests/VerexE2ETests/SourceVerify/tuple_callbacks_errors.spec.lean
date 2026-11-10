-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec apply where
  pragma opaque
  ensures ensures_of<__trait_FnMut_call_mut>(old(callback), spec.result[0], spec.result[1], callback)

spec forward where
  pragma opaque
  ensures ensures_of<__trait_FnMut_call_mut>(old(callback), spec.result[0], spec.result[1], callback)

impl «pair::{closure#0}» where
  spec call_mut where
    pragma opaque
    ensures self.n == (old(self.n) + 1) % 18446744073709551616
    ensures spec.result[0] == self.n
    ensures spec.result[1]

spec pair where
  ensures spec.result[0] == value
  ensures spec.result[1]

spec apply_generic where
  pragma opaque
  ensures ensures_of<__trait_FnMut_call_mut>(old(callback), result, callback)

impl «generic_pair::{closure#0}» where
  spec call_mut where
    pragma opaque
    ensures self.n == (old(self.n) + 1) % 18446744073709551616
    ensures spec.result[0] == self.n
    ensures spec.result[1]

spec generic_pair where
  ensures spec.result[0] == (value + 1) % 18446744073709551616
  ensures !spec.result[1]

impl «singleton::{closure#0}» where
  spec call where
    ensures spec.result[0] == self.value

spec singleton where
  ensures spec.result[0] != value

spec apply_nested where
  pragma opaque
  ensures ensures_of<__trait_FnMut_call_mut>(old(callback), spec.result[0], spec.result[1], callback)

impl «nested_tuple::{closure#0}» where
  spec call where
    ensures spec.result[0] == self.value
    ensures spec.result[1] == (true, (self.value + 1) % 18446744073709551616)

spec nested_tuple where
  ensures spec.result[0] == value
  ensures spec.result[1] == (true, value)
