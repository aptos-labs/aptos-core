-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.ApiRun

/-!
# A struct-API mutable borrow: the call and the wrapper's first two steps

UNTRUSTED (checked by the kernel). A call of a function handle with a
`BorrowFieldMutable(off)` attribute, from a state keeping the reference
invariant, starts a wrapper in progress (`apiRun_call`): the callee is the
struct-API wrapper (`apiCall_wrapper`), entered with the caller's top
reference as its only argument. The wrapper's `MoveLoc(0)` and its borrow
keep the wrapper in progress (`apiRun_step`): they change only the wrapper's
frame and record only the removal of its local 0, and the borrow derives the
field reference from the argument, valid as the caller's tracked reference
was at the call.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph MoveBytecode.Step

/-! ## The frames under the wrapper -/

section Frames

variable {P : Program} {s sc : State} {w fr fr' : Frame} {rest : List Frame}

/-- A frame other than the wrapper is the caller, replaced, or one of `sc`. -/
theorem api_frame? (hsc : sc.frames = fr :: rest) (hs : s.frames = w :: fr' :: rest)
    (hid : fr'.id = fr.id) {f : FrameId} (hf : f ≠ w.id) :
    (s.frame? f = some fr' ∧ sc.frame? f = some fr) ∨ s.frame? f = sc.frame? f := by
  have h1 : (w.id == f) = false := by simpa using fun e => hf e.symm
  by_cases h2 : fr.id = f
  · left
    have h3 : (fr'.id == f) = true := by simp [hid, h2]
    have h4 : (fr.id == f) = true := by simp [h2]
    refine ⟨?_, ?_⟩
    · simp only [State.frame?, hs, List.find?_cons, h1, h3]
    · simp only [State.frame?, hsc, List.find?_cons, h4]
  · right
    have h3 : (fr'.id == f) = false := by simp [hid, h2]
    have h4 : (fr.id == f) = false := by simp [h2]
    simp only [State.frame?, hs, hsc, List.find?_cons, h1, h3, h4]

/-- A root in no local of the wrapper has its type and value of `sc`. -/
theorem api_root (hsc : sc.frames = fr :: rest) (hs : s.frames = w :: fr' :: rest)
    (hid : fr'.id = fr.id) (hfun : fr'.function = fr.function) (hloc : fr'.locals = fr.locals)
    (hst : s.storage = sc.storage) (hhe : s.held = sc.held) {ρ : Root}
    (hne : ∀ f i, ρ = .local_ f i → f ≠ w.id) :
    RootType P s ρ = RootType P sc ρ ∧ s.root? ρ = sc.root? ρ := by
  cases ρ with
  | local_ f i =>
    rcases api_frame? hsc hs hid (hne f i rfl) with ⟨e1, e2⟩ | e
    · simp [RootType, State.root?, e1, e2, hfun, hloc]
    · simp [RootType, State.root?, e]
  | held k => simp [RootType, State.root?, hhe]
  | global a t => simp [RootType, State.root?, hst]

/-- **A reference of the caller in `sc`** is one of the caller in `s`, across
the removal of the wrapper's local. -/
theorem api_refOk (hsc : sc.frames = fr :: rest) (hs : s.frames = w :: fr' :: rest)
    (hid : fr'.id = fr.id) (hfun : fr'.function = fr.function) (hloc : fr'.locals = fr.locals)
    (hst : s.storage = sc.storage) (hhe : s.held = sc.held) {es : List Invalidation}
    (hes : s.invalidations = es ++ sc.invalidations)
    (hesw : ∀ e ∈ es, e = .removed (.local_ w.id 0)) (hlt : fr.id < w.id) {frx : Frame}
    {r : Reference} (h : RefOk P sc frx r) (hfrx : frx.id = fr.id ∨ frx.id < fr.id) :
    RefOk P s frx r := by
  have hne : ∀ f i, r.location.root = .local_ f i → f ≠ w.id := by
    intro f i hρ e
    have hb := h.below
    rw [hρ] at hb
    have hb' : f ≤ frx.id := hb
    rcases hfrx with hfrx | hfrx
    · rw [hfrx] at hb'
      exact Nat.lt_irrefl _ (Nat.lt_of_le_of_lt (e ▸ hb') hlt)
    · exact Nat.lt_irrefl _ (Nat.lt_of_le_of_lt (e ▸ hb') (Nat.lt_trans hfrx hlt))
  obtain ⟨h1, h2⟩ := api_root (P := P) (ρ := r.location.root) hsc hs hid hfun hloc hst hhe hne
  refine h.transfer' rfl h1 hes (fun e he => ?_) (fun hr => ?_)
  · rw [hesw e he]
    simpa [Invalidation.covers] using hne w.id 0
  · rw [read?_eq_of_root h2]; exact hr

end Frames

/-! ## The call -/

section Call

variable {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
  (hs : P.Supported)
  (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
    ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)

/-- The state after a struct-API borrow call is a wrapper in progress. -/
theorem apiRun_of_call {s s' : State} (hinv : InvRefs P s) {fr : Frame} {rest : List Frame}
    (hsf : s.frames = fr :: rest) {info : FunctionInfo} {code : CodeUnit}
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code) {h : Nat}
    (hi : code.code[fr.pc]? = some (.call h)) {fh : FunctionHandle}
    (hfh : info.module.functionHandles[h]? = some fh) {off : Nat}
    (hoff : ReferenceSafety.borrowFieldMutable fh.attributes = some off) {x : Reference}
    {st : List Value} (hst : fr.stack = .ref x :: st) {callee : FunctionRef}
    {cinfo : FunctionInfo} (hres : P.resolve fr.function.module h = some callee)
    (hcf : P.function? callee = some cinfo) (hwc : WrapperCode P callee cinfo off)
    {w fr1 : Frame} (hs'f : s'.frames = w :: fr1 :: rest) (hfr1 : fr1 = { fr with stack := st })
    (hwfun : w.function = callee) (hwid : w.id = s.nextFrame)
    (hwca : w.countedActive = (cinfo.moduleId != info.moduleId))
    (hnext : s'.nextFrame = s.nextFrame + 1) (hstor : s'.storage = s.storage)
    (hheld : s'.held = s.held)
    (hactv : s'.active = (if w.countedActive then s.incActive cinfo.moduleId else s).active)
    (hlock : s'.lockDepth = (if w.lockEntered then s.lockDepth + 1 else s.lockDepth))
    (hinvs : s'.invalidations = s.invalidations) {n : Nat}
    (hloc : w.locals = ([Value.ref x].map some ++ List.replicate n none).toArray)
    (hpc : w.pc = 0) (hstk : w.stack = []) : ApiRun P s' s := by
  subst hfr1
  refine ⟨hinv, fr, rest, info, code, h, fh, off, x, st, w, callee, cinfo, hsf, hact, hi, hfh, hoff,
    hst, hres, hcf, hwc, hs'f, hwfun, hwid, hwca, hnext, hstor, hheld, hactv, hlock,
    ⟨[], by rw [hinvs]; rfl, fun e he => by cases he⟩, fun j v hj => ?_,
    Or.inl ⟨hpc, ?_, hstk⟩⟩
  · rw [hloc] at hj
    cases j with
    | zero => rfl
    | succ j =>
      simp only [List.map_cons, List.map_nil, List.singleton_append, List.getElem?_toArray,
        List.getElem?_cons_succ, List.getElem?_replicate] at hj
      split at hj <;> cases hj
  · rw [hloc]; simp

include hwf hs hapi in
/-- **A struct-API mutable borrow call** from a state keeping the reference
invariant is safe, and starts a wrapper in progress. -/
theorem apiRun_call (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hN : NativesRespect P N) {s : State} (hinv : InvRefs P s) {fr : Frame} {rest : List Frame}
    (hsf : s.frames = fr :: rest) {info : FunctionInfo} {code : CodeUnit}
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {h : FunctionHandleIndex}
    (hi : code.code[fr.pc]? = some (.call h)) {fh : FunctionHandle}
    (hfh : info.module.functionHandles[h]? = some fh) {off : Nat}
    (hoff : ReferenceSafety.borrowFieldMutable fh.attributes = some off) :
    ExecSafe P N (InvAll P) (Step.exec P s fr rest info code (.call h)) := by
  have hnm : NoModeledCall P fr.function.module info.module (.call h) := by
    intro c hres'
    obtain ⟨callee, cinfo, hres, -, hcf, htp, -, -⟩ :=
      hc.calls fr.function info code hact.fn hact.body fr.pc h hact.reachable hi
    rw [hres, Option.some.injEq] at hres'
    subst hres'
    obtain ⟨code', _, _, _, _, _, hcode', -⟩ :=
      (apiCall_wrapper' hwf hapi hact.fn hfh hoff hres hcf htp).code
    exact nativeShape?_of_code hcf hcode'
  have hcore : ExecSafe P N (Inv P (modelTypes P) (modelAvail P))
      (Step.exec P s fr rest info code (.call h)) :=
    exec_safe hc hN hact hchain hi rfl (fun h => by rcases h with h | h <;> cases h) hnm
  suffices hruns : ExecRuns P N (fun s => RefInvariant P s ∨ ∃ sc, ApiRun P s sc)
      (Step.exec P s fr rest info code (.call h)) from ExecSafe.and hcore hruns
  obtain ⟨view, A, stys, tys, ps, hctx⟩ := topCtxGen hwf hs (apiSigs_of hwf hapi) hinv hsf hact hi
  obtain ⟨callee, cinfo, hres, hvis, hcf, htp, hpar, hret⟩ :=
    hc.calls fr.function info code hact.fn hact.body fr.pc h hact.reachable hi
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  have hbt := hctx.before hbefore
  subst hbt
  have hta := hafter
  dsimp only [typeAfter] at hta
  simp only [bind, Option.bind_eq_some_iff] at hta
  obtain ⟨cfh, hcfh, hta⟩ := hta
  split at hta
  · simp at hta
  rename_i hcne
  have hcfh0 : cfh.typeParameters = [] := by simpa using hcne
  simp only [Option.bind_eq_some_iff] at hta
  obtain ⟨params, hparams, returns, hreturns, hta⟩ := hta
  split at hta
  case isFalse => simp at hta
  rename_i hpre
  have hstack := hctx.conf
  have hlen := conformsAll_length hstack
  have htl := assignableAll_length hpre
  have hge : params.length ≤ fr.stack.length := by
    rw [hlen]; simp only [List.length_take, List.length_reverse] at htl; omega
  have hargs : conformsAll P (fr.stack.take params.length).reverse params = true := by
    have := conformsAll_reverse
      (conformsAll_assignable (conformsAll_take params.length hstack) hpre)
    rwa [List.reverse_reverse] at this
  have hpar' : P.paramTypes callee = some params := hpar.trans hparams
  have hret' : P.returnTypes callee = some returns := hret.trans hreturns
  -- The callee is the wrapper: one `&mut` parameter, so one reference argument.
  have hwc := apiCall_wrapper' hwf hapi hact.fn hfh hoff hres hcf htp
  obtain ⟨ccode, b, sd, sdef, sh, inner, hccode, hcl, hpsig, hinner, -, -⟩ := hwc.code
  obtain ⟨sig, hsig, hps⟩ := paramTypes_sig hcf hpar'
  rw [hpsig, Option.some.injEq] at hsig
  subst hsig
  obtain ⟨u, us, hu, hus, rfl⟩ := ofSignatureWith_cons.1 hps
  simp only [RuntimeType.ofSignatureWith, List.mapM_nil, Option.pure_def, Option.some.injEq] at hus
  subst hus
  obtain ⟨ut, rfl⟩ := ofSlotToken_ref hu (b := true) rfl
  obtain ⟨x, st, hst⟩ : ∃ x st, fr.stack = .ref x :: st := by
    cases hfs : fr.stack with
    | nil => rw [hfs] at hge; simp at hge
    | cons a l =>
      rw [hfs] at hargs
      simp only [List.length_cons, List.length_nil, Nat.zero_add, List.take_succ_cons,
        List.take_zero, List.reverse_cons, List.reverse_nil, List.nil_append,
        conformsAll_cons] at hargs
      obtain ⟨r, rfl, -, -⟩ := conforms_ref hargs.1 (by simp [rigid])
      exact ⟨r, l, rfl⟩
  have hge1 : ¬ fr.stack.length < [RuntimeType.ref true ut].length := by
    rw [hst]; simp
  have hceq := withTypeArgs_self (resolve_typeArgs hres)
  rw [hfh, Option.some.injEq] at hcfh
  subst hcfh
  simp only [Step.exec, Step.call, Step.need, hres, hvis, hcf, hfh, hcfh0, htp, hceq, hpar', hret',
    hparams, hreturns, RuntimeType.argumentsWellFormed, List.length_nil,
    Step.popN, hge1, hargs, bne_self_eq_false, Bool.or_false, Bool.not_true,
    Bool.false_eq_true, ↓reduceIte, bind, Except.bind, pure, Except.pure,
    hccode, Option.isNone_some, Bool.false_and]
  obtain ⟨ltys, hltys⟩ := hc.locals callee cinfo ccode hcf hccode
    (typeArgsOk_nil hcf (resolve_typeArgs hres) htp)
  simp only [hltys]
  repeat' split
  all_goals first
    | trivial
    | (refine Or.inr ⟨s, apiRun_of_call hinv hsf hact hi hfh hoff hst hres hcf hwc rfl ?_ rfl ?_ rfl
        ?_ ?_ ?_ ?_ ?_ ?_ (n := ltys.length - 1) ?_ rfl rfl⟩
       all_goals first
         | rfl
         | (simp [hst]; done)
         | simp_all [State.incActive, State.activeCount])

end Call

/-! ## The wrapper's steps -/

theorem RefOk.reframe {P : Program} {s : State} {frx frx' : Frame} {r : Reference}
    (h : RefOk P s frx r) (hid : frx'.id = frx.id) : RefOk P s frx' r :=
  ⟨h.root, by rw [hid]; exact h.below, h.value, h.since, h.valid, h.writable⟩

/-- The wrapper's instruction at a position. -/
theorem wrapper_at {ccode : CodeUnit} {b : Bytecode}
    (hcl : ccode.code.toList = [.moveLoc 0, b, .ret]) {pc : Nat} {instr : Bytecode}
    (hi : ccode.code[pc]? = some instr) :
    (pc = 0 ∧ instr = .moveLoc 0) ∨ (pc = 1 ∧ instr = b) ∨ (pc = 2 ∧ instr = .ret) := by
  rw [← Array.getElem?_toList, hcl] at hi
  match pc, hi with
  | 0, hi => exact Or.inl ⟨rfl, by simpa using hi.symm⟩
  | 1, hi => exact Or.inr (Or.inl ⟨rfl, by simpa using hi.symm⟩)
  | 2, hi => exact Or.inr (Or.inr ⟨rfl, by simpa using hi.symm⟩)
  | n + 3, hi => simp at hi

section Step

variable {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
  (hs : P.Supported)
  (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
    ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)

include hwf hs hapi in
/-- **The wrapper's `MoveLoc(0)` and borrow** are safe and keep the wrapper in
progress: the borrow derives the field reference from the argument, which the
caller tracked at the call. -/
theorem apiRun_step (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hN : NativesRespect P N) {s sc : State} (hrun : ApiRun P s sc) {w : Frame}
    {rest' : List Frame} (hsf : s.frames = w :: rest') {winfo : FunctionInfo}
    {wcode : CodeUnit} (hact : ActiveAt P (modelTypes P) (modelAvail P) w winfo wcode)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) w.function rest') {instr : Bytecode}
    (hi : wcode.code[w.pc]? = some instr) (hne : instr ≠ .ret) :
    ExecSafe P N (InvAll P) (Step.exec P s w rest' winfo wcode instr) := by
  obtain ⟨fr, rest, info, code, h, fh, off, x, st, w0, callee, cinfo, hsc, hact0, hi0, hfh, hoff,
    hst, hres, hcf, hwc, hsf0, hwfun, hwid, hwca, hnext, hstor, hheld, hactv, hlock,
    ⟨es, hes, hesw⟩, hlocs, hpcs⟩ := hrun.call
  rw [hsf, List.cons.injEq] at hsf0
  obtain ⟨hw, hrest⟩ := hsf0
  subst hw
  subst hrest
  have e1 := hact.fn
  rw [hwfun, hcf, Option.some.injEq] at e1
  subst e1
  obtain ⟨ccode, b, sd, sdef, sh, inner, hccode, hcl, hpsig, hinner, hsd, hb⟩ := hwc.code
  have e2 := hact.body
  rw [hccode, Option.some.injEq] at e2
  subst e2
  have hlt : fr.id < w.id := by
    rw [hwid]; exact hrun.inv.2.below fr (by rw [hsc]; exact List.mem_cons_self)
  rcases wrapper_at hcl hi with ⟨hpc, rfl⟩ | ⟨hpc, rfl⟩ | ⟨-, rfl⟩
  · -- `MoveLoc(0)`: the argument moves to the operand stack.
    rcases hpcs with ⟨-, hl0, hstk⟩ | ⟨hp1, -, -⟩ | ⟨hp2, -, -⟩
    rotate_left
    · omega
    · omega
    refine ExecSafe.and (exec_safe hc hN hact hchain hi rfl
      (fun h => by rcases h with h | h <;> cases h) trivial) ?_
    simp only [Step.exec, Step.need, hl0, bind, Except.bind, Step.next, pure, Except.pure]
    refine Or.inr ⟨sc, hrun.inv, fr, rest, info, code, h, fh, off, x, st, _, callee, cinfo,
      hsc, hact0, hi0, hfh, hoff, hst, hres, hcf, hwc, rfl, hwfun, hwid, hwca, hnext, hstor,
      hheld, hactv, hlock, ⟨.removed (.local_ w.id 0) :: es, by simp [State.invalidate, hes],
        fun e he => ?_⟩,
      fun j v hj => ?_, Or.inr (Or.inl ⟨by simp [Step.advance, Step.push, hpc], ?_, by
        simp [Step.advance, Step.push, hstk]⟩)⟩
    · rcases List.mem_cons.1 he with rfl | he
      · rfl
      · exact hesw e he
    · cases j with
      | zero => rfl
      | succ j =>
        simp only [Step.advance, Step.push] at hj
        rw [Array.getElem?_setIfInBounds_ne (by omega)] at hj
        exact hlocs _ _ hj
    · have hsz : 0 < w.locals.size := by
        have := (Array.getElem?_eq_some_iff.1 hl0).1
        exact this
      simp [Step.advance, Step.push, Array.getElem?_setIfInBounds_self_of_lt hsz]
  · -- The borrow: a field reference derived from the argument.
    rcases hpcs with ⟨hp0, -, -⟩ | ⟨-, hl0, hstk⟩ | ⟨hp2, -, -⟩
    · omega
    rotate_left
    · omega
    -- The argument is a reference the caller tracked at the call.
    obtain ⟨view0, A0, stys0, tys0, ps0, hctx0⟩ :=
      topCtxGen hwf hs (apiSigs_of hwf hapi) hrun.inv hsc hact0 hi0
    obtain ⟨A0', hx0⟩ := hctx0.exec
    obtain ⟨id0, ra0, g1, g2, g3, hsa0, -, -, -, -, -⟩ := exec_callApi hx0 hfh hoff
    obtain ⟨L0, hd0, hrefs0⟩ := hctx0.frameOk
    have hlen0 : ra0.length = st.length := by
      have := hctx0.height; rw [hsa0, hst] at this; simpa using this
    obtain ⟨r', hr', hL⟩ := hd0.top hsa0 hst hlen0
    simp only [Value.ref.injEq] at hr'
    subst hr'
    have hid0 : id0 ∈ heldIds A0 := mem_heldIds.2 (Or.inr (by rw [hsa0]; exact List.mem_cons_self))
    have hoks : RefOk P s { fr with stack := st } x :=
      (api_refOk hsc hsf rfl rfl rfl hstor hheld hes hesw hlt (hrefs0 id0 x hid0 hL)
        (Or.inl rfl)).reframe rfl
    have hroots : RootsOk P s := by
      intro g hg
      rw [hsf] at hg
      rcases List.mem_cons.1 hg with rfl | hg
      · exact hact.locals
      rcases List.mem_cons.1 hg with rfl | hg
      · exact hact0.locals
      · exact hctx0.roots g (by rw [hsc]; exact List.mem_cons_of_mem _ hg)
    obtain ⟨-, htypes⟩ := hc.types w.function cinfo ccode hact.fn hact.body hact.typeArgs
    obtain ⟨before, after, hbefore, hafter, -⟩ := htypes w.pc _ hact.reachable hi
    obtain ⟨tys, htc, hconf⟩ := hact.types
    rw [hbefore, Option.some.injEq] at htc
    subst htc
    rw [hstk] at hconf
    let w₁ : Frame := { w with stack := [] }
    let s₁ : State := { s with frames := w₁ :: { fr with stack := st } :: rest }
    have hroot1 : ∀ ρ, s₁.root? ρ = s.root? ρ := root?_replaceTop hsf rfl rfl rfl rfl rfl
    obtain ⟨w0', hw0⟩ := hoks.deref_some
    have hw0c := hoks.deref_conforms hroots hw0
    rcases hb with ⟨fhi, fhd, rfl, hfhd, -, hfoff⟩ | ⟨vh, vfhd, rfl, hvfhd, -, hvoff, -⟩
    · -- `MutBorrowField`.
      obtain ⟨fh', sid, ftys, t, m', brest, hfh', hdef, hftys, ht, hbef, hm', hafter'⟩ :=
        typeAfter_fieldBorrow (mut_ := true) hafter
      rw [hfhd, Option.some.injEq] at hfh'
      subst hfh'
      subst hbef
      subst hafter'
      obtain ⟨sinfo, fields, hsinfo, hfi⟩ := fieldTypes_some hftys
      obtain ⟨v, st', hst', hv', hstc⟩ := conformsAll_cons_right hconf
      simp only [List.cons.injEq] at hst'
      obtain ⟨rfl, rfl⟩ := hst'
      obtain ⟨rp, hrp, hrm, hrt⟩ := conforms_ref hv' (by simp [rigid])
      simp only [Value.ref.injEq] at hrp
      rw [← hrp] at hrm hrt
      have hxm : x.mutable = true := by rw [hrm]; exact hm' rfl
      rw [hrt] at hw0c
      obtain ⟨fs, hsl, hfs⟩ := conforms_struct hw0c hftys
      have hpop : Step.popRefTo w (.struct sid) true = .ok (x, w₁) := by
        simp only [Step.popRefTo, Step.popRef, Step.pop, hstk, bind, Except.bind, pure,
          Except.pure, hrt, bne_self_eq_false]
        simp [hxm]; rfl
      have hder : Step.deref s₁ x = .ok w0' := by
        have : s₁.deref? x.location x.since = some w0' := by
          rw [deref?_congr (hroot1 _) rfl]; exact hw0
        simp [Step.deref, Step.need, this, pure, Except.pure]
      let rn : Reference := ⟨true, t, x.location.child (.field fhd.field), s.invalidations.length⟩
      have hstep : Step.exec P s w ({ fr with stack := st } :: rest) cinfo ccode
          (.mutBorrowField fhi) = .ok (.done (.running { s₁ with
            frames := Step.advance (Step.pushAll w₁ [.ref rn]) :: { fr with stack := st } :: rest })) := by
        simp only [Step.exec, Step.borrowField, Step.need, hfhd, localStruct_ok hdef hsinfo, hfi,
          hftys, ht, bind, Except.bind, pure, Except.pure, hpop, hder, Step.finish, Step.borrow, rn,
          s₁, w₁]
      rw [hstep]
      have htv : t.isValue = true :=
        fieldTypes_values (fieldTypes_args hftys) hftys t (List.mem_of_getElem? ht)
      refine ⟨fallThrough_safe (N := N) (after := .ref true t :: brest) hc hact hchain hi rfl
        (fun _ => rfl) hbefore hafter rfl rfl rfl ?_, Or.inr ⟨sc, hrun.inv, fr, rest, info, code, h,
          fh, off, x, st, _, callee, cinfo, hsc, hact0, hi0, hfh, hoff, hst, hres, hcf, hwc,
          rfl, hwfun, hwid, hwca, hnext, hstor, hheld, hactv, hlock, ⟨es, hes, hesw⟩, hlocs,
          Or.inr (Or.inr ⟨by simp [Step.advance, Step.pushAll, w₁, hpc], hl0, rn, .field fhd.field,
            rfl, rfl, rfl,
            by simp [absPath, hfoff], ?_⟩)⟩⟩
      · simp only [Step.advance, Step.pushAll, w₁, List.reverse_cons, List.reverse_nil,
          List.nil_append, List.cons_append, conformsAll_cons]
        exact ⟨conforms_iff.2 ⟨assignable_refl _, htv⟩, hstc⟩
      · have hok3 := hoks.sameFrames hsf (s' := { s₁ with
          frames := Step.advance (Step.pushAll w₁ [.ref rn]) :: { fr with stack := st } :: rest })
          rfl rfl rfl rfl rfl (es := []) (storageKept_of_eq rfl) rfl (fun e he => by cases he) rfl
        refine hok3.child (st := .field fhd.field) ?_ htv ?_ true (fun _ => hxm) rfl
        · rw [hrt]; simp [stepType, hftys, ht]
        · intro w' hw'
          let rest' : List Frame := { fr with stack := st } :: rest
          have heq : ({ s₁ with frames := (Step.advance (Step.pushAll w₁ [.ref rn])) :: rest' } :
              State).deref? x.location x.since = s.deref? x.location x.since :=
            deref?_congr (root?_replaceTop
              (s₂ := { s₁ with frames := (Step.advance (Step.pushAll w₁ [.ref rn])) :: rest' })
              hsf rfl rfl rfl rfl rfl _) rfl _
          rw [heq, hw0, Option.some.injEq] at hw'
          subst hw'
          have hl := conformsAll_length hfs
          have hlt' : fhd.field < fs.length := by
            rw [hl]; exact (List.getElem?_eq_some_iff.1 ht).1
          simp [hsl.child_field, List.getElem?_eq_getElem hlt']
    · -- `MutBorrowVariantField`.
      obtain ⟨vfh', sid, t, m', brest, hvh, hdef, hvt, hbef, hm', hafter'⟩ :=
        typeAfter_variantBorrow (mut_ := true) hafter
      rw [hvfhd, Option.some.injEq] at hvh
      subst hvh
      subst hbef
      subst hafter'
      have hspec := variantFieldType?_spec hvt
      have hne' : vfhd.variants ≠ [] := by
        intro hn
        simp [Program.variantFieldType?, hn] at hvt
      obtain ⟨v0, hv0⟩ := List.exists_mem_of_ne_nil _ hne'
      obtain ⟨fs0, hfs0, -⟩ := hspec v0 hv0
      obtain ⟨sinfo, variants, -, hsinfo, hfi, -⟩ := variantFieldTypes_some hfs0
      obtain ⟨v, st', hst', hv', hstc⟩ := conformsAll_cons_right hconf
      simp only [List.cons.injEq] at hst'
      obtain ⟨rfl, rfl⟩ := hst'
      obtain ⟨rp, hrp, hrm, hrt⟩ := conforms_ref hv' (by simp [rigid])
      simp only [Value.ref.injEq] at hrp
      rw [← hrp] at hrm hrt
      have hxm : x.mutable = true := by rw [hrm]; exact hm' rfl
      rw [hrt] at hw0c
      obtain ⟨tag, fields, tys', rfl, htys', hfs⟩ := conforms_variant hw0c hfs0
      have hmap : vfhd.variants.mapM (fun v => do
          let _ ← Step.need variants[v]? .badIndex
          let fields ← Step.need (P.variantFieldTypes sid v) (.unsupported "field types")
          Step.need fields[vfhd.field]? .badIndex) = .ok (vfhd.variants.map fun _ => t) := by
        refine mapM_exec_const fun v hv => ?_
        obtain ⟨fs, hfs', hft⟩ := hspec v hv
        obtain ⟨sinfo', variants', vd, hsinfo', hfi', hvd⟩ := variantFieldTypes_some hfs'
        rw [hsinfo, Option.some.injEq] at hsinfo'
        subst hsinfo'
        rw [hfi, StructFieldInformation.declaredVariants.injEq] at hfi'
        subst hfi'
        simp [Step.need, hvd, hfs', hft, bind, Except.bind, pure, Except.pure]
      have hpop : Step.popRefTo w (.struct sid) true = .ok (x, w₁) := by
        simp only [Step.popRefTo, Step.popRef, Step.pop, hstk, bind, Except.bind, pure,
          Except.pure, hrt, bne_self_eq_false]
        simp [hxm]; rfl
      have hder : Step.deref ({ s with frames := w₁ :: { fr with stack := st } :: rest } : State) x
          = .ok (.variant sid tag fields) := by
        have : ({ s with frames := w₁ :: { fr with stack := st } :: rest } : State).deref?
            x.location x.since = some (.variant sid tag fields) :=
          (deref?_congr (hroot1 _) rfl _).trans hw0
        simp [Step.deref, Step.need, this, pure, Except.pure]
      cases hfind : (vfhd.variants.zip (vfhd.variants.map fun _ => t)).find? (·.1 == tag) with
      | none =>
        have hstep : Step.exec P s w ({ fr with stack := st } :: rest) cinfo ccode
            (.mutBorrowVariantField vh) = .error (.failed .variantMismatch) := by
          simp only [Step.exec, Step.borrowVariantField, hvfhd, need_some, ok_bind',
            localStruct_ok hdef hsinfo, hfi, hmap, hpop, hder, hfind, Step.fail]
        rw [hstep]
        trivial
      | some p =>
        obtain ⟨rfl, htag⟩ := find?_zip_map hfind
        obtain ⟨fst, hfst, hftag⟩ := hspec tag htag
        rw [hfst, Option.some.injEq] at htys'
        subst htys'
        let rn : Reference := ⟨true, t, x.location.child (.variantField tag vfhd.field),
          s.invalidations.length⟩
        have hstep : Step.exec P s w ({ fr with stack := st } :: rest) cinfo ccode
            (.mutBorrowVariantField vh) = .ok (.done (.running { s₁ with
              frames := (Step.advance (Step.pushAll w₁ [.ref rn])) :: { fr with stack := st } ::
                rest })) := by
          simp only [Step.exec, Step.borrowVariantField, hvfhd, need_some, ok_bind',
            localStruct_ok hdef hsinfo, hfi, hmap, hpop, hder, hfind, Step.finish]
          rfl
        rw [hstep]
        have htv : t.isValue = true :=
          variantFieldTypes_values (variantFieldTypes_args hfst) hfst t
            (List.mem_of_getElem? hftag)
        refine ⟨fallThrough_safe (N := N) (after := .ref true t :: brest) hc hact hchain hi rfl
          (fun _ => rfl) hbefore hafter rfl rfl rfl ?_, Or.inr ⟨sc, hrun.inv, fr, rest, info, code,
            h, fh, off, x, st, _, callee, cinfo, hsc, hact0, hi0, hfh, hoff, hst, hres, hcf, hwc,
            rfl, hwfun, hwid, hwca, hnext, hstor, hheld, hactv, hlock, ⟨es, hes, hesw⟩, hlocs,
            Or.inr (Or.inr ⟨by simp [Step.advance, Step.pushAll, w₁, hpc], hl0, rn,
              .variantField tag vfhd.field, rfl, rfl, rfl, by simp [absPath, hvoff], ?_⟩)⟩⟩
        · simp only [Step.advance, Step.pushAll, w₁, List.reverse_cons, List.reverse_nil,
            List.nil_append, List.cons_append, conformsAll_cons]
          exact ⟨conforms_iff.2 ⟨assignable_refl _, htv⟩, hstc⟩
        · let rest' : List Frame := { fr with stack := st } :: rest
          have hok3 := hoks.sameFrames hsf
            (s' := { s₁ with frames := (Step.advance (Step.pushAll w₁ [.ref rn])) :: rest' })
            rfl rfl rfl rfl rfl (es := []) (storageKept_of_eq rfl) rfl (fun e he => by cases he) rfl
          refine hok3.child (st := .variantField tag vfhd.field) ?_ htv ?_ true (fun _ => hxm) rfl
          · rw [hrt]; simp [stepType, hfst, hftag]
          · intro w' hw'
            have heq : ({ s₁ with frames := (Step.advance (Step.pushAll w₁ [.ref rn])) :: rest' } :
                State).deref? x.location x.since = s.deref? x.location x.since :=
              deref?_congr (root?_replaceTop
                (s₂ := { s₁ with frames := (Step.advance (Step.pushAll w₁ [.ref rn])) :: rest' })
                hsf rfl rfl rfl rfl rfl _) rfl _
            rw [heq, hw0, Option.some.injEq] at hw'
            subst hw'
            have hl := conformsAll_length hfs
            have hlt' : vfhd.field < fields.length := by
              rw [hl]; exact (List.getElem?_eq_some_iff.1 hftag).1
            simp [Value.child?, List.getElem?_eq_getElem hlt']
  · exact absurd rfl hne

end Step

end MoveBytecode.Proofs
