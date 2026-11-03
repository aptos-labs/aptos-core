-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.GhostEffect

/-!
# Occurrences and fit of shadows

UNTRUSTED (checked by the kernel). How the occurrences of a shadow, and its
fit to a state, decompose over frames, locals and operand stacks, global
storage and held values, and how they change when a local is set or the
stack changes.
-/

namespace MoveBytecode.Proofs

/-! ## Lists -/

theorem zip_set_set {α β : Type} :
    ∀ (l₁ : List α) (l₂ : List β) (i : Nat) (a : α) (b : β),
      (l₁.set i a).zip (l₂.set i b) = (l₁.zip l₂).set i (a, b)
  | [], _, _, _, _ => by simp
  | _ :: _, [], _, _, _ => by simp
  | _ :: _, _ :: _, 0, _, _ => by simp
  | _ :: xs, _ :: ys, i + 1, a, b => by simp [zip_set_set xs ys i a b]

/-- Replacing one element of a list changes its flat map by replacing that
element's part, up to order. -/
theorem flatMap_set_perm {α β : Type} (f : α → List β) (l : List α) (i : Nat)
    (hi : i < l.length) (a : α) :
    ∃ R, (l.flatMap f).Perm (f l[i] ++ R) ∧ ((l.set i a).flatMap f).Perm (f a ++ R) := by
  refine ⟨(l.take i).flatMap f ++ (l.drop (i + 1)).flatMap f, ?_, ?_⟩
  · have hl : l = l.take i ++ l[i] :: l.drop (i + 1) := by
      conv => lhs; rw [← List.set_getElem_self hi]
      rw [List.set_eq_take_append_cons_drop, if_pos hi]
    conv => lhs; rw [hl]
    simp only [List.flatMap_append, List.flatMap_cons]
    rw [← List.append_assoc, ← List.append_assoc]
    exact List.perm_append_comm.append_right _ |>.trans (by simp [List.append_assoc])
  · rw [List.set_eq_take_append_cons_drop, if_pos hi]
    simp only [List.flatMap_append, List.flatMap_cons]
    rw [← List.append_assoc, ← List.append_assoc]
    exact List.perm_append_comm.append_right _ |>.trans (by simp [List.append_assoc])

theorem all_set {α : Type} {p : α → Bool} {l : List α} {i : Nat} {a : α}
    (hl : l.all p = true) (ha : p a = true) : (l.set i a).all p = true := by
  rw [List.all_eq_true] at hl ⊢
  intro x hx
  rcases List.mem_or_eq_of_mem_set hx with hx | rfl
  · exact hl x hx
  · exact ha

/-! ## Occurrences of frames and shadows -/

/-- The occurrences in a frame's locals. -/
def localsOcc (g : ShadowFrame) (fr : Frame) : Occ :=
  (g.locals.toList.zip fr.locals.toList).flatMap fun (t, v) => ShadowFrame.slotOccurrences t v

theorem frame_occurrences (g : ShadowFrame) (fr : Frame) :
    g.occurrences fr = localsOcc g fr ++ IdTree.occurrencesAll g.stack fr.stack := rfl

/-- The occurrences in a list of frames. -/
def framesOcc (gs : List ShadowFrame) (frs : List Frame) : Occ :=
  (gs.zip frs).flatMap fun (g, fr) => g.occurrences fr

/-- The occurrences outside the frames: in global storage and in the values
the transaction holds. -/
def extOcc (h : Shadow) (s : State) : Occ :=
  IdTree.occurrencesAll h.storage (s.storage.map (·.value)) ++ IdTree.occurrencesAll h.held s.held

theorem shadow_occurrences (h : Shadow) (s : State) :
    h.occurrences s = framesOcc h.frames s.frames ++ extOcc h s := by
  simp [Shadow.occurrences, framesOcc, extOcc, List.append_assoc]

@[simp] theorem framesOcc_cons (g : ShadowFrame) (gs : List ShadowFrame) (fr : Frame)
    (frs : List Frame) :
    framesOcc (g :: gs) (fr :: frs) = g.occurrences fr ++ framesOcc gs frs := by
  simp [framesOcc]

/-- Frames that fit, frame for frame. -/
def framesFit (gs : List ShadowFrame) (frs : List Frame) : Bool :=
  gs.length == frs.length && (gs.zip frs).all fun (g, fr) => g.fits fr

/-- Global storage and the held values fit. -/
def extFit (h : Shadow) (s : State) : Bool :=
  IdTree.fitsAll h.storage (s.storage.map (·.value)) && IdTree.fitsAll h.held s.held

