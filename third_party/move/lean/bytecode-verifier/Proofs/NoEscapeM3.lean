-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.ReferencesM3

/-!
# No reference inside a value, for verified programs

UNTRUSTED (checked by the kernel). The storage half of D4d (`NoEscape`):
along every annotated execution of a program the model accepts, at milestone
M3, references occur only as whole values of locals and operand-stack slots
(`RefSafety.unnested`), and a returning step hands back values with no
reference inside and storage with none (`RefSafety.unnestedAfter`). Every
slot conforms to a slot type, a value type or a reference type (D1's
invariant, and `CallFacts.stackConf` for suspended callers); global storage
and the held values are values of value types (`RefsTop`, `StateTyped.held`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph

/-- **Only `Ret` returns.** -/
theorem exec_post_returned {P : Program} {m : ModuleId} {instr : Bytecode} {s : State}
    {fr : Frame} {o : Outcome} {h : Shadow} {ts sts : List IdTree} {evs : List Event}
    {g : ShadowFrame} {hs : List ShadowFrame} (hh : h.frames = g :: hs)
    (hex : Shadow.exec P m instr s fr o h = some (.returned ts sts, evs)) : instr = .ret := by
  cases instr <;>
    simp only [Shadow.exec, hh, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
      Option.some.injEq, Prod.mk.injEq, guard, reduceCtorEq] at hex
  all_goals (try simp_all)
  all_goals (repeat' (first
    | (simp at hex; done)
    | (split at hex)
    | (obtain ⟨_, _, hex⟩ := hex)
    | (obtain ⟨_, hex⟩ := hex)
    | (rw [Option.bind_eq_some_iff] at hex)))
  all_goals
    try simp only [Option.bind_some, Option.bind_eq_some_iff, Option.some.injEq] at *
    first
      | (simp at *; done)
      | (obtain ⟨_, -, rfl⟩ := ‹∃ a, Shadow.topAfter o = some a ∧ _›; simp at hex)
      | (obtain ⟨-, _, -, h1, -⟩ := ‹¬ _ ∧ ∃ a, Shadow.topAfter o = some a ∧ _›
         subst h1
         simp_all)

/-! ## Values of slot types -/

/-- A slot type: a value type, or a reference type. -/
def SlotType (t : RuntimeType) : Prop := t.isValue = true ∨ ∃ b u, t = .ref b u

/-- A value with no reference anywhere inside it holds none strictly inside. -/
theorem noInner_of_plain {v : Value} (h : ∀ u ∈ subvalues v, u.isRef = false) :
    v.noInnerReference = true := by
  unfold Value.noInnerReference
  rw [List.all_eq_true]
  rintro ⟨p, w⟩ hw
  have := h w (List.mem_map.2 ⟨(p, w), hw, rfl⟩)
  simp [this]

/-- A value with no reference anywhere inside it is reference-free. -/
theorem referenceFree_of_plain {v : Value} (h : ∀ u ∈ subvalues v, u.isRef = false) :
    v.referenceFree = true := by
  unfold Value.referenceFree
  rw [List.all_eq_true]
  rintro ⟨p, w⟩ hw
  have := h w (List.mem_map.2 ⟨(p, w), hw, rfl⟩)
  simp [this]

/-- **A value of a slot type holds no reference strictly inside it.** -/
theorem noInner_of_conforms {P : Program} {v : Value} {t : RuntimeType}
    (hc : v.conforms P t = true) (ht : SlotType t) : v.noInnerReference = true := by
  rcases ht with ht | ⟨b, u, rfl⟩
  · exact noInner_of_plain (conforms_plain hc ht)
  · obtain ⟨r, rfl, -, -⟩ := conforms_ref_assignable hc
    simp [Value.noInnerReference, Value.nodes]

