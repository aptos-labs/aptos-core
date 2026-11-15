-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Enums holding a mutable reference: each function has a false twin and an
-- `ensures false` twin, whose clauses are reported here.

spec opt_by_ref where
  ensures old(o) is Some ==> o is Some && o.0 == v
  ensures old(o) is None ==> o is None

spec opt_by_ref_false where
  ensures o is Some -- error: not on the other path

spec opt_by_ref_vacuous where
  ensures false -- error: the paths are not vacuous

spec opt_ref_of_ref where
  ensures old(o) is Some ==> o is Some && o.0 == v
  ensures old(o) is None ==> o is None

spec opt_ref_of_ref_false where
  ensures o is Some -- error: not on the other path

spec opt_ref_of_ref_vacuous where
  ensures false -- error: the paths are not vacuous

spec opt_none_on_one_path where
  ensures c ==> result == a
  ensures !c ==> result == v

spec opt_none_on_one_path_false where
  ensures result == v -- error: not on the other path

spec opt_none_on_one_path_vacuous where
  ensures false -- error: the paths are not vacuous
