-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.LoanCorrespondence

namespace VerexIR.Proofs.Denote

section RefFree
variable [Carriers]

omit [Carriers] in
private theorem readings_eq (rows : Array α) (values : List α) :
    rows.toList = values ↔ rows = values.toArray := by
  constructor
  · intro h; simpa using congrArg List.toArray h
  · intro h; simpa using congrArg Array.toList h

mutual
/-- A value of a reference-free type corresponds to a runtime value if and only
if that runtime value is its ordinary encoding. -/
theorem NTy.corresponds_refFree_iff (loans : LoanCorrespondence) :
    (τ : NTy) → τ.refFree = true → (value : τ.carrier) → (raw : RuntimeValue) →
    (τ.corresponds loans value raw ↔ raw = τ.encode value)
  | .ref _, free, _, _ | .sliceRef _, free, _, _ => by simp [NTy.refFree] at free
  | .tuple fields, free, values, raw => by
      simp only [NTy.corresponds, NTy.refFree] at *
      simp [NRow.corresponds_refFree_iff loans fields free, NTy.encode_tuple, readings_eq]
  | .struct source arguments fields, free, values, raw => by
      simp only [NTy.corresponds, NTy.refFree] at *
      simp [NRow.corresponds_refFree_iff loans fields free, NTy.encode_struct, readings_eq]
  | .enum source arguments names rows distinct, free, value, raw =>
      variantCorresponds_refFree_iff loans source arguments names rows distinct free value raw
  | .unit, _, _, _ | .bool, _, _, _ | .int _ _, _, _, _ | .num, _, _, _ | .address, _, _, _
  | .signer, _, _, _ | .string, _, _, _ | .bytes, _, _, _ | .char, _, _, _ | .vector _, _, _, _
  | .slice _, _, _, _ | .array _ _, _, _, _ | .table _ _, _, _, _ | .ghost _, _, _, _
  | .param _, _, _, _ | .function _ _ _, _, _, _ | .nominal _ _ _, _, _, _
  | .observed _, _, _, _ => Iff.rfl

theorem NRow.corresponds_refFree_iff (loans : LoanCorrespondence) :
    (row : NRow) → row.refFree = true → (values : HList row) → (raw : List RuntimeValue) →
    (NRow.corresponds loans row values raw ↔ raw = HList.encode values)
  | .nil, _, _, raw => Iff.rfl
  | .cons τ rest, free, values, raw => by
      simp only [NRow.refFree, Bool.and_eq_true] at free
      simp [NRow.corresponds, NTy.corresponds_refFree_iff loans τ free.1,
        NRow.corresponds_refFree_iff loans rest free.2, HList.encode_cons]

theorem variantCorresponds_refFree_iff (loans : LoanCorrespondence)
    (source : StructHandle) (arguments : NRow) : (names : List String) → (rows : NRows) →
    (distinct : names.Nodup) → rows.refFree = true →
    (value : variantCarrier names rows) → (raw : RuntimeValue) →
    (variantCorresponds loans source names rows value raw ↔
      raw = NTy.encode (.enum source arguments names rows distinct) value)
  | _, .nil, _, _, value, _ => nomatch value
  | [], .cons _ _, _, _, value, _ => nomatch value
  | name :: names, .cons fields rest, distinct, free, .inl values, raw => by
      simp only [NRows.refFree, Bool.and_eq_true] at free
      simp [variantCorresponds, NRow.corresponds_refFree_iff loans fields free.1,
        NTy.encode_enum_inl, readings_eq]
  | name :: names, .cons fields rest, distinct, free, .inr later, raw => by
      simp only [NRows.refFree, Bool.and_eq_true] at free
      simp only [variantCorresponds, NTy.encode_enum_inr]
      exact variantCorresponds_refFree_iff loans source arguments names rest _ free.2
        later raw
end

end RefFree

section Transport
variable [Θ : Carriers]

private theorem getD_refFree : (row : NRow) → row.refFree = true → (index : Nat) →
    (default : NTy) → default.refFree = true → (row.getD index default).refFree = true
  | .nil, _, _, _, free => free
  | .cons _ _, free, 0, _, _ => by
      simp only [NRow.refFree, Bool.and_eq_true] at free
      exact free.1
  | .cons _ rest, free, index + 1, default, defaultFree => by
      simp only [NRow.refFree, Bool.and_eq_true] at free
      exact getD_refFree rest free.2 index default defaultFree

