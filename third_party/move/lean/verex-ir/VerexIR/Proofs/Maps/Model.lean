-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Std

/-!
# Typed intrinsic maps

The specification model stores ordinary Lean keys and values. It has no
runtime values, codecs, layouts, handles, or ghost slots. The encoding bridge
is proved separately. Sequence maps retain insertion order and swap-remove;
ordered maps insert in key order and remove without reordering other entries.
-/

/-- A typed map is its enumeration of key/value pairs. Validity is a
separate hypothesis so the model also gives a meaning to invalid inputs. -/
abbrev VerexIR.SpecMap (K V : Type) := List (K × V)

namespace VerexIR.SpecMap

/-- A total key order, with the laws needed by ordered maps. -/
structure KeyOrder (K : Type) where
  compare : K → K → Ordering
  lawful : Std.LawfulEqCmp compare
  trans : Std.TransCmp compare

attribute [instance] KeyOrder.lawful KeyOrder.trans

variable {K V : Type} [DecidableEq K]

/-- Whether a key has an entry. -/
def containsB : List (K × V) → K → Bool
  | [], _ => false
  | entry :: rest, key => decide (entry.1 = key) || containsB rest key

/-- The value of a key's first entry; junk without one. -/
def get [Inhabited V] : List (K × V) → K → V
  | [], _ => default
  | entry :: rest, key => if entry.1 = key then entry.2 else get rest key

/-- The position of a key's first entry; the length without one. -/
def indexOf : List (K × V) → K → Nat
  | [], _ => 0
  | entry :: rest, key => if entry.1 = key then 0 else indexOf rest key + 1

/-- The keys are distinct. -/
def Distinct (entries : List (K × V)) : Prop := (entries.map Prod.fst).Nodup

/-- Replace the value of a key's first entry. -/
def replaceFirst (key : K) (value : V) : List (K × V) → List (K × V)
  | [] => []
  | entry :: rest =>
      if entry.1 = key then (key, value) :: rest else entry :: replaceFirst key value rest

/-- Remove the entry at a position by moving the last entry into it. -/
def swapRemove (entries : List (K × V)) (index : Nat) : List (K × V) :=
  match (entries.drop (index + 1)).reverse with
  | [] => entries.take index
  | last :: middle => entries.take index ++ last :: middle.reverse

/-! ## The sequence discipline -/

/-- Update in place, or append. -/
def seqSet (entries : List (K × V)) (key : K) (value : V) : List (K × V) :=
  if containsB entries key then replaceFirst key value entries else entries ++ [(key, value)]

/-- Remove a key's first entry by swapping the last entry into its place. -/
def seqDel (entries : List (K × V)) (key : K) : List (K × V) :=
  if containsB entries key then swapRemove entries (indexOf entries key) else entries

/-! ## Membership -/

theorem containsB_iff (entries : List (K × V)) (key : K) :
    containsB entries key = true ↔ ∃ value, (key, value) ∈ entries := by
  induction entries with
  | nil => simp [containsB]
  | cons entry rest ih =>
      obtain ⟨k, v⟩ := entry
      simp only [containsB, Bool.or_eq_true, decide_eq_true_eq, ih, List.mem_cons,
        Prod.mk.injEq]
      constructor
      · rintro (rfl | ⟨w, h⟩)
        · exact ⟨v, .inl ⟨rfl, rfl⟩⟩
        · exact ⟨w, .inr h⟩
      · rintro ⟨w, ⟨rfl, rfl⟩ | h⟩
        · exact .inl rfl
        · exact .inr ⟨w, h⟩

@[simp] theorem containsB_nil (key : K) : containsB ([] : List (K × V)) key = false :=
  rfl

@[simp] theorem containsB_cons (entry : K × V)
    (rest : List (K × V)) (key : K) :
    containsB (entry :: rest) key = (decide (entry.1 = key) || containsB rest key) := rfl

theorem containsB_append (left right : List (K × V)) (key : K) :
    containsB (left ++ right) key = (containsB left key || containsB right key) := by
  induction left with
  | nil => simp [containsB]
  | cons entry rest ih => simp [containsB, ih, Bool.or_assoc]

theorem containsB_of_perm {left right : List (K × V)} (perm : left.Perm right) (key : K) :
    containsB left key = containsB right key := by
  apply Bool.eq_iff_iff.mpr
  simp only [containsB_iff]
  exact exists_congr fun _ => perm.mem_iff

theorem mem_of_get [Inhabited V] (entries : List (K × V)) (key : K) (present : containsB entries key = true) :
    (key, get entries key) ∈ entries := by
  induction entries with
  | nil => simp [containsB] at present
  | cons entry rest ih =>
      obtain ⟨k, v⟩ := entry
      by_cases h : k = key
      · subst h; simp [get]
      · simp only [containsB, h, decide_false, Bool.false_or] at present
        simp [get, h, ih present]

theorem get_of_mem [Inhabited V] {entries : List (K × V)} (distinct : Distinct entries) {key : K} {value : V}
    (mem : (key, value) ∈ entries) : get entries key = value := by
  induction entries with
  | nil => simp at mem
  | cons entry rest ih =>
      obtain ⟨k, v⟩ := entry
      simp only [Distinct, List.map_cons, List.nodup_cons, List.mem_map] at distinct
      rcases List.mem_cons.mp mem with h | h
      · cases h; simp [get]
      · have hk : k ≠ key := fun e => distinct.1 ⟨(key, value), h, by simp [e]⟩
        simp only [get, hk, if_false]
        exact ih distinct.2 h

