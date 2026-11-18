-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "map_write_block.move"
verex proofs where

verify write_read by
  on assert 1 { }

verify write_read_rest by
  on _ { }