mutual
theorem NTy.corresponds_toSkolem (θ : TypeArgs) (free : θ.1.refFree = true)
    (loans : LoanCorrespondence) : (τ : NTy) → (value : (τ.subst θ.1).carrier) →
    (raw : RuntimeValue) →
    (@NTy.corresponds (Carriers.instantiate θ Θ) loans τ (NTy.toSkolem θ τ value) raw ↔
      NTy.corresponds loans (τ.subst θ.1) value raw)
  | .ref τ, value, raw => by
      simp only [NTy.corresponds, NTy.subst, NTy.toSkolem,
        NTy.corresponds_toSkolem θ free loans τ]
  | .sliceRef τ, value, raw => by
      have current := NTy.encode_toSkolem θ (.vector τ) value.currentVector
      have final := NTy.encode_toSkolem θ (.vector τ) value.finalVector
      simp only [NTy.toSkolem, NTy.subst] at current final
      simp only [NTy.corresponds, NTy.subst, NTy.toSkolem,
        SpecSliceRef.map_currentVector, SpecSliceRef.map_finalVector, SpecVector.map,
        current, final]
  | .tuple fields, values, raw => by
      simp only [NTy.corresponds, NTy.subst, NTy.toSkolem,
        NRow.corresponds_toSkolem θ free loans fields]
  | .struct source arguments fields, values, raw => by
      simp only [NTy.corresponds, NTy.subst, NTy.toSkolem,
        NRow.corresponds_toSkolem θ free loans fields]
  | .enum source arguments names rows distinct, value, raw => by
      simp only [NTy.corresponds, NTy.subst, NTy.toSkolem]
      exact variantCorresponds_toSkolem θ free loans source names rows value raw
  | .param index, value, raw => by
      simp only [NTy.subst]
      rw [NTy.corresponds_refFree_iff loans _
        ((NTy.refFree_unmark _).trans (getD_refFree θ.1 free index (.param index) rfl))]
      change raw = @NTy.encode (Carriers.instantiate θ Θ) (.param index)
        (NTy.toSkolem θ (.param index) value) ↔ _
      rw [NTy.encode_toSkolem]
  | .unit, value, raw | .bool, value, raw | .int _ _, value, raw | .num, value, raw
  | .address, value, raw | .signer, value, raw | .string, value, raw | .bytes, value, raw
  | .char, value, raw | .vector _, value, raw | .slice _, value, raw | .array _ _, value, raw
  | .table _ _, value, raw | .ghost _, value, raw | .function _ _ _, value, raw
  | .nominal _ _ _, value, raw | .observed _, value, raw => by
      simp only [NTy.corresponds, NTy.subst, NTy.encode_toSkolem]

theorem NRow.corresponds_toSkolem (θ : TypeArgs) (free : θ.1.refFree = true)
    (loans : LoanCorrespondence) : (row : NRow) → (values : HList (NRow.subst θ.1 row)) →
    (raw : List RuntimeValue) →
    (@NRow.corresponds (Carriers.instantiate θ Θ) loans row (HList.toSkolem θ row values) raw ↔
      NRow.corresponds loans (NRow.subst θ.1 row) values raw)
  | .nil, _, _ => Iff.rfl
  | .cons τ rest, values, raw => by
      simp only [NRow.corresponds, NRow.subst, HList.toSkolem,
        NTy.corresponds_toSkolem θ free loans τ, NRow.corresponds_toSkolem θ free loans rest]

theorem variantCorresponds_toSkolem (θ : TypeArgs) (free : θ.1.refFree = true)
    (loans : LoanCorrespondence) (source : StructHandle) :
    (names : List String) → (rows : NRows) → (value : variantCarrier names (rows.subst θ.1)) →
    (raw : RuntimeValue) →
    (@variantCorresponds (Carriers.instantiate θ Θ) loans source names rows
      (variantCarrier.toSkolem θ names rows value) raw ↔
      variantCorresponds loans source names (rows.subst θ.1) value raw)
  | _, .nil, value, _ => nomatch value
  | [], .cons _ _, value, _ => nomatch value
  | _ :: _, .cons fields _, .inl values, raw => by
      simp only [variantCorresponds, variantCarrier.toSkolem, NRows.subst,
        NRow.corresponds_toSkolem θ free loans fields]
  | _ :: names, .cons _ rest, .inr later, raw => by
      simp only [variantCorresponds, variantCarrier.toSkolem, NRows.subst]
      exact variantCorresponds_toSkolem θ free loans source names rest later raw
end

end Transport
end VerexIR.Proofs.Denote
