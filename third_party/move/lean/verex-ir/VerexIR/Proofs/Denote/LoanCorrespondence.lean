-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.ReturnedStorage

/-!
# Loan identity across a call boundary

One correspondence table spans entry values, exit values, and all nested
prophecies. Replacing a reference changes its loan identity, while the
original loan retains its own settlement obligation.
-/

namespace VerexIR.Proofs.Denote
open VerexIR.SemanticOperations

/-- Final referent values indexed by dynamic loan identity. -/
abbrev LoanCorrespondence := List (Nat × RuntimeValue)

/-- Synthetic returned borrows fill holes using the same correspondence at
all positions in the boundary. -/
def LoanCorrespondence.row (loans : LoanCorrespondence) : Array RuntimeValue :=
  (loans.map fun entry => RuntimeValue.borrow entry.1 entry.2).toArray

def LoanCorrespondence.resolve (loans : LoanCorrespondence)
    (value : RuntimeValue) : RuntimeValue :=
  resolveReturnedBorrows loans.row value

/-- A loan has one final referent. Final referents have no outstanding holes,
so filling one hole cannot introduce an order-dependent obligation. -/
def LoanCorrespondence.Valid (loans : LoanCorrespondence) : Prop :=
  (loans.map Prod.fst).Nodup ∧ ∀ entry ∈ loans, entry.2.holeless = true

private theorem sizeOf_toList_lt (elements : Array RuntimeValue) :
    sizeOf elements.toList < sizeOf elements := by
  cases elements with
  | mk toList => simp

mutual
/-- All live loan identities, including references nested inside references. -/
def boundaryLoanIds (value : RuntimeValue) : List Nat :=
  match value with
  | .borrow loan current => loan :: boundaryLoanIds current
  | .vector values | .tuple values => boundaryRowLoanIds values.toList
  | .nominal _ _ fields => boundaryRowLoanIds fields.toList
  | .closure _ _ _ captures => boundaryRowLoanIds captures.toList
  | _ => []
termination_by sizeOf value
  decreasing_by
    all_goals simp_wf
    all_goals try (first
      | (have := sizeOf_toList_lt values)
      | (have := sizeOf_toList_lt fields)
      | (have := sizeOf_toList_lt captures))
    all_goals omega

def boundaryRowLoanIds : List RuntimeValue → List Nat
  | [] => []
  | value :: rest => boundaryLoanIds value ++ boundaryRowLoanIds rest
termination_by values => sizeOf values
  decreasing_by all_goals (simp_wf; omega)
end

/-- Only loans actually returned by the callee may fill global storage holes,
including references nested inside another returned reference. -/
def LoanCorrespondence.returnedRow (loans : LoanCorrespondence)
    (results : Array RuntimeValue) : Array RuntimeValue :=
  ((boundaryRowLoanIds results.toList).filterMap fun loan =>
    (exportedRaw? loan loans).map (RuntimeValue.borrow loan)).toArray

/-- The settled boundary view observes the same exit storage as the actual
returned values. This is an internal decoding obligation, independent of a
choice of native memory. -/
def LoanCorrespondence.ReturnsStorage (loans : LoanCorrespondence)
    (results : Array RuntimeValue) (exit : RuntimeState) : Prop :=
  exit.resolveReturned (loans.returnedRow results) = exit.resolveReturned results

variable [Carriers]

