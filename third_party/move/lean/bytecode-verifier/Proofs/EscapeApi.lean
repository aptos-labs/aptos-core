-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.SoundnessAll
import Proofs.ReturnsM3
import Proofs.ApiCore

/-!
# Struct-API borrows in progress, for D4

UNTRUSTED (checked by the kernel). What D4's proofs need of the states where
a struct-API wrapper runs (`ApiCore`, generic or not): the wrapper runs only
its own code, `MoveLoc(0)`, a mutable field or variant field borrow, and
`Ret` (`apiCore_instr`); it holds its argument or the field borrow of it, and
the frames below are the caller's at the call (`frames_unnested_all`); its
`Ret` hands back the field borrow, rooted below it, and it acquires nothing
(`ret_roots_all`). A struct-API call from a state with the reference
invariant makes such a state (`apiCall_post`, and `apiCallG_post` for a
generic one).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph MoveBytecode.Step

/-- An M3 program has no struct-API mutable borrow: the version premise holds
of it. -/
theorem apiVersions_M3 {P : Program} (hm3 : P.M3) :
    ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
      ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version := by
  intro m hm fh hfh off h
  rw [hm3 m hm fh hfh] at h
  cases h

/-- The extraction of a running outcome from `ExecRuns`. -/
theorem ExecRuns.running {P : Program} {N : NativeRel} {R : State → Prop}
    {x : Step.Exec StepResult} (h : ExecRuns P N R x) {s' : State}
    (hx : stepOf x = .done (.running s')) : R s' := by
  match x, h, hx with
  | .ok (.done o), h, hx =>
    simp only [stepOf, StepResult.done.injEq] at hx
    subst hx
    exact h
  | .ok (.callNative _ _ _ _), _, hx => simp [stepOf] at hx
  | .error o, h, hx =>
    simp only [stepOf, StepResult.done.injEq] at hx
    subst hx
    exact h

theorem noInner_ref (r : Reference) : (Value.ref r).noInnerReference = true := by
  simp [Value.noInnerReference, Value.nodes]

section General

variable {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
  (hs : P.Supported)
  (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
    ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)
include hwf hs hapi

/-- **The frames of a state with the reference invariant hold references only
as whole values.** -/
theorem frames_unnested' {s : State} (hinv : InvRefs P s)
    (hargs : ∀ fr ∈ s.frames, ∀ x ∈ fr.function.typeArgs, x.isValue = true) :
    ∀ fr ∈ s.frames, fr.locals.toList.all (fun slot => slot.all Value.noInnerReference) = true ∧
      fr.stack.all Value.noInnerReference = true := by
  obtain ⟨fr, rest, hsf, ⟨info, code, hact⟩, -⟩ := hinv.1
  obtain ⟨instr, hi⟩ : ∃ instr, code.code[fr.pc]? = some instr :=
    ⟨_, Array.getElem?_eq_getElem hact.pcLt⟩
  obtain ⟨view, A, stys, tys, ps, hctx⟩ := topCtxGen hwf hs (apiSigs_of hwf hapi) hinv hsf hact hi
  obtain ⟨S, -, hchain⟩ := hctx.chain rest hsf
  intro g hg
  have hga := hargs g hg
  rw [hsf] at hg
  rcases List.mem_cons.1 hg with rfl | hg
  · refine ⟨locals_noInner' hga hact.locals, List.all_eq_true.2 fun v hv => ?_⟩
    obtain ⟨t, ht, hc⟩ := conformsAll_mem hctx.conf v hv
    exact noInner_of_conforms hc (slotType_of_ofSignature hctx.targs hctx.sig t ht)
  · refine ⟨locals_noInner' hga (ChainRef.locals hchain g hg), List.all_eq_true.2 fun v hv => ?_⟩
    obtain ⟨t, hc, ht⟩ := ChainRef.stack hchain g hg v hv
    exact noInner_of_conforms hc ht

/-- **The frames of an execution hold references only as whole values**, with
a wrapper in progress too: it holds its argument or the field borrow of it,
and the frames below are the caller's at the call. -/
theorem frames_unnested_all {s : State} (hinv : InvCore P s)
    (hargs : ∀ fr ∈ s.frames, ∀ x ∈ fr.function.typeArgs, x.isValue = true) :
    ∀ fr ∈ s.frames, fr.locals.toList.all (fun slot => slot.all Value.noInnerReference) = true ∧
      fr.stack.all Value.noInnerReference = true := by
  rcases hinv.2 with hr | ⟨sc, hrun⟩
  · exact frames_unnested' hwf hs hapi ⟨hinv.1, hr⟩ hargs
  · obtain ⟨fr, rest, info, code, off, x, st, w, cinfo, hsc, -, hst, -, hs'f, -, -, -, -, -,
      hlocs, hpc⟩ := hrun.call
    have hargs' : ∀ g ∈ sc.frames, ∀ x ∈ g.function.typeArgs, x.isValue = true := by
      intro g hg
      rw [hsc] at hg
      rcases List.mem_cons.1 hg with rfl | hg
      · have hm : { g with stack := st } ∈ s.frames := by
          rw [hs'f]; exact List.mem_cons_of_mem _ (List.mem_cons_self ..)
        exact hargs { g with stack := st } hm
      · have hm : g ∈ s.frames := by
          rw [hs'f]; exact List.mem_cons_of_mem _ (List.mem_cons_of_mem _ hg)
        exact hargs _ hm
    have hsc' := frames_unnested' hwf hs hapi hrun.inv hargs'
    intro g hg
    rw [hs'f] at hg
    rcases List.mem_cons.1 hg with rfl | hg
    · refine ⟨List.all_eq_true.2 fun slot hslot => ?_, ?_⟩
      · cases slot with
        | none => rfl
        | some v =>
          obtain ⟨j, hj⟩ := List.mem_iff_getElem?.1 hslot
          rw [Array.getElem?_toList] at hj
          have hj0 := hlocs j v hj
          subst hj0
          rcases hpc with ⟨-, h0, -⟩ | ⟨-, h0, -⟩ | ⟨-, h0, -⟩
          · rw [h0] at hj
            simp only [Option.some.injEq] at hj
            subst hj
            simp [noInner_ref]
          · rw [h0] at hj; cases hj
          · rw [h0] at hj; cases hj
      · rcases hpc with ⟨-, -, hstk⟩ | ⟨-, -, hstk⟩ | ⟨-, -, y, -, hstk, -⟩
        · simp [hstk]
        · simp [hstk, noInner_ref]
        · simp [hstk, noInner_ref]
    · rcases List.mem_cons.1 hg with rfl | hg
      · obtain ⟨h1, h2⟩ := hsc' fr (by rw [hsc]; exact List.mem_cons_self ..)
        refine ⟨h1, ?_⟩
        rw [hst] at h2
        simp only [List.all_cons, Bool.and_eq_true] at h2
        exact h2.2
      · exact hsc' g (by rw [hsc]; exact List.mem_cons_of_mem _ hg)

/-- **No reference inside a value, in storage or among the held values**, in
every state of an execution. -/
theorem unnested_all {N : NativeRel} {ν : NativeFlows} {s₀ : State} (hN : NativesRespect P N)
    (h0 : AdmissibleInitial P s₀) {s : State} {h : Shadow} {E : List Event}
    (hT : Trace P N ν s₀ s h E) : RefSafety.unnested s = true := by
  have hI := refTraceInv_trace h0 hT
  have hinv := invCore_reachable hwf hs hapi hN h0 s (trace_reachable hT)
  have hfr := frames_unnested_all hwf hs hapi hinv hI.typed.args
  simp only [RefSafety.unnested, Bool.and_eq_true, List.all_eq_true]
  refine ⟨⟨fun fr hfr' => ?_, fun r hr => ?_⟩, fun v hv => ?_⟩
  · obtain ⟨h1, h2⟩ := hfr fr hfr'
    exact ⟨List.all_eq_true.1 h1, List.all_eq_true.1 h2⟩
  · exact referenceFree_of_plain (hI.pre.refsTop.storage r hr)
  · obtain ⟨hsh, hval⟩ := hI.typed.held v hv
    exact referenceFree_of_plain (shaped_noRef v hsh hval)

/-- **A step hands back no reference inside a value**, in the next state, or
in the returned values and committed storage. -/
theorem unnestedAfter_all {N : NativeRel} {ν : NativeFlows} {s₀ : State}
    (hN : NativesRespect P N) (h0 : AdmissibleInitial P s₀) {s : State} {h : Shadow}
    {E : List Event} (hT : Trace P N ν s₀ s h E) {o : Outcome} {post : ShadowPost}
    {evs : List Event} (hS : TracedStep P N ν s h o post evs) :
    RefSafety.unnestedAfter o = true := by
  have hI := refTraceInv_trace h0 hT
  have hfit := traced_fits hS
  cases o with
  | running s' =>
    cases post with
    | running h' => exact unnested_all hwf hs hapi hN h0 (.step hT hS)
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
        have hinv := invCore_reachable hwf hs hapi hN h0 s (trace_reachable hT)
        have hfr := (frames_unnested_all hwf hs hapi hinv hI.typed.args fr
          (by rw [hsf]; exact List.mem_cons_self ..)).2
        simp only [RefSafety.unnestedAfter, Bool.and_eq_true, List.all_eq_true, List.mem_reverse]
        exact ⟨fun v hv => List.all_eq_true.1 hfr v hv,
          fun r hr => referenceFree_of_plain (hI.pre.refsTop.storage r hr)⟩
  | failed _ => rfl
  | wrong _ => rfl

/-- **What `Ret` hands back**, from a state with the reference invariant. -/
theorem ret_roots' {s : State} (hinv : InvRefs P s) {fr : Frame} {rest : List Frame}
    (hsf : s.frames = fr :: rest) {info : FunctionInfo} {code : CodeUnit}
    (hinfo : P.function? fr.function = some info) (hcode : info.definition.code = some code)
    (hinstr : code.code[fr.pc]? = some .ret) {rets : List RuntimeType}
    (hrets : P.returnTypes fr.function = some rets)
    (hconf : conformsAll P fr.stack.reverse rets = true) {r : Reference}
    (hr : Value.ref r ∈ fr.stack) :
    (∀ i, r.location.root ≠ .local_ fr.id i) ∧
      ∀ a t, r.location.root = .global a t → acqTypes P fr.function t →
        2 ≤ countActive P (s.frames.map Frame.skel) t.module := by
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
  obtain ⟨rv0, hrv0, hsig0⟩ := hctx.cert.setting.agrees.returns
  rw [hrets, Option.some.injEq] at hrv0
  subst hrv0
  have hlenA : A.stack.length ≤ view.returns.length := by
    rw [hctx.height, ← List.length_reverse, conformsAll_length hconf, ofSignature_length hsig0]
    exact Nat.le_refl _
  obtain ⟨hrc1, -, -, hrc4⟩ := return_contract hctx hlenA
  have hra := mem_argRefs.2 hr
  refine ⟨hrc4 r hra, fun a t hroot hacq => ?_⟩
  obtain ⟨π, hπ, hin, -⟩ := hrc1 r hra
  obtain ⟨S, hglob, -⟩ := hctx.chain rest hsf
  have hSt := hglob.params π hπ a t (by rw [← hin.1, hroot])
  rw [hsf]
  exact hglob.acq t hSt hacq

/-- **What `Ret` hands back**, in every state of an execution: a wrapper hands
back the field borrow of its argument, rooted below it, and acquires
nothing. -/
theorem ret_roots_all {s : State} (hinv : InvCore P s) {fr : Frame} {rest : List Frame}
    (hsf : s.frames = fr :: rest) {info : FunctionInfo} {code : CodeUnit}
    (hinfo : P.function? fr.function = some info) (hcode : info.definition.code = some code)
    (hinstr : code.code[fr.pc]? = some .ret) {rets : List RuntimeType}
    (hrets : P.returnTypes fr.function = some rets)
    (hconf : conformsAll P fr.stack.reverse rets = true) {r : Reference}
    (hr : Value.ref r ∈ fr.stack) :
    (∀ i, r.location.root ≠ .local_ fr.id i) ∧
      ∀ a t, r.location.root = .global a t → acqTypes P fr.function t →
        2 ≤ countActive P (s.frames.map Frame.skel) t.module := by
  rcases hinv.2 with hr' | ⟨sc, hrun⟩
  · exact ret_roots' hwf hs hapi ⟨hinv.1, hr'⟩ hsf hinfo hcode hinstr hrets hconf hr
  · have hpc2 : fr.pc = 2 := by
      rcases apiCore_instr hrun hsf hinfo hcode hinstr with ⟨-, h⟩ |
        ⟨-, ⟨f, h⟩ | ⟨v, h⟩ | ⟨f, h⟩ | ⟨v, h⟩⟩ | ⟨h, -⟩ <;> first | exact h | cases h
    have hnoacq : ∀ t, ¬ acqTypes P fr.function t := fun t => apiCore_noAcq hrun hsf
    obtain ⟨frc, rest₀, info₀, code₀, off, x, st, w, cinfo, hsc, -, -, -, hs'f, -, hwid, -, -, -,
      -, hpc⟩ := hrun.call
    rw [hsf, List.cons.injEq] at hs'f
    obtain ⟨rfl, -⟩ := hs'f
    have hlt : frc.id < fr.id := by
      rw [hwid]; exact hrun.inv.2.below frc (by rw [hsc]; exact List.mem_cons_self ..)
    rcases hpc with ⟨h0, -⟩ | ⟨h1, -⟩ | ⟨-, -, y, step, hstk, -, -, -, hok⟩
    · rw [hpc2] at h0; cases h0
    · rw [hpc2] at h1; cases h1
    · rw [hstk] at hr
      simp only [List.mem_singleton, Value.ref.injEq] at hr
      subst hr
      refine ⟨fun i hi => ?_, fun a t _ hacq => absurd hacq (hnoacq t)⟩
      have hb := hok.below
      rw [hi] at hb
      exact Nat.lt_irrefl _ (Nat.lt_of_le_of_lt hb hlt)

/-- **After a struct-API call** from a state with the reference invariant, a
wrapper is in progress, called from that state. -/
theorem apiCall_post (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel} {s : State}
    (hinv : InvRefs P s) {fr : Frame} {rest : List Frame} (hsf : s.frames = fr :: rest)
    {info : FunctionInfo} {code : CodeUnit}
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code) {h : FunctionHandleIndex}
    (hi : code.code[fr.pc]? = some (.call h)) {fh : FunctionHandle}
    (hfh : info.module.functionHandles[h]? = some fh) {off : Nat}
    (hoff : ReferenceSafety.borrowFieldMutable fh.attributes = some off) {s' : State}
    (hstep : stepOf (Step.exec P s fr rest info code (.call h)) = .done (.running s')) :
    ApiRun P s' s := by
  suffices hruns : ExecRuns P N (fun s'' => ApiRun P s'' s)
      (Step.exec P s fr rest info code (.call h)) from hruns.running hstep
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
    | (refine apiRun_of_call hinv hsf hact hi hfh hoff hst hres hcf hwc rfl ?_ rfl ?_ rfl
        ?_ ?_ ?_ ?_ ?_ ?_ (n := ltys.length - 1) ?_ rfl rfl
       all_goals first
         | rfl
         | (simp [hst]; done)
         | simp_all [State.incActive, State.activeCount])

/-- **After a generic struct-API call** from a state with the reference
invariant, a generic wrapper is in progress, called from that state. -/
theorem apiCallG_post (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel} {s : State}
    (hinv : InvRefs P s) {fr : Frame} {rest : List Frame} (hsf : s.frames = fr :: rest)
    {info : FunctionInfo} {code : CodeUnit}
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code) {i : Nat}
    (hi : code.code[fr.pc]? = some (.callGeneric i)) {fi : FunctionInstantiation}
    (hfi : info.module.functionInstantiations[i]? = some fi) {fh : FunctionHandle}
    (hfh : info.module.functionHandles[fi.handle]? = some fh) {off : Nat}
    (hoff : ReferenceSafety.borrowFieldMutable fh.attributes = some off) {s' : State}
    (hstep : stepOf (Step.exec P s fr rest info code (.callGeneric i)) = .done (.running s')) :
    ApiRunG P s' s := by
  suffices hruns : ExecRuns P N (fun s'' => ApiRunG P s'' s)
      (Step.exec P s fr rest info code (.callGeneric i)) from hruns.running hstep
  obtain ⟨view, A, stys, tys, ps, hctx⟩ := topCtxGen hwf hs (apiSigs_of hwf hapi) hinv hsf hact hi
  obtain ⟨-, htypes⟩ := hc.types fr.function info code hact.fn hact.body hact.typeArgs
  obtain ⟨before, after, hbefore, hafter, -⟩ := htypes fr.pc _ hact.reachable hi
  have hbt := hctx.before hbefore
  subst hbt
  have hta := hafter
  dsimp only [typeAfter] at hta
  simp only [bind, Option.bind_eq_some_iff] at hta
  obtain ⟨x, hx, fh', hfh', tys', ⟨targs, htargs, hmatch⟩, hta⟩ := hta
  rw [hfi, Option.some.injEq] at hx
  subst hx
  rw [hfh, Option.some.injEq] at hfh'
  subst hfh'
  obtain ⟨rfl, hne⟩ := typeArgsMatch_some hmatch
  obtain ⟨callee, cinfo, fh', hres, hvis, hcf, hfh', hclen, hcons, hsig⟩ :=
    hc.callsGeneric fr.function info code hact.fn hact.body fr.pc i fi hact.reachable hi hfi
  rw [hfh, Option.some.injEq] at hfh'
  subst hfh'
  split at hta
  · simp at hta
  next hwfT =>
  have hwfT' : RuntimeType.argumentsWellFormed P targs fh.typeParameters = true := by
    simpa using hwfT
  have hlen := argumentsWellFormed_length hwfT'
  simp only [Option.bind_eq_some_iff] at hta
  obtain ⟨params, hparams, returns, hreturns, hta⟩ := hta
  split at hta
  case isFalse => simp at hta
  rename_i hpre
  obtain ⟨hpar, hret⟩ := hsig targs hlen
  have hexec : Step.exec P s fr rest info code (.callGeneric i) =
      Step.call P s fr rest info fi.handle targs := by
    simp only [Step.exec, Step.need, hfi, pure, Except.pure,
      genericArgs_of_typeArgsWith htargs hne, bind, Except.bind]
  rw [hexec]
  have hwfa := argumentsWellFormed_weaken hwfT' hclen hcons
  have hlenF : fh.typeParameters.length = targs.length := hlen.symm
  have hclen' : cinfo.handle.typeParameters.length = targs.length := hclen.trans hlen.symm
  have hcf' : P.function? { callee with typeArgs := targs } = some cinfo := by
    rw [function?_typeArgs]; exact hcf
  have htok : P.typeArgsOk { callee with typeArgs := targs } = true := by
    simp only [Program.typeArgsOk, hcf']; exact hwfa
  have hpar' : P.paramTypes { callee with typeArgs := targs } = some params := hpar.trans hparams
  have hret' : P.returnTypes { callee with typeArgs := targs } = some returns :=
    hret.trans hreturns
  have hstack := hctx.conf
  have hlen' := conformsAll_length hstack
  have htl := assignableAll_length hpre
  have hge : params.length ≤ fr.stack.length := by
    rw [hlen']; simp only [List.length_take, List.length_reverse] at htl; omega
  have hargs : conformsAll P (fr.stack.take params.length).reverse params = true := by
    have := conformsAll_reverse
      (conformsAll_assignable (conformsAll_take params.length hstack) hpre)
    rwa [List.reverse_reverse] at this
  -- The callee is the wrapper: one `&mut` parameter, so one reference argument.
  have hwc := apiCall_wrapperG hwf hapi hact.fn hfh hoff hres hcf
  obtain ⟨ccode, b, hccode, hcl, -⟩ := hwc.code
  obtain ⟨inner, hpsig⟩ := hwc.param
  obtain ⟨sig, hsig', hps⟩ := paramTypes_sig hcf' hpar'
  rw [hpsig, Option.some.injEq] at hsig'
  subst hsig'
  obtain ⟨u, us, hu, hus, rfl⟩ := ofSignatureWith_cons.1 hps
  simp only [RuntimeType.ofSignatureWith, List.mapM_nil, Option.pure_def, Option.some.injEq] at hus
  subst hus
  obtain ⟨ut, rfl⟩ := ofSlotToken_ref hu (b := true) rfl
  obtain ⟨x, st, hst, hxm⟩ : ∃ x st, fr.stack = .ref x :: st ∧ x.mutable = true := by
    cases hfs : fr.stack with
    | nil => rw [hfs] at hge; simp at hge
    | cons a l =>
      rw [hfs] at hargs
      simp only [List.length_cons, List.length_nil, Nat.zero_add, List.take_succ_cons,
        List.take_zero, List.reverse_cons, List.reverse_nil, List.nil_append,
        conformsAll_cons] at hargs
      obtain ⟨r, rfl, hm, -⟩ := conforms_ref hargs.1 (by simp [rigid])
      exact ⟨r, l, rfl, hm⟩
  have hge1 : ¬ fr.stack.length < [RuntimeType.ref true ut].length := by
    rw [hst]; simp
  clear hlen hclen
  simp only [Step.call, Step.need, hres, hvis, hcf, hfh, hlenF, hclen', hwfa, hpar', hret',
    hparams, hreturns, Step.popN, hge1, hargs, bne_self_eq_false, Bool.or_false, Bool.not_true,
    Bool.false_eq_true, ↓reduceIte, bind, Except.bind, pure, Except.pure,
    hccode, Option.isNone_some, Bool.false_and]
  obtain ⟨ltys, hltys⟩ := hc.locals _ cinfo ccode hcf' hccode htok
  simp only [hltys]
  repeat' split
  all_goals first
    | trivial
    | (refine apiRunG_of_call hinv hsf hact hi hfi hfh hoff hst hxm hwc rfl ?_
        hcf' ?_ rfl ?_ ?_ ?_ ?_ ?_ ?_ (n := ltys.length - 1) ?_ rfl rfl
       all_goals first
         | rfl
         | (simp [hst]; done)
         | simp_all [State.incActive, State.activeCount])

/-- **The argument of a struct-API call**, in the caller's state with the
reference invariant: a tracked reference of the caller, and, when it is into
a resource, a resource of a type whose module is active. -/
theorem apiArg_facts {s : State} (hinv : InvRefs P s) {fr : Frame} {rest : List Frame}
    (hsf : s.frames = fr :: rest) {info : FunctionInfo} {code : CodeUnit}
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code) {instr : Bytecode}
    {h : FunctionHandleIndex} (hi : code.code[fr.pc]? = some instr)
    (hcall : instr = .call h ∨ ∃ i fi, instr = .callGeneric i ∧
      info.module.functionInstantiations[i]? = some fi ∧ fi.handle = h) {fh : FunctionHandle}
    (hfh : info.module.functionHandles[h]? = some fh) {off : Nat}
    (hoff : ReferenceSafety.borrowFieldMutable fh.attributes = some off) {x : Reference}
    {st : List Value} (hst : fr.stack = .ref x :: st) :
    RefOk P s fr x ∧ ∀ a t, x.location.root = .global a t →
      1 ≤ countActive P (s.frames.map Frame.skel) t.module := by
  obtain ⟨view, A, stys, tys, ps, hctx⟩ := topCtxGen hwf hs (apiSigs_of hwf hapi) hinv hsf hact hi
  obtain ⟨A', hx, -⟩ := hctx.next (by
    rcases hcall with rfl | ⟨i, fi, rfl, -⟩ <;> intro h <;> cases h)
  have hx' : (ReferenceSafety.executeInner info.module view (.call h)).run A = .ok ((), A') := by
    rcases hcall with rfl | ⟨i, fi, rfl, hfi, rfl⟩
    · exact hx
    · obtain ⟨fi', hfi', h'⟩ := exec_callGeneric' hx
      rw [hfi, Option.some.injEq] at hfi'
      subst hfi'
      exact h'
  obtain ⟨id, ra, g1, g2, g3, hsa, -, -, -, -, -⟩ := exec_callApi hx' hfh hoff
  obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
  have hlen : ra.length = st.length := by
    have := hctx.height; rw [hsa, hst] at this; simpa using this
  obtain ⟨r', hr', hL⟩ := hd.top hsa hst hlen
  simp only [Value.ref.injEq] at hr'
  subst hr'
  have hp : id ∈ heldIds A := mem_heldIds.2 (Or.inr (by rw [hsa]; exact List.mem_cons_self))
  refine ⟨hrefs id x hp hL, fun a t hr => ?_⟩
  obtain ⟨S, hglob, -⟩ := hctx.chain rest hsf
  have hsk : s.frames.map Frame.skel =
      (fr.function, fr.countedActive) :: rest.map Frame.skel := by rw [hsf]; rfl
  have hc0 := hd.complete
  rw [hsk] at hc0 ⊢
  rcases hc0.prov id x hp hL with ⟨i, hi'⟩ | ⟨d, hrc⟩ | ⟨π, hπ, hin, -⟩
  · rw [hr] at hi'; cases hi'
  · rcases hrc.root_of hL with ⟨i, hi'⟩ | ⟨a', d', t', hd', hr'⟩
    · rw [hr] at hi'; cases hi'
    · rw [hr] at hr'
      simp only [Root.global.injEq] at hr'
      obtain ⟨-, rfl⟩ := hr'
      obtain ⟨hacqT, -⟩ := gsidOf_cons hd'
      have hcounted := hinv.2.active.counted
      rw [hsk] at hcounted
      exact countActive_self hcounted (acqTypes_funModule hacqT)
  · exact hglob.active t (hglob.params π hπ a t (by rw [← hin.1, hr]))

end General

end MoveBytecode.Proofs
