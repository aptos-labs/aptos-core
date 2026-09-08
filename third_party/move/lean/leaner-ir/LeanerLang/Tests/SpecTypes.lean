-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Generated typed-twin tests

`#leaner_unit` materializes one typed twin per supported struct declaration
and keyed accessors per storable family.  These assertions pin the generated
vocabulary: the erasure/decoding roundtrip, certified field bounds, the
defaults, and the agreement of the family accessors with the runtime map.
-/

namespace LeanerLang.Tests.SpecTypes

leaner module 0x42::twins where
  struct Amount has Copy, Drop, Store where
    value : u64
  struct Coin has Key where
    amount : Amount
    flag : Bool
    owner : Address
  public fun balance_of(addr : Address) -> u64 := Coin[addr].amount.value

#leaner_unit 0x42::twins

open LeanerIR
open «0x42»

/-- The roundtrip of a twin with a nested twin and mixed scalar fields is
one `simp` through the tagged codec lemmas. -/
example (coin : twins.Coin) :
    twins.Coin.decode? (twins.Coin.erase coin) = some coin := by simp

/-- Decoding through the erasure recovers typed contents wholesale. -/
example (contents : Option twins.Coin) :
    (contents.map twins.Coin.erase).bind twins.Coin.decode? = contents := by
  simp

/-- The generated default inhabits every field. -/
example : (default : twins.Coin).flag = false := rfl
example : (default : twins.Coin).amount.value.val = 0 := rfl

/-- Certified integer fields carry their bounds in `omega` form. -/
example (amount : twins.Amount) : amount.value.val ≤ 18446744073709551615 := by
  have bounds := amount.value.unsigned_bounds
  omega

/-- The existence test is the raw runtime lookup — the same term a program's
`exists<Coin>` evaluates, with nothing between them. -/
example (globals : GlobalMap) (key : RuntimeValue) :
    twins.Coin.contains globals key
      = (globals.lookup (twins.Coin.key key)).isSome := rfl

/-- The typed read decodes the same lookup. -/
example (globals : GlobalMap) (key : RuntimeValue) :
    twins.Coin.read globals key
      = (globals.lookup (twins.Coin.key key)).bind twins.Coin.decode? := rfl

/-- Publishing an erasure is read back as its typed value. -/
example (globals : GlobalMap) (coin : twins.Coin) (key : RuntimeValue) :
    twins.Coin.read
      (globals.insert (twins.Coin.key key) (twins.Coin.erase coin)) key
      = some coin := by
  simp [twins.Coin.read]

end LeanerLang.Tests.SpecTypes
