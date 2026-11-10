-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.ApiRun

/-!
# The struct-API wrapper returns

UNTRUSTED (checked by the kernel). When the wrapper of a struct-API mutable
borrow returns (`apiRun_ret`), its caller resumes with the field borrow of
its argument on top of its operand stack: as if it had executed a mutable
field borrow in the state `sc` it called from, after the removal of the
wrapper's local 0, which no reference is into. The analysis took the call
for exactly that borrow (`exec_callApi`), so the reference invariant holds
again (`refInv_finish` from `sc`, `Complete.fieldBorrowPre`), with the
activation counts as at the call.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph MoveBytecode.Step

variable {P : Program}

/-! ## Safety, weakened -/

theorem Safe.weaken {Q R : State → Prop} (hqr : ∀ s, Q s → R s) :
    ∀ {o : Outcome}, Safe Q o → Safe R o
  | .running s, h => hqr s h
  | .returned _ _, _ => trivial
  | .failed _, _ => trivial
  | .wrong _, h => h

theorem ExecSafe.weaken {N : NativeRel} {Q R : State → Prop} (hqr : ∀ s, Q s → R s) :
    ∀ {x : Step.Exec StepResult}, ExecSafe P N Q x → ExecSafe P N R x
  | .ok (.done _), h => Safe.weaken hqr h
  | .ok (.callNative _ _ _ _), h => fun r hr => Safe.weaken hqr (h r hr)
  | .error _, h => Safe.weaken hqr h

/-! ## The reference invariant across states that agree -/

/-- **The reference invariant of a state that agrees with one that keeps it**
on its frames, held values, global storage, invalidations and activation
counts, with no fewer frame identities issued. -/
theorem RefInvariant.congr {s s' : State} (h : RefInvariant P s) (hfr : s'.frames = s.frames)
    (hheld : s'.held = s.held) (hstor : s'.storage = s.storage)
    (hinv : s'.invalidations = s.invalidations) (hnext : s.nextFrame ≤ s'.nextFrame)
    (hcount : ∀ M, s'.activeCount M = s.activeCount M) : RefInvariant P s' := by
  obtain ⟨hids, hbelow, hstor0, ⟨fr, rest, ps, hsf, ⟨A, hA, L, hd, hok⟩, S, hglob, hch⟩,
    hactv, hheldT⟩ := h
  have hframe : ∀ f, s'.frame? f = s.frame? f := fun f => by simp only [State.frame?, hfr]
  have hs'f : s'.frames = fr :: rest := hfr.trans hsf
  refine ⟨by rw [hfr]; exact hids, fun x hx => Nat.lt_of_lt_of_le (hbelow x (hfr ▸ hx)) hnext,
    fun r hr => hstor0 r (hstor ▸ hr), ⟨fr, rest, ps, hs'f, ⟨A, hA, L, ?_, fun x r hx hxr => ?_⟩,
    S, hglob, ?_⟩, ⟨fun M => ?_, by rw [hfr]; exact hactv.counted⟩,
    hheldT.of fun g hg r hr _ _ => ⟨g, hfr ▸ hg, r, hr, rfl, rfl⟩⟩
  · rw [hfr]; exact hd
  · exact (hok x r hx hxr).agree (hok x r hx hxr).below
      (fun f _ g hg => ⟨g, by rw [hframe]; exact hg, rfl, rfl⟩) hheld hstor hinv (Nat.le_refl _)
  · exact ChainRef.transfer hsf hs'f rfl hheld (es := []) (by rw [hinv]; rfl)
      (storageKept_of_eq hstor) rest fr.id fr.function ps S (Nat.le_refl _)
      (fun e he => by cases he) hch
  · rw [hcount, hfr]; exact hactv.count M

/-! ## The caller's argument -/

/-- **The argument of a struct-API borrow call is a mutable reference**: the
call's arguments conform to the parameter types of the wrapper, a single
`&mut`. -/
theorem apiArg_mutable (hc : Certified P (modelTypes P) (modelAvail P)) {s : State}
    {fr : Frame} {info : FunctionInfo} {code : CodeUnit}
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code) {h : Nat}
    (hi : code.code[fr.pc]? = some (.call h)) {view : FunctionView} {A : RefState}
    {stys : List SignatureToken} {tys : List RuntimeType} {ps : List Reference}
    (hctx : TopCtx P s fr info code (.call h) view A stys tys ps) {callee : FunctionRef}
    (hres : P.resolve fr.function.module h = some callee) {cinfo : FunctionInfo}
    (hcf : P.function? callee = some cinfo) {inner : SignatureToken}
    (hsig : cinfo.module.signatures[cinfo.handle.parameters]? =
      some [.mutableReference inner])
    {x : Reference} {st : List Value} (hstk : fr.stack = .ref x :: st) : x.mutable = true := by
  obtain ⟨callee', cinfo', hres', -, hcf', -, hpar, -⟩ :=
    hc.calls fr.function info code hact.fn hact.body fr.pc h hact.reachable hi
  rw [hres, Option.some.injEq] at hres'
  subst hres'
  rw [hcf, Option.some.injEq] at hcf'
  subst hcf'
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
  obtain ⟨params, hparams, returns, -, hta⟩ := hta
  split at hta
  case isFalse => simp at hta
  rename_i hpre
  have hstack := hctx.conf
  have hargs : conformsAll P (fr.stack.take params.length).reverse params = true := by
    have := conformsAll_reverse
      (conformsAll_assignable (conformsAll_take params.length hstack) hpre)
    rwa [List.reverse_reverse] at this
  have hpar' : P.paramTypes callee = some params := hpar.trans hparams
  simp only [Program.paramTypes, hcf, hsig, resolve_typeArgs hres, bind, Option.bind_some] at hpar'
  obtain ⟨u, us, hu, hus, rfl⟩ := ofSignature_cons.1 hpar'
  simp only [RuntimeType.ofSignature, RuntimeType.ofSignatureWith, List.mapM_nil, pure,
    Option.some.injEq] at hus
  subst hus
  obtain ⟨v, rfl⟩ := ofSlotToken_ref hu (b := true) rfl
  rw [hstk] at hargs
  simp only [List.length_cons, List.length_nil, Nat.zero_add, List.take_succ_cons, List.take_zero,
    List.reverse_cons, List.reverse_nil, List.nil_append, conformsAll_cons] at hargs
  obtain ⟨r, hr, hm, -⟩ := conforms_ref hargs.1 (by simp [rigid])
  simp only [Value.ref.injEq] at hr
  subst hr
  exact hm

