-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A contract cannot stand for an opaque function: its caller is refused and itself reported.

spec calls_projection where
  ensures false -- error: a call of an opaque function refuses its caller, unverified

spec calls_with_contract where
  ensures result == a -- error: refused all the same, so this is only assumed by its callers

spec calls_a_refused_caller where
  ensures result == a

spec plain where
  ensures result == a / 2
