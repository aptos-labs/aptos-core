-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Representation

/-!
# Fixed-extent mutable slice values

A slice reference owns two readings of the same borrowed extent. The extent
is existential at the reference boundary, but shared by its current value and
prophecy. A differently sized prophecy is not a value of this carrier.
-/

namespace VerexIR

structure SpecSliceRef (α : Type) where
  extent : Nat
  current : SpecArray α extent
  final : SpecArray α extent

namespace SpecSliceRef

def currentVector (value : SpecSliceRef α) : SpecVector α := value.current.vector

def finalVector (value : SpecSliceRef α) : SpecVector α := value.final.vector

def ofArrays {extent : Nat} (current final : SpecArray α extent) : SpecSliceRef α :=
  ⟨extent, current, final⟩

/-- A settled reference has the same current value and prophecy. -/
def settled (value : SpecVector α) : SpecSliceRef α :=
  ofArrays ⟨value, rfl⟩ ⟨value, rfl⟩

/-- Decode a pair of sequence readings into the fixed-extent carrier.
This checks an external representation, not a running loan's write-back. -/
def ofVectors? (current final : SpecVector α) : Option (SpecSliceRef α) :=
  if same : current.values.size = final.values.size then
    some (ofArrays ⟨current, rfl⟩ ⟨final, same.symm⟩)
  else none

@[simp] theorem currentVector_size (value : SpecSliceRef α) :
    value.currentVector.values.size = value.extent := value.current.size_eq

@[simp] theorem finalVector_size (value : SpecSliceRef α) :
    value.finalVector.values.size = value.extent := value.final.size_eq

theorem lengths_equal (value : SpecSliceRef α) :
    value.currentVector.values.size = value.finalVector.values.size :=
  value.current.size_eq.trans value.final.size_eq.symm

@[simp] theorem settled_currentVector (value : SpecVector α) :
    (settled value).currentVector = value := rfl

@[simp] theorem settled_finalVector (value : SpecVector α) :
    (settled value).finalVector = value := rfl

/-- Elementwise transport cannot change either extent. -/
def map (f : α → β) (value : SpecSliceRef α) : SpecSliceRef β :=
  ofArrays
    ⟨value.current.vector.map f, by simpa using value.current.size_eq⟩
    ⟨value.final.vector.map f, by simpa using value.final.size_eq⟩

@[simp] theorem map_currentVector (f : α → β) (value : SpecSliceRef α) :
    (value.map f).currentVector = value.currentVector.map f := rfl

@[simp] theorem map_finalVector (f : α → β) (value : SpecSliceRef α) :
    (value.map f).finalVector = value.finalVector.map f := rfl

@[simp] theorem map_extent (f : α → β) (value : SpecSliceRef α) :
    (value.map f).extent = value.extent := rfl

@[ext] theorem ext {left right : SpecSliceRef α}
    (current : left.currentVector = right.currentVector)
    (final : left.finalVector = right.finalVector) : left = right := by
  have extent : left.extent = right.extent := by
    rw [← currentVector_size left, ← currentVector_size right, current]
  cases left with
  | mk n a b =>
    cases right with
    | mk m c d =>
      cases extent
      have ha : a = c := SpecArray.ext current
      have hb : b = d := SpecArray.ext final
      cases ha
      cases hb
      rfl

@[simp] theorem ofVectors?_readings (value : SpecSliceRef α) :
    ofVectors? value.currentVector value.finalVector = some value := by
  simp only [ofVectors?, dif_pos value.lengths_equal]
  congr 1
  apply ext <;> rfl

theorem ofVectors?_sound {current final : SpecVector α} {value : SpecSliceRef α}
    (decoded : ofVectors? current final = some value) :
    value.currentVector = current ∧ value.finalVector = final := by
  unfold ofVectors? at decoded
  split at decoded
  · cases Option.some.inj decoded
    exact ⟨rfl, rfl⟩
  · cases decoded

theorem map_id (value : SpecSliceRef α) : value.map id = value := by
  apply ext <;> apply SpecVector.ext <;> simp [SpecVector.map]

theorem map_map (f : α → β) (g : β → γ) (value : SpecSliceRef α) :
    (value.map f).map g = value.map (g ∘ f) := by
  apply ext <;> apply SpecVector.ext <;> simp [SpecVector.map, Array.map_map]

/-- The current contents may be replaced only at the borrowed extent. -/
def withCurrent (value : SpecSliceRef α) (current : SpecArray α value.extent) :
    SpecSliceRef α := ofArrays current value.final

@[simp] theorem withCurrent_finalVector (value : SpecSliceRef α)
    (current : SpecArray α value.extent) :
    (value.withCurrent current).finalVector = value.finalVector := rfl

instance : Inhabited (SpecSliceRef α) := ⟨settled default⟩

end SpecSliceRef
end VerexIR
