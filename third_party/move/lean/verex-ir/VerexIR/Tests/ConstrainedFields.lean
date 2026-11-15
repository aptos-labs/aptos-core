-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Base

open VerexIR VerexIR.Proofs.Denote
namespace VerexIR.Tests.ConstrainedFields

-- Negative: Without a field equation, even with explicit `NTy.Holds`, the row's payload remains unknown.
set_option linter.unusedVariables false in
example [Carriers]
    (final : (NTy.struct ⟨⟨0⟩, 0⟩ .nil (.cons (.vector .address) .nil)).carrier)
    (hFinal : NTy.Holds (.struct ⟨⟨0⟩, 0⟩ .nil (.cons (.vector .address) .nil)) final)
    (input : (NTy.vector .address).carrier)
    (write : (NTy.vector .address).carrier → (NTy.vector .address).carrier)
    (P : (NTy.vector .address).carrier → Prop) (known : P (write input)) : True := by
  fail_if_success
    have : P final.1 := by
      verex_subst_vars
      assumption
  trivial

-- Positive: A field equation exposes the plain row via `verex_subst_vars` without opening `NTy.Holds`.
set_option linter.unusedVariables false in
example [Carriers]
    (final : (NTy.struct ⟨⟨0⟩, 0⟩ .nil (.cons (.vector .address) .nil)).carrier)
    (hFinal : NTy.Holds (.struct ⟨⟨0⟩, 0⟩ .nil (.cons (.vector .address) .nil)) final)
    (input : (NTy.vector .address).carrier)
    (write : (NTy.vector .address).carrier → (NTy.vector .address).carrier)
    (h : write input = final.1) (P : (NTy.vector .address).carrier → Prop)
    (known : P (write input)) : P final.1 := by
  verex_subst_vars
  exact known

private def variantType : NTy :=
  .enum (⟨⟨0⟩, 0⟩ : StructHandle) .nil ["First", "Second"]
    (.cons .nil (.cons .nil .nil)) (by decide)

-- Negative: `NTy.Holds` on an enum value alone does not determine the variant payload.
set_option linter.unusedVariables false in
example [Carriers] (final input : variantType.carrier) (hFinal : NTy.Holds variantType final)
    (choose : variantType.carrier → variantType.carrier)
    (P : variantType.carrier → Prop) (known : P (choose input)) : True := by
  fail_if_success
    have : P final := by
      verex_subst_vars
      assumption
  trivial

-- Positive: A whole enum value is substituted before its tag is inspected.
set_option linter.unusedVariables false in
example [Carriers] (final input : variantType.carrier) (hFinal : NTy.Holds variantType final)
    (choose : variantType.carrier → variantType.carrier)
    (h : final = choose input) (P : variantType.carrier → Prop)
    (known : P (choose input)) : P final := by
  verex_subst_vars
  exact known

private def resultShape : ResultShape := .one variantType

-- Negative: A result shape and `ResultShape.Holds` alone do not determine the variant payload.
set_option linter.unusedVariables false in
example [Carriers] (final : (ResultShape.subst .nil resultShape).carrier)
    (hFinal : ResultShape.Holds (ResultShape.subst .nil resultShape) final)
    (input : variantType.carrier) (choose : variantType.carrier → variantType.carrier)
    (P : variantType.carrier → Prop) (known : P (choose input)) : True := by
  fail_if_success
    have : P final := by
      verex_subst_vars
      assumption
  trivial

-- Positive: A call result keeps its declared shape and substitutes when equated.
set_option linter.unusedVariables false in
example [Carriers] (final : (ResultShape.subst .nil resultShape).carrier)
    (hFinal : ResultShape.Holds (ResultShape.subst .nil resultShape) final)
    (input : variantType.carrier) (choose : variantType.carrier → variantType.carrier)
    (h : final = choose input) (P : variantType.carrier → Prop)
    (known : P (choose input)) : P final := by
  verex_subst_vars
  exact known

private abbrev u64Struct : NTy := .struct ⟨⟨0⟩, 0⟩ .nil (.cons (.int 64 false) .nil)

-- Negative: `verex_type_bounds` supplies only the `u64` width bounds, not a stricter bound.
set_option linter.unusedVariables false in
example [Carriers] (stored : u64Struct.carrier) (hStored : NTy.Holds u64Struct stored) : True := by
  fail_if_success
    have : stored.1.val < 2 ^ 64 - 1 := by
      verex_type_bounds
      omega
  trivial

-- Positive: Preparation retains the unsigned `u64` certificate carried by a stored `SpecInt` field.
set_option linter.unusedVariables false in
example [Carriers] (stored : u64Struct.carrier) (hStored : NTy.Holds u64Struct stored) :
    0 ≤ stored.1.val ∧ stored.1.val ≤ 2 ^ 64 - 1 := by
  verex_type_bounds
  omega

end VerexIR.Tests.ConstrainedFields
