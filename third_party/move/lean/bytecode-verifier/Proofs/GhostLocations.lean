-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.GhostShadow

/-!
# Locations in a state and in its shadow

UNTRUSTED (checked by the kernel). The ghost layer finds the identity tree of
the value at a location (`Shadow.tree?`) the way the state finds the value
(`State.read?`): the root's tree, then the path's indices. For a shadow that
fits its state:

- reading a location finds a tree that fits the value there, whose
  occurrences are among the state's (`tree_read`);
- writing a value at a location, and its tree at the same place in the
  shadow (`Shadow.withTree?`), keeps the shadow fitting, and changes the keyed
  occurrences by replacing the old value's with the new value's
  (`tree_write`). The containers above the location keep their identities and
  keys; their values change.

The state finds a frame and a resource by its first match (`List.find?`), the
shadow by the position of the first match (`List.findIdx?`): they agree.
Writing changes every frame with the frame's identity and every resource
with the resource's address and type (`State.setRoot`), the shadow the first
only: they agree when frame identities and resources are unique, which the
trace invariant maintains (`StateInv`).
-/

namespace MoveBytecode.Proofs

/-! ## Lists -/

theorem find_findIdx {α : Type} {p : α → Bool} :
    ∀ {l : List α} {x : α}, l.find? p = some x → ∃ k, l.findIdx? p = some k ∧ l[k]? = some x
  | [], _, h => by simp at h
  | a :: l, x, h => by
    rw [List.find?_cons] at h
    rw [List.findIdx?_cons]
    cases hp : p a with
    | true =>
      rw [hp] at h
      simp only [Option.some.injEq] at h
      subst h
      exact ⟨0, by simp, rfl⟩
    | false =>
      rw [hp] at h
      obtain ⟨k, hk, hx⟩ := find_findIdx h
      exact ⟨k + 1, by simp [hk], by simpa using hx⟩

/-- Mapping the entries a key identifies, when only one has the key, sets that
entry. -/
theorem map_ite_eq_set {α β : Type} [DecidableEq β] (key : α → β) (b : β) (g : α → α) :
    ∀ {l : List α} {k : Nat} {x : α}, (l.map key).Nodup → l[k]? = some x → key x = b →
      l.map (fun y => if key y = b then g y else y) = l.set k (g x)
  | [], _, _, _, hx, _ => by simp at hx
  | a :: l, 0, x, hnd, hx, hk => by
    simp only [List.getElem?_cons_zero, Option.some.injEq] at hx
    subst hx
    simp only [List.map_cons, List.nodup_cons, List.mem_map] at hnd
    simp only [List.map_cons, hk, ↓reduceIte, List.set_cons_zero, List.cons.injEq, true_and]
    conv => rhs; rw [← List.map_id l]
    refine List.map_congr_left fun y hy => ?_
    have : key y ≠ b := fun h => hnd.1 ⟨y, hy, by rw [h, hk]⟩
    simp [this]
  | a :: l, k + 1, x, hnd, hx, hk => by
    simp only [List.getElem?_cons_succ] at hx
    simp only [List.map_cons, List.nodup_cons, List.mem_map] at hnd
    have ha : key a ≠ b := fun h => hnd.1 ⟨x, List.mem_of_getElem? hx, by rw [hk, h]⟩
    simp only [List.map_cons, ha, ↓reduceIte, List.set_cons_succ, List.cons.injEq, true_and]
    exact map_ite_eq_set key b g hnd.2 hx hk

theorem zip_getElem? {α β : Type} :
    ∀ {l₁ : List α} {l₂ : List β} {k : Nat} {a : α} {b : β}, l₁[k]? = some a → l₂[k]? = some b →
      (l₁.zip l₂)[k]? = some (a, b)
  | [], _, _, _, _, h, _ => by simp at h
  | _ :: _, [], _, _, _, _, h => by simp at h
  | _ :: _, _ :: _, 0, _, _, h₁, h₂ => by simp_all
  | _ :: l₁, _ :: l₂, k + 1, _, _, h₁, h₂ => by
    simp only [List.getElem?_cons_succ, List.zip_cons_cons] at h₁ h₂ ⊢
    exact zip_getElem? h₁ h₂

/-- A flat map over a list with an entry at `k`: that entry's part and the
rest, before and after setting the entry. -/
theorem flatMap_getElem_perm {α β : Type} (f : α → List β) {l : List α} {k : Nat} {x : α}
    (hx : l[k]? = some x) (y : α) :
    ∃ R, (l.flatMap f).Perm (f x ++ R) ∧ ((l.set k y).flatMap f).Perm (f y ++ R) := by
  obtain ⟨hk, hxk⟩ := List.getElem?_eq_some_iff.1 hx
  obtain ⟨R, h₁, h₂⟩ := flatMap_set_perm f l k hk y
  rw [hxk] at h₁
  exact ⟨R, h₁, h₂⟩

theorem fitsAll_get : ∀ {ts : List IdTree} {vs : List Value}, IdTree.fitsAll ts vs = true →
    ∀ {j : Nat} {v : Value}, vs[j]? = some v → ∃ t, ts[j]? = some t ∧ t.fits v = true
  | [], [], _, _, _, hv => by simp at hv
  | t :: ts, v' :: vs, h, 0, v, hv => by
    simp only [List.getElem?_cons_zero, Option.some.injEq] at hv
    subst hv
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    exact ⟨t, rfl, h.1⟩
  | t :: ts, v' :: vs, h, j + 1, v, hv => by
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    simp only [List.getElem?_cons_succ] at hv ⊢
    exact fitsAll_get h.2 hv
  | [], _ :: _, h, _, _, _ => by simp [IdTree.fitsAll] at h
  | _ :: _, [], h, _, _, _ => by simp [IdTree.fitsAll] at h

