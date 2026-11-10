-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Whole-array mutable unsizing is supported; this postcondition is false.

spec through_unsize where
  ensures result == 8 -- error: deliberately false postcondition
