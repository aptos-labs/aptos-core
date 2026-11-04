-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A mutable reference moved out of an aggregate on some paths: each function has
-- a false twin and an `ensures false` twin, whose clauses are reported here.

spec cond_move_local where
  ensures c ==> result == v
  ensures !c ==> result == a

spec cond_move_local_false where
  ensures result == v -- error: not on the other path

spec cond_move_local_vacuous where
  ensures false -- error: the paths are not vacuous

spec cond_move_param_by_value where
  ensures c ==> h.r == v
  ensures !c ==> h.r == old(h).r

spec cond_move_param_by_value_false where
  ensures h.r == v -- error: not on the other path

spec cond_move_param_by_value_vacuous where
  ensures false -- error: the paths are not vacuous

spec cond_move_lent where
  ensures c ==> h.r == v
  ensures !c ==> h.r == old(h).r

spec cond_move_lent_false where
  ensures h.r == v -- error: not on the other path

spec cond_move_lent_vacuous where
  ensures false -- error: the paths are not vacuous

spec cond_return where
  ensures c ==> a == v && b == old(b)
  ensures !c ==> b == v && a == old(a)

spec cond_return_false where
  ensures a == v -- error: not on the other path

spec cond_return_vacuous where
  ensures false -- error: the paths are not vacuous

spec cond_store where
  ensures c ==> spec.result[0] == v && spec.result[1] == b
  ensures !c ==> spec.result[1] == v && spec.result[0] == a

spec cond_store_false where
  ensures spec.result[1] == v -- error: not on the other path

spec cond_store_vacuous where
  ensures false -- error: the paths are not vacuous

spec cond_two_components where
  ensures c ==> a == v && b == old(b)
  ensures !c ==> b == v && a == old(a)

spec cond_two_components_false where
  ensures a == v -- error: not on the other path

spec cond_two_components_vacuous where
  ensures false -- error: the paths are not vacuous

spec cond_nested where
  ensures c ==> a == v
  ensures !c ==> a == old(a)

spec cond_nested_false where
  ensures a == v -- error: not on the other path

spec cond_nested_vacuous where
  ensures false -- error: the paths are not vacuous

spec cond_callee where
  ensures c ==> a == v
  ensures !c ==> a == old(a)

spec cond_callee_false where
  ensures a == v -- error: not on the other path

spec cond_callee_vacuous where
  ensures false -- error: the paths are not vacuous

spec cond_loop where
  ensures n == 0 ==> a == old(a)
  ensures n == 1 ==> a == 0
  ensures n >= 2 ==> a == 1
  loop 1 where
    invariant i <= n
    invariant i <= 1
    invariant i == 0 ==> h.r == old(a)
    invariant i == 1 ==> h.r == 0

spec cond_loop_false where
  ensures n >= 2 ==> a == 0 -- error: the move writes 1
  loop 1 where
    invariant i <= n
    invariant i <= 1
    invariant i == 0 ==> h.r == old(a)
    invariant i == 1 ==> h.r == 0

spec cond_loop_vacuous where
  ensures false -- error: the paths are not vacuous
  loop 1 where
    invariant i <= n
    invariant i <= 1
    invariant i == 0 ==> h.r == old(a)
    invariant i == 1 ==> h.r == 0

-- Where a loan dies on the path that did not store it, the place holds the other
-- reference, whose death is not this one's.

spec cond_store_entry_false where
  ensures !c ==> b == v -- error: that is the value on entry

spec whole_repoint where
  ensures c ==> spec.result[0] == v && spec.result[1] == b
  ensures !c ==> spec.result[0] == a && spec.result[1] == v

spec whole_repoint_false where
  ensures !c ==> b == v -- error: that is the value on entry

spec whole_repoint_vacuous where
  ensures false -- error: the paths are not vacuous
