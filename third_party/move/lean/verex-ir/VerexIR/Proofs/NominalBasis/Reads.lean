-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.NominalBasis.Substitution

/-!
# The substitution law of a type's typed reads

The typed reads of a type (`NTy.reads`) are not determined by its encoding: an
integer is read by its value, a type parameter by its family, a marked entry
as its referent, and every other type as itself. The law is proved by those
four cases. A type read as itself has typed reads whose data are a function of
its carrier, encoding, decoder, inhabitant and order, each of which has its
own law (`VerexIR.Proofs.NominalBasis.Substitution`).
-/

namespace VerexIR.Proofs.Denote.NominalBasis

open VerexIR VerexIR.Proofs.Denote

/-- Whether a type's typed values are its values themselves: every type but
an integer, a type parameter and a marked entry. -/
def viewsItself : NTy → Bool
  | .int _ _ | .num | .param _ | .observed _ => false
  | .unit | .bool | .address | .signer | .string | .bytes | .char
  | .tuple _ | .struct _ _ _ | .enum _ _ _ _ _ | .slice _ | .vector _ | .array _ _
  | .table _ _ | .ghost _ | .ref _ | .sliceRef _ | .function _ _ _ | .nominal _ _ _ => true

/-- A substitution keeps a type read as itself so. -/
theorem viewsItself_subst (θ : NRow) (τ : NTy) (views : viewsItself τ = true) :
    viewsItself (τ.subst θ) = true := by
  cases τ <;> first | rfl | cases views

/-- The data of typed reads: everything but the decidable equality, which its
type determines, and the laws. -/
structure ReadsData (α : Type) where
  typed : Type
  read : α → typed
  default : typed
  write : typed → RuntimeValue
  back : RuntimeValue → typed
  unread : typed → α
  compare : ValueRanks → typed → typed → Ordering

/-- The data of typed reads. -/
def readsData {α : Type} {encode : α → RuntimeValue} (reads : TypedReads α encode) :
    ReadsData α where
  typed := reads.typed
  read := reads.read
  default := reads.inhabited.default
  write := reads.write
  back := reads.back
  unread := reads.unread
  compare := fun rank => (reads.order rank).compare

/-- The data of the typed reads of a type read as itself. -/
def identityData (α : Type) (encode : α → RuntimeValue) (decode : RuntimeValue → Option α)
    (default : α) (compare : ValueRanks → α → α → Ordering) : ReadsData α where
  typed := α
  read := fun value => value
  default := default
  write := encode
  back := fun raw => (decode raw).getD default
  unread := fun value => value
  compare := compare

