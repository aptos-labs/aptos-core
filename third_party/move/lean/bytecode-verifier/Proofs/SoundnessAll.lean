-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.ApiSteps
import Proofs.ApiReturn

/-!
# D1: soundness

UNTRUSTED (checked by the kernel). An accepted program on the supported
subset never goes wrong (`soundness`). Executions keep `InvAll`: from a
state with the reference invariant, every instruction but a call of a
struct-API mutable borrow keeps it (`exec_safe_ref`, the instruction proofs
of the M3 milestone, which needed only that no call is one), and such a call
starts a wrapper in progress (`apiRun_call`); the wrapper's steps keep it in
progress (`apiRun_step`), and its return resumes the caller with the
reference invariant (`apiRun_ret`). The struct-API attributes appear only in
modules of version 10 and later (`Program.SupportedVersions`), where the
verifier checks the wrappers' code.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-- **The version premise**: a struct-API mutable borrow attribute appears
only in modules of version 10 and later. -/
theorem apiVersions_of {P : Program} (hv : P.SupportedVersions) :
    ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
      ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version := by
  intro m hm fh hfh off hoff
  apply Nat.le_of_not_lt
  intro hlt
  exact hv.2 m hm hlt fh hfh _ (borrowFieldMutable_mem hoff) rfl

section Main

