-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Enums holding a mutable reference: each function has a false twin and an
-- `ensures false` twin, whose clauses are reported here.

spec opt_by_value where
  ensures old(o) is Some ==> o.0 == v

spec opt_by_value_false where
  ensures o is None -- error: not on the other path

spec opt_by_value_vacuous where
  ensures false -- error: the paths are not vacuous

spec opt_by_value_caller where
  ensures c ==> result == v
  ensures !c ==> result == a

spec opt_by_value_caller_false where
  ensures result == v -- error: not on the other path

spec opt_by_value_caller_vacuous where
  ensures false -- error: the paths are not vacuous

spec opt_returned where
  ensures c ==> a == v
  ensures !c ==> a == old(a)

spec opt_returned_false where
  ensures a == v -- error: not on the other path

spec opt_returned_vacuous where
  ensures false -- error: the paths are not vacuous
