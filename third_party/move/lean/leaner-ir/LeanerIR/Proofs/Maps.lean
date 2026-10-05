-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Order
import LeanerIR.Proofs.Denote.Attr

/-!
# The intrinsic map model

The model of an intrinsic map value (`designs/intrinsic-maps.md`): the list
of its `(key, value)` entries, under one of two disciplines. A sequence keeps
insertion order: an update replaces the entry in place or appends one, and a
removal moves the last entry into the removed position. An ordered map keeps
its keys strictly ascending under the structural order. The operations are
defined for every list, and the laws that need distinct or ascending keys
take them as hypotheses.
-/

namespace LeanerIR.Maps

/-- Equality of runtime values decided by the structural order. -/
@[reducible] def decEq : DecidableEq RuntimeValue := fun a b =>
  decidable_of_iff (RuntimeValue.order ⟨fun _ _ => 0, fun _ => 0⟩ a b = .eq)
    ⟨RuntimeValue.eq_of_order, fun h => h ▸ RuntimeValue.order_self _ a⟩

attribute [local instance] decEq

/-- The entries of a map. -/
abbrev Entries := List (RuntimeValue × RuntimeValue)

/-- Whether a key has an entry. -/
def containsB : Entries → RuntimeValue → Bool
  | [], _ => false
  | entry :: rest, key => decide (entry.1 = key) || containsB rest key

/-- The value of a key's first entry; junk without one. -/
def get : Entries → RuntimeValue → RuntimeValue
  | [], _ => .unit
  | entry :: rest, key => if entry.1 = key then entry.2 else get rest key

/-- The position of a key's first entry; the length without one. -/
def indexOf : Entries → RuntimeValue → Nat
  | [], _ => 0
  | entry :: rest, key => if entry.1 = key then 0 else indexOf rest key + 1

/-- The keys are distinct. -/
def Distinct (entries : Entries) : Prop := (entries.map Prod.fst).Nodup

/-- Replace the value of a key's first entry. -/
def replaceFirst (key value : RuntimeValue) : Entries → Entries
  | [] => []
  | entry :: rest =>
      if entry.1 = key then (key, value) :: rest else entry :: replaceFirst key value rest

/-- Remove the entry at a position by moving the last entry into it. -/
def swapRemove (entries : Entries) (index : Nat) : Entries :=
  match (entries.drop (index + 1)).reverse with
  | [] => entries.take index
  | last :: middle => entries.take index ++ last :: middle.reverse

/-! ## The sequence discipline -/

/-- Update in place, or append. -/
def seqSet (entries : Entries) (key value : RuntimeValue) : Entries :=
  if containsB entries key then replaceFirst key value entries else entries ++ [(key, value)]

/-- Remove a key's first entry by swapping the last entry into its place. -/
def seqDel (entries : Entries) (key : RuntimeValue) : Entries :=
  if containsB entries key then swapRemove entries (indexOf entries key) else entries

/-! ## Membership -/

theorem containsB_iff (entries : Entries) (key : RuntimeValue) :
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

@[simp] theorem containsB_nil (key : RuntimeValue) : containsB [] key = false := rfl

theorem containsB_append (left right : Entries) (key : RuntimeValue) :
    containsB (left ++ right) key = (containsB left key || containsB right key) := by
  induction left with
  | nil => simp [containsB]
  | cons entry rest ih => simp [containsB, ih, Bool.or_assoc]

theorem containsB_of_perm {left right : Entries} (perm : left.Perm right) (key : RuntimeValue) :
    containsB left key = containsB right key := by
  apply Bool.eq_iff_iff.mpr
  simp only [containsB_iff]
  exact exists_congr fun _ => perm.mem_iff

theorem mem_of_get (entries : Entries) (key : RuntimeValue) (present : containsB entries key = true) :
    (key, get entries key) ∈ entries := by
  induction entries with
  | nil => simp [containsB] at present
  | cons entry rest ih =>
      obtain ⟨k, v⟩ := entry
      by_cases h : k = key
      · subst h; simp [get]
      · simp only [containsB, h, decide_false, Bool.false_or] at present
        simp [get, h, ih present]

theorem get_of_mem {entries : Entries} (distinct : Distinct entries) {key value : RuntimeValue}
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