mutual
/-- A native value and an actual runtime value use one shared loan map.
Each reference's current component follows its actual runtime referent;
its final component follows the entry for that very loan identity. -/
def NTy.corresponds (loans : LoanCorrespondence) :
    (τ : NTy) → τ.carrier → RuntimeValue → Prop
  | .ref σ, value, raw =>
      ∃ loan current final, raw = .borrow loan current ∧
        exportedRaw? loan loans = some final ∧
        σ.plainCorresponds loans value.1 current ∧ σ.corresponds loans value.2 final
  | .sliceRef σ, value, raw =>
      ∃ loan, raw = .borrow loan (NTy.encode (.vector σ) value.currentVector) ∧
        exportedRaw? loan loans = some (NTy.encode (.vector σ) value.finalVector)
  | .tuple fields, values, raw =>
      ∃ rows, raw = .tuple rows ∧ NRow.corresponds loans fields values rows.toList
  | .struct source _ fields, values, raw =>
      ∃ rows, raw = .nominal source none rows ∧
        HListPlain.corresponds loans fields values.val rows.toList
  | .enum source _ names rows _, value, raw =>
      variantPlainCorresponds loans source names rows value.val raw
  | .unit, value, raw => raw = NTy.encode .unit value
  | .bool, value, raw => raw = NTy.encode .bool value
  | .int width signed, value, raw => raw = NTy.encode (.int width signed) value
  | .num, value, raw => raw = NTy.encode .num value
  | .address, value, raw => raw = NTy.encode .address value
  | .signer, value, raw => raw = NTy.encode .signer value
  | .string, value, raw => raw = NTy.encode .string value
  | .bytes, value, raw => raw = NTy.encode .bytes value
  | .char, value, raw => raw = NTy.encode .char value
  | .vector element, value, raw => raw = NTy.encode (.vector element) value
  | .slice element, value, raw => raw = NTy.encode (.slice element) value
  | .array element length, value, raw => raw = NTy.encode (.array element length) value
  | .table key element, value, raw => raw = NTy.encode (.table key element) value
  | .ghost slot, value, raw => raw = NTy.encode (.ghost slot) value
  | .param index, value, raw => raw = NTy.encode (.param index) value
  | .function parameters shared results, value, raw =>
      raw = NTy.encode (.function parameters shared results) value
  | .nominal source arguments productive, value, raw =>
      raw = NTy.encode (.nominal source arguments productive) value
  | .observed referent, value, raw => raw = NTy.encode (.observed referent) value

def NRow.corresponds (loans : LoanCorrespondence) :
    (row : NRow) → HList row → List RuntimeValue → Prop
  | .nil, _, raw => raw = []
  | .cons τ rest, values, raw =>
      ∃ head tail, raw = head :: tail ∧ τ.corresponds loans values.1 head ∧
        NRow.corresponds loans rest values.2 tail

/-- A plain value and an actual runtime value use one shared loan map: the
layer a mutable reference's current lives at (`NTy.carrier`). -/
def NTy.plainCorresponds (loans : LoanCorrespondence) :
    (τ : NTy) → τ.plain → RuntimeValue → Prop
  | .ref σ, value, raw =>
      ∃ loan current final, raw = .borrow loan current ∧
        exportedRaw? loan loans = some final ∧
        σ.plainCorresponds loans value.1 current ∧ σ.plainCorresponds loans value.2 final
  | .sliceRef σ, value, raw =>
      ∃ loan, raw = .borrow loan (NTy.plainEncode (.vector σ) value.currentVector) ∧
        exportedRaw? loan loans = some (NTy.plainEncode (.vector σ) value.finalVector)
  | .tuple fields, values, raw =>
      ∃ rows, raw = .tuple rows ∧ HListPlain.corresponds loans fields values rows.toList
  | .struct source _ fields, values, raw =>
      ∃ rows, raw = .nominal source none rows ∧
        HListPlain.corresponds loans fields values rows.toList
  | .enum source _ names rows _, value, raw =>
      variantPlainCorresponds loans source names rows value raw
  | .unit, value, raw => raw = NTy.plainEncode .unit value
  | .bool, value, raw => raw = NTy.plainEncode .bool value
  | .int width signed, value, raw => raw = NTy.plainEncode (.int width signed) value
  | .num, value, raw => raw = NTy.plainEncode .num value
  | .address, value, raw => raw = NTy.plainEncode .address value
  | .signer, value, raw => raw = NTy.plainEncode .signer value
  | .string, value, raw => raw = NTy.plainEncode .string value
  | .bytes, value, raw => raw = NTy.plainEncode .bytes value
  | .char, value, raw => raw = NTy.plainEncode .char value
  | .vector element, value, raw => raw = NTy.plainEncode (.vector element) value
  | .slice element, value, raw => raw = NTy.plainEncode (.slice element) value
  | .array element length, value, raw => raw = NTy.plainEncode (.array element length) value
  | .table key element, value, raw => raw = NTy.plainEncode (.table key element) value
  | .ghost slot, value, raw => raw = NTy.plainEncode (.ghost slot) value
  | .param index, value, raw => raw = NTy.plainEncode (.param index) value
  | .function parameters shared results, value, raw =>
      raw = NTy.plainEncode (.function parameters shared results) value
  | .nominal source arguments productive, value, raw =>
      raw = NTy.plainEncode (.nominal source arguments productive) value
  | .observed referent, value, raw => raw = NTy.plainEncode (.observed referent) value

