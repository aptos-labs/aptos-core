-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- `IterMut` holds its lender as a mutable reference and `next` states the
-- writeback with `final` of the payload it yields, so a loop over it reaches
-- the slice (`core/slice.spec.lean`).
spec set_all where
  ensures values.length == old(values).length
  loop 1 where
    invariant it.pos <= it.end
    invariant it.end == old(values).length
