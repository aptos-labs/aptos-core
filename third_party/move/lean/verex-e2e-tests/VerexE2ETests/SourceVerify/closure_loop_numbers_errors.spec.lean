-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

impl «only_closure_loop::{closure#0}» where
  spec call where
    requires m <= 100
    ensures result == 2 * m
    loop 1 where
      invariant i <= m && s == 2 * i

spec only_closure_loop where
  requires n <= 100
  ensures result == 2 * n
  loop 1 where -- error: the function has no loop
    invariant true
