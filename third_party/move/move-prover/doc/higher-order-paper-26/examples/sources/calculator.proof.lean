-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "calculator.move"
verex proofs where

-- Each case of the stored state at entry: the closure read back and the
-- value published are the ones the state's variant holds.
verify process by
  on _ {
    split old(global<State>(s.address))
  }
