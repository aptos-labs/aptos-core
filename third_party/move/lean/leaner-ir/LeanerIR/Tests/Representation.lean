-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Representation

/-!
# Typed representation tests

Exercises the generic twin vocabulary the way a generated unit uses it: a
hand-written twin with its `erase`/`decode?` pair and their roundtrip.
-/

namespace LeanerIR.Tests.Representation

open LeanerIR

-- Shape recovery is independent of integer width and signedness; the
-- successful decode carries the range certificate, never an extra axiom.
example (width : IntWidth) (signed : Bool) (runtime : RuntimeValue)
    (value : SpecInt width signed)
    (decoded : decodeInt? width signed runtime = some value) :
    runtime = .integer value.val :=
  decodeInt?_shape decoded

example : decodeInt? (.bits 8) false (.integer 256) = none := by decide
example : decodeInt? (.bits 8) false (.integer (-1)) = none := by decide
example : decodeInt? (.bits 8) true (.integer (-128)) =
    some ⟨-128, by decide⟩ :=
  decodeInt?_val (⟨-128, by decide⟩ : SpecInt (.bits 8) true)

-- Identical payload types do not make runtime constructors interchangeable.
example : decodeAddress? (.string "a") = none := rfl
example : decodeString? (.address "a") = none := rfl

/-- A hand-written twin of `struct Coin has Key where value : u64`. -/
private structure Coin where
  value : SpecInt (.bits 64) false

private def Coin.erase (coin : Coin) : RuntimeValue :=
  .nominal ⟨⟨0⟩, 0⟩ none #[.integer coin.value.val]

/- Field rows are matched through `toList`: Lean cannot generate `match`
equations for array-literal patterns. -/
private def Coin.decode? : RuntimeValue → Option Coin
  | .nominal ⟨⟨0⟩, 0⟩ none fields =>
      match fields.toList with
      | [.integer value] =>
          if fits : IntegerValueFits (.bits 64) false value then
            some { value := ⟨value, fits⟩ }
          else none
      | _ => none
  | _ => none

private theorem Coin.decode?_erase (coin : Coin) :
    Coin.decode? coin.erase = some coin := by
  simp [Coin.erase, Coin.decode?, coin.value.fits]

/-- Certified bounds arrive in `omega` form. -/
example (coin : Coin) : coin.value.val < 18446744073709551616 := by
  have bounds := coin.value.unsigned_bounds
  omega

end LeanerIR.Tests.Representation
