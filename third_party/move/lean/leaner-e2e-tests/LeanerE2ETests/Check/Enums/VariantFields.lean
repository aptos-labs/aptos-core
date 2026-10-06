-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-!
# Variant fields of another variant

Selecting, reading, or writing a field of an enum value whose variant lacks
it makes the profile's mismatch throw: in Move, an abort with the
incomplete-match code `0xCA26CBD9BE0B0001`, as for a pattern no value fits.
-/

namespace LeanerLang.Tests.Check.Enums.VariantFields

leaner module 0x42::variant_fields where
  enum Common has Copy, Drop where
    | Foo (x : u64, y : u8)
    | Bar (x : u64, y : u8, z : u32)

  fun select_z(c : Common) -> u32 := c.z
  spec select_z where
    aborts_if !(c is Bar) with 14566554180833181697
    ensures result == c.z

  fun select_z_incorrect(c : Common) -> u32 := c.z
  spec select_z_incorrect where
    aborts_if false -- error: a `Foo` aborts

  fun select_x(c : Common) -> u64 := c.x
  spec select_x where
    aborts_if false

  fun write_z(c : Common) -> Common := do
    let mut d := c
    d.z := 7u32
    d
  spec write_z where
    aborts_if !(c is Bar) with 14566554180833181697

  -- A field several variants give different types is selected from the
  -- variants listed, as Move's specification `s.Number.value` does.
  enum Slot has Copy, Drop where
    | Number (value : u64)
    | Flag (value : Bool)

  fun number(s : Slot) -> u64 := core.data.selectVariants[Slot, Number.value](s)
  spec number where
    aborts_if !(s is Number) with 14566554180833181697
    ensures result == core.data.selectVariants[Slot, Number.value](s)

-- The runtime makes the same throw.
open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  let value (structId : Nat) (variant : String) (fields : Array RuntimeValue) : RuntimeValue :=
    .nominal ⟨⟨0⟩, structId⟩ (some variant) fields
  let common := value 0
  let slot := value 1
  let mismatch : Outcome := .threw .abort #[.integer moveIncompleteMatchAbortCode]
  assertRuns `«0x42».variant_fields #[
    ⟨"select_z", #[common "Bar" #[.integer 1, .integer 2, .integer 3]],
      .returned #[.integer 3], {}⟩,
    ⟨"select_z", #[common "Foo" #[.integer 1, .integer 2]], mismatch, {}⟩,
    ⟨"write_z", #[common "Foo" #[.integer 1, .integer 2]], mismatch, {}⟩,
    ⟨"number", #[slot "Number" #[.integer 5]], .returned #[.integer 5], {}⟩,
    ⟨"number", #[slot "Flag" #[.bool true]], mismatch, {}⟩]

end LeanerLang.Tests.Check.Enums.VariantFields
