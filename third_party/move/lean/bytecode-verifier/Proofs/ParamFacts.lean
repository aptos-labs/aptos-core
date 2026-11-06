-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.ExclusiveDefs
import Proofs.ReturnsDerived
import Proofs.ApiCore

/-!
# D4b″: what a frame's reference parameters keep

UNTRUSTED (checked by the kernel). Along every annotated execution of an M3
program, each frame's reference parameters keep `ParamsOk` (`ParamChain`):
the entry frame's are references to whole held values at distinct locations;
a callee's are its caller's arguments, which the analysis checked at the call
(`call_suspend`): compatible, tracked (so in no local of the callee), and in
global types the callee protects, which include those the caller protects.
Every other step keeps each frame's identity, function and activation, and
borrows no parameter anew.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model MoveBytecode.Step

/-! ## Lists of references -/

theorem compat_symm {d r : Reference} (h : Compat d r) : Compat r d :=
  ⟨h.2.1, h.1, h.2.2.2, h.2.2.1⟩

/-- Distinct members of a list pairwise related by a symmetric relation are
related. -/
theorem pairwise_forall {α : Type} {R : α → α → Prop} (hs : ∀ a b, R a b → R b a) :
    ∀ {l : List α}, l.Pairwise R → ∀ a ∈ l, ∀ b ∈ l, a ≠ b → R a b
  | [], _, a, ha, _, _, _ => by simp at ha
  | x :: l, h, a, ha, b, hb, hab => by
    rw [List.pairwise_cons] at h
    rcases List.mem_cons.1 ha with ha' | ha' <;> rcases List.mem_cons.1 hb with hb' | hb'
    · exact absurd (ha'.trans hb'.symm) hab
    · subst ha'; exact h.1 b hb'
    · subst hb'; exact hs _ _ (h.1 a ha')
    · exact pairwise_forall hs h.2 a ha' b hb' hab

/-- The references among values with identities are the references among the
values. -/
theorem slotRefs_refs : ∀ {ts : List IdTree} {vs : List Value}, ts.length = vs.length →
    (slotRefs ts vs).map (·.2) = argRefs vs
  | [], [], _ => rfl
  | t :: ts, v :: vs, h => by
    have ih := slotRefs_refs (ts := ts) (vs := vs) (by simpa using h)
    cases v <;> simp_all [slotRefs, argRefs]
  | [], _ :: _, h => by simp at h
  | _ :: _, [], h => by simp at h

/-- The references of an initial frame's parameters are those among its
locals. -/
theorem initParams_refs : ∀ (L : List (Option IdTree × Option Value)),
    L.all (fun p => ShadowFrame.slotFits p.1 p.2) = true →
    (L.filterMap fun (t, v) => match t, v with
        | some t, some (.ref r) => some (t.id, r)
        | _, _ => none).map (·.2) =
      (L.map (·.2)).filterMap fun v => match v with
        | some (.ref r) => some r
        | _ => none
  | [], _ => rfl
  | (ot, ov) :: L, h => by
    simp only [List.all_cons, Bool.and_eq_true] at h
    have ih := initParams_refs L h.2
    cases ot with
    | none =>
      cases ov with
      | none => simpa using ih
      | some v => simp [ShadowFrame.slotFits] at h
    | some t =>
      cases ov with
      | none => simp [ShadowFrame.slotFits] at h
      | some v => cases v <;> simpa using ih

/-! ## A call, concretely -/

section Concrete