theorem get_of_perm {left right : Entries} (perm : left.Perm right) (distinct : Distinct left)
    (key : RuntimeValue) : get left key = get right key := by
  by_cases present : containsB left key = true
  · have mem := mem_of_get left key present
    have distinct' : Distinct right := (perm.map Prod.fst).nodup_iff.mp distinct
    exact (get_of_mem distinct' (perm.mem_iff.mp mem)).symm
  · have absent : containsB right key = false := by
      rw [← containsB_of_perm perm]; simpa using present
    simp only [Bool.not_eq_true] at present
    clear perm distinct
    have none (entries : Entries) (h : containsB entries key = false) : get entries key = .unit := by
      induction entries with
      | nil => rfl
      | cons entry rest ih =>
          simp only [containsB, Bool.or_eq_false_iff, decide_eq_false_iff_not] at h
          simp [get, h.1, ih h.2]
    rw [none left present, none right absent]

/-! ## Laws of the sequence discipline -/

theorem containsB_replaceFirst (entries : Entries) (key value probe : RuntimeValue) :
    containsB (replaceFirst key value entries) probe = containsB entries probe := by
  induction entries with
  | nil => rfl
  | cons entry rest ih =>
      by_cases h : entry.1 = key
      · simp [replaceFirst, containsB, h]
      · simp [replaceFirst, containsB, h, ih]

theorem get_replaceFirst (entries : Entries) (key value probe : RuntimeValue)
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

theorem length_replaceFirst (entries : Entries) (key value : RuntimeValue) :
    (replaceFirst key value entries).length = entries.length := by
  induction entries with
  | nil => rfl
  | cons entry rest ih => by_cases h : entry.1 = key <;> simp [replaceFirst, h, ih]

theorem map_fst_replaceFirst (entries : Entries) (key value : RuntimeValue) :
    (replaceFirst key value entries).map Prod.fst = entries.map Prod.fst := by
  induction entries with
  | nil => rfl
  | cons entry rest ih =>
      by_cases h : entry.1 = key
      · simp [replaceFirst, h]
      · simp [replaceFirst, h, ih]

theorem get_append_absent (entries : Entries) (key value probe : RuntimeValue)
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

@[simp] theorem containsB_seqSet (entries : Entries) (key value probe : RuntimeValue) :
    containsB (seqSet entries key value) probe = (decide (key = probe) || containsB entries probe) := by
  unfold seqSet
  split
  · rename_i present
    rw [containsB_replaceFirst]
    by_cases h : key = probe
    · subst h; simp [present]
    · simp [h]
  · simp [containsB_append, containsB, Bool.or_comm]

@[simp] theorem get_seqSet (entries : Entries) (key value probe : RuntimeValue) :
    get (seqSet entries key value) probe = if probe = key then value else get entries probe := by
  unfold seqSet
  split
  · rename_i present; exact get_replaceFirst entries key value probe present
  · rename_i absent; exact get_append_absent entries key value probe (by simpa using absent)

@[simp] theorem length_seqSet (entries : Entries) (key value : RuntimeValue) :
    (seqSet entries key value).length =
      if containsB entries key then entries.length else entries.length + 1 := by
  unfold seqSet
  split <;> simp [length_replaceFirst]

theorem distinct_seqSet {entries : Entries} (distinct : Distinct entries) (key value : RuntimeValue) :
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

theorem swapRemove_perm (entries : Entries) (index : Nat) :
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

theorem getElem_indexOf (entries : Entries) (key : RuntimeValue)
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

theorem seqDel_perm (entries : Entries) (key : RuntimeValue) (present : containsB entries key = true) :
    (seqDel entries key).Perm (entries.eraseIdx (indexOf entries key)) := by
  simp only [seqDel, present, if_true]
  exact swapRemove_perm entries _

theorem mem_eraseIdx_iff_of_distinct {entries : Entries} (distinct : Distinct entries)
    (index : Nat) (bound : index < entries.length) (entry : RuntimeValue × RuntimeValue) :
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

theorem length_seqDel (entries : Entries) (key : RuntimeValue) :
    (seqDel entries key).length =
      if containsB entries key then entries.length - 1 else entries.length := by
  by_cases present : containsB entries key = true
  · obtain ⟨bound, -⟩ := getElem_indexOf entries key present
    rw [(seqDel_perm entries key present).length_eq, List.length_eraseIdx_of_lt bound]
    simp [present]
  · simp [seqDel, present]

theorem distinct_seqDel {entries : Entries} (distinct : Distinct entries) (key : RuntimeValue) :
    Distinct (seqDel entries key) := by
  by_cases present : containsB entries key = true
  · unfold Distinct
    exact ((seqDel_perm entries key present).map Prod.fst).nodup_iff.mpr
      (List.Nodup.sublist ((List.eraseIdx_sublist _ _).map Prod.fst) distinct)
  · simpa [seqDel, present] using distinct

theorem containsB_seqDel {entries : Entries} (distinct : Distinct entries) (key probe : RuntimeValue) :
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

theorem get_seqDel {entries : Entries} (distinct : Distinct entries) (key probe : RuntimeValue)
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
      have none (entries : Entries) (h : containsB entries probe = false) :
          get entries probe = .unit := by
        induction entries with
        | nil => rfl
        | cons entry rest ih =>
            simp only [containsB, Bool.or_eq_false_iff, decide_eq_false_iff_not] at h
            simp [get, h.1, ih h.2]
      rw [none _ absent, none _ (by simpa using probed)]
  · simp [seqDel, present]


/-! ## The ordered discipline -/

/-- The keys are strictly ascending. -/
def Ascending (rank : ValueRanks) (entries : Entries) : Prop :=
  entries.Pairwise fun earlier later => RuntimeValue.order rank earlier.1 later.1 = .lt

/-- Update in place, or insert at the key's position. -/
def ordSet (rank : ValueRanks) (key value : RuntimeValue) : Entries → Entries
  | [] => [(key, value)]
  | entry :: rest =>
      match RuntimeValue.order rank key entry.1 with
      | .lt => (key, value) :: entry :: rest
      | .eq => (key, value) :: rest
      | .gt => entry :: ordSet rank key value rest

/-- Remove a key's first entry. -/
def ordDel (key : RuntimeValue) : Entries → Entries
  | [] => []
  | entry :: rest => if entry.1 = key then rest else entry :: ordDel key rest

/-! ## Laws of the ordered discipline -/

section OrderedLaws
variable (rank : ValueRanks)

theorem ne_of_order_lt {a b : RuntimeValue} (h : RuntimeValue.order rank a b = .lt) : a ≠ b := by
  rintro rfl; rw [RuntimeValue.order_self] at h; cases h

theorem distinct_of_ascending {entries : Entries} (ascending : Ascending rank entries) :
    Distinct entries := by
  unfold Distinct
  rw [List.Nodup, List.pairwise_map]
  exact ascending.imp fun h => ne_of_order_lt rank h

theorem mem_ordSet {key value : RuntimeValue} {entries : Entries} {entry : RuntimeValue × RuntimeValue}
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

theorem containsB_ordSet (entries : Entries) (key value probe : RuntimeValue) :
    containsB (ordSet rank key value entries) probe = (decide (key = probe) || containsB entries probe) := by
  induction entries with
  | nil => simp [ordSet, containsB]
  | cons head rest ih =>
      simp only [ordSet]
      split
      · simp [containsB]
      · rename_i h
        have : head.1 = key := (RuntimeValue.eq_of_order h).symm
        simp only [containsB, this]
        cases decide (key = probe) <;> simp
      · simp only [containsB, ih]
        cases decide (key = probe) <;> cases decide (head.1 = probe) <;> simp

theorem get_ordSet (entries : Entries) (key value probe : RuntimeValue) :
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
        have hk : head.1 = key := (RuntimeValue.eq_of_order h).symm
        by_cases p : probe = key
        · subst p; simp [get]
        · simp [get, Ne.symm p, p, hk]
      · rename_i h
        have hk : head.1 ≠ key := fun e => by rw [e, RuntimeValue.order_self] at h; cases h
        by_cases p : head.1 = probe
        · have : probe ≠ key := fun e => hk (p.trans e)
          simp [get, p, this]
        · simp [get, p, ih]

theorem ascending_ordSet {entries : Entries} (ascending : Ascending rank entries)
    (key value : RuntimeValue) : Ascending rank (ordSet rank key value entries) := by
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
        have hk : head.1 = key := (RuntimeValue.eq_of_order h).symm
        refine List.pairwise_cons.mpr ⟨?_, tail.2⟩
        intro x mem
        simpa [hk] using tail.1 x mem
      · rename_i h
        refine List.pairwise_cons.mpr ⟨?_, ih tail.2⟩
        intro x mem
        rcases mem_ordSet rank mem with rfl | mem
        · exact Std.OrientedCmp.gt_iff_lt.mp h
        · exact tail.1 x mem

theorem containsB_false_of_below {entries : Entries}
    {head : RuntimeValue × RuntimeValue} {key : RuntimeValue}
    (below : RuntimeValue.order rank key head.1 = .lt) (sorted : Ascending rank (head :: entries)) :
    containsB (head :: entries) key = false := by
  have tail := List.pairwise_cons.mp sorted
  apply Bool.eq_false_iff.mpr
  intro present
  obtain ⟨v, mem⟩ := (containsB_iff _ _).mp present
  rcases List.mem_cons.mp mem with h | h
  · rw [← h] at below; rw [RuntimeValue.order_self] at below; cases below
  · have := Std.TransCmp.lt_trans below (tail.1 _ h)
    rw [RuntimeValue.order_self] at this; cases this

theorem length_ordSet {entries : Entries} (ascending : Ascending rank entries) (key value : RuntimeValue) :
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
        have hk : head.1 = key := (RuntimeValue.eq_of_order h).symm
        simp [containsB, hk]
      · rename_i h
        have hk : head.1 ≠ key := fun e => by rw [e, RuntimeValue.order_self] at h; cases h
        simp only [List.length_cons, ih tail.2, containsB, hk, decide_false, Bool.false_or]
        split <;> rfl

theorem sublist_ordDel (key : RuntimeValue) (entries : Entries) : (ordDel key entries).Sublist entries := by
  induction entries with
  | nil => exact List.Sublist.slnil
  | cons head rest ih =>
      simp only [ordDel]
      split
      · exact List.sublist_cons_self _ _
      · exact ih.cons_cons _

theorem ascending_ordDel {entries : Entries} (ascending : Ascending rank entries) (key : RuntimeValue) :
    Ascending rank (ordDel key entries) :=
  ascending.sublist (sublist_ordDel key entries)

end OrderedLaws

theorem containsB_ordDel {entries : Entries} (distinct : Distinct entries) (key probe : RuntimeValue) :
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

theorem get_ordDel (entries : Entries) (key probe : RuntimeValue) (differs : probe ≠ key) :
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
theorem ordDel_eq_eraseIdx (key : RuntimeValue) (entries : Entries) :
    ordDel key entries = entries.eraseIdx (indexOf entries key) := by
  induction entries with
  | nil => rfl
  | cons entry rest ih =>
      simp only [ordDel, indexOf]
      split
      · rfl
      · rw [ih]; rfl

theorem length_ordDel (entries : Entries) (key : RuntimeValue) :
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


/-! ## Disciplines -/

/-- How a map keeps its entries. -/
inductive Discipline where
  | sequence
  | ordered (rank : ValueRanks)

/-- The entries with a key's value updated. -/
def Discipline.set : Discipline → Entries → RuntimeValue → RuntimeValue → Entries
  | .sequence, entries, key, value => seqSet entries key value
  | .ordered rank, entries, key, value => ordSet rank key value entries

/-- The entries without a key. -/
def Discipline.del : Discipline → Entries → RuntimeValue → Entries
  | .sequence, entries, key => seqDel entries key
  | .ordered _, entries, key => ordDel key entries

/-- The representation invariant: distinct keys, ascending for an ordered map. -/
def Discipline.Valid : Discipline → Entries → Prop
  | .sequence, entries => Distinct entries
  | .ordered rank, entries => Ascending rank entries

@[simp] theorem Discipline.set_sequence (entries : Entries) (key value : RuntimeValue) :
    Discipline.sequence.set entries key value = seqSet entries key value := rfl
@[simp] theorem Discipline.del_sequence (entries : Entries) (key : RuntimeValue) :
    Discipline.sequence.del entries key = seqDel entries key := rfl
@[simp] theorem Discipline.valid_sequence (entries : Entries) :
    Discipline.sequence.Valid entries = Distinct entries := rfl
@[simp] theorem Discipline.set_ordered (rank : ValueRanks) (entries : Entries)
    (key value : RuntimeValue) :
    (Discipline.ordered rank).set entries key value = ordSet rank key value entries := rfl
@[simp] theorem Discipline.del_ordered (rank : ValueRanks) (entries : Entries)
    (key : RuntimeValue) : (Discipline.ordered rank).del entries key = ordDel key entries := rfl
@[simp] theorem Discipline.valid_ordered (rank : ValueRanks) (entries : Entries) :
    (Discipline.ordered rank).Valid entries = Ascending rank entries := rfl

/-! ## Layouts -/

/-- Where a map value keeps its entries: the owner's value (a plain struct,
or one variant of an enum) holds one vector of two-field entries. -/
structure Layout where
  owner : StructHandle
  variant : Option String
  entry : StructHandle

