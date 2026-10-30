-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.StepLemmas

/-!
# The lemmas of function values in step proofs

The built-in lemmas of `doc/plan/proof-language.md` ("Lemmas") that read a
function value's behavior: the frame lemma, whose conclusion a step matches
against its obligation (`lemma_stored_invariants_frame`).
-/

namespace VerexIR.Proofs.Denote.StepLemmas

/-- A call that changes one resource whose invariant every value satisfies,
the others kept, preserves the stored invariants: the frame of a function
value's invocation (`modifies_of`). -/
theorem stored_invariants_frame {unit : Validation.ValidatedUnit} [FunctionCarriers unit]
    {invariant : (resource : ResourceType) → resource.carrier unit → Prop}
    {initial final : Memory unit} {modified : ResourceType}
    (frame : ∀ resource, resource ≠ modified → final resource = initial resource)
    (trivial : ∀ value, invariant modified value)
    (holds : MemoryInvariants invariant initial) :
    MemoryInvariants invariant final := by
  intro resource key value read
  by_cases same : resource = modified
  · subst resource
    exact trivial _
  · apply holds resource key value
    simpa only [frame resource same] using read

end VerexIR.Proofs.Denote.StepLemmas
