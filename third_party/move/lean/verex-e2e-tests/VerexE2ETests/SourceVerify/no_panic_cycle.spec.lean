-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A function on a call cycle is used through its contract, so an unspecified
-- one is reported: nothing verifies its implicit clause.

spec caller where
  ensures true

spec tick where
  ensures true
