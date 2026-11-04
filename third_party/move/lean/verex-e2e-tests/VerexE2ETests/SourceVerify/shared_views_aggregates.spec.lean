-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec option_reader where
  ensures o is None ==> result == 0
  ensures o is Some ==> result == (o.0).r

spec option_view where
  ensures result == a % 100 + 1

spec option_reference where
  ensures o is None ==> result == 0
  ensures o is Some ==> result == o.0

spec is_some_view where
  ensures result == a % 100 + 4

spec struct_reader where
  ensures result == v.h.r

spec struct_view where
  ensures result == a % 100 + 2

spec copy_out where
  ensures result == s.v

spec copy_out_at where
  ensures result == a % 100 + 3

spec closure_view where
  ensures result == (a % 100 + a % 100) % 100

spec closure_moved where
  ensures result == (a % 100 + a % 100) % 100 + a % 100

spec iter_first where
  ensures w.v.length == 0 ==> result == 0
  ensures w.v.length > 0 ==> result == w.v[0]

spec iter_count where
  requires w.v.length <= 1000
  ensures result == w.v.length
  loop 1 where
    invariant iter.slice == w.v && iter.end == w.v.length && iter.pos <= iter.end
    invariant n == iter.pos
