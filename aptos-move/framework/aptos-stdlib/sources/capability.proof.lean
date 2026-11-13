-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "capability.move"
verex proofs where

-- The delegate check and the capability state the path read: each
-- obligation is decided from the state's case and the delegate's index.
verify acquire by
  on _ { }

verify acquire_linear by
  on _ { }
