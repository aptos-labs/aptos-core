-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Term

namespace VerexIR.Tests.TakeOwnerValidity
noncomputable section

open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

private abbrev flag : NTy := .struct ⟨⟨0⟩, 0⟩ .nil (.cons .bool .nil)

@[reducible] private def flags : Carriers :=
  { Carriers.ground with
    own := fun _ _ raw => raw = @NTy.encode Carriers.ground.toSorts flag (true, ()) }

attribute [local instance] flags

private abbrev good : flag.carrier := (true, ())
private abbrev bad : flag.carrier := (false, ())

private theorem good_valid : NTy.Holds flag good := ⟨rfl, trivial, trivial⟩

private theorem bad_invalid : ¬ NTy.Holds flag bad := by
  intro h
  have eq := @NTy.encode_injective Carriers.ground.toSorts flag bad good h.deepHolds.1
  cases eq

private abbrev nested (current prophecy : flag.carrier) :
    HEnv (.cons (.ref (.ref flag)) .nil) :=
  (some ((current, prophecy), (good, good)), ())

private abbrev takeNested (ops : InterpreterOps unit flags Unit) :=
  (Term.takeAt (ρ := .none) 1 none (.here (Γ := .nil)) (.deref (.nil (τ := .ref flag)))).denoteIn ops

-- The outer slot is valid at entry even when the extracted prophecy is bad.
example : HEnv.Holds _ (nested good bad) :=
  ⟨⟨good_valid.deepHolds⟩, trivial⟩

example (ops : InterpreterOps unit flags Unit) :
    (takeNested ops (nested good bad)).undefined () := by
  exact Or.inl bad_invalid

-- The extracted current may be invalid. The move returns the original value
-- while the lender is left with its settled replacement.
example (ops : InterpreterOps unit flags Unit) :
    (takeNested ops (nested bad good)).ok ()
      (.value (bad, good) (nested good good)) () := by
  exact ⟨(), (), ⟨good_valid, rfl, rfl⟩, _, (), ⟨rfl, rfl⟩, rfl, rfl⟩

example (ops : InterpreterOps unit flags Unit) :
    ((Term.takeAt (ρ := .none) 1 none (.here (Γ := .nil)) (.nil (τ := .ref flag))).denoteIn
      ops (some (bad, good), ())).ok () (.value (bad, good) (some (good, good), ())) () := by
  exact ⟨_, (), ⟨rfl, rfl⟩, rfl, rfl⟩

private abbrev singleton (value : flag.carrier) : SpecArray flag.carrier 1 :=
  ⟨⟨#[value], by simp⟩, rfl⟩

private abbrev slice (current final : flag.carrier) : (NTy.sliceRef flag).carrier :=
  SpecSliceRef.ofArrays (singleton current) (singleton final)

private abbrev sliceEnv (current : (NTy.sliceRef flag).carrier) :
    HEnv (.cons (.ref (.sliceRef flag)) .nil) :=
  (some (current, slice good good), ())

private abbrev takeSlice :=
  takeSliceAtValue (State := Unit) (ρ := .none) 1 none (.here (Γ := .nil))
    (.deref (.nil (τ := .sliceRef flag)))

private theorem slice_valid : NTy.Holds (.sliceRef flag) (slice good good) := by
  apply (NTy.holds_sliceRef flag _).mpr
  constructor <;> intro value member <;> change value ∈ #[good] at member <;>
    obtain rfl : value = good := by simpa using member
  all_goals exact good_valid

-- Checking only the settled replacement would see two good vectors here.
example : (takeSlice (sliceEnv (slice bad good))).undefined () := by
  apply Or.inl
  intro held
  have member : bad ∈ (slice bad good).currentVector.values := by
    change bad ∈ #[bad]
    simp
  exact bad_invalid (((NTy.holds_sliceRef flag _).mp held).1 _ member)

example : (takeSlice (sliceEnv (slice good good))).ok ()
    (.value (slice good good) (sliceEnv (slice good good))) () := by
  exact ⟨(), (), ⟨slice_valid, rfl, rfl⟩, _, (), ⟨rfl, rfl⟩, rfl, rfl⟩

example : (takeSliceAtValue (State := Unit) (ρ := .none) 1 none (.here (Γ := .nil))
    (.nil (τ := .sliceRef flag)) (some (slice good good), ())).ok ()
    (.value (slice good good) (some (slice good good), ())) () := by
  exact ⟨_, (), ⟨rfl, rfl⟩, rfl, rfl⟩

end

end VerexIR.Tests.TakeOwnerValidity