/-! ## The return -/

/-- The frames of the state the wrapper runs in, by identity, against those
of the resumed caller: below the wrapper they agree, the caller with its new
operand stack and position. -/
theorem api_frames {s s' : State} {w fr fr' : Frame} {rest : List Frame}
    (hsf : s.frames = w :: fr :: rest) (hs'f : s'.frames = fr' :: rest)
    (hid : fr'.id = fr.id) (hfun : fr'.function = fr.function) (hloc : fr'.locals = fr.locals)
    {b : FrameId} (hb : b < w.id) :
    ∀ f, f ≤ b → ∀ g, s.frame? f = some g →
      ∃ g', s'.frame? f = some g' ∧ g'.function = g.function ∧ g'.locals = g.locals := by
  intro f hf g hg
  have hw : (w.id == f) = false := by
    simp only [beq_eq_false_iff_ne]
    intro e
    exact Nat.lt_irrefl f (Nat.lt_of_le_of_lt hf (e ▸ hb))
  simp only [State.frame?, hsf, List.find?_cons, hw] at hg
  simp only [State.frame?, hs'f, List.find?_cons, hid]
  cases hfe : fr.id == f with
  | true =>
    rw [hfe] at hg
    simp only [Option.some.injEq] at hg
    subst hg
    exact ⟨fr', rfl, hfun, hloc⟩
  | false =>
    rw [hfe] at hg
    exact ⟨g, hg, rfl, rfl⟩

theorem activeCount_eq_of {s t : State} (h : s.active = t.active) (M : ModuleId) :
    s.activeCount M = t.activeCount M := by
  simp only [State.activeCount, h]

/-- **The activations are back** after a call increments the callee's
module's count and its return decrements it. -/
theorem api_count {s sc s' : State} {id : ModuleId} {b : Bool}
    (hactv : s.active = (if b then sc.incActive id else sc).active)
    (hs' : s'.active = (if b then s.decActive id else s).active) :
    ∀ M, s'.activeCount M = sc.activeCount M := by
  intro M
  cases b with
  | false =>
    simp only [Bool.false_eq_true, if_false] at hactv hs'
    rw [activeCount_eq_of hs', activeCount_eq_of hactv]
  | true =>
    simp only [if_true] at hactv hs'
    have h1 : ∀ M, s.activeCount M = (sc.incActive id).activeCount M := activeCount_eq_of hactv
    rw [activeCount_eq_of hs', activeCount_decActive]
    by_cases e : M = id
    · subst e
      rw [if_pos rfl, h1, activeCount_incActive, if_pos rfl]
      omega
    · rw [if_neg e, h1, activeCount_incActive, if_neg e]

section Return

variable (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.Supported)
  (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
    ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)
  (hc : Certified P (modelTypes P) (modelAvail P))
include hwf hs hapi hc

/-- **The caller resumes**: after the wrapper's `Ret`, the reference
invariant holds again. -/
theorem apiRet_inv {s sc s' : State} (hrun : ApiRun P s sc) {w caller : Frame}
    {rest' : List Frame} (hsf : s.frames = w :: caller :: rest') (hpc : w.pc = 2)
    (hs'f : s'.frames =
      { caller with stack := w.stack ++ caller.stack, pc := caller.pc + 1 } :: rest')
    (hinv' : s'.invalidations = s.invalidations) (hstor' : s'.storage = s.storage)
    (hheld' : s'.held = s.held) (hnext' : s'.nextFrame = s.nextFrame)
    (hcount : ∀ M, s'.activeCount M = sc.activeCount M) : RefInvariant P s' := by
  obtain ⟨fr, rest, info, code, h, fh, off, x, st, w₀, callee, cinfo, hscf, hactC, hiC, hfh, hoff,
    hstk, hres, hcf, hwc, hsf₀, hwfun, hwid, -, hnext, hstor, hheld, -, -, ⟨es, hes, hesl⟩, -,
    hpcs⟩ := hrun.call
  rw [hsf₀, List.cons.injEq, List.cons.injEq] at hsf
  obtain ⟨rfl, rfl, rfl⟩ := hsf
  rcases hpcs with ⟨h0, -⟩ | ⟨h1, -⟩ | ⟨-, -, y, step, hwst, hym, hyloc, hstep, hyok⟩
  · rw [hpc] at h0; cases h0
  · rw [hpc] at h1; cases h1
  have hinvSc := hrun.inv
  have hr := hinvSc.2
  obtain ⟨codeW, b, sd, sdef, sh, inner, -, -, hsig, hinner, -, -⟩ := hwc.code
  obtain ⟨view, A, stys, tys, ps, hctx⟩ :=
    topCtxGen hwf hs (apiSigs_of hwf hapi) hinvSc hscf hactC hiC
  have hxm := apiArg_mutable hc hactC hiC hctx hres hcf hsig hstk
  obtain ⟨A', hx, hsucc⟩ := hctx.next (by intro e; cases e)
  obtain ⟨id, ra, g1, g2, g3, hsa, hfull, hnew, hfac, hrel, rfl⟩ := exec_callApi hx hfh hoff
  obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
  have hlen : ra.length = st.length := by
    have := hctx.height; rw [hsa, hstk] at this; simpa using this
  obtain ⟨r', hr', hL⟩ := hd.top hsa hstk hlen
  simp only [Value.ref.injEq] at hr'
  subst hr'
  have hp : id ∈ heldIds A := mem_heldIds.2 (Or.inr (by rw [hsa]; exact List.mem_cons_self))
  have hnew' : A.graph.newRef A.nextId y.mutable = .ok g1 := by rw [hym]; exact hnew
  have hc2 := hd.complete.fieldBorrowPre (r := y) hp hL hstep hyloc (fun _ => hxm)
    (fun _ => hfull) (fun hf => by rw [hym] at hf; cases hf) hnew' hfac
  have hc3 := hc2.release (Or.inl hp) hrel
  have hfresh := (Graph.newRef_ok hnew).1
  -- The resumed caller.
  let fr' : Frame := { { fr with stack := st } with stack := w₀.stack ++ st, pc := fr.pc + 1 }
  have hfr'st : fr'.stack = .ref y :: st := by simp [fr', hwst]
  let A₁ : RefState :=
    { A with stack := .reference A.nextId :: ra, graph := g3, nextId := A.nextId + 1 }
  have hd' := hd.replaceTop hctx.typed hsa hstk hlen hfresh hc3 (A' := A₁)
    (fr' := fr') ⟨rfl, rfl, rfl⟩ rfl rfl hfr'st
  obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1) (by simp [successors, hiC, Bytecode.control])
  -- The state after the return, with the activations of the call's state.
  let s'' : State := { s' with active := sc.active }
  have hs''f : s''.frames = fr' :: rest := hs'f
  have hfrBelow : fr.id < sc.nextFrame := hr.below fr (by rw [hscf]; exact List.mem_cons_self)
  have hwlt : fr.id < w₀.id := by rw [hwid]; exact hfrBelow
  -- The wrapper's removal of its local covers no reference below it.
  have hcovers : ∀ {r : Reference}, RootBelow fr.id r.location.root →
      ∀ e ∈ es, e.covers r.location = false := by
    intro r hb e he
    rw [hesl e he]
    simp only [Invalidation.covers, beq_eq_false_iff_ne]
    intro e'
    rw [e'] at hb
    simp only [RootBelow] at hb
    exact Nat.lt_irrefl _ (Nat.lt_of_le_of_lt hb hwlt)
  have hal : AllowedStep P fr rest ps es := by
    intro S _ e he l hl
    rw [hesl e he] at hl
    simp only [Invalidation.covers, beq_iff_eq] at hl
    exact Or.inl ⟨w₀.id, 0, hl, Nat.le_of_lt hwlt⟩
  have hok : ∀ z rz, z ∈ heldIds A₁ → updL L A.nextId y z = some rz → RefOk P s'' fr' rz := by
    intro z rz hz hzr
    by_cases hzc : z = A.nextId
    · subst hzc
      rw [updL_self, Option.some.injEq] at hzr
      subst hzr
      exact hyok.agree hyok.below
        (api_frames (s' := s'') (fr' := fr') hsf₀ hs''f rfl rfl rfl (b := fr.id) hwlt)
        hheld' hstor' hinv' (Nat.le_refl _)
    · rw [updL_ne _ hzc] at hzr
      have hzA : z ∈ heldIds A := by
        have hz' : z ∈ A.locals.filterMap AbsValue.refId? ∨ z = A.nextId ∨
            z ∈ ra.filterMap AbsValue.refId? := by
          simpa [A₁, heldIds, AbsValue.refId?] using hz
        simp only [heldIds, hsa, List.filterMap_cons, AbsValue.refId?, List.mem_append,
          List.mem_cons]
        rcases hz' with hz' | hz' | hz'
        · exact Or.inl hz'
        · exact absurd hz' hzc
        · exact Or.inr (Or.inr hz')
      have hzok := hrefs z rz hzA hzr
      exact hzok.sameFrames hscf hs''f rfl rfl rfl (by rw [hheld', hheld])
        (storageKept_of_eq (by show s'.storage = sc.storage; rw [hstor', hstor]))
        (by show s'.invalidations = es ++ sc.invalidations; rw [hinv', hes])
        (hcovers hzok.below) rfl
  have hr'' : RefInvariant P s'' :=
    refInv_finish hr hscf (hctx.chain rest hscf) hs''f (show s''.held = sc.held by rw [← hheld]; exact hheld')
      rfl rfl rfl rfl (show sc.nextFrame ≤ s'.nextFrame by rw [hnext', hnext]; exact Nat.le_succ _)
      (fun r hr₀ => hr.storage r (by rw [← hstor, ← hstor']; exact hr₀))
      (by show s'.invalidations = es ++ sc.invalidations; rw [hinv', hes])
      (storageKept_of_eq (by show s'.storage = sc.storage; rw [hstor', hstor])) hal hd' hok
      hA'' hle
      (RefsFrom.next (new := [.ref y]) (n := 1) (by rw [hfr'st, hstk]; rfl) rfl fun v hv => by
        simp only [List.mem_singleton] at hv
        subst hv
        exact .of_loc fun k h => by rw [hyloc] at h; simp [Location.child] at h)
  exact hr''.congr (s' := s') rfl rfl rfl rfl (Nat.le_refl _)
    (fun M => by rw [hcount]; rfl)

/-- **The wrapper's `Ret`** is safe, and its caller resumes in the reference
invariant. -/
theorem apiRun_ret {N : NativeRel} (hN : NativesRespect P N) {s sc : State}
    (hrun : ApiRun P s sc) {fr0 : Frame} {rest0 : List Frame} (hsf : s.frames = fr0 :: rest0)
    {info0 : FunctionInfo} {code0 : CodeUnit}
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr0 info0 code0)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr0.function rest0)
    (hi : code0.code[fr0.pc]? = some .ret) :
    ExecSafe P N (InvAll P) (Step.exec P s fr0 rest0 info0 code0 .ret) := by
  have hcore := exec_safe (s := s) hc hN hact hchain hi rfl
    (fun h => by rcases h with h | h <;> cases h) trivial
  refine ExecSafe.weaken (fun s' h => ⟨h.1, Or.inl h.2⟩) (ExecSafe.and hcore ?_)
  obtain ⟨fr, rest, info, code, h, fh, off, x, st, w, callee, cinfo, -, -, -, -, -, -, -, hcf,
    hwc, hsf₀, hwfun, -, -, -, -, -, hactv, -, -, -, -⟩ := hrun.call
  have hsf₁ := hsf₀
  rw [hsf, List.cons.injEq] at hsf₁
  obtain ⟨rfl, rfl⟩ := hsf₁
  -- The wrapper's code, at its `Ret`.
  have hinfo : info0 = cinfo := by
    have := hact.fn; rw [hwfun, hcf, Option.some.injEq] at this; exact this.symm
  subst hinfo
  obtain ⟨codeW, b, sd, sdef, sh, inner, hcodeW, hlist, -, -, -, hb⟩ := hwc.code
  have hcode0 : code0 = codeW := by
    have := hact.body; rw [hcodeW, Option.some.injEq] at this; exact this.symm
  subst hcode0
  have hpc : fr0.pc = 2 := by
    rw [← Array.getElem?_toList, hlist] at hi
    have hlt : fr0.pc < 3 := by
      refine Nat.lt_of_not_le fun hge => ?_
      rw [List.getElem?_eq_none (by simpa using hge)] at hi
      cases hi
    rcases (by omega : fr0.pc = 0 ∨ fr0.pc = 1 ∨ fr0.pc = 2) with e | e | e
    · rw [e] at hi; simp at hi
    · rw [e] at hi
      simp only [List.getElem?_cons_succ, List.getElem?_cons_zero, Option.some.injEq] at hi
      rcases hb with ⟨_, _, rfl, _⟩ | ⟨_, _, rfl, _⟩ <;> cases hi
    · exact e
  -- The return.
  cases hrt : P.returnTypes fr0.function with
  | none =>
    simp only [Step.exec, Step.ret, hrt, Step.need, bind, Except.bind]
    trivial
  | some rets =>
    simp only [Step.exec, Step.ret, hrt, Step.need, bind, Except.bind, pure, Except.pure]
    by_cases h1 : fr0.stack.length < rets.length
    · simp only [h1, ↓reduceIte]; trivial
    by_cases h2 : fr0.stack.length > rets.length
    · simp only [h1, h2, ↓reduceIte]; trivial
    by_cases h3 : conformsAll P fr0.stack.reverse rets = true
    case neg =>
      simp only [h1, h2, ↓reduceIte, Bool.eq_false_iff.2 h3, Bool.not_false]; trivial
    simp only [h1, h2, h3, ↓reduceIte, Bool.not_true, Bool.false_eq_true]
    split
    · trivial
    cases hca : fr0.countedActive <;> cases hle : fr0.lockEntered <;>
      simp only [Bool.false_eq_true, if_false, if_true] <;>
      exact apiRet_inv hwf hs hapi hc hrun hsf₀ hpc rfl rfl rfl rfl rfl
        (api_count hactv (by rw [hca]; rfl))

end Return

end MoveBytecode.Proofs
