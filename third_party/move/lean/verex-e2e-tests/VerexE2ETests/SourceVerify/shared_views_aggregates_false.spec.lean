-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec option_reader where
  ensures result != 0

spec option_view where
  ensures result == a

spec option_reference where
  ensures result != 0

spec is_some_view where
  ensures result == a

spec struct_reader where
  ensures result != v.h.r

spec struct_view where
  ensures result == a

spec copy_out where
  ensures result != s.v

spec copy_out_at where
  ensures result == a

spec closure_view where
  ensures result == a

spec closure_moved where
  ensures result == a

spec iter_first where
  ensures result != 0

spec iter_count where
  requires w.v.length <= 1000
  ensures result == 0
  loop 1 where
    invariant iter.slice == w.v && iter.end == w.v.length && iter.pos <= iter.end
    invariant n == iter.pos
