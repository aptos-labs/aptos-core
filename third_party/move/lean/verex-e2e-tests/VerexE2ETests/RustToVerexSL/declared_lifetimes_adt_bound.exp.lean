-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace declared_lifetimes_adt_bound where
  -- An ADT's bound on its lifetimes is not exposed, so a result that needs it is
  -- rejected.
  struct Ordered {a : lifetime} {b : lifetime} where
    short : &[a] u32
    long : &[b] u32

  fun ordered_long {x : lifetime} {y : lifetime}(
    o : Ordered<lifetime x, lifetime y>
  ) -> &[x] u32 := copy(o.long)
