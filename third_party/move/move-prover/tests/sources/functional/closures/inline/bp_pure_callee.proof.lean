-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Instantiate the established fold-count equation at loop entry and at
-- the current and next iteration. Avoid simplifying the whole loop context.
verify count_all by
  case leaf_1 =>
    exact (assertionHolds 0 (by omega) (by omega) (by omega) (by omega) (by omega)).symm
  case leaf_2 =>
    have current := assertionHolds i.val (by omega) (by omega) (by omega) (by omega) (by omega)
    have next := assertionHolds (i.val + 1) (by omega) (by omega) (by omega) (by omega) (by omega)
    omega