theorem fitsAll_set : ∀ {ts : List IdTree} {vs : List Value}, IdTree.fitsAll ts vs = true →
    ∀ (j : Nat) {t : IdTree} {v : Value}, t.fits v = true →
      IdTree.fitsAll (ts.set j t) (vs.set j v) = true
  | [], [], _, _, _, _, _ => by simp [IdTree.fitsAll]
  | t' :: ts, v' :: vs, h, 0, t, v, ht => by
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    simp [IdTree.fitsAll, ht, h.2]
  | t' :: ts, v' :: vs, h, j + 1, t, v, ht => by
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    simp [IdTree.fitsAll, h.1, fitsAll_set h.2 j ht]
  | [], _ :: _, h, _, _, _, _ => by simp [IdTree.fitsAll] at h
  | _ :: _, [], h, _, _, _, _ => by simp [IdTree.fitsAll] at h

theorem occurrencesAll_eq_flatMap : ∀ (ts : List IdTree) (vs : List Value),
    IdTree.occurrencesAll ts vs = (ts.zip vs).flatMap fun (t, v) => t.occurrences v
  | [], _ => by simp [IdTree.occurrencesAll]
  | _ :: _, [] => by simp [IdTree.occurrencesAll]
  | t :: ts, v :: vs => by simp [IdTree.occurrencesAll, occurrencesAll_eq_flatMap ts vs]

/-- The occurrences of fitting lists of trees and values, before and after
setting the entries at `j`. -/
theorem occurrencesAll_set {ts : List IdTree} {vs : List Value} {j : Nat} {t : IdTree} {v : Value}
    (ht : ts[j]? = some t) (hv : vs[j]? = some v) (t' : IdTree) (v' : Value) :
    ∃ R, (IdTree.occurrencesAll ts vs).Perm (t.occurrences v ++ R) ∧
      (IdTree.occurrencesAll (ts.set j t') (vs.set j v')).Perm (t'.occurrences v' ++ R) := by
  rw [occurrencesAll_eq_flatMap, occurrencesAll_eq_flatMap, zip_set_set]
  exact flatMap_getElem_perm (fun (p : IdTree × Value) => p.1.occurrences p.2)
    (zip_getElem? ht hv) (t', v')