def HListPlain.corresponds (loans : LoanCorrespondence) :
    (row : NRow) → HListPlain row → List RuntimeValue → Prop
  | .nil, _, raw => raw = []
  | .cons τ rest, values, raw =>
      ∃ head tail, raw = head :: tail ∧ τ.plainCorresponds loans values.1 head ∧
        HListPlain.corresponds loans rest values.2 tail

def variantPlainCorresponds (loans : LoanCorrespondence) (source : StructHandle) :
    (names : List String) → (rows : NRows) → variantPlain names rows → RuntimeValue → Prop
  | _, .nil, value, _ => nomatch value
  | [], .cons _ _, value, _ => nomatch value
  | name :: _, .cons fields _, .inl values, raw =>
      ∃ rows, raw = .nominal source (some name) rows ∧
        HListPlain.corresponds loans fields values rows.toList
  | _ :: names, .cons _ rest, .inr later, raw =>
      variantPlainCorresponds loans source names rest later raw
end

def ResultShape.corresponds (loans : LoanCorrespondence) :
    (shape : ResultShape) → shape.carrier → Array RuntimeValue → Prop
  | .none, _, raw => raw = #[]
  | .one τ, value, raw => ∃ result, raw = #[result] ∧ τ.corresponds loans value result

mutual
/-- Reference-free values use their ordinary codec under any correspondence. -/
theorem NTy.corresponds_refFree (loans : LoanCorrespondence) : (τ : NTy) →
    τ.refFree = true → (value : τ.carrier) → τ.corresponds loans value (τ.encode value)
  | .ref _, free, _ | .sliceRef _, free, _ => by simp [NTy.refFree] at free
  | .tuple fields, free, values => by
      refine ⟨values.encode.toArray, rfl, ?_⟩
      simpa using NRow.corresponds_refFree loans fields free values
  | .struct source arguments fields, free, values => by
      refine ⟨(HListPlain.encode values.val).toArray, rfl, ?_⟩
      simpa using HListPlain.corresponds_refFree loans fields free values.val
  | .enum source arguments names rows distinct, free, value =>
      variantPlainCorresponds_refFree loans source arguments names rows distinct free value.val
  | .unit, _, _ | .bool, _, _ | .int _ _, _, _ | .num, _, _ | .address, _, _
  | .signer, _, _ | .string, _, _ | .bytes, _, _ | .char, _, _
  | .vector _, _, _ | .slice _, _, _ | .array _ _, _, _ | .table _ _, _, _
  | .ghost _, _, _ | .param _, _, _ | .function _ _ _, _, _ | .nominal _ _ _, _, _
  | .observed _, _, _ => rfl

theorem NRow.corresponds_refFree (loans : LoanCorrespondence) : (row : NRow) →
    row.refFree = true → (values : HList row) → NRow.corresponds loans row values values.encode
  | .nil, _, _ => rfl
  | .cons τ rest, free, values => by
      simp only [NRow.refFree, Bool.and_eq_true] at free
      exact ⟨_, _, rfl, NTy.corresponds_refFree loans τ free.1 values.1,
        NRow.corresponds_refFree loans rest free.2 values.2⟩

