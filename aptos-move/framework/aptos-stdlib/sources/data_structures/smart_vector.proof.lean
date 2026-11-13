-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "smart_vector.move"
verex proofs where

-- `option::borrow` gives the big vector compared with its payload; preparation reads the validity
-- of a map equated entry by entry to a valid map.
verify to_vector by
  on _ { }
