-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Whole-array mutable unsizing preserves the backing extent and element writes.

spec through_unsize where
  ensures result == 7
