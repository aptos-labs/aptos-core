-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

spec same_point where
  ensures result == (p.a == q.a && p.b == q.b)

spec same_color where
  ensures result == ((x is Red && y is Red) || (x is Green && y is Green))

spec same_string where
  ensures result == (s.length == t.length)
