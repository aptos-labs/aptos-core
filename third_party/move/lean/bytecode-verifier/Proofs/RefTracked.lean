-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Containment
import Proofs.RefAccess
import Proofs.RefSoundness
import Proofs.ApiGeneric

/-!
# The references the verifier tracks

UNTRUSTED (checked by the kernel). The reference-safety analysis follows the
references on each frame's operand stack and in the locals the availability
analysis has available (`Tracked`). A reference left in a local that is
available on one path into a join only is released there, and the analysis no
longer protects its location; such a reference cannot be used again, since
copying or moving a local needs it available. D4b′ for verified programs
follows from `mutableContained_of_steps` with this notion of held.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph

/-- Reference `r`, with identity `i`, is held by frame `fr`, whose shadow is
`g`, where the analysis tracks it: on the operand stack, or in a local the
availability analysis has available at the frame's position. -/
def TrackedIn (P : Program) (g : ShadowFrame) (fr : Frame) (i : Nat) (r : Reference) : Prop :=
  StackSlot g fr i r ∨ ∃ j, LocalSlot g fr j i r ∧ modelAvail P fr.function fr.pc j = .available

/-- The references the analysis tracks in state `s`, whose shadow is `h`. -/
def Tracked (P : Program) (s : State) (h : Shadow) (i : Nat) (r : Reference) : Prop :=
  ∃ g fr, (g, fr) ∈ h.frames.zip s.frames ∧ TrackedIn P g fr i r

/-- A tracked reference is in the frames. -/
theorem tracked_occ {P : Program} {s : State} {h : Shadow} (hfit : h.fits s = true) {i : Nat}
    {r : Reference} (ht : Tracked P s h i r) : (i, Value.ref r) ∈ framesOcc h.frames s.frames := by
  obtain ⟨g, fr, hz, ht⟩ := ht
  have hg : g.fits fr = true := by
    rw [shadow_fits] at hfit
    simp only [framesFit, Bool.and_eq_true, List.all_eq_true] at hfit
    exact hfit.1.2 _ hz
  have hslot : RefSlot g fr i r := by
    rcases ht with hs | ⟨j, hl, -⟩
    · exact hs.refSlot
    · exact hl.refSlot
  simp only [framesOcc, List.mem_flatMap]
  exact ⟨(g, fr), hz, refSlot_occ hg hslot⟩

/-- A traced step is a step of the semantics. -/
theorem tracedStep_rel {P : Program} {N : NativeRel} {ν : NativeFlows} {s : State} {h : Shadow}
    {o : Outcome} {post : ShadowPost} {evs : List Event} (hT : TracedStep P N ν s h o post evs) :
    StepRel P N s o := by
  cases hT with
  | core hstep _ => exact .core hstep
  | native hstep hN _ => exact .native hstep hN

/-- The states of a trace are reachable. -/
theorem trace_reachable {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ s : State}
    {h : Shadow} {evs : List Event} (hT : Trace P N ν s₀ s h evs) : Reachable P N s₀ s := by
  induction hT with
  | start => exact .refl
  | step _ hS ih => exact .step ih (tracedStep_rel hS)

end MoveBytecode.Proofs
