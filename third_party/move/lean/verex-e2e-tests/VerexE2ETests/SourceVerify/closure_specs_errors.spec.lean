-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

impl «specified::{closure#0}» where
  spec call where
    requires a <= 100 && self.c <= 100
    ensures result == a + self.c

spec specified where
  requires c <= 100
  ensures result == c + 1

impl «specified_false::{closure#0}» where
  spec call where
    requires a <= 100 && self.c <= 100
    ensures result == a + self.c

-- The caller's claim is checked against the contract.
spec specified_false where
  requires c <= 100
  ensures result == c -- error: the contract adds 1

impl «caller_false::{closure#0}» where
  spec call where
    requires a <= 100 && self.c <= 100
    ensures result == a + self.c

-- The closure's precondition is the caller's obligation.
spec caller_false where
  ensures result == c + 1

impl «body_false::{closure#0}» where
  spec call where
    requires a <= 100 && self.c <= 100
    ensures result == a -- error: the body adds the upvar

spec body_false where
  requires c <= 100
  ensures result == 1

-- `no panic` is the closure's, at the closure's location.
impl «panicking::{closure#0}» where
  spec call where
    ensures result == a + self.c

impl «looped::{closure#0}» where
  spec call where
    requires m <= 100
    ensures result == 2 * m
    loop 1 where
      invariant i <= m && s == 2 * i

spec looped where
  requires n <= 100
  ensures result == 2 * n

impl «looped_false::{closure#0}» where
  spec call where
    requires m <= 100
    ensures result == 2 * m
    loop 1 where
      invariant i <= m && s == i -- error: `s` grows by 2

spec looped_false where
  requires n <= 100
  ensures result == 2 * n

impl «loops_apart::{closure#0}» where
  spec call where
    requires m <= 100
    ensures result == 2 * m
    loop 1 where
      invariant i <= m && s == 2 * i

spec loops_apart where
  requires n <= 100
  ensures result == n
  loop 1 where
    invariant j <= n && total == j

impl «mut_specified::{closure#0}» where
  spec call_mut where
    requires d <= 100 && self.n <= 100
    ensures self.n == old(self.n) + d

spec mut_specified where
  requires n0 <= 100
  ensures result == n0 + 1

impl «mut_specified_false::{closure#0}» where
  spec call_mut where
    requires d <= 100 && self.n <= 100
    ensures self.n == old(self.n) + d

spec mut_specified_false where
  requires n0 <= 100
  ensures result == n0 -- error: the contract writes through the upvar

-- A proof names the body by its path, as the failure asks.
impl «proved::{closure#0}» where
  spec call where
    requires self.c == 1
    ensures result == 1

verify «proved::{closure#0}»::call by lean =>
  verex_obligations
  verex_on
  | _ => verex_decide

spec proved where
  requires c == 1
  ensures result == 1
