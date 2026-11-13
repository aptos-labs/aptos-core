-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Slices

open VerexIR

private def before : SpecArray Nat 2 := ⟨⟨#[1, 2], by decide⟩, rfl⟩
private def after : SpecArray Nat 2 := ⟨⟨#[3, 4], by decide⟩, rfl⟩
private def loan : SpecSliceRef Nat := .ofArrays before after

example : loan.currentVector.values = #[1, 2] := rfl
example : loan.finalVector.values = #[3, 4] := rfl
example : (loan.map (· + 1)).currentVector.values = #[2, 3] := by
  simp [loan, before, SpecSliceRef.map, SpecSliceRef.ofArrays,
    SpecSliceRef.currentVector, SpecVector.map]
example : (loan.withCurrent after).finalVector = loan.finalVector := rfl

-- No contract premise or run-time guard is needed to state the fixed extent.
example (value : SpecSliceRef α) :
    value.currentVector.values.size = value.finalVector.values.size := value.lengths_equal

example : SpecSliceRef.ofVectors? before.vector after.vector = some loan := by
  exact SpecSliceRef.ofVectors?_readings loan

-- The external codec cannot manufacture an ill-typed prophecy pair.
example : SpecSliceRef.ofVectors? before.vector (⟨#[], by decide⟩ : SpecVector Nat) = none := by
  decide

example (value : SpecVector α) : (SpecSliceRef.settled value).currentVector = value := rfl

example (value : SpecSliceRef α) : value.map id = value := SpecSliceRef.map_id value

example (value : SpecSliceRef α) (f : α → β) (g : β → γ) :
    (value.map f).map g = value.map (g ∘ f) := SpecSliceRef.map_map f g value
