-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.ReturnsM3
import Proofs.EscapeApi
import Proofs.FrameRefs
import Proofs.FrameSteps
import Proofs.Derivations

/-!
# D4d for M3 programs: returned references derive from parameters

UNTRUSTED (checked by the kernel). Every reference a frame holds derives,
through the recorded `borrowed` events, from one of the frame's reference
parameters, or from a borrow the frame made itself: of one of its locals, or
of a resource of a type its function acquires while the type's module is
active once (`Origin`). The frames keep this along every annotated execution
(`IdChain`): a frame's own steps borrow from references it holds or from its
roots; a callee's parameters are references its caller holds; and a callee
returns references derived from its parameters, so they have their origins
in the caller. A verified `Ret` returns no reference into a local of the
frame, nor into a resource its function borrowed while its module was active
once (`ret_roots`: the analysis found every returned reference inside a
parameter, and a parameter into a resource makes the frame protect its type,
R3), so every returned reference derives from a parameter (`returnsDerived`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-! ## Frames with the same key -/

theorem key_id {fr fr' : Frame} (h : fr'.key = fr.key) : fr'.id = fr.id := by
  simp only [Frame.key, Prod.mk.injEq] at h; exact h.1

theorem key_function {fr fr' : Frame} (h : fr'.key = fr.key) : fr'.function = fr.function := by
  simp only [Frame.key, Prod.mk.injEq] at h; exact h.2.1

theorem key_skel {fr fr' : Frame} (h : fr'.key = fr.key) : fr'.skel = fr.skel := by
  simp only [Frame.key, Prod.mk.injEq] at h
  simp only [Frame.skel, h.2.1, h.2.2]

theorem map_key_skel : ∀ {frs frs' : List Frame}, frs'.map Frame.key = frs.map Frame.key →
    frs'.map Frame.skel = frs.map Frame.skel
  | [], [], _ => rfl
  | fr :: frs, fr' :: frs', h => by
    simp only [List.map_cons, List.cons.injEq] at h ⊢
    exact ⟨key_skel h.1, map_key_skel h.2⟩
  | [], _ :: _, h => by simp at h
  | _ :: _, [], h => by simp at h

/-! ## Origins -/

/-- **Where a reference of frame `fr` comes from**, given the frame's
reference parameters `pids` and the functions and activations `sk` of it and
the frames below: it derives from a parameter, or from a borrow the frame
made of one of its locals, or of a resource of a type its function acquires
while the type's module is active at most once. -/
def Origin (P : Program) (E : List Event) (fr : Frame) (sk : List (FunctionRef × Bool))
    (pids : List Nat) (d : Nat) : Prop :=
  (∃ p ∈ pids, Derives E p d) ∨
  ∃ ρ rρ, Event.borrowed ρ rρ none ∈ E ∧ Derives E ρ d ∧
    ((∃ i, rρ.location.root = .local_ fr.id i) ∨
      ∃ a t, rρ.location.root = .global a t ∧ acqTypes P fr.function t ∧
        countActive P sk t.module ≤ 1)

section Origin

variable {P : Program} {E : List Event} {fr : Frame} {sk : List (FunctionRef × Bool)}
  {pids : List Nat}

theorem Origin.append {d : Nat} (h : Origin P E fr sk pids d) (F : List Event) :
    Origin P (E ++ F) fr sk pids d := by
  rcases h with ⟨p, hp, hD⟩ | ⟨ρ, rρ, hb, hD, hc⟩
  · exact Or.inl ⟨p, hp, hD.append⟩
  · exact Or.inr ⟨ρ, rρ, List.mem_append_left _ hb, hD.append, hc⟩

theorem Origin.congr {d : Nat} {fr' : Frame} (h : Origin P E fr sk pids d)
    (hk : fr'.key = fr.key) : Origin P E fr' sk pids d := by
  rw [Origin, key_id hk, key_function hk]
  exact h

theorem Origin.derive {t d : Nat} (h : Origin P E fr sk pids t) (hD : Derives E t d) :
    Origin P E fr sk pids d := by
  rcases h with ⟨p, hp, hD'⟩ | ⟨ρ, rρ, hb, hD', hc⟩
  · exact Or.inl ⟨p, hp, hD'.trans hD⟩
  · exact Or.inr ⟨ρ, rρ, hb, hD'.trans hD, hc⟩

end Origin

/-! ## The frames' origins -/

/-- **The identity invariant of the frames** `gs`, `frs` (the current one
first): each frame has its reference parameters `pids`, which were borrowed;
every reference it holds has its origin in the frame; and, when the frame
above has reference parameters `above`, they have their origins in it. -/
def IdChain (P : Program) (E : List Event) :
    List ShadowFrame → List Frame → Option (List Nat) → Prop
  | [], [], _ => True
  | g :: gs, fr :: frs, above =>
    ∃ pids, RefSafety.params? E fr.id = some pids ∧ (∀ p ∈ pids, p ∈ borrowedIds E) ∧
      (∀ d r, (d, Value.ref r) ∈ g.occurrences fr →
        Origin P E fr ((fr :: frs).map Frame.skel) pids d) ∧
      (∀ cps, above = some cps → ∀ p ∈ cps,
        Origin P E fr ((fr :: frs).map Frame.skel) pids p) ∧
      IdChain P E gs frs (some pids)
  | [], _ :: _, _ => False
  | _ :: _, [], _ => False

/-- Frames `gs'`, `frs'` with the keys of `gs`, `frs`, each holding references
the corresponding one held. -/
def FramesSub : List ShadowFrame → List Frame → List ShadowFrame → List Frame → Prop
  | [], [], [], [] => True
  | g' :: gs', fr' :: frs', g :: gs, fr :: frs =>
    fr'.key = fr.key ∧
      (∀ d r, (d, Value.ref r) ∈ g'.occurrences fr' → (d, Value.ref r) ∈ g.occurrences fr) ∧
      FramesSub gs' frs' gs frs
  | _, _, _, _ => False

theorem FramesSub.skel : ∀ {gs' : List ShadowFrame} {frs' : List Frame} {gs : List ShadowFrame}
    {frs : List Frame}, FramesSub gs' frs' gs frs → frs'.map Frame.skel = frs.map Frame.skel
  | [], [], [], [], _ => rfl
  | _ :: _, _ :: _, _ :: _, _ :: _, ⟨hk, _, hrest⟩ => by
    simp only [List.map_cons, List.cons.injEq]
    exact ⟨key_skel hk, FramesSub.skel hrest⟩

theorem FramesSub.refl : ∀ (gs : List ShadowFrame) (frs : List Frame),
    gs.length = frs.length → FramesSub gs frs gs frs
  | [], [], _ => trivial
  | _ :: gs, _ :: frs, h => ⟨rfl, fun _ _ hm => hm, FramesSub.refl gs frs (by simpa using h)⟩
  | [], _ :: _, h => by simp at h
  | _ :: _, [], h => by simp at h

/-- Frames given position by position. -/
theorem framesSub_of : ∀ {gs' : List ShadowFrame} {frs' : List Frame} {gs : List ShadowFrame}
    {frs : List Frame}, gs'.length = gs.length → gs.length = frs.length →
    frs'.map Frame.key = frs.map Frame.key →
    (∀ (k : Nat) (g g' : ShadowFrame) (fr fr' : Frame), gs[k]? = some g → frs[k]? = some fr →
      gs'[k]? = some g' → frs'[k]? = some fr' → ∀ d r, (d, Value.ref r) ∈ g'.occurrences fr' →
        (d, Value.ref r) ∈ g.occurrences fr) →
    FramesSub gs' frs' gs frs
  | [], [], [], [], _, _, _, _ => trivial
  | g' :: gs', fr' :: frs', g :: gs, fr :: frs, h1, h2, h3, h4 => by
    simp only [List.map_cons, List.cons.injEq] at h3
    refine ⟨h3.1, h4 0 g g' fr fr' rfl rfl rfl rfl, framesSub_of (by simpa using h1)
      (by simpa using h2) h3.2 fun k g g' fr fr' a b c d => h4 (k + 1) g g' fr fr' a b c d⟩
  | [], [], [], _ :: _, _, h2, _, _ => by simp at h2
  | [], [], _ :: _, _, h1, _, _, _ => by simp at h1
  | [], _ :: _, _, _, _, _, h3, _ => by
    cases ‹List Frame› <;> simp_all
  | _ :: _, [], _, _, _, _, h3, _ => by
    cases ‹List Frame› <;> simp_all
  | _ :: _, _ :: _, [], _, h1, _, _, _ => by simp at h1
  | _ :: _, _ :: _, _ :: _, [], _, h2, _, _ => by simp at h2

/-- **The invariant over longer events and frames holding no new reference.** -/
theorem IdChain.sub {P : Program} {E : List Event} (F : List Event) :
    ∀ {gs : List ShadowFrame} {frs : List Frame} {gs' : List ShadowFrame} {frs' : List Frame}
      {above : Option (List Nat)}, IdChain P E gs frs above → FramesSub gs' frs' gs frs →
      IdChain P (E ++ F) gs' frs' above
  | [], [], [], [], _, _, _ => trivial
  | g :: gs, fr :: frs, g' :: gs', fr' :: frs', above, ⟨pids, hp, hb, hrefs, habove, hrest⟩,
      ⟨hk, hsub, hfs⟩ => by
    have hsk : (fr' :: frs').map Frame.skel = (fr :: frs).map Frame.skel := by
      simp only [List.map_cons, List.cons.injEq]
      exact ⟨key_skel hk, hfs.skel⟩
    refine ⟨pids, ?_, fun p hp' => ?_, fun d r hm => ?_, fun cps hc p hp' => ?_,
      IdChain.sub F hrest hfs⟩
    · rw [key_id hk]; exact params?_append hp
    · rw [borrowedIds_append]; exact List.mem_append_left _ (hb p hp')
    · rw [hsk]; exact ((hrefs d r (hsub d r hm)).append F).congr hk
    · rw [hsk]; exact ((habove cps hc p hp').append F).congr hk
  | [], [], _ :: _, _, _, _, h => by cases ‹List Frame› <;> exact h.elim
  | [], [], [], _ :: _, _, _, h => h.elim
  | _ :: _, _ :: _, [], _, _, _, h => by cases ‹List Frame› <;> exact h.elim
  | _ :: _, _ :: _, _ :: _, [], _, _, h => h.elim
  | [], _ :: _, _, _, _, h, _ => h.elim
  | _ :: _, [], _, _, _, h, _ => h.elim

/-! ## Steps -/

theorem framesFit_length {gs : List ShadowFrame} {frs : List Frame}
    (h : framesFit gs frs = true) : gs.length = frs.length := by
  simp only [framesFit, Bool.and_eq_true, beq_iff_eq] at h
  exact h.1

/-- The current frames of an annotated state. -/
theorem shadow_cons {P : Program} {s : State} {h : Shadow} {fr : Frame} {rest : List Frame}
    (hpre : Pre false P s h) (hsf : s.frames = fr :: rest) :
    ∃ g hs, h.frames = g :: hs ∧ hs.length = rest.length := by
  obtain ⟨hframes, hst, hhe, fresh⟩ := h
  cases hframes with
  | nil =>
    have hfit := hpre.fits
    rw [shadow_fits, hsf] at hfit
    simp [framesFit] at hfit
  | cons g hs => exact ⟨g, hs, rfl, framesFit_length (pre_head hpre hsf).2⟩

/-- A traced step runs in a frame. -/
theorem traced_frames_cons {P : Program} {N : NativeRel} {ν : NativeFlows} {s : State}
    {h : Shadow} {o : Outcome} {post : ShadowPost} {evs : List Event}
    (hS : TracedStep P N ν s h o post evs) : ∃ fr rest, s.frames = fr :: rest := by
  cases hS with
  | core _ hc =>
    obtain ⟨fr, rest, -, -, -, hsf, -⟩ := coreStep_exec hc
    exact ⟨fr, rest, hsf⟩
  | native hstep _ _ =>
    obtain ⟨fr, rest, -, hsf, -⟩ := step_callNative hstep
    exact ⟨fr, rest, hsf⟩

/-- A traced step whose instruction is `Ret` is a step of the core. -/
theorem ret_core {P : Program} {N : NativeRel} {ν : NativeFlows} {s : State} {h : Shadow}
    {o : Outcome} {post : ShadowPost} {evs : List Event} (hS : TracedStep P N ν s h o post evs)
    {fr : Frame} {rest : List Frame} (hsf : s.frames = fr :: rest) {info : FunctionInfo}
    {code : CodeUnit} (hinfo : P.function? fr.function = some info)
    (hcode : info.definition.code = some code) (hinstr : code.code[fr.pc]? = some .ret) :
    stepOf (Step.exec P s fr rest info code .ret) = .done o := by
  cases hS with
  | core hstep _ =>
    unfold step at hstep
    rw [hsf] at hstep
    simp only [hinfo, hcode, hinstr] at hstep
    exact hstep
  | native hstep _ _ =>
    exfalso
    unfold step at hstep
    rw [hsf] at hstep
    simp only [hinfo, hcode, hinstr] at hstep
    have hn : NotNative (Step.exec P s fr rest info code .ret) := by
      unfold Step.exec Step.ret
      not_native
    revert hn hstep
    generalize Step.exec P s fr rest info code .ret = x
    intro hstep hn
    cases x with
    | ok r => subst hstep; exact hn
    | error e => cases hstep

/-- **A reference a traced step of a verified program borrows with no
parent** is into a local of the current frame, or into a resource of a type
the frame's function acquires while its module is active at most once. -/
theorem traced_rootBorrow {P : Program} {N : NativeRel} {ν : NativeFlows}
    (hwf : WellFormedBy verifier VerifierConfig.production P) {s s' : State} (hinv : InvRefs P s)
    {h : Shadow} {o : Outcome} {post : ShadowPost} {evs : List Event}
    (hS : TracedStep P N ν s h o post evs) (ho : o = .running s') {fr : Frame}
    {rest : List Frame} (hsf : s.frames = fr :: rest) {d : Nat} {r : Reference}
    (hbd : Event.borrowed d r none ∈ evs) :
    (∃ i, r.location.root = .local_ fr.id i) ∨
      ∃ a t, r.location.root = .global a t ∧ acqTypes P fr.function t ∧
        countActive P (s.frames.map Frame.skel) t.module ≤ 1 := by
  cases hS with
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
    case inr =>
      obtain ⟨-, -, -, -, -, -, -, -, -, he⟩ := nativeStep_borrowed hn hbd
    obtain ⟨rfl, rfl, rfl, g, hs, info, hh, -, -, -, hevs, -⟩ := nativeStep_some hstep hshape hn
    rw [hevs] at hbd
    have := mem_borrowedIds.2 ⟨_, _, hbd⟩
    rw [nativeEvents_borrowedIds] at this
    cases this
  | core hstep hc =>
    subst ho
    obtain ⟨fr₁, rest₁, info, code, instr, hsf₁, hinfo, hcode, hinstr, hex⟩ := coreStep_exec hc
    rw [hsf, List.cons.injEq] at hsf₁
    obtain ⟨rfl, rfl⟩ := hsf₁
    have hst' : stepOf (Step.exec P s fr rest info code instr) = .done (.running s') := by
      unfold step at hstep
      rw [hsf] at hstep
      simp only [hinfo, hcode, hinstr] at hstep
      exact hstep
    exact rootBorrow_roots hwf hinv hinfo hcode hinstr hst' hex hbd

/-- **A reference a traced step borrows with no parent**, in every state of an
execution: a wrapper in progress borrows only from its argument. -/
theorem traced_rootBorrow_all {P : Program} {N : NativeRel} {ν : NativeFlows}
    (hwf : WellFormedBy verifier VerifierConfig.production P) {s s' : State} (hinv : InvCore P s)
    {h : Shadow} {o : Outcome} {post : ShadowPost} {evs : List Event}
    (hS : TracedStep P N ν s h o post evs) (ho : o = .running s') {fr : Frame}
    {rest : List Frame} (hsf : s.frames = fr :: rest) {d : Nat} {r : Reference}
    (hbd : Event.borrowed d r none ∈ evs) :
    (∃ i, r.location.root = .local_ fr.id i) ∨
      ∃ a t, r.location.root = .global a t ∧ acqTypes P fr.function t ∧
        countActive P (s.frames.map Frame.skel) t.module ≤ 1 := by
  rcases hinv.2 with hr | ⟨sc, hrun⟩
  · exact traced_rootBorrow hwf ⟨hinv.1, hr⟩ hS ho hsf hbd
  · exfalso
    cases hS with
    | native hstep _ hn =>
      rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
      case inr =>
        obtain ⟨-, -, -, -, -, -, -, -, -, he⟩ := nativeStep_borrowed hn hbd
      obtain ⟨rfl, rfl, rfl, g, hs, info, hh, -, -, -, hevs, -⟩ := nativeStep_some hstep hshape hn
      rw [hevs] at hbd
      have := mem_borrowedIds.2 ⟨_, _, hbd⟩
      rw [nativeEvents_borrowedIds] at this
      cases this
    | core hstep hc =>
      obtain ⟨fr₁, rest₁, info, code, instr, hsf₁, hinfo, hcode, hinstr, hex⟩ := coreStep_exec hc
      rw [hsf, List.cons.injEq] at hsf₁
      obtain ⟨rfl, rfl⟩ := hsf₁
      obtain ⟨g, hs', hh⟩ : ∃ g hs', h.frames = g :: hs' := by
        cases hf : h.frames with
        | nil => simp [Shadow.exec, hf] at hex
        | cons g hs' => exact ⟨g, hs', rfl⟩
      obtain ⟨-, hk⟩ := exec_rootBorrow hh hex hbd
      rcases apiCore_instr hrun hsf hinfo hcode hinstr with ⟨-, rfl⟩ |
          ⟨-, ⟨f, rfl⟩ | ⟨v, rfl⟩ | ⟨f, rfl⟩ | ⟨v, rfl⟩⟩ | ⟨-, rfl⟩ <;>
        rcases hk with ⟨j, hj | hj⟩ | ⟨j, hj | hj | hj | hj⟩ <;> cases hj

section Steps

variable {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}
  (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.Supported)
  (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
    ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)
  (hN : NativesRespect P N) (h0 : AdmissibleInitial P s₀)
include hwf hs hapi hN h0

/-- **A verified `Ret` returns references derived from the frame's
parameters**: a reference with its origin in a borrow the frame made is into
one of its locals, or into a resource of a type it acquires while the type's
module is active once, which a verified `Ret` does not return. -/
theorem ret_params {s : State} {h : Shadow} {E : List Event} (hT : Trace P N ν s₀ s h E)
    {o : Outcome} {post : ShadowPost} {evs : List Event} (hS : TracedStep P N ν s h o post evs)
    (hcont : o.continues = true) {fr : Frame} {rest : List Frame} {g : ShadowFrame}
    {hs' : List ShadowFrame} (hsf : s.frames = fr :: rest) (hhf : h.frames = g :: hs')
    {rets : List Nat} (hR : RetFacts P s fr g evs rets) {pids : List Nat}
    (hO : ∀ d r, (d, Value.ref r) ∈ g.occurrences fr →
      Origin P E fr ((fr :: rest).map Frame.skel) pids d) :
    ∀ d ∈ rets, ∃ p ∈ pids, Derives E p d := by
  intro d hd
  obtain ⟨r, hrs, hm⟩ := hR.rets d hd
  rcases hO d r hm with hpar | ⟨ρ, rρ, hb, hD, hc⟩
  · exact hpar
  exfalso
  have hI := refTraceInv_trace h0 hT
  have hinv := invCore_reachable hwf hs hapi hN h0 s (trace_reachable hT)
  obtain ⟨par, hbd⟩ := hI.refs d r (by rw [hhf, hsf, framesOcc_cons]; exact List.mem_append_left _ hm)
  have hval : ∀ d x, Event.used d x ∈ E → ∃ par, Event.borrowed d x par ∈ E := by
    intro d x hu'
    obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hu'
    exact used_value (F := []) h0 (heldFacts_frames h0) hT (fun _ _ h => by cases h)
      (by rw [List.append_nil]; exact hk)
  have hBI : BorrowsInside E := by
    have := borrows_inside_all (F := []) h0 hT (fun _ _ _ _ _ _ _ hS' => traced_borrowsInside hS')
      (fun _ _ _ h => by cases h)
    rwa [List.append_nil] at this
  have hin := (derives_inside hI.unique hBI hval hb hD hbd).1
  obtain ⟨info, code, hinfo, hcode, hinstr⟩ := hR.instr
  obtain ⟨retTys, hrets, hconf⟩ := ret_conforms (ret_core hS hsf hinfo hcode hinstr) hcont
  obtain ⟨hloc, hglob⟩ := ret_roots_all hwf hs hapi hinv hsf hinfo hcode hinstr hrets hconf hrs
  rcases hc with ⟨i, hi⟩ | ⟨a, t, ht, hacq, hcnt⟩
  · exact hloc i (by rw [hin.1, hi])
  · have := hglob a t (by rw [hin.1, ht]) hacq
    rw [hsf] at this
    omega

/-- **A traced step keeps the frames' origins.** -/
theorem idChain_step {s : State} {h : Shadow} {E : List Event} (hT : Trace P N ν s₀ s h E)
    (hC : IdChain P E h.frames s.frames none) {s' : State} {h' : Shadow} {evs : List Event}
    (hS : TracedStep P N ν s h (.running s') (.running h') evs) :
    IdChain P (E ++ evs) h'.frames s'.frames none := by
  have hI := refTraceInv_trace h0 hT
  rcases traced_shape hI.pre hI.typed.args hI.typed.valsShaped hS rfl rfl with ⟨hkeys, hlen, -⟩ |
    ⟨fr, rest, g, hs', callee, gc, k, pids, hsf, hhf, hs'f, hh'f, hcid, hent, hcev, hcrefs,
      hpids, hcaller⟩ |
    ⟨fr, caller, rest, g, gcaller, hs', caller', gc', rets, hsf, hhf, hs'f, hh'f, hkey, hR,
      hrefs⟩
  · -- A step of the current frame.
    obtain ⟨fr, rest, hsf⟩ := traced_frames_cons hS
    obtain ⟨g, hs', hhf, hlen'⟩ := shadow_cons hI.pre hsf
    obtain ⟨fr', rest', hs'f⟩ : ∃ fr' rest', s'.frames = fr' :: rest' := by
      cases hf : s'.frames with
      | nil => rw [hf, hsf] at hkeys; simp at hkeys
      | cons fr' rest' => exact ⟨fr', rest', rfl⟩
    obtain ⟨g', hs'', hh'f⟩ : ∃ g' hs'', h'.frames = g' :: hs'' := by
      cases hf : h'.frames with
      | nil => rw [hf, hhf] at hlen; simp at hlen
      | cons g' hs'' => exact ⟨g', hs'', rfl⟩
    have hslen : s'.frames.length = s.frames.length := by
      have := congrArg List.length hkeys
      simpa using this
    have hsk : (fr' :: rest').map Frame.skel = (fr :: rest).map Frame.skel := by
      rw [← hs'f, ← hsf]; exact map_key_skel hkeys
    have hkeys' := hkeys
    rw [hs'f, hsf, List.map_cons, List.map_cons, List.cons.injEq] at hkeys'
    obtain ⟨hk0, hkrest⟩ := hkeys'
    have hAt : ∀ (k : Nat) (g g' : ShadowFrame) (fr fr' : Frame), h.frames[k]? = some g →
        s.frames[k]? = some fr → h'.frames[k]? = some g' → s'.frames[k]? = some fr' →
        ∀ d r, (d, Value.ref r) ∈ g'.occurrences fr' → (d, Value.ref r) ∈ g.occurrences fr ∨
          (k = 0 ∧ ∃ par, Event.borrowed d r par ∈ evs) :=
      fun _ _ _ _ _ a b c e _ _ hm => traced_frameRefsAt hI hS rfl rfl hslen a b c e hm
    rw [hhf, hsf] at hC
    obtain ⟨pids, hp, hb, hrefs, -, hrest⟩ := hC
    rw [hh'f, hs'f]
    refine ⟨pids, by rw [key_id hk0]; exact params?_append hp, fun p hp' => ?_, fun d r hm => ?_,
      fun cps hc => (by cases hc), IdChain.sub evs hrest ?_⟩
    · rw [borrowedIds_append]; exact List.mem_append_left _ (hb p hp')
    · rw [hsk]
      rcases hAt 0 g g' fr fr' (by rw [hhf]; rfl) (by rw [hsf]; rfl) (by rw [hh'f]; rfl)
        (by rw [hs'f]; rfl) d r hm with hm' | ⟨-, par, hbd⟩
      · exact ((hrefs d r hm').append evs).congr hk0
      · cases par with
        | some t =>
          -- Derived from a reference the frame holds.
          obtain ⟨rt, hu, -, -⟩ := traced_borrowsInside hS d r t hbd
          obtain ⟨g0, fr0, hg0, hfr0, hm0⟩ := traced_used_top hI.pre hS hu
          rw [hhf, List.head?_cons, Option.some.injEq] at hg0
          rw [hsf, List.head?_cons, Option.some.injEq] at hfr0
          subst hg0 hfr0
          exact (((hrefs t rt hm0).append evs).derive
            (.step (List.mem_append_right _ hbd) .refl)).congr hk0
        | none =>
          -- Borrowed from a root of the frame.
          have hinv := invCore_reachable hwf hs hapi hN h0 s (trace_reachable hT)
          have hroot := traced_rootBorrow_all hwf hinv hS rfl hsf hbd
          rw [hsf] at hroot
          exact Origin.congr (P := P) (sk := (fr :: rest).map Frame.skel) (pids := pids)
            (Or.inr ⟨d, r, List.mem_append_right _ hbd, .refl, hroot⟩) hk0
    · refine framesSub_of ?_ hlen' hkrest fun k g₁ g₁' fr₁ fr₁' a b c e d r hm => ?_
      · have := hlen
        rw [hh'f, hhf] at this
        simpa using this
      · rcases hAt (k + 1) g₁ g₁' fr₁ fr₁' (by rw [hhf]; exact a) (by rw [hsf]; exact b)
          (by rw [hh'f]; exact c) (by rw [hs'f]; exact e) d r hm with hm' | ⟨hk, -⟩
        · exact hm'
        · omega
  · -- A call.
    have hocc : ∀ d r, (d, Value.ref r) ∈ g.occurrences fr →
        (d, Value.ref r) ∈ framesOcc h.frames s.frames := fun d r hm => by
      rw [hhf, hsf, framesOcc_cons]; exact List.mem_append_left _ hm
    have hlen' : hs'.length = rest.length := by
      obtain ⟨g₁, hs₁, hh₁, hl⟩ := shadow_cons hI.pre hsf
      rw [hhf, List.cons.injEq] at hh₁
      rw [hh₁.2]; exact hl
    rw [hhf, hsf] at hC
    obtain ⟨pids0, hp0, hb0, hrefs0, -, hrest0⟩ := hC
    have hpE : RefSafety.params? E callee.id = none :=
      params?_none_of fun f ps he => by rw [hcid]; exact trace_entered_lt h0 hT f ps he
    have hpev : RefSafety.params? evs callee.id = some pids :=
      params?_of_mem (fun e he f' ps' hef hf' => by
        subst hef
        exact ((hcev _ he).2 f' ps' rfl).2) hent
    rw [hh'f, hs'f]
    refine ⟨pids, by rw [params?_append_none hpE]; exact hpev, fun p hp' => ?_,
      fun d r hm => Or.inl ⟨d, hcrefs d r hm, .refl⟩, fun cps hc => (by cases hc), ?_⟩
    · obtain ⟨r, hm⟩ := hpids p hp'
      obtain ⟨par, hbp⟩ := hI.refs p r (hocc p r hm)
      rw [borrowedIds_append]
      exact List.mem_append_left _ (mem_borrowedIds.2 ⟨r, par, hbp⟩)
    · refine ⟨pids0, params?_append hp0, fun p hp' => ?_, fun d r hm => ?_,
        fun cps hc p hp' => ?_, IdChain.sub evs hrest0 (FramesSub.refl _ _ hlen')⟩
      · rw [borrowedIds_append]; exact List.mem_append_left _ (hb0 p hp')
      · exact (hrefs0 d r (hcaller d r hm)).append evs
      · cases hc
        obtain ⟨r, hm⟩ := hpids p hp'
        exact (hrefs0 p r hm).append evs
  · -- A return to the caller.
    have hlen' : hs'.length = rest.length := by
      obtain ⟨g₁, hs₁, hh₁, hl⟩ := shadow_cons hI.pre hsf
      rw [hhf, List.cons.injEq] at hh₁
      rw [← hh₁.2] at hl
      simpa using hl
    rw [hhf, hsf] at hC
    obtain ⟨pids0, hp0, hb0, hrefs0, -, pids1, hp1, hb1, hrefs1, habove1, hrest1⟩ := hC
    have hder := ret_params hwf hs hapi hN h0 hT hS rfl hsf hhf hR hrefs0
    have hsk : (caller' :: rest).map Frame.skel = (caller :: rest).map Frame.skel := by
      simp only [List.map_cons, key_skel hkey]
    rw [hh'f, hs'f]
    refine ⟨pids1, by rw [key_id hkey]; exact params?_append hp1, fun p hp' => ?_, fun d r hm => ?_,
      fun cps hc => (by cases hc), IdChain.sub evs hrest1 (FramesSub.refl _ _ hlen')⟩
    · rw [borrowedIds_append]; exact List.mem_append_left _ (hb1 p hp')
    · rw [hsk]
      rcases hrefs d r hm with hm' | hd
      · exact ((hrefs1 d r hm').append evs).congr hkey
      · obtain ⟨p, hp', hD⟩ := hder d hd
        exact (((habove1 pids0 rfl p hp').derive hD).append evs).congr hkey

/-- **Every frame-ending event of a traced step** returns references derived
from the frame's parameters. -/
theorem rets_step {s : State} {h : Shadow} {E : List Event} (hT : Trace P N ν s₀ s h E)
    (hC : IdChain P E h.frames s.frames none) {o : Outcome} {post : ShadowPost}
    {evs : List Event} (hS : TracedStep P N ν s h o post evs) :
    ∀ e ∈ evs, ∀ f rets, e = .frameEnded f rets → ∃ ps, RefSafety.params? (E ++ evs) f = some ps ∧
      ∀ d ∈ rets, ∃ p ∈ ps, RefSafety.derivesFrom (E ++ evs) d p = true := by
  intro e he f rets hef
  subst hef
  have hI := refTraceInv_trace h0 hT
  have hfit := traced_fits hS
  obtain ⟨fr, rest, g, hs', hsf, hhf, rets', hR, hcont⟩ : ∃ fr rest g hs', s.frames = fr :: rest ∧
      h.frames = g :: hs' ∧ ∃ rets', RetFacts P s fr g evs rets' ∧ o.continues = true := by
    cases o with
    | running s' =>
      cases post with
      | running h' =>
        rcases traced_shape hI.pre hI.typed.args hI.typed.valsShaped hS rfl rfl with ⟨-, -, hev⟩ | ⟨_, _, _, _, _, _, _, _, _, _, _, _,
            _, _, hcev, _⟩ |
          ⟨fr, caller, rest, g, gcaller, hs', caller', gc', rets, hsf, hhf, -, -, -, hR, -⟩
        · exact absurd (hev _ he).2 (by simp [Event.returned?])
        · exact absurd (hcev _ he).1 (by simp [Event.returned?])
        · exact ⟨fr, caller :: rest, g, gcaller :: hs', hsf, hhf, rets, hR, rfl⟩
      | returned _ _ => simp [ShadowPost.fits] at hfit
    | returned vs st =>
      obtain ⟨fr, g, hs', rets, hsf, hhf, hR⟩ := traced_returned_shape hI.pre hS rfl
      exact ⟨fr, [], g, hs', hsf, hhf, rets, hR, rfl⟩
    | failed _ => cases post <;> simp [ShadowPost.fits] at hfit
    | wrong _ => cases post <;> simp [ShadowPost.fits] at hfit
  obtain ⟨rfl, rfl⟩ := (hR.only _ he).2 f rets rfl
  rw [hhf, hsf] at hC
  obtain ⟨pids0, hp0, hb0, hrefs0, -, -⟩ := hC
  have hder := ret_params hwf hs hapi hN h0 hT hS hcont hsf hhf hR hrefs0
  refine ⟨pids0, params?_append hp0, fun d hd => ?_⟩
  obtain ⟨p, hp, hD⟩ := hder d hd
  exact ⟨p, hp, derivesFrom_append
    (derivesFrom_of_derives hI.unique (trace_parent_lt h0 hT) (hb0 p hp) hD)⟩

end Steps

/-! ## Executions -/

/-- **The entry frame's references are its parameters**, the references the
transaction lent. -/
theorem idChain_initial {P : Program} {s₀ : State} (h0 : AdmissibleInitial P s₀) :
    IdChain P (Shadow.initial s₀).2 (Shadow.initial s₀).1.frames s₀.frames none := by
  obtain ⟨fr₀, g₀, pids₀, hsf, hhf, hp, hrefs, hb, -, -⟩ := initial_entry h0
  rw [hsf, hhf]
  exact ⟨pids₀, hp, hb, fun d r hm => Or.inl ⟨d, hrefs d r hm, .refl⟩, fun cps hc => (by cases hc),
    trivial⟩

/-- No frame has returned at the start. -/
theorem returnsDerived_initial {P : Program} {s₀ : State} (h0 : AdmissibleInitial P s₀) :
    RefSafety.returnsDerived (Shadow.initial s₀).2 = true := by
  obtain ⟨-, -, -, -, -, -, -, -, -, hend⟩ := initial_entry h0
  unfold RefSafety.returnsDerived
  rw [List.all_eq_true]
  intro e he
  cases e with
  | frameEnded f rets => exact absurd he (hend f rets)
  | _ => rfl

section Trace

variable {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}
  (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.Supported)
  (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
    ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)
  (hN : NativesRespect P N) (h0 : AdmissibleInitial P s₀)
include hwf hs hapi hN h0

/-- **Along every annotated execution, every reference a frame holds has its
origin in the frame.** -/
theorem idChain_trace {s : State} {h : Shadow} {E : List Event} (hT : Trace P N ν s₀ s h E) :
    IdChain P E h.frames s.frames none := by
  induction hT with
  | start => exact idChain_initial h0
  | step hT hS ih => exact idChain_step hwf hs hapi hN h0 hT ih hS

/-- **Along every annotated execution, returned references derive from the
returning frame's parameters.** -/
theorem returnsDerived_trace {s : State} {h : Shadow} {E : List Event}
    (hT : Trace P N ν s₀ s h E) : RefSafety.returnsDerived E = true := by
  induction hT with
  | start => exact returnsDerived_initial h0
  | step hT hS ih =>
    exact returnsDerived_append ih
      (rets_step hwf hs hapi hN h0 hT (idChain_trace hwf hs hapi hN h0 hT) hS)

/-- **D4d**: no reference is stored inside a value, in global storage or among
the held values, and every returned reference derives from a reference
parameter of the returning frame. -/
theorem noEscape_all : NoEscape P N ν s₀ := by
  intro s h E hT
  have hR := returnsDerived_trace hwf hs hapi hN h0 hT
  refine ⟨unnested_all hwf hs hapi hN h0 hT, hR, fun o post evs' hS =>
    ⟨unnestedAfter_all hwf hs hapi hN h0 hT hS, returnsDerived_append hR
      (rets_step hwf hs hapi hN h0 hT (idChain_trace hwf hs hapi hN h0 hT) hS)⟩⟩

end Trace

/-- **D4d for M3 programs**, a corollary. -/
theorem noEscape_M3 {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}
    (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.SupportedNonGeneric) (hm3 : P.M3)
    (hN : NativesRespect P N) (h0 : AdmissibleInitial P s₀) : NoEscape P N ν s₀ :=
  noEscape_all hwf (supported_of_nonGeneric hwf hs) (apiVersions_M3 hm3) hN h0

end MoveBytecode.Proofs
