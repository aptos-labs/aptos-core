-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Domain errors of specification functions

Outside its domain a specification function's value is unspecified, for an
expansion and for a recursive definition alike.
-/

leaner module 0x99::spec_function_domain_errors where
  spec fun successor(x : u8) : Int := x + 1

  fun outside() -> u64 := 0
  spec outside where
    ensures successor(300) == 301 -- error: 300 is outside the domain of `successor`

  spec fun triangle(n : u64) : Int := if n == 0 then 0 else triangle(n - 1) + n

  fun negative() -> u64 := 0
  spec negative where
    ensures triangle(-1) == 0 -- error: -1 is outside the domain of `triangle`
