-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A struct holding `&mut` by value: each function has a false twin, whose clause
-- is reported here.

spec hold_write_read where
  ensures result == 5

spec hold_write_read_false where
  ensures result == x -- error: the holder wrote `x`

spec hold_param where
  ensures x == 3

spec hold_param_false where
  ensures x == old(x) -- error: the holder wrote `x`

spec hold_move_out where
  ensures result == 5

spec hold_move_out_false where
  ensures result == x -- error: a stale copy resolved would prove this

spec store where
  ensures h.r == 7

spec hold_callee_val where
  ensures result == 7

spec hold_callee_val_false where
  ensures result == x -- error: the callee wrote it

spec store_mut where
  ensures h.r == 7

spec hold_callee_mut where
  ensures result == 8

spec hold_callee_mut_false where
  ensures result == 7 -- error: the later write wins

spec hold_return where
  ensures result == 9

spec hold_return_false where
  ensures result == x -- error: written through the returned holder

spec hold_move_local where
  ensures result == 7

spec hold_move_local_false where
  ensures result == x -- error: the callee wrote it

spec hold_reassign where
  ensures result.x == 1 && result.y == 2

spec hold_reassign_false where
  ensures b == 2 -- error: that is the value on entry

spec hold_join where
  ensures c ==> result.x == 1 && result.y == b
  ensures !c ==> result.x == a && result.y == 1

spec hold_join_false where
  ensures result.x == 1 -- error: not where `c` fails

spec two_local where
  ensures result.x == 1 && result.y == 2

spec two_local_false where
  ensures result.x == 2 -- error: `a` holds 1
