-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A bare shift by 33 would have no outcome, and an unmasked one is not this value.
spec shift_past_width where
  ensures result == (value * 8589934592) % 4294967296

spec saturate_up where
  ensures result == left + right

spec subtract where
  ensures result is Some
