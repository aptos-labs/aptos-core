-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.SoundnessAll
import Proofs.MutableContained
import Proofs.ApiCore

/-!
# The tracked references of verified programs, and D4a

UNTRUSTED (checked by the kernel). The facts about the references the
verifier's analysis tracks, for every program the model accepts on the
supported subset, generic code and struct-API mutable borrows included: they
are held in the sense of `HeldFacts` (`heldFacts_tracked'`: the type and
availability invariant `Inv` suffices, which `InvCore` keeps), and valid and
of their type (`tracked_refOk'`). While a struct-API wrapper runs (`ApiCore`,
generic or not), the caller's
and the older frames' tracked references are the ones they kept at the call,
and the wrapper holds only the argument or the field it borrows. D4a follows
(`noDanglingUse_all`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-- A reference valid in a frame of the state `sc` of a struct-API call stays
valid while the wrapper `w` runs: the wrapper changes only its own frame. -/
theorem api_refOk' {P : Program} {s sc : State} {fr fr' w : Frame} {rest : List Frame}
    (hsc : sc.frames = fr :: rest) (hs : s.frames = w :: fr' :: rest)
    (hid : fr'.id = fr.id) (hfun : fr'.function = fr.function) (hloc : fr'.locals = fr.locals)
    (hst : s.storage = sc.storage) (hhe : s.held = sc.held) {es : List Invalidation}
    (hes : s.invalidations = es ++ sc.invalidations)
    (hesw : ∀ e ∈ es, e = .removed (.local_ w.id 0)) {frx : Frame} {r : Reference}
    (h : RefOk P sc frx r) (hlt : frx.id < w.id) : RefOk P s frx r := by
  have hne : ∀ f i, r.location.root = .local_ f i → f ≠ w.id := by
    intro f i hρ e
    have hb := h.below
    rw [hρ] at hb
    have hb' : f ≤ frx.id := hb
    exact Nat.lt_irrefl _ (Nat.lt_of_le_of_lt (e ▸ hb') hlt)
  obtain ⟨h1, h2⟩ := api_root (P := P) (ρ := r.location.root) hsc hs hid hfun hloc hst hhe hne
  refine h.transfer' rfl h1 hes (fun e he => ?_) (fun hr => ?_)
  · rw [hesw e he]
    simpa [Invalidation.covers] using hne w.id 0
  · rw [read?_eq_of_root h2]; exact hr

/-- **A tracked reference is valid and of its type**, in every state with
`InvCore` of a verified program. -/
theorem tracked_refOk' {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.Supported)
    (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
      ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)
    {s : State} (hinv : InvCore P s) {h : Shadow} {i : Nat} {r : Reference}
    (ht : Tracked P s h i r) : ∃ fr ∈ s.frames, RefOk P s fr r := by
  have hsigs := apiSigs_of hwf hapi
  obtain ⟨g', fr', hz, ht⟩ := ht
  have hfr' : fr' ∈ s.frames := (List.of_mem_zip hz).2
  have hk : Value.ref r ∈ fr'.stack ∨ ∃ j, fr'.locals[j]? = some (some (.ref r)) ∧
      modelAvail P fr'.function fr'.pc j = .available := by
    rcases ht with ⟨t, -, hz'⟩ | ⟨j, ⟨t, -, -, hl⟩, hav⟩
    · exact Or.inl (List.of_mem_zip hz').2
    · exact Or.inr ⟨j, hl, hav⟩
  rcases hinv.2 with hr | ⟨sc, hrun⟩
  · exact ⟨fr', hfr', kept_refOk hwf hs hsigs ⟨hinv.1, hr⟩ hfr' hk⟩
  · obtain ⟨fr, rest, info, code, off, x, st, w, cinfo, hsc, -, hstk, -, hsf, -, hwid, hstor,
      hheld, ⟨es, hes, hesw⟩, hwloc, hpc⟩ := hrun.call
    have hscinv := hrun.inv
    have hcarry : ∀ frx ∈ sc.frames, RefOk P sc frx r → RefOk P s frx r := fun frx hfrx hok =>
      api_refOk' hsc hsf rfl rfl rfl hstor hheld hes hesw hok
        (by rw [hwid]; exact hscinv.2.below frx hfrx)
    have hfr : fr ∈ sc.frames := by rw [hsc]; exact List.mem_cons_self ..
    have hcaller : ({ fr with stack := st } : Frame) ∈ s.frames := by
      rw [hsf]; exact List.mem_cons_of_mem _ (List.mem_cons_self ..)
    -- The caller's references, carried from the call.
    have hfrom : (Value.ref r ∈ fr.stack ∨ ∃ j, fr.locals[j]? = some (some (.ref r)) ∧
        modelAvail P fr.function fr.pc j = .available) → ∃ fr ∈ s.frames, RefOk P s fr r :=
      fun hk' => ⟨_, hcaller,
        (hcarry fr hfr (kept_refOk hwf hs hsigs hscinv hfr hk')).reframe rfl⟩
    rw [hsf] at hfr'
    rcases List.mem_cons.1 hfr' with he | hfr'
    · -- The wrapper holds the argument, or the field it borrowed.
      subst he
      have hx : Value.ref x ∈ fr.stack := by rw [hstk]; exact List.mem_cons_self ..
      rcases hk with hst' | ⟨j, hj, -⟩
      · rcases hpc with ⟨-, -, hs0⟩ | ⟨-, -, hs1⟩ | ⟨-, -, y, step, hs2, -, -, -, hy⟩
        · rw [hs0] at hst'; cases hst'
        · rw [hs1, List.mem_singleton, Value.ref.injEq] at hst'
          subst hst'
          exact hfrom (Or.inl hx)
        · rw [hs2, List.mem_singleton, Value.ref.injEq] at hst'
          subst hst'
          exact ⟨_, hcaller, hy⟩
      · have hj0 := hwloc j _ hj
        subst hj0
        rcases hpc with ⟨-, hl0, -⟩ | ⟨-, hl0, -⟩ | ⟨-, hl0, -⟩
        · rw [hl0, Option.some.injEq, Option.some.injEq, Value.ref.injEq] at hj
          subst hj
          exact hfrom (Or.inl hx)
        · rw [hl0] at hj; cases hj
        · rw [hl0] at hj; cases hj
    · rcases List.mem_cons.1 hfr' with he | hfr'
      · -- The caller, its operand stack less the argument.
        subst he
        refine hfrom ?_
        rcases hk with hst' | hl
        · exact Or.inl (by rw [hstk]; exact List.mem_cons_of_mem _ hst')
        · exact Or.inr hl
      · -- An older frame.
        have hfrsc : fr' ∈ sc.frames := by rw [hsc]; exact List.mem_cons_of_mem _ hfr'
        exact ⟨fr', by rw [hsf]; exact List.mem_cons_of_mem _ (List.mem_cons_of_mem _ hfr'),
          hcarry fr' hfrsc (kept_refOk hwf hs hsigs hscinv hfrsc hk)⟩

section Verified

variable {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}
  (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.Supported)
  (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
    ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)
  (hN : NativesRespect P N) (h0 : AdmissibleInitial P s₀)

include hwf hs hapi hN h0 in
/-- **The verifier's tracked references are held** in the sense of
`HeldFacts`, along the executions of a verified program from an admissible
initial state. -/
theorem heldFacts_tracked' : HeldFacts P N ν s₀ (Tracked P) :=
  heldFacts_tracked_of (certified_of_wellFormed_gen schedulerSound hwf hs)
    (fun s hr => (invCore_reachable hwf hs hapi hN h0 s hr).1) h0

include hwf hs hapi hN h0 in
/-- **D4a for verified programs**: every step uses only references to valid
locations. -/
theorem noDanglingUse_all : NoDanglingUse P N ν s₀ := by
  intro s h evs hT o post evs' hS
  have hinv := invCore_reachable hwf hs hapi hN h0 s (trace_reachable hT)
  unfold RefSafety.usesValid
  rw [List.all_eq_true]
  intro e he
  cases e
  case used i r =>
    have ht := (heldFacts_tracked' (ν := ν) hwf hs hapi hN h0).used s h evs hT o post evs' hS i r he
    obtain ⟨fr, -, hok⟩ := tracked_refOk' hwf hs hapi hinv ht
    exact hok.valid
  all_goals rfl

end Verified

end MoveBytecode.Proofs
