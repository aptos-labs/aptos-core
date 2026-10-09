-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Domains of specification functions

A fixed-width parameter type is the function's domain: the function is defined
where each argument fits it, and its value elsewhere is unspecified. The body
reads such a parameter as an `Int`, so its arithmetic is mathematical. A
recursive definition unfolds inside the domain; an expansion is guarded by it.
-/

leaner module 0x99::spec_function_domains where
  -- ## Expansions inside the domain

  spec fun successor(x : u8) : Int := x + 1

  fun next(x : u8) -> u64 := x as u64 + 1
  spec next where
    ensures result == successor(x)

  -- The body computes and compares beyond the parameter's width.
  spec fun beyond(x : u64) : Int := x + 18446744073709551616

  spec fun below(x : u64) : Bool := x < 18446744073709551616

  fun widened(x : u64) -> u64 := x
  spec widened where
    ensures beyond(x) == x + 18446744073709551616
    ensures below(x)

  -- An `Int` parameter has no domain.
  spec fun successor_int(x : Int) : Int := x + 1

  fun unbounded() -> u64 := 0
  spec unbounded where
    ensures successor_int(300) == 301

  -- ## Recursion inside the domain

  spec fun triangle(n : u64) : Int := if n == 0 then 0 else triangle(n - 1) + n

  fun six() -> u64 := 6
  spec six where
    ensures result == triangle(3)