/-- The entries a map value holds. -/
def entriesOf (map : RuntimeValue) : Entries :=
  match map.field 0 with
  | .vector elements => elements.toList.map fun entry => (entry.field 0, entry.field 1)
  | _ => []

/-- The map value holding entries. -/
def Layout.build (layout : Layout) (entries : Entries) : RuntimeValue :=
  .nominal layout.owner layout.variant
    #[.vector (entries.map fun entry => .nominal layout.entry none #[entry.1, entry.2]).toArray]

@[simp] theorem entriesOf_build (layout : Layout) (entries : Entries) :
    entriesOf (layout.build entries) = entries := by
  simp [entriesOf, Layout.build, RuntimeValue.field, List.map_map, Function.comp_def]


/-! ## Map values

The model a specification sees of a map value, and the laws a proof uses. -/

/-- The number of entries. -/
def size (map : RuntimeValue) : Int := (entriesOf map).length

/-- Whether a key has an entry. -/
def hasKey (map key : RuntimeValue) : Bool := containsB (entriesOf map) key

/-- The value of a key; junk without an entry. -/
def valueAt (map key : RuntimeValue) : RuntimeValue := get (entriesOf map) key

/-- The key at a position; junk out of range. -/
def keyAt (map : RuntimeValue) (index : Int) : RuntimeValue :=
  if index < 0 then .unit else (((entriesOf map)[index.toNat]?).map Prod.fst).getD .unit