/-- Typed reads with the same data over the same carrier and encoding are the
same. -/
theorem typedReads_heq_of_data {α α' : Type} (same : α = α') {encode : α → RuntimeValue}
    {encode' : α' → RuntimeValue} (encodes : HEq encode encode')
    {left : TypedReads α encode} {right : TypedReads α' encode'}
    (data : HEq (readsData left) (readsData right)) : HEq left right := by
  subst same
  have sameData : readsData left = readsData right := eq_of_heq data
  obtain ⟨leftTyped, leftRead, leftDecEq, ⟨leftDefault⟩, leftWrite, leftBack, _, _, _,
    leftUnread, _, leftOrder, _⟩ := left
  obtain ⟨rightTyped, rightRead, rightDecEq, ⟨rightDefault⟩, rightWrite, rightBack, _, _, _,
    rightUnread, _, rightOrder, _⟩ := right
  simp only [readsData, ReadsData.mk.injEq] at sameData
  obtain ⟨typed, read, default, write, back, unread, compare⟩ := sameData
  subst typed
  refine typedReads_heq rfl encodes rfl read default write back unread ?_
  refine heq_of_eq (funext fun rank => ?_)
  exact eq_of_heq (keyOrder_heq rfl (heq_of_eq (congrFun (eq_of_heq compare) rank)))

/-- The data of a type read as itself depend on its carrier, encoding,
decoder, inhabitant and order alone. -/
theorem identityData_heq {α α' : Type} (same : α = α') {encode : α → RuntimeValue}
    {encode' : α' → RuntimeValue} (encodes : HEq encode encode')
    {decode : RuntimeValue → Option α} {decode' : RuntimeValue → Option α'}
    (decodes : HEq decode decode') {default : α} {default' : α'} (defaults : HEq default default')
    {compare : ValueRanks → α → α → Ordering} {compare' : ValueRanks → α' → α' → Ordering}
    (compares : HEq compare compare') :
    HEq (identityData α encode decode default compare)
      (identityData α' encode' decode' default' compare') := by
  subst same
  cases encodes; cases decodes; cases defaults; cases compares
  rfl

section Reads
variable [Θ : Carriers]

/-- The typed reads of a type read as itself. -/
theorem readsData_of_viewsItself (τ : NTy) (views : viewsItself τ = true)
    (inhabitable : τ.inhabitable = true) :
    readsData (NTy.reads τ inhabitable) =
      identityData τ.carrier (NTy.encode τ) τ.codec.decode? (NTy.inhabitantOf τ inhabitable)
        (fun rank => NTy.order rank τ) := by
  cases τ <;> first | rfl | cases views

variable (θ : TypeArgs)

/-- The law for a type read as itself. -/
theorem reads_instantiate_of_viewsItself (τ : NTy) (views : viewsItself τ = true)
    (inhabitable : τ.inhabitable = true) (substituted : (τ.subst θ.1).inhabitable = true) :
    HEq (@NTy.reads ((Carriers.instantiate θ Θ)).toSorts τ inhabitable)
      (@NTy.reads (Θ).toSorts (τ.subst θ.1) substituted) := by
  refine typedReads_heq_of_data (NTy.carrier_instantiate θ τ) (encode_instantiate θ τ) ?_
  rw [@readsData_of_viewsItself (Carriers.instantiate θ Θ) τ views inhabitable,
    @readsData_of_viewsItself Θ (τ.subst θ.1) (viewsItself_subst θ.1 τ views) substituted]
  exact identityData_heq (NTy.carrier_instantiate θ τ) (encode_instantiate θ τ)
    (decode_instantiate θ τ) (inhabitantOf_instantiate θ τ inhabitable substituted)
    (order_instantiate θ τ)

/-- The typed reads at the induced family are the substituted type's. -/
theorem reads_instantiate : (τ : NTy) → (inhabitable : τ.inhabitable = true) →
    (substituted : (τ.subst θ.1).inhabitable = true) →
    HEq (@NTy.reads ((Carriers.instantiate θ Θ)).toSorts τ inhabitable)
      (@NTy.reads (Θ).toSorts (τ.subst θ.1) substituted)
  | .int _ _, _, _ => HEq.rfl
  | .num, _, _ => HEq.rfl
  | .param _, _, _ => HEq.rfl
  | .observed referent, inhabitable, substituted =>
      reads_instantiate referent inhabitable substituted
  | .unit, i, s => reads_instantiate_of_viewsItself θ .unit rfl i s
  | .bool, i, s => reads_instantiate_of_viewsItself θ .bool rfl i s
  | .address, i, s => reads_instantiate_of_viewsItself θ .address rfl i s
  | .signer, i, s => reads_instantiate_of_viewsItself θ .signer rfl i s
  | .string, i, s => reads_instantiate_of_viewsItself θ .string rfl i s
  | .bytes, i, s => reads_instantiate_of_viewsItself θ .bytes rfl i s
  | .char, i, s => reads_instantiate_of_viewsItself θ .char rfl i s
  | .tuple elements, i, s => reads_instantiate_of_viewsItself θ (.tuple elements) rfl i s
  | .struct source arguments fields, i, s =>
      reads_instantiate_of_viewsItself θ (.struct source arguments fields) rfl i s
  | .enum source arguments names rows distinct, i, s =>
      reads_instantiate_of_viewsItself θ (.enum source arguments names rows distinct) rfl i s
  | .slice element, i, s => reads_instantiate_of_viewsItself θ (.slice element) rfl i s
  | .vector element, i, s => reads_instantiate_of_viewsItself θ (.vector element) rfl i s
  | .array element length, i, s =>
      reads_instantiate_of_viewsItself θ (.array element length) rfl i s
  | .table key value, i, s => reads_instantiate_of_viewsItself θ (.table key value) rfl i s
  | .ghost slot, i, s => reads_instantiate_of_viewsItself θ (.ghost slot) rfl i s
  | .ref referent, i, s => reads_instantiate_of_viewsItself θ (.ref referent) rfl i s
  | .sliceRef referent, i, s => reads_instantiate_of_viewsItself θ (.sliceRef referent) rfl i s
  | .function parameters shared results, i, s =>
      reads_instantiate_of_viewsItself θ (.function parameters shared results) rfl i s
  | .nominal source arguments kind, i, s =>
      reads_instantiate_of_viewsItself θ (.nominal source arguments kind) rfl i s

end Reads

end VerexIR.Proofs.Denote.NominalBasis
