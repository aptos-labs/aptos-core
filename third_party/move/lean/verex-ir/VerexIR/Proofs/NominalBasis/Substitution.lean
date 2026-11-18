-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.NominalBasis.Heterogeneous

/-!
# The substitution law of a family's operations

Every operation a family gives a type parameter (`Carriers`) is, at the family
type arguments induce (`Carriers.instantiate`) and a type, heterogeneously the
operation at the outer family and the substituted type. The carrier's equation
(`NTy.carrier_instantiate`), the transport's identity (`NTy.toSkolem_heq`), the
encoding's and the inhabitant's laws (`NTy.encode_toSkolem`,
`NTy.inhabitant_instantiate`) are the kernel's; this module derives from them
the codec, the decidable equality, the order, the erasure and the typed reads.
-/

namespace VerexIR.Proofs.Denote.NominalBasis

open VerexIR VerexIR.Proofs.Denote

/-! ## Operations determined by the carrier and the encoding -/

section Operations
variable [Θ : Carriers] (θ : TypeArgs)

/-- The encoding at the induced family is the encoding of the substituted type. -/
theorem encode_instantiate (τ : NTy) :
    HEq (@NTy.encode ((Carriers.instantiate θ Θ)).toSorts τ) (@NTy.encode (Θ).toSorts (τ.subst θ.1)) :=
  heq_fun_of_transport (NTy.carrier_instantiate θ τ) (NTy.toSkolem θ τ)
    (NTy.toSkolem_heq θ τ) (NTy.encode_toSkolem θ τ)

/-- The codec at the induced family is the codec of the substituted type. -/
theorem codec_instantiate (τ : NTy) :
    HEq (@NTy.codec ((Carriers.instantiate θ Θ)).toSorts τ) (@NTy.codec (Θ).toSorts (τ.subst θ.1)) :=
  codec_heq (NTy.carrier_instantiate θ τ) (encode_instantiate θ τ)
    (@NTy.codec_tight ((Carriers.instantiate θ Θ)).toSorts τ) (@NTy.codec_tight (Θ).toSorts (τ.subst θ.1))

/-- The decoder at the induced family is the decoder of the substituted type. -/
theorem decode_instantiate (τ : NTy) :
    HEq (@NTy.codec ((Carriers.instantiate θ Θ)).toSorts τ).decode? (@NTy.codec (Θ).toSorts (τ.subst θ.1)).decode? := by
  have codecs := codec_instantiate θ τ
  have same := NTy.carrier_instantiate θ τ
  generalize @NTy.codec ((Carriers.instantiate θ Θ)).toSorts τ = left at codecs
  generalize @NTy.codec (Θ).toSorts (τ.subst θ.1) = right at codecs
  generalize @NTy.carrier ((Carriers.instantiate θ Θ)).toSorts τ = α at same left
  subst same
  cases codecs
  rfl

/-- The decidable equality at the induced family is the substituted type's. -/
theorem decEq_instantiate (τ : NTy) :
    HEq (@NTy.decEq ((Carriers.instantiate θ Θ)).toSorts τ) (@NTy.decEq (Θ).toSorts (τ.subst θ.1)) :=
  decEq_heq (NTy.carrier_instantiate θ τ) _ _

/-- The inhabitant at the induced family is the substituted type's. -/
theorem inhabitant_instantiate (τ : NTy) (inhabitable : τ.inhabitable = true)
    (substituted : (τ.subst θ.1).inhabitable = true) :
    HEq (@NTy.inhabitant ((Carriers.instantiate θ Θ)).toSorts τ inhabitable)
      (@NTy.inhabitant (Θ).toSorts (τ.subst θ.1) substituted) := by
  rw [NTy.inhabitant_instantiate θ τ inhabitable]
  exact NTy.toSkolem_heq θ τ _

/-- The named inhabitant at the induced family is the substituted type's. -/
theorem inhabitantOf_instantiate (τ : NTy) (inhabitable : τ.inhabitable = true)
    (substituted : (τ.subst θ.1).inhabitable = true) :
    HEq (@NTy.inhabitantOf ((Carriers.instantiate θ Θ)).toSorts τ inhabitable)
      (@NTy.inhabitantOf (Θ).toSorts (τ.subst θ.1) substituted) := by
  rw [NTy.inhabitantOf_instantiate θ τ inhabitable]
  exact NTy.toSkolem_heq θ τ _

/-- The typed order at the induced family is the substituted type's. -/
theorem typedOrder_instantiate (τ : NTy) :
    HEq (@NTy.typedOrder ((Carriers.instantiate θ Θ)).toSorts τ) (@NTy.typedOrder (Θ).toSorts (τ.subst θ.1)) :=
  typedOrder_heq (NTy.carrier_instantiate θ τ) (encode_instantiate θ τ) _ _

/-- The typed erasure at the induced family is the substituted type's. -/
theorem typedErasure_instantiate (τ : NTy) :
    HEq (@NTy.typedErasure ((Carriers.instantiate θ Θ)).toSorts τ) (@NTy.typedErasure (Θ).toSorts (τ.subst θ.1)) :=
  typedErasure_heq (NTy.carrier_instantiate θ τ) (encode_instantiate θ τ) _ _

/-- The structural order at the induced family is the substituted type's. -/
theorem order_instantiate (τ : NTy) :
    HEq (fun rank => @NTy.order ((Carriers.instantiate θ Θ)).toSorts rank τ)
      (fun rank => @NTy.order (Θ).toSorts rank (τ.subst θ.1)) := by
  have orders := typedOrder_instantiate θ τ
  have same := NTy.carrier_instantiate θ τ
  have encodes := encode_instantiate θ τ
  show HEq (@NTy.typedOrder ((Carriers.instantiate θ Θ)).toSorts τ).order
    (@NTy.typedOrder (Θ).toSorts (τ.subst θ.1)).order
  generalize @NTy.typedOrder ((Carriers.instantiate θ Θ)).toSorts τ = left at orders
  generalize @NTy.typedOrder (Θ).toSorts (τ.subst θ.1) = right at orders
  revert left right
  show ∀ (left : TypedOrder _ (@NTy.encode ((Carriers.instantiate θ Θ)).toSorts τ))
    (right : TypedOrder _ (@NTy.encode (Θ).toSorts (τ.subst θ.1))), HEq left right →
      HEq left.order right.order
  generalize @NTy.encode ((Carriers.instantiate θ Θ)).toSorts τ = encode at encodes
  generalize @NTy.encode (Θ).toSorts (τ.subst θ.1) = encode' at encodes
  revert encode
  generalize @NTy.carrier ((Carriers.instantiate θ Θ)).toSorts τ = α at same
  subst same
  intro encode encodes left right orders
  cases encodes
  cases orders
  rfl

end Operations

end VerexIR.Proofs.Denote.NominalBasis
