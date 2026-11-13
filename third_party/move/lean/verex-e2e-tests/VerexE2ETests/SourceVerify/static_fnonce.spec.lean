-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec take where
  pragma opaque
  ensures result == token.value

spec apply_once where
  pragma opaque
  ensures ensures_of<__trait_FnOnce_call_once>(callback, argument, result)

spec forward where
  pragma opaque
  ensures ensures_of<__trait_FnOnce_call_once>(callback, argument, result)

impl «scalar::{closure#0}» where
  spec call_once where
    pragma opaque
    ensures result == (self.token.value + x) % 18446744073709551616

spec scalar where
  ensures result == (value + 1) % 18446744073709551616

impl «pair::{closure#0}» where
  spec call_once where
    pragma opaque
    ensures spec.result[0] == (self.token.value + x) % 18446744073709551616
    ensures spec.result[1]

spec pair where
  ensures spec.result[0] == (value + 1) % 18446744073709551616
  ensures spec.result[1]

spec apply_unit where
  pragma opaque
  ensures ensures_of<__trait_FnOnce_call_once>(callback)

spec unit where
  ensures true
