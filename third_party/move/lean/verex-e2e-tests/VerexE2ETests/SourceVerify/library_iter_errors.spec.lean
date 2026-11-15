-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec sum_slice where
  requires values.length <= 1000
  ensures result <= values.length * 4294967295
  loop 1 where
    invariant iter.slice == values && iter.end == values.length && iter.pos < iter.end
    invariant total <= iter.pos * 4294967295

spec sum_vec where
  requires values.length <= 1000
  ensures result < values.length * 4294967295
  loop 1 where
    invariant iter.slice == values && iter.end == values.length && iter.pos <= iter.end
    invariant total <= iter.pos * 4294967295

spec sum_borrowed where
  requires values.length <= 1000
  ensures result <= values.length * 4294967295
  loop 1 where
    invariant iter.slice == values && iter.end == values.length && iter.pos <= iter.end
    invariant total + 1 <= iter.pos * 4294967295

spec sum_vec_borrowed where
  ensures result <= values.length * 4294967295
  loop 1 where
    invariant iter.slice == values && iter.end == values.length && iter.pos <= iter.end
    invariant total <= iter.pos * 4294967295

spec count where
  ensures result == values.length + 1
  loop 1 where
    invariant iter.slice == values && iter.end == values.length && iter.pos <= iter.end
    invariant n == iter.pos

spec contains_value where
  ensures result == contains(values, target)
  loop 1 where
    invariant iter.slice == values && iter.end == values.length && iter.pos <= iter.end
    invariant ∀ (k in 0 .. iter.pos + 1), values[k] != target

spec first_or where
  ensures values.length > 0 ==> result == values[values.length - 1]
  ensures values.length == 0 ==> result == default

spec last_or where
  ensures values.length > 0 ==> result == values[0]
  ensures values.length == 0 ==> result == default
  loop 1 where
    invariant iter.slice == values && iter.end == values.length && iter.pos <= iter.end
    invariant iter.pos == 0 ==> last == default
    invariant iter.pos > 0 ==> last == values[iter.pos - 1]
