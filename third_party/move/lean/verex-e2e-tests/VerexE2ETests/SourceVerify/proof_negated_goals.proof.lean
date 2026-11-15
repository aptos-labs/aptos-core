-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "proof_negated_goals.move"
verex proofs where

verify not_not by
  on ensures 1 { }
  on _ { }

verify not_or by
  on ensures 1 { }
  on _ { }

verify not_exists by
  on ensures 1 { }
  on _ { }

verify not_and by
  on ensures 1 { }
  on _ { }