/-- The position of a key; the size without an entry. -/
def rank (map key : RuntimeValue) : Int := indexOf (entriesOf map) key

/-- A map's size is a count. -/
theorem size_nonneg (map : RuntimeValue) : 0 ≤ size map := by
  simp [size]

/-- A map value built as a vector literal holds as many entries as the
vector, stated by bounds: its size stays the abstract size the laws read. -/
theorem size_nominal_vector_bounds (owner : StructHandle) (variant : Option String)
    (elements : Array RuntimeValue) :
    0 ≤ size (.nominal owner variant #[.vector elements]) ∧
      size (.nominal owner variant #[.vector elements]) ≤ elements.size ∧
      (elements.size : Int) ≤ size (.nominal owner variant #[.vector elements]) := by
  simp [size, entriesOf, RuntimeValue.field]

theorem indexOf_le_length (entries : Entries) (key : RuntimeValue) :
    indexOf entries key ≤ entries.length := by
  induction entries with
  | nil => simp [indexOf]
  | cons entry rest ih => simp only [indexOf]; split <;> simp; omega

/-- A key's position is at most the size: the size exactly without an entry. -/
theorem rank_bounds (map key : RuntimeValue) : 0 ≤ rank map key ∧ rank map key ≤ size map := by
  simp only [rank, size]; exact ⟨by omega, by exact_mod_cast indexOf_le_length _ key⟩

/-- An integer reads as itself. -/
@[grind =] theorem asInt_integer (value : Int) : RuntimeValue.asInt (.integer value) = value := rfl

/-- A Boolean reads as itself. -/
@[grind =] theorem asBool_bool (value : Bool) : RuntimeValue.asBool (.bool value) = value := rfl

/-- An address reads as itself. -/
@[grind =] theorem asString_address (value : String) :
    RuntimeValue.asString (.address value) = value := rfl

/-- The keys of a map value lie in the image of a scalar encoding `write`,
which `read` inverts: a specification reading a key at a scalar type and
writing it back as a key denotes the key itself. -/
def KeysRead {α : Type} (read : RuntimeValue → α) (write : α → RuntimeValue)
    (map : RuntimeValue) : Prop :=
  ∀ entry ∈ entriesOf map, write (read entry.1) = entry.1

theorem write_read_keyAt {α : Type} {read : RuntimeValue → α} {write : α → RuntimeValue}
    {map : RuntimeValue} {index : Int} (keys : KeysRead read write map) (nonneg : 0 ≤ index)
    (bound : index < size map) : write (read (keyAt map index)) = keyAt map index := by
  have within : index.toNat < (entriesOf map).length := by unfold size at bound; omega
  simp only [keyAt, show ¬ index < 0 by omega, if_false, List.getElem?_eq_getElem within,
    Option.map_some, Option.getD_some]
  exact keys _ (List.getElem_mem within)

/-- The keys of entries encoded one by one, the key as their first field, are
read back when every encoded first field is. -/
@[lir_denote_norm] theorem keysRead_map {α β : Type} {read : RuntimeValue → α}
    {write : α → RuntimeValue} (owner : StructHandle) (variant : Option String)
    (encode : β → RuntimeValue) (entries : Array β)
    (roundTrip : ∀ value, write (read ((encode value).field 0)) = (encode value).field 0) :
    KeysRead read write (.nominal owner variant #[.vector (entries.map encode)]) := by
  intro pair member
  simp only [entriesOf, RuntimeValue.field, List.getElem?_toArray, List.getElem?_cons_zero,
    Option.getD_some, Array.toList_map, List.map_map, List.mem_map, Function.comp_def] at member
  obtain ⟨value, _, rfl⟩ := member
  exact roundTrip value

theorem integer_asInt_keyAt {map : RuntimeValue} {index : Int}
    (keys : KeysRead RuntimeValue.asInt RuntimeValue.integer map) (nonneg : 0 ≤ index)
    (bound : index < size map) : RuntimeValue.integer (keyAt map index).asInt = keyAt map index :=
  write_read_keyAt keys nonneg bound

theorem bool_asBool_keyAt {map : RuntimeValue} {index : Int}
    (keys : KeysRead RuntimeValue.asBool RuntimeValue.bool map) (nonneg : 0 ≤ index)
    (bound : index < size map) : RuntimeValue.bool (keyAt map index).asBool = keyAt map index :=
  write_read_keyAt keys nonneg bound

theorem address_asString_keyAt {map : RuntimeValue} {index : Int}
    (keys : KeysRead RuntimeValue.asString RuntimeValue.address map) (nonneg : 0 ≤ index)
    (bound : index < size map) :
    RuntimeValue.address (keyAt map index).asString = keyAt map index :=
  write_read_keyAt keys nonneg bound

/-- An order on an integer key at a position within the map is the order on
its reading. -/
theorem order_keyAt_integer {map : RuntimeValue} {index : Int}
    (keys : KeysRead RuntimeValue.asInt RuntimeValue.integer map) (nonneg : 0 ≤ index)
    (bound : index < size map) (rank : ValueRanks) (other : Int) :
    RuntimeValue.order rank (keyAt map index) (.integer other) =
      compare (keyAt map index).asInt other := by
  rw [← RuntimeValue.order_integer rank, integer_asInt_keyAt keys nonneg bound]

theorem order_integer_keyAt {map : RuntimeValue} {index : Int}
    (keys : KeysRead RuntimeValue.asInt RuntimeValue.integer map) (nonneg : 0 ≤ index)
    (bound : index < size map) (rank : ValueRanks) (other : Int) :
    RuntimeValue.order rank (.integer other) (keyAt map index) =
      compare other (keyAt map index).asInt := by
  rw [← RuntimeValue.order_integer rank, integer_asInt_keyAt keys nonneg bound]

theorem order_keyAt_keyAt {map other : RuntimeValue} {index position : Int}
    (keys : KeysRead RuntimeValue.asInt RuntimeValue.integer map) (nonneg : 0 ≤ index)
    (bound : index < size map)
    (otherKeys : KeysRead RuntimeValue.asInt RuntimeValue.integer other) (low : 0 ≤ position)
    (high : position < size other) (rank : ValueRanks) :
    RuntimeValue.order rank (keyAt map index) (keyAt other position) =
      compare (keyAt map index).asInt (keyAt other position).asInt := by
  rw [← RuntimeValue.order_integer rank, integer_asInt_keyAt keys nonneg bound,
    integer_asInt_keyAt otherKeys low high]

/-- The empty map. -/
def empty (layout : Layout) : RuntimeValue := layout.build []

/-- The map with a key's value updated. -/
def update (layout : Layout) (discipline : Discipline) (map key value : RuntimeValue) :
    RuntimeValue :=
  layout.build (discipline.set (entriesOf map) key value)

/-- The map without a key. -/
def remove (layout : Layout) (discipline : Discipline) (map key : RuntimeValue) : RuntimeValue :=
  layout.build (discipline.del (entriesOf map) key)

/-- The representation invariant of a map value. -/
def Valid (discipline : Discipline) (map : RuntimeValue) : Prop :=
  discipline.Valid (entriesOf map)

@[simp, lir_denote_norm, grind =] theorem size_empty (layout : Layout) : size (empty layout) = 0 := by
  simp [size, empty]

@[simp, lir_denote_norm, grind =] theorem hasKey_empty (layout : Layout) (key : RuntimeValue) :
    hasKey (empty layout) key = false := by
  simp [hasKey, empty]

@[simp, lir_denote_norm, grind .] theorem valid_empty (layout : Layout) (discipline : Discipline) : Valid discipline (empty layout) := by
  cases discipline <;> simp [Valid, empty, Distinct, Ascending]

/-! Updates and removals, for either discipline. -/

@[simp, lir_denote_norm, grind =] theorem hasKey_update (layout : Layout) (discipline : Discipline)
    (map key value probe : RuntimeValue) :
    hasKey (update layout discipline map key value) probe =
      (decide (key = probe) || hasKey map probe) := by
  cases discipline <;> simp [hasKey, update, containsB_ordSet]

@[simp, lir_denote_norm, grind =] theorem valueAt_update (layout : Layout) (discipline : Discipline)
    (map key value probe : RuntimeValue) :
    valueAt (update layout discipline map key value) probe =
      if probe = key then value else valueAt map probe := by
  cases discipline <;> simp [valueAt, update, get_ordSet]

@[lir_denote_norm, grind =] theorem size_update (layout : Layout) {discipline : Discipline}
    {map : RuntimeValue} (valid : Valid discipline map) (key value : RuntimeValue) :
    size (update layout discipline map key value) =
      if hasKey map key then size map else size map + 1 := by
  cases discipline with
  | sequence =>
      simp only [size, update, entriesOf_build, Discipline.set_sequence, length_seqSet, hasKey]
      split <;> simp
  | ordered rank =>
      simp only [size, update, entriesOf_build, Discipline.set_ordered, hasKey]
      rw [length_ordSet rank valid]
      split <;> simp

@[lir_denote_norm, grind .] theorem valid_update (layout : Layout) {discipline : Discipline}
    {map : RuntimeValue} (valid : Valid discipline map) (key value : RuntimeValue) :
    Valid discipline (update layout discipline map key value) := by
  cases discipline with
  | sequence => simpa [Valid, update] using distinct_seqSet valid key value
  | ordered rank => simpa [Valid, update] using ascending_ordSet rank valid key value

theorem distinct_of_valid {discipline : Discipline} {entries : Entries}
    (valid : discipline.Valid entries) : Distinct entries := by
  cases discipline with
  | sequence => exact valid
  | ordered rank => exact distinct_of_ascending rank valid

@[lir_denote_norm, grind =] theorem hasKey_remove (layout : Layout) {discipline : Discipline}
    {map : RuntimeValue} (valid : Valid discipline map) (key probe : RuntimeValue) :
    hasKey (remove layout discipline map key) probe = (!decide (key = probe) && hasKey map probe) := by
  have distinct := distinct_of_valid valid
  cases discipline with
  | sequence => simpa [hasKey, remove] using containsB_seqDel distinct key probe
  | ordered _ => simpa [hasKey, remove] using containsB_ordDel distinct key probe

@[lir_denote_norm, grind =] theorem valueAt_remove (layout : Layout) {discipline : Discipline}
    {map : RuntimeValue} (valid : Valid discipline map) (key probe : RuntimeValue)
    (differs : probe ≠ key) : valueAt (remove layout discipline map key) probe = valueAt map probe := by
  cases discipline with
  | sequence => simpa [valueAt, remove] using get_seqDel valid key probe differs
  | ordered _ => simpa [valueAt, remove] using get_ordDel (entriesOf map) key probe differs

@[simp, lir_denote_norm, grind =] theorem size_remove (layout : Layout) (discipline : Discipline)
    (map key : RuntimeValue) :
    size (remove layout discipline map key) = if hasKey map key then size map - 1 else size map := by
  have positive (present : containsB (entriesOf map) key = true) : 0 < (entriesOf map).length := by
    cases h : entriesOf map with
    | nil => simp [h] at present
    | cons _ _ => simp
  cases discipline with
  | sequence =>
      simp only [size, remove, entriesOf_build, Discipline.del_sequence, length_seqDel, hasKey]
      split
      · have := positive ‹_›; omega
      · rfl
  | ordered _ =>
      simp only [size, remove, entriesOf_build, Discipline.del_ordered, length_ordDel, hasKey]
      split
      · have := positive ‹_›; omega
      · rfl

@[lir_denote_norm, grind .] theorem valid_remove (layout : Layout) {discipline : Discipline}
    {map : RuntimeValue} (valid : Valid discipline map) (key : RuntimeValue) :
    Valid discipline (remove layout discipline map key) := by
  cases discipline with
  | sequence => simpa [Valid, remove] using distinct_seqDel valid key
  | ordered rank => simpa [Valid, remove] using ascending_ordDel rank valid key

/-! Positions. -/

theorem indexOf_lt_length {entries : Entries} {key : RuntimeValue}
    (present : containsB entries key = true) : indexOf entries key < entries.length :=
  (getElem_indexOf entries key present).1

theorem fst_getElem_indexOf {entries : Entries} {key : RuntimeValue}
    (present : containsB entries key = true) :
    (entries[indexOf entries key]'(indexOf_lt_length present)).1 = key :=
  (getElem_indexOf entries key present).2

theorem indexOf_getElem {entries : Entries} (distinct : Distinct entries) (index : Nat)
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

theorem map_fst_seqSet_present {entries : Entries} {key : RuntimeValue} (value : RuntimeValue)
    (present : containsB entries key = true) :
    (seqSet entries key value).map Prod.fst = entries.map Prod.fst := by
  simp [seqSet, present, map_fst_replaceFirst]

theorem map_fst_ordSet_present (rank : ValueRanks) {entries : Entries}
    {key : RuntimeValue} (value : RuntimeValue) (present : containsB entries key = true)
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
        have hk : head.1 = key := (RuntimeValue.eq_of_order h).symm
        simp [hk]
      · rename_i h
        have hk : head.1 ≠ key := fun e => by rw [e, RuntimeValue.order_self] at h; cases h
        simp only [containsB, hk, decide_false, Bool.false_or] at present
        simp [ih present tail.2]

/-! Positions, over map values. -/

@[grind →] theorem rank_lt_size {map key : RuntimeValue} (present : hasKey map key = true) :
    rank map key < size map := by
  simp only [rank, size]; exact_mod_cast indexOf_lt_length present

theorem indexOf_eq_length {entries : Entries} {key : RuntimeValue}
    (absent : containsB entries key = false) : indexOf entries key = entries.length := by
  induction entries with
  | nil => rfl
  | cons entry rest ih =>
      simp only [containsB, Bool.or_eq_false_iff, decide_eq_false_iff_not] at absent
      simp [indexOf, absent.1, ih absent.2]

/-- A key's position is within the map exactly when the map has the key. -/
@[lir_denote_norm] theorem rank_lt_size_iff (map key : RuntimeValue) :
    rank map key < size map ↔ hasKey map key = true := by
  constructor
  · intro within
    cases present : hasKey map key
    · simp only [rank, size, hasKey] at within present
      rw [indexOf_eq_length present] at within; omega
    · rfl
  · exact rank_lt_size

@[lir_denote_norm] theorem size_le_rank_iff (map key : RuntimeValue) :
    size map ≤ rank map key ↔ hasKey map key = false := by
  rw [← Int.not_lt, rank_lt_size_iff, Bool.not_eq_true]

@[lir_denote_norm] theorem rank_nonneg (map key : RuntimeValue) : 0 ≤ rank map key := by
  simp [rank]

grind_pattern rank_nonneg => rank map key

@[lir_denote_norm, grind →] theorem keyAt_rank {map key : RuntimeValue} (present : hasKey map key = true) :
    keyAt map (rank map key) = key := by
  have bound := indexOf_lt_length present
  simp only [keyAt, rank, Int.natCast_nonneg, Int.not_lt.mpr, if_false, Int.toNat_natCast,
    List.getElem?_eq_getElem bound, Option.map_some, Option.getD_some]
  exact fst_getElem_indexOf present

/-- The key at a position within the map is one of its keys. -/
theorem hasKey_keyAt {map : RuntimeValue} {index : Int} (low : 0 ≤ index)
    (high : index < size map) : hasKey map (keyAt map index) = true := by
  have bound : index.toNat < (entriesOf map).length := by simp only [size] at high; omega
  simp only [hasKey, keyAt, Int.not_lt.mpr low, if_false, List.getElem?_eq_getElem bound,
    Option.map_some, Option.getD_some, containsB_iff]
  exact ⟨_, List.getElem_mem bound⟩

/-- The key at a position within the map is no key the map lacks. -/
theorem keyAt_asInt_ne_of_absent {map : RuntimeValue} {index value : Int}
    (keys : KeysRead RuntimeValue.asInt RuntimeValue.integer map) (low : 0 ≤ index)
    (high : index < size map) (absent : hasKey map (.integer value) = false) :
    (keyAt map index).asInt ≠ value := by
  intro same
  have present := hasKey_keyAt low high
  rw [← integer_asInt_keyAt keys low high, same, absent] at present
  contradiction

/-- Equal positions hold equal keys, stated by bounds, as omega reads it. -/
theorem keyAt_asInt_congr (map : RuntimeValue) (first second : Int) :
    first ≤ second → second ≤ first → (keyAt map first).asInt ≤ (keyAt map second).asInt := by
  intro below above; rw [show first = second by omega]; exact Int.le_refl _

/-- The keys of a valid ordered map ascend with their positions. -/
theorem order_keyAt_lt {rank : ValueRanks} {map : RuntimeValue}
    (valid : Valid (.ordered rank) map) {first second : Int} (low : 0 ≤ first)
    (before : first < second) (high : second < size map) :
    RuntimeValue.order rank (keyAt map first) (keyAt map second) = .lt := by
  have secondBound : second.toNat < (entriesOf map).length := by simp only [size] at high; omega
  have firstBound : first.toNat < (entriesOf map).length := by omega
  simp only [keyAt, Int.not_lt.mpr low, Int.not_lt.mpr (Int.le_of_lt (Int.lt_of_le_of_lt low before)),
    if_false, List.getElem?_eq_getElem firstBound, List.getElem?_eq_getElem secondBound,
    Option.map_some, Option.getD_some]
  exact List.pairwise_iff_getElem.mp valid _ _ firstBound secondBound (by omega)

/-- The same for integer keys, at their readings. -/
theorem keyAt_asInt_lt {rank : ValueRanks} {map : RuntimeValue}
    (valid : Valid (.ordered rank) map) (keys : KeysRead RuntimeValue.asInt RuntimeValue.integer map)
    {first second : Int} : 0 ≤ first → first < second → second < size map →
      (keyAt map first).asInt < (keyAt map second).asInt := by
  intro low before high
  have ordered := order_keyAt_lt valid low before high
  rw [order_keyAt_keyAt keys low (Int.lt_trans before high) keys
    (Int.le_of_lt (Int.lt_of_le_of_lt low before)) high] at ordered
  exact Int.compare_eq_lt.mp ordered

theorem rank_keyAt {discipline : Discipline} {map : RuntimeValue} (valid : Valid discipline map)
    {index : Int} (low : 0 ≤ index) (high : index < size map) : rank map (keyAt map index) = index := by
  have bound : index.toNat < (entriesOf map).length := by simp only [size] at high; omega
  have distinct := distinct_of_valid valid
  simp only [keyAt, rank, Int.not_lt.mpr low, if_false, List.getElem?_eq_getElem bound,
    Option.map_some, Option.getD_some]
  rw [indexOf_getElem distinct _ bound]; omega

/-- Positions within a valid map hold distinct keys. -/
theorem keyAt_eq_keyAt_iff {discipline : Discipline} {map : RuntimeValue}
    (valid : Valid discipline map) {first second : Int} (firstLow : 0 ≤ first)
    (firstHigh : first < size map) (secondLow : 0 ≤ second) (secondHigh : second < size map) :
    keyAt map first = keyAt map second ↔ first = second := by
  constructor
  · intro same
    rw [← rank_keyAt valid firstLow firstHigh, ← rank_keyAt valid secondLow secondHigh, same]
  · intro same; rw [same]

@[lir_denote_norm] theorem keyAt_update_present (layout : Layout) {discipline : Discipline}
    {map key : RuntimeValue} (valid : Valid discipline map) (present : hasKey map key = true)
    (value : RuntimeValue) (index : Int) :
    keyAt (update layout discipline map key value) index = keyAt map index := by
  have keys : (discipline.set (entriesOf map) key value).map Prod.fst = (entriesOf map).map Prod.fst := by
    cases discipline with
    | sequence => exact map_fst_seqSet_present value present
    | ordered rank => exact map_fst_ordSet_present rank value present valid
  simp only [keyAt, update, entriesOf_build]
  split
  · rfl
  · rw [← List.getElem?_map, keys, List.getElem?_map]

grind_pattern keyAt_eq_keyAt_iff => Valid discipline map, keyAt map first, keyAt map second

grind_pattern hasKey_keyAt => keyAt map index

grind_pattern rank_keyAt => Valid discipline map, rank map (keyAt map index)

/-- After an ordered removal, the positions from the removed key's on hold
the keys one position further. -/
@[lir_denote_norm, grind =] theorem keyAt_remove_ordered (layout : Layout) (order : ValueRanks)
    (map key : RuntimeValue) (index : Int) :
    keyAt (remove layout (.ordered order) map key) index =
      keyAt map (if index < rank map key then index else index + 1) := by
  have nonneg : 0 ≤ rank map key := by simp [rank]
  simp only [keyAt, remove, Discipline.del_ordered, entriesOf_build, ordDel_eq_eraseIdx,
    List.getElem?_eraseIdx, rank] at nonneg ⊢
  by_cases negative : index < 0
  · have : index < (indexOf (entriesOf map) key : Int) := by omega
    simp [negative, this]
  · by_cases before : index < (indexOf (entriesOf map) key : Int)
    · have : index.toNat < indexOf (entriesOf map) key := by omega
      simp [negative, before, this]
    · have : ¬index.toNat < indexOf (entriesOf map) key := by omega
      have shifted : (index + 1).toNat = index.toNat + 1 := by omega
      simp [negative, before, this, shifted, show ¬index + 1 < 0 by omega]

/-! ## Enumeration and bulk operations

The remaining roles of `designs/intrinsic-maps.md`, over the entries in the
order the map holds them: ascending for an ordered map. -/

/-- The elements of a vector value; none off a vector. -/
def elementsOf : RuntimeValue → List RuntimeValue
  | .vector elements => elements.toList
  | _ => []

/-- The keys in the order the map holds them. -/
def keysOf (map : RuntimeValue) : RuntimeValue := .vector ((entriesOf map).map Prod.fst).toArray

/-- The values in the order the map holds them. -/
def valuesOf (map : RuntimeValue) : RuntimeValue := .vector ((entriesOf map).map Prod.snd).toArray

/-- The largest key below a key, under the order. -/
def prevKey? (rank : ValueRanks) (map key : RuntimeValue) : Option RuntimeValue :=
  (((entriesOf map).filter fun entry => RuntimeValue.order rank entry.1 key == .lt).getLast?).map
    Prod.fst

/-- The smallest key above a key, under the order. -/
def nextKey? (rank : ValueRanks) (map key : RuntimeValue) : Option RuntimeValue :=
  ((entriesOf map).find? fun entry => RuntimeValue.order rank entry.1 key == .gt).map Prod.fst

/-- The map with the keys of one vector set to the values of another, in
order: a later equal key wins. -/
def updateAll (layout : Layout) (discipline : Discipline) (map keys values : RuntimeValue) :
    RuntimeValue :=
  layout.build (((elementsOf keys).zip (elementsOf values)).foldl
    (fun entries entry => discipline.set entries entry.1 entry.2) (entriesOf map))

/-- The map holding the entries of the first `count` positions. -/
def takeEntries (layout : Layout) (map : RuntimeValue) (count : Int) : RuntimeValue :=
  layout.build ((entriesOf map).take count.toNat)

/-- The map holding the entries from position `count` on. -/
def dropEntries (layout : Layout) (map : RuntimeValue) (count : Int) : RuntimeValue :=
  layout.build ((entriesOf map).drop count.toNat)

/-- The map with a key replaced by another at its position. -/
def replaceKey (layout : Layout) (map old new : RuntimeValue) : RuntimeValue :=
  layout.build ((entriesOf map).map fun entry => if entry.1 = old then (new, entry.2) else entry)

/-- Adding several entries aborts on vectors of different lengths, a key
the map holds, or a key twice. -/
def AbortsAddAll (map keys values : RuntimeValue) : Prop :=
  (elementsOf keys).length ≠ (elementsOf values).length ∨
    (∃ key ∈ elementsOf keys, hasKey map key = true) ∨ ¬(elementsOf keys).Nodup

/-- Building a map aborts on vectors of different lengths or a key twice. -/
def AbortsNewFrom (keys values : RuntimeValue) : Prop :=
  (elementsOf keys).length ≠ (elementsOf values).length ∨ ¬(elementsOf keys).Nodup

/-- Updating several entries aborts on vectors of different lengths. -/
def AbortsUpsertAll (keys values : RuntimeValue) : Prop :=
  (elementsOf keys).length ≠ (elementsOf values).length

/-- Appending a map aborts on a key both hold. -/
def AbortsAppendDisjoint (map other : RuntimeValue) : Prop :=
  ∃ key, hasKey map key = true ∧ hasKey other key = true

/-- Replacing a key aborts when the map lacks it, or when the new key breaks
the representation invariant at its position. -/
def AbortsReplaceKey (layout : Layout) (discipline : Discipline) (map old new : RuntimeValue) :
    Prop :=
  hasKey map old = false ∨ ¬Valid discipline (replaceKey layout map old new)

@[simp, lir_denote_norm] theorem elementsOf_vector (elements : Array RuntimeValue) :
    elementsOf (.vector elements) = elements.toList := rfl

end LeanerIR.Maps
