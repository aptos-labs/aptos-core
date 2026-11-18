-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Types

/-!
# Heterogeneous equality of a family's operations

A family's operations at two types whose carriers are equal types are compared
heterogeneously. The lemmas here reduce such a comparison to the comparison of
the carrier and of the encoding wherever an operation is determined by them:
a tight codec by its encoder, a typed order and a typed erasure by the law
that ties them to the encoding, a decidable equality by its type.
-/

namespace VerexIR.Proofs.Denote.NominalBasis

open VerexIR VerexIR.Proofs.Denote

universe u v

/-- Dependent functions that agree pointwise, heterogeneously, are equal
heterogeneously. -/
theorem heq_pi {ι : Sort u} {β γ : ι → Sort v} {f : ∀ i, β i} {g : ∀ i, γ i}
    (pointwise : ∀ i, HEq (f i) (g i)) : HEq f g := by
  have same : β = γ := funext fun i => type_eq_of_heq (pointwise i)
  subst same
  exact heq_of_eq (funext fun i => eq_of_heq (pointwise i))

/-- Functions out of equal types that agree along a heterogeneously
identical map of their arguments are equal heterogeneously. -/
theorem heq_fun_of_transport {α α' : Type} {β : Type u} (same : α = α')
    {f : α → β} {g : α' → β} (move : α' → α) (identity : ∀ x, HEq (move x) x)
    (agree : ∀ x, f (move x) = g x) : HEq f g := by
  subst same
  refine heq_of_eq (funext fun x => ?_)
  have moved : move x = x := eq_of_heq (identity x)
  rw [← agree x, moved]

/-- A statement about a term at an index holds heterogeneously at an equal
index. -/
theorem heq_of_index_eq {ι : Sort u} {β : ι → Sort v} (f : ∀ i, β i) {i j : ι}
    (same : i = j) : HEq (f i) (f j) := by
  subst same; rfl

/-- Two decidable equalities of equal types are equal heterogeneously. -/
theorem decEq_heq {α α' : Type} (same : α = α') (left : DecidableEq α)
    (right : DecidableEq α') : HEq left right := by
  subst same
  exact heq_of_eq (Subsingleton.elim left right)

/-- A tight codec is determined by its encoder. -/
theorem codec_heq {α α' : Type} (same : α = α') {left : Codec α RuntimeValue}
    {right : Codec α' RuntimeValue} (encode : HEq left.encode right.encode)
    (leftTight : left.Tight) (rightTight : right.Tight) : HEq left right := by
  subst same
  obtain ⟨leftEncode, leftDecode, leftLaw⟩ := left
  obtain ⟨rightEncode, rightDecode, rightLaw⟩ := right
  have sameEncode : leftEncode = rightEncode := eq_of_heq encode
  subst sameEncode
  have sameDecode : leftDecode = rightDecode := by
    funext raw
    cases decoded : leftDecode raw with
    | some value =>
        have encoded : leftEncode value = raw := leftTight raw value decoded
        rw [← encoded]
        exact (rightLaw value).symm
    | none =>
        cases other : rightDecode raw with
        | none => rfl
        | some value =>
            have encoded : leftEncode value = raw := rightTight raw value other
            rw [← encoded, leftLaw value] at decoded
            cases decoded
  subst sameDecode
  rfl

/-- A typed order is determined by the encoding it orders by. -/
theorem typedOrder_heq {α α' : Type} (same : α = α') {encode : α → RuntimeValue}
    {encode' : α' → RuntimeValue} (encodes : HEq encode encode')
    (left : TypedOrder α encode) (right : TypedOrder α' encode') : HEq left right := by
  subst same
  have sameEncode : encode = encode' := eq_of_heq encodes
  subst sameEncode
  obtain ⟨leftOrder, leftLaw⟩ := left
  obtain ⟨rightOrder, rightLaw⟩ := right
  have sameOrder : leftOrder = rightOrder := by
    funext rank first second
    rw [← leftLaw, ← rightLaw]
  subst sameOrder
  rfl

/-- A typed erasure is determined by the encoding it compares. -/
theorem typedErasure_heq {α α' : Type} (same : α = α') {encode : α → RuntimeValue}
    {encode' : α' → RuntimeValue} (encodes : HEq encode encode')
    (left : TypedErasure α encode) (right : TypedErasure α' encode') : HEq left right := by
  subst same
  have sameEncode : encode = encode' := eq_of_heq encodes
  subst sameEncode
  obtain ⟨leftSame, leftLaw⟩ := left
  obtain ⟨rightSame, rightLaw⟩ := right
  have sameSame : leftSame = rightSame := by
    funext first second
    rw [leftLaw, rightLaw]
  subst sameSame
  rfl

/-- A key order is determined by its comparison. -/
theorem keyOrder_heq {κ κ' : Type} (same : κ = κ') {left : SpecMap.KeyOrder κ}
    {right : SpecMap.KeyOrder κ'} (compare : HEq left.compare right.compare) :
    HEq left right := by
  subst same
  obtain ⟨leftCompare, _, _⟩ := left
  obtain ⟨rightCompare, _, _⟩ := right
  have sameCompare : leftCompare = rightCompare := eq_of_heq compare
  subst sameCompare
  rfl

/-- Typed reads are equal heterogeneously where their carriers, encodings and
data are: the typed values, the read, the default, the write and its inverse,
the inverse of the read, and the order. The decidable equality and the laws
follow. -/
theorem typedReads_heq {α α' : Type} (same : α = α') {encode : α → RuntimeValue}
    {encode' : α' → RuntimeValue} (encodes : HEq encode encode')
    {left : TypedReads α encode} {right : TypedReads α' encode'}
    (typed : left.typed = right.typed) (read : HEq left.read right.read)
    (default : HEq left.inhabited.default right.inhabited.default)
    (write : HEq left.write right.write) (back : HEq left.back right.back)
    (unread : HEq left.unread right.unread) (order : HEq left.order right.order) :
    HEq left right := by
  subst same
  have sameEncode : encode = encode' := eq_of_heq encodes
  subst sameEncode
  obtain ⟨leftTyped, leftRead, leftDecEq, ⟨leftDefault⟩, leftWrite, leftBack, _, _, _,
    leftUnread, _, leftOrder, _⟩ := left
  obtain ⟨rightTyped, rightRead, rightDecEq, ⟨rightDefault⟩, rightWrite, rightBack, _, _, _,
    rightUnread, _, rightOrder, _⟩ := right
  dsimp only at typed read default write back unread order
  subst typed
  cases read; cases default; cases write; cases back; cases unread; cases order
  have sameDecEq : leftDecEq = rightDecEq := Subsingleton.elim _ _
  subst sameDecEq
  rfl

end VerexIR.Proofs.Denote.NominalBasis
