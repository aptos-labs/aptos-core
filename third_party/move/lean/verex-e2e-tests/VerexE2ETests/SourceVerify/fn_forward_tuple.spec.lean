-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec tuple_mut where
  pragma opaque
  ensures ensures_of<__trait_FnMut_call_mut>(old(callback), spec.result[0], spec.result[1], callback)

spec tuple_forward where
  pragma opaque
  ensures callback == old(callback)
  ensures ensures_of<__trait_Fn_call>(old(callback), spec.result[0], spec.result[1])

impl «tuple_value::{closure#0}» where
  spec call where
    pragma opaque
    ensures spec.result[0] == self.value
    ensures spec.result[1]

spec tuple_value where
  ensures spec.result[0] == value
  ensures spec.result[1]