theorem get_of_perm [Inhabited V] {left right : List (K × V)} (perm : left.Perm right) (distinct : Distinct left)
    (key : K) : get left key = get right key := by
  by_cases present : containsB left key = true
  · have mem := mem_of_get left key present
    have distinct' : Distinct right := (perm.map Prod.fst).nodup_iff.mp distinct
    exact (get_of_mem distinct' (perm.mem_iff.mp mem)).symm
  · have absent : containsB right key = false := by
      rw [← containsB_of_perm perm]; simpa using present
    simp only [Bool.not_eq_true] at present
    clear perm distinct
    have none (entries : List (K × V)) (h : containsB entries key = false) : get entries key = default := by
      induction entries with
      | nil => rfl
      | cons entry rest ih =>
          simp only [containsB, Bool.or_eq_false_iff, decide_eq_false_iff_not] at h
          simp [get, h.1, ih h.2]
    rw [none left present, none right absent]

/-! ## Lemmas of the sequence discipline -/

theorem containsB_replaceFirst (entries : List (K × V)) (key : K) (value : V) (probe : K) :
    containsB (replaceFirst key value entries) probe = containsB entries probe := by
  induction entries with
  | nil => rfl
  | cons entry rest ih =>
      by_cases h : entry.1 = key
      · simp [replaceFirst, containsB, h]
      · simp [replaceFirst, containsB, h, ih]

theorem get_replaceFirst [Inhabited V] (entries : List (K × V)) (key : K) (value : V) (probe : K)
    (present : containsB entries key = true) :
    get (replaceFirst key value entries) probe = if probe = key then value else get entries probe := by
  induction entries with
  | nil => simp [containsB] at present
  | cons entry rest ih =>
      by_cases h : entry.1 = key
      · by_cases p : probe = key
        · subst p; simp [replaceFirst, get, h]
        · have differs : key ≠ probe := fun e => p e.symm
          simp [replaceFirst, get, h, p, differs]
      · simp only [containsB, h, decide_false, Bool.false_or] at present
        by_cases p : entry.1 = probe
        · have : probe ≠ key := fun e => h (p.trans e)
          simp [replaceFirst, get, p, this]
        · simp [replaceFirst, get, h, p, ih present]

theorem length_replaceFirst (entries : List (K × V)) (key : K) (value : V) :
    (replaceFirst key value entries).length = entries.length := by
  induction entries with
  | nil => rfl
  | cons entry rest ih => by_cases h : entry.1 = key <;> simp [replaceFirst, h, ih]

theorem map_fst_replaceFirst (entries : List (K × V)) (key : K) (value : V) :
    (replaceFirst key value entries).map Prod.fst = entries.map Prod.fst := by
  induction entries with
  | nil => rfl
  | cons entry rest ih =>
      by_cases h : entry.1 = key
      · simp [replaceFirst, h]
      · simp [replaceFirst, h, ih]

theorem get_append_absent [Inhabited V] (entries : List (K × V)) (key : K) (value : V) (probe : K)
    (absent : containsB entries key = false) :
    get (entries ++ [(key, value)]) probe = if probe = key then value else get entries probe := by
  induction entries with
  | nil =>
      by_cases p : probe = key
      · subst p; simp [get]
      · simp [get, Ne.symm p, p]
  | cons entry rest ih =>
      simp only [containsB, Bool.or_eq_false_iff, decide_eq_false_iff_not] at absent
      by_cases p : entry.1 = probe
      · have : probe ≠ key := fun e => absent.1 (p.trans e)
        simp [get, p, this]
      · simp [get, p, ih absent.2]

@[simp] theorem containsB_seqSet (entries : List (K × V)) (key : K) (value : V) (probe : K) :
    containsB (seqSet entries key value) probe = (decide (key = probe) || containsB entries probe) := by
  unfold seqSet
  split
  · rename_i present
    rw [containsB_replaceFirst]
    by_cases h : key = probe
    · subst h; simp [present]
    · simp [h]
  · simp [containsB_append, containsB, Bool.or_comm]

@[simp] theorem get_seqSet [Inhabited V] (entries : List (K × V)) (key : K) (value : V) (probe : K) :
    get (seqSet entries key value) probe = if probe = key then value else get entries probe := by
  unfold seqSet
  split
  · rename_i present; exact get_replaceFirst entries key value probe present
  · rename_i absent; exact get_append_absent entries key value probe (by simpa using absent)

@[simp] theorem length_seqSet (entries : List (K × V)) (key : K) (value : V) :
    (seqSet entries key value).length =
      if containsB entries key then entries.length else entries.length + 1 := by
  unfold seqSet
  split <;> simp [length_replaceFirst]

theorem distinct_seqSet {entries : List (K × V)} (distinct : Distinct entries) (key : K) (value : V) :
    Distinct (seqSet entries key value) := by
  unfold seqSet Distinct at *
  split
  · rw [map_fst_replaceFirst]; exact distinct
  · rename_i absent
    simp only [List.map_append, List.map_cons, List.map_nil]
    refine List.nodup_append.mpr ⟨distinct, by simp, ?_⟩
    intro a ha b hb
    simp only [List.mem_singleton] at hb
    subst hb
    rintro rfl
    obtain ⟨⟨k, v⟩, mem, rfl⟩ := List.mem_map.mp ha
    exact absent ((containsB_iff _ _).mpr ⟨v, mem⟩)

omit [DecidableEq K] in
theorem swapRemove_perm (entries : List (K × V)) (index : Nat) :
    (swapRemove entries index).Perm (entries.eraseIdx index) := by
  rw [List.eraseIdx_eq_take_drop_succ]
  unfold swapRemove
  split
  · rename_i empty
    have : entries.drop (index + 1) = [] := List.reverse_eq_nil_iff.mp empty
    simp [this]
  · rename_i last middle split
    have tail : entries.drop (index + 1) = middle.reverse ++ [last] := by
      rw [← List.reverse_reverse (entries.drop (index + 1)), split]; simp
    rw [tail]
    refine List.Perm.append_left _ ?_
    exact List.perm_append_comm (l₁ := [last]) (l₂ := middle.reverse)

theorem getElem_indexOf (entries : List (K × V)) (key : K)
    (present : containsB entries key = true) :
    ∃ bound : indexOf entries key < entries.length, (entries[indexOf entries key]'bound).1 = key := by
  induction entries with
  | nil => simp [containsB] at present
  | cons entry rest ih =>
      by_cases h : entry.1 = key
      · exact ⟨by simp [indexOf, h], by simp [indexOf, h]⟩
      · simp only [containsB, h, decide_false, Bool.false_or] at present
        obtain ⟨bound, found⟩ := ih present
        exact ⟨by simp [indexOf, h]; omega, by simpa [indexOf, h] using found⟩

theorem seqDel_perm (entries : List (K × V)) (key : K) (present : containsB entries key = true) :
    (seqDel entries key).Perm (entries.eraseIdx (indexOf entries key)) := by
  simp only [seqDel, present, if_true]
  exact swapRemove_perm entries _

omit [DecidableEq K] in
theorem mem_eraseIdx_iff_of_distinct {entries : List (K × V)} (distinct : Distinct entries)
    (index : Nat) (bound : index < entries.length) (entry : K × V) :
    entry ∈ entries.eraseIdx index ↔ entry ∈ entries ∧ entry.1 ≠ (entries[index]'bound).1 := by
  rw [List.mem_eraseIdx_iff_getElem]
  constructor
  · rintro ⟨i, hi, differs, rfl⟩
    refine ⟨List.getElem_mem hi, fun same => differs ?_⟩
    exact (List.getElem_inj (i := i) (j := index) (h₀ := by simpa using hi)
      (h₁ := by simpa using bound) distinct).mp (by simpa using same)
  · rintro ⟨mem, differs⟩
    obtain ⟨i, hi, rfl⟩ := List.getElem_of_mem mem
    exact ⟨i, hi, fun e => differs (by subst e; rfl), rfl⟩

theorem length_seqDel (entries : List (K × V)) (key : K) :
    (seqDel entries key).length =
      if containsB entries key then entries.length - 1 else entries.length := by
  by_cases present : containsB entries key = true
  · obtain ⟨bound, -⟩ := getElem_indexOf entries key present
    rw [(seqDel_perm entries key present).length_eq, List.length_eraseIdx_of_lt bound]
    simp [present]
  · simp [seqDel, present]

theorem distinct_seqDel {entries : List (K × V)} (distinct : Distinct entries) (key : K) :
    Distinct (seqDel entries key) := by
  by_cases present : containsB entries key = true
  · unfold Distinct
    exact ((seqDel_perm entries key present).map Prod.fst).nodup_iff.mpr
      (List.Nodup.sublist ((List.eraseIdx_sublist _ _).map Prod.fst) distinct)
  · simpa [seqDel, present] using distinct

theorem containsB_seqDel {entries : List (K × V)} (distinct : Distinct entries) (key probe : K) :
    containsB (seqDel entries key) probe = (!decide (key = probe) && containsB entries probe) := by
  by_cases present : containsB entries key = true
  · obtain ⟨bound, found⟩ := getElem_indexOf entries key present
    rw [containsB_of_perm (seqDel_perm entries key present)]
    apply Bool.eq_iff_iff.mpr
    simp only [containsB_iff, Bool.and_eq_true, Bool.not_eq_true', decide_eq_false_iff_not]
    simp only [mem_eraseIdx_iff_of_distinct distinct _ bound, found]
    constructor
    · rintro ⟨value, mem, differs⟩; exact ⟨fun e => differs e.symm, value, mem⟩
    · rintro ⟨differs, value, mem⟩; exact ⟨value, mem, fun e => differs e.symm⟩
  · simp only [seqDel, present]
    by_cases h : key = probe
    · subst h; simpa using present
    · simp [h]

theorem get_seqDel [Inhabited V] {entries : List (K × V)} (distinct : Distinct entries) (key probe : K)
    (differs : probe ≠ key) : get (seqDel entries key) probe = get entries probe := by
  by_cases present : containsB entries key = true
  · obtain ⟨bound, found⟩ := getElem_indexOf entries key present
    have perm := seqDel_perm entries key present
    have distinctErased : Distinct (entries.eraseIdx (indexOf entries key)) :=
      List.Nodup.sublist ((List.eraseIdx_sublist _ _).map Prod.fst) distinct
    rw [get_of_perm perm (distinct_seqDel distinct key)]
    by_cases probed : containsB entries probe = true
    · have mem := mem_of_get entries probe probed
      have mem' : (probe, get entries probe) ∈ entries.eraseIdx (indexOf entries key) :=
        (mem_eraseIdx_iff_of_distinct distinct _ bound _).mpr ⟨mem, by rw [found]; exact differs⟩
      exact get_of_mem distinctErased mem'
    · have absent : containsB (entries.eraseIdx (indexOf entries key)) probe = false := by
        rw [← containsB_of_perm perm, containsB_seqDel distinct]; simp [probed]
      have none (entries : List (K × V)) (h : containsB entries probe = false) :
          get entries probe = default := by
        induction entries with
        | nil => rfl
        | cons entry rest ih =>
            simp only [containsB, Bool.or_eq_false_iff, decide_eq_false_iff_not] at h
            simp [get, h.1, ih h.2]
      rw [none _ absent, none _ (by simpa using probed)]
  · simp [seqDel, present]

/-! ## The ordered discipline -/

/-- The keys are strictly ascending. -/
def Ascending (rank : KeyOrder K) (entries : List (K × V)) : Prop :=
  entries.Pairwise fun earlier later => rank.compare earlier.1 later.1 = .lt

/-- Update in place, or insert at the key's position. -/
def ordSet (rank : KeyOrder K) (key : K) (value : V) : List (K × V) → List (K × V)
  | [] => [(key, value)]
  | entry :: rest =>
      match rank.compare key entry.1 with
      | .lt => (key, value) :: entry :: rest
      | .eq => (key, value) :: rest
      | .gt => entry :: ordSet rank key value rest

/-- Remove a key's first entry. -/
def ordDel (key : K) : List (K × V) → List (K × V)
  | [] => []
  | entry :: rest => if entry.1 = key then rest else entry :: ordDel key rest

/-! ## Lemmas of the ordered discipline -/

section OrderedLaws
variable (rank : KeyOrder K)

omit [DecidableEq K] in
theorem ne_of_order_lt {a b : K} (h : rank.compare a b = .lt) : a ≠ b := by
  rintro rfl; rw [Std.ReflCmp.compare_self (cmp := rank.compare)] at h; cases h

omit [DecidableEq K] in
theorem distinct_of_ascending {entries : List (K × V)} (ascending : Ascending rank entries) :
    Distinct entries := by
  unfold Distinct
  rw [List.Nodup, List.pairwise_map]
  exact ascending.imp fun h => ne_of_order_lt rank h

omit [DecidableEq K] in
theorem mem_ordSet {key : K} {value : V} {entries : List (K × V)} {entry : K × V}
    (mem : entry ∈ ordSet rank key value entries) : entry = (key, value) ∨ entry ∈ entries := by
  induction entries with
  | nil => simpa [ordSet] using mem
  | cons head rest ih =>
      simp only [ordSet] at mem
      split at mem
      · rcases List.mem_cons.mp mem with h | h
        · exact .inl h
        · exact .inr h
      · rcases List.mem_cons.mp mem with h | h
        · exact .inl h
        · exact .inr (List.mem_cons_of_mem _ h)
      · rcases List.mem_cons.mp mem with h | h
        · exact .inr (h ▸ List.mem_cons_self)
        · rcases ih h with h | h
          · exact .inl h
          · exact .inr (List.mem_cons_of_mem _ h)

theorem containsB_ordSet (entries : List (K × V)) (key : K) (value : V) (probe : K) :
    containsB (ordSet rank key value entries) probe = (decide (key = probe) || containsB entries probe) := by
  induction entries with
  | nil => simp [ordSet, containsB]
  | cons head rest ih =>
      simp only [ordSet]
      split
      · simp [containsB]
      · rename_i h
        have : head.1 = key := (Std.LawfulEqCmp.eq_of_compare h).symm
        simp only [containsB, this]
        cases decide (key = probe) <;> simp
      · simp only [containsB, ih]
        cases decide (key = probe) <;> cases decide (head.1 = probe) <;> simp

theorem get_ordSet [Inhabited V] (entries : List (K × V)) (key : K) (value : V) (probe : K) :
    get (ordSet rank key value entries) probe = if probe = key then value else get entries probe := by
  induction entries with
  | nil =>
      by_cases p : probe = key
      · subst p; simp [ordSet, get]
      · simp [ordSet, get, Ne.symm p, p]
  | cons head rest ih =>
      simp only [ordSet]
      split
      · by_cases p : probe = key
        · subst p; simp [get]
        · simp [get, Ne.symm p, p]
      · rename_i h
        have hk : head.1 = key := (Std.LawfulEqCmp.eq_of_compare h).symm
        by_cases p : probe = key
        · subst p; simp [get]
        · simp [get, Ne.symm p, p, hk]
      · rename_i h
        have hk : head.1 ≠ key := fun e => by rw [e, Std.ReflCmp.compare_self (cmp := rank.compare)] at h; cases h
        by_cases p : head.1 = probe
        · have : probe ≠ key := fun e => hk (p.trans e)
          simp [get, p, this]
        · simp [get, p, ih]

omit [DecidableEq K] in
theorem ascending_ordSet {entries : List (K × V)} (ascending : Ascending rank entries)
    (key : K) (value : V) : Ascending rank (ordSet rank key value entries) := by
  induction entries with
  | nil => simp [ordSet, Ascending]
  | cons head rest ih =>
      have tail := (List.pairwise_cons.mp ascending)
      simp only [ordSet]
      split
      · rename_i h
        refine List.pairwise_cons.mpr ⟨?_, ascending⟩
        intro x mem
        rcases List.mem_cons.mp mem with rfl | mem
        · exact h
        · exact Std.TransCmp.lt_trans h (tail.1 x mem)
      · rename_i h
        have hk : head.1 = key := (Std.LawfulEqCmp.eq_of_compare h).symm
        refine List.pairwise_cons.mpr ⟨?_, tail.2⟩
        intro x mem
        simpa [hk] using tail.1 x mem
      · rename_i h
        refine List.pairwise_cons.mpr ⟨?_, ih tail.2⟩
        intro x mem
        rcases mem_ordSet rank mem with rfl | mem
        · exact Std.OrientedCmp.gt_iff_lt.mp h
        · exact tail.1 x mem

theorem containsB_false_of_below {entries : List (K × V)}
    {head : K × V} {key : K}
    (below : rank.compare key head.1 = .lt) (sorted : Ascending rank (head :: entries)) :
    containsB (head :: entries) key = false := by
  have tail := List.pairwise_cons.mp sorted
  apply Bool.eq_false_iff.mpr
  intro present
  obtain ⟨v, mem⟩ := (containsB_iff _ _).mp present
  rcases List.mem_cons.mp mem with h | h
  · rw [← h] at below; rw [Std.ReflCmp.compare_self (cmp := rank.compare)] at below; cases below
  · have := Std.TransCmp.lt_trans below (tail.1 _ h)
    rw [Std.ReflCmp.compare_self (cmp := rank.compare)] at this; cases this

theorem length_ordSet {entries : List (K × V)} (ascending : Ascending rank entries) (key : K) (value : V) :
    (ordSet rank key value entries).length =
      if containsB entries key then entries.length else entries.length + 1 := by
  induction entries with
  | nil => simp [ordSet, containsB]
  | cons head rest ih =>
      have tail := List.pairwise_cons.mp ascending
      simp only [ordSet]
      split
      · rename_i h
        rw [containsB_false_of_below rank h ascending]; simp
      · rename_i h
        have hk : head.1 = key := (Std.LawfulEqCmp.eq_of_compare h).symm
        simp [containsB, hk]
      · rename_i h
        have hk : head.1 ≠ key := fun e => by rw [e, Std.ReflCmp.compare_self (cmp := rank.compare)] at h; cases h
        simp only [List.length_cons, ih tail.2, containsB, hk, decide_false, Bool.false_or]
        split <;> rfl

theorem sublist_ordDel (key : K) (entries : List (K × V)) : (ordDel key entries).Sublist entries := by
  induction entries with
  | nil => exact List.Sublist.slnil
  | cons head rest ih =>
      simp only [ordDel]
      split
      · exact List.sublist_cons_self _ _
      · exact ih.cons_cons _

theorem ascending_ordDel {entries : List (K × V)} (ascending : Ascending rank entries) (key : K) :
    Ascending rank (ordDel key entries) :=
  ascending.sublist (sublist_ordDel key entries)

end OrderedLaws

theorem containsB_ordDel {entries : List (K × V)} (distinct : Distinct entries) (key probe : K) :
    containsB (ordDel key entries) probe = (!decide (key = probe) && containsB entries probe) := by
  induction entries with
  | nil => simp [ordDel, containsB]
  | cons head rest ih =>
      simp only [Distinct, List.map_cons, List.nodup_cons, List.mem_map] at distinct
      simp only [ordDel]
      split
      · rename_i hk
        have absent : containsB rest key = false := by
          apply Bool.eq_false_iff.mpr
          intro present
          obtain ⟨v, mem⟩ := (containsB_iff _ _).mp present
          exact distinct.1 ⟨(key, v), mem, by simp [hk]⟩
        by_cases p : key = probe
        · subst p; simpa using absent
        · simp [containsB, hk, p]
      · rename_i hk
        simp only [containsB, ih distinct.2]
        by_cases p : key = probe
        · subst p; simp [hk]
        · simp [p]

theorem get_ordDel [Inhabited V] (entries : List (K × V)) (key probe : K) (differs : probe ≠ key) :
    get (ordDel key entries) probe = get entries probe := by
  induction entries with
  | nil => rfl
  | cons head rest ih =>
      simp only [ordDel]
      split
      · rename_i hk
        have : head.1 ≠ probe := fun e => differs (e ▸ hk)
        simp [get, this]
      · simp only [get, ih]

/-- Removing a key removes the entry at its position; nothing without one. -/
theorem ordDel_eq_eraseIdx (key : K) (entries : List (K × V)) :
    ordDel key entries = entries.eraseIdx (indexOf entries key) := by
  induction entries with
  | nil => rfl
  | cons entry rest ih =>
      simp only [ordDel, indexOf]
      split
      · rfl
      · rw [ih]; rfl

theorem length_ordDel (entries : List (K × V)) (key : K) :
    (ordDel key entries).length = if containsB entries key then entries.length - 1 else entries.length := by
  induction entries with
  | nil => simp [ordDel, containsB]
  | cons head rest ih =>
      simp only [ordDel]
      split
      · rename_i hk; simp [containsB, hk]
      · rename_i hk
        simp only [List.length_cons, ih, containsB, hk, decide_false, Bool.false_or]
        split
        · rename_i present
          have : 0 < rest.length := by
            cases rest with
            | nil => simp [containsB] at present
            | cons _ _ => simp
          omega
        · rfl

/-- The source owner's choice of enumeration and removal semantics. -/
inductive Discipline (K : Type) where
  | sequence
  | ordered (order : KeyOrder K)

def Discipline.set : Discipline K → SpecMap K V → K → V → SpecMap K V
  | .sequence => seqSet
  | .ordered order => fun entries key value => ordSet order key value entries

def Discipline.del : Discipline K → SpecMap K V → K → SpecMap K V
  | .sequence => seqDel
  | .ordered _ => fun entries key => ordDel key entries

def Valid : Discipline K → SpecMap K V → Prop
  | .sequence => Distinct
  | .ordered order => Ascending order

def size (map : SpecMap K V) : Int := map.length

def hasKey (map : SpecMap K V) (key : K) : Bool := containsB map key

@[simp] theorem size_empty : size ([] : SpecMap K V) = 0 := rfl

@[simp] theorem hasKey_empty (key : K) : hasKey ([] : SpecMap K V) key = false := rfl

def valueAt [Inhabited V] (map : SpecMap K V) (key : K) : V := get map key

def keyAt [Inhabited K] (map : SpecMap K V) (index : Int) : K :=
  if index < 0 then default else (map[index.toNat]?.map Prod.fst).getD default

def rank (map : SpecMap K V) (key : K) : Int := indexOf map key

def update (discipline : Discipline K) (map : SpecMap K V) (key : K) (value : V) : SpecMap K V :=
  discipline.set map key value

def remove (discipline : Discipline K) (map : SpecMap K V) (key : K) : SpecMap K V :=
  discipline.del map key

omit [DecidableEq K] in
theorem distinct_of_valid {discipline : Discipline K} {map : SpecMap K V}
    (valid : Valid discipline map) : Distinct map := by
  cases discipline with
  | sequence => exact valid
  | ordered order => exact distinct_of_ascending order valid

theorem indexOf_le_length (entries : SpecMap K V) (key : K) :
    indexOf entries key ≤ entries.length := by
  induction entries with
  | nil => simp [indexOf]
  | cons entry rest ih => simp only [indexOf]; split <;> simp; omega

/-- A key's position is at most the size: the size exactly without an entry. -/
theorem rank_bounds (map : SpecMap K V) (key : K) : 0 ≤ rank map key ∧ rank map key ≤ size map := by
  simp only [rank, size]; exact ⟨by omega, by exact_mod_cast indexOf_le_length _ key⟩

theorem indexOf_lt_length {entries : SpecMap K V} {key : K}
    (present : containsB entries key = true) : indexOf entries key < entries.length :=
  (getElem_indexOf entries key present).1

theorem fst_getElem_indexOf {entries : SpecMap K V} {key : K}
    (present : containsB entries key = true) :
    (entries[indexOf entries key]'(indexOf_lt_length present)).1 = key :=
  (getElem_indexOf entries key present).2

theorem indexOf_getElem {entries : SpecMap K V} (distinct : Distinct entries) (index : Nat)
    (bound : index < entries.length) : indexOf entries (entries[index]'bound).1 = index := by
  induction entries generalizing index with
  | nil => simp at bound
  | cons head rest ih =>
      simp only [Distinct, List.map_cons, List.nodup_cons, List.mem_map] at distinct
      cases index with
      | zero => simp [indexOf]
      | succ index =>
          have bound' : index < rest.length := by simpa using bound
          have differs : head.1 ≠ (rest[index]'bound').1 := fun e =>
            distinct.1 ⟨rest[index]'bound', List.getElem_mem bound', e.symm⟩
          simp only [List.getElem_cons_succ, indexOf, differs, if_false]
          rw [ih distinct.2 index bound']

theorem map_fst_seqSet_present {entries : SpecMap K V} {key : K} (value : V)
    (present : containsB entries key = true) :
    (seqSet entries key value).map Prod.fst = entries.map Prod.fst := by
  simp [seqSet, present, map_fst_replaceFirst]

theorem map_fst_ordSet_present (rank : KeyOrder K) {entries : SpecMap K V}
    {key : K} (value : V) (present : containsB entries key = true)
    (ascending : Ascending rank entries) :
    (ordSet rank key value entries).map Prod.fst = entries.map Prod.fst := by
  induction entries with
  | nil => simp [containsB] at present
  | cons head rest ih =>
      have tail := List.pairwise_cons.mp ascending
      simp only [ordSet]
      split
      · rename_i h
        rw [containsB_false_of_below rank h ascending] at present
        cases present
      · rename_i h
        have hk : head.1 = key := (Std.LawfulEqCmp.eq_of_compare h).symm
        simp [hk]
      · rename_i h
        have hk : head.1 ≠ key := fun e => by rw [e, Std.ReflCmp.compare_self (cmp := rank.compare)] at h; cases h
        simp only [containsB, hk, decide_false, Bool.false_or] at present
        simp [ih present tail.2]

/-! Positions, over map values. -/

theorem rank_lt_size {map : SpecMap K V} {key : K} (present : hasKey map key = true) :
    rank map key < size map := by
  simp only [rank, size]; exact_mod_cast indexOf_lt_length present

theorem indexOf_eq_length {entries : SpecMap K V} {key : K}
    (absent : containsB entries key = false) : indexOf entries key = entries.length := by
  induction entries with
  | nil => rfl
  | cons entry rest ih =>
      simp only [containsB, Bool.or_eq_false_iff, decide_eq_false_iff_not] at absent
      simp [indexOf, absent.1, ih absent.2]

/-- A key's position is within the map exactly when the map has the key. -/
theorem rank_lt_size_iff (map : SpecMap K V) (key : K) :
    rank map key < size map ↔ hasKey map key = true := by
  constructor
  · intro within
    cases present : hasKey map key
    · simp only [rank, size, hasKey] at within present
      rw [indexOf_eq_length present] at within; omega
    · rfl
  · exact rank_lt_size

theorem size_le_rank_iff (map : SpecMap K V) (key : K) :
    size map ≤ rank map key ↔ hasKey map key = false := by
  rw [← Int.not_lt, rank_lt_size_iff, Bool.not_eq_true]

theorem rank_nonneg (map : SpecMap K V) (key : K) : 0 ≤ rank map key := by
  simp [rank]

theorem keyAt_rank [Inhabited K] {map : SpecMap K V} {key : K} (present : hasKey map key = true) :
    keyAt map (rank map key) = key := by
  have bound := indexOf_lt_length present
  simp only [keyAt, rank, Int.natCast_nonneg, Int.not_lt.mpr, if_false, Int.toNat_natCast,
    List.getElem?_eq_getElem bound, Option.map_some, Option.getD_some]
  exact fst_getElem_indexOf present

/-- The key at a position within the map is one of its keys. -/
theorem hasKey_keyAt [Inhabited K] {map : SpecMap K V} {index : Int} (low : 0 ≤ index)
    (high : index < size map) : hasKey map (keyAt map index) = true := by
  have bound : index.toNat < map.length := by simp only [size] at high; omega
  simp only [hasKey, keyAt, Int.not_lt.mpr low, if_false, List.getElem?_eq_getElem bound,
    Option.map_some, Option.getD_some, containsB_iff]
  exact ⟨_, List.getElem_mem bound⟩

omit [DecidableEq K] in
/-- Ordered enumeration is strictly increasing. -/
theorem order_keyAt_lt [Inhabited K] {rank : KeyOrder K} {map : SpecMap K V}
    (valid : Valid (.ordered rank) map) {first second : Int} (low : 0 ≤ first)
    (before : first < second) (high : second < size map) :
    rank.compare (keyAt map first) (keyAt map second) = .lt := by
  have secondBound : second.toNat < map.length := by simp only [size] at high; omega
  have firstBound : first.toNat < map.length := by omega
  simp only [keyAt, Int.not_lt.mpr low, Int.not_lt.mpr (Int.le_of_lt (Int.lt_of_le_of_lt low before)),
    if_false, List.getElem?_eq_getElem firstBound, List.getElem?_eq_getElem secondBound,
    Option.map_some, Option.getD_some]
  exact List.pairwise_iff_getElem.mp valid _ _ firstBound secondBound (by omega)

/-- Looking up an enumerated key returns its original position. -/
theorem rank_keyAt [Inhabited K] {discipline : Discipline K} {map : SpecMap K V} (valid : Valid discipline map)
    {index : Int} (low : 0 ≤ index) (high : index < size map) : rank map (keyAt map index) = index := by
  have bound : index.toNat < map.length := by simp only [size] at high; omega
  have distinct := distinct_of_valid valid
  simp only [keyAt, rank, Int.not_lt.mpr low, if_false, List.getElem?_eq_getElem bound,
    Option.map_some, Option.getD_some]
  rw [indexOf_getElem distinct _ bound]; omega

/-- Positions within a valid map hold distinct keys. -/
theorem keyAt_eq_keyAt_iff [Inhabited K] {discipline : Discipline K} {map : SpecMap K V}
    (valid : Valid discipline map) {first second : Int} (firstLow : 0 ≤ first)
    (firstHigh : first < size map) (secondLow : 0 ≤ second) (secondHigh : second < size map) :
    keyAt map first = keyAt map second ↔ first = second := by
  constructor
  · intro same
    rw [← rank_keyAt valid firstLow firstHigh, ← rank_keyAt valid secondLow secondHigh, same]
  · intro same; rw [same]

theorem keyAt_update_present [Inhabited K] {discipline : Discipline K}
    {map : SpecMap K V} {key : K} (valid : Valid discipline map) (present : hasKey map key = true)
    (value : V) (index : Int) :
    keyAt (update discipline map key value) index = keyAt map index := by
  have keys : (discipline.set map key value).map Prod.fst = map.map Prod.fst := by
    cases discipline with
    | sequence => exact map_fst_seqSet_present value present
    | ordered rank => exact map_fst_ordSet_present rank value present valid
  simp only [keyAt, update]
  split
  · rfl
  · rw [← List.getElem?_map, keys, List.getElem?_map]

/-- A key's position depends on the keys alone. -/
theorem indexOf_of_keys {entries others : List (K × V)}
    (same : entries.map Prod.fst = others.map Prod.fst) (key : K) :
    indexOf entries key = indexOf others key := by
  induction entries generalizing others with
  | nil => cases others with
    | nil => rfl
    | cons _ _ => simp at same
  | cons head rest ih => cases others with
    | nil => simp at same
    | cons other tail =>
      simp only [List.map_cons, List.cons.injEq] at same
      simp only [indexOf, same.1, ih same.2]

/-- Setting a key the map holds keeps every key's rank. -/
theorem rank_update_present {discipline : Discipline K}
    {map : SpecMap K V} {key : K} (valid : Valid discipline map) (present : hasKey map key = true)
    (value : V) (probe : K) :
    rank (update discipline map key value) probe = rank map probe := by
  have keys : (discipline.set map key value).map Prod.fst = map.map Prod.fst := by
    cases discipline with
    | sequence => exact map_fst_seqSet_present value present
    | ordered rank => exact map_fst_ordSet_present rank value present valid
  simp only [rank, update, indexOf_of_keys keys]

/-- After an ordered removal, the positions from the removed key's on hold
the keys one position further. -/
theorem keyAt_remove_ordered [Inhabited K] (order : KeyOrder K)
    (map : SpecMap K V) (key : K) (index : Int) :
    keyAt (remove (.ordered order) map key) index =
      keyAt map (if index < rank map key then index else index + 1) := by
  have nonneg : 0 ≤ rank map key := by simp [rank]
  simp only [keyAt, remove, Discipline.del, ordDel_eq_eraseIdx,
    List.getElem?_eraseIdx, rank] at nonneg ⊢
  by_cases negative : index < 0
  · have : index < (indexOf map key : Int) := by omega
    simp [negative, this]
  · by_cases before : index < (indexOf map key : Int)
    · have : index.toNat < indexOf map key := by omega
      simp [negative, before, this]
    · have : ¬index.toNat < indexOf map key := by omega
      have shifted : (index + 1).toNat = index.toNat + 1 := by omega
      simp [negative, before, this, shifted, show ¬index + 1 < 0 by omega]

/-- Mathematical order of integer keys. -/
def intOrder : KeyOrder Int := ⟨compare, inferInstance, inferInstance⟩

@[simp] theorem intOrder_compare (a b : Int) : intOrder.compare a b = compare a b := rfl

omit [DecidableEq K] in
theorem size_nonneg (map : SpecMap K V) : 0 ≤ size map := Int.natCast_nonneg _

theorem hasKey_update (discipline : Discipline K) (map : SpecMap K V) (key : K) (value : V) (probe : K) :
    hasKey (update discipline map key value) probe = (decide (key = probe) || hasKey map probe) := by
  cases discipline with
  | sequence => exact containsB_seqSet map key value probe
  | ordered order => exact containsB_ordSet order map key value probe

theorem valueAt_update [Inhabited V] (discipline : Discipline K) (map : SpecMap K V) (key : K) (value : V) (probe : K) :
    valueAt (update discipline map key value) probe = if probe = key then value else valueAt map probe := by
  cases discipline with
  | sequence => exact get_seqSet map key value probe
  | ordered order => exact get_ordSet order map key value probe

theorem size_update {discipline : Discipline K} {map : SpecMap K V}
    (valid : Valid discipline map) (key : K) (value : V) :
    size (update discipline map key value) = if hasKey map key then size map else size map + 1 := by
  cases discipline with
  | sequence => simp only [size, update, Discipline.set, length_seqSet, hasKey]; split <;> simp
  | ordered order => simp only [size, update, Discipline.set, length_ordSet order valid, hasKey]; split <;> simp

theorem size_remove (discipline : Discipline K) (map : SpecMap K V) (key : K) :
    size (remove discipline map key) = if hasKey map key then size map - 1 else size map := by
  have positive : hasKey map key = true → 0 < map.length := fun h => by have := indexOf_lt_length h; omega
  cases discipline <;> simp only [size, remove, Discipline.del, length_seqDel, length_ordDel, hasKey]
  all_goals split <;> rename_i h <;> first | rfl | (have := positive h; omega)

omit [DecidableEq K] in
theorem keyAt_lt {map : SpecMap Int V} (valid : Valid (.ordered intOrder) map)
    {first second : Int} (low : 0 ≤ first) (before : first < second) (high : second < size map) :
    keyAt map first < keyAt map second :=
  Int.compare_eq_lt.mp (order_keyAt_lt valid low before high)

omit [DecidableEq K] in
/-- `keyAt_lt` as a fact of two positions, its conditions premises: what the
solver states of the positions a leaf reads. -/
theorem keyAt_lt_of_lt {map : SpecMap Int V} (valid : Valid (.ordered intOrder) map)
    (first second : Int) :
    0 ≤ first → first < second → second < size map → keyAt map first < keyAt map second :=
  fun low before high => keyAt_lt valid low before high

omit [DecidableEq K] in
/-- Equal positions hold equal keys, as a fact over the positions' bounds. -/
theorem keyAt_congr (map : SpecMap Int V) (first second : Int) :
    first ≤ second → second ≤ first → keyAt map first ≤ keyAt map second := by
  intro below above
  rw [show first = second by omega]
  exact Int.le_refl _

theorem hasKey_remove {discipline : Discipline K} {map : SpecMap K V}
    (valid : Valid discipline map) (key probe : K) :
    hasKey (remove discipline map key) probe = (!decide (key = probe) && hasKey map probe) := by
  cases discipline with
  | sequence => exact containsB_seqDel valid key probe
  | ordered order => exact containsB_ordDel (distinct_of_ascending order valid) key probe

theorem valueAt_remove [Inhabited V] {discipline : Discipline K} {map : SpecMap K V}
    (valid : Valid discipline map) (key probe : K) (different : probe ≠ key) :
    valueAt (remove discipline map key) probe = valueAt map probe := by
  cases discipline with
  | sequence => exact get_seqDel valid key probe different
  | ordered order => exact get_ordDel map key probe different

theorem valid_remove {discipline : Discipline K} {map : SpecMap K V}
    (valid : Valid discipline map) (key : K) : Valid discipline (remove discipline map key) := by
  cases discipline with
  | sequence => exact distinct_seqDel valid key
  | ordered order => exact ascending_ordDel order valid key

/-- A property of every key, such as the source integer range. -/
def KeysSatisfy (property : K → Prop) (map : SpecMap K V) : Prop :=
  ∀ entry ∈ map, property entry.1

/-- A property of every value, such as the source integer range. -/
def ValuesSatisfy (property : V → Prop) (map : SpecMap K V) : Prop :=
  ∀ entry ∈ map, property entry.2

/-- The value at any key has a property every value and the default have. -/
theorem valueAt_satisfies [Inhabited V] {map : SpecMap K V} {property : V → Prop}
    (values : ValuesSatisfy property map) (fallback : property default) (key : K) :
    property (valueAt map key) := by
  unfold valueAt
  induction map with
  | nil => exact fallback
  | cons entry rest ih =>
      simp only [get]
      split
      · exact values entry (List.mem_cons_self ..)
      · exact ih fun other member => values other (List.mem_cons_of_mem _ member)

omit [DecidableEq K] in
/-- The key at any position has a property every key and the default have. -/
theorem keyAt_satisfies_all [Inhabited K] {map : SpecMap K V} {property : K → Prop}
    (keys : KeysSatisfy property map) (fallback : property default) (index : Int) :
    property (keyAt map index) := by
  unfold keyAt
  split
  · exact fallback
  · cases found : map[index.toNat]? with
    | none => exact fallback
    | some entry => exact keys entry (List.mem_of_getElem? found)

omit [DecidableEq K] in
theorem keyAt_satisfies [Inhabited K] {map : SpecMap K V} {property : K → Prop}
    (keys : KeysSatisfy property map) {index : Int} (low : 0 ≤ index) (high : index < size map) :
    property (keyAt map index) := by
  have bound : index.toNat < map.length := by simp only [size] at high; omega
  simpa only [keyAt, Int.not_lt.mpr low, if_false, List.getElem?_eq_getElem bound,
    Option.map_some, Option.getD_some] using keys _ (List.getElem_mem bound)

omit [DecidableEq K] in
/-- A present key has a property every key has. -/
theorem hasKey_satisfies [DecidableEq K] [Inhabited K] {map : SpecMap K V} {property : K → Prop}
    (keys : KeysSatisfy property map) {key : K} (present : hasKey map key = true) :
    property key :=
  keyAt_rank present ▸ keyAt_satisfies keys (rank_nonneg map key) (rank_lt_size present)

/-- Present integer keys have the same strict order as their positions. -/
theorem rank_lt_rank {map : SpecMap Int V}
    (valid : Valid (.ordered intOrder) map) {a b : Int}
    (ha : hasKey map a = true) (hb : hasKey map b = true)
    (lt : a < b) : rank map a < rank map b := by
  have lowA := rank_nonneg map a
  have lowB := rank_nonneg map b
  have highA := rank_lt_size ha
  have highB := rank_lt_size hb
  have keyA := keyAt_rank ha
  have keyB := keyAt_rank hb
  by_cases yes : rank map a < rank map b
  · exact yes
  exfalso
  have reverse : rank map b ≤ rank map a := by omega
  rcases Int.lt_or_eq_of_le reverse with before | same
  · have ordered := keyAt_lt valid lowB before highA
    rw [keyA, keyB] at ordered
    omega
  · rw [same, keyA] at keyB
    have : a = b := keyB
    omega

/-- Inserting a missing ordered key shifts only the following positions. -/
theorem ordSet_keys_insert (order : KeyOrder K) (key : K) (value : V)
    (entries : SpecMap K V) (missing : containsB entries key = false) :
    (ordSet order key value entries).map Prod.fst =
      (entries.map Prod.fst).insertIdx (indexOf (ordSet order key value entries) key) key := by
  induction entries with
  | nil => simp [ordSet, indexOf]
  | cons entry rest ih =>
    simp only [containsB, Bool.or_eq_false_iff, decide_eq_false_iff_not] at missing
    have different : entry.1 ≠ key := missing.1
    cases comparison : order.compare key entry.1 with
    | lt => simp [ordSet, comparison, indexOf]
    | eq => exact False.elim (different (Std.LawfulEqCmp.eq_of_compare comparison).symm)
    | gt =>
      simp only [ordSet, comparison, indexOf, if_neg different, List.map_cons]
      simpa only [List.insertIdx_succ_cons] using congrArg (List.cons entry.1) (ih missing.2)

theorem keyAt_update_before [Inhabited K] (order : KeyOrder K)
    (map : SpecMap K V) (key : K) (value : V) (index : Int) (missing : hasKey map key = false)
    (low : 0 ≤ index) (before : index < rank (update (.ordered order) map key value) key) :
    keyAt (update (.ordered order) map key value) index = keyAt map index := by
  have bound : index.toNat < indexOf (ordSet order key value map) key := by
    simp only [rank, update, Discipline.set] at before
    omega
  change containsB map key = false at missing
  simp only [keyAt, if_neg (Int.not_lt.mpr low), update, Discipline.set,
    ← List.getElem?_map]
  rw [ordSet_keys_insert order key value _ missing, List.getElem?_insertIdx_of_lt bound]

theorem keyAt_update_after [Inhabited K] (order : KeyOrder K)
    (map : SpecMap K V) (key : K) (value : V) (index : Int) (missing : hasKey map key = false)
    (after : rank (update (.ordered order) map key value) key < index) :
    keyAt (update (.ordered order) map key value) index = keyAt map (index - 1) := by
  have rankLow := rank_nonneg (update (.ordered order) map key value) key
  have low : 0 ≤ index := by omega
  have prevLow : 0 ≤ index - 1 := by omega
  have bound : indexOf (ordSet order key value map) key < index.toNat := by
    simp only [rank, update, Discipline.set] at after
    omega
  change containsB map key = false at missing
  simp only [keyAt, if_neg (Int.not_lt.mpr low), if_neg (Int.not_lt.mpr prevLow),
    update, Discipline.set, ← List.getElem?_map]
  rw [ordSet_keys_insert order key value _ missing, List.getElem?_insertIdx_of_gt bound]
  rw [show (index - 1).toNat = index.toNat - 1 by omega]



def nextKey? (order : KeyOrder K) (map : SpecMap K V) (key : K) : Option K :=
  (map.find? fun entry => order.compare entry.1 key == .gt).map Prod.fst

def prevKey? (order : KeyOrder K) (map : SpecMap K V) (key : K) : Option K :=
  ((map.filter fun entry => order.compare entry.1 key == .lt).getLast?).map Prod.fst

omit [DecidableEq K] in
/-- The next key of a map literal, entry by entry. -/
theorem nextKey?_nil (order : KeyOrder K) (key : K) : nextKey? order ([] : SpecMap K V) key = none :=
  rfl

omit [DecidableEq K] in
theorem nextKey?_cons (order : KeyOrder K) (k : K) (v : V) (rest : SpecMap K V) (key : K) :
    nextKey? order ((k, v) :: rest) key =
      if order.compare k key = .gt then some k else nextKey? order rest key := by
  unfold nextKey?
  by_cases above : order.compare k key = .gt
  · simp [List.find?_cons, above]
  · simp [List.find?_cons, above]

omit [DecidableEq K] in
/-- The previous key of a map literal, entry by entry: the last below the
key among the later entries, else this one where it is below. -/
theorem prevKey?_nil (order : KeyOrder K) (key : K) : prevKey? order ([] : SpecMap K V) key = none :=
  rfl

omit [DecidableEq K] in
theorem prevKey?_cons (order : KeyOrder K) (k : K) (v : V) (rest : SpecMap K V) (key : K) :
    prevKey? order ((k, v) :: rest) key =
      (prevKey? order rest key).or (if order.compare k key = .lt then some k else none) := by
  unfold prevKey?
  by_cases below : order.compare k key = .lt
  · simp only [List.filter_cons, below, beq_self_eq_true, if_true, List.getLast?_cons]
    cases h : (List.filter (fun entry => order.compare entry.1 key == .lt) rest).getLast? <;>
      simp [h]
  · have : (order.compare k key == .lt) = false := by simpa using below
    simp only [List.filter_cons, this, Bool.false_eq_true, if_false, below]
    cases (List.filter (fun entry => order.compare entry.1 key == .lt) rest).getLast? <;> rfl

omit [DecidableEq K] in
theorem nextKey_at [Inhabited K] {order : KeyOrder K} {m : SpecMap K V}
    (valid : Valid (.ordered order) m) {i : Int} (low : 0 ≤ i) (high : i < size m) :
    nextKey? order m (keyAt m i) =
      if i + 1 < size m then some (keyAt m (i + 1)) else none := by
  have read (j : Nat) (hj : j < m.length) :
      m[j].1 = keyAt m j := by
    simp only [keyAt, Int.natCast_nonneg, Int.not_lt.mpr, if_false,
      Int.toNat_natCast, List.getElem?_eq_getElem hj, Option.map_some, Option.getD_some]
  have earlier (j : Nat) (hj : j < m.length) (before : (j : Int) ≤ i) :
      order.compare m[j].1 (keyAt m i) ≠ .gt := by
    rw [read j hj]
    by_cases same : (j : Int) = i
    · rw [same, Std.ReflCmp.compare_self (cmp := order.compare)]; decide
    · rw [order_keyAt_lt valid (by omega) (by omega) high]; decide
  unfold nextKey?
  split
  next within =>
    have bound : (i + 1).toNat < m.length := by simp only [size] at within; omega
    have forward := order_keyAt_lt valid low (show i < i + 1 by omega) within
    have found : m.find? (fun entry => order.compare entry.1 (keyAt m i) == .gt) =
        some m[(i + 1).toNat] := by
      apply List.find?_eq_some_iff_getElem.mpr
      refine ⟨?_, (i + 1).toNat, bound, rfl, ?_⟩
      · rw [read _ bound, Int.toNat_of_nonneg (by omega), Std.OrientedCmp.eq_swap (cmp := order.compare) (a := keyAt m (i + 1)), forward]
        rfl
      · intro j hj
        simp [earlier j (by omega) (by omega)]
    rw [found, Option.map_some, read _ bound, Int.toNat_of_nonneg (by omega)]
  next last =>
    have absent : m.find? (fun entry => order.compare entry.1 (keyAt m i) == .gt) = none := by
      apply List.find?_eq_none.mpr
      intro entry mem
      obtain ⟨j, bound, equal⟩ := List.getElem_of_mem mem
      subst entry
      have : (j : Int) ≤ i := by simp only [size] at high last; omega
      simpa only [beq_iff_eq] using earlier j bound this
    rw [absent, Option.map_none]


/-- No successor of a present key means that key is last. -/
theorem nextKey_none [Inhabited K] {order : KeyOrder K} {map : SpecMap K V} {key : K}
    (valid : Valid (.ordered order) map) (present : hasKey map key = true)
    (absent : nextKey? order map key = none) : rank map key + 1 = size map := by
  have low := rank_nonneg map key
  have high := rank_lt_size present
  rw [← keyAt_rank present, nextKey_at valid low high] at absent
  split at absent
  · simp at absent
  · omega

/-- A successor is present at the next position. -/
theorem nextKey_some [Inhabited K] {order : KeyOrder K} {map : SpecMap K V} {key next : K}
    (valid : Valid (.ordered order) map) (present : hasKey map key = true)
    (found : nextKey? order map key = some next) :
    hasKey map next = true ∧ rank map next = rank map key + 1 := by
  have low := rank_nonneg map key
  have high := rank_lt_size present
  rw [← keyAt_rank present, nextKey_at valid low high] at found
  split at found
  · have equal := Option.some.inj found
    rw [← equal]
    exact ⟨hasKey_keyAt (by omega) (by assumption), rank_keyAt valid (by omega) (by assumption)⟩
  · simp at found

omit [DecidableEq K] in
theorem prevKey_at [Inhabited K] {order : KeyOrder K} {m : SpecMap K V}
    (valid : Valid (.ordered order) m) {i : Int} (low : 0 ≤ i) (high : i < size m) :
    prevKey? order m (keyAt m i) = if 0 < i then some (keyAt m (i - 1)) else none := by
  have read (j : Nat) (hj : j < m.length) :
      m[j].1 = keyAt m j := by
    simp only [keyAt, Int.natCast_nonneg, Int.not_lt.mpr, if_false,
      Int.toNat_natCast, List.getElem?_eq_getElem hj, Option.map_some, Option.getD_some]
  have below (j : Nat) (hj : j < m.length) :
      (order.compare m[j].1 (keyAt m i) == .lt) = decide ((j : Int) < i) := by
    rw [read j hj]
    by_cases before : (j : Int) < i
    · rw [order_keyAt_lt valid (by omega) before high]; simp [before]
    · by_cases same : (j : Int) = i
      · rw [same, Std.ReflCmp.compare_self (cmp := order.compare)]; simp
      · rw [Std.OrientedCmp.eq_swap (cmp := order.compare),
          order_keyAt_lt valid low (by omega) (by simp only [size]; omega)]
        simp [before]
  have belowPrefix : m.filter (fun entry => order.compare entry.1 (keyAt m i) == .lt) =
      m.take i.toNat := by
    conv => lhs; arg 2; rw [← List.take_append_drop i.toNat m]
    rw [List.filter_append, List.filter_eq_self.mpr, List.filter_eq_nil_iff.mpr, List.append_nil]
    · intro entry mem
      obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem mem
      rw [List.getElem_drop, below _ (by simp at hj; omega)]; simp; omega
    · intro entry mem
      obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem mem
      rw [List.getElem_take, below _ (by simp at hj; omega)]; simp at hj ⊢; omega
  unfold prevKey?
  have bound : i.toNat < m.length := by simp only [size] at high; omega
  rw [belowPrefix, List.getLast?_eq_getElem?, List.length_take, List.getElem?_take,
    show min i.toNat m.length = i.toNat by omega]
  split
  next positive =>
    have previous : i.toNat - 1 < m.length := by omega
    rw [if_pos (by omega), List.getElem?_eq_getElem previous, Option.map_some, read _ previous,
      show ((i.toNat - 1 : Nat) : Int) = i - 1 by omega]
  next first =>
    rw [if_neg (by omega), Option.map_none]

/-- No predecessor of a present key means that key is first. -/
theorem prevKey_none [Inhabited K] {order : KeyOrder K} {map : SpecMap K V} {key : K}
    (valid : Valid (.ordered order) map) (present : hasKey map key = true)
    (absent : prevKey? order map key = none) : rank map key = 0 := by
  have low := rank_nonneg map key
  have high := rank_lt_size present
  rw [← keyAt_rank present, prevKey_at valid low high] at absent
  split at absent
  · simp at absent
  · omega

/-- A predecessor is present at the previous position. -/
theorem prevKey_some [Inhabited K] {order : KeyOrder K} {map : SpecMap K V} {key prev : K}
    (valid : Valid (.ordered order) map) (present : hasKey map key = true)
    (found : prevKey? order map key = some prev) :
    hasKey map prev = true ∧ rank map prev + 1 = rank map key := by
  have low := rank_nonneg map key
  have high := rank_lt_size present
  rw [← keyAt_rank present, prevKey_at valid low high] at found
  split at found
  · have equal := Option.some.inj found
    rw [← equal]
    exact ⟨hasKey_keyAt (by omega) (by omega), by rw [rank_keyAt valid (by omega) (by omega)]; omega⟩
  · simp at found

omit [DecidableEq K] in
/-- A key's successor is a key of the map: it has what every key has. -/
theorem nextKey_satisfies {order : KeyOrder K} {map : SpecMap K V} {property : K → Prop}
    (keys : KeysSatisfy property map) {key next : K}
    (found : nextKey? order map key = some next) : property next := by
  obtain ⟨entry, hit, rfl⟩ := Option.map_eq_some_iff.mp found
  exact keys entry (List.mem_of_find?_eq_some hit)

omit [DecidableEq K] in
/-- A key's predecessor is a key of the map. -/
theorem prevKey_satisfies {order : KeyOrder K} {map : SpecMap K V} {property : K → Prop}
    (keys : KeysSatisfy property map) {key prev : K}
    (found : prevKey? order map key = some prev) : property prev := by
  obtain ⟨entry, hit, rfl⟩ := Option.map_eq_some_iff.mp found
  exact keys entry (List.mem_filter.mp (List.mem_of_getLast? hit)).1

omit [DecidableEq K] in
/-- The returned key sequence has the map's cardinality. -/
theorem size_of_keys {map : SpecMap K V} {keys : List K} (same : keys = map.map Prod.fst) :
    (keys.length : Int) = size map := by simp [same, size]

omit [DecidableEq K] in
/-- Reads of a returned key sequence agree with map positions. -/
theorem keyAt_of_keys [Inhabited K] {map : SpecMap K V} {keys : List K}
    (same : keys = map.map Prod.fst) (index : Int) (low : 0 ≤ index) :
    keyAt map index = keys[index.toNat]?.getD default := by
  simp only [same, keyAt, Int.not_lt.mpr low, if_false, List.getElem?_map]

omit [DecidableEq K] in
/-- Integer-key enumeration establishes validity without exposing its list representation. -/
theorem valid_of_keyAt_lt {map : SpecMap Int V}
    (ordered : ∀ i j : Int, 0 ≤ i → i < j → j < size map → keyAt map i < keyAt map j) :
    Valid (.ordered intOrder) map := by
  apply List.pairwise_iff_getElem.mpr
  intro i j hi hj before
  have h := Int.compare_eq_lt.mpr (ordered i j (by omega) (by omega) (by simp only [size]; omega))
  simpa only [intOrder, keyAt, Int.natCast_nonneg, Int.not_lt.mpr (Int.natCast_nonneg _),
    if_false, Int.toNat_natCast, List.getElem?_eq_getElem hi, List.getElem?_eq_getElem hj,
    Option.map_some, Option.getD_some] using h

omit [DecidableEq K] in
/-- A slice is ordered when its keys enumerate a bounded interval of an ordered map.
The slice and parent may have different value types. -/
theorem valid_slice {W : Type} {map : SpecMap Int V} {slice : SpecMap Int W} {offset : Int}
    (valid : Valid (.ordered intOrder) map) (low : 0 ≤ offset)
    (high : offset + size slice ≤ size map)
    (same : ∀ i : Int, 0 ≤ i → i < size slice → keyAt slice i = keyAt map (offset + i)) :
    Valid (.ordered intOrder) slice := by
  apply valid_of_keyAt_lt
  intro i j ilo before jhi
  rw [same i ilo (by omega), same j (by omega) jhi]
  exact keyAt_lt valid (by omega) (by omega) (by omega)

/-- An update keeps a map valid under its discipline. -/
theorem valid_update {discipline : Discipline K} {map : SpecMap K V}
    (valid : Valid discipline map) (key : K) (value : V) :
    Valid discipline (update discipline map key value) := by
  cases discipline with
  | sequence => exact distinct_seqSet valid key value
  | ordered order => exact ascending_ordSet order valid key value

/-! ## Bulk operations

Several entries set in order, the entries of a prefix or a suffix, and a
key replaced at its position: the typed forms of the runtime bulk
operations (`Maps.updateAll`, `takeEntries`, `dropEntries`, `replaceKey`),
with the conditions under which the intrinsic roles abort. -/

/-- The keys of a map, in order. -/
def keys (map : SpecMap K V) : List K := map.map Prod.fst

/-- The values of a map, in order. -/
def values (map : SpecMap K V) : List V := map.map Prod.snd

omit [DecidableEq K] in
theorem keys_nil : keys ([] : SpecMap K V) = [] := rfl
omit [DecidableEq K] in
theorem keys_cons (entry : K × V) (rest : SpecMap K V) :
    keys (entry :: rest) = entry.1 :: keys rest := rfl
omit [DecidableEq K] in
theorem values_nil : values ([] : SpecMap K V) = [] := rfl
omit [DecidableEq K] in
theorem values_cons (entry : K × V) (rest : SpecMap K V) :
    values (entry :: rest) = entry.2 :: values rest := rfl

/-- The map with the keys of one list set to the values of another, in
order: a later equal key wins. -/
def updateAll (discipline : Discipline K) (map : SpecMap K V) (keys : List K) (values : List V) :
    SpecMap K V :=
  (keys.zip values).foldl (fun map entry => update discipline map entry.1 entry.2) map

/-- The entries of the first `count` positions. -/
def takeEntries (map : SpecMap K V) (count : Int) : SpecMap K V := map.take count.toNat

/-- The entries from position `count` on. -/
def dropEntries (map : SpecMap K V) (count : Int) : SpecMap K V := map.drop count.toNat

/-- The map with a key replaced by another at its position. -/
def replaceKey (map : SpecMap K V) (old new : K) : SpecMap K V :=
  map.map fun entry => if entry.1 = old then (new, entry.2) else entry

/-- Building a map aborts on lists of different lengths or a key twice. -/
def AbortsNewFrom (keys : List K) (values : List V) : Prop :=
  keys.length ≠ values.length ∨ ¬keys.Nodup

/-- Adding several entries aborts on lists of different lengths, a key the
map holds, or a key twice. -/
def AbortsAddAll (map : SpecMap K V) (keys : List K) (values : List V) : Prop :=
  keys.length ≠ values.length ∨ (∃ key ∈ keys, hasKey map key = true) ∨ ¬keys.Nodup

/-- Updating several entries aborts on lists of different lengths. -/
def AbortsUpsertAll (keys : List K) (values : List V) : Prop := keys.length ≠ values.length

/-- Appending a map aborts on a key both hold. -/
def AbortsAppendDisjoint (map other : SpecMap K V) : Prop :=
  ∃ key, hasKey map key = true ∧ hasKey other key = true

/-- Replacing a key aborts when the map lacks it, or when the new key breaks
the discipline at its position. -/
def AbortsReplaceKey (discipline : Discipline K) (map : SpecMap K V) (old new : K) : Prop :=
  hasKey map old = false ∨ ¬Valid discipline (replaceKey map old new)

theorem mem_keys_iff (map : SpecMap K V) (key : K) : key ∈ keys map ↔ hasKey map key = true := by
  simp only [keys, hasKey, containsB_iff, List.mem_map]
  constructor
  · rintro ⟨⟨_, value⟩, mem, rfl⟩
    exact ⟨value, mem⟩
  · rintro ⟨value, mem⟩
    exact ⟨(key, value), mem, rfl⟩

private theorem valid_foldl_update {discipline : Discipline K} (pairs : List (K × V))
    {map : SpecMap K V} (valid : Valid discipline map) :
    Valid discipline (pairs.foldl (fun map entry => update discipline map entry.1 entry.2) map) := by
  induction pairs generalizing map with
  | nil => exact valid
  | cons pair rest ih => exact ih (valid_update valid pair.1 pair.2)

private theorem hasKey_foldl_update (discipline : Discipline K) (pairs : List (K × V))
    (map : SpecMap K V) (probe : K) :
    hasKey (pairs.foldl (fun map entry => update discipline map entry.1 entry.2) map) probe =
      (decide (probe ∈ pairs.map Prod.fst) || hasKey map probe) := by
  induction pairs generalizing map with
  | nil => simp
  | cons pair rest ih =>
      rw [List.foldl_cons, ih, hasKey_update]
      apply Bool.eq_iff_iff.mpr
      simp only [Bool.or_eq_true, decide_eq_true_eq, List.map_cons, List.mem_cons]
      constructor
      · rintro (h | h | h)
        · exact .inl (.inr h)
        · exact .inl (.inl h.symm)
        · exact .inr h
      · rintro ((h | h) | h)
        · exact .inr (.inl h.symm)
        · exact .inl h
        · exact .inr (.inr h)

theorem length_ordSet_bounds (rank : KeyOrder K) (entries : List (K × V)) (key : K) (value : V) :
    entries.length ≤ (ordSet rank key value entries).length ∧
      (ordSet rank key value entries).length ≤ entries.length + 1 := by
  induction entries with
  | nil => simp [ordSet]
  | cons head rest ih =>
      simp only [ordSet]
      split <;> simp only [List.length_cons] <;> omega

/-- An update keeps every entry and adds at most one. -/
theorem size_update_bounds (discipline : Discipline K) (map : SpecMap K V) (key : K) (value : V) :
    size map ≤ size (update discipline map key value) ∧
      size (update discipline map key value) ≤ size map + 1 := by
  cases discipline with
  | sequence =>
      simp only [size, update, Discipline.set, length_seqSet]
      split <;> omega
  | ordered order =>
      have := length_ordSet_bounds order map key value
      simp only [size, update, Discipline.set]
      omega

private theorem size_foldl_update_bounds (discipline : Discipline K) (pairs : List (K × V))
    (map : SpecMap K V) :
    size map ≤ size (pairs.foldl (fun map entry => update discipline map entry.1 entry.2) map) ∧
      size (pairs.foldl (fun map entry => update discipline map entry.1 entry.2) map) ≤
        size map + (pairs.length : Int) := by
  induction pairs generalizing map with
  | nil => simp
  | cons pair rest ih =>
      rw [List.foldl_cons]
      have step := size_update_bounds discipline map pair.1 pair.2
      have := ih (update discipline map pair.1 pair.2)
      simp only [List.length_cons]
      omega

private theorem size_foldl_update_fresh {discipline : Discipline K} (pairs : List (K × V))
    {map : SpecMap K V} (valid : Valid discipline map) (nodup : (pairs.map Prod.fst).Nodup)
    (fresh : ∀ pair ∈ pairs, hasKey map pair.1 = false) :
    size (pairs.foldl (fun map entry => update discipline map entry.1 entry.2) map) =
      size map + (pairs.length : Int) := by
  induction pairs generalizing map with
  | nil => simp
  | cons pair rest ih =>
      rw [List.foldl_cons]
      simp only [List.map_cons, List.nodup_cons] at nodup
      have step := size_update valid pair.1 pair.2
      simp only [fresh pair (List.mem_cons_self ..), Bool.false_eq_true, if_false] at step
      rw [ih (valid_update valid pair.1 pair.2) nodup.2 ?_, step, List.length_cons]
      · omega
      intro other mem
      rw [hasKey_update]
      simp only [Bool.or_eq_false_iff, decide_eq_false_iff_not]
      exact ⟨fun eq => nodup.1 (by rw [eq]; exact List.mem_map_of_mem mem),
        fresh other (List.mem_cons_of_mem _ mem)⟩

theorem valid_updateAll {discipline : Discipline K} {map : SpecMap K V}
    (valid : Valid discipline map) (keys : List K) (values : List V) :
    Valid discipline (updateAll discipline map keys values) :=
  valid_foldl_update _ valid

/-- Setting entries keeps every entry and adds at most one per key set. -/
theorem size_updateAll_bounds (discipline : Discipline K) (map : SpecMap K V) (keys : List K)
    (values : List V) :
    size map ≤ size (updateAll discipline map keys values) ∧
      size (updateAll discipline map keys values) ≤ size map + (keys.length : Int) := by
  have := size_foldl_update_bounds discipline (keys.zip values) map
  rw [List.length_zip] at this
  unfold updateAll
  omega

theorem hasKey_updateAll (discipline : Discipline K) (map : SpecMap K V) {keys : List K}
    {values : List V} (fits : keys.length ≤ values.length) (probe : K) :
    hasKey (updateAll discipline map keys values) probe = (decide (probe ∈ keys) || hasKey map probe) := by
  rw [updateAll, hasKey_foldl_update, List.map_fst_zip fits]

theorem hasKey_updateAll_add_all (discipline : Discipline K) (map : SpecMap K V) (keys : List K)
    (values : List V) (ok : ¬AbortsAddAll map keys values) (probe : K) :
    hasKey (updateAll discipline map keys values) probe = (decide (probe ∈ keys) || hasKey map probe) :=
  hasKey_updateAll discipline map (Nat.le_of_eq (Decidable.of_not_not fun h => ok (Or.inl h))) probe

theorem hasKey_updateAll_upsert_all (discipline : Discipline K) (map : SpecMap K V) (keys : List K)
    (values : List V) (ok : ¬AbortsUpsertAll keys values) (probe : K) :
    hasKey (updateAll discipline map keys values) probe = (decide (probe ∈ keys) || hasKey map probe) :=
  hasKey_updateAll discipline map (Nat.le_of_eq (Decidable.of_not_not ok)) probe

theorem hasKey_updateAll_new_from (discipline : Discipline K) (map : SpecMap K V) (keys : List K)
    (values : List V) (ok : ¬AbortsNewFrom keys values) (probe : K) :
    hasKey (updateAll discipline map keys values) probe = (decide (probe ∈ keys) || hasKey map probe) :=
  hasKey_updateAll discipline map (Nat.le_of_eq (Decidable.of_not_not fun h => ok (Or.inl h))) probe

theorem hasKey_updateAll_append (discipline : Discipline K) (map other : SpecMap K V) (probe : K) :
    hasKey (updateAll discipline map (keys other) (values other)) probe =
      (hasKey other probe || hasKey map probe) := by
  rw [hasKey_updateAll discipline map (by simp [keys, values])]
  apply Bool.eq_iff_iff.mpr
  simp only [Bool.or_eq_true, decide_eq_true_eq, mem_keys_iff]

/-- Adding entries at fresh keys adds one each. -/
theorem size_updateAll_add_all {discipline : Discipline K} {map : SpecMap K V}
    (valid : Valid discipline map) (keys : List K) (values : List V)
    (ok : ¬AbortsAddAll map keys values) :
    size (updateAll discipline map keys values) = size map + (keys.length : Int) := by
  have lengths : keys.length = values.length := Decidable.of_not_not fun h => ok (Or.inl h)
  have fresh : ∀ pair ∈ keys.zip values, hasKey map pair.1 = false := by
    intro pair mem
    have absent := fun h => ok (Or.inr (Or.inl ⟨pair.1, (List.of_mem_zip mem).1, h⟩))
    cases h : hasKey map pair.1 with
    | false => rfl
    | true => exact absurd h absent
  have nodup : ((keys.zip values).map Prod.fst).Nodup := by
    rw [List.map_fst_zip (Nat.le_of_eq lengths)]
    exact Classical.byContradiction fun h => ok (Or.inr (Or.inr h))
  rw [updateAll, size_foldl_update_fresh _ valid nodup fresh, List.length_zip, ← lengths, Nat.min_self]

/-- A map built from fresh lists holds one entry per key. -/
theorem size_updateAll_new_from (discipline : Discipline K) (keys : List K) (values : List V)
    (ok : ¬AbortsNewFrom keys values) :
    size (updateAll discipline ([] : SpecMap K V) keys values) = (keys.length : Int) := by
  have valid : Valid discipline ([] : SpecMap K V) := by
    cases discipline <;> simp [Valid, Distinct, Ascending]
  have ok' : ¬AbortsAddAll ([] : SpecMap K V) keys values := fun
    | .inl h => ok (.inl h)
    | .inr (.inl ⟨_, _, h⟩) => by simp [hasKey, containsB] at h
    | .inr (.inr h) => ok (.inr h)
  rw [size_updateAll_add_all valid keys values ok']
  simp [size]

/-- Appending a disjoint map adds its entries. -/
theorem size_updateAll_append_disjoint {discipline : Discipline K} {map other : SpecMap K V}
    (valid : Valid discipline map) (validOther : Valid discipline other)
    (ok : ¬AbortsAppendDisjoint map other) :
    size (updateAll discipline map (keys other) (values other)) = size map + size other := by
  have ok' : ¬AbortsAddAll map (keys other) (values other) := fun
    | .inl h => h (by simp [keys, values])
    | .inr (.inl ⟨key, mem, held⟩) => ok ⟨key, held, (mem_keys_iff other key).mp mem⟩
    | .inr (.inr h) => h (distinct_of_valid validOther)
  rw [size_updateAll_add_all valid _ _ ok']
  simp [keys, size]

omit [DecidableEq K] in
theorem valid_takeEntries {discipline : Discipline K} {map : SpecMap K V}
    (valid : Valid discipline map) (count : Int) : Valid discipline (takeEntries map count) := by
  cases discipline with
  | sequence =>
      simp only [Valid, Distinct, takeEntries, List.map_take] at valid ⊢
      exact valid.sublist (List.take_sublist _ _)
  | ordered order => exact List.Pairwise.sublist (List.take_sublist _ _) valid

omit [DecidableEq K] in
theorem valid_dropEntries {discipline : Discipline K} {map : SpecMap K V}
    (valid : Valid discipline map) (count : Int) : Valid discipline (dropEntries map count) := by
  cases discipline with
  | sequence =>
      simp only [Valid, Distinct, dropEntries, List.map_drop] at valid ⊢
      exact valid.sublist (List.drop_sublist _ _)
  | ordered order => exact List.Pairwise.sublist (List.drop_sublist _ _) valid

omit [DecidableEq K] in
theorem size_takeEntries (map : SpecMap K V) (count : Int) :
    size (takeEntries map count) = min (max count 0) (size map) := by
  simp only [size, takeEntries, List.length_take]
  omega

omit [DecidableEq K] in
theorem size_dropEntries (map : SpecMap K V) (count : Int) :
    size (dropEntries map count) = size map - min (max count 0) (size map) := by
  simp only [size, dropEntries, List.length_drop]
  omega

theorem size_replaceKey (map : SpecMap K V) (old new : K) :
    size (replaceKey map old new) = size map := by
  simp [size, replaceKey]

instance (keys : List K) (values : List V) : Decidable (AbortsNewFrom keys values) := by
  unfold AbortsNewFrom; infer_instance

omit [DecidableEq K] in
instance (keys : List K) (values : List V) : Decidable (AbortsUpsertAll keys values) := by
  unfold AbortsUpsertAll; infer_instance

omit [DecidableEq K] in
theorem length_keys (map : SpecMap K V) : (keys map).length = map.length := List.length_map ..

omit [DecidableEq K] in
/-- A read of the key list at a position is the key at the position. -/
theorem getElem_keys [Inhabited K] (map : SpecMap K V) (n : Nat) (h : n < (keys map).length) :
    (keys map)[n] = keyAt map n := by
  have bound : n < map.length := by simpa only [keys, List.length_map] using h
  simp only [keys, keyAt, List.getElem_map, Int.not_lt.mpr (Int.natCast_nonneg n), if_false,
    Int.toNat_natCast, List.getElem?_eq_getElem bound, Option.map_some, Option.getD_some]

omit [DecidableEq K] in
/-- The key vector read at a nonnegative integer position, as Move reads it. -/
theorem getElem_keys_toNat [Inhabited K] (map : SpecMap K V) {index : Int} (low : 0 ≤ index)
    (h : index.toNat < (keys map).length) : (keys map)[index.toNat] = keyAt map index := by
  rw [getElem_keys, Int.toNat_of_nonneg low]

omit [DecidableEq K] in
theorem length_values (map : SpecMap K V) : (values map).length = map.length := List.length_map ..

/-- Appending a map keeps every entry and adds at most the other's. -/
theorem size_updateAll_append_bounds (discipline : Discipline K) (map other : SpecMap K V) :
    size map ≤ size (updateAll discipline map (keys other) (values other)) ∧
      size (updateAll discipline map (keys other) (values other)) ≤ size map + size other := by
  have := size_updateAll_bounds discipline map (keys other) (values other)
  simp only [length_keys, size] at this ⊢
  omega

private theorem length_le_of_nodup_subset {l₁ l₂ : List K} (nodup : l₁.Nodup) (subset : l₁ ⊆ l₂) :
    l₁.length ≤ l₂.length := by
  induction l₁ generalizing l₂ with
  | nil => simp
  | cons head rest ih =>
      rw [List.nodup_cons] at nodup
      have mem : head ∈ l₂ := subset (List.mem_cons_self ..)
      have restSubset : rest ⊆ l₂.erase head := by
        intro x hx
        have ne : x ≠ head := fun eq => nodup.1 (eq ▸ hx)
        exact (List.mem_erase_of_ne ne).mpr (subset (List.mem_cons_of_mem _ hx))
      have := ih nodup.2 restSubset
      rw [List.length_erase_of_mem mem] at this
      have pos : 0 < l₂.length := List.length_pos_of_mem mem
      simp only [List.length_cons]
      omega

/-- Appending a valid map holds at least its entries. -/
theorem size_other_le_updateAll_append {discipline : Discipline K} (map : SpecMap K V)
    {other : SpecMap K V} (validOther : Valid discipline other) :
    size other ≤ size (updateAll discipline map (keys other) (values other)) := by
  have subset : keys other ⊆ keys (updateAll discipline map (keys other) (values other)) := by
    intro key mem
    rw [mem_keys_iff] at mem ⊢
    rw [hasKey_updateAll_append, mem, Bool.true_or]
  have nodup : (keys other).Nodup := distinct_of_valid validOther
  have := length_le_of_nodup_subset nodup subset
  simp only [length_keys, size] at this ⊢
  omega

theorem keys_replaceKey (map : SpecMap K V) (old new : K) :
    keys (replaceKey map old new) = (keys map).map fun key => if key = old then new else key := by
  simp only [keys, replaceKey, List.map_map]
  congr 1
  funext entry
  by_cases same : entry.1 = old <;> simp [same]

/-- After a key is replaced, the new key is held where either was, and
every other key but the old one as before. -/
theorem hasKey_replaceKey (map : SpecMap K V) (old new probe : K) :
    hasKey (replaceKey map old new) probe =
      if probe = new then (hasKey map old || hasKey map new)
      else (!decide (probe = old) && hasKey map probe) := by
  rw [Bool.eq_iff_iff, ← mem_keys_iff, keys_replaceKey, List.mem_map]
  split
  · subst probe
    simp only [Bool.or_eq_true, ← mem_keys_iff]
    constructor
    · rintro ⟨key, mem, eq⟩
      by_cases same : key = old
      · exact .inl (same ▸ mem)
      · simp only [same, if_false] at eq
        exact .inr (eq ▸ mem)
    · rintro (mem | mem)
      · exact ⟨old, mem, by simp⟩
      · exact ⟨new, mem, by by_cases h : new = old <;> simp [h]⟩
  · rename_i differs
    simp only [Bool.and_eq_true, Bool.not_eq_true', decide_eq_false_iff_not, ← mem_keys_iff]
    constructor
    · rintro ⟨key, mem, eq⟩
      by_cases same : key = old
      · simp only [same, if_true] at eq
        exact absurd eq.symm differs
      · simp only [same, if_false] at eq
        exact ⟨eq ▸ same, eq ▸ mem⟩
    · rintro ⟨ne, mem⟩
      exact ⟨probe, mem, by simp [ne]⟩

/-! ## Entries after an update or a removal

An update's entries are the entry it writes and entries of the map before;
a removal's entries are entries of the map before. -/

theorem mem_replaceFirst {key : K} {value : V} : ∀ {entries : List (K × V)} {entry : K × V},
    entry ∈ replaceFirst key value entries → entry = (key, value) ∨ entry ∈ entries
  | [], _, member => by simp [replaceFirst] at member
  | head :: rest, entry, member => by
      unfold replaceFirst at member
      split at member
      · rcases List.mem_cons.mp member with same | inner
        · exact .inl same
        · exact .inr (List.mem_cons_of_mem _ inner)
      · rcases List.mem_cons.mp member with same | inner
        · exact .inr (same ▸ List.mem_cons_self ..)
        · rcases mem_replaceFirst inner with same | inner
          · exact .inl same
          · exact .inr (List.mem_cons_of_mem _ inner)

omit [DecidableEq K] in
theorem mem_swapRemove {entries : List (K × V)} {index : Nat} {entry : K × V}
    (member : entry ∈ swapRemove entries index) : entry ∈ entries :=
  List.mem_of_mem_eraseIdx ((swapRemove_perm entries index).mem_iff.mp member)

theorem mem_ordDel {key : K} : ∀ {entries : List (K × V)} {entry : K × V},
    entry ∈ ordDel key entries → entry ∈ entries
  | [], _, member => by simp [ordDel] at member
  | head :: rest, entry, member => by
      unfold ordDel at member
      split at member
      · exact List.mem_cons_of_mem _ member
      · rcases List.mem_cons.mp member with same | inner
        · exact same ▸ List.mem_cons_self ..
        · exact List.mem_cons_of_mem _ (mem_ordDel inner)

theorem mem_update {discipline : Discipline K} {map : SpecMap K V} {key : K} {value : V}
    {entry : K × V} (member : entry ∈ update discipline map key value) :
    entry = (key, value) ∨ entry ∈ map := by
  cases discipline with
  | sequence =>
      simp only [update, Discipline.set, seqSet] at member
      split at member
      · exact mem_replaceFirst member
      · rcases List.mem_append.mp member with inner | added
        · exact .inr inner
        · exact .inl (List.mem_singleton.mp added)
  | ordered order => exact mem_ordSet order member

theorem mem_remove {discipline : Discipline K} {map : SpecMap K V} {key : K} {entry : K × V}
    (member : entry ∈ remove discipline map key) : entry ∈ map := by
  cases discipline with
  | sequence =>
      simp only [remove, Discipline.del, seqDel] at member
      split at member
      · exact mem_swapRemove member
      · exact member
  | ordered _ => exact mem_ordDel member

/-- Whether every entry of a map satisfies a property. -/
def EntriesSatisfy (property : K × V → Prop) (map : SpecMap K V) : Prop :=
  ∀ entry ∈ map, property entry

omit [DecidableEq K] in
theorem entriesSatisfy_nil (property : K × V → Prop) :
    EntriesSatisfy property ([] : SpecMap K V) := fun _ member => nomatch member

theorem entriesSatisfy_update {property : K × V → Prop} {discipline : Discipline K}
    {map : SpecMap K V} {key : K} {value : V} (before : EntriesSatisfy property map)
    (written : property (key, value)) :
    EntriesSatisfy property (update discipline map key value) := fun _ member =>
  (mem_update member).elim (fun same => same ▸ written) (before _)

theorem entriesSatisfy_remove {property : K × V → Prop} {discipline : Discipline K}
    {map : SpecMap K V} {key : K} (before : EntriesSatisfy property map) :
    EntriesSatisfy property (remove discipline map key) := fun _ member =>
  before _ (mem_remove member)

end VerexIR.SpecMap
