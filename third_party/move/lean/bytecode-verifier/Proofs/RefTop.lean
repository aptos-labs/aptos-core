-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefExecValue

/-!
# The reference invariant across a step of the current frame

UNTRUSTED (checked by the kernel). For each instruction, a step of the current
frame keeps the reference invariant `RefInvariant`, given the analysis's transfer
of the instruction from the certificate's state before it.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model BorrowGraph MoveBytecode.Step

theorem ValueStep.heldIds {j k : Nat} {A A' : RefState} (h : ValueStep j k A A') :
    heldIds A' = heldIds A := by
  obtain ⟨-, hv, rfl⟩ := h
  simp only [Proofs.heldIds, List.filterMap_append, filterMap_refId_replicate, List.nil_append,
    filterMap_refId_drop hv]

/-- The frame at the head of the frames is found by its identity. -/
theorem frame?_head {s : State} {fr : Frame} {rest : List Frame} (h : s.frames = fr :: rest) :
    s.frame? fr.id = some fr := by
  simp [State.frame?, h]

/-- **A tracked reference stays tracked** across a step that replaces the
current frame by one of the same identity, function and locals, keeps the
frames below, the held values and global storage, and invalidates none of
its locations. -/
theorem RefOk.sameFrames {P : Program} {s s' : State} {fr fr' frx : Frame} {rest : List Frame}
    {r : Reference} (h : RefOk P s frx r) (hsf : s.frames = fr :: rest)
    (hs'f : s'.frames = fr' :: rest) (hid : fr'.id = fr.id) (hfun : fr'.function = fr.function)
    (hloc : fr'.locals = fr.locals) (hheld : s'.held = s.held)
    {es : List Invalidation} (hst : StorageKept s s' es)
    (hes : s'.invalidations = es ++ s.invalidations)
    (hcov : ∀ e ∈ es, e.covers r.location = false)
    {frx' : Frame} (hidx : frx'.id = frx.id) : RefOk P s' frx' r := by
  have hframe : ∀ f, s'.frame? f = if f = fr.id then some fr' else s.frame? f := by
    intro f
    split
    · next e => subst e; rw [← hid]; exact frame?_head hs'f
    · next hne => exact frame?_cons_ne hsf hs'f hid hne
  refine h.transfer' hidx ?_ hes hcov (fun hr => ?_)
  · cases hρ : r.location.root with
    | local_ f i =>
      simp only [RootType, hframe]
      split
      · next e => subst e; simp [frame?_head hsf, hfun]
      · rfl
    | held k => simp only [RootType, hheld]
    | global a t => rw [← hρ]; exact (root_kept hst h hcov hρ).1
  · rw [read?_eq_of_root]
    · exact hr
    cases hρ : r.location.root with
    | local_ f i =>
      simp only [State.root?, hframe]
      split
      · next e => subst e; simp [frame?_head hsf, hloc]
      · rfl
    | held k => simp only [State.root?, hheld]
    | global a t => rw [← hρ]; exact (root_kept hst h hcov hρ).2

/-- A tracked reference of the current frame stays tracked across a step that
changes neither the current frame's locals, nor the frames below, the held
values, global storage or the invalidations. -/
theorem RefOk.same {P : Program} {s s' : State} {fr fr' : Frame} {rest : List Frame}
    {r : Reference} (h : RefOk P s fr r) (hsf : s.frames = fr :: rest)
    (hs'f : s'.frames = fr' :: rest) (hid : fr'.id = fr.id) (hfun : fr'.function = fr.function)
    (hloc : fr'.locals = fr.locals) (hinv : s'.invalidations = s.invalidations)
    (hheld : s'.held = s.held) (hstor : s'.storage = s.storage) : RefOk P s' fr' r :=
  h.sameFrames hsf hs'f hid hfun hloc hheld (es := []) (storageKept_of_eq hstor)
    (by rw [hinv]; rfl) (fun e he => by simp at he) hid

/-- A tracked reference of the current frame stays tracked across a step that
changes neither the current frame's locals, nor the frames below, the held
values or the invalidations, and keeps every global resource. -/
theorem RefOk.sameKept {P : Program} {s s' : State} {fr fr' : Frame} {rest : List Frame}
    {r : Reference} (h : RefOk P s fr r) (hsf : s.frames = fr :: rest)
    (hs'f : s'.frames = fr' :: rest) (hid : fr'.id = fr.id) (hfun : fr'.function = fr.function)
    (hloc : fr'.locals = fr.locals) (hinv : s'.invalidations = s.invalidations)
    (hheld : s'.held = s.held) (hst : StorageKept s s' []) : RefOk P s' fr' r :=
  h.sameFrames hsf hs'f hid hfun hloc hheld hst (by rw [hinv]; rfl) (fun e he => by simp at he) hid

variable {P : Program} {s s' : State} {fr : Frame} {rest : List Frame} {info : FunctionInfo}
  {code : CodeUnit}

/-- The analysis names the same global types for frames with the same
functions and activations. -/
theorem gsidOf_frames {s s' : State} {fr fr' : Frame} {rest : List Frame}
    (hsf : s.frames = fr :: rest) (hs'f : s'.frames = fr' :: rest)
    (hfun : fr'.function = fr.function) (hca : fr'.countedActive = fr.countedActive) :
    gsidOf P (s'.frames.map Frame.skel) = gsidOf P (s.frames.map Frame.skel) := by
  rw [hs'f, hsf, List.map_cons, List.map_cons, Frame.skel, Frame.skel, hfun, hca]

/-- **A step on values keeps the reference invariant.** -/
theorem refInv_valueOnly (hr : RefInvariant P s) (hsf : s.frames = fr :: rest)
    (hargs : ∀ x ∈ fr.function.typeArgs, x.isValue = true) {instr : Bytecode}
    (hi : code.code[fr.pc]? = some instr) (hv : instr.ValueOnly = true)
    (h : stepOf (Step.exec P s fr rest info code instr) = .done (.running s'))
    {A A' : RefState} (hA : modelRef P fr.function fr.pc = some A) {j k : Nat}
    (hvs : ValueStep j k A A')
    (hsucc : ∀ pc' ∈ successors code.code fr.pc, ∃ A'', modelRef P fr.function pc' = some A'' ∧
      RefLe A' A'') (hwf : ∀ v ∈ fr.stack, v.wellFormed P = true) : RefInvariant P s' := by
  obtain ⟨⟨fr', new, j', hs'f, hid, hfun, hloc, hstk, hpop, hpc, hca, hnew⟩, hinv, hnext,
    hstor, hheld, hact⟩ := exec_value hi hv h
  obtain ⟨ps, ⟨L, hd, hrefs⟩, hchain⟩ := hr.topOk hsf hA
  have hd' := hd.valueStep hvs hid hloc hstk (hpop hargs)
  have hheldIds := hvs.heldIds
  have hok' : FrameOk P s' fr' ps A' := ⟨L, by rw [gsidOf_frames hsf hs'f hfun hca]; exact hd',
    fun x r hx hxr => by
      rw [hheldIds] at hx
      exact (hrefs x r hx hxr).same hsf hs'f hid hfun hloc hinv hheld hstor⟩
  obtain ⟨A'', hA'', hle⟩ := hsucc fr'.pc hpc
  exact hr.top hsf hs'f hid hfun hca hact (Nat.le_of_eq hnext.symm)
    (fun r hr' => hr.storage r (by rw [← hstor]; exact hr'))
    hheld (es := []) (by rw [hinv]; rfl) (storageKept_of_eq hstor) allowedBy_nil hchain
    ⟨A'', by rw [hfun]; exact hA'', hok'.le hle⟩
    (RefsFrom.next hstk hloc fun v hv' => .of_not_ref
      (hnew hargs (fun w hw => hwf w (List.mem_of_mem_take hw)) v hv'))

end MoveBytecode.Proofs
