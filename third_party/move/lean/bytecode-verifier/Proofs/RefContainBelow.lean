-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefContainTop

/-!
# Containment for the suspended frames

UNTRUSTED (checked by the kernel). No access of a step of a verified program
reaches over a `&mut` reference that a suspended frame tracks
(`contained_below`). The frames from a callee up access, through references,
only their own locals, locations inside the references the callee received,
and global storage of types the frames below do not protect (`MayLoc`); and
directly, only their own locals and global storage (`MayRoot`). One frame
down, that is within what the caller may access (`cb_lift`), and it reaches
over none of the caller's references (`cb_exclude`): the call left none of
them strictly inside a mutable argument, nothing mutable is strictly inside
an immutable one, the callee's locals are newer, and the callee protects the
global types the caller's references are in. A resource the current frame
moves out is of a type it acquires in a module active once, so no frame below
protects it; one it publishes did not exist, so no valid reference is into
it.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph MoveBytecode.Step

/-! ## What the frames from a callee up may access -/

/-- The roots the frames from `cid` up may access directly: their own locals,
and global storage of a type the frames below do not protect (`Sc`). -/
private def MayRoot (cid : FrameId) (Sc : StructId → Prop) (ρ : Root) : Prop :=
  (∃ f i, ρ = .local_ f i ∧ cid ≤ f) ∨ ∃ a t, ρ = .global a t ∧ ¬ Sc t

/-- The locations the frames from `cid` up may access through a reference:
those at the roots they may access, and those inside a reference the frame
`cid` received (`cps`). -/
private def MayLoc (cid : FrameId) (cps : List Reference) (Sc : StructId → Prop)
    (l : Location) : Prop :=
  MayRoot cid Sc l.root ∨ ∃ π ∈ cps, l.Inside π.location

/-- A reference a frame holds where the analysis tracks it. -/
private def Kept (P : Program) (fr : Frame) (r : Reference) : Prop :=
  Value.ref r ∈ fr.stack ∨ ∃ j, fr.locals[j]? = some (some (.ref r)) ∧
    modelAvail P fr.function fr.pc j = .available

/-- One frame down, the roots stay within what the caller may access. -/
private theorem cb_liftRoot {P : Program} {s : State} {caller : Frame} {rest : List Frame}
    {ps : List Reference} {S : StructId → Prop} {cid : FrameId} {cfn : FunctionRef}
    {cps : List Reference} {Sc : StructId → Prop}
    (h : SuspOk P s caller rest ps S cid cfn cps Sc) {ρ : Root} (hρ : MayRoot cid Sc ρ) :
    MayRoot caller.id S ρ := by
  rcases hρ with ⟨f, i, hr, hf⟩ | ⟨a, t, hr, hn⟩
  · exact Or.inl ⟨f, i, hr, Nat.le_of_lt (Nat.lt_of_lt_of_le h.order hf)⟩
  · exact Or.inr ⟨a, t, hr, fun hS => hn (h.sub t hS)⟩

/-- **One frame down**, the locations stay within what the caller may access:
a reference the callee received was an argument, into a local of the caller,
global storage it borrowed, or a reference it received. -/
private theorem cb_lift {P : Program} {s : State} {caller : Frame} {rest : List Frame}
    {ps : List Reference} {S : StructId → Prop} {cid : FrameId} {cfn : FunctionRef}
    {cps : List Reference} {Sc : StructId → Prop}
    (h : SuspOk P s caller rest ps S cid cfn cps Sc) {l : Location} (hl : MayLoc cid cps Sc l) :
    MayLoc caller.id ps S l := by
  rcases hl with hρ | ⟨π, hπ, hin⟩
  · exact Or.inl (cb_liftRoot h hρ)
  obtain ⟨m, rs, rets, args, A, A', A'', L, hf⟩ := h.call
  obtain ⟨x, hxs, hxL⟩ := hf.cpsArgs π (hf.cpsSub.subset hπ)
  have hxH : x ∈ heldIds A := mem_heldIds.2 (Or.inr (List.mem_of_mem_take hxs))
  rcases hf.desc.complete.prov3 hxH hxL with ⟨i, hi⟩ | ⟨a, d, t, hd, hr⟩ | ⟨π', hπ', hin', -⟩
  · exact Or.inl (Or.inl ⟨caller.id, i, by rw [hin.1, hi], Nat.le_refl _⟩)
  · refine Or.inl (Or.inr ⟨a, t, by rw [hin.1, hr], fun hS => ?_⟩)
    obtain ⟨hacq, hcnt⟩ := gsidOf_cons (b := caller.countedActive) (sk := rest.map Frame.skel) hd
    have := h.glob.acq t hS hacq
    have e : (caller :: rest).map Frame.skel =
        (caller.function, caller.countedActive) :: rest.map Frame.skel := rfl
    rw [e] at this
    omega
  · exact Or.inr ⟨π', hπ', hin.trans hin'⟩

