-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefCalls
import Proofs.Soundness

/-!
# D1 with references, milestone M3

UNTRUSTED (checked by the kernel). A program the model accepts, on the
supported subset, whose function handles are no struct-API mutable borrows
(`Program.M3`), never goes wrong. References are created, read, written,
frozen and compared, passed to callees and returned to callers, and borrowed
from global storage: the reference-safety analysis's state at each
instruction describes the current frame's references, and its state at each
suspended caller's call describes the caller's (`RefInvariant`); each
instruction keeps that description and the type and availability invariant
(`exec_safe_M3`). An execution may start with references to values the
transaction holds, as for the `&signer` parameters of entry functions. M2 (no
global borrows) and M1 (functions take and return values) are corollaries
(`soundness_M2`, `soundness_M1`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph MoveBytecode.Step

variable {P : Program}

/-- Only `Call` and `CallGeneric` call natives. -/
theorem exec_notNative {s : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
    {code : CodeUnit} {instr : Bytecode} (hne : ∀ h, instr ≠ .call h)
    (hng : ∀ i, instr ≠ .callGeneric i) :
    NotNative (Step.exec P s fr rest info code instr) := by
  cases instr
  case call hd => exact absurd rfl (hne hd)
  case callGeneric i => exact absurd rfl (hng i)
  case ret =>
    unfold Step.exec Step.ret
    not_native
  all_goals
    unfold Step.exec
    not_native

theorem execRuns_of {N : NativeRel} {R : State → Prop} {x : Step.Exec StepResult}
    (hn : NotNative x) (h : ∀ s', stepOf x = .done (.running s') → R s') :
    ExecRuns P N R x := by
  match x, hn with
  | .ok (.done o), _ =>
    cases o with
    | running s' => exact h s' rfl
    | returned _ _ => trivial
    | failed _ => trivial
    | wrong _ => trivial
  | .error o, _ =>
    cases o with
    | running s' => exact h s' rfl
    | returned _ _ => trivial
    | failed _ => trivial
    | wrong _ => trivial

/-! ## Each instruction -/

section Main

variable (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.SupportedNonGeneric)
  (hm3 : P.M3)

include hwf hs hm3 in
/-- **Every instruction of an accepted M3 program** is safe from the
invariant and keeps it. -/
theorem exec_safe_M3 (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hN : NativesRespect P N) {s : State} (hinv : InvRefs P s) {fr : Frame} {rest : List Frame}
    (hsf : s.frames = fr :: rest) {info : FunctionInfo} {code : CodeUnit}
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {instr : Bytecode}
    (hi : code.code[fr.pc]? = some instr) :
    ExecSafe P N (InvRefs P) (Step.exec P s fr rest info code instr) := by
  have hr := hinv.2
  obtain ⟨view, A, stys, tys, ps, hctx⟩ := topCtx hwf hs hm3 hinv hsf hact hi
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
      (refRuns_call hwf (supported_of_nonGeneric hwf hs) hm3 hc hr hsf hact hctx hi
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

include hwf hs hm3 in
theorem step_safe_M3 (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hN : NativesRespect P N) {s : State} (hinv : InvRefs P s) :
    StepSafe P N (InvRefs P) (step P s) := by
  obtain ⟨fr, rest, hfr, ⟨info, code, hact⟩, hchain⟩ := hinv.1
  have hlt := hact.pcLt
  have hpc : code.code[fr.pc]? = some code.code[fr.pc] := Array.getElem?_eq_getElem hlt
  have hsafe := exec_safe_M3 hwf hs hm3 hc hN hinv hfr hact hchain hpc
  unfold step
  rw [hfr]
  simp only [hact.fn, hact.body, hpc]
  cases hx : Step.exec P s fr rest info code code.code[fr.pc] <;> rw [hx] at hsafe <;>
    exact hsafe

include hwf hs hm3 in
theorem stepRel_safe_M3 (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hN : NativesRespect P N) {s : State} {o : Outcome} (hinv : InvRefs P s)
    (hstep : StepRel P N s o) : Safe (InvRefs P) o := by
  have hsafe := step_safe_M3 hwf hs hm3 hc hN hinv
  cases hstep with
  | core h => rw [h] at hsafe; exact hsafe
  | native h hr => rw [h] at hsafe; exact hsafe _ hr

end Main

/-! ## The initial state -/

/-- What `heldArguments` checks of the reference arguments: each designates a
whole held value of a value type, which it conforms to, and was taken before
anything was invalidated; no two designate the same value. -/
theorem heldArguments_refs {held args : List Value} (h : heldArguments P held args = true) :
    (∀ r ∈ argRefs args, r.since = 0 ∧ r.location.path = [] ∧ ∃ k v,
      r.location.root = .held k ∧ held[k]? = some v ∧ v.conforms P r.target = true ∧
      v.type.isValue = true) ∧
    ((argRefs args).map (·.location)).Nodup := by
  unfold heldArguments at h
  simp only [Bool.and_eq_true, List.all_eq_true, decide_eq_true_eq] at h
  obtain ⟨⟨⟨hvals, -⟩, hrefs⟩, hnd⟩ := h
  refine ⟨fun r hr => ?_, hnd⟩
  have hr' := hrefs r (List.mem_filterMap.2 ⟨.ref r, mem_argRefs.1 hr, rfl⟩)
  simp only [beq_iff_eq] at hr'
  obtain ⟨hs0, hm⟩ := hr'
  split at hm
  · rename_i k hroot hpath
    split at hm
    · rename_i v hv
      exact ⟨hs0, hpath, k, v, hroot, hv, hm, (hvals v (List.mem_of_getElem? hv)).1⟩
    · cases hm
  · cases hm

theorem not_strictlyInside_of_paths {l m : Location} (hl : l.path = []) (hm : m.path = []) :
    ¬ l.StrictlyInside m := fun h =>
  h.2 (by rw [Location.eta l, Location.eta m, h.1.1, hl, hm])

/-- References to whole values at distinct locations are compatible. -/
theorem compat_of_held {args : List Value}
    (hpath : ∀ r ∈ argRefs args, r.location.path = [])
    (hnd : ((argRefs args).map (·.location)).Nodup) : (argRefs args).Pairwise Compat := by
  have hp : (argRefs args).Pairwise (fun a b => a.location ≠ b.location) :=
    List.pairwise_map.1 hnd
  refine List.Pairwise.imp_of_mem (fun {a b} ha hb hne => ?_) hp
  exact ⟨fun _ => ⟨not_strictlyInside_of_paths (hpath b hb) (hpath a ha), Ne.symm hne⟩,
    fun _ => ⟨not_strictlyInside_of_paths (hpath a ha) (hpath b hb), hne⟩,
    fun _ hin => absurd hin (not_strictlyInside_of_paths (hpath a ha) (hpath b hb)),
    fun _ hin => absurd hin (not_strictlyInside_of_paths (hpath b hb) (hpath a ha))⟩

section Initial

theorem eq_of_nodup_map {α β : Type} {f : α → β} :
    ∀ {l : List α}, (l.map f).Nodup → ∀ {a b : α}, a ∈ l → b ∈ l → f a = f b → a = b
  | [], _, _, _, ha, _, _ => by simp at ha
  | x :: l, hnd, a, b, ha, hb, he => by
    rw [List.map_cons, List.nodup_cons] at hnd
    rcases List.mem_cons.1 ha with ha | ha <;> rcases List.mem_cons.1 hb with hb | hb
    · rw [ha, hb]
    · exact absurd (by rw [← ha, he]; exact List.mem_map_of_mem hb) hnd.1
    · exact absurd (by rw [← hb, ← he]; exact List.mem_map_of_mem ha) hnd.1
    · exact eq_of_nodup_map hnd.2 ha hb he


variable (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.SupportedNonGeneric)
  (hm3 : P.M3) (hapi : P.ApiSigs)

include hwf hapi in
/-- **The initial state satisfies the invariant**: the entry frame's
reference parameters are the references to the values the transaction
holds. -/
theorem initial_invRefs' (hsG : P.Supported) (hc : Certified P (modelTypes P) (modelAvail P))
    {s₀ : State}
    (h0 : AdmissibleInitial P s₀) : InvRefs P s₀ := by
  refine ⟨initial_inv hc h0, ?_⟩
  obtain ⟨f, args, ps, storage, held, hpar, hargs, hwt, hinit⟩ := h0
  simp only [initialState, bind, Option.bind_eq_some_iff, pure] at hinit
  obtain ⟨info, hf, code, hcode, ltys, hltys, hinit⟩ := hinit
  split at hinit
  · simp at hinit
  rename_i htok0
  have htok : P.typeArgsOk f = true := by simpa using htok0
  split at hinit
  · simp at hinit
  rename_i hha
  simp only [Option.some.injEq] at hinit
  subst hinit
  obtain ⟨hrefs, hnd⟩ := heldArguments_refs (by simpa using hha)
  refine ⟨by simp, fun g hg => ?_, fun r hr => ?_, ⟨_, [], argRefs args, rfl, ?_,
    fun _ => False, ⟨fun _ h => h.elim, fun _ h => h.elim, fun π hπ a t hρ => ?_⟩, trivial⟩,
    ⟨fun M => ?_, rfl⟩, ?_⟩
  · simp only [List.mem_singleton] at hg
    subst hg
    exact Nat.zero_lt_one
  · simp only [GlobalStorage.wellTyped, Bool.and_eq_true, List.all_eq_true] at hwt
    exact (hwt.2 r hr).1
  rotate_left
  · obtain ⟨-, -, k, v, hroot, -⟩ := hrefs π hπ
    rw [hroot] at hρ
    cases hρ
  · by_cases h : info.moduleId = M
    · subst h
      simp [State.activeCount, countActive, funModule, hf, Frame.skel]
    · simp [State.activeCount, countActive, funModule, hf, Frame.skel, h]
  · classical
    refine ⟨fun k => if h : ∃ r ∈ argRefs args, r.location = ⟨.held k, []⟩ then
      (Classical.choose h).target else .bool, fun g hg r hr k hk => ?_⟩
    simp only [List.mem_singleton] at hg
    subst hg
    have hra : r ∈ argRefs args := by
      rw [mem_argRefs]
      simp only [frameVals, List.nil_append, List.mem_filterMap, id,
        List.mem_append, List.mem_map, List.mem_replicate] at hr
      obtain ⟨o, ho, ho'⟩ := hr
      rcases ho with ⟨w, hw, rfl⟩ | ⟨-, ho⟩
      · simp only [Option.some.injEq] at ho'
        rw [← ho']; exact hw
      · subst ho; cases ho'
    have hex : ∃ r ∈ argRefs args, r.location = ⟨.held k, []⟩ := ⟨r, hra, hk⟩
    dsimp only
    rw [dif_pos hex]
    obtain ⟨hmem, hloc⟩ := Classical.choose_spec hex
    rw [eq_of_nodup_map hnd hmem hra (hloc.trans hk.symm)]
  refine frameOk_entry' hwf hsG hapi hf hcode htok hpar rfl hargs
    (compat_of_held (fun r hr => (hrefs r hr).2.1) hnd) (fun r hr => ?_) (fun r hr f' i hρ => ?_)
  · obtain ⟨hs0, hp, k, v, hroot, hv, hconf, hval⟩ := hrefs r hr
    refine ⟨⟨v.type, v.type, ?_, by rw [hp]; rfl,
      Or.inr ⟨⟨hp, by rw [hroot]; exact fun _ _ e => Root.noConfusion e⟩,
        conforms_assignable hconf⟩⟩, by rw [hroot]; trivial,
      by rw [← assignable_isValue (conforms_assignable hconf)]; exact hval,
      by rw [hs0]; exact Nat.zero_le _, ?_, fun _ => by rw [hp]; rfl⟩
    · simp only [RootType, hroot, hv, Option.bind_some, conforms_wellFormed hconf, ↓reduceIte]
    · rw [deref?_isSome]
      refine ⟨fun e he => by simp [State.invalidatedSince] at he, ?_⟩
      rw [Location.eta r.location, hroot, hp, read?_held]
      simp [hv, Value.get?]
  · obtain ⟨-, -, k, v, hroot, -⟩ := hrefs r hr
    rw [hroot] at hρ
    cases hρ

include hwf hs hm3 in
/-- **The initial state satisfies the invariant**, for an accepted M3
program. -/
theorem initial_invM3 (hc : Certified P (modelTypes P) (modelAvail P)) {s₀ : State}
    (h0 : AdmissibleInitial P s₀) : InvRefs P s₀ :=
  initial_invRefs' hwf hm3.apiSigs (supported_of_nonGeneric hwf hs) hc h0

include hwf hs hm3 in
/-- Every reachable state satisfies the invariant. -/
theorem invM3_reachable (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hN : NativesRespect P N) {s₀ : State} (h0 : AdmissibleInitial P s₀) :
    ∀ s, Reachable P N s₀ s → InvRefs P s := by
  intro s hr
  induction hr with
  | refl => exact initial_invM3 hwf hs hm3 hc h0
  | step _ hst ih => exact stepRel_safe_M3 hwf hs hm3 hc hN ih hst

end Initial

/-- **D1** for M3 programs: `Soundness` with the premise `P.M3`. -/
def SoundnessM3 (V : VerifierInterface) : Prop :=
  ∀ (P : Program) (N : NativeRel) (s₀ : State),
    WellFormedBy V VerifierConfig.production P → P.SupportedVersions → P.SupportedNonGeneric →
    P.M3 → NativesRespect P N → AdmissibleInitial P s₀ → NoWrong P N s₀

/-- **D1 with references (milestone M3)**: a program the model accepts, on
the supported subset, whose function handles are no struct-API mutable
borrows, never goes wrong, whatever its natives do within their contracts.
References cross calls in both directions, and code borrows global storage. -/
theorem soundness_M3 : SoundnessM3 verifier := by
  intro P N s₀ hwf _ hs hm3 hN h0 s hr r hw
  have hc := certified_of_wellFormed schedulerSound hwf hs
  exact stepRel_safe_M3 hwf hs hm3 hc hN (invM3_reachable hwf hs hm3 hc hN h0 s hr) hw

/-- **D1** for M2 programs: `Soundness` with the premise `P.M2`. -/
def SoundnessM2 (V : VerifierInterface) : Prop :=
  ∀ (P : Program) (N : NativeRel) (s₀ : State),
    WellFormedBy V VerifierConfig.production P → P.SupportedVersions → P.SupportedNonGeneric →
    P.M2 → NativesRespect P N → AdmissibleInitial P s₀ → NoWrong P N s₀

/-- **D1 with references (milestone M2)**, a corollary of M3: no code borrows
global storage. -/
theorem soundness_M2 : SoundnessM2 verifier := fun P N s₀ hwf hv hs hm2 =>
  soundness_M3 P N s₀ hwf hv hs hm2.toM3

/-- **D1** for M1 programs: `Soundness` with the premise `P.M1`. -/
def SoundnessM1 (V : VerifierInterface) : Prop :=
  ∀ (P : Program) (N : NativeRel) (s₀ : State),
    WellFormedBy V VerifierConfig.production P → P.SupportedVersions → P.SupportedNonGeneric →
    P.M1 → NativesRespect P N → AdmissibleInitial P s₀ → NoWrong P N s₀

/-- **D1 with references (milestone M1)**, a corollary of M3: a program whose
functions take and return values and whose code borrows no global storage. -/
theorem soundness_M1 : SoundnessM1 verifier := fun P N s₀ hwf hv hs hm1 =>
  soundness_M3 P N s₀ hwf hv hs hm1.toM2.toM3

end MoveBytecode.Proofs
