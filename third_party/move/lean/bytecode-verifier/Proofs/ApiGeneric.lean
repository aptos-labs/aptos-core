-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.SoundnessGeneric

/-!
# Generic struct-API mutable borrows

UNTRUSTED (checked by the kernel). A `CallGeneric` of a function handle with
`BorrowFieldMutable` calls a generic wrapper at the call's type arguments. As
for a non-generic call (`Proofs.ApiSteps`, `Proofs.ApiReturn`), the caller's
analysis takes the call for a field borrow, and the reference invariant does
not hold while the wrapper runs: a generic wrapper in progress (`ApiRunG`)
takes its place in the invariant of executions (`InvAllG`). The wrapper is
`MoveLoc(0)`, a mutable field or variant field borrow, generic or not, and
`Ret` (`WrapperCodeG`); its steps keep it in progress (`apiRunG_step`), and
its return resumes the caller as from a field borrow (`apiRunG_ret`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-- **The callee of a generic struct-API mutable borrow call**: one `&mut`
parameter, and the code `MoveLoc(0)`, a mutable borrow of field `off`, then
`Ret`. -/
structure WrapperCodeG (cinfo : FunctionInfo) (off : Nat) : Prop where
  code : ∃ code b, cinfo.definition.code = some code ∧ code.code.toList = [.moveLoc 0, b, .ret] ∧
    ((∃ fhi fhd, b = .mutBorrowField fhi ∧ cinfo.module.fieldHandles[fhi]? = some fhd ∧
        fhd.field = off) ∨
      (∃ vh vfhd, b = .mutBorrowVariantField vh ∧
        cinfo.module.variantFieldHandles[vh]? = some vfhd ∧ vfhd.field = off) ∨
      (∃ i fi fhd, b = .mutBorrowFieldGeneric i ∧ cinfo.module.fieldInstantiations[i]? = some fi ∧
        cinfo.module.fieldHandles[fi.handle]? = some fhd ∧ fhd.field = off) ∨
      (∃ i fi vfhd, b = .mutBorrowVariantFieldGeneric i ∧
        cinfo.module.variantFieldInstantiations[i]? = some fi ∧
        cinfo.module.variantFieldHandles[fi.handle]? = some vfhd ∧ vfhd.field = off))
  param : ∃ inner, cinfo.module.signatures[cinfo.handle.parameters]? = some [.mutableReference inner]

/-- **A struct-API mutable borrow call calls a wrapper.** -/
theorem apiCall_wrapperG {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
      ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)
    {f : FunctionRef} {info : FunctionInfo} (hf : P.function? f = some info) {h : Nat}
    {fh : FunctionHandle} (hfh : info.module.functionHandles[h]? = some fh) {off : Nat}
    (hoff : ReferenceSafety.borrowFieldMutable fh.attributes = some off) {callee : FunctionRef}
    (hres : P.resolve f.module h = some callee) {cinfo : FunctionInfo}
    (hcf : P.function? callee = some cinfo) : WrapperCodeG cinfo off := by
  obtain ⟨hm, -, -, -⟩ := function?_parts hf
  have hattr := resolve_attr hwf hm hfh hres hcf (borrowFieldMutable_mem hoff) rfl (by simp)
  obtain ⟨hcm, -, hcd, hch⟩ := function?_parts hcf
  obtain ⟨⟨⟨code, b, hcode, hb, sd, sh, hop, inner, hps, -⟩, -⟩, -⟩ :=
    defApi hwf hapi hcm hcd hch hattr
  refine ⟨⟨code, b, hcode, hb, ?_⟩, ⟨inner, hps⟩⟩
  rcases hop with ⟨fhi, fhd, rfl, h1, -, h2⟩ | ⟨vh, vfhd, rfl, h1, -, h2⟩ |
    ⟨i, fi, fhd, -, rfl, h0, h1, -, h2, -⟩ | ⟨i, fi, vfhd, -, rfl, h0, h1, -, h2, -⟩
  · exact Or.inl ⟨fhi, fhd, rfl, h1, h2⟩
  · exact Or.inr (Or.inl ⟨vh, vfhd, rfl, h1, h2⟩)
  · exact Or.inr (Or.inr (Or.inl ⟨i, fi, fhd, rfl, h0, h1, h2⟩))
  · exact Or.inr (Or.inr (Or.inr ⟨i, fi, vfhd, rfl, h0, h1, h2⟩))

/-- **A generic struct-API mutable borrow in progress**: as `ApiRun`, for a
`CallGeneric` of the wrapper; its argument is a mutable reference. -/
structure ApiRunG (P : Program) (s sc : State) : Prop where
  inv : InvRefs P sc
  call : ∃ (fr : Frame) (rest : List Frame) (info : FunctionInfo) (code : CodeUnit) (i : Nat)
      (fi : FunctionInstantiation) (fh : FunctionHandle) (off : Nat) (x : Reference)
      (st : List Value) (w : Frame) (cinfo : FunctionInfo),
    sc.frames = fr :: rest ∧ ActiveAt P (modelTypes P) (modelAvail P) fr info code ∧
    code.code[fr.pc]? = some (.callGeneric i) ∧ info.module.functionInstantiations[i]? = some fi ∧
    info.module.functionHandles[fi.handle]? = some fh ∧
    ReferenceSafety.borrowFieldMutable fh.attributes = some off ∧ fr.stack = .ref x :: st ∧
    x.mutable = true ∧ WrapperCodeG cinfo off ∧
    s.frames = w :: { fr with stack := st } :: rest ∧ P.function? w.function = some cinfo ∧
    w.id = sc.nextFrame ∧ w.countedActive = (cinfo.moduleId != info.moduleId) ∧
    s.nextFrame = sc.nextFrame + 1 ∧ s.storage = sc.storage ∧ s.held = sc.held ∧
    s.active = (if w.countedActive then sc.incActive cinfo.moduleId else sc).active ∧
    s.lockDepth = (if w.lockEntered then sc.lockDepth + 1 else sc.lockDepth) ∧
    (∃ es : List Invalidation, s.invalidations = es ++ sc.invalidations ∧
      ∀ e ∈ es, e = .removed (.local_ w.id 0)) ∧
    (∀ j v, w.locals[j]? = some (some v) → j = 0) ∧
    ((w.pc = 0 ∧ w.locals[0]? = some (some (.ref x)) ∧ w.stack = []) ∨
     (w.pc = 1 ∧ w.locals[0]? = some none ∧ w.stack = [.ref x]) ∨
     (w.pc = 2 ∧ w.locals[0]? = some none ∧ ∃ (y : Reference) (step : PathStep),
        w.stack = [.ref y] ∧ y.mutable = true ∧ y.location = x.location.child step ∧
        absPath [step] = [.field off] ∧ RefOk P s { fr with stack := st } y))

/-- **The invariant of executions with struct-API borrows, generic or not**:
types and availability, and the reference invariant or a wrapper in
progress. -/
def InvAllG (P : Program) (s : State) : Prop :=
  Inv P (modelTypes P) (modelAvail P) s ∧
    (RefInvariant P s ∨ (∃ sc, ApiRun P s sc) ∨ ∃ sc, ApiRunG P s sc)

theorem InvAll.invAllG {P : Program} {s : State} (h : InvAll P s) : InvAllG P s :=
  ⟨h.1, h.2.elim Or.inl (fun h => Or.inr (Or.inl h))⟩

section Call

variable {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
  (hs : P.Supported)
  (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
    ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)

/-- The state after a generic struct-API borrow call is a wrapper in progress. -/
theorem apiRunG_of_call {s s' : State} (hinv : InvRefs P s) {fr : Frame} {rest : List Frame}
    (hsf : s.frames = fr :: rest) {info : FunctionInfo} {code : CodeUnit}
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    {i : Nat} (hi : code.code[fr.pc]? = some (.callGeneric i)) {fi : FunctionInstantiation}
    (hfi : info.module.functionInstantiations[i]? = some fi) {fh : FunctionHandle}
    (hfh : info.module.functionHandles[fi.handle]? = some fh) {off : Nat}
    (hoff : ReferenceSafety.borrowFieldMutable fh.attributes = some off) {x : Reference}
    {st : List Value} (hst : fr.stack = .ref x :: st) (hxm : x.mutable = true)
    {cinfo : FunctionInfo} (hwc : WrapperCodeG cinfo off)
    {w fr1 : Frame} (hs'f : s'.frames = w :: fr1 :: rest) (hfr1 : fr1 = { fr with stack := st })
    (hwfn : P.function? w.function = some cinfo) (hwid : w.id = s.nextFrame)
    (hwca : w.countedActive = (cinfo.moduleId != info.moduleId))
    (hnext : s'.nextFrame = s.nextFrame + 1) (hstor : s'.storage = s.storage)
    (hheld : s'.held = s.held)
    (hactv : s'.active = (if w.countedActive then s.incActive cinfo.moduleId else s).active)
    (hlock : s'.lockDepth = (if w.lockEntered then s.lockDepth + 1 else s.lockDepth))
    (hinvs : s'.invalidations = s.invalidations) {n : Nat}
    (hloc : w.locals = ([Value.ref x].map some ++ List.replicate n none).toArray)
    (hpc : w.pc = 0) (hstk : w.stack = []) : ApiRunG P s' s := by
  subst hfr1
  refine ⟨hinv, fr, rest, info, code, i, fi, fh, off, x, st, w, cinfo, hsf, hact, hi, hfi, hfh,
    hoff, hst, hxm, hwc, hs'f, hwfn, hwid, hwca, hnext, hstor, hheld, hactv, hlock,
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
/-- **A generic struct-API mutable borrow call** from a state keeping the
reference invariant is safe, and starts a generic wrapper in progress. -/
theorem apiRunG_call (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hN : NativesRespect P N) {s : State} (hinv : InvRefs P s) {fr : Frame} {rest : List Frame}
    (hsf : s.frames = fr :: rest) {info : FunctionInfo} {code : CodeUnit}
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr info code)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr.function rest) {i : Nat}
    (hi : code.code[fr.pc]? = some (.callGeneric i)) {fi : FunctionInstantiation}
    (hfi : info.module.functionInstantiations[i]? = some fi) {fh : FunctionHandle}
    (hfh : info.module.functionHandles[fi.handle]? = some fh) {off : Nat}
    (hoff : ReferenceSafety.borrowFieldMutable fh.attributes = some off) :
    ExecSafe P N (InvAllG P) (Step.exec P s fr rest info code (.callGeneric i)) := by
  have hnm : NoModeledCall P fr.function.module info.module (.callGeneric i) := by
    intro x c hx hres
    rw [hfi, Option.some.injEq] at hx
    subst hx
    cases hcf : P.function? c with
    | none => simp [Program.nativeShape?, hcf]
    | some cinfo =>
      obtain ⟨ccode, -, hccode, -⟩ := (apiCall_wrapperG hwf hapi hact.fn hfh hoff hres hcf).code
      exact nativeShape?_of_code hcf hccode
  have hcore : ExecSafe P N (Inv P (modelTypes P) (modelAvail P))
      (Step.exec P s fr rest info code (.callGeneric i)) :=
    execGeneric_safe hc hN hact hchain hi rfl hnm
  suffices hruns : ExecRuns P N
      (fun s => RefInvariant P s ∨ (∃ sc, ApiRun P s sc) ∨ ∃ sc, ApiRunG P s sc)
      (Step.exec P s fr rest info code (.callGeneric i)) from ExecSafe.and hcore hruns
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
    | (refine Or.inr (Or.inr ⟨s, apiRunG_of_call hinv hsf hact hi hfi hfh hoff hst hxm hwc rfl ?_
        hcf' ?_ rfl ?_ ?_ ?_ ?_ ?_ ?_ (n := ltys.length - 1) ?_ rfl rfl⟩)
       all_goals first
         | rfl
         | (simp [hst]; done)
         | simp_all [State.incActive, State.activeCount])

end Call

section Step

variable {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
  (hs : P.Supported)
  (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
    ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)

include hwf hs hapi in
/-- **The wrapper's `MoveLoc(0)` and borrow** are safe and keep the wrapper in
progress: the borrow derives the field reference from the argument, which the
caller tracked at the call. -/
theorem apiRunG_step (hc : Certified P (modelTypes P) (modelAvail P)) {N : NativeRel}
    (hN : NativesRespect P N) {s sc : State} (hrun : ApiRunG P s sc) {w : Frame}
    {rest' : List Frame} (hsf : s.frames = w :: rest') {winfo : FunctionInfo}
    {wcode : CodeUnit} (hact : ActiveAt P (modelTypes P) (modelAvail P) w winfo wcode)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) w.function rest') {instr : Bytecode}
    (hi : wcode.code[w.pc]? = some instr) (hne : instr ≠ .ret) :
    ExecSafe P N (InvAllG P) (Step.exec P s w rest' winfo wcode instr) := by
  obtain ⟨fr, rest, info, code, i0, fi0, fh, off, x, st, w0, cinfo, hsc, hact0, hi0, hfi0, hfh,
    hoff, hst, hxm0, hwc, hsf0, hwfn, hwid, hwca, hnext, hstor, hheld, hactv, hlock,
    ⟨es, hes, hesw⟩, hlocs, hpcs⟩ := hrun.call
  rw [hsf, List.cons.injEq] at hsf0
  obtain ⟨hw, hrest⟩ := hsf0
  subst hw
  subst hrest
  have e1 := hact.fn
  rw [hwfn, Option.some.injEq] at e1
  subst e1
  obtain ⟨ccode, b, hccode, hcl, hb⟩ := hwc.code
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
    refine Or.inr (Or.inr ⟨sc, hrun.inv, fr, rest, info, code, i0, fi0, fh, off, x, st, _, cinfo,
      hsc, hact0, hi0, hfi0, hfh, hoff, hst, hxm0, hwc, rfl, hwfn, hwid, hwca, hnext, hstor,
      hheld, hactv, hlock, ⟨.removed (.local_ w.id 0) :: es, by simp [State.invalidate, hes],
        fun e he => ?_⟩,
      fun j v hj => ?_, Or.inr (Or.inl ⟨by simp [Step.advance, Step.push, hpc], ?_, by
        simp [Step.advance, Step.push, hstk]⟩)⟩)
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
    obtain ⟨fi', hfi', hx0⟩ := exec_callGeneric' hx0
    rw [hfi0, Option.some.injEq] at hfi'
    subst hfi'
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
    rcases hb with ⟨fhi, fhd, rfl, hfhd, hfoff⟩ | ⟨vh, vfhd, rfl, hvfhd, hvoff⟩ |
      ⟨gi, gfi, fhd, rfl, hgfi, hfhd, hfoff⟩ | ⟨gi, gfi, vfhd, rfl, hgfi, hvfhd, hvoff⟩
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
        (fun _ => rfl) hbefore hafter rfl rfl rfl ?_, Or.inr (Or.inr ⟨sc, hrun.inv, fr, rest, info, code, i0,
          fi0, fh, off, x, st, _, cinfo, hsc, hact0, hi0, hfi0, hfh, hoff, hst, hxm0, hwc,
          rfl, hwfn, hwid, hwca, hnext, hstor, hheld, hactv, hlock, ⟨es, hes, hesw⟩, hlocs,
          Or.inr (Or.inr ⟨by simp [Step.advance, Step.pushAll, w₁, hpc], hl0, rn, .field fhd.field,
            rfl, rfl, rfl,
            by simp [absPath, hfoff], ?_⟩)⟩)⟩
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
          (fun _ => rfl) hbefore hafter rfl rfl rfl ?_, Or.inr (Or.inr ⟨sc, hrun.inv, fr, rest, info, code,
            i0, fi0, fh, off, x, st, _, cinfo, hsc, hact0, hi0, hfi0, hfh, hoff, hst, hxm0, hwc,
            rfl, hwfn, hwid, hwca, hnext, hstor, hheld, hactv, hlock, ⟨es, hes, hesw⟩, hlocs,
            Or.inr (Or.inr ⟨by simp [Step.advance, Step.pushAll, w₁, hpc], hl0, rn,
              .variantField tag vfhd.field, rfl, rfl, rfl, by simp [absPath, hvoff], ?_⟩)⟩)⟩
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
    · -- `MutBorrowFieldGeneric`.
      obtain ⟨gx, fh', targs, sid, ftys, t, m', brest, hgx, hfh', hinst, hftys, ht, hbef, hm',
        hafter'⟩ := typeAfter_fieldBorrowGeneric (mut_ := true) hafter
      rw [hgfi, Option.some.injEq] at hgx
      subst hgx
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
          (.mutBorrowFieldGeneric gi) = .ok (.done (.running { s₁ with
            frames := Step.advance (Step.pushAll w₁ [.ref rn]) :: { fr with stack := st } :: rest })) := by
        simp only [Step.exec, Step.need, hgfi, hinst.genericArgs, Step.borrowField, hfhd,
          hinst.localStruct hsinfo, hfi,
          hftys, ht, bind, Except.bind, pure, Except.pure, hpop, hder, Step.finish, Step.borrow, rn,
          s₁, w₁]
      rw [hstep]
      have htv : t.isValue = true :=
        fieldTypes_values (fieldTypes_args hftys) hftys t (List.mem_of_getElem? ht)
      refine ⟨fallThrough_safe (N := N) (after := .ref true t :: brest) hc hact hchain hi rfl
        (fun _ => rfl) hbefore hafter rfl rfl rfl ?_, Or.inr (Or.inr ⟨sc, hrun.inv, fr, rest, info, code, i0,
          fi0, fh, off, x, st, _, cinfo, hsc, hact0, hi0, hfi0, hfh, hoff, hst, hxm0, hwc,
          rfl, hwfn, hwid, hwca, hnext, hstor, hheld, hactv, hlock, ⟨es, hes, hesw⟩, hlocs,
          Or.inr (Or.inr ⟨by simp [Step.advance, Step.pushAll, w₁, hpc], hl0, rn, .field fhd.field,
            rfl, rfl, rfl,
            by simp [absPath, hfoff], ?_⟩)⟩)⟩
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
    · -- `MutBorrowVariantFieldGeneric`.
      obtain ⟨gx, vfh', targs, sid, t, m', brest, hgx, hvh, hinst, hvt, hbef, hm', hafter'⟩ :=
        typeAfter_variantBorrowGeneric (mut_ := true) hafter
      rw [hgfi, Option.some.injEq] at hgx
      subst hgx
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
            (.mutBorrowVariantFieldGeneric gi) = .error (.failed .variantMismatch) := by
          simp only [Step.exec, need_some, hgfi, ok_bind', hinst.genericArgs,
            Step.borrowVariantField, hvfhd, hinst.localStruct hsinfo, hfi, hmap, hpop, hder, hfind, Step.fail]
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
            (.mutBorrowVariantFieldGeneric gi) = .ok (.done (.running { s₁ with
              frames := (Step.advance (Step.pushAll w₁ [.ref rn])) :: { fr with stack := st } ::
                rest })) := by
          simp only [Step.exec, need_some, hgfi, ok_bind', hinst.genericArgs,
            Step.borrowVariantField, hvfhd, hinst.localStruct hsinfo, hfi, hmap, hpop, hder, hfind, Step.finish]
          rfl
        rw [hstep]
        have htv : t.isValue = true :=
          variantFieldTypes_values (variantFieldTypes_args hfst) hfst t
            (List.mem_of_getElem? hftag)
        refine ⟨fallThrough_safe (N := N) (after := .ref true t :: brest) hc hact hchain hi rfl
          (fun _ => rfl) hbefore hafter rfl rfl rfl ?_, Or.inr (Or.inr ⟨sc, hrun.inv, fr, rest, info, code,
            i0, fi0, fh, off, x, st, _, cinfo, hsc, hact0, hi0, hfi0, hfh, hoff, hst, hxm0, hwc,
            rfl, hwfn, hwid, hwca, hnext, hstor, hheld, hactv, hlock, ⟨es, hes, hesw⟩, hlocs,
            Or.inr (Or.inr ⟨by simp [Step.advance, Step.pushAll, w₁, hpc], hl0, rn,
              .variantField tag vfhd.field, rfl, rfl, rfl, by simp [absPath, hvoff], ?_⟩)⟩)⟩
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

section Return

variable {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.Supported)
  (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
    ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)
  (hc : Certified P (modelTypes P) (modelAvail P))
include hwf hs hapi hc

omit hc in
/-- **The caller resumes**: after the wrapper's `Ret`, the reference
invariant holds again. -/
theorem apiRetG_inv {s sc s' : State} (hrun : ApiRunG P s sc) {w caller : Frame}
    {rest' : List Frame} (hsf : s.frames = w :: caller :: rest') (hpc : w.pc = 2)
    (hs'f : s'.frames =
      { caller with stack := w.stack ++ caller.stack, pc := caller.pc + 1 } :: rest')
    (hinv' : s'.invalidations = s.invalidations) (hstor' : s'.storage = s.storage)
    (hheld' : s'.held = s.held) (hnext' : s'.nextFrame = s.nextFrame)
    (hcount : ∀ M, s'.activeCount M = sc.activeCount M) : RefInvariant P s' := by
  obtain ⟨fr, rest, info, code, i0, fi0, fh, off, x, st, w₀, cinfo, hscf, hactC, hiC, hfi0, hfh,
    hoff, hstk, hxm, hwc, hsf₀, hwfn, hwid, -, hnext, hstor, hheld, -, -, ⟨es, hes, hesl⟩, -,
    hpcs⟩ := hrun.call
  rw [hsf₀, List.cons.injEq, List.cons.injEq] at hsf
  obtain ⟨rfl, rfl, rfl⟩ := hsf
  rcases hpcs with ⟨h0, -⟩ | ⟨h1, -⟩ | ⟨-, -, y, step, hwst, hym, hyloc, hstep, hyok⟩
  · rw [hpc] at h0; cases h0
  · rw [hpc] at h1; cases h1
  have hinvSc := hrun.inv
  have hr := hinvSc.2
  obtain ⟨view, A, stys, tys, ps, hctx⟩ :=
    topCtxGen hwf hs (apiSigs_of hwf hapi) hinvSc hscf hactC hiC
  obtain ⟨A', hx, hsucc⟩ := hctx.next (by intro e; cases e)
  obtain ⟨fi', hfi', hx⟩ := exec_callGeneric' hx
  rw [hfi0, Option.some.injEq] at hfi'
  subst hfi'
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
theorem apiRunG_ret {N : NativeRel} (hN : NativesRespect P N) {s sc : State}
    (hrun : ApiRunG P s sc) {fr0 : Frame} {rest0 : List Frame} (hsf : s.frames = fr0 :: rest0)
    {info0 : FunctionInfo} {code0 : CodeUnit}
    (hact : ActiveAt P (modelTypes P) (modelAvail P) fr0 info0 code0)
    (hchain : ChainOk P (modelTypes P) (modelAvail P) fr0.function rest0)
    (hi : code0.code[fr0.pc]? = some .ret) :
    ExecSafe P N (InvAllG P) (Step.exec P s fr0 rest0 info0 code0 .ret) := by
  have hcore := exec_safe (s := s) hc hN hact hchain hi rfl
    (fun h => by rcases h with h | h <;> cases h) trivial
  refine ExecSafe.weaken (fun s' h => ⟨h.1, Or.inl h.2⟩) (ExecSafe.and hcore ?_)
  obtain ⟨fr, rest, info, code, i0, fi0, fh, off, x, st, w, cinfo, -, -, -, -, -, -, -, -, hwc,
    hsf₀, hwfn, -, -, -, -, -, hactv, -, -, -, -⟩ := hrun.call
  have hsf₁ := hsf₀
  rw [hsf, List.cons.injEq] at hsf₁
  obtain ⟨rfl, rfl⟩ := hsf₁
  -- The wrapper's code, at its `Ret`.
  have hinfo : info0 = cinfo := by
    have := hact.fn; rw [hwfn, Option.some.injEq] at this; exact this.symm
  subst hinfo
  obtain ⟨codeW, b, hcodeW, hlist, hb⟩ := hwc.code
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
      rcases hb with ⟨_, _, rfl, _⟩ | ⟨_, _, rfl, _⟩ | ⟨_, _, _, rfl, _⟩ | ⟨_, _, _, rfl, _⟩ <;>
        cases hi
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
      exact apiRetG_inv hwf hs hapi hrun hsf₀ hpc rfl rfl rfl rfl rfl
        (api_count hactv (by rw [hca]; rfl))

end Return

end MoveBytecode.Proofs
