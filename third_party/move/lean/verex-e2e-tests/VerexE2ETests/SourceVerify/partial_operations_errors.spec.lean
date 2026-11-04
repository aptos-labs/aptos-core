-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A partial operation owes its domain, which an `aborts_if` does not excuse:
-- outside it the operation has no outcome at all.

spec divide where
  aborts_if b == 0

spec overflow_value where
  ensures result == a + b
