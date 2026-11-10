-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace declared_lifetimes_adt_bound where
  -- An input ADT's declared lifetime bound also holds in this function.
  struct Ordered {a : lifetime} {b : lifetime outlives a} where
    short : &[a] u32
    long : &[b] u32

  fun ordered_long {x : lifetime} {y : lifetime outlives x}(
    o : Ordered<lifetime x, lifetime y>
  ) -> &[x] u32 := copy(o.long)
