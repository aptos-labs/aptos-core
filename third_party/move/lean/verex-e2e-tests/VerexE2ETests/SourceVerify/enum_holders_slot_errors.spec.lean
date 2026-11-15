-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Enums holding a mutable reference: each function has a false twin and an
-- `ensures false` twin, whose clauses are reported here.

spec slot_repointed where
  ensures c ==> spec.result[0] == a && spec.result[1] == (v + 1) % 256
  ensures !c ==> spec.result[0] == v && spec.result[1] == b

spec slot_repointed_false where
  ensures spec.result[0] == v -- error: not on the other path

spec slot_repointed_vacuous where
  ensures false -- error: the paths are not vacuous
