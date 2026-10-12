-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec max where
  ensures result >= left && result >= right
  ensures result == left || result == right

spec min where
  ensures result <= left && result <= right
  ensures result == left || result == right

spec clamp where
  requires low <= high
  ensures low <= result && result <= high

spec raise where
  ensures slot >= floor
  ensures slot == old(slot) || slot == floor
