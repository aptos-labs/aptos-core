-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec option_reader where
  ensures false

spec option_view where
  ensures false

spec option_reference where
  ensures false

spec is_some_view where
  ensures false

spec struct_reader where
  ensures false

spec struct_view where
  ensures false

spec copy_out where
  ensures false

spec copy_out_at where
  ensures false

spec closure_view where
  ensures false

spec closure_moved where
  ensures false

spec iter_first where
  ensures false

spec iter_count where
  requires w.v.length <= 1000
  ensures false
  loop 1 where
    invariant iter.slice == w.v && iter.end == w.v.length && iter.pos <= iter.end
    invariant n == iter.pos