/-- The local types of a function are slot types. -/
theorem localTypes_slot {P : Program} {f : FunctionRef} {tys : List RuntimeType}
    (hf : ∀ x ∈ f.typeArgs, x.isValue = true)
    (h : P.localTypes f = some tys) : ∀ t ∈ tys, SlotType t := by
  unfold Program.localTypes at h
  simp only [bind, Option.bind_eq_some_iff, pure, Option.some.injEq] at h
  obtain ⟨info, -, code, -, psig, -, ps, hps, lsig, -, ls, hls, rfl⟩ := h
  intro t ht
  rcases List.mem_append.1 ht with ht | ht
  · exact slotType_of_ofSignature hf hps t ht
  · exact slotType_of_ofSignature hf hls t ht

/-- The locals of a frame whose locals conform hold no reference strictly
inside a value. -/
theorem locals_noInner {P : Program} (hs : P.SupportedNonGeneric) {fr : Frame}
    (h : LocalsOk P fr) :
    fr.locals.toList.all (fun slot => slot.all Value.noInnerReference) = true := by
  obtain ⟨tys, htys, -, hconf⟩ := h
  rw [localTypes_nonGeneric hs] at htys
  rw [List.all_eq_true]
  intro slot hslot
  cases slot with
  | none => rfl
  | some v =>
    obtain ⟨i, hi⟩ := List.mem_iff_getElem?.1 hslot
    rw [Array.getElem?_toList] at hi
    obtain ⟨t, ht, hc⟩ := hconf i v hi
    simpa using noInner_of_conforms hc
      (localTypes_slot (by simp) htys t (List.mem_of_getElem? ht))

/-- The locals of a frame whose locals conform, of an instance at value types,
hold no reference strictly inside a value. -/
theorem locals_noInner' {P : Program} {fr : Frame}
    (htargs : ∀ x ∈ fr.function.typeArgs, x.isValue = true) (h : LocalsOk P fr) :
    fr.locals.toList.all (fun slot => slot.all Value.noInnerReference) = true := by
  obtain ⟨tys, htys, -, hconf⟩ := h
  rw [List.all_eq_true]
  intro slot hslot
  cases slot with
  | none => rfl
  | some v =>
    obtain ⟨i, hi⟩ := List.mem_iff_getElem?.1 hslot
    rw [Array.getElem?_toList] at hi
    obtain ⟨t, ht, hc⟩ := hconf i v hi
    simpa using noInner_of_conforms hc (localTypes_slot htargs htys t (List.mem_of_getElem? ht))

/-- The suspended frames' operand stacks hold values of slot types. -/
theorem ChainRef.stack {P : Program} {s : State} :
    ∀ {cid : FrameId} {cfn : FunctionRef} {cps : List Reference} {Sc : StructId → Prop}
      {frs : List Frame}, ChainRef P s cid cfn cps Sc frs → ∀ g ∈ frs, ∀ v ∈ g.stack,
      ∃ t, v.conforms P t = true ∧ SlotType t
  | _, _, _, _, [], _, g, hg => by simp at hg
  | _, _, _, _, caller :: frs, ⟨ps, S, hsusp, hrest⟩, g, hg => by
    rcases List.mem_cons.1 hg with rfl | hg
    · obtain ⟨m, rs, rets, args, A, A', A'', L, hf⟩ := hsusp.call
      exact hf.stackConf
    · exact ChainRef.stack hrest g hg

section M3

