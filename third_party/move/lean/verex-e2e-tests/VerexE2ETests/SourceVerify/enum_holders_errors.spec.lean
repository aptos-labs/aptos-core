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

spec slot_repointed where
  ensures c ==> spec.result[0] == a && spec.result[1] == (v + 1) % 256
  ensures !c ==> spec.result[0] == v && spec.result[1] == b

spec slot_repointed_false where
  ensures spec.result[0] == v -- error: not on the other path

spec slot_repointed_vacuous where
  ensures false -- error: the paths are not vacuous