/-- **A suspended caller's tracked references** are valid, not at a root the
frames above may access, and, when mutable, not strictly inside a location
they may access through a reference. -/
private theorem cb_exclude {P : Program} {s : State} {caller : Frame} {rest : List Frame}
    {ps : List Reference} {S : StructId → Prop} {cid : FrameId} {cfn : FunctionRef}
    {cps : List Reference} {Sc : StructId → Prop}
    (h : SuspOk P s caller rest ps S cid cfn cps Sc) {r : Reference} (hk : Kept P caller r) :
    RefOk P s caller r ∧ (∀ ρ, MayRoot cid Sc ρ → r.location.root ≠ ρ) ∧
      (r.mutable = true → ∀ l, MayLoc cid cps Sc l → ¬ r.location.StrictlyInside l) := by
  obtain ⟨m, rs, rets, args, A, A', A'', L, hf⟩ := h.call
  obtain ⟨x, hx, hxL⟩ := hf.tracked r hk
  obtain ⟨hok, hsh⟩ := hf.refs x r hx hxL
  have hxH : x ∈ heldIds A := heldIds_drop hx
  have hroot : ∀ ρ, MayRoot cid Sc ρ → r.location.root ≠ ρ := by
    rintro ρ (⟨f, i, rfl, hf'⟩ | ⟨a, t, rfl, hn⟩) e
    · have hb := hok.below
      rw [e] at hb
      exact Nat.lt_irrefl _ (Nat.lt_of_lt_of_le h.order (Nat.le_trans hf' hb))
    · exact hn (hf.globals x r hxH hxL a t e)
  refine ⟨hok, hroot, fun hm l hl hin => ?_⟩
  rcases hl with hρ | ⟨π, hπ, hinπ⟩
  · exact hroot _ hρ hin.1.1
  · have hin' : r.location.StrictlyInside π.location := StrictlyInside.trans_inside hin hinπ
    cases hπm : π.mutable with
    | true => exact hsh π hπ hπm hin'
    | false =>
      obtain ⟨y, hys, hyL⟩ := hf.cpsArgs π (hf.cpsSub.subset hπ)
      have hyH : y ∈ heldIds A := mem_heldIds.2 (Or.inr (List.mem_of_mem_take hys))
      have := hf.desc.complete.immInside y π x r hyH hxH hyL hxL hπm hin'
      rw [hm] at this
      cases this

/-- **Down the chain of suspended frames**: what the frames from `cid` up may
access reaches over none of the tracked references of the frames below. -/
private theorem cb_chain {P : Program} {s : State} :
    ∀ (frs : List Frame) (cid : FrameId) (cfn : FunctionRef) (cps : List Reference)
      (Sc : StructId → Prop), ChainRef P s cid cfn cps Sc frs → ∀ fr' ∈ frs, ∀ r,
      Kept P fr' r → RefOk P s fr' r ∧ (∀ ρ, MayRoot cid Sc ρ → r.location.root ≠ ρ) ∧
        (r.mutable = true → ∀ l, MayLoc cid cps Sc l → ¬ r.location.StrictlyInside l)
  | [], _, _, _, _, _, fr', hfr', _, _ => by simp at hfr'
  | caller :: frs, cid, cfn, cps, Sc, ⟨ps, S, hsusp, hrest⟩, fr', hfr', r, hk => by
    rcases List.mem_cons.1 hfr' with rfl | hfr'
    · exact cb_exclude hsusp hk
    · obtain ⟨hok, hroot, hin⟩ :=
        cb_chain frs caller.id caller.function ps S hrest fr' hfr' r hk
      exact ⟨hok, fun ρ hρ => hroot ρ (cb_liftRoot hsusp hρ),
        fun hm l hl => hin hm l (cb_lift hsusp hl)⟩


/-! ## The accesses of a step -/

/-- The step of the current frame's instruction. -/
private theorem cb_step {P : Program} {s : State} {fr : Frame} {rest : List Frame}
    {info : FunctionInfo} {code : CodeUnit} {instr : Bytecode} (hsf : s.frames = fr :: rest)
    (hinfo : P.function? fr.function = some info) (hcode : info.definition.code = some code)
    (hi : code.code[fr.pc]? = some instr) :
    step P s = stepOf (Step.exec P s fr rest info code instr) := by
  unfold step
  rw [hsf]
  simp only [hinfo, hcode, hi]
  rfl

/-- What `MoveFrom`'s write of a resource is. -/
private theorem cb_ev_moveFrom {P : Program} {m : ModuleId} {s : State} {fr : Frame}
    {o : Outcome} {h : Shadow} {post : ShadowPost} {evs : List Event} {g : ShadowFrame}
    {hs : List ShadowFrame} {d : StructDefinitionIndex} (hh : h.frames = g :: hs)
    (hex : Shadow.exec P m (.moveFrom d) s fr o h = some (post, evs)) {addr : Address}
    {sid : StructId} (ha : Event.accessed .write ⟨.global addr sid, []⟩ none ∈ evs) :
    fr.stack.head? = some (.address addr) ∧
      ∃ v, Shadow.topAfter o = some v ∧ v.type = .struct sid := by
  simp only [Shadow.exec, hh, Option.bind_eq_bind, Option.pure_def] at hex
  split at hex
  · next a hhead =>
    rw [Option.bind_eq_some_iff] at hex
    obtain ⟨v, hv, hm⟩ := hex
    split at hm
    · next sid' htype =>
      simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨k, -, t, -, -, rfl⟩ := hm
      simp at ha
      obtain ⟨rfl, rfl⟩ := ha
      exact ⟨hhead, v, hv, htype⟩
    · cases hm
  · cases hex

/-- What `MoveTo`'s write of a resource is. -/
private theorem cb_ev_moveTo {P : Program} {m : ModuleId} {s : State} {fr : Frame}
    {o : Outcome} {h : Shadow} {post : ShadowPost} {evs : List Event} {g : ShadowFrame}
    {hs : List ShadowFrame} {d : StructDefinitionIndex} (hh : h.frames = g :: hs)
    (hex : Shadow.exec P m (.moveTo d) s fr o h = some (post, evs)) {addr : Address}
    {sid : StructId} (ha : Event.accessed .write ⟨.global addr sid, []⟩ none ∈ evs) :
    (∃ v, fr.stack.head? = some v ∧ v.type = .struct sid) ∧
      ∃ rs, fr.stack[1]? = some (.ref rs) ∧ s.deref? rs.location rs.since = some (.signer addr) := by
  simp only [Shadow.exec, hh, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def] at hex
  obtain ⟨tv, -, v, hhead, hm⟩ := hex
  split at hm
  · next sid' htype =>
    rw [Option.bind_eq_some_iff] at hm
    obtain ⟨x, hra, hm⟩ := hm
    split at hm
    · next a₂ hd =>
      simp only [Option.some.injEq, Prod.mk.injEq] at hm
      obtain ⟨-, rfl⟩ := hm
      simp at ha
      obtain ⟨rfl, rfl⟩ := ha
      exact ⟨⟨v, hhead, htype⟩, x.2, (refAt_spec hra).1, hd⟩
    · cases hm
  · cases hm


/-! ## The resources a step writes directly -/

/-- A resource holds a value of its type. -/
private theorem cb_get?_type {st : GlobalStorage}
    (ht : ∀ r ∈ st, r.value.type = .struct r.type) {a : Address} {sid : StructId} {v : Value}
    (h : st.get? a sid = some v) : v.type = .struct sid := by
  unfold GlobalStorage.get? at h
  rw [Option.map_eq_some_iff] at h
  obtain ⟨r, hr, rfl⟩ := h
  have hp := List.find?_some hr
  simp only [Bool.and_eq_true, beq_iff_eq] at hp
  rw [ht r (List.mem_of_find?_eq_some hr), hp.2]

/-- `MoveFrom` takes a resource of the type its operand names, under the
address on top of the operand stack, with the type's module active at most
once. -/
private theorem cb_moveFrom {P : Program} {s : State} {fr : Frame} {rest : List Frame}
    {info : FunctionInfo} {code : CodeUnit} {d : StructDefinitionIndex} {o : Outcome}
    (h : stepOf (Step.exec P s fr rest info code (.moveFrom d)) = .done o)
    (hcont : o.continues = true) :
    ∃ sid a v, P.definedStruct? fr.function.module d = some sid ∧
      ¬ s.activeCount sid.module > 1 ∧ fr.stack.head? = some (.address a) ∧
      s.storage.get? a sid = some v ∧ Shadow.topAfter o = some v := by
  obtain ⟨sid, hsid, h⟩ := done_bind h hcont globalType_ends
  obtain ⟨⟨a, fr₁⟩, hpa, h⟩ := done_bind h hcont popAddress_ends
  obtain ⟨st, hstk, rfl⟩ := popAddress_ok hpa
  obtain ⟨_, hun, h⟩ := done_bind h hcont unlocked_ends
  have hnl : ¬ s.activeCount sid.module > 1 := by
    intro hgt
    simp [unlocked, hgt, fail] at hun
  split at h
  · next v hget =>
    have ho := done_finish rfl h
    subst ho
    exact ⟨sid, a, v, definedStruct?_nil (globalType_ok hsid), hnl, by simp [hstk], hget,
      by simp [Shadow.topAfter, advance, pushAll]⟩
  · exact (done_fail h hcont).elim

/-- `MoveTo` publishes a value of the type its operand names under the
address of the signer it is given a reference to, where no resource of the
type is. -/
private theorem cb_moveTo {P : Program} {s : State} {fr : Frame} {rest : List Frame}
    {info : FunctionInfo} {code : CodeUnit} {d : StructDefinitionIndex} {o : Outcome}
    (hsf : s.frames = fr :: rest)
    (h : stepOf (Step.exec P s fr rest info code (.moveTo d)) = .done o)
    (hcont : o.continues = true) :
    ∃ sid v rs a, fr.stack[0]? = some v ∧ v.type = .struct sid ∧ fr.stack[1]? = some (.ref rs) ∧
      s.deref? rs.location rs.since = some (.signer a) ∧ s.storage.get? a sid = none := by
  obtain ⟨sid, -, h⟩ := done_bind h hcont globalType_ends
  obtain ⟨⟨v, fr₁⟩, hpv, h⟩ := done_bind h hcont pop_ends
  obtain ⟨st₁, hst₁, rfl⟩ := pop_ok hpv
  obtain ⟨⟨r, fr₂⟩, hpr, h⟩ := done_bind h hcont popRefTo_ends
  obtain ⟨st, hst₂, rfl, -, -⟩ := popRefTo_ok hpr
  simp only at hst₂
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  split at h
  · exact (done_wrong h hcont).elim
  next hconf =>
  simp only [Bool.not_eq_true', Bool.not_eq_false] at hconf
  split at h
  · exact (done_wrong h hcont).elim
  obtain ⟨sv, hsv, h⟩ := done_bind h hcont deref_ends
  split at h
  · next a =>
    obtain ⟨_, -, h⟩ := done_bind h hcont unlocked_ends
    split at h
    · exact (done_fail h hcont).elim
    · next hnone =>
      have hget : s.storage.get? a sid = none := by simpa using hnone
      have hstk : fr.stack = v :: .ref r :: st := by rw [hst₁, hst₂]
      have hd := deref_ok hsv
      rw [deref_eq (s' := { s with frames := { fr with stack := st } :: rest }) (s := s)
        (by simp [hsf]) rfl rfl rfl] at hd
      exact ⟨sid, v, r, a, by simp [hstk], conforms_type' hconf rfl, by simp [hstk], hd, hget⟩
  · exact (done_wrong h hcont).elim

/-! ## The tracked references are valid -/

/-- **A tracked reference is one the analysis tracks**, and so valid and of
its type (`RefOk`): through the current frame's analysis state, or through a
suspended frame's call. -/
theorem tracked_refOk {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.SupportedNonGeneric) (hm3 : P.M3) {s : State} (hinv : InvRefs P s) {h : Shadow}
    {i : Nat} {r : Reference} (ht : Tracked P s h i r) :
    ∃ fr ∈ s.frames, RefOk P s fr r := by
  obtain ⟨g', fr', hz, ht⟩ := ht
  obtain ⟨fr, rest, hsf, ⟨info, code, hact⟩, -⟩ := hinv.1
  have hfr' : fr' ∈ s.frames := (List.of_mem_zip hz).2
  cases hfr : h.frames with
  | nil => rw [hfr] at hz; cases hz
  | cons g hs' =>
    rw [hfr, hsf, List.zip_cons_cons, List.mem_cons] at hz
    obtain ⟨instr, hi⟩ : ∃ instr, code.code[fr.pc]? = some instr :=
      ⟨_, Array.getElem?_eq_getElem hact.pcLt⟩
    obtain ⟨view, A, stys, tys, ps, hctx⟩ := topCtx hwf hs hm3 hinv hsf hact hi
    rcases hz with he | hz
    · simp only [Prod.mk.injEq] at he
      obtain ⟨rfl, rfl⟩ := he
      obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
      obtain ⟨y, hy, hyL⟩ := tracked_top_held hctx hact hd ht
      exact ⟨fr', hfr', hrefs y r hy hyL⟩
    · have hk : Kept P fr' r := by
        rcases ht with ⟨t, -, hz'⟩ | ⟨j, ⟨t, -, -, hl⟩, hav⟩
        · exact Or.inl (List.of_mem_zip hz').2
        · exact Or.inr ⟨j, hl, hav⟩
      obtain ⟨S, -, hchain⟩ := hctx.chain rest hsf
      exact ⟨fr', hfr', (cb_chain rest fr.id fr.function ps S hchain fr' (List.of_mem_zip hz).2 r
        hk).1⟩

/-- **A reference a frame keeps** — on its operand stack, or in a local the
availability analysis has available — **is valid and of its type**, in a state
with the reference invariant of a program whose struct-API mutable borrows
have their signatures. -/
theorem kept_refOk {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.Supported) (hsigs : P.ApiSigs) {s : State} (hinv : InvRefs P s) {fr' : Frame}
    (hfr' : fr' ∈ s.frames) {r : Reference}
    (hk : Value.ref r ∈ fr'.stack ∨ ∃ j, fr'.locals[j]? = some (some (.ref r)) ∧
      modelAvail P fr'.function fr'.pc j = .available) :
    RefOk P s fr' r := by
  obtain ⟨fr, rest, hsf, ⟨info, code, hact⟩, -⟩ := hinv.1
  obtain ⟨instr, hi⟩ : ∃ instr, code.code[fr.pc]? = some instr :=
    ⟨_, Array.getElem?_eq_getElem hact.pcLt⟩
  obtain ⟨view, A, stys, tys, ps, hctx⟩ := topCtxGen hwf hs hsigs hinv hsf hact hi
  rw [hsf] at hfr'
  rcases List.mem_cons.1 hfr' with he | hfr'
  · subst he
    obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
    rcases hk with hst | ⟨j, hj, hav⟩
    · obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hst
      obtain ⟨x, hx', hxL⟩ := abs_of_ref hd hctx.typed hctx.targs hctx.sig hctx.conf hctx.height hk
      exact hrefs x r (mem_heldIds.2 (Or.inr (List.mem_of_getElem? hx'))) hxL
    · obtain ⟨t', ht', hkind⟩ := localToken_ref hact.fn hact.body hctx.view hact.locals (typeArgsOk_values hact.typeArgs) hj
      obtain ⟨x, hx'⟩ := hctx.typed.locKind j t' r.mutable ht' hkind hav
      obtain ⟨r', hr', hxL⟩ := hd.locals j x hx'
      rw [hj, Option.some.injEq, Option.some.injEq, Value.ref.injEq] at hr'
      subst hr'
      exact hrefs x r (mem_heldIds.2 (Or.inl (List.mem_of_getElem? hx'))) hxL
  · obtain ⟨S, -, hchain⟩ := hctx.chain rest hsf
    exact (cb_chain rest fr.id fr.function ps S hchain fr' hfr' r hk).1

/-! ## Containment for the suspended frames -/

/-- **A reference a frame keeps where the analysis tracks it is valid**: on
its operand stack, or in a local the availability analysis has available
(`P.ApiSigs` in place of the M3 premise). -/
theorem kept_refOk_api {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.Supported) (hsigs : P.ApiSigs) {s : State} (hinv : InvRefs P s) {fr' : Frame}
    (hfr' : fr' ∈ s.frames) {r : Reference}
    (hk : Value.ref r ∈ fr'.stack ∨ ∃ j, fr'.locals[j]? = some (some (.ref r)) ∧
      modelAvail P fr'.function fr'.pc j = .available) :
    RefOk P s fr' r := by
  obtain ⟨fr, rest, hsf, ⟨info, code, hact⟩, -⟩ := hinv.1
  obtain ⟨instr, hi⟩ : ∃ instr, code.code[fr.pc]? = some instr :=
    ⟨_, Array.getElem?_eq_getElem hact.pcLt⟩
  obtain ⟨view, A, stys, tys, ps, hctx⟩ := topCtxGen hwf hs hsigs hinv hsf hact hi
  rw [hsf, List.mem_cons] at hfr'
  rcases hfr' with rfl | hfr'
  · obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
    rcases hk with hk | ⟨j, hj, hav⟩
    · obtain ⟨k, hk⟩ := List.mem_iff_getElem?.1 hk
      obtain ⟨x, hx', hxL⟩ := abs_of_ref hd hctx.typed hctx.targs hctx.sig hctx.conf hctx.height hk
      exact hrefs x r (mem_heldIds.2 (Or.inr (List.mem_of_getElem? hx'))) hxL
    · obtain ⟨t', ht', hkind⟩ := localToken_ref hact.fn hact.body hctx.view hact.locals (typeArgsOk_values hact.typeArgs) hj
      obtain ⟨x, hx'⟩ := hctx.typed.locKind j t' r.mutable ht' hkind hav
      obtain ⟨r', hr', hxL⟩ := hd.locals j x hx'
      rw [hj, Option.some.injEq, Option.some.injEq, Value.ref.injEq] at hr'
      subst hr'
      exact hrefs x r (mem_heldIds.2 (Or.inl (List.mem_of_getElem? hx'))) hxL
  · obtain ⟨S, -, hchain⟩ := hctx.chain rest hsf
    exact (cb_chain rest fr.id fr.function ps S hchain fr' hfr' r hk).1

/-- `contained_below` for programs whose struct-API handles have their signatures
(`P.ApiSigs`). -/
theorem contained_below' {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.Supported) (hsigs : P.ApiSigs) {s : State} (hinv : InvRefs P s) {h : Shadow}
    (hpre : Pre false P s h) (_htyped : StateTyped P s) {o : Outcome} {post : ShadowPost}
    {evs : List Event} (hstep : step P s = .done o) (hc : Shadow.coreStep P s o h = some (post, evs))
    {g : ShadowFrame} {hs' : List ShadowFrame} {fr : Frame} {rest : List Frame}
    (hh : h.frames = g :: hs') (hsf : s.frames = fr :: rest)
    {a : Access} {l : Location} {via : Option Nat} (ha : Event.accessed a l via ∈ evs)
    {g' : ShadowFrame} {fr' : Frame} (hmem : (g', fr') ∈ hs'.zip rest)
    {i : Nat} {r : Reference} (ht : TrackedIn P g' fr' i r) (hm : r.mutable = true) :
    Access.reachesOver a l via r.location = false := by
  obtain ⟨fr₀, rest₀, info, code, instr, hsf₀, hinfo, hcode, hinstr, hex⟩ := coreStep_exec hc
  rw [hsf, List.cons.injEq] at hsf₀
  obtain ⟨rfl, rfl⟩ := hsf₀
  have hcont := coreStep_continues hc
  have hstep' := (cb_step hsf hinfo hcode hinstr).symm.trans hstep
  -- The suspended frame's call tracks the reference.
  have hk : Kept P fr' r := by
    rcases ht with ⟨t, -, hz⟩ | ⟨j, ⟨t, -, -, hl⟩, hav⟩
    · exact Or.inl (List.of_mem_zip hz).2
    · exact Or.inr ⟨j, hl, hav⟩
  -- The current frame against the certificates, and the frames below it.
  obtain ⟨fr₁, rest₁, hsf₁, ⟨info₁, code₁, hact⟩, -⟩ := hinv.1
  rw [hsf, List.cons.injEq] at hsf₁
  obtain ⟨rfl, rfl⟩ := hsf₁
  obtain rfl : info₁ = info := Option.some.inj (hact.fn.symm.trans hinfo)
  obtain rfl : code₁ = code := Option.some.inj (hact.body.symm.trans hcode)
  obtain ⟨view, A, stys, tys, ps, hctx⟩ := topCtxGen hwf hs hsigs hinv hsf hact hinstr
  obtain ⟨S, hglob, hchain⟩ := hctx.chain rest hsf
  obtain ⟨hok, hroot, hin⟩ :=
    cb_chain rest fr.id fr.function ps S hchain fr' (List.of_mem_zip hmem).2 r hk
  cases exec_accessed hh hex ha with
  | inspect => rfl
  | viaRef _ hkk _ _ =>
    -- The reference accessed through is the current frame's: inside its locals, global
    -- storage it borrowed, or a reference it received.
    obtain ⟨L, hd, -⟩ := hctx.frameOk
    obtain ⟨x, hx, hxL⟩ := abs_of_ref hd hctx.typed hctx.targs hctx.sig hctx.conf hctx.height hkk
    have hxH : x ∈ heldIds A := mem_heldIds.2 (Or.inr (List.mem_of_getElem? hx))
    have hsk : s.frames.map Frame.skel =
        (fr.function, fr.countedActive) :: rest.map Frame.skel := by rw [hsf]; rfl
    have hc0 := hd.complete
    rw [hsk] at hc0
    refine ct_reach_via (hin hm _ ?_)
    rcases hc0.prov3 hxH hxL with ⟨j, hj⟩ | ⟨a', d', t', hd', hr⟩ | ⟨π, hπ, hinπ, -⟩
    · exact Or.inl (Or.inl ⟨fr.id, j, hj, Nat.le_refl _⟩)
    · refine Or.inl (Or.inr ⟨a', t', hr, fun hS => ?_⟩)
      obtain ⟨hacq, hcnt⟩ := gsidOf_cons hd'
      have := hglob.acq t' hS hacq
      have e : (fr :: rest).map Frame.skel =
          (fr.function, fr.countedActive) :: rest.map Frame.skel := rfl
      rw [e] at this
      omega
    · exact Or.inr ⟨π, hπ, hinπ⟩
  | direct _ => exact ct_reach_direct (hroot _ (Or.inl ⟨fr.id, _, rfl, Nat.le_refl _⟩))
  | @global addr sid hop =>
    have hcase : ((∃ d, instr = .moveFrom d) ∨ ∃ i, instr = .moveFromGeneric i) ∨
        ((∃ d, instr = .moveTo d) ∨ ∃ i, instr = .moveToGeneric i) := by
      rcases hop with h | h | h | h
      · exact Or.inl (Or.inl h)
      · exact Or.inr (Or.inl h)
      · exact Or.inl (Or.inr h)
      · exact Or.inr (Or.inr h)
    rcases hcase with hmf | hmt
    · -- `MoveFrom`, generic or not: a type the function acquires, in a module active
      -- once, which no frame below protects.
      obtain ⟨a', v', sid', hhead', htop', hvt', hl⟩ := ct_moveFrom_loc hmf hh hex ha
      simp only [Location.mk.injEq, Root.global.injEq, and_true] at hl
      obtain ⟨rfl, rfl⟩ := hl
      obtain ⟨d, tys, hdcase, sid0, a0, v0, st, hsid, hstk, hnl, hget, htop0⟩ :=
        ct_moveFrom_step hmf hstep' hcont
      rw [hstk] at hhead'
      simp only [List.head?_cons, Option.some.injEq, Value.address.injEq] at hhead'
      subst hhead'
      rw [htop0, Option.some.injEq] at htop'
      subst htop'
      rw [cb_get?_type hpre.inv.typed hget, RuntimeType.struct.injEq] at hvt'
      subst hvt'
      obtain ⟨A', hx⟩ := hctx.exec
      obtain ⟨-, hacq⟩ := moveFrom_analysis hwf hinfo hcode hinstr hdcase hx
      have hacqT : acqTypes P fr.function sid0 :=
        ⟨d, by simp only [acqOf, hinfo, Option.map_some, Option.getD_some]; exact hacq,
          globalType_erase hsid⟩
      have hcount : countActive P ((fr :: rest).map Frame.skel) sid0.module ≤ 1 := by
        rw [← hsf, ← hinv.2.active.count]; omega
      have hnotS : ¬ S sid0 := fun hS => by have := hglob.acq sid0 hS hacqT; omega
      exact ct_reach_direct (hroot _ (Or.inr ⟨_, sid0, rfl, hnotS⟩))
    · -- `MoveTo`, generic or not: no resource was there, so no valid reference is into it.
      obtain ⟨v', sid', rs', a', hhead', hvt', hrs', hd', hl⟩ := ct_moveTo_loc hmt hh hex ha
      simp only [Location.mk.injEq, Root.global.injEq, and_true] at hl
      obtain ⟨rfl, rfl⟩ := hl
      obtain ⟨sid0, v0, rs0, st, a0, hstk, -, hvt0, hderef0, hget⟩ :=
        ct_moveTo_step hmt hsf hstep' hcont
      rw [hstk] at hhead' hrs'
      simp only [List.head?_cons, Option.some.injEq] at hhead'
      subst hhead'
      simp only [List.getElem?_cons_succ, List.getElem?_cons_zero, Option.some.injEq,
        Value.ref.injEq] at hrs'
      subst hrs'
      rw [hvt0, RuntimeType.struct.injEq] at hvt'
      subst hvt'
      rw [hderef0, Option.some.injEq, Value.signer.injEq] at hd'
      subst hd'
      refine ct_reach_direct fun e => ?_
      have hread := (deref?_isSome.1 hok.valid).2
      rw [Location.eta r.location, e] at hread
      simp [State.read?, State.root?, hget] at hread

/-- **No access of a step reaches over a tracked `&mut` reference of a
suspended frame**, in a verified program. -/
theorem contained_below {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.SupportedNonGeneric) (hm3 : P.M3) {s : State} (hinv : InvRefs P s) {h : Shadow}
    (hpre : Pre false P s h) (_htyped : StateTyped P s) {o : Outcome} {post : ShadowPost}
    {evs : List Event} (hstep : step P s = .done o) (hc : Shadow.coreStep P s o h = some (post, evs))
    {g : ShadowFrame} {hs' : List ShadowFrame} {fr : Frame} {rest : List Frame}
    (hh : h.frames = g :: hs') (hsf : s.frames = fr :: rest)
    {a : Access} {l : Location} {via : Option Nat} (ha : Event.accessed a l via ∈ evs)
    {g' : ShadowFrame} {fr' : Frame} (hmem : (g', fr') ∈ hs'.zip rest)
    {i : Nat} {r : Reference} (ht : TrackedIn P g' fr' i r) (hm : r.mutable = true) :
    Access.reachesOver a l via r.location = false :=
  contained_below' hwf (supported_of_nonGeneric hwf hs) hm3.apiSigs hinv hpre _htyped hstep hc hh hsf
    ha hmem ht hm

/-- **No access of a call of a modeled native reaches over a tracked `&mut`**
of any frame (D4b′): it reads and writes through its reference arguments,
which the analysis checked readable, or writable. -/
theorem contained_native {P : Program} {ν : NativeFlows}
    (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.Supported)
    (hsigs : P.ApiSigs)
    (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
      ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)
    {s s₁ : State} (hinv : InvRefs P s) {h : Shadow} (hpre : Pre false P s h) {f : FunctionRef}
    {args views vals writes : List Value} {places : List NativePlace} {o : Outcome}
    {post : ShadowPost} {evs : List Event} {k : NativeShape} (hshape : P.nativeShape? f = some k)
    (hstep : step P s = .callNative s₁ f args views)
    (hn : Shadow.nativeStep P ν s f args views vals writes places o h = some (post, evs))
    {a : Access} {l : Location} {via : Option Nat} (ha : Event.accessed a l via ∈ evs)
    {g' : ShadowFrame} {fr' : Frame} (hz : (g', fr') ∈ h.frames.zip s.frames)
    {i : Nat} {r : Reference} (ht : TrackedIn P g' fr' i r) (hm : r.mutable = true) :
    Access.reachesOver a l via r.location = false := by
  obtain ⟨g, hs', fr, rest, n, t, v, hh, hsf, hnl, hargs, -, hmem, hvt, hl, -⟩ :=
    native_access hpre hstep hn ha
  have hargl : args.length = n := by rw [hargs]; simp [Nat.min_eq_left hnl]
  obtain ⟨fr₀, rest₀, info, code, instr, view, A, stys, tys, ps, L, hsf₀, hact, hi, hctx, hd,
    hrefs, hargsA⟩ := nativeCall_analysis hwf hs hsigs hapi hinv hshape hstep
  rw [hsf, List.cons.injEq] at hsf₀
  obtain ⟨rfl, rfl⟩ := hsf₀
  obtain ⟨kk, x, hkk, hxk, hxH, hxL, hchk⟩ := hargsA v (by rw [hargl]; exact hmem)
  subst hvt hl
  apply ct_reach_via
  rw [hh, hsf, List.zip_cons_cons, List.mem_cons] at hz
  rcases hz with he | hz
  · -- A reference of the current frame.
    simp only [Prod.mk.injEq] at he
    obtain ⟨rfl, rfl⟩ := he
    obtain ⟨y, hy, hyL⟩ := tracked_top_held hctx hact hd ht
    rcases hchk with ⟨-, hrd⟩ | ⟨-, hwr⟩
    · exact hd.complete.readable_inside hxH hxL hrd hy hyL hm
    · exact hd.complete.writable_inside hxH hxL hwr hy hyL
  · -- A reference of a suspended frame: the current frame's reference argument is inside
    -- its locals, global storage it borrowed, or a reference it received.
    have hk : Kept P fr' r := by
      rcases ht with ⟨t', -, hz'⟩ | ⟨j, ⟨t', -, -, hl'⟩, hav⟩
      · exact Or.inl (List.of_mem_zip hz').2
      · exact Or.inr ⟨j, hl', hav⟩
    obtain ⟨S, hglob, hchain⟩ := hctx.chain rest hsf
    obtain ⟨-, -, hin⟩ :=
      cb_chain rest fr.id fr.function ps S hchain fr' (List.of_mem_zip hz).2 r hk
    have hsk : s.frames.map Frame.skel =
        (fr.function, fr.countedActive) :: rest.map Frame.skel := by rw [hsf]; rfl
    have hc0 := hd.complete
    rw [hsk] at hc0
    refine hin hm _ ?_
    rcases hc0.prov3 hxH hxL with ⟨j, hj⟩ | ⟨a', d', t', hd', hr⟩ | ⟨π, hπ, hinπ, -⟩
    · exact Or.inl (Or.inl ⟨fr.id, j, hj, Nat.le_refl _⟩)
    · refine Or.inl (Or.inr ⟨a', t', hr, fun hS => ?_⟩)
      obtain ⟨hacq, hcnt⟩ := gsidOf_cons hd'
      have := hglob.acq t' hS hacq
      have e : (fr :: rest).map Frame.skel =
          (fr.function, fr.countedActive) :: rest.map Frame.skel := rfl
      rw [e] at this
      omega
    · exact Or.inr ⟨π, hπ, hinπ⟩

end MoveBytecode.Proofs
