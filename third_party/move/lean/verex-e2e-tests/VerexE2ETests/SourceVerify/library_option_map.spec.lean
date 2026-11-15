-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- `Option::map` applies its argument where there is a payload, and states the
-- result through the `FnOnce` evidence its declared bound supplies
-- (`core/option.spec.lean`).

impl «bump::{closure#0}» where
  spec call_once where
    pragma opaque
    ensures result == (x + 1) % 4294967296

spec bump where
  ensures (result is Some) == (value is Some)
  ensures value is Some ==> result.0 == (value.0 + 1) % 4294967296

impl «constant::{closure#0}» where
  spec call_once where
    pragma opaque
    ensures result == 7

spec constant where
  ensures (result is Some) == (value is Some)
  ensures value is Some ==> result.0 == 7
