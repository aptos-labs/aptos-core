-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Each states another equality.

spec same_point where
  ensures result == (p.a == q.a)

spec same_color where
  ensures result == (x is Red)

spec same_string where
  ensures result == (s.length != t.length)

-- `&str == &str` declares two elided impl lifetimes, named apart; it has no contract.
spec same_text where
  ensures true
