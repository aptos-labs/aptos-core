-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.StepsDefs
import Proofs.DanglingM3
import Proofs.NativeAnalysis

/-!
# D4c at each step, for verified programs

UNTRUSTED (checked by the kernel). No write of a step of a verified program
overlaps an immutable reference the analysis tracks (`stepsStable_tracked`),
as `stepsContained_tracked` is for D4b′. A write goes through a reference the
analysis checked writable, or is direct: to a local the analysis checked
unborrowed, to a resource of a type no reference is into, or to a resource
that did not exist.

- In the current frame, a writable reference overlaps nothing immutable the
  analysis holds (`st_writable_overlaps`): nothing held is strictly inside it,
  a reference at its location would have a chain from it or to it, and
  nothing mutable is strictly inside an immutable reference.
- In a suspended frame: what the frames above may write through a reference
  is inside their own locals, global storage no frame below protects, or a
  mutable reference their caller lent them (`WLoc`). One frame down, that
  stays within what the caller may write (`st_lift`), and none of the
  caller's tracked immutable references overlaps it (`st_exclude`): none is
  strictly inside a mutable argument, at its location, or around it.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph ReferenceSafety MoveBytecode.Step

/-- A core step, unfolded: the instruction, its execution and its annotation. -/
private theorem st_parts {P : Program} {s : State} {o : Outcome} {h : Shadow}
    {post : ShadowPost} {evs : List Event} (hstep : step P s = .done o)
    (hc : Shadow.coreStep P s o h = some (post, evs)) :
    ∃ fr rest info code instr, s.frames = fr :: rest ∧ P.function? fr.function = some info ∧
      info.definition.code = some code ∧ code.code[fr.pc]? = some instr ∧
      stepOf (Step.exec P s fr rest info code instr) = .done o ∧
      Shadow.exec P info.moduleId instr s fr o h = some (post, evs) := by
  obtain ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hinstr, hex⟩ := coreStep_exec hc
  refine ⟨fr, rest, info, code, instr, hsf, hinfo, hcode, hinstr, ?_, hex⟩
  simp only [step, hsf, hinfo, hcode, hinstr] at hstep
  exact hstep

/-! ## Overlaps -/

/-- Locations with the same root and path are equal. -/
private theorem st_loc_ext {l m : Location} (hr : l.root = m.root) (hp : l.path = m.path) :
    l = m := by
  obtain ⟨lr, lp⟩ := l
  obtain ⟨mr, mp⟩ := m
  simp only at hr hp
  rw [hr, hp]

/-- Overlapping locations are equal, or one is strictly inside the other. -/
theorem st_overlaps_cases {l m : Location} (h : Access.write.overlaps l m = true) :
    l = m ∨ m.StrictlyInside l ∨ l.StrictlyInside m := by
  simp only [Access.overlaps, Bool.and_eq_true, beq_iff_eq, Bool.or_eq_true,
    List.isPrefixOf_iff_prefix] at h
  obtain ⟨hr, hp | hp⟩ := h
  · by_cases he : l.path = m.path
    · exact Or.inl (st_loc_ext hr he)
    · exact Or.inr (Or.inl ⟨⟨hr.symm, hp⟩, fun e => he e.symm⟩)
  · by_cases he : l.path = m.path
    · exact Or.inl (st_loc_ext hr he)
    · exact Or.inr (Or.inr ⟨⟨hr, hp⟩, he⟩)

/-- A write overlaps nothing rooted elsewhere. -/
theorem st_overlaps_root {l m : Location} (h : m.root ≠ l.root) :
    Access.write.overlaps l m = false := by
  have key : (l.root == m.root) = false := by simpa using fun e => h e.symm
  simp [Access.overlaps, key]

/-- A direct write of a root overlaps nothing rooted elsewhere. -/
theorem st_overlaps_direct {ρ : Root} {m : Location} (h : m.root ≠ ρ) :
    Access.write.overlaps ⟨ρ, []⟩ m = false :=
  st_overlaps_root h

/-! ## The current frame -/

