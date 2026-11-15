-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- The contract reads the other element.

spec through_generic where
  ensures result == pair[0]