theorem NTy.plainCorresponds_refFree (loans : LoanCorrespondence) : (τ : NTy) →
    τ.refFree = true → (value : τ.plain) →
      τ.plainCorresponds loans value (τ.plainEncode value)
  | .ref _, free, _ | .sliceRef _, free, _ => by simp [NTy.refFree] at free
  | .tuple fields, free, values => by
      refine ⟨(HListPlain.encode values).toArray, rfl, ?_⟩
      simpa using HListPlain.corresponds_refFree loans fields free values
  | .struct source arguments fields, free, values => by
      refine ⟨(HListPlain.encode values).toArray, rfl, ?_⟩
      simpa using HListPlain.corresponds_refFree loans fields free values
  | .enum source arguments names rows distinct, free, value =>
      variantPlainCorresponds_refFree loans source arguments names rows distinct free value
  | .unit, _, _ | .bool, _, _ | .int _ _, _, _ | .num, _, _ | .address, _, _
  | .signer, _, _ | .string, _, _ | .bytes, _, _ | .char, _, _
  | .vector _, _, _ | .slice _, _, _ | .array _ _, _, _ | .table _ _, _, _
  | .ghost _, _, _ | .param _, _, _ | .function _ _ _, _, _ | .nominal _ _ _, _, _
  | .observed _, _, _ => rfl

theorem HListPlain.corresponds_refFree (loans : LoanCorrespondence) : (row : NRow) →
    row.refFree = true → (values : HListPlain row) →
      HListPlain.corresponds loans row values values.encode
  | .nil, _, _ => rfl
  | .cons τ rest, free, values => by
      simp only [NRow.refFree, Bool.and_eq_true] at free
      exact ⟨_, _, rfl, NTy.plainCorresponds_refFree loans τ free.1 values.1,
        HListPlain.corresponds_refFree loans rest free.2 values.2⟩

theorem variantPlainCorresponds_refFree (loans : LoanCorrespondence) (source : StructHandle)
    (arguments : NRow) : (names : List String) → (rows : NRows) →
    (distinct : names.Nodup) → rows.refFree = true → (value : variantPlain names rows) →
    variantPlainCorresponds loans source names rows value
      (NTy.plainEncode (.enum source arguments names rows distinct) value)
  | _, .nil, _, _, value => nomatch value
  | [], .cons _ _, _, _, value => nomatch value
  | name :: names, .cons fields rest, distinct, free, .inl values => by
      simp only [NRows.refFree, Bool.and_eq_true] at free
      refine ⟨(HListPlain.encode values).toArray, rfl, ?_⟩
      simpa using HListPlain.corresponds_refFree loans fields free.1 values
  | name :: names, .cons fields rest, distinct, free, .inr later => by
      simp only [NRows.refFree, Bool.and_eq_true] at free
      exact variantPlainCorresponds_refFree loans source arguments names rest (List.nodup_cons.mp distinct).2 free.2 later
end

/-- Every initially lent loan settles separately, or remains live at exit.
Exports are checked for every mapped identity, including newly minted loans.
Live mutable references cannot be duplicated across the exit boundary. -/
def LoanCorrespondence.Settles (loans : LoanCorrespondence)
    (initialLoans : List Nat) (results : Array RuntimeValue)
    (exports : List (Nat × RuntimeValue)) : Prop :=
  let boundary := results ++ (exports.map Prod.snd).toArray
  let live := boundaryRowLoanIds boundary.toList
  let resolve := resolveReturnedBorrows (loans.returnedRow boundary)
  (∀ entry ∈ loans,
    (entry.1 ∈ initialLoans ∨ entry.1 ∈ live ∨ entry.1 ∈ exports.map Prod.fst) ∧
    live.count entry.1 ≤ 1 ∧ (exports.map Prod.fst).count entry.1 ≤ 1 ∧
    match exportedRaw? entry.1 exports with
    | some raw => resolve raw = entry.2
    | none => True) ∧
  (∀ loan ∈ initialLoans,
    match exportedRaw? loan loans with
    | none => False
    | some final =>
        match exportedRaw? loan exports with
        | some raw => resolve raw = final
        | none => loan ∈ live)

/-- One shared correspondence covers initial arguments, returned values and
all separately settled original loans. -/
structure BoundaryCorrespondence (σs : NRow) (args : HList σs)
    (shape : ResultShape) (result : shape.carrier) (arguments results : Array RuntimeValue)
    (initialLoans : List Nat) (exports : List (Nat × RuntimeValue))
    (loans : LoanCorrespondence) : Prop where
  valid : loans.Valid
  arguments : NRow.corresponds loans σs args arguments.toList
  result : shape.corresponds loans result results
  settles : loans.Settles initialLoans results exports

end VerexIR.Proofs.Denote