/-- **A writable reference overlaps nothing immutable the analysis holds**:
nothing held is strictly inside it; a reference at its location would have a
chain from it, which writability excludes, or to it, which would make it
mutable; and nothing mutable is strictly inside an immutable reference. -/
theorem st_writable_overlaps {g : Graph} {L : Nat → Option Reference} {H : Nat → Prop}
    {root : Nat} {fid : FrameId} {gsid : Nat → Option StructId} {ps : List Reference}
    (hc : Complete g L H root fid gsid ps) {x : Nat} {rx : Reference} (hx : H x)
    (hxL : L x = some rx) (hw : g.isWritable x = .ok true) {y : Nat} {ry : Reference}
    (hy : H y) (hyL : L y = some ry) (hym : ry.mutable = false) :
    Access.write.overlaps rx.location ry.location = false := by
  obtain ⟨hxm, hnone⟩ := Graph.isWritable_true hw
  rw [hc.held_mut hx hxL, Option.some.injEq] at hxm
  cases ho : Access.write.overlaps rx.location ry.location with
  | false => rfl
  | true =>
    exfalso
    rcases st_overlaps_cases ho with he | hin | hin
    · have hxy : x ≠ y := by
        rintro rfl
        rw [hxL, Option.some.injEq] at hyL
        subst hyL
        rw [hxm] at hym
        cases hym
      rcases hc.same x rx y ry hx hy hxy hxL hyL hxm he.symm with hch | hch
      · obtain ⟨e, n, -, -, -, -, hed, -⟩ := hch.first
        exact hnone e n hed
      · have := hch.mutable_src hyL hxL hxm
        rw [hym] at this
        cases this
    · exact hc.writable_inside hx hxL hw hy hyL hin
    · have := hc.immInside y ry x rx hy hx hyL hxL hym hin
      rw [hxm] at this
      cases this

/-- **A step writes through a reference the analysis checked writable.** -/
private theorem st_writable {m : CompiledModule} {view : FunctionView} {instr : Bytecode}
    {A A' : RefState} (hx : (executeInner m view instr).run A = .ok ((), A')) {k : Nat}
    (hop : RefOperand instr k .write) {x : Nat} (hxk : A.stack[k]? = some (.reference x)) :
    A.graph.isWritable x = .ok true := by
  unfold RefOperand at hop
  rcases hop with ⟨-, -, hrd⟩ | ⟨-, -, hrd⟩ | ⟨rfl, rfl, -⟩ | ⟨⟨sig, rfl⟩, rfl, -⟩ |
    ⟨⟨sig, rfl⟩, rfl, -⟩ | ⟨⟨sig, rfl⟩, rfl, -⟩ | ⟨-, -, hrd⟩
  · cases hrd
  · cases hrd
  · obtain ⟨id, rest', g', hst, hwr, -, -⟩ := exec_writeRef hx
    rw [hst] at hxk
    simp only [List.getElem?_cons_zero, Option.some.injEq, AbsValue.reference.injEq] at hxk
    subst hxk
    exact hwr
  · obtain ⟨id, rest', g', hst, hwr, -, -⟩ := exec_vecPushBack hx
    rw [hst] at hxk
    simp only [List.getElem?_cons_succ, List.getElem?_cons_zero, Option.some.injEq,
      AbsValue.reference.injEq] at hxk
    subst hxk
    exact hwr
  · obtain ⟨id, rest', hst, hwr⟩ := ct_exec_vecPopBack hx
    rw [hst] at hxk
    simp only [List.getElem?_cons_zero, Option.some.injEq, AbsValue.reference.injEq] at hxk
    subst hxk
    exact hwr
  · obtain ⟨id, rest', g', hst, hwr, -, -⟩ := exec_vecSwap hx
    rw [hst] at hxk
    simp only [List.getElem?_cons_succ, List.getElem?_cons_zero, Option.some.injEq,
      AbsValue.reference.injEq] at hxk
    subst hxk
    exact hwr
  · cases hrd

