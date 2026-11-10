-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.LoanCorrespondenceTransport

namespace VerexIR.Proofs.Denote
open VerexIR.SemanticOperations

private theorem boundaryRowLoanIds_empty {values : List RuntimeValue}
    (empty : ∀ value ∈ values, boundaryLoanIds value = []) :
    boundaryRowLoanIds values = [] := by
  induction values with
  | nil => simp [boundaryRowLoanIds]
  | cons value rest ih =>
      rw [boundaryRowLoanIds, empty value (by simp)]
      exact ih fun next member => empty next (by simp [member])

theorem boundaryLoanIds_plain {value : RuntimeValue} (plain : Plain value) :
    boundaryLoanIds value = [] := by
  induction plain with
  | unit | bool | character | integer | address | signer | string | bytes | ghost | retired =>
      simp [boundaryLoanIds]
  | vector values fields ih | tuple values fields ih
  | nominal _ _ values fields ih | closure _ _ _ values fields ih =>
      simp only [boundaryLoanIds]
      exact boundaryRowLoanIds_empty fun value member => ih value (by simpa using member)

theorem LoanCorrespondence.returnedRow_plain (loans : LoanCorrespondence)
    {values : Array RuntimeValue} (plain : ∀ value ∈ values, Plain value) :
    loans.returnedRow values = #[] := by
  have empty : boundaryRowLoanIds values.toList = [] :=
    boundaryRowLoanIds_empty fun value member => boundaryLoanIds_plain (plain value (by simpa using member))
  simp [LoanCorrespondence.returnedRow, empty]

theorem LoanCorrespondence.returnedRow_singleBorrow (loans : LoanCorrespondence)
    {loan : Nat} {current final : RuntimeValue} (plain : Plain current)
    (found : exportedRaw? loan loans = some final) :
    loans.returnedRow #[.borrow loan current] = #[.borrow loan final] := by
  simp [LoanCorrespondence.returnedRow, boundaryRowLoanIds, boundaryLoanIds,
    boundaryLoanIds_plain plain, found]

theorem LoanCorrespondence.resolveReturned_plain (loans : LoanCorrespondence)
    {results : Array RuntimeValue} (plain : ∀ value ∈ results, Plain value)
    (value : RuntimeValue) :
    resolveReturnedBorrows (loans.returnedRow results) value =
      resolveReturnedBorrows results value := by
  rw [loans.returnedRow_plain plain, resolveReturnedBorrows_empty]
  exact (resolveReturnedBorrows_plain results plain value).symm

@[simp] theorem LoanCorrespondence.valid_nil : LoanCorrespondence.Valid [] := by
  simp [LoanCorrespondence.Valid]

@[simp] theorem LoanCorrespondence.settles_nil (results : Array RuntimeValue)
    (exports : List (Nat × RuntimeValue)) : LoanCorrespondence.Settles [] [] results exports := by
  simp [LoanCorrespondence.Settles]

@[simp] theorem LoanCorrespondence.returnedRow_nil (results : Array RuntimeValue) :
    LoanCorrespondence.returnedRow [] results = #[] := by
  simp [LoanCorrespondence.returnedRow, exportedRaw?]

theorem LoanCorrespondence.returnsStorage_plain (loans : LoanCorrespondence)
    {results : Array RuntimeValue} (plain : ∀ value ∈ results, Plain value)
    (exit : RuntimeState) : loans.ReturnsStorage results exit := by
  unfold LoanCorrespondence.ReturnsStorage
  rw [loans.returnedRow_plain plain, RuntimeState.resolveReturned_plain results plain,
    RuntimeState.resolveReturned_plain #[] (by simp)]

variable [Carriers]

theorem ResultShape.corresponds_refFree (loans : LoanCorrespondence) :
    (shape : ResultShape) → shape.row.refFree = true → (value : shape.carrier) →
    shape.corresponds loans value ((resultCodec shape).encode value)
  | .none, _, _ => rfl
  | .one τ, free, value => by
      simp only [ResultShape.row, NRow.refFree, Bool.and_true] at free
      exact ⟨τ.encode value, rfl, NTy.corresponds_refFree loans τ free value⟩

theorem ResultShape.encode_plain : (shape : ResultShape) → (value : shape.carrier) →
    ∀ raw ∈ (resultCodec shape).encode value, Plain raw
  | .none, _, _, member => by simp [resultCodec] at member
  | .one τ, value, raw, member => by
      have same : raw = τ.encode value := by simpa [resultCodec] using member
      subst raw
      exact NTy.encode_plain τ value

/-- Settling a flat reference makes its mapped returned row literally the
actual returned reference. -/
theorem LoanCorrespondence.returnedRow_ref_settled (loans : LoanCorrespondence)
    {σ : NTy} (free : σ.refFree = true) (value : (NTy.ref σ).carrier)
    (settled : value.1 = σ.forget value.2) {raw : RuntimeValue}
    (related : NTy.corresponds loans (.ref σ) value raw) :
    loans.returnedRow #[raw] = #[raw] := by
  obtain ⟨loan, current, final, rfl, found, currentRelated, finalRelated⟩ := related
  have currentEq :=
    (NTy.plainCorresponds_refFree_iff loans σ free value.1 current).mp currentRelated
  have finalEq := (NTy.corresponds_refFree_iff loans σ free value.2 final).mp finalRelated
  subst current
  subst final
  rw [← NTy.plainEncode_forget σ value.2, ← settled] at found
  exact loans.returnedRow_singleBorrow (NTy.plainEncode_plain σ value.1) found

theorem LoanCorrespondence.returnedRow_slice_settled (loans : LoanCorrespondence)
    {σ : NTy} (value : (NTy.sliceRef σ).carrier)
    (settled : value.currentVector = value.finalVector) {raw : RuntimeValue}
    (related : NTy.corresponds loans (.sliceRef σ) value raw) :
    loans.returnedRow #[raw] = #[raw] := by
  obtain ⟨loan, rfl, found⟩ := related
  rw [← settled] at found
  exact loans.returnedRow_singleBorrow (NTy.encode_plain (.vector σ) value.currentVector) found

theorem LoanCorrespondence.returnsStorage_refFree (loans : LoanCorrespondence)
    {τ : NTy} (free : τ.refFree = true) (value : τ.carrier) {raw : RuntimeValue}
    (related : τ.corresponds loans value raw) (exit : RuntimeState) :
    loans.ReturnsStorage #[raw] exit := by
  have encoded := (NTy.corresponds_refFree_iff loans τ free value raw).mp related
  subst raw
  exact loans.returnsStorage_plain (by
    intro current member
    have same : current = τ.encode value := by simpa using member
    subst current
    exact NTy.encode_plain τ value) exit

end VerexIR.Proofs.Denote
