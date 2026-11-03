-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Stability
import Proofs.RefCallComplete

/-!
# D4b″: definitions

UNTRUSTED (checked by the kernel). The one-step form of D4b″
(`StepsExclusive`), and what a frame's reference parameters keep while it
runs (`ParamChain`): they are in no local of the frame or a newer one, the
`&mut` ones are exclusive among them (`Compat`), and the global types they
are in are protected: a function of a frame at or above acquires one only in
a module active more than once (R3).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-- **D4b″ at each step**: for a `&mut` reference parameter `r` of a frame of
the state, an access of the step that is not through a reference derived from
`r` does not overlap a reference derived from `r` held before the step. -/
def StepsExclusive (P : Program) (N : NativeRel) (ν : NativeFlows) (s₀ : State)
    (Held : State → Shadow → Nat → Reference → Prop) : Prop :=
  ∀ s h E, Trace P N ν s₀ s h E → ∀ o post evs, TracedStep P N ν s h o post evs →
    ∀ a l via, Event.accessed a l via ∈ evs →
    ∀ fr ∈ s.frames, ∀ ps, RefSafety.params? E fr.id = some ps → ∀ r ∈ ps,
    ∀ rr par, Event.borrowed r rr par ∈ E → rr.mutable = true →
    (∀ v, via = some v → ¬ Derives E r v) →
    ∀ i ri, Held s h i ri → Derives E r i → a.overlaps l ri.location = false

/-- **What a frame's reference parameters `ps` keep** while frame `fr` runs
over the frames `sk` (it first): they are in no local of the frame or a newer
one; distinct ones are compatible (a `&mut` one overlaps no other); and the
global types they are in are among the types `T` the frame protects, which
its module activations account for. -/
structure ParamsOk (P : Program) (E : List Event) (fr : Frame) (sk : List (FunctionRef × Bool))
    (ps : List Nat) (T : StructId → Prop) : Prop where
  mem : ∀ p ∈ ps, p ∈ borrowedIds E
  below : ∀ p ∈ ps, ∀ rp par, Event.borrowed p rp par ∈ E →
    ∀ f i, rp.location.root = .local_ f i → f < fr.id
  glob : ∀ p ∈ ps, ∀ rp par, Event.borrowed p rp par ∈ E →
    ∀ a t, rp.location.root = .global a t → T t
  compat : ∀ p ∈ ps, ∀ q ∈ ps, p ≠ q → ∀ rp rq pp pq, Event.borrowed p rp pp ∈ E →
    Event.borrowed q rq pq ∈ E → Compat rp rq
  active : ∀ t, T t → 1 ≤ countActive P sk t.module
  acq : ∀ t, T t → acqTypes P fr.function t → 2 ≤ countActive P sk t.module

/-- **The parameter facts of the frames** `frs` (the current one first), the
first protecting `T`: each frame's parameters keep `ParamsOk`, the frames
below are older, and protect fewer types. -/
def ParamChain (P : Program) (E : List Event) : List Frame → (StructId → Prop) → Prop
  | [], _ => True
  | fr :: frs, T => ∃ ps, RefSafety.params? E fr.id = some ps ∧
      ParamsOk P E fr ((fr :: frs).map Frame.skel) ps T ∧ (∀ fr' ∈ frs, fr'.id < fr.id) ∧
      ∃ T' : StructId → Prop, (∀ t, T' t → T t) ∧ ParamChain P E frs T'

end MoveBytecode.Proofs
