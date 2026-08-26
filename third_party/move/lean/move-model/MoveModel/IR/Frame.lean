-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Checked

/-!
# Generic IR frame infrastructure

This module contains pass-independent facts about IR program points and call
frames.  Transformation correctness proofs instantiate the predicates of
`FrameSafe`; frontend-checked program and runtime facts live in
`Checked.lean`.
-/

namespace MoveModel.IR

/-! ## Predicate-parametric frame validity -/

/-- Source-only facts at one IR program point.

`plainRoot` describes a valid root in the tracked frame; `covers` connects a
live reference local to arbitrary analysis metadata.  Both predicates are
chosen by the client proof. -/
structure FrameSafe {Live : Type} [Membership LocalIndex Live] (live : Live)
    (plainRoot : LocalIndex → Prop)
    (covers : LocalIndex → RefTarget → Prop)
    (frame : FrameId) (s : MoveState) : Prop where
  shape : ∀ x ∈ live, ∀ v, s.frames frame x = some v →
    v.refFree ∨ ∃ rt, v = .ref rt
  roots_below : ∀ x ∈ live, ∀ rt,
    s.frames frame x = some (.ref rt) →
    ∀ rootFrame rootLocal, rt.root = .loc rootFrame rootLocal →
      rootFrame ≤ frame
  roots_plain : ∀ x ∈ live, ∀ rt,
    s.frames frame x = some (.ref rt) →
    ∀ rootLocal, rt.root = .loc frame rootLocal → plainRoot rootLocal
  covers : ∀ x ∈ live, ∀ rt,
    s.frames frame x = some (.ref rt) → covers x rt

