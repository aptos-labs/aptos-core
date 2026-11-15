-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "reconfiguration.move"
verex proofs where

-- `new_event_handle` writes `Account` only, so `move_to` tests the entry's `Configuration`.
verify initialize by
  after call 1 of new_event_handle {
    assert exists<Configuration>(@0x1) == old(exists<Configuration>(@0x1)) by solve
    assert 0 <= spec_now_microseconds()
  }
  on _ { }
