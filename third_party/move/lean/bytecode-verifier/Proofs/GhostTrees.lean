-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec
import Proofs.Values

/-!
# The ghost layer's identity trees

UNTRUSTED (checked by the kernel). Facts about the identity trees of the ghost
layer (`Spec.Semantics.Trace`):

- the occurrences of a tree that fits a value are the value's nodes in
  pre-order (`occurrences_snd`);
- `IdTree.build` gives each node the identity its path asks for, or the next
  fresh one with a creation event (`build_spec`, stated with the flat
  assignment `assign`);
- an ability a value has deeply (`Value.abilities`) is an ability of every
  node's type (`deep_copy`, `deep_drop`).
-/

namespace MoveBytecode.Proofs

/-! ## Nodes of a value -/

/-- The values of a value's nodes, in pre-order. -/
def subvalues (v : Value) : List Value := v.nodes.map (·.2)

theorem nodesFrom_snd : ∀ (i : Nat) (vs : List Value),
    (Value.nodesFrom i vs).map (·.2) = vs.flatMap subvalues
  | _, [] => by simp [Value.nodesFrom]
  | i, v :: vs => by
    simp [Value.nodesFrom, nodesFrom_snd (i + 1) vs, subvalues, Function.comp_def]

theorem subvalues_eq (v : Value) : subvalues v = v :: v.children.flatMap subvalues := by
  cases v <;> simp [subvalues, Value.nodes, Value.children, nodesFrom_snd]

theorem self_mem_subvalues (v : Value) : v ∈ subvalues v := by
  rw [subvalues_eq]; exact List.mem_cons_self ..

/-! ## Deep abilities -/

theorem AbilitySet.inter_subset_left' (a b : AbilitySet) : (a.inter b).subset a = true := by
  cases a; cases b; simp [MoveBytecode.AbilitySet.subset, MoveBytecode.AbilitySet.inter]; grind

theorem contentAbilities_sub {P : Program} :
    ∀ (vs : List Value) (c : Value), c ∈ vs →
      (Value.contentAbilities P vs).subset (c.abilities P).forContainer = true
  | [], _, h => by simp at h
  | v :: vs, c, h => by
    simp only [Value.contentAbilities]
    rcases List.mem_cons.1 h with rfl | h
    · exact AbilitySet.inter_subset_left' _ _
    · exact AbilitySet.subset_trans (AbilitySet.inter_subset_right _ _)
        (contentAbilities_sub vs c h)

