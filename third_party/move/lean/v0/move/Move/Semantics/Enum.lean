-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Semantics.Spec

/-!
# Relational semantics of enum payload access through references

`&r.f` / `&mut r.f` with `r` a reference to an enum selects the payload
field `f` of the variants that have it; when the referent is any other
variant the VM fails.  `variantFieldSpec` is that selection (or the focus of
a mutable payload loan) as a `Spec`.
-/

namespace Move.Semantics

/-- The VM's failure when a payload field is selected through a reference to
a variant that does not have it: a runtime failure rather than a user abort,
modeled as the IR's runtime abort code. -/
def variantMismatch : Nat := 0

/-- Selecting a payload field through a reference: `value` — the field of the
referent — when `holds` (the referent is one of the variants that have the
field), the VM's variant-mismatch failure otherwise. -/
def variantFieldSpec (holds : Bool) (value : α) : Spec σ α where
  ok := fun initial result final => holds = true ∧ result = value ∧ final = initial
  aborts := fun _ code => holds = false ∧ code = variantMismatch

@[simp] theorem variantFieldSpec_eq_pure {holds : Bool} {value : α} (h : holds = true) :
    (variantFieldSpec holds value : Spec σ α) = Spec.pure value := by
  apply Spec.extensionality
  · funext initial result final
    simp [variantFieldSpec, Spec.pure, h]
  · funext initial code
    simp [variantFieldSpec, Spec.pure, h]
  · rfl

@[simp] theorem variantFieldSpec_eq_abort {holds : Bool} {value : α} (h : holds = false) :
    (variantFieldSpec holds value : Spec σ α) = Spec.abort variantMismatch := by
  apply Spec.extensionality
  · funext initial result final
    simp [variantFieldSpec, Spec.abort, h]
  · funext initial code
    simp [variantFieldSpec, Spec.abort, h]
  · rfl

@[simp] theorem total_variantFieldSpec (holds : Bool) (value : α) :
    Spec.Total (variantFieldSpec holds value : Spec σ α) := fun _ h => h.elim

end Move.Semantics