variable {P : Program} {s s' : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit}

/-- **What `Call` of a function with code does**, with the facts the
analysis's call needs. -/
theorem call_full {hd : FunctionHandleIndex}
    (h : stepOf (Step.exec P s fr rest info code (.call hd)) = .done (.running s')) :
    ∃ callee cinfo paramTys retTys frame,
      P.resolve fr.function.module hd = some callee ∧ P.function? callee = some cinfo ∧
      P.returnTypes callee = some retTys ∧
      P.handleParamTypes fr.function.module hd = some paramTys ∧
      P.handleReturnTypes fr.function.module hd = some retTys ∧
      paramTys.length ≤ fr.stack.length ∧
      frame.function = callee ∧ frame.countedActive = (cinfo.moduleId != info.moduleId) ∧
      frame.id = s.nextFrame ∧
      s'.frames = frame :: { fr with stack := fr.stack.drop paramTys.length } :: rest := by
  unfold Step.exec Step.call at h
  obtain ⟨callee, hcal, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨cinfo, hci, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨fh, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  dsimp only at h
  rw [withTypeArgs_self (resolve_typeArgs (need_ok' hcal))] at h
  obtain ⟨paramTys, hpt, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨retTys, hrt, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨hps, hhp, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨hrs, hhr, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  next hlink =>
  simp only [Bool.or_eq_true, bne_iff_ne, ne_eq, not_or, Decidable.not_not] at hlink
  obtain ⟨rfl, rfl⟩ := hlink
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨⟨args, fr₁⟩, hpop, h⟩ := done_bind h rfl popN_ends
  obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_native h rfl).elim
  obtain ⟨localTys, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h <;> split at h
  all_goals first
    | exact (done_fail h rfl).elim
    | skip
  all_goals
    simp only [stepOf, pure, Except.pure, StepResult.done.injEq, Outcome.running.injEq] at h
    subst h
    exact ⟨callee, cinfo, hps, hrs, _, need_ok' hcal, need_ok' hci, need_ok' hrt, need_ok' hhp,
      need_ok' hhr, hn, rfl, rfl, by (try split) <;> rfl, rfl⟩

/-- **What `Call` or `CallGeneric` of a function with code does**, at type
arguments `tys`, with the facts the analysis's call needs. -/
theorem call_fullAt {hd : FunctionHandleIndex} {tys : List RuntimeType}
    (h : stepOf (Step.call P s fr rest info hd tys) = .done (.running s')) :
    ∃ callee cinfo paramTys retTys frame,
      P.resolve fr.function.module hd = some callee ∧ P.function? callee = some cinfo ∧
      P.returnTypes { callee with typeArgs := tys } = some retTys ∧
      P.handleParamTypesAt fr.function.module hd tys = some paramTys ∧
      P.handleReturnTypesAt fr.function.module hd tys = some retTys ∧
      paramTys.length ≤ fr.stack.length ∧
      frame.function = { callee with typeArgs := tys } ∧
      frame.countedActive = (cinfo.moduleId != info.moduleId) ∧
      frame.id = s.nextFrame ∧
      s'.frames = frame :: { fr with stack := fr.stack.drop paramTys.length } :: rest := by
  unfold Step.call at h
  obtain ⟨callee, hcal, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨cinfo, hci, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨fh, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_wrong h rfl).elim
  dsimp only at h
  obtain ⟨paramTys, hpt, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨retTys, hrt, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨hps, hhp, h⟩ := done_bind h rfl (ends_need _ _)
  obtain ⟨hrs, hhr, h⟩ := done_bind h rfl (ends_need _ _)
  split at h
  · exact (done_wrong h rfl).elim
  next hlink =>
  simp only [Bool.or_eq_true, bne_iff_ne, ne_eq, not_or, Decidable.not_not] at hlink
  obtain ⟨rfl, rfl⟩ := hlink
  split at h
  · exact (done_wrong h rfl).elim
  obtain ⟨⟨args, fr₁⟩, hpop, h⟩ := done_bind h rfl popN_ends
  obtain ⟨hn, rfl, rfl⟩ := popN_ok hpop
  dsimp only at h
  split at h
  · exact (done_wrong h rfl).elim
  split at h
  · exact (done_native h rfl).elim
  obtain ⟨localTys, -, h⟩ := done_bind h rfl (ends_need _ _)
  split at h <;> split at h
  all_goals first
    | exact (done_fail h rfl).elim
    | skip
  all_goals
    simp only [stepOf, pure, Except.pure, StepResult.done.injEq, Outcome.running.injEq] at h
    subst h
    exact ⟨callee, cinfo, hps, hrs, _, need_ok' hcal, need_ok' hci, need_ok' hrt, need_ok' hhp,
      need_ok' hhr, hn, rfl, rfl, by (try split) <;> rfl, rfl⟩

end Concrete

/-- The analysis runs a call of handle `h`, `Call` or `CallGeneric`, as
`Call(h)`. -/
theorem callsHandle_run {m : CompiledModule} {instr : Bytecode} {h : Nat}
    (hc : CallsHandle m instr h) {view : FunctionView} {A A' : RefState}
    (hx : (ReferenceSafety.executeInner m view instr).run A = .ok ((), A')) :
    (ReferenceSafety.executeInner m view (.call h)).run A = .ok ((), A') := by
  rcases hc with rfl | ⟨i, fi, rfl, hfi, rfl⟩
  · exact hx
  · obtain ⟨fi', hfi', h'⟩ := exec_callGeneric' hx
    rw [hfi, Option.some.injEq] at hfi'
    subst hfi'
    exact h'

/-! ## Monotonicity -/

/-- `ParamsOk` over more events that borrow no parameter anew, for a frame
with the same identity and function. -/
theorem ParamsOk.mono {P : Program} {E E' : List Event} {fr fr' : Frame}
    {sk : List (FunctionRef × Bool)} {ps : List Nat} {T : StructId → Prop}
    (h : ParamsOk P E fr sk ps T) (hsub : ∀ e ∈ E, e ∈ E')
    (hback : ∀ p ∈ borrowedIds E, ∀ rp par, Event.borrowed p rp par ∈ E' →
      Event.borrowed p rp par ∈ E)
    (hid : fr'.id = fr.id) (hfun : fr'.function = fr.function) : ParamsOk P E' fr' sk ps T where
  mem p hp := by
    obtain ⟨r, par, hb⟩ := mem_borrowedIds.1 (h.mem p hp)
    exact mem_borrowedIds.2 ⟨r, par, hsub _ hb⟩
  below p hp rp par hb f i hρ := by
    rw [hid]; exact h.below p hp rp par (hback p (h.mem p hp) rp par hb) f i hρ
  glob p hp rp par hb := h.glob p hp rp par (hback p (h.mem p hp) rp par hb)
  compat p hp q hq hpq rp rq pp pq hbp hbq :=
    h.compat p hp q hq hpq rp rq pp pq (hback p (h.mem p hp) rp pp hbp)
      (hback q (h.mem q hq) rq pq hbq)
  active := h.active
  acq t ht hacq := h.acq t ht (by rw [← hfun]; exact hacq)

/-- `ParamChain` over more events that borrow no parameter anew, for frames
with the same keys. -/
theorem ParamChain.mono {P : Program} {E E' : List Event} (hsub : ∀ e ∈ E, e ∈ E')
    (hback : ∀ p ∈ borrowedIds E, ∀ rp par, Event.borrowed p rp par ∈ E' →
      Event.borrowed p rp par ∈ E)
    (hpar : ∀ f ps, RefSafety.params? E f = some ps → RefSafety.params? E' f = some ps) :
    ∀ {frs frs' : List Frame} {T : StructId → Prop}, ParamChain P E frs T →
      frs'.map Frame.key = frs.map Frame.key → ParamChain P E' frs' T
  | [], [], _, _, _ => trivial
  | fr :: frs, fr' :: frs', T, ⟨ps, hp, hok, hids, T', hT', hrest⟩, hk => by
    have hk' := hk
    simp only [List.map_cons, List.cons.injEq] at hk'
    refine ⟨ps, by rw [key_id hk'.1]; exact hpar _ _ hp, ?_, fun x hx => ?_, T', hT',
      ParamChain.mono hsub hback hpar hrest hk'.2⟩
    · rw [map_key_skel hk]
      exact hok.mono hsub hback (key_id hk'.1) (key_function hk'.1)
    · obtain ⟨y, hy, hyx⟩ := List.mem_map.1 (hk'.2 ▸ List.mem_map_of_mem (f := Frame.key) hx)
      rw [← key_id hyx, key_id hk'.1]
      exact hids y hy
  | [], _ :: _, _, _, hk => by simp at hk
  | _ :: _, [], _, _, hk => by simp at hk

/-- Events appended to a trace's borrow no identity anew. -/
theorem back_of_nodup {E F : List Event} (hnd : (borrowedIds (E ++ F)).Nodup) :
    ∀ p ∈ borrowedIds E, ∀ rp par, Event.borrowed p rp par ∈ E ++ F →
      Event.borrowed p rp par ∈ E := by
  intro p hp rp par hb
  rcases List.mem_append.1 hb with hb | hb
  · exact hb
  · rw [borrowedIds_append, List.nodup_append] at hnd
    exact absurd rfl (hnd.2.2 p hp p (mem_borrowedIds.2 ⟨rp, par, hb⟩))

/-! ## The initial state -/

/-- **The entry frame's parameters** are references to whole held values at
distinct locations; it protects nothing. -/
theorem paramChain_initial {P : Program} {s₀ : State} (h0 : AdmissibleInitial P s₀) :
    ∃ T, ParamChain P (Shadow.initial s₀).2 s₀.frames T := by
  obtain ⟨fr, g, hfr, hg, hcls, hin⟩ := initial_events_entry h0
  have hI := refTraceInv_initial h0
  have hfit : g.fits fr = true := by
    have := hI.pre.fits
    rw [shadow_fits, hfr, hg, framesFit_cons, Bool.and_eq_true, Bool.and_eq_true] at this
    exact this.1.1
  obtain ⟨f, args, tys, storage, held, -, -, -, hinit⟩ := h0
  obtain ⟨fr', localTys, hfr', -, -, -, -, hloc, -, -, -, hha⟩ := initialState_shape hinit
  rw [hfr, List.cons.injEq] at hfr'
  obtain ⟨rfl, -⟩ := hfr'
  obtain ⟨hrefs, hnd⟩ := heldArguments_refs hha
  -- The parameters' references are the arguments' references.
  have hsize : g.locals.toList.length = fr.locals.toList.length := by
    simp only [frame_fits, Bool.and_eq_true, beq_iff_eq] at hfit
    simpa using hfit.1.1
  have hslots : (g.locals.toList.zip fr.locals.toList).all
      (fun p => ShadowFrame.slotFits p.1 p.2) = true := by
    simp only [frame_fits, slotsFit, Bool.and_eq_true] at hfit
    exact hfit.1.2
  have hmap : (initParams g fr).map (·.2) = argRefs args := by
    refine Eq.trans (initParams_refs _ hslots) ?_
    rw [List.map_snd_zip (by omega), hloc, List.toList_toArray, List.filterMap_append,
      List.filterMap_map]
    have hnone : (List.replicate (localTys.length - args.length) (none : Option Value)).filterMap
        (fun v => match v with
          | some (.ref r) => some r
          | _ => none) = [] := by
      simp
    rw [hnone, List.append_nil]
    simp only [argRefs]
    congr 1
    funext v
    cases v <;> rfl
  -- The borrowed events of the initial events are the parameters'.
  have hbor : ∀ p rp par, Event.borrowed p rp par ∈ (Shadow.initial s₀).2 →
      (p, rp) ∈ initParams g fr := by
    intro p rp par hb
    rcases hcls _ hb with ⟨_, _, _, h⟩ | hb
    · cases h
    · simp only [initRefs, List.mem_append, List.mem_map, List.mem_singleton] at hb
      rcases hb with ⟨⟨i, r⟩, hir, h⟩ | h
      · simp only [Event.borrowed.injEq] at h
        obtain ⟨rfl, rfl, -⟩ := h
        exact hir
      · cases h
  have harg : ∀ p rp, (p, rp) ∈ initParams g fr → rp ∈ argRefs args := fun p rp h => by
    rw [← hmap]; exact List.mem_map.2 ⟨(p, rp), h, rfl⟩
  have hpw : (initParams g fr).Pairwise (fun x y => Compat x.2 y.2) := by
    have := compat_of_held (fun r hr => (hrefs r hr).2.1) hnd
    rw [← hmap] at this
    exact List.pairwise_map.1 this
  have hp : RefSafety.params? (Shadow.initial s₀).2 fr.id = some ((initParams g fr).map (·.1)) := by
    refine params?_of_mem (fun e he f' ps' hee hf' => ?_) (hin _ (by simp [initRefs]))
    subst hee
    rcases hcls _ he with ⟨_, _, _, h⟩ | he
    · cases h
    · simp only [initRefs, List.mem_append, List.mem_map, List.mem_singleton] at he
      rcases he with ⟨_, -, h⟩ | h
      · cases h
      · simp only [Event.entered.injEq] at h
        exact h.2
  refine ⟨fun _ => False, ?_⟩
  rw [hfr]
  refine ⟨(initParams g fr).map (·.1), hp, ⟨?_, ?_, ?_, ?_, fun _ h => h.elim,
    fun _ h => h.elim⟩, by simp, fun _ => False, fun _ h => h, trivial⟩
  · intro p hp'
    obtain ⟨⟨i, r⟩, hir, rfl⟩ := List.mem_map.1 hp'
    refine mem_borrowedIds.2 ⟨r, none, hin _ ?_⟩
    simp only [initRefs, List.mem_append, List.mem_map]
    exact Or.inl ⟨(i, r), hir, rfl⟩
  · intro p _ rp par hb f' i hρ
    obtain ⟨-, -, k, v, hroot, -⟩ := hrefs rp (harg p rp (hbor p rp par hb))
    rw [hroot] at hρ
    cases hρ
  · intro p _ rp par hb a t hρ
    obtain ⟨-, -, k, v, hroot, -⟩ := hrefs rp (harg p rp (hbor p rp par hb))
    rw [hroot] at hρ
    cases hρ
  · intro p _ q _ hpq rp rq pp pq hbp hbq
    exact pairwise_forall (fun _ _ h => compat_symm h) hpw (p, rp) (hbor p rp pp hbp) (q, rq)
      (hbor q rq pq hbq) (fun e => hpq (congrArg Prod.fst e))

/-! ## Steps -/

section Steps

variable {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}
  (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.Supported)
  (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
    ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)
  (hN : NativesRespect P N) (h0 : AdmissibleInitial P s₀)
include hwf hs hapi hN h0

omit hN in
/-- **A call gives the callee's parameters their facts**, by the analysis's
checks at the call: they are compatible, tracked by the caller (so in its
locals or older roots), and in global types the callee protects, which
include those the caller protects. -/
theorem paramChain_call {s s' : State} {h : Shadow} {E : List Event} (hT : Trace P N ν s₀ s h E)
    (hinv : InvRefs P s)
    {T : StructId → Prop} (hC : ParamChain P E s.frames T) {fr : Frame} {rest : List Frame}
    (hsf : s.frames = fr :: rest) {g : ShadowFrame} {hs' : List ShadowFrame}
    {hst hhe : List IdTree} {f : Nat} (hhf : h = ⟨g :: hs', hst, hhe, f⟩)
    {info : FunctionInfo} {code : CodeUnit} {instr : Bytecode} {hd : FunctionHandleIndex}
    {τ : List RuntimeType}
    (hinfo : P.function? fr.function = some info) (hcode : info.definition.code = some code)
    (hinstr : code.code[fr.pc]? = some instr) (hcallI : CallsHandle info.module instr hd)
    (hattrH : ∀ fh, info.module.functionHandles[hd]? = some fh →
      ReferenceSafety.borrowFieldMutable fh.attributes = none)
    (hτ : ∀ x ∈ τ, x.isValue = true)
    (hstep' : stepOf (Step.call P s fr rest info hd τ) = .done (.running s'))
    {post : ShadowPost} {evs : List Event}
    (hex : Shadow.exec P info.moduleId (.call hd) s fr (.running s') h = some (post, evs))
    (hnd : (borrowedIds (E ++ evs)).Nodup) :
    ∃ T', ParamChain P (E ++ evs) s'.frames T' := by
  subst hhf
  have hI := refTraceInv_trace h0 hT
  -- The concrete call and its annotation.
  obtain ⟨callee, cinfo, params, rets, frame, hres, hcf, hrt, hhp, hhr, hn, hfun, hca, hid, h2f⟩ :=
    call_fullAt hstep'
  have hg : g.fits fr = true := (pre_head hI.pre hsf).1
  have hgl := fitsAll_length (frame_stack_fits hg)
  obtain ⟨-, hevs⟩ := call_shadow hn hgl h2f hex
  have hret' : instr ≠ .ret := by
    rcases hcallI with rfl | ⟨i, fi, rfl, -⟩ <;> intro h <;> cases h
  have hctl : instr.control = .fallThrough := by
    rcases hcallI with rfl | ⟨i, fi, rfl, -⟩ <;> rfl
  -- The analysis at the call.
  obtain ⟨fr₀, rest₀, hsf₀, ⟨info₀, code₀, hact⟩, -⟩ := hinv.1
  rw [hsf, List.cons.injEq] at hsf₀
  obtain ⟨rfl, rfl⟩ := hsf₀
  have e1 := hact.fn
  rw [hinfo, Option.some.injEq] at e1
  subst e1
  have e2 := hact.body
  rw [hcode, Option.some.injEq] at e2
  subst e2
  obtain ⟨view, A, stys, tys, ps, hctx⟩ :=
    topCtxGen hwf hs (apiSigs_of hwf hapi) hinv hsf hact hinstr
  obtain ⟨A', hx, -⟩ := hctx.next hret'
  obtain ⟨fh, hfh, -⟩ := exec_call (callsHandle_run hcallI hx)
  have hattr := hattrH fh hfh
  obtain ⟨Sw, hglobW, -⟩ := hctx.chain _ hsf
  rw [hsf] at hC
  obtain ⟨ps₀, hp₀, hok₀, hids₀, T₀, hT₀, hrest₀⟩ := hC
  have hglob : GlobOk P fr.function ((fr :: rest).map Frame.skel) ps (fun t => T t ∨ Sw t) :=
    ⟨fun t ht => ht.elim (hok₀.active t) (hglobW.active t),
     fun t ht hacq => ht.elim (fun h => hok₀.acq t h hacq) (fun h => hglobW.acq t h hacq),
     fun π hπ a t hρ => Or.inr (hglobW.params π hπ a t hρ)⟩
  have hcid : fr.id < s.nextFrame := hinv.2.below fr (by rw [hsf]; exact List.mem_cons_self ..)
  obtain ⟨Sc, hexcl, hargOk, hsusp, hglobC⟩ := call_suspendAt hwf hctx hact.locals
    (hI.typed.args fr (by rw [hsf]; exact List.mem_cons_self ..)) hinstr hcallI hret' hctl
    (fun _ hx => callsHandle_run hcallI hx) hact.fn hact.body hfh hattr hres hcf hτ hhp hhr hrt hn
    (cid := frame.id) (by rw [hid]; exact hcid) hsf hinv.2.active.counted hglob
  -- The callee's parameters are the reference arguments, position for position.
  obtain ⟨R, hR⟩ : ∃ R, R = slotRefs (g.stack.take params.length).reverse
      (fr.stack.take params.length).reverse := ⟨_, rfl⟩
  have hRmap : R.map (·.2) = argRefs (fr.stack.take params.length).reverse := by
    rw [hR]; exact slotRefs_refs (by simp [hgl])
  rw [← hR] at hevs
  have hocc : ∀ p r, (p, r) ∈ R → (p, Value.ref r) ∈ framesOcc (g :: hs') s.frames := by
    intro p r hm
    rw [hR] at hm
    obtain ⟨t, hz, rfl⟩ := mem_slotRefs.1 hm
    rw [hsf, framesOcc_cons]
    exact List.mem_append_left _ (refSlot_occ hg ⟨t, rfl, Or.inl (zip_rev_take hgl hz)⟩)
  -- A parameter's borrowed event is its argument's.
  have hbR : ∀ p ∈ R.map (·.1), ∀ rp par, Event.borrowed p rp par ∈ E ++ evs → (p, rp) ∈ R := by
    intro p hp rp par hb
    obtain ⟨⟨p', r⟩, hm, rfl⟩ := List.mem_map.1 hp
    obtain ⟨par', hb'⟩ := hI.refs p' r (hocc p' r hm)
    obtain ⟨rfl, -⟩ := borrowed_unique hnd (List.mem_append_left _ hb') hb
    exact hm
  have harg : ∀ p rp, (p, rp) ∈ R → rp ∈ argRefs (fr.stack.take params.length).reverse :=
    fun p rp h => by rw [← hRmap]; exact List.mem_map.2 ⟨(p, rp), h, rfl⟩
  have hpw : R.Pairwise (fun x y => Compat x.2 y.2) := by
    rw [← hRmap] at hexcl
    exact List.pairwise_map.1 hexcl
  -- The callee is entered once, with these parameters.
  have hpE : RefSafety.params? E frame.id = none :=
    params?_none_of fun f ps he => by rw [hid]; exact trace_entered_lt h0 hT f ps he
  have hpev : RefSafety.params? evs frame.id = some (R.map (·.1)) := by
    rw [hevs]
    refine params?_of_mem (fun e he f' ps' hee hf' => ?_) (by simp)
    subst hee
    simp only [List.mem_append, List.mem_map, List.mem_singleton] at he
    rcases he with ⟨_, -, h⟩ | h
    · cases h
    · simp only [Event.entered.injEq] at h
      exact h.2
  have hsk : (frame :: { fr with stack := fr.stack.drop params.length } :: rest).map Frame.skel =
      ({ callee with typeArgs := τ }, cinfo.moduleId != info.moduleId) ::
        (fr :: rest).map Frame.skel := by
    simp only [List.map_cons, Frame.skel, hfun, hca]
  have hback := back_of_nodup hnd
  have hsub : ∀ e ∈ E, e ∈ E ++ evs := fun e he => List.mem_append_left _ he
  refine ⟨Sc, ?_⟩
  rw [h2f]
  refine ⟨R.map (·.1), by rw [params?_append_none hpE]; exact hpev, ?_, fun x hx => ?_, T,
    fun t ht => hsusp.sub t (Or.inl ht), ?_⟩
  · rw [hsk]
    refine ⟨fun p hp => ?_, fun p hp rp par hb f' i hρ => ?_, fun p hp rp par hb a t hρ => ?_,
      fun p hp q hq hpq rp rq pp pq hbp hbq => ?_, hglobC.active,
      fun t ht hacq => hglobC.acq t ht (by rw [← hfun]; exact hacq)⟩
    · obtain ⟨⟨p', r⟩, hm, rfl⟩ := List.mem_map.1 hp
      obtain ⟨par', hb'⟩ := hI.refs p' r (hocc p' r hm)
      rw [borrowedIds_append]
      exact List.mem_append_left _ (mem_borrowedIds.2 ⟨r, par', hb'⟩)
    · have hb' := (hargOk rp (harg p rp (hbR p hp rp par hb))).below
      rw [hρ] at hb'
      rw [hid]
      exact Nat.lt_of_le_of_lt hb' hcid
    · exact hglobC.params rp (harg p rp (hbR p hp rp par hb)) a t hρ
    · exact pairwise_forall (fun _ _ h => compat_symm h) hpw (p, rp) (hbR p hp rp pp hbp) (q, rq)
        (hbR q hq rq pq hbq) (fun e => hpq (congrArg Prod.fst e))
  · rw [hid]
    simp only [List.mem_cons] at hx
    rcases hx with rfl | hx
    · exact hcid
    · exact hinv.2.below x (by rw [hsf]; exact List.mem_cons_of_mem _ hx)
  · exact ⟨ps₀, params?_append hp₀, hok₀.mono hsub hback rfl rfl, hids₀, T₀, hT₀,
      ParamChain.mono hsub hback (fun _ _ h => params?_append h) hrest₀ rfl⟩

omit hN in
/-- **A struct-API call gives the wrapper's parameter its facts**: its one
parameter is the caller's argument, which the caller tracks (so it is in the
caller's locals or older roots), into a global type, if any, whose module is
active; the wrapper acquires nothing. -/
theorem paramChain_callApi {s s' : State} {h : Shadow} {E : List Event}
    (hT : Trace P N ν s₀ s h E) (hinv : InvRefs P s)
    {T : StructId → Prop} (hC : ParamChain P E s.frames T) {fr : Frame} {rest : List Frame}
    (hsf : s.frames = fr :: rest) {g : ShadowFrame} {hs' : List ShadowFrame}
    {hst hhe : List IdTree} {f : Nat} (hhf : h = ⟨g :: hs', hst, hhe, f⟩)
    {info : FunctionInfo} {code : CodeUnit} {instr : Bytecode} {hd : FunctionHandleIndex}
    {τ : List RuntimeType}
    (hinfo : P.function? fr.function = some info) (hcode : info.definition.code = some code)
    (hinstr : code.code[fr.pc]? = some instr) (hcallI : CallsHandle info.module instr hd)
    {fh : FunctionHandle}
    (hfh : info.module.functionHandles[hd]? = some fh) {off : Nat}
    (hoff : ReferenceSafety.borrowFieldMutable fh.attributes = some off)
    (hstep' : stepOf (Step.call P s fr rest info hd τ) = .done (.running s'))
    {post : ShadowPost} {evs : List Event}
    (hex : Shadow.exec P info.moduleId (.call hd) s fr (.running s') h = some (post, evs))
    (hrun : ApiCore P s' s) (hnd : (borrowedIds (E ++ evs)).Nodup) :
    ∃ T', ParamChain P (E ++ evs) s'.frames T' := by
  subst hhf
  have hI := refTraceInv_trace h0 hT
  obtain ⟨callee, cinfo, params, rets, frame, hres, hcf, hrt, hhp, hhr, hn, hfun, hca, hid, h2f⟩ :=
    call_fullAt hstep'
  have hg : g.fits fr = true := (pre_head hI.pre hsf).1
  have hgl := fitsAll_length (frame_stack_fits hg)
  obtain ⟨-, hevs⟩ := call_shadow hn hgl h2f hex
  obtain ⟨fr₀, rest₀, hsf₀, ⟨info₀, code₀, hact⟩, -⟩ := hinv.1
  rw [hsf, List.cons.injEq] at hsf₀
  obtain ⟨rfl, rfl⟩ := hsf₀
  have e1 := hact.fn
  rw [hinfo, Option.some.injEq] at e1
  subst e1
  have e2 := hact.body
  rw [hcode, Option.some.injEq] at e2
  subst e2
  -- The state after the call is the wrapper in progress, over the argument `x`.
  obtain ⟨fr₁, rest₁, info₁, code₁, off₁, x, st, w, cinfo₁, hsc, -, hst, -, hs'f, -⟩ := hrun.call
  rw [hsf, List.cons.injEq] at hsc
  obtain ⟨rfl, rfl⟩ := hsc
  have hs'f' := hs'f
  rw [h2f, List.cons.injEq, List.cons.injEq] at hs'f'
  obtain ⟨hfw, hfr', -⟩ := hs'f'
  have hdrop : fr.stack.drop params.length = st := congrArg Frame.stack hfr'
  have hn1 : params.length = 1 := by
    have h1 := congrArg List.length hdrop
    rw [hst, List.length_drop] at h1
    rw [hst] at hn
    simp only [List.length_cons] at h1 hn
    omega
  have hnoacq : ∀ t, ¬ acqTypes P frame.function t := fun t => by
    rw [hfw]; exact apiCore_noAcq hrun hs'f
  obtain ⟨hxok, hxact⟩ := apiArg_facts hwf hs hapi hinv hsf hact hinstr hcallI hfh hoff hst
  have hcid : fr.id < s.nextFrame := hinv.2.below fr (by rw [hsf]; exact List.mem_cons_self ..)
  -- The wrapper's one parameter is the argument.
  have hfa := frame_stack_fits hg
  rw [hst] at hfa
  obtain ⟨t0, ts0, hgst, -, -⟩ := fitsAll_cons_right hfa
  obtain ⟨R, hR⟩ : ∃ R, R = slotRefs (g.stack.take params.length).reverse
      (fr.stack.take params.length).reverse := ⟨_, rfl⟩
  rw [← hR] at hevs
  have hRx : ∀ p r, (p, r) ∈ R → p = t0.id ∧ r = x := by
    intro p r hm
    rw [hR, hn1, hgst, hst] at hm
    obtain ⟨t, hz, rfl⟩ := mem_slotRefs.1 hm
    simp only [List.take_succ_cons, List.take_zero, List.reverse_cons, List.reverse_nil,
      List.nil_append, List.zip_cons_cons, List.zip_nil_left, List.mem_singleton,
      Prod.mk.injEq, Value.ref.injEq] at hz
    obtain ⟨rfl, rfl⟩ := hz
    exact ⟨rfl, rfl⟩
  have hocc : ∀ p r, (p, r) ∈ R → (p, Value.ref r) ∈ framesOcc (g :: hs') s.frames := by
    intro p r hm
    rw [hR] at hm
    obtain ⟨t, hz, rfl⟩ := mem_slotRefs.1 hm
    rw [hsf, framesOcc_cons]
    exact List.mem_append_left _ (refSlot_occ hg ⟨t, rfl, Or.inl (zip_rev_take hgl hz)⟩)
  have hbR : ∀ p ∈ R.map (·.1), ∀ rp par, Event.borrowed p rp par ∈ E ++ evs → (p, rp) ∈ R := by
    intro p hp rp par hb
    obtain ⟨⟨p', r⟩, hm, rfl⟩ := List.mem_map.1 hp
    obtain ⟨par', hb'⟩ := hI.refs p' r (hocc p' r hm)
    obtain ⟨rfl, -⟩ := borrowed_unique hnd (List.mem_append_left _ hb') hb
    exact hm
  -- The wrapper is entered once, with this parameter.
  have hpE : RefSafety.params? E frame.id = none :=
    params?_none_of fun f ps he => by rw [hid]; exact trace_entered_lt h0 hT f ps he
  have hpev : RefSafety.params? evs frame.id = some (R.map (·.1)) := by
    rw [hevs]
    refine params?_of_mem (fun e he f' ps' hee hf' => ?_) (by simp)
    subst hee
    simp only [List.mem_append, List.mem_map, List.mem_singleton] at he
    rcases he with ⟨_, -, h⟩ | h
    · cases h
    · simp only [Event.entered.injEq] at h
      exact h.2
  have hsk : (frame :: { fr with stack := fr.stack.drop params.length } :: rest).map
      Frame.skel = ({ callee with typeArgs := τ }, cinfo.moduleId != info.moduleId) ::
        (fr :: rest).map Frame.skel := by
    simp only [List.map_cons, Frame.skel, hfun, hca]
  have hback := back_of_nodup hnd
  have hsub : ∀ e ∈ E, e ∈ E ++ evs := fun e he => List.mem_append_left _ he
  rw [hsf] at hC
  obtain ⟨ps₀, hp₀, hok₀, hids₀, T₀, hT₀, hrest₀⟩ := hC
  have hmono : ∀ t : StructId, 1 ≤ countActive P ((fr :: rest).map Frame.skel) t.module →
      1 ≤ countActive P (({ callee with typeArgs := τ }, cinfo.moduleId != info.moduleId) ::
        (fr :: rest).map Frame.skel) t.module := by
    intro t ht
    rw [countActive_cons]
    omega
  refine ⟨fun t => T t ∨ ∃ a, x.location.root = .global a t, ?_⟩
  rw [h2f]
  refine ⟨R.map (·.1), by rw [params?_append_none hpE]; exact hpev, ?_, fun y hy => ?_, T,
    fun t ht => Or.inl ht, ?_⟩
  · rw [hsk]
    refine ⟨fun p hp => ?_, fun p hp rp par hb f' i hρ => ?_, fun p hp rp par hb a t hρ => ?_,
      fun p hp q hq hpq rp rq pp pq hbp hbq => ?_, fun t ht => ?_,
      fun t _ hacq => absurd hacq (hnoacq t)⟩
    · obtain ⟨⟨p', r⟩, hm, rfl⟩ := List.mem_map.1 hp
      obtain ⟨par', hb'⟩ := hI.refs p' r (hocc p' r hm)
      rw [borrowedIds_append]
      exact List.mem_append_left _ (mem_borrowedIds.2 ⟨r, par', hb'⟩)
    · obtain ⟨-, rfl⟩ := hRx p rp (hbR p hp rp par hb)
      have hb' := hxok.below
      rw [hρ] at hb'
      rw [hid]
      exact Nat.lt_of_le_of_lt hb' hcid
    · obtain ⟨-, rfl⟩ := hRx p rp (hbR p hp rp par hb)
      exact Or.inr ⟨a, hρ⟩
    · obtain ⟨e1, -⟩ := hRx p rp (hbR p hp rp pp hbp)
      obtain ⟨e2, -⟩ := hRx q rq (hbR q hq rq pq hbq)
      exact absurd (e1.trans e2.symm) hpq
    · rcases ht with ht | ⟨a, hρ⟩
      · exact hmono t (hok₀.active t ht)
      · have := hxact a t hρ
        rw [hsf] at this
        exact hmono t this
  · rw [hid]
    simp only [List.mem_cons] at hy
    rcases hy with rfl | hy
    · exact hcid
    · exact hinv.2.below y (by rw [hsf]; exact List.mem_cons_of_mem _ hy)
  · exact ⟨ps₀, params?_append hp₀, hok₀.mono hsub hback rfl rfl, hids₀, T₀, hT₀,
      ParamChain.mono hsub hback (fun _ _ h => params?_append h) hrest₀ rfl⟩

omit hN in
/-- **A closure call gives the callee's parameters their facts**, as a call
does (`paramChain_call`): the provided arguments' references are compatible,
tracked by the caller, and in global types the callee protects; the callee's
frame counts as an activation of its module (`closure_suspendAt`). -/
theorem paramChain_callClosure {s s' : State} {h : Shadow} {E : List Event}
    (hT : Trace P N ν s₀ s h E) (hinv : InvRefs P s)
    {T : StructId → Prop} (hC : ParamChain P E s.frames T) {fr : Frame} {rest : List Frame}
    (hsf : s.frames = fr :: rest) {g : ShadowFrame} {hs' : List ShadowFrame}
    {hst hhe : List IdTree} {f : Nat} (hhf : h = ⟨g :: hs', hst, hhe, f⟩)
    {info : FunctionInfo} {code : CodeUnit} {sig : SignatureIndex}
    (hinfo : P.function? fr.function = some info) (hcode : info.definition.code = some code)
    (hinstr : code.code[fr.pc]? = some (.callClosure sig))
    (hstep' : stepOf (Step.callClosure P s fr rest info sig) = .done (.running s'))
    {post : ShadowPost} {evs : List Event}
    (hex : Shadow.exec P info.moduleId (.callClosure sig) s fr (.running s') h = some (post, evs))
    (hnd : (borrowedIds (E ++ evs)).Nodup) :
    ∃ T', ParamChain P (E ++ evs) s'.frames T' := by
  subst hhf
  have hI := refTraceInv_trace h0 hT
  -- The concrete closure call and its annotation.
  obtain ⟨tf, mask, cs, cps, crs, ca, vst, frame, m, paramTys, ps0, rs0, a, htoks, hstk, hn, hps0,
    hrs0, -, hfun, hca, -, hcr, -, -, -, hid, h2f⟩ := callClosure_concrete (P := P) hstep'
  have hg : g.fits fr = true := (pre_head hI.pre hsf).1
  have hfs := frame_stack_fits hg
  obtain ⟨ct, gst, hgst⟩ : ∃ ct gst, g.stack = ct :: gst := by
    cases hgs : g.stack with
    | nil => rw [hgs, hstk] at hfs; simp [IdTree.fitsAll] at hfs
    | cons ct gst => exact ⟨ct, gst, rfl⟩
  rw [hstk, hgst] at hfs
  simp only [IdTree.fitsAll, Bool.and_eq_true] at hfs
  obtain ⟨-, hgsf⟩ := hfs
  obtain ⟨i, cts⟩ := ct
  have hgl' := fitsAll_length hgsf
  obtain ⟨-, hevs⟩ := callClosure_shadow (sig := sig) (callee := frame) hstk hgst
    (by rw [hgl']; exact hn) hn h2f hex
  -- The analysis at the call.
  obtain ⟨fr₀, rest₀, hsf₀, ⟨info₀, code₀, hact⟩, -⟩ := hinv.1
  rw [hsf, List.cons.injEq] at hsf₀
  obtain ⟨rfl, rfl⟩ := hsf₀
  have e1 := hact.fn
  rw [hinfo, Option.some.injEq] at e1
  subst e1
  have e2 := hact.body
  rw [hcode, Option.some.injEq] at e2
  subst e2
  obtain ⟨view, A, stys, tys, ps, hctx⟩ :=
    topCtxGen hwf hs (apiSigs_of hwf hapi) hinv hsf hact hinstr
  obtain ⟨Sw, hglobW, -⟩ := hctx.chain _ hsf
  rw [hsf] at hC
  obtain ⟨ps₀, hp₀, hok₀, hids₀, T₀, hT₀, hrest₀⟩ := hC
  have hglob : GlobOk P fr.function ((fr :: rest).map Frame.skel) ps (fun t => T t ∨ Sw t) :=
    ⟨fun t ht => ht.elim (hok₀.active t) (hglobW.active t),
     fun t ht hacq => ht.elim (fun h => hok₀.acq t h hacq) (fun h => hglobW.acq t h hacq),
     fun π hπ a t hρ => Or.inr (hglobW.params π hπ a t hρ)⟩
  have hcid : fr.id < s.nextFrame := hinv.2.below fr (by rw [hsf]; exact List.mem_cons_self ..)
  obtain ⟨Sc, hexcl, hargOk, hsusp, hglobC⟩ := closure_suspendAt hctx hact.locals
    (hI.typed.args fr (by rw [hsf]; exact List.mem_cons_self ..)) hinstr hact.fn hact.body hstk
    (fun r h => by cases h) htoks hps0 hrs0 hcr hn (cid := frame.id) (by rw [hid]; exact hcid)
    hsf hinv.2.active.counted hglob (List.Sublist.refl _)
  -- The callee's parameters are the provided reference arguments.
  obtain ⟨R, hR⟩ : ∃ R, R = slotRefs (gst.take cps.length).reverse
      (vst.take cps.length).reverse := ⟨_, rfl⟩
  have hRmap : R.map (·.2) = argRefs (vst.take cps.length).reverse := by
    rw [hR]; exact slotRefs_refs (by simp [hgl'])
  rw [← hR] at hevs
  have hocc : ∀ p r, (p, r) ∈ R → (p, Value.ref r) ∈ framesOcc (g :: hs') s.frames := by
    intro p r hm
    rw [hR] at hm
    obtain ⟨t, hz, rfl⟩ := mem_slotRefs.1 hm
    have hz' : (t, Value.ref r) ∈ g.stack.zip fr.stack := by
      rw [hgst, hstk, List.zip_cons_cons]
      exact List.mem_cons_of_mem _ (zip_rev_take hgl' hz)
    rw [hsf, framesOcc_cons]
    exact List.mem_append_left _ (refSlot_occ hg ⟨t, rfl, Or.inl hz'⟩)
  -- A parameter's borrowed event is its argument's.
  have hbR : ∀ p ∈ R.map (·.1), ∀ rp par, Event.borrowed p rp par ∈ E ++ evs → (p, rp) ∈ R := by
    intro p hp rp par hb
    obtain ⟨⟨p', r⟩, hm, rfl⟩ := List.mem_map.1 hp
    obtain ⟨par', hb'⟩ := hI.refs p' r (hocc p' r hm)
    obtain ⟨rfl, -⟩ := borrowed_unique hnd (List.mem_append_left _ hb') hb
    exact hm
  have harg : ∀ p rp, (p, rp) ∈ R → rp ∈ argRefs (vst.take cps.length).reverse :=
    fun p rp h => by rw [← hRmap]; exact List.mem_map.2 ⟨(p, rp), h, rfl⟩
  have hpw : R.Pairwise (fun x y => Compat x.2 y.2) := by
    rw [← hRmap] at hexcl
    exact List.pairwise_map.1 hexcl
  -- The callee is entered once, with these parameters.
  have hpE : RefSafety.params? E frame.id = none :=
    params?_none_of fun f ps he => by rw [hid]; exact trace_entered_lt h0 hT f ps he
  have hpev : RefSafety.params? evs frame.id = some (R.map (·.1)) := by
    rw [hevs]
    refine params?_of_mem (fun e he f' ps' hee hf' => ?_) (by simp)
    subst hee
    simp only [List.mem_append, List.mem_map, List.mem_cons, List.not_mem_nil, or_false] at he
    rcases he with ⟨_, -, h⟩ | h | h
    · cases h
    · simp only [Event.entered.injEq] at h
      exact h.2
    · cases h
  have hsk : (frame :: { fr with stack := vst.drop cps.length } :: rest).map Frame.skel =
      (tf, true) :: (fr :: rest).map Frame.skel := by
    simp only [List.map_cons, Frame.skel, hfun, hca]
  have hback := back_of_nodup hnd
  have hsub : ∀ e ∈ E, e ∈ E ++ evs := fun e he => List.mem_append_left _ he
  refine ⟨Sc, ?_⟩
  rw [h2f]
  refine ⟨R.map (·.1), by rw [params?_append_none hpE]; exact hpev, ?_, fun x hx => ?_, T,
    fun t ht => hsusp.sub t (Or.inl ht), ?_⟩
  · rw [hsk]
    refine ⟨fun p hp => ?_, fun p hp rp par hb f' i hρ => ?_, fun p hp rp par hb a t hρ => ?_,
      fun p hp q hq hpq rp rq pp pq hbp hbq => ?_, hglobC.active,
      fun t ht hacq => hglobC.acq t ht (by rw [← hfun]; exact hacq)⟩
    · obtain ⟨⟨p', r⟩, hm, rfl⟩ := List.mem_map.1 hp
      obtain ⟨par', hb'⟩ := hI.refs p' r (hocc p' r hm)
      rw [borrowedIds_append]
      exact List.mem_append_left _ (mem_borrowedIds.2 ⟨r, par', hb'⟩)
    · have hb' := (hargOk rp (harg p rp (hbR p hp rp par hb))).below
      rw [hρ] at hb'
      rw [hid]
      exact Nat.lt_of_le_of_lt hb' hcid
    · exact hglobC.params rp (harg p rp (hbR p hp rp par hb)) a t hρ
    · exact pairwise_forall (fun _ _ h => compat_symm h) hpw (p, rp) (hbR p hp rp pp hbp) (q, rq)
        (hbR q hq rq pq hbq) (fun e => hpq (congrArg Prod.fst e))
  · rw [hid]
    simp only [List.mem_cons] at hx
    rcases hx with rfl | hx
    · exact hcid
    · exact hinv.2.below x (by rw [hsf]; exact List.mem_cons_of_mem _ hx)
  · exact ⟨ps₀, params?_append hp₀, hok₀.mono hsub hback rfl rfl, hids₀, T₀, hT₀,
      ParamChain.mono hsub hback (fun _ _ h => params?_append h) hrest₀ rfl⟩

/-- A traced step that enters a frame is a call or a closure call. -/
theorem paramChain_entered {s s' : State} {h : Shadow} {E : List Event}
    (hT : Trace P N ν s₀ s h E) {T : StructId → Prop} (hC : ParamChain P E s.frames T)
    {o : Outcome} {post : ShadowPost} {evs : List Event} (hS : TracedStep P N ν s h o post evs)
    (ho : o = .running s') {fr : Frame} {rest : List Frame} (hsf : s.frames = fr :: rest)
    {g : ShadowFrame} {hs' : List ShadowFrame} (hhf : h.frames = g :: hs') {f : FrameId}
    {pids : List Nat} (hent : Event.entered f pids ∈ evs) (hnd : (borrowedIds (E ++ evs)).Nodup) :
    ∃ T', ParamChain P (E ++ evs) s'.frames T' := by
  cases hS with
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
    case inr =>
      have := (nativeStep_quiet hn hent).1
      simp [Event.entered?] at this
    obtain ⟨rfl, rfl, rfl, g₁, hs₁, info, hh, -, -, -, hevs, -⟩ := nativeStep_some hstep hshape hn
    rw [hevs] at hent
    have := (nativeEvents_quiet hent).1
    simp [Event.entered?] at this
  | core hstep hc =>
    subst ho
    obtain ⟨fr₁, rest₁, info, code, instr, hsf₁, hinfo, hcode, hinstr, hstep', hex⟩ :=
      core_unfold hstep hc
    rw [hsf, List.cons.injEq] at hsf₁
    obtain ⟨rfl, rfl⟩ := hsf₁
    by_cases hcall : (∃ hd, instr = .call hd) ∨ ∃ i, instr = .callGeneric i
    · obtain ⟨hframes, hst, hhe, f⟩ := h
      simp only at hhf
      subst hhf
      have hI := refTraceInv_trace h0 hT
      -- The handle the call names, its type arguments, and the concrete call.
      obtain ⟨hd, τ, hcallI, hτ, hstep'', hex'⟩ : ∃ hd τ, CallsHandle info.module instr hd ∧
          (∀ x ∈ τ, x.isValue = true) ∧
          stepOf (Step.call P s fr rest info hd τ) = .done (.running s') ∧
          Shadow.exec P info.moduleId (.call hd) s fr (.running s') ⟨g :: hs', hst, hhe, f⟩ =
            some (post, evs) := by
        rcases hcall with ⟨hd, rfl⟩ | ⟨i, rfl⟩
        · exact ⟨hd, [], Or.inl rfl, by simp, hstep', hex⟩
        · obtain ⟨x, hx, h1⟩ := done_bind hstep' rfl (ends_need _ _)
          obtain ⟨tys, htys, h1⟩ := done_bind h1 rfl genericArgs_ends
          exact ⟨x.handle, tys, Or.inr ⟨i, x, rfl, need_ok' hx, rfl⟩,
            typeArgsWith_values (hI.typed.args fr (by rw [hsf]; exact List.mem_cons_self ..))
              (genericArgs_ok htys), h1, hex⟩
      have hinvA := invCore_reachable hwf hs hapi hN h0 s (trace_reachable hT)
      rcases hinvA.2 with hr | ⟨sc, hrun⟩
      · have hinv : InvRefs P s := ⟨hinvA.1, hr⟩
        by_cases hapiC : ∃ fh off, info.module.functionHandles[hd]? = some fh ∧
            ReferenceSafety.borrowFieldMutable fh.attributes = some off
        · obtain ⟨fh, off, hfh, hoff⟩ := hapiC
          -- The state after the call is the wrapper in progress.
          have hpost : ApiCore P s' s := by
            obtain ⟨fr₀, rest₀, hsf₀, ⟨info₀, code₀, hact⟩, -⟩ := hinv.1
            rw [hsf, List.cons.injEq] at hsf₀
            obtain ⟨rfl, rfl⟩ := hsf₀
            obtain rfl : info₀ = info := Option.some.inj (hact.fn.symm.trans hinfo)
            obtain rfl : code₀ = code := Option.some.inj (hact.body.symm.trans hcode)
            have hcert := certified_of_wellFormed_gen schedulerSound hwf hs
            rcases hcallI with rfl | ⟨i, fi, rfl, hfi, rfl⟩
            · exact (apiCall_post hwf hs hapi hcert (N := N) hinv hsf hact hinstr hfh hoff
                hstep').core
            · exact (apiCallG_post hwf hs hapi hcert (N := N) hinv hsf hact hinstr hfi hfh hoff
                hstep').core hwf
          exact paramChain_callApi hwf hs hapi h0 hT hinv hC hsf rfl hinfo hcode hinstr hcallI hfh
            hoff hstep'' hex' hpost hnd
        · refine paramChain_call hwf hs hapi h0 hT hinv hC hsf rfl hinfo hcode hinstr hcallI ?_ hτ
            hstep'' hex' hnd
          intro fh hfh
          cases hb : ReferenceSafety.borrowFieldMutable fh.attributes with
          | none => rfl
          | some off => exact absurd ⟨fh, off, hfh, hb⟩ hapiC
      · exfalso
        rcases apiCore_instr hrun hsf hinfo hcode hinstr with ⟨-, h⟩ |
          ⟨-, ⟨f, h⟩ | ⟨v, h⟩ | ⟨f, h⟩ | ⟨v, h⟩⟩ | ⟨-, h⟩ <;>
          rcases hcall with ⟨hd', rfl⟩ | ⟨i, rfl⟩ <;> cases h
    · by_cases hcc : ∃ sig, instr = .callClosure sig
      · obtain ⟨sig, rfl⟩ := hcc
        obtain ⟨hframes, hst, hhe, f⟩ := h
        simp only at hhf
        subst hhf
        have hinvA := invCore_reachable hwf hs hapi hN h0 s (trace_reachable hT)
        rcases hinvA.2 with hr | ⟨sc, hrun⟩
        · exact paramChain_callClosure hwf hs hapi h0 hT ⟨hinvA.1, hr⟩ hC hsf rfl hinfo hcode
            hinstr hstep' hex hnd
        · exfalso
          rcases apiCore_instr hrun hsf hinfo hcode hinstr with ⟨-, h⟩ |
            ⟨-, ⟨f, h⟩ | ⟨v, h⟩ | ⟨f, h⟩ | ⟨v, h⟩⟩ | ⟨-, h⟩ <;> cases h
      · exact (exec_entered hhf hex (fun hd e => hcall (Or.inl ⟨hd, e⟩))
          (fun i e => hcall (Or.inr ⟨i, e⟩)) (fun sig e => hcc ⟨sig, e⟩) hent rfl).elim

/-- **A traced step keeps the parameter facts.** -/
theorem paramChain_step {s : State} {h : Shadow} {E : List Event} (hT : Trace P N ν s₀ s h E)
    {T : StructId → Prop} (hC : ParamChain P E s.frames T) {s' : State} {h' : Shadow}
    {evs : List Event} (hS : TracedStep P N ν s h (.running s') (.running h') evs) :
    ∃ T', ParamChain P (E ++ evs) s'.frames T' := by
  have hI := refTraceInv_trace h0 hT
  have hnd := traced_nodup h0 hT hS
  have hback := back_of_nodup hnd
  have hsub : ∀ e ∈ E, e ∈ E ++ evs := fun e he => List.mem_append_left _ he
  have hpar : ∀ f ps, RefSafety.params? E f = some ps → RefSafety.params? (E ++ evs) f = some ps :=
    fun _ _ h => params?_append h
  rcases traced_shape hI.pre hI.typed.args hI.typed.valsShaped hS rfl rfl with ⟨hkeys, -, -⟩ |
    ⟨fr, rest, g, hs', callee, gc, k, pids, hsf, hhf, -, -, -, hent, -⟩ |
    ⟨fr, caller, rest, g, gcaller, hs', caller', gc', rets, hsf, -, hs'f, -, hkey, -, -⟩
  · exact ⟨T, ParamChain.mono hsub hback hpar hC hkeys⟩
  · exact paramChain_entered hwf hs hapi hN h0 hT hC hS rfl hsf hhf hent hnd
  · -- A return to the caller.
    rw [hsf] at hC
    obtain ⟨-, -, -, -, T', -, hrest⟩ := hC
    refine ⟨T', ?_⟩
    rw [hs'f]
    exact ParamChain.mono hsub hback hpar hrest (by simp only [List.map_cons, hkey])

/-- **Along every annotated execution, each frame's reference parameters keep
their facts.** -/
theorem paramChain_trace' {s : State} {h : Shadow} {E : List Event}
    (hT : Trace P N ν s₀ s h E) : ∃ T, ParamChain P E s.frames T := by
  induction hT with
  | start => exact paramChain_initial h0
  | step hT hS ih =>
    obtain ⟨T, hC⟩ := ih
    exact paramChain_step hwf hs hapi hN h0 hT hC hS

end Steps

/-- **Along every annotated execution of an M3 program, each frame's reference
parameters keep their facts**, a corollary. -/
theorem paramChain_trace {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}
    (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.SupportedNonGeneric) (hm3 : P.M3)
    (hN : NativesRespect P N) (h0 : AdmissibleInitial P s₀) {s : State} {h : Shadow}
    {E : List Event} (hT : Trace P N ν s₀ s h E) : ∃ T, ParamChain P E s.frames T :=
  paramChain_trace' hwf (supported_of_nonGeneric hwf hs) (apiVersions_M3 hm3) hN h0 hT

end MoveBytecode.Proofs
