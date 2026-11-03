-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.ApiGeneric

/-!
# D1: soundness on the whole supported subset

UNTRUSTED (checked by the kernel). An accepted program of the supported
subset, generic functions, structs and enums included, never goes wrong
(`soundness`). Executions keep `InvAllG`: from a state with the reference
invariant, every instruction but a call of a struct-API mutable borrow keeps
it (`exec_safe_refGen`); such a call, `Call` or `CallGeneric`, starts a
wrapper in progress (`apiRun_call`, `apiRunG_call`), whose steps keep it in
progress and whose return resumes the caller with the reference invariant.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

section Main

variable {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
  (hs : P.Supported)
  (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
    ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)

include hwf hs hapi in
/-- **Every instruction of an accepted program** is safe from the invariant
and keeps it. -/
theorem exec_safe_allG (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hN : NativesRespect P N) {s : State} (hinv : InvAllG P s) {fr : Frame} {rest : List Frame}
    (hsf : s.frames = fr :: rest) {info : FunctionInfo} {code : CodeUnit}
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {instr : Bytecode}
    (hi : code.code[fr.pc]? = some instr) :
    ExecSafe P N (InvAllG P) (Step.exec P s fr rest info code instr) := by
  rcases hinv.2 with hr | ⟨sc, hrun⟩ | ⟨sc, hrun⟩
  · have hinv' : InvRefs P s := ⟨hinv.1, hr⟩
    by_cases hapiC : ∃ h fh off, instr = .call h ∧ info.module.functionHandles[h]? = some fh ∧
        ReferenceSafety.borrowFieldMutable fh.attributes = some off
    · obtain ⟨h, fh, off, rfl, hfh, hoff⟩ := hapiC
      exact ExecSafe.weaken (fun _ h => InvAll.invAllG h)
        (apiRun_call hwf hs hapi hc hN hinv' hsf hact hchain hi hfh hoff)
    by_cases hapiG : ∃ i fi fh off, instr = .callGeneric i ∧
        info.module.functionInstantiations[i]? = some fi ∧
        info.module.functionHandles[fi.handle]? = some fh ∧
        ReferenceSafety.borrowFieldMutable fh.attributes = some off
    · obtain ⟨i, fi, fh, off, rfl, hfi, hfh, hoff⟩ := hapiG
      exact apiRunG_call hwf hs hapi hc hN hinv' hsf hact hchain hi hfi hfh hoff
    refine ExecSafe.weaken (fun _ h => (InvRefs.invAll h).invAllG)
      (exec_safe_refGen hwf hs hapi hc hN hinv' hsf hact hchain hi ?_ ?_)
    · intro h e fh hfh
      cases hb : ReferenceSafety.borrowFieldMutable fh.attributes with
      | none => rfl
      | some off => exact absurd ⟨h, fh, off, e, hfh, hb⟩ hapiC
    · intro i e fi fh hfi hfh
      cases hb : ReferenceSafety.borrowFieldMutable fh.attributes with
      | none => rfl
      | some off => exact absurd ⟨i, fi, fh, off, e, hfi, hfh, hb⟩ hapiG
  · by_cases hret : instr = .ret
    · subst hret
      exact ExecSafe.weaken (fun _ h => InvAll.invAllG h)
        (apiRun_ret hwf hs hapi hc hN hrun hsf hact hchain hi)
    · exact ExecSafe.weaken (fun _ h => InvAll.invAllG h)
        (apiRun_step hwf hs hapi hc hN hrun hsf hact hchain hi hret)
  · by_cases hret : instr = .ret
    · subst hret
      exact apiRunG_ret hwf hs hapi hc hN hrun hsf hact hchain hi
    · exact apiRunG_step hwf hs hapi hc hN hrun hsf hact hchain hi hret

include hwf hs hapi in
theorem step_safe_allG (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hN : NativesRespect P N) {s : State} (hinv : InvAllG P s) :
    StepSafe P N (InvAllG P) (step P s) := by
  obtain ⟨fr, rest, hfr, ⟨info, code, hact⟩, hchain⟩ := hinv.1
  have hlt := hact.pcLt
  have hpc : code.code[fr.pc]? = some code.code[fr.pc] := Array.getElem?_eq_getElem hlt
  have hsafe := exec_safe_allG hwf hs hapi hc hN hinv hfr hact hchain hpc
  unfold step
  rw [hfr]
  simp only [hact.fn, hact.body, hpc]
  cases hx : Step.exec P s fr rest info code code.code[fr.pc] <;> rw [hx] at hsafe <;>
    exact hsafe

include hwf hs hapi in
theorem stepRel_safe_allG (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hN : NativesRespect P N) {s : State} {o : Outcome} (hinv : InvAllG P s)
    (hstep : StepRel P N s o) : Safe (InvAllG P) o := by
  have hsafe := step_safe_allG hwf hs hapi hc hN hinv
  cases hstep with
  | core h => rw [h] at hsafe; exact hsafe
  | native h hr => rw [h] at hsafe; exact hsafe _ hr

include hwf hs hapi in
/-- **Every reachable state satisfies the invariant.** -/
theorem invAllG_reachable (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hN : NativesRespect P N) {s₀ : State} (h0 : AdmissibleInitial P s₀) :
    ∀ s, Reachable P N s₀ s → InvAllG P s := by
  intro s hr
  induction hr with
  | refl => exact (initial_invRefs' hwf (apiSigs_of hwf hapi) hs hc h0).invAll.invAllG
  | step _ hst ih => exact stepRel_safe_allG hwf hs hapi hc hN ih hst

include hwf hs hapi in
/-- **The certificates agree with every execution**: the top frame of every
reachable state is active, by the invariant. -/
theorem certificates_agreeG (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hN : NativesRespect P N) :
    StaticTypes P N (modelTypes P) ∧ StaticAvailability P N (modelAvail P) := by
  have hact : ∀ s₀, AdmissibleInitial P s₀ → ∀ s, Reachable P N s₀ s → ∀ fr rest,
      s.frames = fr :: rest →
        ∃ info code, ActiveAt P (modelTypes P) (modelAvail P) fr info code := by
    intro s₀ h0 s hr fr rest hfr
    obtain ⟨fr', rest', hfr', hok, -⟩ := (invAllG_reachable hwf hs hapi hc hN h0 s hr).1
    rw [hfr, List.cons.injEq] at hfr'
    obtain ⟨rfl, -⟩ := hfr'
    exact hok
  refine ⟨fun s₀ h0 s hr fr rest hfr => ?_, fun s₀ h0 s hr fr rest hfr => ?_⟩
  · obtain ⟨info, code, ha⟩ := hact s₀ h0 s hr fr rest hfr
    obtain ⟨tys, htys, -, hconf⟩ := ha.locals
    exact ⟨ha.types, tys, htys, hconf⟩
  · obtain ⟨info, code, ha⟩ := hact s₀ h0 s hr fr rest hfr
    exact ⟨ha.available, ha.unavailable⟩

end Main

/-- **C1, C2**: the model's certificates are consistent and parametric, and
every execution agrees with them, on the whole supported subset. -/
theorem certificatesHold : CertificatesHold verifier := by
  intro P hwf hv hs
  have hapi := apiVersions_of hv
  have hc := certified_of_wellFormed_gen schedulerSound hwf hs
  exact ⟨modelTypes _, modelAvail _, hc.types, modelTypes_parametric P, hc.availability,
    fun _ hN => certificates_agreeG hwf hs hapi hc hN⟩

/-- **D1**, the main theorem: a program the model accepts never goes wrong,
whatever its natives do within their contracts, on the whole supported
subset: generic functions, structs and enums, references across calls in
both directions, global storage, and struct-API mutable borrows, generic or
not. -/
theorem soundness : Soundness verifier := by
  intro P N s₀ hwf hv hs hN h0 s hr r hw
  have hapi := apiVersions_of hv
  have hc := certified_of_wellFormed_gen schedulerSound hwf hs
  exact stepRel_safe_allG hwf hs hapi hc hN (invAllG_reachable hwf hs hapi hc hN h0 s hr) hw

end MoveBytecode.Proofs
