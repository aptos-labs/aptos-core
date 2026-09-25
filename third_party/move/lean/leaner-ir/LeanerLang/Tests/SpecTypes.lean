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
  struct Buffer has Copy, Drop, Store where
    bytes : Vector<u8>
    flags : Vector<Bool>
    amounts : Vector<Amount>
    rows : Vector<Vector<u64> >
  struct LoopBits has Copy, Drop, Store where
    length : u64
    flags : Vector<Bool>
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

/-- Vector fields retain their element's native representation, including
nested vectors and nominal elements, and roundtrip through their codecs. -/
example (buffer : twins.Buffer) :
    twins.Buffer.decode? (twins.Buffer.erase buffer) = some buffer := by simp

example : (default : twins.Buffer).rows.values = #[] := rfl

/-- Each Move vector field, including a nested row, carries its own bound. -/
example (buffer : twins.Buffer) : buffer.rows.values.size < 2 ^ 64 :=
  buffer.rows.bounded

example (row : LeanerIR.SpecVector (LeanerIR.SpecInt (.bits 64) false)) :
    row.values.size < 2 ^ 64 := row.bounded

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
  simp [twins.Coin.read, twins.Coin.readAt, twins.Coin.key]

end LeanerLang.Tests.SpecTypes

namespace LeanerLang.Tests.EnumCodecs

leaner module 0x42::enum_codecs where
  enum Choice has Copy, Drop, Store where
    | Empty
    | Number (value : u8)
    | OtherNumber (value : u8)
    | Flag (value : Bool)

#leaner_unit 0x42::enum_codecs

open LeanerIR «0x42».enum_codecs

-- Raw payload equations select the tag without requiring an erased
-- SpecInt metavariable to unify with a scalar integer expression.
example (value : RuntimeValue) :
    Choice.decode? (.nominal ⟨⟨0⟩, 0⟩ (some "Number") #[value]) =
      (decodeInt? (.bits 8) false value).map Choice.Number := by
  rw [Choice.decode?_Number_literal]
  cases decodeInt? (.bits 8) false value <;> rfl

example : Choice.decode? (.nominal ⟨⟨0⟩, 0⟩ (some "Number") #[.integer 256]) = none := by
  simp [decodeInt?, IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?]

example : Choice.decode? (.nominal ⟨⟨0⟩, 0⟩ (some "Number") #[.bool true]) = none := by
  simp [decodeInt?]

example : Choice.decode? (.nominal ⟨⟨0⟩, 0⟩ (some "Number") #[]) = none := by
  simp [Choice.decode?]

example : Choice.decode? (.nominal ⟨⟨0⟩, 0⟩ (some "Missing") #[]) = none := by
  simp [Choice.decode?]

example : Choice.decode? (.nominal ⟨⟨0⟩, 1⟩ (some "Empty") #[]) = none := by
  simp [Choice.decode?]

end LeanerLang.Tests.EnumCodecs

namespace LeanerLang.Tests.VectorProfiles

leaner namespace rust_vector_twins where
  struct Buffer where
    values : Vector<u64>

#leaner_unit rust_vector_twins

/-- The Move-specific bound must not change Rust's existing native array. -/
example (buffer : rust_vector_twins.Buffer) : Array (LeanerIR.SpecInt (.bits 64) false) :=
  buffer.values

example (buffer : rust_vector_twins.Buffer) :
    rust_vector_twins.Buffer.decode? (rust_vector_twins.Buffer.erase buffer) = some buffer := by
  simp

end LeanerLang.Tests.VectorProfiles

namespace LeanerLang.Tests.GenericEnumCodecs

leaner module 0x42::generic_enum_codecs where
  enum Choice {T has Copy, Drop, Store} {U has Copy, Drop, Store} has Copy, Drop, Store where
    | Empty
    | First (value : T)
    | Second (value : U)
    | Batch (values : Vector<T>)

#leaner_unit 0x42::generic_enum_codecs

open LeanerIR «0x42».generic_enum_codecs

set_option maxHeartbeats 1000 in
example {α β : Type} (left : Proofs.Codec α RuntimeValue)
    (right : Proofs.Codec β RuntimeValue) (value : Choice α β) :
    Choice.decode? left right (Choice.erase left right value) = some value := by
  simp

set_option maxHeartbeats 1000 in
example {α β : Type} (left : Proofs.Codec α RuntimeValue)
    (right : Proofs.Codec β RuntimeValue) (contents : Option (Choice α β)) :
    (contents.map (Choice.erase left right)).bind (Choice.decode? left right) = contents := by
  simp

set_option maxHeartbeats 1000 in
example {α β : Type} (left : Proofs.Codec α RuntimeValue)
    (right : Proofs.Codec β RuntimeValue) (value : RuntimeValue) :
    Choice.decode? left right (.nominal ⟨⟨0⟩, 0⟩ (some "Second") #[value]) =
      (right.decode? value).map Choice.Second := by
  rw [Choice.decode?_Second_literal]
  cases right.decode? value <;> rfl

example : Choice.decode? Proofs.Codec.bool Proofs.Codec.bool
    (.nominal ⟨⟨0⟩, 0⟩ (some "First") #[.integer 7]) = none := by
  simp [Proofs.Codec.bool, decodeBool?]

example : Choice.decode? Proofs.Codec.bool Proofs.Codec.bool
    (.nominal ⟨⟨0⟩, 0⟩ (some "Missing") #[]) = none := by
  simp [Choice.decode?]

open Lean Elab Command in
run_cmd do
  for name in [``Choice.decode?_erase, ``Choice.decode?_map_erase,
      ``Choice.decode?_First_literal, ``Choice.decode?_Second_literal,
      ``Choice.decode?_Batch_literal] do
    if (← collectAxioms name).contains ``sorryAx then
      throwError "generic enum helper contains an admission: {name}"

end LeanerLang.Tests.GenericEnumCodecs