variable {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
  (hs : P.SupportedNonGeneric)
  (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
    ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)

include hwf hs hapi in
/-- **Every instruction but a call of a struct-API mutable borrow** is safe
from the reference invariant and keeps it. -/
theorem exec_safe_ref (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hN : NativesRespect P N) {s : State} (hinv : InvRefs P s) {fr : Frame} {rest : List Frame}
    (hsf : s.frames = fr :: rest) {info : FunctionInfo} {code : CodeUnit}
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {instr : Bytecode}
    (hi : code.code[fr.pc]? = some instr)
    (hattr : ∀ h, instr = .call h → ∀ fh, info.module.functionHandles[h]? = some fh →
      ReferenceSafety.borrowFieldMutable fh.attributes = none) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code instr) := by
  have hr := hinv.2
  have hsigs := apiSigs_of hwf hapi
  obtain ⟨view, A, stys, tys, ps, hctx⟩ := topCtx' hwf hs hsigs hinv hsf hact hi
  have hparts := function?_parts hact.fn
  have hmod : info.module.SupportedNonGeneric := hs _ (module_mem hparts.1)
  have hsub := (hmod.2.2.2.2.2 _ (getElem?_mem_toList hparts.2.2.1) code hact.body).2 instr
    (getElem?_mem_toList hi)
  have hcore : (instr.inCore || instr.inDataByValue) = true → instr ≠ .eq → instr ≠ .neq →
      ExecSafe P N (Inv P (modelTypes P) (modelAvail P)) (Step.exec P s fr rest info code instr) :=
    fun hb h1 h2 => exec_safe hc hN hact hchain hi hb (fun h => by
      rcases h with h | h
      · exact absurd h h1
      · exact absurd h h2) (noModeledCall_nonGeneric hs _ _ _)
  have hvalue : instr.ValueOnly = true →
      ExecRuns P N (RefInvariant P) (Step.exec P s fr rest info code instr) := fun hv =>
    execRuns_of (exec_notNative (fun h e => by subst e; simp [Bytecode.ValueOnly] at hv)
      (fun i e => by subst e; simp [Bytecode.ValueOnly] at hv))
      (fun s' h => by
        obtain ⟨A', hx, hsucc⟩ := hctx.next (by intro e; subst e; simp [Bytecode.ValueOnly] at hv)
        obtain ⟨j, k, hvs⟩ := exec_valueStep hv
          (fun sig k e => by subst e; exact vectorElemType_head hsub.2) hmod.2.2.2.2.1 hx
        exact refInv_valueOnly hr hsf (typeArgsOk_values hact.typeArgs) hi hv h hctx.ref hvs hsucc
          (conformsAll_wellFormed hctx.conf))
  have hruns : (∀ h, instr ≠ .call h) →
      (∀ s', stepOf (Step.exec P s fr rest info code instr) = .done (.running s') →
        RefInvariant P s') →
      ExecRuns P N (RefInvariant P) (Step.exec P s fr rest info code instr) := fun hne h =>
    execRuns_of (exec_notNative hne (fun i e => by
      subst e; simp [Bytecode.inSubsetNG, Bytecode.inCore, Bytecode.inDataByValue,
        Bytecode.inReferences] at hsub)) h
  cases instr
  case brTrue | brFalse | branch | nop | ldU8 | ldU16 | ldU32 | ldU64 | ldU128 | ldU256 | ldI8
    | ldI16 | ldI32 | ldI64 | ldI128 | ldI256 | ldTrue | ldFalse | ldConst | castU8 | castU16
    | castU32 | castU64 | castU128 | castU256 | castI8 | castI16 | castI32 | castI64 | castI128
    | castI256 | add | sub | mul | mod | div | bitOr | bitAnd | xor | or | and | not | lt | gt | le
    | ge | shl | shr | negate | abort | abortMsg | pack | unpack | packVariant | unpackVariant
    | vecPack | vecUnpack =>
    exact ExecSafe.and (hcore rfl (by intro h; cases h) (by intro h; cases h)) (hvalue rfl)
  case exists_ d => exact ExecSafe.and (exists_safe hc hact hchain hi) (hvalue rfl)
  case moveFrom d => exact refSafe_moveFrom hwf hc hr hsf hact hchain hctx hi
  case pop =>
    exact ExecSafe.and (hcore rfl (by intro h; cases h) (by intro h; cases h))
      (hruns (fun _ h => by cases h) fun s' h => refInv_pop hr hsf hctx hi h)
  case copyLoc i =>
    exact ExecSafe.and (hcore rfl (by intro h; cases h) (by intro h; cases h))
      (hruns (fun _ h => by cases h) fun s' h => refInv_copyLoc hr hsf hctx hi h)
  case moveLoc i =>
    exact ExecSafe.and (hcore rfl (by intro h; cases h) (by intro h; cases h))
      (hruns (fun _ h => by cases h) fun s' h => refInv_moveLoc hr hsf hctx hact.locals hi h)
  case stLoc i =>
    exact ExecSafe.and (hcore rfl (by intro h; cases h) (by intro h; cases h))
      (hruns (fun _ h => by cases h) fun s' h => refInv_stLoc hr hsf hctx hact.locals hi h)
  case call h =>
    exact ExecSafe.and (hcore rfl (by intro h; cases h) (by intro h; cases h))
      (refRuns_call' hwf (supported_of_nonGeneric hwf hs) hsigs hc hr hsf hact hctx hi (hattr h rfl)
        (noModeledCall_nonGeneric hs _ _ _))
  case ret =>
    exact ExecSafe.and (hcore rfl (by intro h; cases h) (by intro h; cases h))
      (refRuns_ret hr hsf hctx hact.fn)
  case eq => exact refSafe_eq (positive := true) hc hr hsf hact hchain hctx hi
  case neq => exact refSafe_eq (positive := false) hc hr hsf hact hchain hctx hi
  case readRef => exact refSafe_readRef hc hr hsf hact hchain hctx hi
  case writeRef => exact refSafe_writeRef hc hr hsf hact hchain hctx hi
  case freezeRef => exact refSafe_freezeRef hc hr hsf hact hchain hctx hi
  case mutBorrowLoc i => exact refSafe_borrowLoc (mut_ := true) hc hr hsf hact hchain hctx hi
  case immBorrowLoc i => exact refSafe_borrowLoc (mut_ := false) hc hr hsf hact hchain hctx hi
  case mutBorrowField h => exact refSafe_borrowField (mut_ := true) hc hr hsf hact hchain hctx hi
  case immBorrowField h => exact refSafe_borrowField (mut_ := false) hc hr hsf hact hchain hctx hi
  case mutBorrowVariantField h =>
    exact refSafe_borrowVariantField (mut_ := true) hc hr hsf hact hchain hctx hi
  case immBorrowVariantField h =>
    exact refSafe_borrowVariantField (mut_ := false) hc hr hsf hact hchain hctx hi
  case testVariant h =>
    exact refSafe_testVariant hc (structsAgree hwf hs hparts.1) hctx.cert.setting.bounds hr hsf hact
      hchain hctx hi
  case vecLen sig => exact refSafe_vecLen hc hr hsf hact hchain hctx hi
  case vecImmBorrow sig => exact refSafe_vecBorrow (mut_ := false) hc hr hsf hact hchain hctx hi
  case vecMutBorrow sig => exact refSafe_vecBorrow (mut_ := true) hc hr hsf hact hchain hctx hi
  case vecPushBack sig => exact refSafe_vecPushBack hc hr hsf hact hchain hctx hi
  case vecPopBack sig => exact refSafe_vecPopBack hc hr hsf hact hchain hctx hi
  case vecSwap sig => exact refSafe_vecSwap hc hr hsf hact hchain hctx hi
  case moveTo d => exact refSafe_moveTo hc hr hsf hact hchain hctx hi
  case mutBorrowGlobal d =>
    exact refSafe_borrowGlobal (mut_ := true) hwf hc hr hsf hact hchain hctx hi
  case immBorrowGlobal d =>
    exact refSafe_borrowGlobal (mut_ := false) hwf hc hr hsf hact hchain hctx hi
  all_goals
    simp [Bytecode.inSubsetNG, Bytecode.inCore, Bytecode.inDataByValue, Bytecode.inReferences] at hsub

include hwf hs hapi in
/-- **Every instruction of an accepted program** is safe from the invariant
and keeps it. -/
theorem exec_safe_all (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hN : NativesRespect P N) {s : State} (hinv : InvAll P s) {fr : Frame} {rest : List Frame}
    (hsf : s.frames = fr :: rest) {info : FunctionInfo} {code : CodeUnit}
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {instr : Bytecode}
    (hi : code.code[fr.pc]? = some instr) :
    ExecSafe P N (InvAll P) (Step.exec P s fr rest info code instr) := by
  rcases hinv.2 with hr | ⟨sc, hrun⟩
  · have hinv' : InvRefs P s := ⟨hinv.1, hr⟩
    by_cases hapiC : ∃ h fh off, instr = .call h ∧ info.module.functionHandles[h]? = some fh ∧
        ReferenceSafety.borrowFieldMutable fh.attributes = some off
    · obtain ⟨h, fh, off, rfl, hfh, hoff⟩ := hapiC
      exact apiRun_call hwf (supported_of_nonGeneric hwf hs) hapi hc hN hinv' hsf hact hchain hi hfh hoff
    · refine ExecSafe.weaken (fun _ h => InvRefs.invAll h)
        (exec_safe_ref hwf hs hapi hc hN hinv' hsf hact hchain hi ?_)
      intro h e fh hfh
      cases hb : ReferenceSafety.borrowFieldMutable fh.attributes with
      | none => rfl
      | some off => exact absurd ⟨h, fh, off, e, hfh, hb⟩ hapiC
  · by_cases hret : instr = .ret
    · subst hret
      exact apiRun_ret hwf (supported_of_nonGeneric hwf hs) hapi hc hN hrun hsf hact hchain hi
    · exact apiRun_step hwf (supported_of_nonGeneric hwf hs) hapi hc hN hrun hsf hact hchain hi hret

include hwf hs hapi in
theorem step_safe_all (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hN : NativesRespect P N) {s : State} (hinv : InvAll P s) :
    StepSafe P N (InvAll P) (step P s) := by
  obtain ⟨fr, rest, hfr, ⟨info, code, hact⟩, hchain⟩ := hinv.1
  have hlt := hact.pcLt
  have hpc : code.code[fr.pc]? = some code.code[fr.pc] := Array.getElem?_eq_getElem hlt
  have hsafe := exec_safe_all hwf hs hapi hc hN hinv hfr hact hchain hpc
  unfold step
  rw [hfr]
  simp only [hact.fn, hact.body, hpc]
  cases hx : Step.exec P s fr rest info code code.code[fr.pc] <;> rw [hx] at hsafe <;>
    exact hsafe

include hwf hs hapi in
theorem stepRel_safe_all (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hN : NativesRespect P N) {s : State} {o : Outcome} (hinv : InvAll P s)
    (hstep : StepRel P N s o) : Safe (InvAll P) o := by
  have hsafe := step_safe_all hwf hs hapi hc hN hinv
  cases hstep with
  | core h => rw [h] at hsafe; exact hsafe
  | native h hr => rw [h] at hsafe; exact hsafe _ hr

include hwf hs hapi in
/-- **Every reachable state satisfies the invariant.** -/
theorem invAll_reachable (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hN : NativesRespect P N) {s₀ : State} (h0 : AdmissibleInitial P s₀) :
    ∀ s, Reachable P N s₀ s → InvAll P s := by
  intro s hr
  induction hr with
  | refl => exact (initial_invRefs' hwf (apiSigs_of hwf hapi) (supported_of_nonGeneric hwf hs) hc h0).invAll
  | step _ hst ih => exact stepRel_safe_all hwf hs hapi hc hN ih hst

end Main

/-- **D1** on the non-generic subset: a program the model accepts never goes
wrong, whatever its natives do within their contracts. References cross calls
in both directions, code borrows global storage, and calls of struct-API
mutable borrows run their wrappers while the caller's analysis takes them for
field borrows. -/
theorem soundness_nonGeneric : SoundnessNonGeneric verifier := by
  intro P N s₀ hwf hv hs hN h0 s hr r hw
  have hapi := apiVersions_of hv
  have hc := certified_of_wellFormed schedulerSound hwf hs
  exact stepRel_safe_all hwf hs hapi hc hN (invAll_reachable hwf hs hapi hc hN h0 s hr) hw

end MoveBytecode.Proofs
