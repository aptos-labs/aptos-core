-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec take where
  ensures (result is Some) == (value is Err) -- error: deliberately false postcondition
