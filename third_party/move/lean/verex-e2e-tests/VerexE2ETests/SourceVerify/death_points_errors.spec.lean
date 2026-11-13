-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Where a loan dies: each function has a false twin and an `ensures false` twin,
-- whose clauses are reported here.

spec whole_repoint_trailing where
  ensures c ==> spec.result[0] == v && spec.result[1] == b
  ensures !c ==> spec.result[0] == a && spec.result[1] == v

spec whole_repoint_trailing_false where
  ensures spec.result[1] == v -- error: not on the other path

spec whole_repoint_trailing_vacuous where
  ensures false -- error: the paths are not vacuous

spec cond_store_trailing where
  ensures c ==> spec.result[0] == v && spec.result[1] == b
  ensures !c ==> spec.result[0] == a && spec.result[1] == v

spec cond_store_trailing_false where
  ensures spec.result[1] == v -- error: not on the other path

spec cond_store_trailing_vacuous where
  ensures false -- error: the paths are not vacuous

spec reborrow_chain where
  ensures result == v

spec reborrow_chain_false where
  ensures result == a -- error: the write reaches it

spec reborrow_chain_vacuous where
  ensures false -- error: the paths are not vacuous

spec cond_consume where
  ensures c ==> result == v
  ensures !c ==> result == 1

spec cond_consume_false where
  ensures result == v -- error: not on the other path

spec cond_consume_vacuous where
  ensures false -- error: the paths are not vacuous

spec sibling_live where
  ensures c ==> spec.result[0] == v
  ensures !c ==> spec.result[0] == p
  ensures spec.result[1] == v

spec sibling_live_false where
  ensures spec.result[0] == v -- error: not on the other path

spec sibling_live_vacuous where
  ensures false -- error: the paths are not vacuous

spec then_reads_else_repoints where
  ensures c ==> spec.result[0] == v && spec.result[1] == b
  ensures !c ==> spec.result[0] == a && spec.result[1] == v

spec then_reads_else_repoints_false where
  ensures spec.result[1] == v -- error: not on the other path

spec then_reads_else_repoints_vacuous where
  ensures false -- error: the paths are not vacuous

spec loop_local_write where
  ensures n == 0 ==> result == a
  ensures n >= 1 ==> result == n - 1
  loop 1 where
    invariant i <= n
    invariant i == 0 ==> r == a
    invariant i >= 1 ==> r == i - 1

spec loop_local_write_false where
  ensures n >= 1 ==> result == n -- error: the last write is n - 1
  loop 1 where
    invariant i <= n
    invariant i == 0 ==> r == a
    invariant i >= 1 ==> r == i - 1

spec loop_local_write_vacuous where
  ensures false -- error: the paths are not vacuous
  loop 1 where
    invariant i <= n
    invariant i == 0 ==> r == a
    invariant i >= 1 ==> r == i - 1

spec break_skips_repoint where
  pragma unroll = 2
  requires n <= 2
  ensures c || n == 0 ==> spec.result[0] == v && spec.result[1] == b
  ensures !c && n > 0 ==> spec.result[0] == a && spec.result[1] == v

spec break_skips_repoint_false where
  pragma unroll = 2
  requires n <= 2
  ensures spec.result[1] == v -- error: not on the other path

spec break_skips_repoint_vacuous where
  pragma unroll = 2
  requires n <= 2
  ensures false -- error: the paths are not vacuous
