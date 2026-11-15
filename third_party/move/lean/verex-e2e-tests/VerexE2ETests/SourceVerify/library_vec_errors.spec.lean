-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec push_one where
  ensures values.length == old(values).length

spec pop_some where
  ensures result is Some

spec drop_guards where
  ensures result == 1

-- A standard function the library does not model.
spec deduped where
  ensures true
