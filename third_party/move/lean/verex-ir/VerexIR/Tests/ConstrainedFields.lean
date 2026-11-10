-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Base

open VerexIR VerexIR.Proofs.Denote
namespace VerexIR.Tests.ConstrainedFields

-- A field equation exposes the row without opening its nominal invariant.
example [Carriers]
    (final : (NTy.struct ⟨⟨0⟩, 0⟩ .nil (.cons (.vector .address) .nil)).carrier)
    (input : (NTy.vector .address).plain)
    (write : (NTy.vector .address).plain → (NTy.vector .address).plain)
    (h : write input = final.val.1) (P : (NTy.vector .address).plain → Prop)
    (known : P (write input)) : P final.val.1 := by
  verex_subst_vars
  exact known

-- Without a field equation, the row's payload remains unknown.
example [Carriers]
    (final : (NTy.struct ⟨⟨0⟩, 0⟩ .nil (.cons (.vector .address) .nil)).carrier)
    (input : (NTy.vector .address).plain)
    (write : (NTy.vector .address).plain → (NTy.vector .address).plain)
    (P : (NTy.vector .address).plain → Prop) (known : P (write input)) : True := by
  fail_if_success
    have : P final.val.1 := by
      verex_subst_vars
      assumption
  trivial

-- A whole enum payload is substituted before its tag is inspected.
private def variantType : NTy :=
  .enum (⟨⟨0⟩, 0⟩ : StructHandle) .nil ["First", "Second"]
    (.cons .nil (.cons .nil .nil)) (by decide)

example [Carriers] (final : (NTy.carrier variantType)) (input : (NTy.plain variantType))
    (choose : (NTy.plain variantType) → (NTy.plain variantType))
    (h : final.val = choose input) (P : (NTy.plain variantType) → Prop)
    (known : P (choose input)) : P final.val := by
  verex_subst_vars
  exact known

private def resultShape : ResultShape := .one variantType

-- A call result keeps its declared shape until its payload is read.
example [Carriers] (final : (ResultShape.subst .nil resultShape).carrier)
    (input : (NTy.plain variantType))
    (choose : (NTy.plain variantType) → (NTy.plain variantType))
    (h : final.val = choose input) (P : (NTy.plain variantType) → Prop)
    (known : P (choose input)) : P final.val := by
  verex_subst_vars
  exact known

-- A result shape alone does not determine the variant payload.
example [Carriers] (final : (ResultShape.subst .nil resultShape).carrier)
    (input : (NTy.plain variantType))
    (choose : (NTy.plain variantType) → (NTy.plain variantType))
    (P : (NTy.plain variantType) → Prop) (known : P (choose input)) : True := by
  fail_if_success
    have : P final.val := by
      verex_subst_vars
      assumption
  trivial

-- Authored preparation retains the integer certificate carried by a stored
-- field, even though the local itself has the whole nominal carrier type.
example [Carriers]
    (stored : (NTy.struct ⟨⟨0⟩, 0⟩ .nil (.cons (.int 64 false) .nil)).carrier) :
    0 ≤ stored.val.1.val ∧ stored.val.1.val ≤ 2 ^ 64 - 1 := by
  verex_type_bounds
  omega

end VerexIR.Tests.ConstrainedFields