variable {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
  (hs : P.SupportedNonGeneric) (hm3 : P.M3)
include hwf hs hm3

/-- **The frames of a verified execution hold references only as whole
values.** -/
theorem frames_unnested {s : State} (hinv : InvRefs P s) :
    ∀ fr ∈ s.frames, fr.locals.toList.all (fun slot => slot.all Value.noInnerReference) = true ∧
      fr.stack.all Value.noInnerReference = true := by
  obtain ⟨fr, rest, hsf, ⟨info, code, hact⟩, -⟩ := hinv.1
  obtain ⟨instr, hi⟩ : ∃ instr, code.code[fr.pc]? = some instr :=
    ⟨_, Array.getElem?_eq_getElem hact.pcLt⟩
  obtain ⟨view, A, stys, tys, ps, hctx⟩ := topCtx hwf hs hm3 hinv hsf hact hi
  obtain ⟨S, -, hchain⟩ := hctx.chain rest hsf
  intro g hg
  rw [hsf] at hg
  rcases List.mem_cons.1 hg with rfl | hg
  · refine ⟨locals_noInner hs hact.locals, List.all_eq_true.2 fun v hv => ?_⟩
    obtain ⟨t, ht, hc⟩ := conformsAll_mem hctx.conf v hv
    exact noInner_of_conforms hc (slotType_of_ofSignature hctx.targs hctx.sig t ht)
  · refine ⟨locals_noInner hs (ChainRef.locals hchain g hg), List.all_eq_true.2 fun v hv => ?_⟩
    obtain ⟨t, hc, ht⟩ := ChainRef.stack hchain g hg v hv
    exact noInner_of_conforms hc ht

end M3

/-- **No reference inside a value, in storage or among the held values**, in
every state of a verified execution. -/
theorem unnested_M3 {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}
    (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.SupportedNonGeneric) (hm3 : P.M3)
    (hN : NativesRespect P N) (h0 : AdmissibleInitial P s₀) {s : State} {h : Shadow}
    {E : List Event} (hT : Trace P N ν s₀ s h E) : RefSafety.unnested s = true := by
  have hI := refTraceInv_trace h0 hT
  have hinv := invM3_reachable hwf hs hm3 (certified_of_wellFormed schedulerSound hwf hs) hN h0 s
    (trace_reachable hT)
  have hfr := frames_unnested hwf hs hm3 hinv
  simp only [RefSafety.unnested, Bool.and_eq_true, List.all_eq_true]
  refine ⟨⟨fun fr hfr' => ?_, fun r hr => ?_⟩, fun v hv => ?_⟩
  · obtain ⟨h1, h2⟩ := hfr fr hfr'
    exact ⟨List.all_eq_true.1 h1, List.all_eq_true.1 h2⟩
  · exact referenceFree_of_plain (hI.pre.refsTop.storage r hr)
  · obtain ⟨hsh, hval⟩ := hI.typed.held v hv
    exact referenceFree_of_plain (shaped_noRef v hsh hval)

/-- What a returning `Ret` hands back: the frame's operand stack, and global
storage as it is. -/
theorem ret_returned {P : Program} {s : State} {fr : Frame} {rest : List Frame}
    {info : FunctionInfo} {code : CodeUnit} {vs : List Value} {st : GlobalStorage}
    (h : stepOf (Step.exec P s fr rest info code .ret) = .done (.returned vs st)) :
    vs = fr.stack.reverse ∧ st = s.storage := by
  have hcont : (Outcome.returned vs st).continues = true := rfl
  unfold Step.exec Step.ret at h
  obtain ⟨retTys, -, h⟩ := done_bind h hcont (ends_need _ _)
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  cases rest with
  | nil =>
    simp only [stepOf, pure, Except.pure, StepResult.done.injEq, Outcome.returned.injEq] at h
    obtain ⟨rfl, rfl⟩ := h
    refine ⟨rfl, ?_⟩
    split <;> (try split) <;> rfl
  | cons caller rest' =>
    simp only [stepOf, pure, Except.pure, StepResult.done.injEq] at h
    cases h

/-- A traced step's annotation fits its outcome. -/
theorem traced_fits {P : Program} {N : NativeRel} {ν : NativeFlows} {s : State} {h : Shadow}
    {o : Outcome} {post : ShadowPost} {evs : List Event} (hS : TracedStep P N ν s h o post evs) :
    post.fits o = true := by
  cases hS with
  | core _ hc => exact coreStep_fits hc
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
    case inr =>
      obtain ⟨g, hs, info, tys, ptys, seen, s', fr', tl, newRefs, resultTrees, h₂, -, -, -, -, -,
        -, -, -, -, -, -, -, -, -, -, hpost, hfit, -⟩ := nativeStep_spec hn
      rw [hpost]
      exact hfit
    obtain ⟨rfl, rfl, rfl, -, -, -, -, -, -, -, -, hfit⟩ := nativeStep_some hstep hshape hn
    exact hfit

/-- A native call does not return from the transaction. -/
theorem resumeNative_not_returned {P : Program} {s : State} {f : FunctionRef}
    {args vals writes : List Value} {places : List NativePlace} {vs : List Value}
    {st : GlobalStorage} : resumeNative P s f args (.returned vals writes places) ≠ .returned vs st := by
  intro h
  simp only [resumeNative] at h
  repeat' split at h
  all_goals simp at h

/-- A traced step that returns from the transaction is a core step. -/
theorem traced_returned {P : Program} {N : NativeRel} {ν : NativeFlows} {s : State} {h : Shadow}
    {o : Outcome} {post : ShadowPost} {evs : List Event} (hS : TracedStep P N ν s h o post evs)
    {vs : List Value} {st : GlobalStorage} (ho : o = .returned vs st) :
    step P s = .done (.returned vs st) ∧ Shadow.coreStep P s (.returned vs st) h = some (post, evs) := by
  cases hS with
  | core hstep hc => subst ho; exact ⟨hstep, hc⟩
  | native _ _ _ => exact absurd ho resumeNative_not_returned

/-- **A verified step hands back no reference inside a value**, in the next
state, or in the returned values and committed storage. -/
theorem unnestedAfter_M3 {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}
    (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.SupportedNonGeneric) (hm3 : P.M3)
    (hN : NativesRespect P N) (h0 : AdmissibleInitial P s₀) {s : State} {h : Shadow}
    {E : List Event} (hT : Trace P N ν s₀ s h E) {o : Outcome} {post : ShadowPost}
    {evs : List Event} (hS : TracedStep P N ν s h o post evs) :
    RefSafety.unnestedAfter o = true := by
  have hI := refTraceInv_trace h0 hT
  have hfit := traced_fits hS
  cases o with
  | running s' =>
    cases post with
    | running h' => exact unnested_M3 hwf hs hm3 hN h0 (.step hT hS)
    | returned _ _ => simp [ShadowPost.fits] at hfit
  | returned vs st =>
    obtain ⟨hstep, hc⟩ := traced_returned hS rfl
    obtain ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hinstr, hex⟩ := coreStep_exec hc
    cases post with
    | running _ => simp [ShadowPost.fits] at hfit
    | returned ts sts =>
      obtain ⟨hframes, hst, hhe, fresh⟩ := h
      cases hframes with
      | nil => simp [Shadow.exec] at hex
      | cons g hs' =>
        have hi := exec_post_returned rfl hex
        subst hi
        have hst' : stepOf (Step.exec P s fr rest info code .ret) = .done (.returned vs st) := by
          unfold step at hstep
          rw [hsf] at hstep
          simp only [hinfo, hcode, hinstr] at hstep
          exact hstep
        obtain ⟨rfl, rfl⟩ := ret_returned hst'
        have hinv := invM3_reachable hwf hs hm3 (certified_of_wellFormed schedulerSound hwf hs)
          hN h0 s (trace_reachable hT)
        have hfr := (frames_unnested hwf hs hm3 hinv fr (by rw [hsf]; exact List.mem_cons_self ..)).2
        simp only [RefSafety.unnestedAfter, Bool.and_eq_true, List.all_eq_true, List.mem_reverse]
        exact ⟨fun v hv => List.all_eq_true.1 hfr v hv,
          fun r hr => referenceFree_of_plain (hI.pre.refsTop.storage r hr)⟩
  | failed _ => rfl
  | wrong _ => rfl

end MoveBytecode.Proofs
