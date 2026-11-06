-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefTrackedFacts
import Proofs.RefContainTop
import Proofs.RefContainBelow

/-!
# D4b′ for verified programs

UNTRUSTED (checked by the kernel). D4b′ (`MutableContained`) for programs the
model accepts, at milestone M3: no access reaches over a `&mut` reference
still to be used, from an admissible initial state, whatever the natives do
within their contracts. The references the analysis tracks are held in the
sense of `HeldFacts` (`heldFacts_tracked`); no access of a step reaches over
a tracked `&mut` of the current frame (`contained_top`: the analysis checked
the reference read or written through readable or writable, the local
copied, moved or stored unborrowed, the resource moved out unborrowed) or of
a suspended frame (`contained_below`: what the frames above may access
through their references is inside what their callers lent them, or their
own); and a step that borrows only inspects. `mutableContained_of_steps`
assembles them.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph

/-- A native call accesses nothing. -/
theorem nativeEvents_not_accessed {ν : NativeFlows} {f : FunctionRef} {args vals : List Value}
    {g : ShadowFrame} {m : ModuleId} {fresh : Nat} {a : Access} {l : Location}
    {via : Option Nat} : Event.accessed a l via ∉ nativeEvents ν f args vals g m fresh := by
  intro hb
  simp only [nativeEvents, List.mem_append] at hb
  rcases hb with hb | hb
  · obtain ⟨_, _, _, _, he, -⟩ := consumedFrom_mem _ hb
    cases he
  · obtain ⟨_, _, _, he⟩ := (nativeResults_spec ν f args vals _ m fresh).2.2 _ hb
    cases he

/-- **No access of a verified step reaches over a tracked `&mut`**, nor over
a reference the step borrows. -/
theorem stepsContained_tracked {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}
    (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.SupportedNonGeneric) (hm3 : P.M3)
    (hN : NativesRespect P N) (h0 : AdmissibleInitial P s₀) :
    StepsContained P N ν s₀ (Tracked P) := by
  intro s h E hT o post evs hS a l via ha i r hheld hm
  have hI := refTraceInv_trace h0 hT
  have hinv := invM3_reachable hwf hs hm3 (certified_of_wellFormed schedulerSound hwf hs) hN h0 s
    (trace_reachable hT)
  cases hS with
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
    case inr =>
      rw [nativeShape?_nonGeneric hs] at hshape
      cases hshape
    obtain ⟨rfl, rfl, rfl, g, hs', info, -, -, -, -, hevs, -⟩ := nativeStep_some hstep hshape hn
    rw [hevs] at ha
    exact absurd ha nativeEvents_not_accessed
  | core hstep hc =>
    obtain ⟨fr, rest, info, code, instr, hsf, -, -, -, hex⟩ := coreStep_exec hc
    rcases hheld with ⟨g', fr', hz, ht⟩ | ⟨par, j, k', hbj, -, -⟩
    · cases hfr : h.frames with
      | nil => rw [hfr] at hz; cases hz
      | cons g hs' =>
        rw [hfr, hsf, List.zip_cons_cons, List.mem_cons] at hz
        rcases hz with he | hz
        · simp only [Prod.mk.injEq] at he
          obtain ⟨rfl, rfl⟩ := he
          exact contained_top hwf (supported_of_nonGeneric hwf hs) hm3.apiSigs hinv hI.pre hI.typed hstep hc hfr hsf ha ht hm
        · exact contained_below hwf hs hm3 hinv hI.pre hI.typed hstep hc hfr hsf ha hz ht hm
    · rw [exec_borrow_inspect hex (List.mem_of_getElem? hbj) ha]
      cases via <;> rfl

/-- **D4b′ for M3 programs**: `MutableContained` for the programs the model
accepts, on the supported subset, whose function handles are no struct-API
mutable borrows. -/
def MutableContainedM3 (V : VerifierInterface) : Prop :=
  ∀ (P : Program) (N : NativeRel) (ν : NativeFlows) (s₀ : State),
    WellFormedBy V VerifierConfig.production P → P.SupportedVersions → P.SupportedNonGeneric → P.M3 →
    NativesRespect P N → AdmissibleInitial P s₀ → MutableContained P N ν s₀

theorem mutableContained_M3 : MutableContainedM3 verifier := by
  intro P N ν s₀ hwf _ hs hm3 hN h0
  exact mutableContained_of_steps h0 (heldFacts_tracked hwf hs hm3 hN h0)
    (stepsContained_tracked hwf hs hm3 hN h0)

end MoveBytecode.Proofs
