-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Representation

/-!
# Typed representation tests

Exercises the generic typed-view vocabulary the way a generated unit uses
it: a hand-written twin with its `erase`/`decode?` pair, the roundtrip, and
the representation laws under publish/take within and across families.
-/

namespace LeanerIR.Tests.Representation

open LeanerIR

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

/-- Decoding through the erasure recovers the typed contents. -/
example (contents : Option Coin) :
    (contents.map Coin.erase).bind Coin.decode? = contents :=
  map_erase_bind_decode Coin.decode?_erase contents

/-- Certified bounds arrive in `omega` form. -/
example (coin : Coin) : coin.value.val < 18446744073709551616 := by
  have bounds := coin.value.unsigned_bounds
  omega

private def coinFamily (contents : StorageKey → Option Coin) :
    GlobalMap → Prop :=
  FamilyRepresentation Coin.erase ⟨0⟩ ⟨5⟩ contents

/-- The empty state represents the empty family. -/
example : coinFamily (fun _ => none) {} :=
  FamilyRepresentation.empty Coin.erase ⟨0⟩ ⟨5⟩

/-- Publish then read back: the representation tracks the typed update, and
the runtime lookup of the published key is the erasure by reduction. -/
example (contents : StorageKey → Option Coin) (globals : GlobalMap)
    (represented : coinFamily contents globals) (coin : Coin) :
    (globals.insert ⟨⟨0⟩, ⟨5⟩, .address "a"⟩ coin.erase).lookup
        ⟨⟨0⟩, ⟨5⟩, .address "a"⟩ = some coin.erase ∧
      coinFamily (updateContents contents (.address "a") (some coin))
        (globals.insert ⟨⟨0⟩, ⟨5⟩, .address "a"⟩ coin.erase) :=
  ⟨by simp, represented.insert_self (.address "a") coin⟩

/-- Take: the representation drops the key. -/
example (contents : StorageKey → Option Coin) (globals : GlobalMap)
    (represented : coinFamily contents globals) :
    coinFamily (updateContents contents (.address "a") none)
      (globals.erase ⟨⟨0⟩, ⟨5⟩, .address "a"⟩) :=
  represented.erase_self (.address "a")

/-- Cross-family independence is a theorem: a write under another family's
type identity leaves this family's representation untouched. -/
example (contents : StorageKey → Option Coin) (globals : GlobalMap)
    (represented : coinFamily contents globals) (value : RuntimeValue) :
    coinFamily contents (globals.insert ⟨⟨0⟩, ⟨7⟩, .address "a"⟩ value) :=
  represented.insert_other ⟨⟨0⟩, ⟨7⟩, .address "a"⟩ value (by right; decide)

end LeanerIR.Tests.Representation
