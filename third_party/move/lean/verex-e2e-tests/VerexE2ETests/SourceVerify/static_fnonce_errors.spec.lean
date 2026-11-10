-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec take where
  pragma opaque
  ensures result == token.value

spec apply where
  pragma opaque
  ensures ensures_of<__trait_FnOnce_call_once>(callback, result)

impl «wrong::{closure#0}» where
  spec call_once where
    pragma opaque
    ensures result == self.token.value

spec wrong where
  ensures result != value

impl «panicking::{closure#0}» where
  spec call_once where
    pragma opaque
    panics_if self.token.value == 0
    ensures result == self.token.value

spec panicking where
  ensures result == value