/-- A value's abilities are among its type's. -/
theorem abilities_type {P : Program} (v : Value) :
    (v.abilities P).subset (v.type.abilities P) = true := by
  cases v <;> simp only [Value.abilities, Value.type] <;>
    first
      | exact AbilitySet.subset_refl _
      | exact AbilitySet.inter_subset_left' _ _
      | exact AbilitySet.subset_trans (AbilitySet.inter_subset_left' _ _)
          (AbilitySet.inter_subset_left' _ _)

/-- A container's abilities are among those its contents permit. -/
theorem abilities_child {P : Program} (v c : Value) (hc : c ∈ v.children) :
    (v.abilities P).subset (c.abilities P).forContainer = true := by
  cases v
  case signer a =>
    simp only [Value.children, List.mem_singleton] at hc
    subst hc; rfl
  case table s fs ks es =>
    simp only [Value.children, List.mem_append] at hc
    simp only [Value.abilities]
    rcases hc with hc | hc
    · exact AbilitySet.subset_trans (AbilitySet.inter_subset_left' _ _)
        (AbilitySet.subset_trans (AbilitySet.inter_subset_right _ _) (contentAbilities_sub _ c hc))
    · exact AbilitySet.subset_trans (AbilitySet.inter_subset_right _ _) (contentAbilities_sub _ c hc)
  all_goals simp only [Value.children, List.not_mem_nil] at hc
  all_goals simp only [Value.abilities]
  all_goals
    exact AbilitySet.subset_trans (AbilitySet.inter_subset_right _ _) (contentAbilities_sub _ c hc)

theorem copy_of_subset' {a b : AbilitySet} (h : a.subset b = true) (ha : a.copy = true) :
    b.copy = true := by
  cases a; cases b; simp_all [MoveBytecode.AbilitySet.subset]

theorem drop_of_subset' {a b : AbilitySet} (h : a.subset b = true) (ha : a.drop = true) :
    b.drop = true := by
  cases a; cases b; simp_all [MoveBytecode.AbilitySet.subset]

/-- A value with `copy` has `copy` all the way down: every node's type has it. -/
theorem deep_copy {P : Program} (v : Value) (h : (v.abilities P).copy = true) :
    ∀ w ∈ subvalues v, (w.type.abilities P).copy = true := by
  induction v using Value.rec
    (motive_2 := fun vs => ∀ c ∈ vs, (c.abilities P).copy = true →
      ∀ w ∈ subvalues c, (w.type.abilities P).copy = true) with
  | vector e vs ih =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact copy_of_subset' (abilities_type _) h
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      have hc' := copy_of_subset' (abilities_child _ c hc) h
      simp only [MoveBytecode.AbilitySet.forContainer] at hc'
      exact ih c hc hc' w hwc
  | struct s fs ih =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact copy_of_subset' (abilities_type _) h
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      have hc' := copy_of_subset' (abilities_child _ c hc) h
      simp only [MoveBytecode.AbilitySet.forContainer] at hc'
      exact ih c hc hc' w hwc
  | variant s tag fs ih =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact copy_of_subset' (abilities_type _) h
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      have hc' := copy_of_subset' (abilities_child _ c hc) h
      simp only [MoveBytecode.AbilitySet.forContainer] at hc'
      exact ih c hc hc' w hwc
  | closure f mask cs ps rs a ih =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact copy_of_subset' (abilities_type _) h
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      have hc' := copy_of_subset' (abilities_child _ c hc) h
      simp only [MoveBytecode.AbilitySet.forContainer] at hc'
      exact ih c hc hc' w hwc
  | table s fs ks es ihf _ ihe =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact copy_of_subset' (abilities_type _) h
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      have hc' := copy_of_subset' (abilities_child _ c hc) h
      simp only [MoveBytecode.AbilitySet.forContainer] at hc'
      simp only [Value.children, List.mem_append] at hc
      rcases hc with hc | hc
      · exact ihf c hc hc' w hwc
      · exact ihe c hc hc' w hwc
  | nil => rename_i hc _ _ _; simp at hc
  | cons x xs ihx ihxs =>
    rename_i c hc hab w hw
    rcases List.mem_cons.1 hc with rfl | hc
    · exact ihx hab w hw
    · exact ihxs c hc hab w hw
  | signer a =>
    intro w hw
    simp [Value.abilities, RuntimeType.abilities] at h
  | _ =>
    intro w hw
    rw [subvalues_eq] at hw
    simp only [Value.children, List.flatMap_nil, List.mem_singleton] at hw
    subst hw
    exact copy_of_subset' (abilities_type _) h

/-- A value with `drop` has `drop` all the way down: every node's type has it. -/
theorem deep_drop {P : Program} (v : Value) (h : (v.abilities P).drop = true) :
    ∀ w ∈ subvalues v, (w.type.abilities P).drop = true := by
  induction v using Value.rec
    (motive_2 := fun vs => ∀ c ∈ vs, (c.abilities P).drop = true →
      ∀ w ∈ subvalues c, (w.type.abilities P).drop = true) with
  | vector e vs ih =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact drop_of_subset' (abilities_type _) h
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      have hc' := drop_of_subset' (abilities_child _ c hc) h
      simp only [MoveBytecode.AbilitySet.forContainer] at hc'
      exact ih c hc hc' w hwc
  | struct s fs ih =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact drop_of_subset' (abilities_type _) h
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      have hc' := drop_of_subset' (abilities_child _ c hc) h
      simp only [MoveBytecode.AbilitySet.forContainer] at hc'
      exact ih c hc hc' w hwc
  | variant s tag fs ih =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact drop_of_subset' (abilities_type _) h
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      have hc' := drop_of_subset' (abilities_child _ c hc) h
      simp only [MoveBytecode.AbilitySet.forContainer] at hc'
      exact ih c hc hc' w hwc
  | closure f mask cs ps rs a ih =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact drop_of_subset' (abilities_type _) h
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      have hc' := drop_of_subset' (abilities_child _ c hc) h
      simp only [MoveBytecode.AbilitySet.forContainer] at hc'
      exact ih c hc hc' w hwc
  | table s fs ks es ihf _ ihe =>
    intro w hw
    rw [subvalues_eq] at hw
    rcases List.mem_cons.1 hw with rfl | hw
    · exact drop_of_subset' (abilities_type _) h
    · obtain ⟨c, hc, hwc⟩ := List.mem_flatMap.1 hw
      have hc' := drop_of_subset' (abilities_child _ c hc) h
      simp only [MoveBytecode.AbilitySet.forContainer] at hc'
      simp only [Value.children, List.mem_append] at hc
      rcases hc with hc | hc
      · exact ihf c hc hc' w hwc
      · exact ihe c hc hc' w hwc
  | nil => rename_i hc _ _ _; simp at hc
  | cons x xs ihx ihxs =>
    rename_i c hc hab w hw
    rcases List.mem_cons.1 hc with rfl | hc
    · exact ihx hab w hw
    · exact ihxs c hc hab w hw
  | signer a =>
    intro w hw
    simp [subvalues, Value.nodes] at hw
    rcases hw with rfl | rfl <;> rfl
  | _ =>
    intro w hw
    rw [subvalues_eq] at hw
    simp only [Value.children, List.flatMap_nil, List.mem_singleton] at hw
    subst hw
    exact drop_of_subset' (abilities_type _) h

/-! ## Occurrences of a fitting tree -/

mutual
/-- The occurrences of a tree that fits a value are the value's nodes. -/
theorem occurrences_snd : ∀ (t : IdTree) (v : Value), t.fits v = true →
    (t.occurrences v).map (·.2) = subvalues v
  | .node i cs, v, h => by
    simp only [IdTree.fits] at h
    simp only [IdTree.occurrences, List.map_cons]
    rw [occurrencesAll_snd cs v.children h, subvalues_eq]

theorem occurrencesAll_snd :
    ∀ (ts : List IdTree) (vs : List Value), IdTree.fitsAll ts vs = true →
    (IdTree.occurrencesAll ts vs).map (·.2) = vs.flatMap subvalues
  | [], [], _ => by simp [IdTree.occurrencesAll]
  | t :: ts, v :: vs, h => by
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    simp only [IdTree.occurrencesAll, List.map_append, List.flatMap_cons]
    rw [occurrences_snd t v h.1, occurrencesAll_snd ts vs h.2]
  | [], _ :: _, h => by simp [IdTree.fitsAll] at h
  | _ :: _, [], h => by simp [IdTree.fitsAll] at h
end

theorem occurrences_mem {t : IdTree} {v : Value} (h : t.fits v = true) {o : Nat × Value}
    (ho : o ∈ t.occurrences v) : o.2 ∈ subvalues v := by
  rw [← occurrences_snd t v h]
  exact List.mem_map_of_mem ho

theorem occurrencesAll_mem {ts : List IdTree} {vs : List Value} (h : IdTree.fitsAll ts vs = true)
    {o : Nat × Value} (ho : o ∈ IdTree.occurrencesAll ts vs) :
    ∃ v ∈ vs, o.2 ∈ subvalues v := by
  have := List.mem_map_of_mem (f := (·.2)) ho
  rw [occurrencesAll_snd ts vs h] at this
  exact List.mem_flatMap.1 this

theorem fitsAll_length : ∀ {ts : List IdTree} {vs : List Value}, IdTree.fitsAll ts vs = true →
    ts.length = vs.length
  | [], [], _ => rfl
  | _ :: _, _ :: _, h => by
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    simp [fitsAll_length h.2]
  | [], _ :: _, h => by simp [IdTree.fitsAll] at h
  | _ :: _, [], h => by simp [IdTree.fitsAll] at h

theorem fitsAll_append : ∀ {ts ts' : List IdTree} {vs vs' : List Value}, ts.length = vs.length →
    IdTree.fitsAll (ts ++ ts') (vs ++ vs') = (IdTree.fitsAll ts vs && IdTree.fitsAll ts' vs')
  | [], _, [], _, _ => by simp [IdTree.fitsAll]
  | t :: ts, _, v :: vs, _, h => by
    simp only [List.length_cons, Nat.add_right_cancel_iff] at h
    simp [IdTree.fitsAll, fitsAll_append h, Bool.and_assoc]
  | [], _, _ :: _, _, h => by simp at h
  | _ :: _, _, [], _, h => by simp at h

theorem occurrencesAll_append : ∀ {ts ts' : List IdTree} {vs vs' : List Value},
    ts.length = vs.length →
    IdTree.occurrencesAll (ts ++ ts') (vs ++ vs') =
      IdTree.occurrencesAll ts vs ++ IdTree.occurrencesAll ts' vs'
  | [], _, [], _, _ => by simp [IdTree.occurrencesAll]
  | t :: ts, _, v :: vs, _, h => by
    simp only [List.length_cons, Nat.add_right_cancel_iff] at h
    simp [IdTree.occurrencesAll, occurrencesAll_append h]
  | [], _, _ :: _, _, h => by simp at h
  | _ :: _, _, [], _, h => by simp at h

theorem fitsAll_reverse {ts : List IdTree} {vs : List Value} (h : IdTree.fitsAll ts vs = true) :
    IdTree.fitsAll ts.reverse vs.reverse = true := by
  induction ts generalizing vs with
  | nil => cases vs <;> simp_all [IdTree.fitsAll]
  | cons t ts ih =>
    cases vs with
    | nil => simp [IdTree.fitsAll] at h
    | cons v vs =>
      simp only [IdTree.fitsAll, Bool.and_eq_true] at h
      simp only [List.reverse_cons]
      rw [fitsAll_append (by simp [fitsAll_length h.2])]
      simp [ih h.2, IdTree.fitsAll, h.1]

/-- Reversing both lists permutes the occurrences. -/
theorem occurrencesAll_reverse {ts : List IdTree} {vs : List Value}
    (h : IdTree.fitsAll ts vs = true) :
    (IdTree.occurrencesAll ts.reverse vs.reverse).Perm (IdTree.occurrencesAll ts vs) := by
  induction ts generalizing vs with
  | nil => cases vs <;> simp [IdTree.occurrencesAll]
  | cons t ts ih =>
    cases vs with
    | nil => simp [IdTree.fitsAll] at h
    | cons v vs =>
      simp only [IdTree.fitsAll, Bool.and_eq_true] at h
      simp only [List.reverse_cons]
      rw [occurrencesAll_append (by simp [fitsAll_length h.2])]
      simp only [IdTree.occurrencesAll, List.append_nil]
      exact ((ih h.2).append_right _).trans List.perm_append_comm

/-! ## Building identity trees -/

/-- The identities `build` gives nodes listed in pre-order with their paths
below `p`: each keeps the identity `idOf` gives its path, or takes the next
fresh one, created by `c`. -/
def assign (idOf : Path → Option Nat) (c : Creator) (p : Path) :
    List (Path × Value) → Nat → List (Nat × Value) × Nat × List Event
  | [], n => ([], n, [])
  | (q, w) :: rest, n =>
    match idOf (p ++ q) with
    | some i =>
      let r := assign idOf c p rest n
      ((i, w) :: r.1, r.2.1, r.2.2)
    | none =>
      let r := assign idOf c p rest (n + 1)
      ((n, w) :: r.1, r.2.1, .created n w c :: r.2.2)

theorem assign_append (idOf : Path → Option Nat) (c : Creator) (p : Path) :
    ∀ (xs ys : List (Path × Value)) (n : Nat),
      assign idOf c p (xs ++ ys) n =
        ((assign idOf c p xs n).1 ++ (assign idOf c p ys (assign idOf c p xs n).2.1).1,
          (assign idOf c p ys (assign idOf c p xs n).2.1).2.1,
          (assign idOf c p xs n).2.2 ++ (assign idOf c p ys (assign idOf c p xs n).2.1).2.2)
  | [], ys, n => by simp [assign]
  | (q, w) :: xs, ys, n => by
    simp only [List.cons_append, assign]
    split <;> simp [assign_append idOf c p xs ys]

theorem assign_prefix (idOf : Path → Option Nat) (c : Creator) (p : Path) (i : Nat) :
    ∀ (xs : List (Path × Value)) (n : Nat),
      assign idOf c p (xs.map fun (q, w) => (i :: q, w)) n = assign idOf c (p ++ [i]) xs n
  | [], n => by simp [assign]
  | (q, w) :: xs, n => by
    simp only [List.map_cons, assign, List.append_assoc, List.cons_append, List.nil_append]
    cases idOf (p ++ i :: q) <;> simp [assign_prefix idOf c p i xs]

mutual
/-- `build` assigns identities as `assign` does, and its tree fits the value. -/
theorem build_spec (idOf : Path → Option Nat) (c : Creator) :
    ∀ (p : Path) (v : Value) (n : Nat),
      (IdTree.build idOf c p v n).1.fits v = true ∧
      ((IdTree.build idOf c p v n).1.occurrences v, (IdTree.build idOf c p v n).2.1,
        (IdTree.build idOf c p v n).2.2) = assign idOf c p v.nodes n
  | p, .vector e vs, n => by
    simp only [IdTree.build, IdTree.rootId, Value.nodes, assign, List.append_nil]
    cases idOf p with
    | some i =>
      obtain ⟨hf, he⟩ := buildAll_spec idOf c p 0 vs n
      rw [← he]; simp [IdTree.fits, IdTree.occurrences, Value.children, hf]
    | none =>
      obtain ⟨hf, he⟩ := buildAll_spec idOf c p 0 vs (n + 1)
      rw [← he]; simp [IdTree.fits, IdTree.occurrences, Value.children, hf]
  | p, .struct s fs, n => by
    simp only [IdTree.build, IdTree.rootId, Value.nodes, assign, List.append_nil]
    cases idOf p with
    | some i =>
      obtain ⟨hf, he⟩ := buildAll_spec idOf c p 0 fs n
      rw [← he]; simp [IdTree.fits, IdTree.occurrences, Value.children, hf]
    | none =>
      obtain ⟨hf, he⟩ := buildAll_spec idOf c p 0 fs (n + 1)
      rw [← he]; simp [IdTree.fits, IdTree.occurrences, Value.children, hf]
  | p, .variant s tag fs, n => by
    simp only [IdTree.build, IdTree.rootId, Value.nodes, assign, List.append_nil]
    cases idOf p with
    | some i =>
      obtain ⟨hf, he⟩ := buildAll_spec idOf c p 0 fs n
      rw [← he]; simp [IdTree.fits, IdTree.occurrences, Value.children, hf]
    | none =>
      obtain ⟨hf, he⟩ := buildAll_spec idOf c p 0 fs (n + 1)
      rw [← he]; simp [IdTree.fits, IdTree.occurrences, Value.children, hf]
  | p, .bool b, n => by
    simp only [IdTree.build, IdTree.rootId, Value.nodes, assign, List.append_nil]
    cases idOf p <;> simp [IdTree.fits, IdTree.fitsAll, IdTree.occurrences,
      IdTree.occurrencesAll, Value.children]
  | p, .int t x, n => by
    simp only [IdTree.build, IdTree.rootId, Value.nodes, assign, List.append_nil]
    cases idOf p <;> simp [IdTree.fits, IdTree.fitsAll, IdTree.occurrences,
      IdTree.occurrencesAll, Value.children]
  | p, .address a, n => by
    simp only [IdTree.build, IdTree.rootId, Value.nodes, assign, List.append_nil]
    cases idOf p <;> simp [IdTree.fits, IdTree.fitsAll, IdTree.occurrences,
      IdTree.occurrencesAll, Value.children]
  | p, .signer a, n => by
    simp only [IdTree.build, IdTree.rootId, Value.nodes, assign, List.append_nil]
    cases idOf p <;> cases idOf (p ++ [0]) <;> simp [IdTree.fits, IdTree.fitsAll, IdTree.occurrences,
      IdTree.occurrencesAll, Value.children]
  | p, .ref r, n => by
    simp only [IdTree.build, IdTree.rootId, Value.nodes, assign, List.append_nil]
    cases idOf p <;> simp [IdTree.fits, IdTree.fitsAll, IdTree.occurrences,
      IdTree.occurrencesAll, Value.children]
  | p, .closure f mask cs ps rs a, n => by
    simp only [IdTree.build, IdTree.rootId, Value.nodes, assign, List.append_nil]
    cases idOf p with
    | some i =>
      obtain ⟨hf, he⟩ := buildAll_spec idOf c p 0 cs n
      rw [← he]; simp [IdTree.fits, IdTree.occurrences, Value.children, hf]
    | none =>
      obtain ⟨hf, he⟩ := buildAll_spec idOf c p 0 cs (n + 1)
      rw [← he]; simp [IdTree.fits, IdTree.occurrences, Value.children, hf]
  | p, .table s fs ks es, n => by
    simp only [IdTree.build, IdTree.rootId, Value.nodes, assign, List.append_nil]
    cases idOf p with
    | some i =>
      obtain ⟨hf, he⟩ := buildAll_spec idOf c p 0 fs n
      obtain ⟨hf', he'⟩ := buildAll_spec idOf c p fs.length es (IdTree.buildAll idOf c p 0 fs n).2.1
      have hl := fitsAll_length hf
      rw [assign_append, ← he, ← he']
      simp [IdTree.fits, IdTree.occurrences, Value.children, fitsAll_append hl,
        occurrencesAll_append hl, hf, hf']
    | none =>
      obtain ⟨hf, he⟩ := buildAll_spec idOf c p 0 fs (n + 1)
      obtain ⟨hf', he'⟩ := buildAll_spec idOf c p fs.length es
        (IdTree.buildAll idOf c p 0 fs (n + 1)).2.1
      have hl := fitsAll_length hf
      simp only [assign_append]
      rw [← he, ← he']
      simp [IdTree.fits, IdTree.occurrences, Value.children, fitsAll_append hl,
        occurrencesAll_append hl, hf, hf']

theorem buildAll_spec (idOf : Path → Option Nat) (c : Creator) (p : Path) :
    ∀ (i : Nat) (vs : List Value) (n : Nat),
      IdTree.fitsAll (IdTree.buildAll idOf c p i vs n).1 vs = true ∧
      (IdTree.occurrencesAll (IdTree.buildAll idOf c p i vs n).1 vs,
        (IdTree.buildAll idOf c p i vs n).2.1, (IdTree.buildAll idOf c p i vs n).2.2) =
        assign idOf c p (Value.nodesFrom i vs) n
  | _, [], n => by
    simp [IdTree.buildAll, IdTree.fitsAll, IdTree.occurrencesAll, Value.nodesFrom, assign]
  | i, v :: vs, n => by
    obtain ⟨hf, he⟩ := build_spec idOf c (p ++ [i]) v n
    obtain ⟨hf', he'⟩ := buildAll_spec idOf c p (i + 1) vs
      (IdTree.build idOf c (p ++ [i]) v n).2.1
    simp only [IdTree.buildAll, IdTree.fitsAll, IdTree.occurrencesAll, Value.nodesFrom,
      assign_append, assign_prefix]
    rw [← he, ← he']
    simp [hf, hf']
end

/-! ## Fresh trees -/

theorem assign_none (c : Creator) (p : Path) : ∀ (xs : List (Path × Value)) (n : Nat),
    assign (fun _ => none) c p xs n =
      ((List.range' n xs.length).zip (xs.map (·.2)), n + xs.length,
        ((List.range' n xs.length).zip (xs.map (·.2))).map fun (i, w) => Event.created i w c)
  | [], n => by simp [assign]
  | (q, w) :: xs, n => by
    simp only [assign, assign_none c p xs (n + 1), List.length_cons, List.range'_succ,
      List.map_cons, List.zip_cons_cons]
    simp [Nat.add_assoc, Nat.add_comm 1]

/-- A value created whole: each node gets the next fresh identity, in
pre-order, with a creation event. -/
theorem fresh_spec (c : Creator) (v : Value) (n : Nat) :
    (IdTree.fresh c v n).1.fits v = true ∧
    (IdTree.fresh c v n).1.occurrences v =
      (List.range' n (subvalues v).length).zip (subvalues v) ∧
    (IdTree.fresh c v n).2.1 = n + (subvalues v).length ∧
    (IdTree.fresh c v n).2.2 =
      ((List.range' n (subvalues v).length).zip (subvalues v)).map
        fun (i, w) => Event.created i w c := by
  obtain ⟨hf, he⟩ := build_spec (fun _ => none) c [] v n
  rw [assign_none] at he
  simp only [IdTree.fresh, Prod.mk.injEq] at he ⊢
  obtain ⟨h1, h2, h3⟩ := he
  have hlen : v.nodes.length = (subvalues v).length := by simp [subvalues]
  refine ⟨hf, ?_, ?_, ?_⟩
  · rw [h1, hlen]; rfl
  · rw [h2, hlen]
  · rw [h3, hlen]; rfl

end MoveBytecode.Proofs
