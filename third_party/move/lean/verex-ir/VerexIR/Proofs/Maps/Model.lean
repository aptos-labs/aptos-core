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

/-! ## Laws of the sequence discipline -/

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

/-! ## Laws of the ordered discipline -/

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

omit [DecidableEq K] in
theorem keyAt_satisfies [Inhabited K] {map : SpecMap K V} {property : K → Prop}
    (keys : KeysSatisfy property map) {index : Int} (low : 0 ≤ index) (high : index < size map) :
    property (keyAt map index) := by
  have bound : index.toNat < map.length := by simp only [size] at high; omega
  simpa only [keyAt, Int.not_lt.mpr low, if_false, List.getElem?_eq_getElem bound,
    Option.map_some, Option.getD_some] using keys _ (List.getElem_mem bound)

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

end VerexIR.SpecMap
