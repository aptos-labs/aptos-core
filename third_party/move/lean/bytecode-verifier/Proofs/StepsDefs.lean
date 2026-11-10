-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Containment

/-!
# D4c and D4e at each step

UNTRUSTED (checked by the kernel). What the verifier guarantees of one step
for D4c and D4e, for a notion of held references (`HeldFacts`), as
`StepsContained` is for D4b′; and what a step that borrows a reference from
another establishes (`BorrowsInside`). `Proofs.Stability` derives D4c and D4e
from them.
-/

namespace MoveBytecode.Proofs

/-- **D4c at each step**: no write of a step overlaps an immutable reference
held before it, or borrowed by it. -/
def StepsStable (P : Program) (N : NativeRel) (ν : NativeFlows) (s₀ : State)
    (Held : State → Shadow → Nat → Reference → Prop) : Prop :=
  ∀ s h evs, Trace P N ν s₀ s h evs → ∀ o post evs', TracedStep P N ν s h o post evs' →
    ∀ l via, Event.accessed .write l via ∈ evs' → ∀ i r,
      (Held s h i r ∨ ∃ par, Event.borrowed i r par ∈ evs') →
      r.mutable = false → Access.write.overlaps l r.location = false

/-- **D4e at each step**: after a step that makes a local unavailable or ends
a frame, no reference held points into it. -/
def StepsDead (P : Program) (N : NativeRel) (ν : NativeFlows) (s₀ : State)
    (Held : State → Shadow → Nat → Reference → Prop) : Prop :=
  ∀ s h evs, Trace P N ν s₀ s h evs → ∀ s' h' evs',
    TracedStep P N ν s h (.running s') (.running h') evs' →
    ∀ e ∈ evs', ∀ ended, e.ends? = some ended → ∀ i r, Held s' h' i r →
      ended r.location.root = false

/-- A reference the events `evs` of a step borrow from another is inside the
other's location, and immutable when the other is; the step uses the other. -/
def BorrowsInside (evs : List Event) : Prop :=
  ∀ d rd t, Event.borrowed d rd (some t) ∈ evs → ∃ rt, Event.used t rt ∈ evs ∧
    rd.location.Inside rt.location ∧ (rt.mutable = false → rd.mutable = false)

end MoveBytecode.Proofs