/-- **No write of a verified step overlaps a tracked immutable reference of
the current frame** (D4c within the frame). -/
theorem stable_top {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.Supported) (hsigs : P.ApiSigs) {s : State} (hinv : InvRefs P s) {h : Shadow}
    (hpre : Pre false P s h) {o : Outcome} {post : ShadowPost} {evs : List Event}
    (hstep : step P s = .done o) (hc : Shadow.coreStep P s o h = some (post, evs))
    {g : ShadowFrame} {hs' : List ShadowFrame} {fr : Frame} {rest : List Frame}
    (hh : h.frames = g :: hs') (hsf : s.frames = fr :: rest)
    {l : Location} {via : Option Nat} (ha : Event.accessed .write l via ∈ evs)
    {i : Nat} {r : Reference} (ht : TrackedIn P g fr i r) (hm : r.mutable = false) :
    Access.write.overlaps l r.location = false := by
  obtain ⟨fr₀, rest₀, info, code, instr, hsf₀, hinfo, hcode, hinstr, hstep', hex⟩ :=
    st_parts hstep hc
  rw [hsf, List.cons.injEq] at hsf₀
  obtain ⟨rfl, rfl⟩ := hsf₀
  have hcont : o.continues = true := coreStep_continues hc
  -- The analysis at the current instruction.
  obtain ⟨fr₁, rest₁, hsf₁, ⟨info₁, code₁, hact⟩, -⟩ := hinv.1
  rw [hsf, List.cons.injEq] at hsf₁
  obtain ⟨rfl, -⟩ := hsf₁
  obtain rfl : info₁ = info := Option.some.inj (hact.fn.symm.trans hinfo)
  obtain rfl : code₁ = code := Option.some.inj (hact.body.symm.trans hcode)
  obtain ⟨view, A, stys, tys, ps, hctx⟩ := topCtxGen hwf hs hsigs hinv hsf hact hinstr
  obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
  obtain ⟨A', hx⟩ := hctx.exec
  -- The tracked reference is held by the analysis.
  obtain ⟨y, hy, hyL⟩ := tracked_top_held hctx hact hd ht
  have hok := hrefs y r hy hyL
  -- Nothing tracked is into a local holding a reference.
  have hnotref : ∀ {j y' : Nat}, A.locals[j]? = some (.reference y') →
      r.location.root ≠ .local_ fr.id j := by
    intro j y' hj' e
    obtain ⟨ry, hry, -⟩ := hd.locals j y' hj'
    obtain ⟨T, hT, hTr⟩ := localType_ref hact.locals hry
    exact hok.not_into_ref (frame?_head hsf) hT hTr j e rfl
  cases exec_accessed hh hex ha with
  | @viaRef _ k t rt _ hk _ hop =>
    obtain ⟨x, hxk, hxL⟩ := abs_of_ref hd hctx.typed hctx.targs hctx.sig hctx.conf hctx.height hk
    have hxH : x ∈ heldIds A := mem_heldIds.2 (Or.inr (List.mem_of_getElem? hxk))
    exact st_writable_overlaps hd.complete hxH hxL (st_writable hx hop hxk) hy hyL hm
  | @direct _ j hop =>
    apply st_overlaps_direct
    unfold LocalOperand at hop
    rcases hop with ⟨-, hrd⟩ | ⟨rfl, -⟩ | ⟨rfl, -⟩
    · cases hrd
    · obtain ⟨va, hva, -, hunb⟩ := exec_moveLoc hx
      cases va with
      | nonReference => exact hd.complete.local_unborrowed (hunb rfl) hy hyL
      | reference y' => exact hnotref hva
    · obtain ⟨v, rest', old, -, hold, hcase⟩ := exec_stLoc hx
      rcases hcase with ⟨rfl, hunb, -⟩ | ⟨id, g', rfl, -, -⟩
      · exact hd.complete.local_unborrowed hunb hy hyL
      · exact hnotref hold
  | @global addr sid hop =>
    apply st_overlaps_direct
    unfold GlobalOperand at hop
    have hcase : ((∃ d, instr = .moveFrom d) ∨ ∃ i, instr = .moveFromGeneric i) ∨
        ((∃ d, instr = .moveTo d) ∨ ∃ i, instr = .moveToGeneric i) := by
      rcases hop with h | h | h | h
      · exact Or.inl (Or.inl h)
      · exact Or.inr (Or.inl h)
      · exact Or.inl (Or.inr h)
      · exact Or.inr (Or.inr h)
    rcases hcase with hmf | hmt
    · -- `MoveFrom`, generic or not: the analysis found the type unborrowed.
      obtain ⟨addr', v, sid', hhead, htop, hvt, hl⟩ := ct_moveFrom_loc hmf hh hex ha
      simp only [Location.mk.injEq, Root.global.injEq, and_true] at hl
      obtain ⟨rfl, rfl⟩ := hl
      obtain ⟨d, tys, hdcase, sid0, a0, v0, st, hsid, hstk, hl1, hget, htop0⟩ :=
        ct_moveFrom_step hmf hstep' hcont
      rw [hstk] at hhead
      simp only [List.head?_cons, Option.some.injEq, Value.address.injEq] at hhead
      subst hhead
      rw [htop0, Option.some.injEq] at htop
      subst htop
      obtain ⟨res, hres, hrt, hrv⟩ := get?_mem hget
      have hvt0 := hpre.inv.typed res hres
      rw [hrt, hrv, hvt, RuntimeType.struct.injEq] at hvt0
      subst hvt0
      have hds : P.definedStruct? fr.function.module d = some sid.erase := globalType_erase hsid
      obtain ⟨hx', hacq⟩ := moveFrom_analysis hwf hact.fn hact.body hinstr hdcase hx
      obtain ⟨hcb, -⟩ := exec_moveFrom hx'
      obtain ⟨S0, hglob0, -⟩ := hctx.chain _ hsf
      have hacqT : acqTypes P fr.function sid :=
        ⟨d, by simp only [acqOf, hact.fn, Option.map_some, Option.getD_some]; exact hacq, hds⟩
      have hcount : countActive P ((fr :: rest).map Frame.skel) sid.module ≤ 1 := by
        rw [← hsf, ← hinv.2.active.count]; omega
      have hsk : s.frames.map Frame.skel =
          (fr.function, fr.countedActive) :: rest.map Frame.skel := by
        rw [hsf]; rfl
      intro hr'
      have hc0 := hd.complete
      rw [hsk] at hc0
      rcases hc0.prov y r hy hyL with ⟨i', hi'⟩ | ⟨d', hrc⟩ | ⟨π, hπ, hin, -⟩
      · rw [hr'] at hi'; cases hi'
      · exact rootChain_global_borrowed hrc hyL hr' hds hcb
      · have := hglob0.acq sid (hglob0.params π hπ _ sid (by rw [← hin.1, hr'])) hacqT
        omega
    · -- `MoveTo`, generic or not: no resource was there, so no valid reference is into it.
      obtain ⟨v, sid', rs, addr', hhead, hvt, hrs, hderef, hl⟩ := ct_moveTo_loc hmt hh hex ha
      simp only [Location.mk.injEq, Root.global.injEq, and_true] at hl
      obtain ⟨rfl, rfl⟩ := hl
      obtain ⟨sid0, v0, rs0, st, a0, hstk, -, hvt0, hderef0, hget⟩ :=
        ct_moveTo_step hmt hsf hstep' hcont
      rw [hstk] at hhead hrs
      simp only [List.head?_cons, Option.some.injEq] at hhead
      subst hhead
      simp only [List.getElem?_cons_succ, List.getElem?_cons_zero, Option.some.injEq,
        Value.ref.injEq] at hrs
      subst hrs
      rw [hvt0, RuntimeType.struct.injEq] at hvt
      subst hvt
      rw [hderef0, Option.some.injEq, Value.signer.injEq] at hderef
      subst hderef
      intro hr'
      have hread := (deref?_isSome.1 hok.valid).2
      rw [Location.eta r.location, hr'] at hread
      simp [State.read?, State.root?, hget] at hread

/-! ## The suspended frames -/

/-- The roots the frames from `cid` up may access directly: their own locals,
and global storage of a type the frames below do not protect (`Sc`). -/
private def MayRoot (cid : FrameId) (Sc : StructId → Prop) (ρ : Root) : Prop :=
  (∃ f i, ρ = .local_ f i ∧ cid ≤ f) ∨ ∃ a t, ρ = .global a t ∧ ¬ Sc t

/-- The locations the frames from `cid` up may write through a reference:
those at the roots they may access, and those inside a mutable reference the
frame `cid` received (`cps`). -/
private def WLoc (cid : FrameId) (cps : List Reference) (Sc : StructId → Prop)
    (l : Location) : Prop :=
  MayRoot cid Sc l.root ∨ ∃ π ∈ cps, π.mutable = true ∧ l.Inside π.location

/-- A reference a frame holds where the analysis tracks it. -/
private def Kept (P : Program) (fr : Frame) (r : Reference) : Prop :=
  Value.ref r ∈ fr.stack ∨ ∃ j, fr.locals[j]? = some (some (.ref r)) ∧
    modelAvail P fr.function fr.pc j = .available

/-- One frame down, the roots stay within what the caller may access. -/
private theorem st_liftRoot {P : Program} {s : State} {caller : Frame} {rest : List Frame}
    {ps : List Reference} {S : StructId → Prop} {cid : FrameId} {cfn : FunctionRef}
    {cps : List Reference} {Sc : StructId → Prop}
    (h : SuspOk P s caller rest ps S cid cfn cps Sc) {ρ : Root} (hρ : MayRoot cid Sc ρ) :
    MayRoot caller.id S ρ := by
  rcases hρ with ⟨f, i, hr, hf⟩ | ⟨a, t, hr, hn⟩
  · exact Or.inl ⟨f, i, hr, Nat.le_of_lt (Nat.lt_of_lt_of_le h.order hf)⟩
  · exact Or.inr ⟨a, t, hr, fun hS => hn (h.sub t hS)⟩

/-- **One frame down**, what the frames above may write stays within what the
caller may: a mutable reference the callee received is inside a local of the
caller, global storage it borrowed, or a mutable reference it received
(`CallFacts.exposure`). -/
private theorem st_lift {P : Program} {s : State} {caller : Frame} {rest : List Frame}
    {ps : List Reference} {S : StructId → Prop} {cid : FrameId} {cfn : FunctionRef}
    {cps : List Reference} {Sc : StructId → Prop}
    (h : SuspOk P s caller rest ps S cid cfn cps Sc) {l : Location} (hl : WLoc cid cps Sc l) :
    WLoc caller.id ps S l := by
  rcases hl with hρ | ⟨π, hπ, hπm, hin⟩
  · exact Or.inl (st_liftRoot h hρ)
  obtain ⟨m, rs, rets, args, A, A', A'', L, hf⟩ := h.call
  rcases hf.exposure π hπ hπm with ⟨i, hi⟩ | ⟨a, d, t, hd, hr⟩ | ⟨π', hπ', hm', hin'⟩
  · exact Or.inl (Or.inl ⟨caller.id, i, by rw [hin.1, hi], Nat.le_refl _⟩)
  · refine Or.inl (Or.inr ⟨a, t, by rw [hin.1, hr], fun hS => ?_⟩)
    obtain ⟨hacq, hcnt⟩ := gsidOf_cons (b := caller.countedActive) (sk := rest.map Frame.skel) hd
    have := h.glob.acq t hS hacq
    have e : (caller :: rest).map Frame.skel =
        (caller.function, caller.countedActive) :: rest.map Frame.skel := rfl
    rw [e] at this
    omega
  · exact Or.inr ⟨π', hπ', hm', hin.trans hin'⟩

/-- **A suspended caller's tracked references** are valid, not at a root the
frames above may access, and, when immutable, overlap nothing they may write
through a reference: none is strictly inside a mutable argument
(`CallFacts.refs`), at its location (`CallFacts.immSame`), or strictly around
it (nothing mutable is strictly inside an immutable reference). -/
private theorem st_exclude {P : Program} {s : State} {caller : Frame} {rest : List Frame}
    {ps : List Reference} {S : StructId → Prop} {cid : FrameId} {cfn : FunctionRef}
    {cps : List Reference} {Sc : StructId → Prop}
    (h : SuspOk P s caller rest ps S cid cfn cps Sc) {r : Reference} (hk : Kept P caller r) :
    RefOk P s caller r ∧ (∀ ρ, MayRoot cid Sc ρ → r.location.root ≠ ρ) ∧
      (r.mutable = false → ∀ l, WLoc cid cps Sc l → Access.write.overlaps l r.location = false) := by
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
  refine ⟨hok, hroot, fun hm l hl => ?_⟩
  rcases hl with hρ | ⟨π, hπ, hπm, hinπ⟩
  · exact st_overlaps_root (hroot _ hρ)
  · cases ho : Access.write.overlaps l r.location with
    | false => rfl
    | true =>
      exfalso
      simp only [Access.overlaps, Bool.and_eq_true, beq_iff_eq, Bool.or_eq_true,
        List.isPrefixOf_iff_prefix] at ho
      obtain ⟨hlr, hp⟩ := ho
      have hrπ : r.location.root = π.location.root := hlr.symm.trans hinπ.1
      -- `r` is inside `π`, or `π` strictly inside `r`.
      have hcmp : π.location.path <+: r.location.path ∨ r.location.path <+: π.location.path := by
        rcases hp with hp | hp
        · exact Or.inl (hinπ.2.trans hp)
        · exact List.prefix_or_prefix_of_prefix hinπ.2 hp
      obtain ⟨y, hys, hyL⟩ := hf.cpsArgs π (hf.cpsSub.subset hπ)
      have hyH : y ∈ heldIds A := mem_heldIds.2 (Or.inr (List.mem_of_mem_take hys))
      by_cases he : r.location.path = π.location.path
      · exact hf.immSame x r hx hxL hm π hπ hπm (st_loc_ext hrπ he)
      rcases hcmp with hc | hc
      · exact hsh π hπ hπm ⟨⟨hrπ, hc⟩, he⟩
      · have := hf.desc.complete.immInside x r y π hxH hyH hxL hyL hm
          ⟨⟨hrπ.symm, hc⟩, fun e => he e.symm⟩
        rw [hπm] at this
        cases this

/-- **Down the chain of suspended frames**: what the frames from `cid` up may
write overlaps none of the tracked immutable references of the frames below. -/
private theorem st_chain {P : Program} {s : State} :
    ∀ (frs : List Frame) (cid : FrameId) (cfn : FunctionRef) (cps : List Reference)
      (Sc : StructId → Prop), ChainRef P s cid cfn cps Sc frs → ∀ fr' ∈ frs, ∀ r,
      Kept P fr' r → RefOk P s fr' r ∧ (∀ ρ, MayRoot cid Sc ρ → r.location.root ≠ ρ) ∧
        (r.mutable = false → ∀ l, WLoc cid cps Sc l → Access.write.overlaps l r.location = false)
  | [], _, _, _, _, _, fr', hfr', _, _ => by simp at hfr'
  | caller :: frs, cid, cfn, cps, Sc, ⟨ps, S, hsusp, hrest⟩, fr', hfr', r, hk => by
    rcases List.mem_cons.1 hfr' with rfl | hfr'
    · exact st_exclude hsusp hk
    · obtain ⟨hok, hroot, hin⟩ :=
        st_chain frs caller.id caller.function ps S hrest fr' hfr' r hk
      exact ⟨hok, fun ρ hρ => hroot ρ (st_liftRoot hsusp hρ),
        fun hm l hl => hin hm l (st_lift hsusp hl)⟩

/-- **No write of a verified step overlaps a tracked immutable reference of a
suspended frame.** -/
theorem stable_below {P : Program} (hwf : WellFormedBy verifier VerifierConfig.production P)
    (hs : P.Supported) (hsigs : P.ApiSigs) {s : State} (hinv : InvRefs P s) {h : Shadow}
    (hpre : Pre false P s h) {o : Outcome} {post : ShadowPost} {evs : List Event}
    (hstep : step P s = .done o) (hc : Shadow.coreStep P s o h = some (post, evs))
    {g : ShadowFrame} {hs' : List ShadowFrame} {fr : Frame} {rest : List Frame}
    (hh : h.frames = g :: hs') (hsf : s.frames = fr :: rest)
    {l : Location} {via : Option Nat} (ha : Event.accessed .write l via ∈ evs)
    {g' : ShadowFrame} {fr' : Frame} (hmem : (g', fr') ∈ hs'.zip rest)
    {i : Nat} {r : Reference} (ht : TrackedIn P g' fr' i r) (hm : r.mutable = false) :
    Access.write.overlaps l r.location = false := by
  obtain ⟨fr₀, rest₀, info, code, instr, hsf₀, hinfo, hcode, hinstr, hstep', hex⟩ :=
    st_parts hstep hc
  rw [hsf, List.cons.injEq] at hsf₀
  obtain ⟨rfl, rfl⟩ := hsf₀
  have hcont := coreStep_continues hc
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
    st_chain rest fr.id fr.function ps S hchain fr' (List.of_mem_zip hmem).2 r hk
  obtain ⟨A', hx⟩ := hctx.exec
  cases exec_accessed hh hex ha with
  | @viaRef _ k t rt _ hkk _ hop =>
    -- The reference written through is the current frame's, writable, so mutable: inside
    -- its locals, global storage it borrowed, or a mutable reference it received.
    obtain ⟨L, hd, -⟩ := hctx.frameOk
    obtain ⟨x, hxk, hxL⟩ := abs_of_ref hd hctx.typed hctx.targs hctx.sig hctx.conf hctx.height hkk
    have hxH : x ∈ heldIds A := mem_heldIds.2 (Or.inr (List.mem_of_getElem? hxk))
    have hrtm : rt.mutable = true := by
      have := (Graph.isWritable_true (st_writable hx hop hxk)).1
      rw [hd.complete.held_mut hxH hxL, Option.some.injEq] at this
      exact this
    have hsk : s.frames.map Frame.skel =
        (fr.function, fr.countedActive) :: rest.map Frame.skel := by rw [hsf]; rfl
    have hc0 := hd.complete
    rw [hsk] at hc0
    refine hin hm _ ?_
    rcases hc0.prov3 hxH hxL with ⟨j, hj⟩ | ⟨a', d', t', hd', hr⟩ | ⟨π, hπ, hinπ, hmπ⟩
    · exact Or.inl (Or.inl ⟨fr.id, j, hj, Nat.le_refl _⟩)
    · refine Or.inl (Or.inr ⟨a', t', hr, fun hS => ?_⟩)
      obtain ⟨hacq, hcnt⟩ := gsidOf_cons hd'
      have := hglob.acq t' hS hacq
      have e : (fr :: rest).map Frame.skel =
          (fr.function, fr.countedActive) :: rest.map Frame.skel := rfl
      rw [e] at this
      omega
    · exact Or.inr ⟨π, hπ, hmπ hrtm, hinπ⟩
  | @direct _ j _ => exact st_overlaps_direct (hroot _ (Or.inl ⟨fr.id, j, rfl, Nat.le_refl _⟩))
  | @global addr sid hop =>
    apply st_overlaps_direct
    unfold GlobalOperand at hop
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
      obtain ⟨addr', v, sid', hhead, htop, hvt, hl⟩ := ct_moveFrom_loc hmf hh hex ha
      simp only [Location.mk.injEq, Root.global.injEq, and_true] at hl
      obtain ⟨rfl, rfl⟩ := hl
      obtain ⟨d, tys, hdcase, sid0, a0, v0, st, hsid, hstk, hl1, hget, htop0⟩ :=
        ct_moveFrom_step hmf hstep' hcont
      rw [hstk] at hhead
      simp only [List.head?_cons, Option.some.injEq, Value.address.injEq] at hhead
      subst hhead
      rw [htop0, Option.some.injEq] at htop
      subst htop
      obtain ⟨res, hres, hrt, hrv⟩ := get?_mem hget
      have hvt0 := hpre.inv.typed res hres
      rw [hrt, hrv, hvt, RuntimeType.struct.injEq] at hvt0
      subst hvt0
      obtain ⟨-, hacq⟩ := moveFrom_analysis hwf hact.fn hact.body hinstr hdcase hx
      have hacqT : acqTypes P fr.function sid :=
        ⟨d, by simp only [acqOf, hact.fn, Option.map_some, Option.getD_some]; exact hacq,
          globalType_erase hsid⟩
      have hcount : countActive P ((fr :: rest).map Frame.skel) sid.module ≤ 1 := by
        rw [← hsf, ← hinv.2.active.count]; omega
      have hnotS : ¬ S sid := fun hS => by have := hglob.acq sid hS hacqT; omega
      exact hroot _ (Or.inr ⟨_, sid, rfl, hnotS⟩)
    · -- `MoveTo`, generic or not: no resource was there, so no valid reference is into it.
      obtain ⟨v, sid', rs, addr', hhead, hvt, hrs, hderef, hl⟩ := ct_moveTo_loc hmt hh hex ha
      simp only [Location.mk.injEq, Root.global.injEq, and_true] at hl
      obtain ⟨rfl, rfl⟩ := hl
      obtain ⟨sid0, v0, rs0, st, a0, hstk, -, hvt0, hderef0, hget⟩ :=
        ct_moveTo_step hmt hsf hstep' hcont
      rw [hstk] at hhead hrs
      simp only [List.head?_cons, Option.some.injEq] at hhead
      subst hhead
      simp only [List.getElem?_cons_succ, List.getElem?_cons_zero, Option.some.injEq,
        Value.ref.injEq] at hrs
      subst hrs
      rw [hvt0, RuntimeType.struct.injEq] at hvt
      subst hvt
      rw [hderef0, Option.some.injEq, Value.signer.injEq] at hderef
      subst hderef
      intro hr'
      have hread := (deref?_isSome.1 hok.valid).2
      rw [Location.eta r.location, hr'] at hread
      simp [State.read?, State.root?, hget] at hread

/-! ## Every step -/

/-- **No write of a verified step overlaps a tracked immutable reference**, nor
a reference the step borrows (D4c at each step). -/
theorem stepsStable_tracked {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}
    (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.SupportedNonGeneric) (hm3 : P.M3)
    (hN : NativesRespect P N) (h0 : AdmissibleInitial P s₀) :
    StepsStable P N ν s₀ (Tracked P) := by
  intro s h E hT o post evs hS l via ha i r hheld hm
  have hI := refTraceInv_trace h0 hT
  have hinv := invM3_reachable hwf hs hm3 (certified_of_wellFormed schedulerSound hwf hs) hN h0 s
    (trace_reachable hT)
  cases hS with
  | native hstep _ hn =>
    rcases callNative_shape hstep with hshape | ⟨k, hshape⟩
    case inr =>
      rw [nativeShape?_nonGeneric hs] at hshape
      cases hshape
    obtain ⟨rfl, rfl, rfl, g, hs', info, -, -, -, -, hevs, -⟩ := nativeStep_some hstep hshape hn
    rw [hevs] at ha
    exact absurd ha nativeEvents_not_accessed
  | core hstep hc =>
    obtain ⟨fr, rest, info, code, instr, hsf, -, -, -, hex⟩ := coreStep_exec hc
    rcases hheld with ⟨g', fr', hz, ht⟩ | ⟨par, hb⟩
    · cases hfr : h.frames with
      | nil => rw [hfr] at hz; cases hz
      | cons g hs' =>
        rw [hfr, hsf, List.zip_cons_cons, List.mem_cons] at hz
        rcases hz with he | hz
        · simp only [Prod.mk.injEq] at he
          obtain ⟨rfl, rfl⟩ := he
          exact stable_top hwf (supported_of_nonGeneric hwf hs) hm3.apiSigs hinv hI.pre hstep hc
            hfr hsf ha ht hm
        · exact stable_below hwf (supported_of_nonGeneric hwf hs) hm3.apiSigs hinv hI.pre hstep hc
            hfr hsf ha hz ht hm
    · cases exec_borrow_inspect hex hb ha

/-- **No write of a call of a modeled native overlaps a tracked immutable
reference** of any frame (D4c): it writes through its mutable reference
arguments, which the analysis checked writable. -/
theorem stable_native {P : Program} {ν : NativeFlows}
    (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.Supported)
    (hsigs : P.ApiSigs)
    (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
      ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)
    {s s₁ : State} (hinv : InvRefs P s) {h : Shadow} (hpre : Pre false P s h) {f : FunctionRef}
    {args views vals writes : List Value} {places : List NativePlace} {o : Outcome}
    {post : ShadowPost} {evs : List Event} {k : NativeShape} (hshape : P.nativeShape? f = some k)
    (hstep : step P s = .callNative s₁ f args views)
    (hn : Shadow.nativeStep P ν s f args views vals writes places o h = some (post, evs))
    {l : Location} {via : Option Nat} (ha : Event.accessed .write l via ∈ evs)
    {g' : ShadowFrame} {fr' : Frame} (hz : (g', fr') ∈ h.frames.zip s.frames)
    {i : Nat} {r : Reference} (ht : TrackedIn P g' fr' i r) (hm : r.mutable = false) :
    Access.write.overlaps l r.location = false := by
  obtain ⟨g, hs', fr, rest, n, t, v, hh, hsf, hnl, hargs, -, hmem, -, hl, hkind⟩ :=
    native_access hpre hstep hn ha
  have hvm : v.mutable = true := by
    rcases hkind with he | ⟨-, hvm, -⟩
    · cases he
    · exact hvm
  have hargl : args.length = n := by rw [hargs]; simp [Nat.min_eq_left hnl]
  obtain ⟨fr₀, rest₀, info, code, instr, view, A, stys, tys, ps, L, hsf₀, hact, hi, hctx, hd,
    hrefs, hargsA⟩ := nativeCall_analysis hwf hs hsigs hapi hinv hshape hstep
  rw [hsf, List.cons.injEq] at hsf₀
  obtain ⟨rfl, rfl⟩ := hsf₀
  obtain ⟨kk, x, hkk, hxk, hxH, hxL, hchk⟩ := hargsA v (by rw [hargl]; exact hmem)
  have hwr : A.graph.isWritable x = .ok true := by
    rcases hchk with ⟨hf, -⟩ | ⟨-, hw⟩
    · rw [hvm] at hf; cases hf
    · exact hw
  subst hl
  rw [hh, hsf, List.zip_cons_cons, List.mem_cons] at hz
  rcases hz with he | hz
  · -- A reference of the current frame.
    simp only [Prod.mk.injEq] at he
    obtain ⟨rfl, rfl⟩ := he
    obtain ⟨y, hy, hyL⟩ := tracked_top_held hctx hact hd ht
    exact st_writable_overlaps hd.complete hxH hxL hwr hy hyL hm
  · -- A reference of a suspended frame: the current frame writes inside its locals, global
    -- storage it borrowed, or a mutable reference it received.
    have hk : Kept P fr' r := by
      rcases ht with ⟨t', -, hz'⟩ | ⟨j, ⟨t', -, -, hl'⟩, hav⟩
      · exact Or.inl (List.of_mem_zip hz').2
      · exact Or.inr ⟨j, hl', hav⟩
    obtain ⟨S, hglob, hchain⟩ := hctx.chain rest hsf
    obtain ⟨-, -, hin⟩ :=
      st_chain rest fr.id fr.function ps S hchain fr' (List.of_mem_zip hz).2 r hk
    have hsk : s.frames.map Frame.skel =
        (fr.function, fr.countedActive) :: rest.map Frame.skel := by rw [hsf]; rfl
    have hc0 := hd.complete
    rw [hsk] at hc0
    refine hin hm _ ?_
    rcases hc0.prov3 hxH hxL with ⟨j, hj⟩ | ⟨a', d', t', hd', hr⟩ | ⟨π, hπ, hinπ, hmπ⟩
    · exact Or.inl (Or.inl ⟨fr.id, j, hj, Nat.le_refl _⟩)
    · refine Or.inl (Or.inr ⟨a', t', hr, fun hS => ?_⟩)
      obtain ⟨hacq, hcnt⟩ := gsidOf_cons hd'
      have := hglob.acq t' hS hacq
      have e : (fr :: rest).map Frame.skel =
          (fr.function, fr.countedActive) :: rest.map Frame.skel := rfl
      rw [e] at this
      omega
    · exact Or.inr ⟨π, hπ, hmπ hvm, hinπ⟩

end MoveBytecode.Proofs
