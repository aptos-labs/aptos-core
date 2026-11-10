-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefSteps

/-!
# The reference analysis on generic forms

UNTRUSTED (checked by the kernel). The reference-safety analysis treats each
generic instruction form as its non-generic form at the entity its
instantiation names: `PackGeneric i` as `Pack` of the instantiation's
definition, `MutBorrowFieldGeneric i` as `MutBorrowField` of its field handle,
`CallGeneric i` as `Call` of its function handle, and so on; the type
arguments play no part (`Model/ReferenceSafety.lean`). Each lemma here turns
a successful step of a generic form into the same step of its counterpart,
so the analysis's facts about the non-generic forms carry over.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph ReferenceSafety

variable {m : CompiledModule} {view : FunctionView} {st st' : RefState} {i : Nat}

theorem instDef_run {d : Nat} {st₁ : RefState} :
    (instDef m i).run st = .ok (d, st₁) ↔
      (∃ si, m.structDefInstantiations[i]? = some si ∧ d = si.def_) ∧ st₁ = st := by
  unfold instDef
  rw [run_bind]
  constructor
  · rintro ⟨si, st₂, hl, hp⟩
    obtain ⟨hsi, rfl⟩ := lookup_run.1 hl
    obtain ⟨rfl, rfl⟩ := pure_run.1 hp
    exact ⟨⟨si, hsi, rfl⟩, rfl⟩
  · rintro ⟨⟨si, hsi, rfl⟩, rfl⟩
    exact ⟨si, _, lookup_run.2 ⟨hsi, rfl⟩, pure_run.2 ⟨rfl, rfl⟩⟩

theorem variantInstHandle_run {h : Nat} {st₁ : RefState} :
    (variantInstHandle m i).run st = .ok (h, st₁) ↔
      (∃ vi, m.structVariantInstantiations[i]? = some vi ∧ h = vi.handle) ∧ st₁ = st := by
  unfold variantInstHandle
  rw [run_bind]
  constructor
  · rintro ⟨vi, st₂, hl, hp⟩
    obtain ⟨hvi, rfl⟩ := lookup_run.1 hl
    obtain ⟨rfl, rfl⟩ := pure_run.1 hp
    exact ⟨⟨vi, hvi, rfl⟩, rfl⟩
  · rintro ⟨⟨vi, hvi, rfl⟩, rfl⟩
    exact ⟨vi, _, lookup_run.2 ⟨hvi, rfl⟩, pure_run.2 ⟨rfl, rfl⟩⟩

/-- A step that begins with a pure lookup is the rest of the step at the
looked-up entry. -/
theorem lookup_then_run {α β : Type} {t : Array α} {w : String} {k : α → RefM β} {b : β} :
    ((liftM (lookup t i w) : RefM α) >>= k).run st = .ok (b, st') ↔
      ∃ a, t[i]? = some a ∧ (k a).run st = .ok (b, st') := by
  rw [run_bind]
  constructor
  · rintro ⟨a, st₁, hl, hk⟩
    obtain ⟨ha, rfl⟩ := lookup_run.1 hl
    exact ⟨a, ha, hk⟩
  · rintro ⟨a, ha, hk⟩
    exact ⟨a, st, lookup_run.2 ⟨ha, rfl⟩, hk⟩

theorem exec_packGeneric' (h : (executeInner m view (.packGeneric i)).run st = .ok ((), st')) :
    ∃ si, m.structDefInstantiations[i]? = some si ∧
      (executeInner m view (.pack si.def_)).run st = .ok ((), st') := by
  simp only [executeInner, run_bind] at h ⊢
  obtain ⟨d, st₁, hd, n, st₂, hn, h⟩ := h
  obtain ⟨⟨si, hsi, rfl⟩, rfl⟩ := instDef_run.1 hd
  exact ⟨si, hsi, n, st₂, hn, h⟩

theorem exec_unpackGeneric' (h : (executeInner m view (.unpackGeneric i)).run st = .ok ((), st')) :
    ∃ si, m.structDefInstantiations[i]? = some si ∧
      (executeInner m view (.unpack si.def_)).run st = .ok ((), st') := by
  simp only [executeInner, run_bind] at h ⊢
  obtain ⟨d, st₁, hd, n, st₂, hn, h⟩ := h
  obtain ⟨⟨si, hsi, rfl⟩, rfl⟩ := instDef_run.1 hd
  exact ⟨si, hsi, n, st₂, hn, h⟩

theorem exec_packVariantGeneric'
    (h : (executeInner m view (.packVariantGeneric i)).run st = .ok ((), st')) :
    ∃ vi, m.structVariantInstantiations[i]? = some vi ∧
      (executeInner m view (.packVariant vi.handle)).run st = .ok ((), st') := by
  simp only [executeInner, run_bind] at h ⊢
  obtain ⟨hh, st₁, hd, n, st₂, hn, h⟩ := h
  obtain ⟨⟨vi, hvi, rfl⟩, rfl⟩ := variantInstHandle_run.1 hd
  exact ⟨vi, hvi, n, st₂, hn, h⟩

theorem exec_unpackVariantGeneric'
    (h : (executeInner m view (.unpackVariantGeneric i)).run st = .ok ((), st')) :
    ∃ vi, m.structVariantInstantiations[i]? = some vi ∧
      (executeInner m view (.unpackVariant vi.handle)).run st = .ok ((), st') := by
  simp only [executeInner, run_bind] at h ⊢
  obtain ⟨hh, st₁, hd, n, st₂, hn, h⟩ := h
  obtain ⟨⟨vi, hvi, rfl⟩, rfl⟩ := variantInstHandle_run.1 hd
  exact ⟨vi, hvi, n, st₂, hn, h⟩

theorem exec_testVariantGeneric'
    (h : (executeInner m view (.testVariantGeneric i)).run st = .ok ((), st')) :
    ∃ vi, m.structVariantInstantiations[i]? = some vi ∧
      (executeInner m view (.testVariant vi.handle)).run st = .ok ((), st') := by
  simp only [executeInner] at h ⊢
  rw [run_bind] at h
  obtain ⟨hh, st₁, hd, h⟩ := h
  obtain ⟨⟨vi, hvi, rfl⟩, rfl⟩ := variantInstHandle_run.1 hd
  exact ⟨vi, hvi, h⟩

theorem exec_mutBorrowFieldGeneric'
    (h : (executeInner m view (.mutBorrowFieldGeneric i)).run st = .ok ((), st')) :
    ∃ fi, m.fieldInstantiations[i]? = some fi ∧
      (executeInner m view (.mutBorrowField fi.handle)).run st = .ok ((), st') := by
  simp only [executeInner] at h ⊢
  exact lookup_then_run.1 h

theorem exec_immBorrowFieldGeneric'
    (h : (executeInner m view (.immBorrowFieldGeneric i)).run st = .ok ((), st')) :
    ∃ fi, m.fieldInstantiations[i]? = some fi ∧
      (executeInner m view (.immBorrowField fi.handle)).run st = .ok ((), st') := by
  simp only [executeInner] at h ⊢
  exact lookup_then_run.1 h

theorem exec_mutBorrowVariantFieldGeneric'
    (h : (executeInner m view (.mutBorrowVariantFieldGeneric i)).run st = .ok ((), st')) :
    ∃ fi, m.variantFieldInstantiations[i]? = some fi ∧
      (executeInner m view (.mutBorrowVariantField fi.handle)).run st = .ok ((), st') := by
  simp only [executeInner] at h ⊢
  exact lookup_then_run.1 h

theorem exec_immBorrowVariantFieldGeneric'
    (h : (executeInner m view (.immBorrowVariantFieldGeneric i)).run st = .ok ((), st')) :
    ∃ fi, m.variantFieldInstantiations[i]? = some fi ∧
      (executeInner m view (.immBorrowVariantField fi.handle)).run st = .ok ((), st') := by
  simp only [executeInner] at h ⊢
  exact lookup_then_run.1 h

theorem exec_callGeneric'
    (h : (executeInner m view (.callGeneric i)).run st = .ok ((), st')) :
    ∃ fi, m.functionInstantiations[i]? = some fi ∧
      (executeInner m view (.call fi.handle)).run st = .ok ((), st') := by
  simp only [executeInner] at h ⊢
  exact lookup_then_run.1 h

theorem exec_borrowGlobalGeneric' {mut_ : Bool}
    (h : (executeInner m view
      (if mut_ then .mutBorrowGlobalGeneric i else .immBorrowGlobalGeneric i)).run st =
        .ok ((), st')) :
    ∃ si, m.structDefInstantiations[i]? = some si ∧
      (executeInner m view
        (if mut_ then .mutBorrowGlobal si.def_ else .immBorrowGlobal si.def_)).run st =
          .ok ((), st') := by
  cases mut_ <;> simp only [Bool.false_eq_true, ↓reduceIte, executeInner, run_bind] at h ⊢ <;>
  · obtain ⟨u, st₁, hp, d, st₂, hd, h⟩ := h
    obtain ⟨⟨si, hsi, rfl⟩, rfl⟩ := instDef_run.1 hd
    exact ⟨si, hsi, u, _, hp, h⟩

theorem exec_moveFromGeneric'
    (h : (executeInner m view (.moveFromGeneric i)).run st = .ok ((), st')) :
    ∃ si, m.structDefInstantiations[i]? = some si ∧
      (executeInner m view (.moveFrom si.def_)).run st = .ok ((), st') := by
  simp only [executeInner, run_bind] at h ⊢
  obtain ⟨u, st₁, hp, d, st₂, hd, h⟩ := h
  obtain ⟨⟨si, hsi, rfl⟩, rfl⟩ := instDef_run.1 hd
  exact ⟨si, hsi, u, _, hp, h⟩

theorem exec_existsGeneric' (d : Nat) :
    (executeInner m view (.existsGeneric i)).run st = (executeInner m view (.exists_ d)).run st :=
  rfl

theorem exec_moveToGeneric' (d : Nat) :
    (executeInner m view (.moveToGeneric i)).run st = (executeInner m view (.moveTo d)).run st :=
  rfl

end MoveBytecode.Proofs
