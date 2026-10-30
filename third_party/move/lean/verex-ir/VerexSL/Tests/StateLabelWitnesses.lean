-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

-- Keep constructive state-label witnesses inside the ordinary verification
-- budget: searching every execution point first used to exceed it. The
-- budget is 8% above the cost of 2026-10-21 (`doc/designs/performance.md`,
-- "Measurement debts").
set_option maxHeartbeats 27000
set_option verex.verifyHeartbeats 27000

verex module 0x42::state_label_witnesses where

  -- Copyright © Aptos Foundation
  -- SPDX-License-Identifier: Apache-2.0
  -- Labeled spec-function call where the callee uses `old(p)` on a `&mut`
  -- parameter and also reads global memory.
  struct Counter has Copy, Drop, Store where
    value : u64

  struct Cap has Key where
    max : u64

  spec fun under_cap(old_c : Counter, c : Counter, addr : Address) : Bool :=
    old_c.value < c.value && c.value <= global<Cap>(addr).max

  fun inc_under_cap_twice(c : &mut Counter, addr : Address) -> Unit := do
    if c.value + 1 < Cap[addr].max then c.value := c.value + 1
    if c.value + 1 < Cap[addr].max then c.value := c.value + 1

  spec inc_under_cap_twice where
    requires c.value + 2 < global<Cap>(addr).max
    ensures ∃ (S : StateDomain),
        ((..S |~ under_cap(old(c), c, addr)))
          && (S.. |~ under_cap(old(c), c, addr))
