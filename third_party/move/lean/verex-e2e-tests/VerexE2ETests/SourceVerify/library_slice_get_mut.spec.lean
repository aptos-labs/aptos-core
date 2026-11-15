-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A mutable reference yielded inside an `Option` carries a prophecy: the
-- element takes what the caller writes through it, which the model states
-- with `final` of the payload (`core/slice.spec.lean`).

spec store where
  ensures result == (i < old(values).length)
  ensures values.length == old(values).length
  ensures result ==> values == update(old(values), i, value)
  ensures !result ==> values == old(values)
