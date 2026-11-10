-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefCtx

/-!
# The reference invariant across the instructions on locals and the stack

UNTRUSTED (checked by the kernel). `Pop`, `CopyLoc`, `MoveLoc`, `StLoc`, and
`Eq`/`Neq` on values keep the reference invariant: the analysis's transfer of
each mirrors the frame's step, and the checks it makes rule out invalidating a
tracked reference.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph MoveBytecode.Step

variable {P : Program} {s s' : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit} {view : FunctionView} {A : RefState} {stys : List SignatureToken}
  {tys : List RuntimeType} {ps : List Reference}

/-- **Closing a step** of the current frame. -/
theorem refInv_finish (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hchain : Below P s fr ps rest) {fr' : Frame}
    (hs'f : s'.frames = fr' :: rest) (hheld : s'.held = s.held) (hid : fr'.id = fr.id)
    (hfun : fr'.function = fr.function) (hca : fr'.countedActive = fr.countedActive)
    (hact : s'.active = s.active) (hnext : s.nextFrame ≤ s'.nextFrame)
    (hstor : ∀ r ∈ s'.storage, r.value.conforms P (.struct r.type) = true)
    {es : List Invalidation} (hes : s'.invalidations = es ++ s.invalidations)
    (hst : StorageKept s s' es) (hal : AllowedStep P fr rest ps es) {A' : RefState}
    {L' : Nat → Option Reference} (hd : Desc A' fr' (gsidOf P (s.frames.map Frame.skel)) ps L')
    (hok : ∀ x r, x ∈ heldIds A' → L' x = some r → RefOk P s' fr' r) {A'' : RefState}
    (hA'' : modelRef P fr.function fr'.pc = some A'') (hle : RefLe A' A'')
    (hfrom : RefsFrom fr fr') : RefInvariant P s' :=
  hr.top hsf hs'f hid hfun hca hact hnext hstor hheld hes hst hal hchain
    ⟨A'', by rw [hfun]; exact hA'',
      FrameOk.le ⟨L', by rw [gsidOf_frames hsf hs'f hfun hca]; exact hd, hok⟩ hle⟩ hfrom

/-- **`Pop`.** -/
theorem refInv_pop (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hctx : TopCtx P s fr info code .pop view A stys tys ps)
    (hi : code.code[fr.pc]? = some .pop)
    (h : stepOf (Step.exec P s fr rest info code .pop) = .done (.running s')) :
    RefInvariant P s' := by
  have hcont : (Outcome.running s').continues = true := rfl
  obtain ⟨⟨v, fr₁⟩, hpop, h⟩ := done_bind h hcont pop_ends
  obtain ⟨st, hst, rfl⟩ := pop_ok hpop
  dsimp only at h
  split at h
  · exact (done_wrong h hcont).elim
  · have hs' := Outcome.running.inj (done_next h)
    subst hs'
    obtain ⟨A', hx, hsucc⟩ := hctx.next (by decide)
    obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
    obtain ⟨va, ra, hsa, hcase⟩ := exec_pop hx
    have hlen : ra.length = st.length := by
      have := hctx.height; rw [hsa, hst] at this; simpa using this
    obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1) (by simp [successors, hi, Bytecode.control])
    rcases hcase with ⟨rfl, rfl⟩ | ⟨id, g, rfl, hrel, rfl⟩
    · refine refInv_finish hr hsf (hctx.chain _ hsf) rfl rfl rfl rfl rfl rfl (Nat.le_refl _) hr.storage
        (es := []) rfl (storageKept_of_eq rfl) allowedBy_nil (hd.pop hsa hst hlen rfl rfl rfl rfl rfl) (fun x r hx hxr => ?_)
        hA'' hle (RefsFrom.next (new := []) (n := 1) (by simp [Step.advance, hst]) rfl (by simp))
      refine (hrefs x r ?_ hxr).same hsf rfl rfl rfl rfl rfl rfl rfl
      simp only [heldIds, List.mem_append] at hx ⊢
      rw [hsa]
      simpa [AbsValue.refId?] using hx
    · refine refInv_finish hr hsf (hctx.chain _ hsf) rfl rfl rfl rfl rfl rfl (Nat.le_refl _) hr.storage
        (es := []) rfl (storageKept_of_eq rfl) allowedBy_nil (hd.popRelease hctx.typed hsa hst hlen hrel rfl rfl rfl rfl)
        (fun x r hx hxr => ?_) hA'' hle
        (RefsFrom.next (new := []) (n := 1) (by simp [Step.advance, hst]) rfl (by simp))
      refine (hrefs x r ?_ hxr).same hsf rfl rfl rfl rfl rfl rfl rfl
      simp only [heldIds, List.mem_append] at hx ⊢
      rw [hsa]
      simp only [List.filterMap_cons, AbsValue.refId?, List.mem_cons]
      rcases hx with hx | hx
      · exact Or.inl hx
      · exact Or.inr (Or.inr hx)

/-- **A tracked reference stays tracked** when the current frame's locals it
is not into keep their values, the frames below and the held values are kept,
and no new invalidation covers it. -/
theorem RefOk.transferAt {r : Reference} {frx frx' fr' : Frame} (h : RefOk P s frx r)
    (hidx : frx'.id = frx.id) (hsf : s.frames = fr :: rest) (hs'f : s'.frames = fr' :: rest)
    (hid : fr'.id = fr.id) (hfun : fr'.function = fr.function) (hheld : s'.held = s.held)
    (hstor : s'.storage = s.storage)
    {i : Nat} (hloc : ∀ j, j ≠ i → fr'.locals[j]? = fr.locals[j]?)
    (hroot : ∀ j, r.location.root = .local_ fr.id j → j ≠ i) {es : List Invalidation}
    (hes : s'.invalidations = es ++ s.invalidations) (hcov : ∀ e ∈ es, e.covers r.location = false) :
    RefOk P s' frx' r := by
  have hframe : ∀ f, s'.frame? f = if f = fr.id then some fr' else s.frame? f := by
    intro f
    split
    · next e => subst e; rw [← hid]; exact frame?_head hs'f
    · next hne => exact frame?_cons_ne hsf hs'f hid hne
  refine h.transfer' hidx ?_ hes hcov (fun hr => ?_)
  · cases hρ : r.location.root with
    | local_ f i' =>
      simp only [RootType, hframe]
      split
      · next e => subst e; simp [frame?_head hsf, hfun]
      · rfl
    | held k => simp only [RootType, hheld]
    | global a t => simp only [RootType, hstor]
  · cases hρ : r.location.root with
    | local_ f j =>
      have hl : r.location = ⟨.local_ f j, r.location.path⟩ := by
        cases hl : r.location; simp_all
      rw [hl] at hr ⊢
      by_cases hf : f = fr.id
      · subst hf
        rw [read?_local (frame?_head hsf)] at hr
        rw [read?_local (by rw [← hid]; exact frame?_head hs'f), hloc j (hroot j hρ)]
        exact hr
      · rw [show s'.read? ⟨.local_ f j, r.location.path⟩ = s.read? ⟨.local_ f j, r.location.path⟩ from
          read?_eq_of_root (by simp only [State.root?, frame?_cons_ne hsf hs'f hid hf])]
        exact hr
    | held k =>
      have h1 : s'.root? r.location.root = s.root? r.location.root := by
        rw [hρ]; simp only [State.root?, hheld]
      rw [read?_eq_of_root h1]; exact hr
    | global a t =>
      have h1 : s'.root? r.location.root = s.root? r.location.root := by
        rw [hρ]; simp only [State.root?, hstor]
      rw [read?_eq_of_root h1]; exact hr

/-- No tracked reference is into a local holding a reference. -/
theorem RefOk.not_into_ref {r : Reference} {frx g : Frame} (h : RefOk P s frx r) {f : FrameId}
    (hf : s.frame? f = some g) {i : Nat} {T : RuntimeType}
    (hT : (P.localTypes g.function).bind (·[i]?) = some T) (hTref : ∃ m t, T = .ref m t) :
    ∀ j, r.location.root = .local_ f j → j ≠ i := by
  intro j hj hji
  subst hji
  obtain ⟨⟨T', T'', hT', hpath, hasg⟩, -, hval, -, -⟩ := h
  rw [hj] at hT'
  simp only [RootType, hf, Option.bind_some] at hT'
  rw [hT, Option.some.injEq] at hT'
  subst hT'
  obtain ⟨m, t, rfl⟩ := hTref
  cases hp : r.location.path with
  | nil =>
    rw [hp] at hpath
    simp only [pathType, Option.some.injEq] at hpath
    subst hpath
    rw [← assignable_isValue hasg.assignable] at hval
    simp [RuntimeType.isValue] at hval
  | cons st path =>
    rw [hp] at hpath
    simp [pathType, stepType] at hpath

/-- The declared type of a local holding a reference is a reference type. -/
theorem localType_ref {i : Nat} {r : Reference} (hloc : LocalsOk P fr)
    (hv : fr.locals[i]? = some (some (.ref r))) :
    ∃ T, (P.localTypes fr.function).bind (·[i]?) = some T ∧ ∃ m t, T = .ref m t := by
  obtain ⟨tys, htys, -, hconf⟩ := hloc
  obtain ⟨T, hT, hvT⟩ := hconf i _ hv
  refine ⟨T, by simp [htys, hT], ?_⟩
  rcases assignable_cases (conforms_assignable hvT) with
    ⟨_, _, _, _, he, -, -⟩ | ⟨_, w, -, rfl, -⟩ | he
  · simp [Value.type] at he
  · exact ⟨false, w, rfl⟩
  · exact ⟨r.mutable, r.target, by rw [← he]; rfl⟩

theorem copyLoc_local {m : CompiledModule} {view : FunctionView} {A A' : RefState} {i : Nat}
    (hx : (ReferenceSafety.executeInner m view (.copyLoc i)).run A = .ok ((), A')) :
    ∃ v, A.locals[i]? = some v := by
  simp only [ReferenceSafety.executeInner, run_bind] at hx
  obtain ⟨v, st₁, hc, -⟩ := hx
  simp only [ReferenceSafety.copyLoc, run_bind] at hc
  obtain ⟨old, st₂, hl, -⟩ := hc
  exact ⟨old, (localAt_run.1 hl).1⟩

/-- **`CopyLoc`.** -/
theorem refInv_copyLoc (hr : RefInvariant P s) (hsf : s.frames = fr :: rest) {i : Nat}
    (hctx : TopCtx P s fr info code (.copyLoc i) view A stys tys ps)
    (hi : code.code[fr.pc]? = some (.copyLoc i))
    (h : stepOf (Step.exec P s fr rest info code (.copyLoc i)) = .done (.running s')) :
    RefInvariant P s' := by
  have hcont : (Outcome.running s').continues = true := rfl
  obtain ⟨slot, hslot, h⟩ := done_bind h hcont (ends_need _ _)
  have hslot' := need_ok' hslot
  cases slot with
  | none => exact (done_wrong h hcont).elim
  | some v =>
    dsimp only at h
    split at h
    · have hs' := Outcome.running.inj (done_next h)
      subst hs'
      obtain ⟨A', hx, hsucc⟩ := hctx.next (by intro h; cases h)
      obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
      obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1) (by simp [successors, hi, Bytecode.control])
      obtain ⟨va, hva⟩ := copyLoc_local hx
      cases va with
      | nonReference =>
        obtain ⟨-, rfl⟩ := exec_copyLoc_val hva hx
        refine refInv_finish hr hsf (hctx.chain _ hsf) rfl rfl rfl rfl rfl rfl (Nat.le_refl _) hr.storage
          (es := []) rfl (storageKept_of_eq rfl) allowedBy_nil (hd.pushVal (v := v) rfl rfl rfl rfl) (fun x r hx' hxr => ?_)
          hA'' hle (RefsFrom.next (new := [v]) (n := 0) (by simp [Step.advance, Step.push]) rfl
            (fun w hw => by
              simp only [List.mem_singleton] at hw; subst hw; exact .of_mem (mem_frameVals_locals hslot')))
        refine (hrefs x r ?_ hxr).same hsf rfl rfl rfl rfl rfl rfl rfl
        simpa [heldIds, AbsValue.refId?] using hx'
      | reference id =>
        obtain ⟨b, g1, g2, hmut, hnew, hfac, rfl⟩ := copyLoc_ref hva hx
        obtain ⟨rp, hrp, hLp⟩ := hd.locals i id hva
        rw [hslot'] at hrp
        simp only [Option.some.injEq] at hrp
        subst hrp
        have hp : id ∈ heldIds A := mem_heldIds.2 (Or.inl (List.mem_of_getElem? hva))
        have hbm : b = rp.mutable := by
          have := Graph.isMutable_ok hmut
          rw [hd.complete.held_mut hp hLp, Option.some.injEq] at this
          exact this.symm
        subst hbm
        have hc := hd.complete.copy hp hLp hnew hfac
        have hd' := hd.pushNew (A' := { A with
            graph := g2, nextId := A.nextId + 1, stack := .reference A.nextId :: A.stack })
          (fr' := Step.advance (Step.push fr (.ref rp))) hctx.typed (Graph.newRef_ok hnew).1
          hctx.height hc ⟨rfl, rfl, rfl⟩ rfl rfl rfl
        refine refInv_finish hr hsf (hctx.chain _ hsf) rfl rfl rfl rfl rfl rfl (Nat.le_refl _) hr.storage
          (es := []) rfl (storageKept_of_eq rfl) allowedBy_nil hd' (fun x r hx' hxr => ?_) hA'' hle
          (RefsFrom.next (new := [.ref rp]) (n := 0) (by simp [Step.advance, Step.push]) rfl
            (fun w hw => by
              simp only [List.mem_singleton] at hw; subst hw; exact .of_mem (mem_frameVals_locals hslot')))
        by_cases hxc : x = A.nextId
        · subst hxc
          rw [updL_self, Option.some.injEq] at hxr
          subst hxr
          exact (hrefs id _ hp hLp).same hsf rfl rfl rfl rfl rfl rfl rfl
        · rw [updL_ne _ hxc] at hxr
          refine (hrefs x r ?_ hxr).same hsf rfl rfl rfl rfl rfl rfl rfl
          simp only [heldIds, List.filterMap_cons, AbsValue.refId?, List.mem_append,
            List.mem_cons] at hx' ⊢
          rcases hx' with hx' | hx' | hx'
          · exact Or.inl hx'
          · exact absurd hx' hxc
          · exact Or.inr hx'
    · exact (done_wrong h hcont).elim

theorem heldIds_moveLoc {A : RefState} {i : Nat} {va : AbsValue} (h : A.locals[i]? = some va) :
    ∀ x, x ∈ heldIds { A with locals := A.locals.set i .nonReference, stack := va :: A.stack } ↔
      x ∈ heldIds A := by
  have hperm : List.Perm
      (heldIds { A with locals := A.locals.set i .nonReference, stack := va :: A.stack })
      (heldIds A) := by
    refine perm_of_count fun a => ?_
    have := count_filterMap_set h .nonReference a
    cases va <;> simp_all [heldIds, List.count_append, AbsValue.refId?, List.count_cons] <;> omega
  exact fun x => hperm.mem_iff

theorem Array.getElem?_setIfInBounds_ne {α : Type} {a : Array α} {i j : Nat} {x : α}
    (h : j ≠ i) : (a.setIfInBounds i x)[j]? = a[j]? := by
  rw [Array.getElem?_setIfInBounds]
  simp [Ne.symm h]

/-- An invalidation of local `i` of frame `fid` covers no location rooted
elsewhere. -/
theorem covers_local_false {fid : FrameId} {i : Nat} {l : Location}
    (hl : ∀ j, l.root = .local_ fid j → j ≠ i) :
    Invalidation.covers (.removed (.local_ fid i)) l = false ∧
      Invalidation.covers (.mutated ⟨.local_ fid i, []⟩) l = false := by
  have hne : l.root ≠ .local_ fid i := fun e => hl i e rfl
  have hb : (l.root == Root.local_ fid i) = false := by simpa using hne
  simp only [Invalidation.covers, hb, Bool.false_and, and_self]

/-- What a step on the current frame's local `i` may invalidate. -/
theorem allowed_local {fid : FrameId} {ps : List Reference} {S : StructId → Prop} {i : Nat} :
    AllowedBy fid ps S (.removed (.local_ fid i)) ∧
      AllowedBy fid ps S (.mutated ⟨.local_ fid i, []⟩) := by
  constructor
  · intro l hl
    simp only [Invalidation.covers, beq_iff_eq] at hl
    exact Or.inl ⟨fid, i, hl, Nat.le_refl _⟩
  · intro l hl
    simp only [Invalidation.covers, Bool.and_eq_true, beq_iff_eq] at hl
    exact Or.inl ⟨fid, i, hl.1.1, Nat.le_refl _⟩

/-- **`MoveLoc`.** -/
theorem refInv_moveLoc (hr : RefInvariant P s) (hsf : s.frames = fr :: rest) {i : Nat}
    (hctx : TopCtx P s fr info code (.moveLoc i) view A stys tys ps) (hlocals : LocalsOk P fr)
    (hi : code.code[fr.pc]? = some (.moveLoc i))
    (h : stepOf (Step.exec P s fr rest info code (.moveLoc i)) = .done (.running s')) :
    RefInvariant P s' := by
  have hcont : (Outcome.running s').continues = true := rfl
  obtain ⟨slot, hslot, h⟩ := done_bind h hcont (ends_need _ _)
  have hslot' := need_ok' hslot
  cases slot with
  | none => exact (done_wrong h hcont).elim
  | some v =>
    have hs' := Outcome.running.inj (done_next h)
    subst hs'
    obtain ⟨A', hx, hsucc⟩ := hctx.next (by intro h; cases h)
    obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
    obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1) (by simp [successors, hi, Bytecode.control])
    obtain ⟨va, hva, rfl, hunb⟩ := exec_moveLoc hx
    have hheld := heldIds_moveLoc hva
    -- No tracked reference is into the local moved.
    have hnot : ∀ x r, x ∈ heldIds A → L x = some r →
        ∀ j, r.location.root = .local_ fr.id j → j ≠ i := by
      intro x r hxH hxr
      cases va with
      | nonReference =>
        intro j hj hji
        subst hji
        exact hd.complete.local_unborrowed (hunb rfl) hxH hxr hj
      | reference y =>
        obtain ⟨ry, hry, -⟩ := hd.locals i y hva
        obtain ⟨T, hT, hTr⟩ := localType_ref hlocals hry
        exact (hrefs x r hxH hxr).not_into_ref (frame?_head hsf) hT hTr
    let fr' := Step.advance (Step.push { fr with locals := fr.locals.setIfInBounds i none } v)
    have hd' : Desc { A with locals := A.locals.set i .nonReference, stack := va :: A.stack } fr'
        (gsidOf P (s.frames.map Frame.skel)) ps L := by
      refine ⟨fun j x hj => ?_, fun q x hq => ?_, ?_⟩
      · simp only [List.getElem?_set] at hj
        split at hj
        · split at hj <;> cases hj
        · next hji =>
          obtain ⟨r, hr, hL⟩ := hd.locals j x hj
          refine ⟨r, ?_, hL⟩
          simp only [fr', Step.advance, Step.push]
          rw [Array.getElem?_setIfInBounds_ne (Ne.symm hji)]
          exact hr
      · simp only at hq
        by_cases hq' : q < A.stack.length
        · rw [reverse_cons_getElem?_lt hq'] at hq
          obtain ⟨r, hr, hL⟩ := hd.stack q x hq
          refine ⟨r, ?_, hL⟩
          simp only [fr', Step.advance, Step.push]
          rw [reverse_cons_getElem?_lt (by rw [← hctx.height]; exact hq')]
          exact hr
        · have hqe : q = A.stack.length := by
            have := (List.getElem?_eq_some_iff.1 hq).1; simp at this; omega
          subst hqe
          rw [reverse_cons_getElem?_top, Option.some.injEq] at hq
          subst hq
          obtain ⟨r, hr, hL⟩ := hd.locals i x hva
          rw [hslot'] at hr
          simp only [Option.some.injEq] at hr
          subst hr
          refine ⟨r, ?_, hL⟩
          simp only [fr', Step.advance, Step.push]
          rw [hctx.height, reverse_cons_getElem?_top]
      · simp only [List.length_set]
        exact hd.complete.congrH fun x => (hheld x).symm
    refine refInv_finish hr hsf (hctx.chain _ hsf) (fr' := fr') rfl rfl rfl rfl rfl rfl (Nat.le_refl _)
      hr.storage (es := [.removed (.local_ fr.id i)]) rfl (storageKept_of_eq rfl) ?_ hd'
      (fun x r hx hxr => ?_) hA'' hle
      (RefsFrom.frame (new := [v]) (n := 0) (i := i) (o := none)
        (by simp [fr', Step.advance, Step.push]) rfl
        (fun w hw => by
          simp only [List.mem_singleton] at hw; subst hw; exact .of_mem (mem_frameVals_locals hslot'))
        (by simp))
    · intro S _ e he
      simp only [List.mem_singleton] at he
      subst he
      exact allowed_local.1
    · have hx' := (hheld x).1 hx
      have hok := hrefs x r hx' hxr
      have hloc : ∀ j, j ≠ i → fr'.locals[j]? = fr.locals[j]? := fun j hj => by
        simp only [fr', Step.advance, Step.push]
        exact Array.getElem?_setIfInBounds_ne hj
      refine hok.transferAt rfl hsf rfl rfl rfl rfl rfl hloc (hnot x r hx' hxr)
        (es := [.removed (.local_ fr.id i)]) rfl (fun e he => ?_)
      simp only [List.mem_singleton] at he
      subst he
      exact (covers_local_false (hnot x r hx' hxr)).1

/-- **`StLoc`.** -/
theorem refInv_stLoc (hr : RefInvariant P s) (hsf : s.frames = fr :: rest) {i : Nat}
    (hctx : TopCtx P s fr info code (.stLoc i) view A stys tys ps) (hlocals : LocalsOk P fr)
    (hi : code.code[fr.pc]? = some (.stLoc i))
    (h : stepOf (Step.exec P s fr rest info code (.stLoc i)) = .done (.running s')) :
    RefInvariant P s' := by
  have hcont : (Outcome.running s').continues = true := rfl
  obtain ⟨⟨v, fr₁⟩, hpop, h⟩ := done_bind h hcont pop_ends
  obtain ⟨st, hst, rfl⟩ := pop_ok hpop
  dsimp only at h
  obtain ⟨tl, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨t, -, h⟩ := done_bind h hcont (ends_need _ _)
  obtain ⟨slot, hslot, h⟩ := done_bind h hcont (ends_need _ _)
  have hslot' : fr.locals[i]? = some slot := need_ok' hslot
  have hil : i < fr.locals.size := (Array.getElem?_eq_some_iff.1 hslot').1
  obtain ⟨A', hx, hsucc⟩ := hctx.next (by intro h; cases h)
  obtain ⟨L, hd, hrefs⟩ := hctx.frameOk
  obtain ⟨A'', hA'', hle⟩ := hsucc (fr.pc + 1) (by simp [successors, hi, Bytecode.control])
  obtain ⟨va, ra, old, hsa, hold, hcase⟩ := exec_stLoc hx
  have hlen : ra.length = st.length := by
    have := hctx.height; rw [hsa, hst] at this; simpa using this
  -- The new frame and its locals.
  let fr' := Step.advance { fr with stack := st, locals := fr.locals.setIfInBounds i (some v) }
  have hloc : ∀ j, j ≠ i → fr'.locals[j]? = fr.locals[j]? := fun j hj => by
    simp only [fr', Step.advance]
    exact Array.getElem?_setIfInBounds_ne hj
  have hloci : fr'.locals[i]? = some (some v) := by
    simp only [fr', Step.advance]
    rw [Array.getElem?_setIfInBounds]
    simp [hil]
  -- No tracked reference is into the local overwritten.
  have hnot : ∀ x r, x ∈ heldIds A → L x = some r →
      ∀ j, r.location.root = .local_ fr.id j → j ≠ i := by
    intro x r hxH hxr
    rcases hcase with ⟨rfl, hunb, -⟩ | ⟨id, g, rfl, -, -⟩
    · intro j hj hji
      subst hji
      exact hd.complete.local_unborrowed hunb hxH hxr hj
    · obtain ⟨ro, hro, -⟩ := hd.locals i id hold
      obtain ⟨T, hT, hTr⟩ := localType_ref hlocals hro
      exact (hrefs x r hxH hxr).not_into_ref (frame?_head hsf) hT hTr
  -- The description, for the new locals, given the graph's completeness.
  have hdesc : ∀ (g : Graph) (H : Nat → Prop),
      Complete g L H A.locals.length fr.id (gsidOf P (s.frames.map Frame.skel)) ps →
      (∀ y, H y ↔ y ∈ heldIds { A with locals := A.locals.set i va, stack := ra, graph := g }) →
      Desc { A with locals := A.locals.set i va, stack := ra, graph := g } fr' (gsidOf P (s.frames.map Frame.skel)) ps L := by
    intro g H hc hH
    refine ⟨fun j y hj => ?_, fun q y hq => ?_, ?_⟩
    · simp only [List.getElem?_set] at hj
      split at hj
      · next hji =>
        subst hji
        split at hj
        · simp only [Option.some.injEq] at hj
          subst hj
          obtain ⟨r, hr, hL⟩ := hd.top hsa hst hlen
          exact ⟨r, by rw [hloci, hr], hL⟩
        · cases hj
      · next hji =>
        obtain ⟨r, hr, hL⟩ := hd.locals j y hj
        exact ⟨r, by rw [hloc j (Ne.symm hji)]; exact hr, hL⟩
    · have hq' : q < ra.length := by
        have := (List.getElem?_eq_some_iff.1 hq).1; simpa using this
      obtain ⟨r, hr, hL⟩ := hd.stack q y (by rw [hsa, reverse_cons_getElem?_lt hq']; exact hq)
      rw [hst, reverse_cons_getElem?_lt (by omega)] at hr
      exact ⟨r, hr, hL⟩
    · simp only [List.length_set]
      exact hc.congrH hH
  have hperm : ∀ a, (heldIds { A with locals := A.locals.set i va, stack := ra }).count a +
      (AbsValue.refId? old).toList.count a = (heldIds A).count a := by
    intro a
    have := count_filterMap_set hold va a
    simp only [heldIds, List.count_append, hsa, List.filterMap_cons] at this ⊢
    cases va <;> simp_all [AbsValue.refId?, List.count_cons] <;> omega
  -- The step.
  split at h
  · exact (done_wrong h hcont).elim
  · have hfin : ∀ (es : List Invalidation) (s₁ : State),
        s' = { s₁ with frames := fr' :: rest } → s₁.invalidations = es ++ s.invalidations →
        s₁.nextFrame = s.nextFrame → s₁.storage = s.storage → s₁.held = s.held →
        s₁.active = s.active →
        (∀ e ∈ es, e = .mutated ⟨.local_ fr.id i, []⟩) → RefInvariant P s' := by
      intro es s₁ hs' hes hnf hstor hheld₁ hact₁ hes1
      subst hs'
      have hcov : ∀ x r, x ∈ heldIds A → L x = some r → ∀ e ∈ es, e.covers r.location = false := by
        intro x r hxH hxr e he
        rw [hes1 e he]
        exact (covers_local_false (hnot x r hxH hxr)).2
      have hto : AllowedStep P fr rest ps es := by
        intro S _ e he
        rw [hes1 e he]
        exact allowed_local.2
      rcases hcase with ⟨rfl, -, rfl⟩ | ⟨id, g, rfl, hrel, rfl⟩
      · -- The old value was a value: the graph is unchanged.
        have hH : ∀ y, y ∈ heldIds A ↔
            y ∈ heldIds { A with locals := A.locals.set i va, stack := ra, graph := A.graph } := by
          intro y
          have hp : List.Perm (heldIds { A with locals := A.locals.set i va, stack := ra })
              (heldIds A) := perm_of_count fun a => by
            have := hperm a; simp [AbsValue.refId?] at this; omega
          exact hp.mem_iff.symm
        refine refInv_finish hr hsf (hctx.chain _ hsf) (fr' := fr') rfl hheld₁ rfl rfl rfl hact₁
          (Nat.le_of_eq hnf.symm) (fun r hr' => hr.storage r (hstor ▸ hr')) hes (storageKept_of_eq hstor) hto
          (hdesc A.graph _ hd.complete hH) (fun x r hx' hxr => ?_) hA'' hle
          (RefsFrom.frame (new := []) (n := 1) (i := i) (o := some v)
            (by simp [fr', Step.advance, hst]) rfl (by simp)
            (fun w hw => by
              cases hw; exact .of_mem (mem_frameVals_stack (by rw [hst]; exact List.mem_cons_self))))
        have hxH := (hH x).2 hx'
        exact (hrefs x r hxH hxr).transferAt rfl hsf rfl rfl rfl hheld₁ hstor hloc
          (hnot x r hxH hxr) hes (hcov x r hxH hxr)
      · -- The old value was a reference: released.
        have hidH : id ∈ heldIds A := mem_heldIds.2 (Or.inl (List.mem_of_getElem? hold))
        have hc := hd.complete.release hidH hrel
        have hnd := hctx.typed.distinct
        have hH : ∀ y, (y ∈ heldIds A ∧ y ≠ id) ↔
            y ∈ heldIds { A with locals := A.locals.set i va, stack := ra, graph := g } := by
          have hp : List.Perm (heldIds { A with locals := A.locals.set i va, stack := ra } ++ [id])
              (heldIds A) := perm_of_count fun a => by
            have := hperm a; simp only [AbsValue.refId?, Option.toList_some] at this
            simp only [List.count_append]; omega
          have hnd' := hp.nodup_iff.2 hnd
          intro y
          constructor
          · rintro ⟨hy, hne⟩
            rcases List.mem_append.1 (hp.mem_iff.2 hy) with h1 | h1
            · exact h1
            · simp at h1; exact absurd h1 hne
          · intro hy
            refine ⟨hp.mem_iff.1 (List.mem_append_left _ hy), fun e => ?_⟩
            subst e
            exact (List.nodup_append.1 hnd').2.2 _ hy _ (by simp) rfl
        refine refInv_finish hr hsf (hctx.chain _ hsf) (fr' := fr') rfl hheld₁ rfl rfl rfl hact₁
          (Nat.le_of_eq hnf.symm) (fun r hr' => hr.storage r (hstor ▸ hr')) hes (storageKept_of_eq hstor) hto
          (hdesc g _ hc hH) (fun x r hx' hxr => ?_) hA'' hle
          (RefsFrom.frame (new := []) (n := 1) (i := i) (o := some v)
            (by simp [fr', Step.advance, hst]) rfl (by simp)
            (fun w hw => by
              cases hw; exact .of_mem (mem_frameVals_stack (by rw [hst]; exact List.mem_cons_self))))
        have hxH := ((hH x).2 hx').1
        exact (hrefs x r hxH hxr).transferAt rfl hsf rfl rfl rfl hheld₁ hstor hloc
          (hnot x r hxH hxr) hes (hcov x r hxH hxr)
    split at h
    · next old' =>
      split at h
      · exact (done_wrong h hcont).elim
      · exact hfin [.mutated ⟨.local_ fr.id i, []⟩] _ (Outcome.running.inj (done_next h)) rfl rfl
          rfl rfl rfl (by simp)
    · exact hfin [] _ (Outcome.running.inj (done_next h)) rfl rfl rfl rfl rfl (by simp)

end MoveBytecode.Proofs