theorem shadow_fits (h : Shadow) (s : State) :
    h.fits s = (framesFit h.frames s.frames && extFit h s) := by
  simp [Shadow.fits, framesFit, extFit, Bool.and_assoc]

/-- The parts outside the frames depend on global storage and the held values
only. -/
theorem extOcc_congr {h h' : Shadow} {s s' : State} (hst : h'.storage = h.storage)
    (hhe : h'.held = h.held) (sst : s'.storage = s.storage) (she : s'.held = s.held) :
    extOcc h' s' = extOcc h s := by
  simp [extOcc, hst, hhe, sst, she]

theorem extFit_congr {h h' : Shadow} {s s' : State} (hst : h'.storage = h.storage)
    (hhe : h'.held = h.held) (sst : s'.storage = s.storage) (she : s'.held = s.held) :
    extFit h' s' = extFit h s := by
  simp [extFit, hst, hhe, sst, she]

@[simp] theorem framesFit_cons (g : ShadowFrame) (gs : List ShadowFrame) (fr : Frame)
    (frs : List Frame) : framesFit (g :: gs) (fr :: frs) = (g.fits fr && framesFit gs frs) := by
  simp only [framesFit, List.length_cons, List.zip_cons_cons, List.all_cons]
  cases g.fits fr <;> simp

theorem localsOcc_stack (g : ShadowFrame) (fr : Frame) (ts : List IdTree) (vs : List Value)
    (pc : Nat) :
    localsOcc { g with stack := ts } { fr with stack := vs, pc := pc } = localsOcc g fr := rfl

/-- The slots of a frame that fit. -/
def slotsFit (g : ShadowFrame) (fr : Frame) : Bool :=
  (g.locals.toList.zip fr.locals.toList).all fun (t, v) => ShadowFrame.slotFits t v

theorem frame_fits (g : ShadowFrame) (fr : Frame) :
    g.fits fr = (g.locals.size == fr.locals.size && slotsFit g fr &&
      IdTree.fitsAll g.stack fr.stack) := rfl

/-- Setting slot `i` of both the shadow's and the frame's locals replaces that
slot's occurrences. -/
theorem localsOcc_set {g : ShadowFrame} {fr : Frame} (hsize : g.locals.size = fr.locals.size)
    {i : Nat} (hi : i < fr.locals.size) (t : Option IdTree) (v : Option Value) :
    ∃ R, (localsOcc g fr).Perm (ShadowFrame.slotOccurrences g.locals[i] fr.locals[i] ++ R) ∧
      (localsOcc { g with locals := g.locals.setIfInBounds i t }
        { fr with locals := fr.locals.setIfInBounds i v }).Perm
        (ShadowFrame.slotOccurrences t v ++ R) := by
  have hlen : i < (g.locals.toList.zip fr.locals.toList).length := by simp [hsize, hi]
  obtain ⟨R, h₁, h₂⟩ := flatMap_set_perm (fun (p : Option IdTree × Option Value) =>
    ShadowFrame.slotOccurrences p.1 p.2) _ i hlen (t, v)
  refine ⟨R, ?_, ?_⟩
  · simp only [localsOcc] at h₁ ⊢
    simpa using h₁
  · simp only [localsOcc, Array.toList_setIfInBounds]
    rw [zip_set_set]
    simpa using h₂

theorem slotsFit_set {g : ShadowFrame} {fr : Frame} (h : slotsFit g fr = true) (i : Nat)
    {t : Option IdTree} {v : Option Value} (ht : ShadowFrame.slotFits t v = true) :
    slotsFit { g with locals := g.locals.setIfInBounds i t }
      { fr with locals := fr.locals.setIfInBounds i v } = true := by
  simp only [slotsFit, Array.toList_setIfInBounds] at h ⊢
  rw [zip_set_set]
  exact all_set h ht

/-- A slot that fits, looked up in fitting locals. -/
theorem slot_fits {g : ShadowFrame} {fr : Frame} (hsize : g.locals.size = fr.locals.size)
    (h : slotsFit g fr = true) {i : Nat} (hi : i < fr.locals.size) :
    ShadowFrame.slotFits g.locals[i] fr.locals[i] = true := by
  simp only [slotsFit, List.all_eq_true] at h
  apply h (g.locals[i], fr.locals[i])
  rw [List.mem_iff_getElem]
  exact ⟨i, by simp [hsize, hi], by simp⟩

end MoveBytecode.Proofs