/-- Restrict a frame-safety certificate to a subset of live locals. -/
theorem FrameSafe.restrict {Live Live' : Type}
    [Membership LocalIndex Live] [Membership LocalIndex Live']
    {live : Live} {live' : Live'}
    {plainRoot : LocalIndex → Prop} {covers : LocalIndex → RefTarget → Prop}
    {frame : FrameId} {s : MoveState}
    (h : FrameSafe live plainRoot covers frame s)
    (hsub : ∀ x, x ∈ live' → x ∈ live) :
    FrameSafe live' plainRoot covers frame s :=
  { shape := fun x hx => h.shape x (hsub x hx)
    roots_below := fun x hx => h.roots_below x (hsub x hx)
    roots_plain := fun x hx => h.roots_plain x (hsub x hx)
    covers := fun x hx => h.covers x (hsub x hx) }

/-- Transport frame safety to a state with the same tracked frame. -/
theorem FrameSafe.changeState {Live : Type} [Membership LocalIndex Live]
    {live : Live}
    {plainRoot : LocalIndex → Prop} {covers : LocalIndex → RefTarget → Prop}
    {frame : FrameId} {s u : MoveState}
    (h : FrameSafe live plainRoot covers frame s)
    (hframe : u.frames frame = s.frames frame) :
    FrameSafe live plainRoot covers frame u := by
  refine ⟨?_, ?_, ?_, ?_⟩
  · simpa [hframe] using h.shape
  · simpa [hframe] using h.roots_below
  · simpa [hframe] using h.roots_plain
  · simpa [hframe] using h.covers

/-- Changing only memory preserves frame safety. -/
theorem FrameSafe.setMemory {Live : Type} [Membership LocalIndex Live]
    {live : Live}
    {plainRoot : LocalIndex → Prop} {covers : LocalIndex → RefTarget → Prop}
    {frame : FrameId} {s : MoveState}
    (h : FrameSafe live plainRoot covers frame s) (m : Memory) :
    FrameSafe live plainRoot covers frame (s.setMemory m) :=
  h.changeState (by simp)

/-- Writing a reference-free value to any frame preserves frame safety. -/
theorem FrameSafe.writeFrameLocal {Live : Type} [Membership LocalIndex Live]
    {live : Live}
    {plainRoot : LocalIndex → Prop} {covers : LocalIndex → RefTarget → Prop}
    {frame rootFrame : FrameId} {s : MoveState}
    (h : FrameSafe live plainRoot covers frame s)
    (rootLocal : LocalIndex) (w : Value) (hfree : w.refFree) :
    FrameSafe live plainRoot covers frame
      (s.writeFrameLocal rootFrame rootLocal w) := by
  by_cases hf : frame = rootFrame
  · subst rootFrame
    have oldSource : ∀ x v,
        (s.writeFrameLocal frame rootLocal w).frames frame x = some v →
        x ≠ rootLocal → s.frames frame x = some v := by
      intro x v hv hne
      simpa [MoveState.writeFrameLocal_frames, hne] using hv
    have impossible {rt : RefTarget}
        (href : (s.writeFrameLocal frame rootLocal w).frames frame rootLocal =
          some (.ref rt)) : False := by
      have : w = .ref rt := by
        simpa [MoveState.writeFrameLocal_frames] using href
      subst w
      simp at hfree
    refine ⟨?_, ?_, ?_, ?_⟩
    · intro x hx v hv
      by_cases hroot : x = rootLocal
      · subst x
        left
        have : v = w := by
          symm
          simpa [MoveState.writeFrameLocal_frames] using hv
        simpa [this] using hfree
      · exact h.shape x hx v (oldSource x v hv hroot)
    · intro x hx rt href root rootLocal' hroot'
      by_cases hroot : x = rootLocal
      · subst x
        exact (impossible href).elim
      · exact h.roots_below x hx rt
          (oldSource x (.ref rt) href hroot) root rootLocal' hroot'
    · intro x hx rt href rootLocal' hroot'
      by_cases hroot : x = rootLocal
      · subst x
        exact (impossible href).elim
      · exact h.roots_plain x hx rt
          (oldSource x (.ref rt) href hroot) rootLocal' hroot'
    · intro x hx rt href
      by_cases hroot : x = rootLocal
      · subst x
        exact (impossible href).elim
      · exact h.covers x hx rt (oldSource x (.ref rt) href hroot)
  · exact h.changeState (by simp [MoveState.writeFrameLocal_frames, hf])

/-- Reference-free frame locals satisfy any frame-safety specialization. -/
theorem FrameSafe.of_refFree {Live : Type} [Membership LocalIndex Live]
    {live : Live}
    {plainRoot : LocalIndex → Prop} {covers : LocalIndex → RefTarget → Prop}
    {frame : FrameId} {s : MoveState}
    (hloc : ∀ x v, s.frames frame x = some v → v.refFree) :
    FrameSafe live plainRoot covers frame s := by
  have noRef : ∀ x rt, s.frames frame x = some (.ref rt) → False := by
    intro x rt href
    exact absurd (hloc x (.ref rt) href) (by simp)
  refine ⟨?_, ?_, ?_, ?_⟩
  · exact fun x _ v hv => Or.inl (hloc x v hv)
  · exact fun x _ rt href => (noRef x rt href).elim
  · exact fun x _ rt href => (noRef x rt href).elim
  · exact fun x _ rt href => (noRef x rt href).elim

/-- Preserve a frame certificate across one active-frame write. -/
theorem FrameSafe.writeLocal {Before Live : Type}
    [Membership LocalIndex Before] [Membership LocalIndex Live]
    {before : Before} {live : Live}
    {plainRoot : LocalIndex → Prop}
    {covers covers' : LocalIndex → RefTarget → Prop}
    {frame : FrameId} {s : MoveState}
    (h : FrameSafe before plainRoot covers frame s)
    (hframe : frame = s.current) {dst : LocalIndex}
    (hsurvive : ∀ y, y ∈ live → y ≠ dst → y ∈ before)
    (hcoverMono : ∀ y rt, covers y rt → covers' y rt) {v : Value}
    (hshapeNew : v.refFree ∨ ∃ rt, v = .ref rt)
    (hbelow : ∀ rt, v = .ref rt → ∀ rootFrame rootLocal,
      rt.root = .loc rootFrame rootLocal → rootFrame ≤ frame)
    (hplain : ∀ rt, v = .ref rt → ∀ rootLocal,
      rt.root = .loc frame rootLocal → plainRoot rootLocal)
    (hcover : ∀ rt, v = .ref rt → covers' dst rt) :
    FrameSafe live plainRoot covers' frame (s.writeLocal dst v) := by
  refine ⟨?_, ?_, ?_, ?_⟩
  · intro y hy w hw
    by_cases hydst : y = dst
    · subst y
      have : v = w := by
        simpa [MoveState.writeLocal_frames, hframe] using hw
      simpa [this] using hshapeNew
    · exact h.shape y (hsurvive y hy hydst) w
        (by simpa [MoveState.writeLocal_frames, hframe, hydst] using hw)
  · intro y hy rt href rootFrame rootLocal hroot
    by_cases hydst : y = dst
    · subst y
      exact hbelow rt
        (by simpa [MoveState.writeLocal_frames, hframe] using href)
        rootFrame rootLocal hroot
    · exact h.roots_below y (hsurvive y hy hydst) rt
        (by simpa [MoveState.writeLocal_frames, hframe, hydst] using href)
        rootFrame rootLocal hroot
  · intro y hy rt href rootLocal hroot
    by_cases hydst : y = dst
    · subst y
      exact hplain rt
        (by simpa [MoveState.writeLocal_frames, hframe] using href)
        rootLocal hroot
    · exact h.roots_plain y (hsurvive y hy hydst) rt
        (by simpa [MoveState.writeLocal_frames, hframe, hydst] using href)
        rootLocal hroot
  · intro y hy rt href
    by_cases hydst : y = dst
    · subst y
      exact hcover rt
        (by simpa [MoveState.writeLocal_frames, hframe] using href)
    · exact hcoverMono y rt (h.covers y (hsurvive y hy hydst) rt
        (by simpa [MoveState.writeLocal_frames, hframe, hydst] using href))

/-! ## Generic stack-indexed relations -/

/-- Lift a relation on individual frame points to an entire call stack. -/
structure FrameStackRel {Point : Type} (pointFrame : Point → FrameId)
    (Rel : Point → MoveState → MoveState → Prop)
    (points : List Point) (s s' : MoveState) : Prop where
  current_eq : s'.current = s.current
  memory_eq : s'.memory = s.memory
  tracked : ∀ p ∈ points, Rel p s s'
  untracked : ∀ frame, (∀ p ∈ points, pointFrame p ≠ frame) →
    s'.frames frame = s.frames frame

/-- Project the relation for the active head point of a related stack. -/
theorem FrameStackRel.head {Point : Type} {pointFrame : Point → FrameId}
    {Rel : Point → MoveState → MoveState → Prop}
    {p : Point} {points : List Point} {s s' : MoveState}
    (h : FrameStackRel pointFrame Rel (p :: points) s s') : Rel p s s' :=
  h.tracked p (by simp)

/-- Drop a stack head whose frame has become equal in both states. -/
theorem FrameStackRel.tail {Point : Type} {pointFrame : Point → FrameId}
    {Rel : Point → MoveState → MoveState → Prop}
    {p : Point} {points : List Point} {s s' : MoveState}
    (h : FrameStackRel pointFrame Rel (p :: points) s s')
    (hframe : ∀ q ∈ points, pointFrame q ≠ pointFrame p)
    (heq : s'.frames (pointFrame p) = s.frames (pointFrame p)) :
    FrameStackRel pointFrame Rel points s s' := by
  refine ⟨h.current_eq, h.memory_eq, ?_, ?_⟩
  · intro q hq
    exact h.tracked q (by simp [hq])
  · intro frame hnone
    by_cases hpf : pointFrame p = frame
    · simpa [hpf] using heq
    · exact h.untracked frame (by
        intro q hq
        rcases List.mem_cons.mp hq with rfl | hq
        · exact hpf
        · exact hnone q hq)

/-- Replace the head point while preserving its frame and establishing its relation. -/
theorem FrameStackRel.replaceHead {Point : Type}
    {pointFrame : Point → FrameId}
    {Rel : Point → MoveState → MoveState → Prop}
    {p p' : Point} {points : List Point} {s s' : MoveState}
    (h : FrameStackRel pointFrame Rel (p :: points) s s')
    (hframe : pointFrame p' = pointFrame p) (hactive : Rel p' s s') :
    FrameStackRel pointFrame Rel (p' :: points) s s' := by
  refine ⟨h.current_eq, h.memory_eq, ?_, ?_⟩
  · intro q hq
    rcases List.mem_cons.mp hq with rfl | hq
    · exact hactive
    · exact h.tracked q (by simp [hq])
  · intro frame hnone
    apply h.untracked frame
    intro q hq
    rcases List.mem_cons.mp hq with rfl | hq
    · have := hnone p' (by simp)
      simpa [hframe] using this
    · exact hnone q (by simp [hq])

/-! ## Returned frame worlds -/

/-- View a returned world as a state at an arbitrary inactive frame. -/
def FrameWorld.asState (w : FrameWorld) : MoveState :=
  { current := 0, frames := w.frames, memory := w.memory }

/-- Finishing the active frame preserves reads rooted in every other frame. -/
theorem FrameWorld.asState_finishFrame_readTarget_of_rootFrame_ne
    (s : MoveState) (t : RefTarget)
    (hroot : ∀ frame rootLocal, t.root = .loc frame rootLocal →
      frame ≠ s.current) :
    s.finishFrame.asState.readTarget t = s.readTarget t := by
  cases t with
  | mk root path =>
    cases root with
    | global r a => rfl
    | loc frame rootLocal =>
      have hne := hroot frame rootLocal rfl
      simp [FrameWorld.asState, MoveState.finishFrame,
        MoveState.readTarget, setFrame, hne]

/-- Lift a stack relation from running states to returned frame worlds. -/
def FrameStackWorldRel {Point : Type} (pointFrame : Point → FrameId)
    (Rel : Point → MoveState → MoveState → Prop)
    (points : List Point) (w w' : FrameWorld) : Prop :=
  FrameStackRel pointFrame Rel points w.asState w'.asState

/-- With no tracked frames, related returned worlds are equal. -/
theorem FrameStackWorldRel.nil_eq {Point : Type}
    {pointFrame : Point → FrameId}
    {Rel : Point → MoveState → MoveState → Prop} {w w' : FrameWorld}
    (h : FrameStackWorldRel pointFrame Rel [] w w') : w' = w := by
  apply FrameWorld.ext
  · funext frame
    simpa [FrameStackWorldRel, FrameWorld.asState] using
      h.untracked frame (by simp)
  · exact h.memory_eq

/-- Finishing the sole tracked active frame produces related returned worlds. -/
theorem FrameStackRel.finish_singleton {Point : Type}
    {pointFrame : Point → FrameId}
    {Rel : Point → MoveState → MoveState → Prop}
    {p : Point} {s s' : MoveState}
    (h : FrameStackRel pointFrame Rel [p] s s')
    (hp : pointFrame p = s.current) : s'.finishFrame = s.finishFrame := by
  apply FrameWorld.ext
  · funext frame
    by_cases hframe : frame = s.current
    · subst frame
      simp [MoveState.finishFrame, h.current_eq]
    · have hframe' : frame ≠ s'.current := by
        simpa [h.current_eq] using hframe
      have hout := h.untracked frame (by
        intro q hq
        simp only [List.mem_singleton] at hq
        subst q
        simpa [hp] using Ne.symm hframe)
      simpa [MoveState.finishFrame, setFrame, hframe, hframe'] using hout
  · exact h.memory_eq

end MoveModel.IR
