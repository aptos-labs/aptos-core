-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

impl C where
  spec get where
    ensures result == self.n

  spec add where
    requires self.n + d <= 4000000000
    ensures self.n == old(self.n) + d

spec add_to where
  requires target + d <= 4000000000
  ensures target == old(target) + d

spec two_phase_local where
  requires n <= 1000
  ensures result == 2 * n

spec two_phase_local_false where
  requires n <= 1000
  ensures result == n -- error: the read sees `n`, and the write doubles it

spec two_phase_free where
  requires c.n <= 1000
  ensures c.n == 2 * old(c.n)

spec two_phase_free_false where
  requires c.n <= 1000
  ensures c.n == old(c.n) -- error: the call writes through it

spec push_len where
  ensures v.length == old(v).length + 1
  ensures v[old(v).length] == old(v).length

spec push_len_false where
  ensures v[old(v).length] == v.length -- error: the read is before the push

spec holder where
  requires h.r <= 1000
  ensures h.r == 2 * old(h).r

spec holder_false where
  requires h.r <= 1000
  ensures h.r == old(h).r -- error: the call writes through it

spec index_self where
  requires v.length > 0
  ensures v.length == old(v).length + 1
  ensures v[old(v).length] == old(v)[0]

spec index_self_false where
  requires v.length > 0
  ensures v.length == old(v).length -- error: the push extends it

spec index_operand where
  requires c.n <= 1000 && i < a.length && a[i] <= 1000
  ensures c.n == old(c.n) + a[i]

spec index_operand_false where
  requires c.n <= 1000 && i < a.length && a[i] <= 1000
  ensures c.n == old(c.n) -- error: the call adds the element

spec across_blocks where
  requires n <= 1000
  ensures b ==> result == 2 * n
  ensures !b ==> result == n + 1

spec across_blocks_false where
  requires n <= 1000
  ensures result == 2 * n -- error: not where `b` fails