/-- Closes a permutation between two arrangements of the same parts, by
counting. -/
macro "perm_parts" : tactic =>
  `(tactic| (rw [List.perm_iff_count]; intro _
             simp only [List.count_append, List.count_cons, List.count_nil]; omega))

/-- Moving a middle part to the front. -/
theorem perm_swap3' {α : Type} (A B C D : List α) : (A ++ (B ++ (C ++ D))).Perm (B ++ (A ++ C ++ D)) := by
  rw [← List.append_assoc]
  refine (List.perm_append_comm.append_right _).trans ?_
  simp [List.append_assoc]

/-! ## Paths inside a value -/

@[simp] theorem occurrences_node (i : Nat) (cs : List IdTree) (v : Value) :
    (IdTree.node i cs).occurrences v = (i, v) :: IdTree.occurrencesAll cs v.children := rfl

theorem child_children {v c : Value} {step : PathStep} (h : v.child? step = some c) :
    v.children[v.childIndex step]? = some c := by
  cases v <;> cases step <;> simp_all [Value.child?, Value.children, PathStep.index,
    Value.childIndex, List.getElem?_append_right]
  rw [List.getElem?_append_left (List.getElem?_eq_some_iff.1 h).1, h]

theorem indicesOf_child {v c : Value} {step : PathStep} (h : v.child? step = some c)
    (path : List PathStep) : v.indicesOf (step :: path) = v.childIndex step :: c.indicesOf path := by
  simp [Value.indicesOf, h]

theorem withChild_spec {v c : Value} {step : PathStep} (h : v.child? step = some c)
    (hst : step.writable = true) (w : Value) :
    ∃ v', v.withChild? step w = some v' ∧ v'.children = v.children.set (v.childIndex step) w ∧
      v'.key = v.key := by
  cases v <;> cases step <;> simp only [Value.child?] at h <;> try (simp at h; done)
  all_goals try (simp [PathStep.writable] at hst; done)
  case struct.field s fs i =>
    exact ⟨.struct s (fs.set i w), by simp [Value.withChild?, (List.getElem?_eq_some_iff.1 h).1],
      rfl, rfl⟩
  case vector.element e es i =>
    exact ⟨.vector e (es.set i w), by simp [Value.withChild?, (List.getElem?_eq_some_iff.1 h).1],
      rfl, rfl⟩
  case variant.variantField s tag fs t i =>
    split at h
    · next ht =>
      exact ⟨.variant s tag (fs.set i w),
        by simp [Value.withChild?, ht, (List.getElem?_eq_some_iff.1 h).1], rfl, rfl⟩
    · simp at h
  case table.field s fs ks es i =>
    exact ⟨.table s (fs.set i w) ks es,
      by simp [Value.withChild?, (List.getElem?_eq_some_iff.1 h).1],
      by simp [Value.children, Value.childIndex, PathStep.index,
        List.set_append_left _ _ (List.getElem?_eq_some_iff.1 h).1], rfl⟩
  case table.tableEntry s fs ks es i =>
    exact ⟨.table s fs ks (es.set i w),
      by simp [Value.withChild?, (List.getElem?_eq_some_iff.1 h).1],
      by simp [Value.children, Value.childIndex, List.set_append_right _ _ (Nat.le_add_right _ _)],
      rfl⟩

/-- The occurrences of a node, rearranged around those of a part of one of
its children. -/
theorem perm_node {α : Type} (x : α) {A B C : List α} (D : List α) (h : A.Perm (B ++ C)) :
    (x :: (A ++ D)).Perm (B ++ (x :: D ++ C)) := by
  refine ((h.append_right D).cons x).trans ?_
  rw [List.append_assoc]
  refine List.perm_middle.symm.trans (List.Perm.append_left B ?_)
  exact (List.perm_append_comm).cons x

theorem get_cons {v w : Value} {step : PathStep} {path : List PathStep}
    (h : v.get? (step :: path) = some w) : ∃ c, v.child? step = some c ∧ c.get? path = some w := by
  simp only [Value.get?, bind, Option.bind_eq_some_iff] at h
  exact h

/-- Reading a path: the tree at the path's indices fits the value there, and
its occurrences are among the whole tree's. -/
theorem path_read : ∀ (path : List PathStep) {t : IdTree} {v w : Value}, t.fits v = true →
    v.get? path = some w →
      ∃ tw, t.at? (v.indicesOf path) = some tw ∧ tw.fits w = true ∧
        ∃ R : Occ, (t.occurrences v).Perm (tw.occurrences w ++ R)
  | [], t, v, w, ht, hw => by
    simp only [Value.get?, Option.some.injEq] at hw
    subst hw
    exact ⟨t, rfl, ht, [], by simp⟩
  | step :: path, .node i cs, v, w, ht, hw => by
    obtain ⟨c, hc, hw⟩ := get_cons hw
    simp only [IdTree.fits] at ht
    obtain ⟨tc, htc, hfc⟩ := fitsAll_get ht (child_children hc)
    obtain ⟨tw, hat, hfw, R, hR⟩ := path_read path hfc hw
    obtain ⟨R', hR', -⟩ := occurrencesAll_set htc (child_children hc) tc c
    refine ⟨tw, by simp [indicesOf_child hc, IdTree.at?, IdTree.children, htc, hat], hfw,
      (i, v) :: R' ++ R, ?_⟩
    rw [occurrences_node]
    exact (List.Perm.cons _ hR').trans (perm_node _ R' hR)

/-- Writing a path: the value and the tree, set at the path, still fit; the
containers above keep their identities and keys; the keyed occurrences change
by replacing the old value's with the new value's. -/
theorem path_write : ∀ (path : List PathStep) {t : IdTree} {v w : Value} {tw' : IdTree} {w' : Value},
    t.fits v = true → v.get? path = some w → tw'.fits w' = true → path.all PathStep.writable = true →
      ∃ tw v'' t'', t.at? (v.indicesOf path) = some tw ∧ tw.fits w = true ∧
        v.set? path w' = some v'' ∧ t.set? (v.indicesOf path) tw' = some t'' ∧
        t''.fits v'' = true ∧ (path ≠ [] → v''.key = v.key ∧ t''.id = t.id) ∧
        ∃ R : KOcc, (keys (t.occurrences v)).Perm (keys (tw.occurrences w) ++ R) ∧
          (keys (t''.occurrences v'')).Perm (keys (tw'.occurrences w') ++ R)
  | [], t, v, w, tw', w', ht, hw, hfw', _ => by
    simp only [Value.get?, Option.some.injEq] at hw
    subst hw
    exact ⟨t, w', tw', rfl, ht, rfl, rfl, hfw', by simp, [], by simp, by simp⟩
  | step :: path, .node i cs, v, w, tw', w', ht, hw, hfw', hwr => by
    simp only [List.all_cons, Bool.and_eq_true] at hwr
    obtain ⟨c, hc, hw⟩ := get_cons hw
    simp only [IdTree.fits] at ht
    have hcc := child_children hc
    obtain ⟨tc, htc, hfc⟩ := fitsAll_get ht hcc
    obtain ⟨tw, c'', tc'', hat, hfw, hset, htset, hfc'', -, R, hR, hR''⟩ :=
      path_write path hfc hw hfw' hwr.2
    obtain ⟨v'', hwc, hch, hkey⟩ := withChild_spec hc hwr.1 c''
    obtain ⟨R', hR', hR'set⟩ := occurrencesAll_set htc hcc tc'' c''
    refine ⟨tw, v'', .node i (cs.set (v.childIndex step) tc''), ?_, hfw, ?_, ?_, ?_, ?_,
      (i, v.key) :: keys R' ++ R, ?_, ?_⟩
    · simp [indicesOf_child hc, IdTree.at?, IdTree.children, htc, hat]
    · simp [Value.set?, hc, hset, hwc]
    · simp [indicesOf_child hc, IdTree.set?, IdTree.children, htc, htset, IdTree.id]
    · simp only [IdTree.fits, hch]
      exact fitsAll_set ht _ hfc''
    · intro _; exact ⟨hkey, rfl⟩
    · rw [occurrences_node, keys_cons]
      refine (List.Perm.cons _ (keys_perm hR')).trans ?_
      rw [keys_append]
      exact perm_node _ (keys R') hR
    · rw [occurrences_node, keys_cons, hch, hkey]
      refine (List.Perm.cons _ (keys_perm hR'set)).trans ?_
      rw [keys_append]
      exact perm_node _ (keys R') hR''

/-! ## Unique entries -/

theorem nodup_getElem?_inj {β : Type} : ∀ {l : List β} {j k : Nat} {b : β}, l.Nodup →
    l[j]? = some b → l[k]? = some b → j = k
  | [], _, _, _, _, hj, _ => by simp at hj
  | _ :: _, 0, 0, _, _, _, _ => rfl
  | a :: l, 0, k + 1, b, hnd, hj, hk => by
    simp only [List.getElem?_cons_zero, Option.some.injEq, List.getElem?_cons_succ] at hj hk
    subst hj
    exact absurd (List.mem_of_getElem? hk) (List.nodup_cons.1 hnd).1
  | a :: l, j + 1, 0, b, hnd, hj, hk => by
    simp only [List.getElem?_cons_zero, Option.some.injEq, List.getElem?_cons_succ] at hj hk
    subst hk
    exact absurd (List.mem_of_getElem? hj) (List.nodup_cons.1 hnd).1
  | _ :: l, j + 1, k + 1, b, hnd, hj, hk => by
    simp only [List.getElem?_cons_succ] at hj hk
    rw [nodup_getElem?_inj (List.nodup_cons.1 hnd).2 hj hk]

/-- Entries with distinct keys: an entry with the key of entry `k` is entry
`k`. -/
theorem key_unique {α β : Type} {key : α → β} {l : List α} (hnd : (l.map key).Nodup) {j k : Nat}
    {x y : α} (hj : l[j]? = some y) (hk : l[k]? = some x) (h : key y = key x) : j = k := by
  refine nodup_getElem?_inj hnd (b := key x) ?_ ?_
  · simp [hj, h]
  · simp [hk]

/-- Changing the entries a predicate selects, when only entry `k` satisfies
it, sets entry `k`. -/
theorem map_ite_set {α : Type} {p : α → Bool} {g : α → α} {l : List α} {k : Nat} {x : α}
    (hx : l[k]? = some x) (hp : p x = true)
    (hu : ∀ j y, l[j]? = some y → p y = true → j = k) :
    l.map (fun y => if p y then g y else y) = l.set k (g x) := by
  apply List.ext_getElem?
  intro j
  rw [List.getElem?_map, List.getElem?_set]
  by_cases hjk : k = j
  · subst hjk
    obtain ⟨hk, hxk⟩ := List.getElem?_eq_some_iff.1 hx
    simp [hxk, hp, hk]
  · rw [if_neg hjk]
    cases hl : l[j]? with
    | none => rfl
    | some y =>
      have : p y = false := by
        cases hpy : p y
        · rfl
        · exact absurd (hu j y hl hpy).symm hjk
      simp [this]

/-! ## Slots and frames -/

theorem slot_rw {g : ShadowFrame} {fr : Frame} (hg : g.fits fr = true) {i : Nat} {v : Value}
    (hv : fr.locals[i]? = some (some v)) (t' : IdTree) (v' : Value) :
    ∃ t R, g.locals[i]? = some (some t) ∧ t.fits v = true ∧
      (g.occurrences fr).Perm (t.occurrences v ++ R) ∧
      ({ g with locals := g.locals.setIfInBounds i (some t') }.occurrences
        { fr with locals := fr.locals.setIfInBounds i (some v') }).Perm (t'.occurrences v' ++ R) ∧
      (t'.fits v' = true → { g with locals := g.locals.setIfInBounds i (some t') }.fits
        { fr with locals := fr.locals.setIfInBounds i (some v') } = true) := by
  rw [frame_fits] at hg
  simp only [Bool.and_eq_true, beq_iff_eq] at hg
  obtain ⟨⟨hsize, hslots⟩, hstack⟩ := hg
  have hi : i < fr.locals.size := (Array.getElem?_eq_some_iff.1 hv).1
  have hgi : i < g.locals.size := by rw [hsize]; exact hi
  have hfit := slot_fits hsize hslots hi
  have hfv : fr.locals[i] = some v := (Array.getElem?_eq_some_iff.1 hv).2
  rw [hfv] at hfit
  cases hgt : g.locals[i] with
  | none => rw [hgt] at hfit; simp [ShadowFrame.slotFits] at hfit
  | some t =>
    rw [hgt] at hfit
    obtain ⟨R, hR, hR'⟩ := localsOcc_set hsize hi (some t') (some v')
    rw [hgt, hfv] at hR
    simp only [ShadowFrame.slotOccurrences] at hR hR'
    refine ⟨t, R ++ IdTree.occurrencesAll g.stack fr.stack,
      by rw [Array.getElem?_eq_getElem hgi, hgt], hfit, ?_, ?_, ?_⟩
    · rw [frame_occurrences, ← List.append_assoc]
      exact hR.append_right _
    · rw [frame_occurrences, ← List.append_assoc]
      exact hR'.append_right _
    · intro ht'
      rw [frame_fits]
      simp only [Bool.and_eq_true, beq_iff_eq, Array.size_setIfInBounds]
      exact ⟨⟨hsize, slotsFit_set hslots i (by simpa [ShadowFrame.slotFits] using ht')⟩, hstack⟩

theorem framesFit_get {gs : List ShadowFrame} {frs : List Frame} (h : framesFit gs frs = true)
    {k : Nat} {fr : Frame} (hk : frs[k]? = some fr) : ∃ g, gs[k]? = some g ∧ g.fits fr = true := by
  simp only [framesFit, Bool.and_eq_true, beq_iff_eq, List.all_eq_true] at h
  have hlt : k < gs.length := by rw [h.1]; exact (List.getElem?_eq_some_iff.1 hk).1
  refine ⟨gs[k], List.getElem?_eq_getElem hlt, h.2 (gs[k], fr) ?_⟩
  exact List.mem_of_getElem? (zip_getElem? (List.getElem?_eq_getElem hlt) hk)

theorem framesFit_set {gs : List ShadowFrame} {frs : List Frame} (h : framesFit gs frs = true)
    (k : Nat) {g : ShadowFrame} {fr : Frame} (hg : g.fits fr = true) :
    framesFit (gs.set k g) (frs.set k fr) = true := by
  simp only [framesFit, Bool.and_eq_true, beq_iff_eq, List.length_set] at h ⊢
  refine ⟨h.1, ?_⟩
  rw [zip_set_set]
  exact all_set h.2 hg

theorem framesOcc_set {gs : List ShadowFrame} {frs : List Frame} {k : Nat} {g : ShadowFrame}
    {fr : Frame} (hg : gs[k]? = some g) (hf : frs[k]? = some fr) (g' : ShadowFrame) (fr' : Frame) :
    ∃ R, (framesOcc gs frs).Perm (g.occurrences fr ++ R) ∧
      (framesOcc (gs.set k g') (frs.set k fr')).Perm (g'.occurrences fr' ++ R) := by
  simp only [framesOcc]
  rw [zip_set_set]
  exact flatMap_getElem_perm (fun (p : ShadowFrame × Frame) => p.1.occurrences p.2)
    (zip_getElem? hg hf) (g', fr')

/-! ## Roots -/

/-- The position of the frame with identity `f` depends on the frames'
identities only, and that of a resource on the resources' addresses and
types only. -/
theorem rootTree_congr {h : Shadow} {s s' : State}
    (hf : s'.frames.map (·.id) = s.frames.map (·.id))
    (hst : s'.storage.map (fun r => (r.address, r.type)) = s.storage.map (fun r => (r.address, r.type))) :
    h.rootTree? s' = h.rootTree? s ∧ h.withRootTree? s' = h.withRootTree? s := by
  have hpos : ∀ f, Shadow.framePos s' f = Shadow.framePos s f := by
    intro f
    have h1 : Shadow.framePos s' f = (s'.frames.map (·.id)).findIdx? (· == f) := by
      simp [Shadow.framePos, Function.comp_def]
    have h2 : Shadow.framePos s f = (s.frames.map (·.id)).findIdx? (· == f) := by
      simp [Shadow.framePos, Function.comp_def]
    rw [h1, h2, hf]
  have hres : ∀ a t, Shadow.resourcePos s' a t = Shadow.resourcePos s a t := by
    intro a t
    have h1 : Shadow.resourcePos s' a t =
        (s'.storage.map (fun r => (r.address, r.type))).findIdx? (fun p => p.1 == a && p.2 == t) := by
      simp [Shadow.resourcePos, Function.comp_def]
    have h2 : Shadow.resourcePos s a t =
        (s.storage.map (fun r => (r.address, r.type))).findIdx? (fun p => p.1 == a && p.2 == t) := by
      simp [Shadow.resourcePos, Function.comp_def]
    rw [h1, h2, hst]
  constructor
  · funext root
    cases root <;> simp [Shadow.rootTree?, hpos, hres]
  · funext root t
    cases root <;> simp [Shadow.withRootTree?, hpos, hres]

theorem storage_get {s : State} {a : Address} {ty : StructId} {v : Value}
    (h : s.storage.get? a ty = some v) :
    ∃ k res, Shadow.resourcePos s a ty = some k ∧ s.storage[k]? = some res ∧ res.value = v ∧
      res.address = a ∧ res.type = ty := by
  simp only [GlobalStorage.get?, Option.map_eq_some_iff] at h
  obtain ⟨res, hf, rfl⟩ := h
  have hp := List.find?_some hf
  simp only [Bool.and_eq_true, beq_iff_eq] at hp
  obtain ⟨k, hk, hres⟩ := find_findIdx hf
  exact ⟨k, res, hk, hres, rfl, hp⟩

theorem frame_get {s : State} {f : FrameId} {fr : Frame} (h : s.frame? f = some fr) :
    ∃ k, Shadow.framePos s f = some k ∧ s.frames[k]? = some fr :=
  find_findIdx h

/-- Reading a root: the shadow's tree there fits the value, and its
occurrences are among the state's. -/
theorem root_read {h : Shadow} {s : State} (hfit : h.fits s = true) {root : Root} {v : Value}
    (hr : s.root? root = some v) :
    ∃ t, h.rootTree? s root = some t ∧ t.fits v = true ∧
      ∃ R : Occ, (h.occurrences s).Perm (t.occurrences v ++ R) := by
  rw [shadow_fits, Bool.and_eq_true] at hfit
  obtain ⟨hfr, hext⟩ := hfit
  simp only [extFit, Bool.and_eq_true] at hext
  cases root with
  | local_ f i =>
    simp only [State.root?, bind, Option.bind_eq_some_iff] at hr
    obtain ⟨fr, hfr', slot, hslot, rfl⟩ := hr
    obtain ⟨k, hk, hfk⟩ := frame_get hfr'
    obtain ⟨g, hgk, hg⟩ := framesFit_get hfr hfk
    obtain ⟨t, R, hgt, ht, hR, -⟩ := slot_rw hg hslot (.node 0 []) (.bool false)
    obtain ⟨R', hR', -⟩ := framesOcc_set hgk hfk g fr
    refine ⟨t, by simp [Shadow.rootTree?, hk, hgk, hgt], ht, R ++ R' ++ extOcc h s, ?_⟩
    rw [shadow_occurrences]
    refine (hR'.append_right _).trans ?_
    refine ((hR.append_right _).append_right _).trans ?_
    simp [List.append_assoc]
  | global a ty =>
    obtain ⟨k, res, hk, hres, rfl, -⟩ := storage_get hr
    have hv : (s.storage.map (·.value))[k]? = some res.value := by simp [hres]
    obtain ⟨t, htk, ht⟩ := fitsAll_get hext.1 hv
    obtain ⟨R, hR, -⟩ := occurrencesAll_set htk hv t res.value
    refine ⟨t, by simp [Shadow.rootTree?, hk, htk], ht,
      framesOcc h.frames s.frames ++ R ++ IdTree.occurrencesAll h.held s.held, ?_⟩
    rw [shadow_occurrences, extOcc]
    refine ((hR.append_right _).append_left _).trans ?_
    simp only [List.append_assoc]
    rw [← List.append_assoc]
    refine (List.perm_append_comm.append_right _).trans ?_
    simp [List.append_assoc]
  | held k =>
    simp only [State.root?] at hr
    obtain ⟨t, htk, ht⟩ := fitsAll_get hext.2 hr
    obtain ⟨R, hR, -⟩ := occurrencesAll_set htk hr t v
    refine ⟨t, by simp [Shadow.rootTree?, htk], ht,
      framesOcc h.frames s.frames ++ IdTree.occurrencesAll h.storage (s.storage.map (·.value)) ++ R,
      ?_⟩
    rw [shadow_occurrences, extOcc]
    refine ((hR.append_left _).append_left _).trans ?_
    perm_parts

/-- Writing a root, with frames and resources unique: the shadow, with the
new value's tree at the root, fits the new state; the occurrences change by
replacing the old value's with the new value's. -/
theorem root_write {h : Shadow} {s : State} (hfit : h.fits s = true)
    (hids : (s.frames.map (·.id)).Nodup)
    (hres : (s.storage.map fun r => (r.address, r.type)).Nodup) {root : Root} {v : Value}
    (hr : s.root? root = some v) {t' : IdTree} {v' : Value} (ht' : t'.fits v' = true) :
    ∃ t h', h.rootTree? s root = some t ∧ t.fits v = true ∧ h.withRootTree? s root t' = some h' ∧
      h'.fits (s.setRoot root v') = true ∧ h'.fresh = h.fresh ∧
      (∀ x ∈ framesOcc h'.frames (s.setRoot root v').frames,
        x ∈ framesOcc h.frames s.frames ∨
          (x ∈ t'.occurrences v' ∧ ∀ y ∈ t.occurrences v, y ∈ framesOcc h.frames s.frames)) ∧
      ∃ R : Occ, (h.occurrences s).Perm (t.occurrences v ++ R) ∧
        (h'.occurrences (s.setRoot root v')).Perm (t'.occurrences v' ++ R) := by
  rw [shadow_fits, Bool.and_eq_true] at hfit
  obtain ⟨hfr, hext⟩ := hfit
  have hext' := hext
  simp only [extFit, Bool.and_eq_true] at hext'
  cases root with
  | local_ f i =>
    simp only [State.root?, bind, Option.bind_eq_some_iff] at hr
    obtain ⟨fr, hfr', slot, hslot, rfl⟩ := hr
    have hfr'' : s.frames.find? (·.id == f) = some fr := hfr'
    have hpf : (fr.id == f) = true := List.find?_some (p := fun (x : Frame) => x.id == f) hfr''
    obtain ⟨k, hk, hfk⟩ := frame_get hfr'
    obtain ⟨g, hgk, hg⟩ := framesFit_get hfr hfk
    obtain ⟨t, R, hgt, ht, hR, hR', hfit'⟩ := slot_rw hg hslot t' v'
    obtain ⟨R', hF, hF'⟩ := framesOcc_set hgk hfk { g with locals := g.locals.setIfInBounds i (some t') }
      { fr with locals := fr.locals.setIfInBounds i (some v') }
    have hmap : s.frames.map (fun fr' => if (fr'.id == f) = true then
          { fr' with locals := fr'.locals.setIfInBounds i (some v') } else fr') =
        s.frames.set k { fr with locals := fr.locals.setIfInBounds i (some v') } := by
      refine map_ite_set (p := fun fr' => fr'.id == f) hfk hpf fun j y hy hpy => ?_
      exact key_unique hids hy hfk (by simp at hpf hpy; rw [hpy, hpf])
    refine ⟨t, { h with frames := h.frames.set k { g with locals := g.locals.setIfInBounds i (some t') } },
      by simp [Shadow.rootTree?, hk, hgk, hgt], ht,
      by simp [Shadow.withRootTree?, hk, hgk, hgt], ?_, rfl, fun x hx => ?_, R ++ R' ++ extOcc h s,
      ?_, ?_⟩
    · rw [shadow_fits, Bool.and_eq_true]
      refine ⟨?_, ?_⟩
      · simp only [State.setRoot, hmap]
        exact framesFit_set hfr k (hfit' ht')
      · exact hext
    · simp only [State.setRoot, hmap] at hx
      rcases List.mem_append.1 ((hF'.trans ((hR'.append_right _).trans
          (by simp only [List.append_assoc]; exact List.Perm.refl _))).subset hx) with hx | hx
      · refine Or.inr ⟨hx, fun y hy => hF.symm.subset ?_⟩
        exact List.mem_append_left _ (hR.symm.subset (List.mem_append_left _ hy))
      · refine Or.inl (hF.symm.subset ?_)
        rcases List.mem_append.1 hx with hx | hx
        · exact List.mem_append_left _ (hR.symm.subset (List.mem_append_right _ hx))
        · exact List.mem_append_right _ hx
    · rw [shadow_occurrences]
      refine (hF.append_right _).trans ?_
      refine ((hR.append_right _).append_right _).trans ?_
      simp [List.append_assoc]
    · rw [shadow_occurrences]
      simp only [State.setRoot, hmap]
      refine (hF'.append_right _).trans ?_
      refine ((hR'.append_right _).append_right _).trans ?_
      simp only [List.append_assoc]
      exact List.Perm.refl _
  | global a ty =>
    obtain ⟨k, res, hk, hresk, rfl, hra, hrt⟩ := storage_get hr
    have hpf : (res.address == a && res.type == ty) = true := by simp [hra, hrt]
    have hv : (s.storage.map (·.value))[k]? = some res.value := by simp [hresk]
    obtain ⟨t, htk, ht⟩ := fitsAll_get hext'.1 hv
    obtain ⟨R, hR, hR'⟩ := occurrencesAll_set htk hv t' v'
    have hmap : s.storage.update a ty v' = s.storage.set k { res with value := v' } := by
      unfold GlobalStorage.update
      refine map_ite_set (p := fun r => r.address == a && r.type == ty) hresk hpf
        fun j y hy hpy => ?_
      refine key_unique hres hy hresk ?_
      simp only [Bool.and_eq_true, beq_iff_eq] at hpf hpy
      rw [hpy.1, hpy.2, hpf.1, hpf.2]
    refine ⟨t, { h with storage := h.storage.set k t' }, by simp [Shadow.rootTree?, hk, htk], ht,
      by simp [Shadow.withRootTree?, hk, htk], ?_, rfl,
      fun x hx => Or.inl (by simpa [State.setRoot] using hx),
      framesOcc h.frames s.frames ++ R ++ IdTree.occurrencesAll h.held s.held, ?_, ?_⟩
    · rw [shadow_fits, Bool.and_eq_true]
      refine ⟨by simpa [State.setRoot] using hfr, ?_⟩
      simp only [extFit, State.setRoot, hmap, List.map_set, Bool.and_eq_true]
      exact ⟨fitsAll_set hext'.1 k ht', hext'.2⟩
    · rw [shadow_occurrences, extOcc]
      refine ((hR.append_right _).append_left _).trans ?_
      perm_parts
    · rw [shadow_occurrences, extOcc]
      simp only [State.setRoot, hmap, List.map_set]
      refine ((hR'.append_right _).append_left _).trans ?_
      perm_parts
  | held k =>
    simp only [State.root?] at hr
    obtain ⟨t, htk, ht⟩ := fitsAll_get hext'.2 hr
    obtain ⟨R, hR, hR'⟩ := occurrencesAll_set htk hr t' v'
    refine ⟨t, { h with held := h.held.set k t' }, by simp [Shadow.rootTree?, htk], ht,
      by simp [Shadow.withRootTree?, htk], ?_, rfl,
      fun x hx => Or.inl (by simpa [State.setRoot] using hx),
      framesOcc h.frames s.frames ++ IdTree.occurrencesAll h.storage (s.storage.map (·.value)) ++ R,
      ?_, ?_⟩
    · rw [shadow_fits, Bool.and_eq_true]
      refine ⟨by simpa [State.setRoot] using hfr, ?_⟩
      simp only [extFit, State.setRoot, Bool.and_eq_true]
      exact ⟨hext'.1, fitsAll_set hext'.2 k ht'⟩
    · rw [shadow_occurrences, extOcc]
      refine ((hR.append_left _).append_left _).trans ?_
      perm_parts
    · rw [shadow_occurrences, extOcc]
      simp only [State.setRoot]
      refine ((hR'.append_left _).append_left _).trans ?_
      perm_parts

/-! ## Locations -/

/-- Reading a location: the shadow's tree there fits the value, and its
occurrences are among the state's. -/
theorem tree_read {h : Shadow} {s : State} (hfit : h.fits s = true) {l : Location} {w : Value}
    (hr : s.read? l = some w) :
    ∃ tw, h.tree? s l = some tw ∧ tw.fits w = true ∧
      ∃ R : Occ, (h.occurrences s).Perm (tw.occurrences w ++ R) := by
  simp only [State.read?, bind, Option.bind_eq_some_iff] at hr
  obtain ⟨v, hv, hw⟩ := hr
  obtain ⟨t, ht, hft, R, hR⟩ := root_read hfit hv
  obtain ⟨tw, hat, hfw, R', hR'⟩ := path_read l.path hft hw
  refine ⟨tw, by simp [Shadow.tree?, ht, Location.indicesIn, hv, hat], hfw, R' ++ R, ?_⟩
  refine hR.trans ((hR'.append_right R).trans ?_)
  simp [List.append_assoc]

/-- Writing a location, with frames and resources unique: the shadow, with
the new value's tree at the location, fits the new state; the containers
above the location keep their keys; the keyed occurrences change by
replacing the old value's with the new value's. -/
theorem tree_write {h : Shadow} {s : State} (hfit : h.fits s = true)
    (hids : (s.frames.map (·.id)).Nodup)
    (hres : (s.storage.map fun r => (r.address, r.type)).Nodup) {l : Location} {w : Value}
    (hr : s.read? l = some w) {tw' : IdTree} {w' : Value} (hf' : tw'.fits w' = true)
    (hwr : l.path.all PathStep.writable = true) :
    ∃ tw v v'' h', s.root? l.root = some v ∧ h.tree? s l = some tw ∧ tw.fits w = true ∧
      v.set? l.path w' = some v'' ∧ s.write? l w' = some (s.setRoot l.root v'') ∧
      h.withTree? s l tw' = some h' ∧ h'.fits (s.setRoot l.root v'') = true ∧
      h'.fresh = h.fresh ∧ (l.path ≠ [] → v''.key = v.key) ∧
      (∀ x ∈ framesOcc h'.frames (s.setRoot l.root v'').frames,
        (x.1, x.2.key) ∈ keys (framesOcc h.frames s.frames) ∨
          (x.1, x.2.key) ∈ keys (tw'.occurrences w')) ∧
      ∃ R : KOcc, (keys (h.occurrences s)).Perm (keys (tw.occurrences w) ++ R) ∧
        (keys (h'.occurrences (s.setRoot l.root v''))).Perm (keys (tw'.occurrences w') ++ R) := by
  simp only [State.read?, bind, Option.bind_eq_some_iff] at hr
  obtain ⟨v, hv, hw⟩ := hr
  obtain ⟨t, ht, hft, -⟩ := root_read hfit hv
  obtain ⟨tw, v'', t'', hat, hfw, hset, htset, hft'', hkey, R, hR, hR2⟩ :=
    path_write l.path hft hw hf' hwr
  obtain ⟨t₀, h', ht₀, -, hwr, hfit', hfresh, hfrm, R', hR', hR3⟩ :=
    root_write hfit hids hres hv hft''
  rw [ht] at ht₀
  simp only [Option.some.injEq] at ht₀
  subst ht₀
  refine ⟨tw, v, v'', h', hv, by simp [Shadow.tree?, ht, Location.indicesIn, hv, hat], hfw, hset,
    by simp [State.write?, hv, hset],
    by simp [Shadow.withTree?, ht, Location.indicesIn, hv, htset, hwr],
    hfit', hfresh, fun hp => (hkey hp).1, fun x hx => ?_, R ++ keys R', ?_, ?_⟩
  · rcases hfrm x hx with hx | ⟨hx, hall⟩
    · exact Or.inl (mem_keys hx)
    · rcases List.mem_append.1 (hR2.subset (mem_keys hx)) with hx | hx
      · exact Or.inr hx
      · obtain ⟨o, ho, h1, h2⟩ := mem_keys_iff.1 (hR.symm.subset (List.mem_append_right _ hx))
        have := mem_keys (hall o ho)
        rw [h1, h2] at this
        exact Or.inl this
  · refine (keys_perm hR').trans ?_
    rw [keys_append]
    refine (hR.append_right _).trans ?_
    simp [List.append_assoc]
  · refine (keys_perm hR3).trans ?_
    rw [keys_append]
    refine (hR2.append_right _).trans ?_
    simp [List.append_assoc]

/-- **Per frame, writing a root**: a frame has after the write the occurrences
it had, or, for the frame holding the root, the new value's. -/
theorem root_write_frames {h : Shadow} {s : State} (hfit : h.fits s = true)
    (hids : (s.frames.map (·.id)).Nodup) {root : Root} {v : Value}
    (hr : s.root? root = some v) {t t' : IdTree} {v' : Value} {h' : Shadow}
    (ht : h.rootTree? s root = some t) (hw : h.withRootTree? s root t' = some h') :
    ∀ (k : Nat) (g' : ShadowFrame) (fr' : Frame) (g : ShadowFrame) (fr : Frame),
      h'.frames[k]? = some g' → (s.setRoot root v').frames[k]? = some fr' →
      h.frames[k]? = some g → s.frames[k]? = some fr → ∀ x ∈ g'.occurrences fr',
        x ∈ g.occurrences fr ∨
          (x ∈ t'.occurrences v' ∧ ∀ y ∈ t.occurrences v, y ∈ g.occurrences fr) := by
  rw [shadow_fits, Bool.and_eq_true] at hfit
  obtain ⟨hfr, -⟩ := hfit
  intro k g' fr' g fr h1 h2 h3 h4 x hx
  cases root with
  | local_ f i =>
    simp only [State.root?, bind, Option.bind_eq_some_iff] at hr
    obtain ⟨fr₀, hfr', slot, hslot, rfl⟩ := hr
    have hfr'' : s.frames.find? (·.id == f) = some fr₀ := hfr'
    have hpf : (fr₀.id == f) = true := List.find?_some (p := fun (x : Frame) => x.id == f) hfr''
    obtain ⟨k₀, hk, hfk⟩ := frame_get hfr'
    obtain ⟨g₀, hgk, hg⟩ := framesFit_get hfr hfk
    obtain ⟨t₀, R, hgt, -, hR, hR', -⟩ := slot_rw hg hslot t' v'
    have htt : t₀ = t := by
      simp only [Shadow.rootTree?, hk, hgk, hgt, bind, Option.bind_some] at ht
      exact Option.some.inj ht
    subst htt
    have hh' : h'.frames =
        h.frames.set k₀ { g₀ with locals := g₀.locals.setIfInBounds i (some t') } := by
      simp only [Shadow.withRootTree?, hk, hgk, hgt, bind, Option.bind_some, pure,
        Option.some.injEq] at hw
      rw [← hw]
    have hmap : (s.setRoot (.local_ f i) v').frames =
        s.frames.set k₀ { fr₀ with locals := fr₀.locals.setIfInBounds i (some v') } := by
      simp only [State.setRoot]
      refine map_ite_set (p := fun fr' => fr'.id == f) hfk hpf fun j y hy hpy => ?_
      exact key_unique hids hy hfk (by simp at hpf hpy; rw [hpy, hpf])
    rw [hh', List.getElem?_set] at h1
    rw [hmap, List.getElem?_set] at h2
    by_cases hkk : k₀ = k
    · subst hkk
      rw [if_pos rfl, if_pos (List.getElem?_eq_some_iff.1 hgk).1, Option.some.injEq] at h1
      rw [if_pos rfl, if_pos (List.getElem?_eq_some_iff.1 hfk).1, Option.some.injEq] at h2
      rw [hgk, Option.some.injEq] at h3
      rw [hfk, Option.some.injEq] at h4
      subst h1 h2 h3 h4
      rcases List.mem_append.1 (hR'.subset hx) with hx | hx
      · exact Or.inr ⟨hx, fun y hy => hR.symm.subset (List.mem_append_left _ hy)⟩
      · exact Or.inl (hR.symm.subset (List.mem_append_right _ hx))
    · rw [if_neg hkk, h3, Option.some.injEq] at h1
      rw [if_neg hkk, h4, Option.some.injEq] at h2
      subst h1 h2
      exact Or.inl hx
  | global a ty =>
    have hh' : h'.frames = h.frames := by
      simp only [Shadow.withRootTree?, bind, Option.bind_eq_some_iff, pure,
        Option.some.injEq] at hw
      obtain ⟨_, -, _, -, rfl⟩ := hw
      rfl
    rw [hh', h3, Option.some.injEq] at h1
    simp only [State.setRoot] at h2
    rw [h4, Option.some.injEq] at h2
    subst h1 h2
    exact Or.inl hx
  | held j =>
    have hh' : h'.frames = h.frames := by
      simp only [Shadow.withRootTree?, bind, Option.bind_eq_some_iff, pure,
        Option.some.injEq] at hw
      obtain ⟨_, -, rfl⟩ := hw
      rfl
    rw [hh', h3, Option.some.injEq] at h1
    simp only [State.setRoot] at h2
    rw [h4, Option.some.injEq] at h2
    subst h1 h2
    exact Or.inl hx

/-- **Per frame, writing a location**: a frame has after the write, by key,
the occurrences it had, or the written value's. -/
theorem tree_write_frames {h : Shadow} {s : State} (hfit : h.fits s = true)
    (hids : (s.frames.map (·.id)).Nodup) {l : Location} {w : Value} (hr : s.read? l = some w)
    {tw' : IdTree} {w' : Value} (hf' : tw'.fits w' = true) {h' : Shadow} {s' : State}
    (hwith : h.withTree? s l tw' = some h') (hwrite : s.write? l w' = some s') :
    ∀ (k : Nat) (g' : ShadowFrame) (fr' : Frame) (g : ShadowFrame) (fr : Frame),
      h'.frames[k]? = some g' → s'.frames[k]? = some fr' →
      h.frames[k]? = some g → s.frames[k]? = some fr → ∀ x ∈ g'.occurrences fr',
        (x.1, x.2.key) ∈ keys (g.occurrences fr) ∨ (x.1, x.2.key) ∈ keys (tw'.occurrences w') := by
  simp only [State.read?, bind, Option.bind_eq_some_iff] at hr
  obtain ⟨v, hv, hw⟩ := hr
  obtain ⟨t, ht, hft, -⟩ := root_read hfit hv
  obtain ⟨tw, v'', t'', -, -, hset, htset, -, -, R, hR, hR2⟩ :=
    path_write l.path hft hw hf' (write?_writable hwrite)
  have hw' : h.withRootTree? s l.root t'' = some h' := by
    simpa [Shadow.withTree?, ht, Location.indicesIn, hv, htset] using hwith
  have hs' : s' = s.setRoot l.root v'' := by
    simp only [State.write?, hv, hset, bind, Option.bind_some, pure, Option.some.injEq] at hwrite
    exact hwrite.symm
  subst hs'
  intro k g' fr' g fr h1 h2 h3 h4 x hx
  rcases root_write_frames hfit hids hv ht hw' k g' fr' g fr h1 h2 h3 h4 x hx with hx | ⟨hx, hall⟩
  · exact Or.inl (mem_keys hx)
  · rcases List.mem_append.1 (hR2.subset (mem_keys hx)) with hx | hx
    · exact Or.inr hx
    · obtain ⟨o, ho, e1, e2⟩ := mem_keys_iff.1 (hR.symm.subset (List.mem_append_right _ hx))
      have := mem_keys (hall o ho)
      rw [e1, e2] at this
      exact Or.inl this

/-! ## The state after a write -/

theorem setRoot_ids (s : State) (root : Root) (v : Value) :
    (s.setRoot root v).frames.map (·.id) = s.frames.map (·.id) := by
  cases root <;> simp [State.setRoot, Function.comp_def]
  intro a _
  split <;> rfl

theorem setRoot_resources (s : State) (root : Root) (v : Value) :
    (s.setRoot root v).storage.map (fun r => (r.address, r.type)) =
      s.storage.map (fun r => (r.address, r.type)) := by
  cases root <;> simp [State.setRoot, GlobalStorage.update, Function.comp_def]
  intro a _
  split <;> simp

end MoveBytecode.Proofs
