-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Order
import VerexIR.Proofs.Denote.Attr

/-!
# The intrinsic map model

The model of an intrinsic map value (`doc/designs/intrinsic-maps.md`): the list
of its `(key, value)` entries, under one of two disciplines. A sequence keeps
insertion order: an update replaces the entry in place or appends one, and a
removal moves the last entry into the removed position. An ordered map keeps
its keys strictly ascending under the structural order. The operations are
defined for every list, and the lemmas that need distinct or ascending keys
take them as hypotheses.
-/

namespace VerexIR.Maps

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

@[simp] theorem containsB_nil (key : RuntimeValue) : containsB [] key = false :=
  rfl

@[simp] theorem containsB_cons (entry : RuntimeValue × RuntimeValue)
    (rest : Entries) (key : RuntimeValue) :
    containsB (entry :: rest) key = (decide (entry.1 = key) || containsB rest key) := rfl

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

/-! ## Lemmas of the sequence discipline -/

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

/-! ## Lemmas of the ordered discipline -/

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
or one variant of an enum) holds its entries in its last field, a vector of
two-field entry values. An entries layout has that field alone, its entries
values of the entry declaration. A table layout holds the handle of the
table first, then its cached length if the owner caches one, then the
hidden contents field of entry tuples (`doc/designs/intrinsic-maps.md`,
"Views"). -/
structure Layout where
  owner : StructHandle
  variant : Option String
  /-- The declaration of the entry values; `none` for the tuples of a table. -/
  entry : Option StructHandle
  /-- A table layout: the first field is the table's handle. -/
  handle : Bool
  /-- A table layout caching its length before its contents. -/
  length : Bool
  /-- A table layout of an owner declared without fields, whose first field
  is the Boolean placeholder Move's compilers give an empty struct
  (`dummy_field`); the value keeps it as a table keeps its handle. -/
  dummy : Bool
  /-- The ghost slots of an owner declaring ghost fields, after its fields and
  before its hidden contents (`doc/designs/ghost-fields.md`). -/
  ghosts : Nat := 0

/-- The fields before an owner's ghost slots: its handle or placeholder, and
its cached length. -/
def Layout.leading (layout : Layout) : Nat :=
  (if layout.handle || layout.dummy then 1 else 0) + (if layout.length then 1 else 0)

/-- A ghost slot of a map value, by its position among the slots. -/
def Layout.slot (layout : Layout) (map : RuntimeValue) (k : Nat) : RuntimeValue :=
  map.field (layout.leading + k)

/-- The ghost slots a map value holds, in order. -/
def Layout.slotsOf (layout : Layout) (map : RuntimeValue) : List RuntimeValue :=
  (List.range layout.ghosts).map (layout.slot map)

/-- The ghost slots of a map a specification builds: zero, as the Move
Prover's. -/
def Layout.freshSlots (layout : Layout) : List RuntimeValue :=
  List.replicate layout.ghosts (.ghost (.integer 0))

/-- The entries a map value holds. -/
def entriesOf (map : RuntimeValue) : Entries :=
  match map with
  | .nominal _ _ fields =>
      match fields.back? with
      | some (.vector elements) => elements.toList.map fun entry => (entry.field 0, entry.field 1)
      | _ => []
  | _ => []

/-- An entry's value. -/
def Layout.entryValue (layout : Layout) (entry : RuntimeValue × RuntimeValue) : RuntimeValue :=
  match layout.entry with
  | some declaration => .nominal declaration none #[entry.1, entry.2]
  | none => .tuple #[entry.1, entry.2]

/-- The map value holding entries, at a handle for a table layout, or at its
placeholder for one of an owner without fields, with its ghost slots, fresh
unless given. -/
def Layout.build (layout : Layout) (handle : RuntimeValue) (entries : Entries)
    (slots : List RuntimeValue := layout.freshSlots) : RuntimeValue :=
  .nominal layout.owner layout.variant
    ((((if layout.handle || layout.dummy then #[handle] else #[]) ++
        (if layout.length then #[RuntimeValue.integer entries.length] else #[])) ++
        slots.toArray).push
      (.vector (entries.map layout.entryValue).toArray))

/-- The handle of a map value: of a table layout, its first field, which a
placeholder layout keeps the same way. -/
def Layout.handleOf (layout : Layout) (map : RuntimeValue) : RuntimeValue :=
  if layout.handle || layout.dummy then map.field 0 else .unit

/-- A map value with its ghost slots replaced, the `k`th by `stamp k`: what
a structural mutation leaves of an owner's slots, chosen afresh. -/
def Layout.restamp (layout : Layout) (map : RuntimeValue) (stamp : Nat → RuntimeValue) :
    RuntimeValue :=
  layout.build (layout.handleOf map) (entriesOf map) ((List.range layout.ghosts).map stamp)

@[simp] theorem Layout.field_entryValue_zero (layout : Layout) (entry : RuntimeValue × RuntimeValue) :
    (layout.entryValue entry).field 0 = entry.1 := by
  cases declaration : layout.entry <;> simp [Layout.entryValue, declaration, RuntimeValue.field]

@[simp] theorem Layout.field_entryValue_one (layout : Layout) (entry : RuntimeValue × RuntimeValue) :
    (layout.entryValue entry).field 1 = entry.2 := by
  cases declaration : layout.entry <;> simp [Layout.entryValue, declaration, RuntimeValue.field]

@[simp] theorem entriesOf_build (layout : Layout) (handle : RuntimeValue)
    (slots : List RuntimeValue) (entries : Entries) :
    entriesOf (layout.build handle entries slots) = entries := by
  simp [entriesOf, Layout.build, Array.back?_push, List.map_map, Function.comp_def]

/-- The leading fields of a built value, then its slots. -/
private theorem Layout.build_toList (layout : Layout) (handle : RuntimeValue)
    (slots : List RuntimeValue) (entries : Entries) :
    ∃ leading : List RuntimeValue, leading.length = layout.leading ∧
      layout.build handle entries slots =
        .nominal layout.owner layout.variant
          (leading ++ slots ++ [RuntimeValue.vector (entries.map layout.entryValue).toArray]).toArray := by
  refine ⟨(if layout.handle || layout.dummy then [handle] else []) ++
    (if layout.length then [RuntimeValue.integer entries.length] else []), ?_, ?_⟩
  · unfold Layout.leading
    split <;> split <;> rfl
  · unfold Layout.build
    congr 1
    apply Array.ext'
    simp only [Array.toList_push, Array.toList_append, List.toList_toArray, List.append_assoc]
    split <;> split <;> simp

/-- A built value's slot is the one given. -/
theorem Layout.slot_build (layout : Layout) (handle : RuntimeValue) (slots : List RuntimeValue)
    (entries : Entries) {k : Nat} (bound : k < slots.length) :
    layout.slot (layout.build handle entries slots) k = slots[k] := by
  obtain ⟨leading, length, built⟩ := layout.build_toList handle slots entries
  rw [Layout.slot, built]
  simp only [RuntimeValue.field, List.getElem?_toArray, List.append_assoc]
  rw [List.getElem?_append_right (by omega), length, Nat.add_sub_cancel_left,
    List.getElem?_append_left bound, List.getElem?_eq_getElem bound]
  rfl

/-- A value rebuilt with a map's own slots keeps each. -/
theorem Layout.slot_build_slotsOf (layout : Layout) (handle : RuntimeValue)
    (map : RuntimeValue) (entries : Entries) {k : Nat} (bound : k < layout.ghosts) :
    layout.slot (layout.build handle entries (layout.slotsOf map)) k = layout.slot map k := by
  rw [Layout.slot_build layout handle _ entries (by simpa [Layout.slotsOf] using bound)]
  simp [Layout.slotsOf]

/-- The slots of a built value are the ones given, as many as the layout
holds. -/
theorem Layout.slotsOf_build (layout : Layout) (handle : RuntimeValue) (entries : Entries)
    (slots : List RuntimeValue) (length : slots.length = layout.ghosts) :
    layout.slotsOf (layout.build handle entries slots) = slots := by
  unfold Layout.slotsOf
  apply List.ext_getElem
  · simp [length]
  · intro i _ bound
    simp only [List.getElem_map, List.getElem_range]
    exact layout.slot_build handle slots entries bound

/-- A value built with fresh slots holds fresh slots. -/
@[simp] theorem Layout.slotsOf_build_fresh (layout : Layout) (handle : RuntimeValue)
    (entries : Entries) : layout.slotsOf (layout.build handle entries) = layout.freshSlots :=
  layout.slotsOf_build handle entries _ (by simp [Layout.freshSlots])

/-- The entries of a value whose one field is a vector of entries. -/
@[simp] theorem entriesOf_single (owner : StructHandle) (variant : Option String)
    (elements : Array RuntimeValue) :
    entriesOf (.nominal owner variant #[.vector elements]) =
      elements.toList.map fun entry => (entry.field 0, entry.field 1) := by
  simp [entriesOf, Array.back?]


/-! ## Map values

The model a specification sees of a map value, and the lemmas a proof uses. -/

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
vector, stated by bounds: its size stays the abstract size the lemmas read. -/
theorem size_nominal_vector_bounds (owner : StructHandle) (variant : Option String)
    (elements : Array RuntimeValue) :
    0 ≤ size (.nominal owner variant #[.vector elements]) ∧
      size (.nominal owner variant #[.vector elements]) ≤ elements.size ∧
      (elements.size : Int) ≤ size (.nominal owner variant #[.vector elements]) := by
  simp [size, entriesOf, RuntimeValue.field]

/-- A map value holding the encodings of a list of entries has as many
entries as the list: the size of an abstract map's encoding is its carrier's
length, which the carrier bounds (`SpecEntries`): the size of a carrier's
encoding reads as the carrier's length, an integer the bounds name. A
fact a leaf asserts, not a normal form: the lemmas read `size` of the map. -/
theorem size_nominal_vector_map {α : Type} (owner : StructHandle)
    (variant : Option String) (encode : α → RuntimeValue) (entries : List α) :
    size (.nominal owner variant #[.vector (entries.map encode).toArray]) = entries.length := by
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

/-- Every key has a value of the source key type, including that type's
range certificate. Scalar round trips alone do not preserve those bounds. -/
def KeysEncoded {α : Type} (encode : α → RuntimeValue) (map : RuntimeValue) : Prop :=
  ∀ entry ∈ entriesOf map, ∃ key, encode key = entry.1

theorem encoded_keyAt {α : Type} {encode : α → RuntimeValue}
    {map : RuntimeValue} {index : Int} (keys : KeysEncoded encode map)
    (nonneg : 0 ≤ index) (bound : index < size map) :
    ∃ key, encode key = keyAt map index := by
  have within : index.toNat < (entriesOf map).length := by unfold size at bound; omega
  simp only [keyAt, show ¬index < 0 by omega, if_false, List.getElem?_eq_getElem within,
    Option.map_some, Option.getD_some]
  exact keys _ (List.getElem_mem within)

theorem write_read_keyAt {α : Type} {read : RuntimeValue → α} {write : α → RuntimeValue}
    {map : RuntimeValue} {index : Int} (keys : KeysRead read write map) (nonneg : 0 ≤ index)
    (bound : index < size map) : write (read (keyAt map index)) = keyAt map index := by
  have within : index.toNat < (entriesOf map).length := by unfold size at bound; omega
  simp only [keyAt, show ¬ index < 0 by omega, if_false, List.getElem?_eq_getElem within,
    Option.map_some, Option.getD_some]
  exact keys _ (List.getElem_mem within)

/-- The keys of entries encoded one by one, the key as their first field, are
read back when every encoded first field is. -/
theorem keysRead_map {α β : Type} {read : RuntimeValue → α}
    {write : α → RuntimeValue} (owner : StructHandle) (variant : Option String)
    (encode : β → RuntimeValue) (entries : Array β)
    (roundTrip : ∀ value, write (read ((encode value).field 0)) = (encode value).field 0) :
    KeysRead read write (.nominal owner variant #[.vector (entries.map encode)]) := by
  intro pair member
  simp only [entriesOf_single, Array.toList_map, List.map_map, List.mem_map,
    Function.comp_def] at member
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

/-- The empty map of an entries layout, its ghost slots fresh. -/
def empty (layout : Layout) : RuntimeValue :=
  layout.build (if layout.dummy then .bool false else .unit) []

@[simp] theorem Layout.slotsOf_empty (layout : Layout) :
    layout.slotsOf (empty layout) = layout.freshSlots := by
  unfold empty
  exact layout.slotsOf_build_fresh _ _

/-- The map with a key's value updated, at the map's handle and slots. -/
def update (layout : Layout) (discipline : Discipline) (map key value : RuntimeValue) :
    RuntimeValue :=
  layout.build (layout.handleOf map) (discipline.set (entriesOf map) key value) (layout.slotsOf map)

/-- The map without a key, at the map's handle and slots. -/
def remove (layout : Layout) (discipline : Discipline) (map key : RuntimeValue) : RuntimeValue :=
  layout.build (layout.handleOf map) (discipline.del (entriesOf map) key) (layout.slotsOf map)

/-- A table layout's map holds its handle first. -/
@[simp] theorem Layout.field_zero_build (layout : Layout) (handle : RuntimeValue)
    (slots : List RuntimeValue) (entries : Entries) (h : layout.handle = true) :
    (layout.build handle entries slots).field 0 = handle := by
  cases hl : layout.length <;> simp [Layout.build, RuntimeValue.field, h, hl]

/-! The model operations keep a map's ghost slots; a map built afresh holds
fresh ones. -/

@[simp] theorem Layout.slot_update (layout : Layout) (discipline : Discipline)
    (map key value : RuntimeValue) {k : Nat} (bound : k < layout.ghosts) :
    layout.slot (update layout discipline map key value) k = layout.slot map k :=
  layout.slot_build_slotsOf _ _ _ bound

@[simp] theorem Layout.slot_remove (layout : Layout) (discipline : Discipline)
    (map key : RuntimeValue) {k : Nat} (bound : k < layout.ghosts) :
    layout.slot (remove layout discipline map key) k = layout.slot map k :=
  layout.slot_build_slotsOf _ _ _ bound

@[simp] theorem Layout.slot_empty (layout : Layout) {k : Nat}
    (bound : k < layout.ghosts) :
    layout.slot (empty layout) k = .ghost (.integer 0) := by
  unfold empty
  rw [Layout.slot_build layout _ _ _ (by simpa [Layout.freshSlots] using bound)]
  simp [Layout.freshSlots]

@[simp] theorem Layout.slot_restamp (layout : Layout) (map : RuntimeValue)
    (stamp : Nat → RuntimeValue) {k : Nat} (bound : k < layout.ghosts) :
    layout.slot (layout.restamp map stamp) k = stamp k := by
  unfold Layout.restamp
  rw [Layout.slot_build layout _ _ _ (by simpa using bound)]
  simp

@[simp] theorem entriesOf_restamp (layout : Layout) (map : RuntimeValue)
    (stamp : Nat → RuntimeValue) : entriesOf (layout.restamp map stamp) = entriesOf map := by
  simp [Layout.restamp]

@[simp] theorem field_zero_restamp (layout : Layout) (map : RuntimeValue)
    (stamp : Nat → RuntimeValue) (h : layout.handle = true) :
    (layout.restamp map stamp).field 0 = map.field 0 := by
  simp [Layout.restamp, Layout.handleOf, Layout.field_zero_build _ _ _ _ h, h]

/-! Restamping keeps every observation of the entries. -/

@[simp, grind =] theorem size_restamp (layout : Layout) (map : RuntimeValue)
    (stamp : Nat → RuntimeValue) : size (layout.restamp map stamp) = size map := by
  simp only [size, entriesOf_restamp]

@[simp, grind =] theorem hasKey_restamp (layout : Layout) (map key : RuntimeValue)
    (stamp : Nat → RuntimeValue) : hasKey (layout.restamp map stamp) key = hasKey map key := by
  simp only [hasKey, entriesOf_restamp]

@[simp, grind =] theorem valueAt_restamp (layout : Layout) (map key : RuntimeValue)
    (stamp : Nat → RuntimeValue) : valueAt (layout.restamp map stamp) key = valueAt map key := by
  simp only [valueAt, entriesOf_restamp]

@[simp, grind =] theorem keyAt_restamp (layout : Layout) (map : RuntimeValue)
    (index : Int) (stamp : Nat → RuntimeValue) :
    keyAt (layout.restamp map stamp) index = keyAt map index := by
  simp only [keyAt, entriesOf_restamp]

@[simp, grind =] theorem rank_restamp (layout : Layout) (map key : RuntimeValue)
    (stamp : Nat → RuntimeValue) : rank (layout.restamp map stamp) key = rank map key := by
  simp only [rank, entriesOf_restamp]

/-! Updating and removing keep a table's handle. -/

@[simp] theorem field_zero_update (layout : Layout) (discipline : Discipline)
    (map key value : RuntimeValue) (h : layout.handle = true) :
    (update layout discipline map key value).field 0 = map.field 0 := by
  simp [update, Layout.handleOf, Layout.field_zero_build _ _ _ _ h, h]

@[simp] theorem field_zero_remove (layout : Layout) (discipline : Discipline)
    (map key : RuntimeValue) (h : layout.handle = true) :
    (remove layout discipline map key).field 0 = map.field 0 := by
  simp [remove, Layout.handleOf, Layout.field_zero_build _ _ _ _ h, h]

/-! ## Tree maps

The runtime's `BigOrderedMap` holds its entries in the leaves of a tree of
nodes: a root node in one field and the other nodes in an allocator in
another, each node an entries map from keys to children, of which those of
one variant hold a value. Its abstract value reads those entries off the
concrete value (`SemanticOperations.normalize`). -/

/-- The values of a declaration a value holds, through fields, vectors, and
tuples, not looking into those found: the nodes an allocator holds. -/
def holdings (declaration : StructHandle) : RuntimeValue → List RuntimeValue
  | value@(.nominal handle _ fields) =>
      if handle == declaration then [value]
      else fields.attach.toList.flatMap fun ⟨field, _⟩ => holdings declaration field
  | .vector elements => elements.attach.toList.flatMap fun ⟨element, _⟩ =>
      holdings declaration element
  | .tuple elements => elements.attach.toList.flatMap fun ⟨element, _⟩ =>
      holdings declaration element
  | _ => []

/-- The entries the leaves of a tree owner hold: of its root node, at its
field `root`, and of each node its field `nodes` holds, the children, in
the entries map at a node's field `children`, of the variant `leaf`, at
the value that variant holds. -/
def treeEntries (node : StructHandle) (root nodes children : Nat) (leaf : String)
    (map : RuntimeValue) : Entries :=
  (map.field root :: holdings node (map.field nodes)).flatMap fun node =>
    (entriesOf (node.field children)).filterMap fun entry =>
      match entry.2 with
      | .nominal _ (some variant) fields =>
          if variant == leaf then fields[0]?.map (entry.1, ·) else none
      | _ => none

/-- The representation invariant of a map value. -/
def Valid (discipline : Discipline) (map : RuntimeValue) : Prop :=
  discipline.Valid (entriesOf map)

@[simp, grind =] theorem size_empty (layout : Layout) : size (empty layout) = 0 := by
  simp [size, empty]

@[simp, grind =] theorem hasKey_empty (layout : Layout) (key : RuntimeValue) :
    hasKey (empty layout) key = false := by
  simp [hasKey, empty]

@[simp, grind =] theorem valid_restamp (layout : Layout) (discipline : Discipline)
    (map : RuntimeValue) (stamp : Nat → RuntimeValue) :
    Valid discipline (layout.restamp map stamp) ↔ Valid discipline map := by
  simp only [Valid, entriesOf_restamp]

@[simp, grind .] theorem valid_empty (layout : Layout) (discipline : Discipline) : Valid discipline (empty layout) := by
  cases discipline <;> simp [Valid, empty, Distinct, Ascending]

/-! A map built at a handle, as a table allocation yields it. -/

@[simp] theorem size_build (layout : Layout) (handle : RuntimeValue)
    (slots : List RuntimeValue) (entries : Entries) :
    size (layout.build handle entries slots) = (entries.length : Int) := by
  simp [size]

@[simp] theorem hasKey_build (layout : Layout) (handle key : RuntimeValue)
    (slots : List RuntimeValue) (entries : Entries) :
    hasKey (layout.build handle entries slots) key = containsB entries key := by
  simp [hasKey]

@[simp] theorem valueAt_build (layout : Layout) (handle key : RuntimeValue)
    (slots : List RuntimeValue) (entries : Entries) :
    valueAt (layout.build handle entries slots) key = get entries key := by
  simp [valueAt]

@[simp] theorem valid_build_nil (layout : Layout) (handle : RuntimeValue)
    (slots : List RuntimeValue) (discipline : Discipline) :
    Valid discipline (layout.build handle [] slots) := by
  cases discipline <;> simp [Valid, Distinct, Ascending]

/-! Updates and removals, for either discipline. -/

@[simp, grind =] theorem hasKey_update (layout : Layout) (discipline : Discipline)
    (map key value probe : RuntimeValue) :
    hasKey (update layout discipline map key value) probe =
      (decide (key = probe) || hasKey map probe) := by
  cases discipline <;> simp [hasKey, update, containsB_ordSet]

@[simp, grind =] theorem valueAt_update (layout : Layout) (discipline : Discipline)
    (map key value probe : RuntimeValue) :
    valueAt (update layout discipline map key value) probe =
      if probe = key then value else valueAt map probe := by
  cases discipline <;> simp [valueAt, update, get_ordSet]

@[grind =] theorem size_update (layout : Layout) {discipline : Discipline}
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

/-- Under the sequence discipline the size lemma needs no validity: an update
replaces the key's first entry or appends. A table's contents follow it. -/
theorem size_update_sequence (layout : Layout) (map key value : RuntimeValue) :
    size (update layout .sequence map key value) =
      if hasKey map key then size map else size map + 1 := by
  simp only [size, update, entriesOf_build, Discipline.set_sequence, length_seqSet, hasKey]
  split <;> simp

@[grind .] theorem valid_update (layout : Layout) {discipline : Discipline}
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

@[grind =] theorem hasKey_remove (layout : Layout) {discipline : Discipline}
    {map : RuntimeValue} (valid : Valid discipline map) (key probe : RuntimeValue) :
    hasKey (remove layout discipline map key) probe = (!decide (key = probe) && hasKey map probe) := by
  have distinct := distinct_of_valid valid
  cases discipline with
  | sequence => simpa [hasKey, remove] using containsB_seqDel distinct key probe
  | ordered _ => simpa [hasKey, remove] using containsB_ordDel distinct key probe

@[grind =] theorem valueAt_remove (layout : Layout) {discipline : Discipline}
    {map : RuntimeValue} (valid : Valid discipline map) (key probe : RuntimeValue)
    (differs : probe ≠ key) : valueAt (remove layout discipline map key) probe = valueAt map probe := by
  cases discipline with
  | sequence => simpa [valueAt, remove] using get_seqDel valid key probe differs
  | ordered _ => simpa [valueAt, remove] using get_ordDel (entriesOf map) key probe differs

@[simp, grind =] theorem size_remove (layout : Layout) (discipline : Discipline)
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

@[grind .] theorem valid_remove (layout : Layout) {discipline : Discipline}
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
theorem rank_lt_size_iff (map key : RuntimeValue) :
    rank map key < size map ↔ hasKey map key = true := by
  constructor
  · intro within
    cases present : hasKey map key
    · simp only [rank, size, hasKey] at within present
      rw [indexOf_eq_length present] at within; omega
    · rfl
  · exact rank_lt_size

theorem size_le_rank_iff (map key : RuntimeValue) :
    size map ≤ rank map key ↔ hasKey map key = false := by
  rw [← Int.not_lt, rank_lt_size_iff, Bool.not_eq_true]

theorem rank_nonneg (map key : RuntimeValue) : 0 ≤ rank map key := by
  simp [rank]

grind_pattern rank_nonneg => rank map key

@[grind →] theorem keyAt_rank {map key : RuntimeValue} (present : hasKey map key = true) :
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

theorem keyAt_update_present (layout : Layout) {discipline : Discipline}
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
@[grind =] theorem keyAt_remove_ordered (layout : Layout) (order : ValueRanks)
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

The remaining roles of `doc/designs/intrinsic-maps.md`, over the entries in the
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

@[simp, grind =] theorem keysOf_restamp (layout : Layout) (map : RuntimeValue)
    (stamp : Nat → RuntimeValue) : keysOf (layout.restamp map stamp) = keysOf map := by
  simp only [keysOf, entriesOf_restamp]

@[simp, grind =] theorem valuesOf_restamp (layout : Layout) (map : RuntimeValue)
    (stamp : Nat → RuntimeValue) : valuesOf (layout.restamp map stamp) = valuesOf map := by
  simp only [valuesOf, entriesOf_restamp]

@[simp, grind =] theorem prevKey?_restamp (layout : Layout) (rank : ValueRanks)
    (map key : RuntimeValue) (stamp : Nat → RuntimeValue) :
    prevKey? rank (layout.restamp map stamp) key = prevKey? rank map key := by
  simp only [prevKey?, entriesOf_restamp]

@[simp, grind =] theorem nextKey?_restamp (layout : Layout) (rank : ValueRanks)
    (map key : RuntimeValue) (stamp : Nat → RuntimeValue) :
    nextKey? rank (layout.restamp map stamp) key = nextKey? rank map key := by
  simp only [nextKey?, entriesOf_restamp]

/-- The map with the keys of one vector set to the values of another, in
order: a later equal key wins. -/
def updateAll (layout : Layout) (discipline : Discipline) (map keys values : RuntimeValue) :
    RuntimeValue :=
  layout.build (layout.handleOf map)
    (((elementsOf keys).zip (elementsOf values)).foldl
      (fun entries entry => discipline.set entries entry.1 entry.2) (entriesOf map))
    (layout.slotsOf map)

/-- The map holding the entries of the first `count` positions. -/
def takeEntries (layout : Layout) (map : RuntimeValue) (count : Int) : RuntimeValue :=
  layout.build (layout.handleOf map) ((entriesOf map).take count.toNat) (layout.slotsOf map)

/-- The map holding the entries from position `count` on, at a fresh value
of an entries layout. -/
def dropEntries (layout : Layout) (map : RuntimeValue) (count : Int) : RuntimeValue :=
  layout.build .unit ((entriesOf map).drop count.toNat)

/-- The map with a key replaced by another at its position. -/
def replaceKey (layout : Layout) (map old new : RuntimeValue) : RuntimeValue :=
  layout.build (layout.handleOf map)
    ((entriesOf map).map fun entry => if entry.1 = old then (new, entry.2) else entry)
    (layout.slotsOf map)

@[simp] theorem Layout.slot_updateAll (layout : Layout) (discipline : Discipline)
    (map keys values : RuntimeValue) {k : Nat} (bound : k < layout.ghosts) :
    layout.slot (updateAll layout discipline map keys values) k = layout.slot map k :=
  layout.slot_build_slotsOf _ _ _ bound

@[simp] theorem Layout.slot_takeEntries (layout : Layout) (map : RuntimeValue)
    (count : Int) {k : Nat} (bound : k < layout.ghosts) :
    layout.slot (takeEntries layout map count) k = layout.slot map k :=
  layout.slot_build_slotsOf _ _ _ bound

@[simp] theorem Layout.slot_replaceKey (layout : Layout) (map old new : RuntimeValue)
    {k : Nat} (bound : k < layout.ghosts) :
    layout.slot (replaceKey layout map old new) k = layout.slot map k :=
  layout.slot_build_slotsOf _ _ _ bound

@[simp] theorem field_zero_updateAll (layout : Layout) (discipline : Discipline)
    (map keys values : RuntimeValue) (h : layout.handle = true) :
    (updateAll layout discipline map keys values).field 0 = map.field 0 := by
  simp [updateAll, Layout.handleOf, Layout.field_zero_build _ _ _ _ h, h]

@[simp] theorem field_zero_takeEntries (layout : Layout) (map : RuntimeValue)
    (count : Int) (h : layout.handle = true) :
    (takeEntries layout map count).field 0 = map.field 0 := by
  simp [takeEntries, Layout.handleOf, Layout.field_zero_build _ _ _ _ h, h]

@[simp] theorem field_zero_replaceKey (layout : Layout) (map old new : RuntimeValue)
    (h : layout.handle = true) : (replaceKey layout map old new).field 0 = map.field 0 := by
  simp [replaceKey, Layout.handleOf, Layout.field_zero_build _ _ _ _ h, h]

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

@[simp] theorem elementsOf_vector (elements : Array RuntimeValue) :
    elementsOf (.vector elements) = elements.toList := rfl


/-! ## Lemmas of the bulk operations -/

theorem containsB_eq_decide_mem (entries : Entries) (probe : RuntimeValue) :
    containsB entries probe = decide (probe ∈ entries.map Prod.fst) := by
  apply Bool.eq_iff_iff.mpr
  simp only [containsB_iff, decide_eq_true_eq, List.mem_map]
  constructor
  · rintro ⟨value, mem⟩; exact ⟨(probe, value), mem, rfl⟩
  · rintro ⟨⟨key, value⟩, mem, rfl⟩; exact ⟨value, mem⟩

theorem mem_fst_zip {α β : Type} {k : α} :
    ∀ {l₁ : List α} {l₂ : List β}, k ∈ (l₁.zip l₂).map Prod.fst → k ∈ l₁
  | [], _, h => by simp at h
  | _ :: _, [], h => by simp at h
  | a :: l₁, b :: l₂, h => by
      simp only [List.zip_cons_cons, List.map_cons, List.mem_cons] at h
      rcases h with h | h
      · simp [h]
      · exact List.mem_cons_of_mem _ (mem_fst_zip h)

theorem Discipline.valid_set {discipline : Discipline} {entries : Entries}
    (valid : discipline.Valid entries) (key value : RuntimeValue) :
    discipline.Valid (discipline.set entries key value) := by
  cases discipline with
  | sequence => exact distinct_seqSet valid key value
  | ordered rank => exact ascending_ordSet rank valid key value

theorem Discipline.containsB_set (discipline : Discipline) (entries : Entries)
    (key value probe : RuntimeValue) :
    containsB (discipline.set entries key value) probe = (decide (key = probe) || containsB entries probe) := by
  cases discipline <;> simp [containsB_ordSet] <;> congr

theorem Discipline.length_set_bounds (discipline : Discipline) (entries : Entries)
    (key value : RuntimeValue) :
    entries.length ≤ (discipline.set entries key value).length ∧
      (discipline.set entries key value).length ≤ entries.length + 1 := by
  cases discipline with
  | sequence => simp only [Discipline.set_sequence, length_seqSet]; split <;> omega
  | ordered rank =>
      simp only [Discipline.set_ordered]
      induction entries with
      | nil => simp [ordSet]
      | cons entry rest ih =>
          simp only [ordSet]
          split <;> simp only [List.length_cons] <;> omega

theorem Discipline.length_set {discipline : Discipline} {entries : Entries}
    (valid : discipline.Valid entries) (key value : RuntimeValue) :
    (discipline.set entries key value).length =
      if containsB entries key then entries.length else entries.length + 1 := by
  cases discipline with
  | sequence => exact length_seqSet entries key value
  | ordered rank => exact length_ordSet rank valid key value

theorem Discipline.valid_foldl_set (discipline : Discipline)
    (pairs : List (RuntimeValue × RuntimeValue)) {entries : Entries}
    (valid : discipline.Valid entries) :
    discipline.Valid (pairs.foldl (fun entries entry => discipline.set entries entry.1 entry.2) entries) := by
  induction pairs generalizing entries with
  | nil => exact valid
  | cons pair rest ih => exact ih (Discipline.valid_set valid pair.1 pair.2)

theorem Discipline.containsB_foldl_set (discipline : Discipline)
    (pairs : List (RuntimeValue × RuntimeValue)) (entries : Entries) (probe : RuntimeValue) :
    containsB (pairs.foldl (fun entries entry => discipline.set entries entry.1 entry.2) entries) probe =
      (decide (probe ∈ pairs.map Prod.fst) || containsB entries probe) := by
  induction pairs generalizing entries with
  | nil => simp
  | cons pair rest ih =>
      rw [List.foldl_cons, ih, Discipline.containsB_set]
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

theorem Discipline.length_foldl_set_bounds (discipline : Discipline)
    (pairs : List (RuntimeValue × RuntimeValue)) (entries : Entries) :
    entries.length ≤
        (pairs.foldl (fun entries entry => discipline.set entries entry.1 entry.2) entries).length ∧
      (pairs.foldl (fun entries entry => discipline.set entries entry.1 entry.2) entries).length ≤
        entries.length + pairs.length := by
  induction pairs generalizing entries with
  | nil => simp
  | cons pair rest ih =>
      rw [List.foldl_cons]
      have step := Discipline.length_set_bounds discipline entries pair.1 pair.2
      have := ih (discipline.set entries pair.1 pair.2)
      simp only [List.length_cons]; omega

theorem Discipline.length_foldl_set_fresh (discipline : Discipline)
    (pairs : List (RuntimeValue × RuntimeValue)) {entries : Entries}
    (valid : discipline.Valid entries) (nodup : (pairs.map Prod.fst).Nodup)
    (fresh : ∀ pair ∈ pairs, containsB entries pair.1 = false) :
    (pairs.foldl (fun entries entry => discipline.set entries entry.1 entry.2) entries).length =
      entries.length + pairs.length := by
  induction pairs generalizing entries with
  | nil => simp
  | cons pair rest ih =>
      rw [List.foldl_cons]
      simp only [List.map_cons, List.nodup_cons] at nodup
      have length := Discipline.length_set valid pair.1 pair.2
      simp only [fresh pair (List.mem_cons_self ..), Bool.false_eq_true, if_false] at length
      rw [ih (Discipline.valid_set valid pair.1 pair.2) nodup.2 ?_, length, List.length_cons]
      · omega
      intro other mem
      rw [Discipline.containsB_set]
      simp only [Bool.or_eq_false_iff, decide_eq_false_iff_not]
      exact ⟨fun eq => nodup.1 (by rw [eq]; exact List.mem_map_of_mem mem),
        fresh other (List.mem_cons_of_mem _ mem)⟩

/-- The keys set by a list of pairs are the keys of the pairs or of the entries. -/
theorem mem_fst_foldl_set (discipline : Discipline) (pairs : List (RuntimeValue × RuntimeValue))
    (entries : Entries) {key : RuntimeValue}
    (mem : key ∈ (pairs.foldl (fun entries entry => discipline.set entries entry.1 entry.2) entries).map Prod.fst) :
    key ∈ pairs.map Prod.fst ∨ key ∈ entries.map Prod.fst := by
  have := Discipline.containsB_foldl_set discipline pairs entries key
  rw [containsB_eq_decide_mem, containsB_eq_decide_mem] at this
  simpa using (Bool.eq_iff_iff.mp this).mp (by simpa using mem)

/-! ### `updateAll` -/

@[grind .] theorem valid_updateAll (layout : Layout) {discipline : Discipline}
    {map : RuntimeValue} (valid : Valid discipline map) (keys values : RuntimeValue) :
    Valid discipline (updateAll layout discipline map keys values) := by
  simp only [Valid, updateAll, entriesOf_build]
  exact Discipline.valid_foldl_set discipline _ valid

/-- Setting entries keeps every entry and adds at most one per key set. -/
theorem size_updateAll_bounds (layout : Layout) (discipline : Discipline)
    (map keys values : RuntimeValue) :
    size map ≤ size (updateAll layout discipline map keys values) ∧
      size (updateAll layout discipline map keys values) ≤
        size map + ((elementsOf keys).length : Int) := by
  simp only [size, updateAll, entriesOf_build]
  have := Discipline.length_foldl_set_bounds discipline ((elementsOf keys).zip (elementsOf values))
    (entriesOf map)
  rw [List.length_zip] at this
  omega

@[simp] theorem elementsOf_keysOf (map : RuntimeValue) :
    elementsOf (keysOf map) = (entriesOf map).map Prod.fst := rfl

@[simp] theorem elementsOf_valuesOf (map : RuntimeValue) :
    elementsOf (valuesOf map) = (entriesOf map).map Prod.snd := rfl

/-- The same for an appended map, by its size. -/
theorem size_updateAll_append_bounds (layout : Layout) (discipline : Discipline)
    (map other : RuntimeValue) :
    size map ≤ size (updateAll layout discipline map (keysOf other) (valuesOf other)) ∧
      size (updateAll layout discipline map (keysOf other) (valuesOf other)) ≤
        size map + size other := by
  have := size_updateAll_bounds layout discipline map (keysOf other) (valuesOf other)
  simp only [elementsOf_keysOf, List.length_map] at this
  exact this

theorem hasKey_updateAll_of_fits (layout : Layout) (discipline : Discipline)
    (map keys values probe : RuntimeValue)
    (fits : (elementsOf keys).length ≤ (elementsOf values).length) :
    hasKey (updateAll layout discipline map keys values) probe =
      (decide (probe ∈ elementsOf keys) || hasKey map probe) := by
  simp only [hasKey, updateAll, entriesOf_build, Discipline.containsB_foldl_set,
    List.map_fst_zip fits]

theorem hasKey_updateAll_add_all (layout : Layout) (discipline : Discipline)
    (map keys values probe : RuntimeValue) (ok : ¬AbortsAddAll map keys values) :
    hasKey (updateAll layout discipline map keys values) probe =
      (decide (probe ∈ elementsOf keys) || hasKey map probe) := by
  exact hasKey_updateAll_of_fits _ _ _ _ _ _
    (Nat.le_of_eq (Decidable.of_not_not fun h => ok (Or.inl h)))

theorem hasKey_updateAll_upsert_all (layout : Layout) (discipline : Discipline)
    (map keys values probe : RuntimeValue) (ok : ¬AbortsUpsertAll keys values) :
    hasKey (updateAll layout discipline map keys values) probe =
      (decide (probe ∈ elementsOf keys) || hasKey map probe) := by
  unfold AbortsUpsertAll at ok
  exact hasKey_updateAll_of_fits _ _ _ _ _ _ (Nat.le_of_eq (Decidable.of_not_not ok))

theorem hasKey_updateAll_new_from (layout : Layout) (discipline : Discipline)
    (map keys values probe : RuntimeValue) (ok : ¬AbortsNewFrom keys values) :
    hasKey (updateAll layout discipline map keys values) probe =
      (decide (probe ∈ elementsOf keys) || hasKey map probe) := by
  exact hasKey_updateAll_of_fits _ _ _ _ _ _
    (Nat.le_of_eq (Decidable.of_not_not fun h => ok (Or.inl h)))

theorem hasKey_updateAll_append (layout : Layout) (discipline : Discipline)
    (map other probe : RuntimeValue) :
    hasKey (updateAll layout discipline map (keysOf other) (valuesOf other)) probe =
      (hasKey other probe || hasKey map probe) := by
  rw [hasKey_updateAll_of_fits _ _ _ _ _ _ (by simp)]
  simp only [elementsOf_keysOf, hasKey, containsB_eq_decide_mem]
  rfl

/-- Adding entries at fresh keys adds one each. -/
theorem size_updateAll_add_all (layout : Layout) {discipline : Discipline}
    {map : RuntimeValue} (valid : Valid discipline map) (keys values : RuntimeValue)
    (ok : ¬AbortsAddAll map keys values) :
    size (updateAll layout discipline map keys values) = size map + ((elementsOf keys).length : Int) := by
  have lengths : (elementsOf keys).length = (elementsOf values).length :=
    Decidable.of_not_not fun h => ok (Or.inl h)
  have fresh : ∀ pair ∈ (elementsOf keys).zip (elementsOf values),
      containsB (entriesOf map) pair.1 = false := by
    intro pair mem
    have inKeys : pair.1 ∈ elementsOf keys := mem_fst_zip (List.mem_map_of_mem mem)
    have absent := fun h => ok (Or.inr (Or.inl ⟨pair.1, inKeys, h⟩))
    cases h : hasKey map pair.1 with
    | false => simpa [hasKey] using h
    | true => exact absurd h absent
  have nodup : (((elementsOf keys).zip (elementsOf values)).map Prod.fst).Nodup := by
    rw [List.map_fst_zip (Nat.le_of_eq lengths)]
    exact Classical.byContradiction fun h => ok (Or.inr (Or.inr h))
  simp only [size, updateAll, entriesOf_build]
  rw [Discipline.length_foldl_set_fresh discipline _ valid nodup fresh, List.length_zip]
  omega

/-- A map built from fresh vectors holds one entry per key. -/
theorem size_updateAll_new_from (layout : Layout) (discipline : Discipline)
    (keys values : RuntimeValue) (ok : ¬AbortsNewFrom keys values) :
    size (updateAll layout discipline (empty layout) keys values) = ((elementsOf keys).length : Int) := by
  have ok' : ¬AbortsAddAll (empty layout) keys values := fun
    | .inl h => ok (.inl h)
    | .inr (.inl ⟨_, _, h⟩) => by simp at h
    | .inr (.inr h) => ok (.inr h)
  rw [size_updateAll_add_all layout (valid_empty layout discipline) keys values ok', size_empty]
  omega

/-- Appending a disjoint map adds its entries. -/
theorem size_updateAll_append_disjoint (layout : Layout) {discipline : Discipline}
    {map other : RuntimeValue} (valid : Valid discipline map) (validOther : Valid discipline other)
    (ok : ¬AbortsAppendDisjoint map other) :
    size (updateAll layout discipline map (keysOf other) (valuesOf other)) = size map + size other := by
  have ok' : ¬AbortsAddAll map (keysOf other) (valuesOf other) := fun
    | .inl h => h (by simp)
    | .inr (.inl ⟨key, mem, held⟩) => ok ⟨key, held, by
        rw [elementsOf_keysOf] at mem
        simpa [hasKey, containsB_eq_decide_mem] using mem⟩
    | .inr (.inr h) => h (by rw [elementsOf_keysOf]; exact distinct_of_valid validOther)
  rw [size_updateAll_add_all layout valid _ _ ok', elementsOf_keysOf, List.length_map]
  rfl

/-! ### `takeEntries`, `dropEntries`, and `replaceKey` -/

theorem size_takeEntries (layout : Layout) (map : RuntimeValue) (count : Int) :
    size (takeEntries layout map count) = min (max count 0) (size map) := by
  simp only [size, takeEntries, entriesOf_build, List.length_take]; omega

theorem size_dropEntries (layout : Layout) (map : RuntimeValue) (count : Int) :
    size (dropEntries layout map count) = size map - min (max count 0) (size map) := by
  simp only [size, dropEntries, entriesOf_build, List.length_drop]; omega

theorem size_replaceKey (layout : Layout) (map old new : RuntimeValue) :
    size (replaceKey layout map old new) = size map := by
  simp only [size, replaceKey, entriesOf_build, List.length_map]

@[grind .] theorem valid_takeEntries (layout : Layout) {discipline : Discipline}
    {map : RuntimeValue} (valid : Valid discipline map) (count : Int) :
    Valid discipline (takeEntries layout map count) := by
  simp only [Valid, takeEntries, entriesOf_build]
  cases discipline with
  | sequence => exact List.Nodup.sublist ((List.take_sublist _ _).map Prod.fst) valid
  | ordered rank => exact List.Pairwise.sublist (List.take_sublist _ _) valid

@[grind .] theorem valid_dropEntries (layout : Layout) {discipline : Discipline}
    {map : RuntimeValue} (valid : Valid discipline map) (count : Int) :
    Valid discipline (dropEntries layout map count) := by
  simp only [Valid, dropEntries, entriesOf_build]
  cases discipline with
  | sequence => exact List.Nodup.sublist ((List.drop_sublist _ _).map Prod.fst) valid
  | ordered rank => exact List.Pairwise.sublist (List.drop_sublist _ _) valid

/-- A key's position within distinct entries, by its membership in a prefix. -/
theorem mem_fst_take_iff {entries : Entries} (distinct : Distinct entries) (count : Nat)
    (key : RuntimeValue) :
    key ∈ (entries.take count).map Prod.fst ↔ containsB entries key = true ∧ indexOf entries key < count := by
  constructor
  · intro mem
    rw [List.map_take] at mem
    obtain ⟨index, bound, found⟩ := List.getElem_of_mem mem
    rw [List.getElem_take, List.getElem_map] at found
    have inRange : index < entries.length := by simp [List.length_take] at bound; omega
    have within : index < count := by simp [List.length_take] at bound; omega
    refine ⟨?_, ?_⟩
    · rw [containsB_eq_decide_mem, decide_eq_true_eq, ← found]
      exact List.mem_map_of_mem (List.getElem_mem inRange)
    · rw [← found, indexOf_getElem distinct index inRange]; exact within
  · rintro ⟨present, below⟩
    obtain ⟨bound, found⟩ := getElem_indexOf entries key present
    rw [List.map_take]
    rw [← found]
    exact List.mem_take_iff_getElem.mpr ⟨indexOf entries key, by simp; omega,
      by rw [List.getElem_map]⟩

theorem mem_fst_drop_iff {entries : Entries} (distinct : Distinct entries) (count : Nat)
    (key : RuntimeValue) :
    key ∈ (entries.drop count).map Prod.fst ↔ containsB entries key = true ∧ count ≤ indexOf entries key := by
  constructor
  · intro mem
    rw [List.map_drop] at mem
    obtain ⟨index, bound, found⟩ := List.getElem_of_mem mem
    rw [List.getElem_drop, List.getElem_map] at found
    have inRange : count + index < entries.length := by simp [List.length_drop] at bound; omega
    refine ⟨?_, ?_⟩
    · rw [containsB_eq_decide_mem, decide_eq_true_eq, ← found]
      exact List.mem_map_of_mem (List.getElem_mem inRange)
    · rw [← found, indexOf_getElem distinct _ inRange]; omega
  · rintro ⟨present, above⟩
    obtain ⟨bound, found⟩ := getElem_indexOf entries key present
    rw [List.map_drop, ← found]
    exact List.mem_drop_iff_getElem.mpr ⟨indexOf entries key - count, by simp; omega,
      by rw [List.getElem_map]; congr 2; omega⟩

theorem hasKey_takeEntries (layout : Layout) {discipline : Discipline}
    {map : RuntimeValue} (valid : Valid discipline map) (count : Int) (probe : RuntimeValue) :
    hasKey (takeEntries layout map count) probe = (hasKey map probe && decide (rank map probe < count)) := by
  simp only [hasKey, takeEntries, entriesOf_build, rank, containsB_eq_decide_mem (entriesOf map)]
  rw [containsB_eq_decide_mem, Bool.eq_iff_iff]
  simp only [Bool.and_eq_true, decide_eq_true_eq]
  rw [mem_fst_take_iff (distinct_of_valid valid), containsB_eq_decide_mem, decide_eq_true_eq,
    Int.lt_toNat]

theorem hasKey_dropEntries (layout : Layout) {discipline : Discipline}
    {map : RuntimeValue} (valid : Valid discipline map) (count : Int) (probe : RuntimeValue) :
    hasKey (dropEntries layout map count) probe = (hasKey map probe && decide (count ≤ rank map probe)) := by
  simp only [hasKey, dropEntries, entriesOf_build, rank, containsB_eq_decide_mem (entriesOf map)]
  rw [containsB_eq_decide_mem, Bool.eq_iff_iff]
  simp only [Bool.and_eq_true, decide_eq_true_eq]
  rw [mem_fst_drop_iff (distinct_of_valid valid), containsB_eq_decide_mem, decide_eq_true_eq,
    Int.toNat_le]

theorem hasKey_replaceKey (layout : Layout) (map old new probe : RuntimeValue) :
    hasKey (replaceKey layout map old new) probe =
      if probe = new then (hasKey map old || hasKey map new)
      else (!decide (probe = old) && hasKey map probe) := by
  simp only [hasKey, replaceKey, entriesOf_build, containsB_eq_decide_mem, List.map_map,
    Function.comp_def]
  split
  · subst probe
    rw [Bool.eq_iff_iff]
    simp only [Bool.or_eq_true, decide_eq_true_eq, List.mem_map]
    constructor
    · rintro ⟨entry, mem, found⟩
      by_cases same : entry.1 = old
      · exact .inl ⟨entry, mem, same⟩
      · simp [same] at found; exact .inr ⟨entry, mem, found⟩
    · rintro (⟨entry, mem, found⟩ | ⟨entry, mem, found⟩)
      · exact ⟨entry, mem, by simp [found]⟩
      · by_cases same : entry.1 = old
        · exact ⟨entry, mem, by simp [same]⟩
        · exact ⟨entry, mem, by by_cases h : new = old <;> simp [h, found]⟩
  · rename_i differs
    rw [Bool.eq_iff_iff]
    simp only [Bool.and_eq_true, Bool.not_eq_true', decide_eq_false_iff_not, decide_eq_true_eq,
      List.mem_map]
    constructor
    · rintro ⟨entry, mem, found⟩
      by_cases same : entry.1 = old
      · simp [same] at found; exact absurd found.symm differs
      · simp [same] at found; exact ⟨found ▸ same, entry, mem, found⟩
    · rintro ⟨notOld, entry, mem, found⟩
      exact ⟨entry, mem, by simp [found, notOld]⟩

/-! ### Keys read back through the bulk operations -/

theorem keysRead_iff {α : Type} (read : RuntimeValue → α) (write : α → RuntimeValue)
    (map : RuntimeValue) :
    KeysRead read write map ↔ ∀ key ∈ (entriesOf map).map Prod.fst, write (read key) = key := by
  simp only [KeysRead, List.forall_mem_map]

@[vir_denote_side] theorem keysRead_updateAll {α : Type} {read : RuntimeValue → α}
    {write : α → RuntimeValue} (layout : Layout) (discipline : Discipline) {map : RuntimeValue}
    (keys : KeysRead read write map) {vector : RuntimeValue}
    (keysRead : ∀ key ∈ elementsOf vector, write (read key) = key) (values : RuntimeValue) :
    KeysRead read write (updateAll layout discipline map vector values) := by
  rw [keysRead_iff] at keys ⊢
  simp only [updateAll, entriesOf_build]
  intro key mem
  rcases mem_fst_foldl_set discipline _ _ mem with h | h
  · exact keysRead key (mem_fst_zip h)
  · exact keys key h

@[vir_denote_side] theorem keysRead_takeEntries {α : Type} {read : RuntimeValue → α}
    {write : α → RuntimeValue} (layout : Layout) {map : RuntimeValue}
    (keys : KeysRead read write map) (count : Int) :
    KeysRead read write (takeEntries layout map count) := by
  rw [keysRead_iff] at keys ⊢
  simp only [takeEntries, entriesOf_build, List.map_take]
  exact fun key mem => keys key (List.take_subset _ _ mem)

@[vir_denote_side] theorem keysRead_dropEntries {α : Type} {read : RuntimeValue → α}
    {write : α → RuntimeValue} (layout : Layout) {map : RuntimeValue}
    (keys : KeysRead read write map) (count : Int) :
    KeysRead read write (dropEntries layout map count) := by
  rw [keysRead_iff] at keys ⊢
  simp only [dropEntries, entriesOf_build, List.map_drop]
  exact fun key mem => keys key (List.drop_subset _ _ mem)

@[vir_denote_side] theorem keysRead_replaceKey {α : Type} {read : RuntimeValue → α}
    {write : α → RuntimeValue} (layout : Layout) {map : RuntimeValue}
    (keys : KeysRead read write map) (old : RuntimeValue) {new : RuntimeValue}
    (newRead : write (read new) = new) :
    KeysRead read write (replaceKey layout map old new) := by
  rw [keysRead_iff] at keys ⊢
  simp only [replaceKey, entriesOf_build, List.map_map, Function.comp_def]
  intro key mem
  simp only [List.mem_map] at mem
  obtain ⟨entry, entryMem, rfl⟩ := mem
  split
  · exact newRead
  · exact keys entry.1 (List.mem_map_of_mem entryMem)


/-- The keys of entries encoded one by one as a list, held after an owner's
other fields, are read back when every encoded first field is: the
contents of a table owner, unfolded. -/
theorem keysRead_list_map_at {α β : Type} {read : RuntimeValue → α} {write : α → RuntimeValue}
    (owner : StructHandle) (variant : Option String) (fields : Array RuntimeValue)
    (encode : β → RuntimeValue) (entries : List β)
    (roundTrip : ∀ value, write (read ((encode value).field 0)) = (encode value).field 0) :
    KeysRead read write (.nominal owner variant (fields.push (.vector (entries.map encode).toArray))) := by
  intro pair member
  simp only [entriesOf, Array.back?_push, List.toList_toArray, List.map_map, List.mem_map,
    Function.comp_def] at member
  obtain ⟨value, _, rfl⟩ := member
  exact roundTrip value

theorem keysRead_list_map {α β : Type} {read : RuntimeValue → α}
    {write : α → RuntimeValue} (owner : StructHandle) (variant : Option String)
    (encode : β → RuntimeValue) (entries : List β)
    (roundTrip : ∀ value, write (read ((encode value).field 0)) = (encode value).field 0) :
    KeysRead read write (.nominal owner variant #[.vector (entries.map encode).toArray]) :=
  keysRead_list_map_at owner variant #[] encode entries roundTrip

theorem keysRead_list_map_one {α β : Type} {read : RuntimeValue → α}
    {write : α → RuntimeValue} (owner : StructHandle) (variant : Option String)
    (first : RuntimeValue) (encode : β → RuntimeValue) (entries : List β)
    (roundTrip : ∀ value, write (read ((encode value).field 0)) = (encode value).field 0) :
    KeysRead read write (.nominal owner variant #[first, .vector (entries.map encode).toArray]) :=
  keysRead_list_map_at owner variant #[first] encode entries roundTrip

theorem keysRead_list_map_two {α β : Type} {read : RuntimeValue → α}
    {write : α → RuntimeValue} (owner : StructHandle) (variant : Option String)
    (first second : RuntimeValue) (encode : β → RuntimeValue) (entries : List β)
    (roundTrip : ∀ value, write (read ((encode value).field 0)) = (encode value).field 0) :
    KeysRead read write
      (.nominal owner variant #[first, second, .vector (entries.map encode).toArray]) :=
  keysRead_list_map_at owner variant #[first, second] encode entries roundTrip

@[vir_denote_side] theorem keysRead_updateAll_append {α : Type} {read : RuntimeValue → α}
    {write : α → RuntimeValue} (layout : Layout) (discipline : Discipline) {map other : RuntimeValue}
    (keys : KeysRead read write map) (otherKeys : KeysRead read write other) :
    KeysRead read write (updateAll layout discipline map (keysOf other) (valuesOf other)) := by
  apply keysRead_updateAll layout discipline keys _ (valuesOf other)
  rw [keysRead_iff] at otherKeys
  simpa only [elementsOf_keysOf] using otherKeys


/-- A list without repetition is no longer than one holding its members. -/
theorem length_le_of_nodup_subset {α : Type} [DecidableEq α] {l₁ l₂ : List α}
    (nodup : l₁.Nodup) (subset : ∀ a ∈ l₁, a ∈ l₂) : l₁.length ≤ l₂.length := by
  induction l₁ generalizing l₂ with
  | nil => simp
  | cons a rest ih =>
      simp only [List.nodup_cons] at nodup
      have mem : a ∈ l₂ := subset a (List.mem_cons_self ..)
      have := ih nodup.2 (l₂ := l₂.erase a) fun b memb => by
        have ne : b ≠ a := fun eq => nodup.1 (eq ▸ memb)
        exact (List.mem_erase_of_ne ne).mpr (subset b (List.mem_cons_of_mem _ memb))
      have erased := List.length_erase_of_mem mem
      have positive := List.length_pos_of_mem mem
      rw [List.length_cons]
      omega

/-- An appended map holds every entry of the appended one. -/
theorem size_other_le_updateAll_append (layout : Layout) (discipline : Discipline)
    (map : RuntimeValue) {other : RuntimeValue} (validOther : Valid discipline other) :
    size other ≤ size (updateAll layout discipline map (keysOf other) (valuesOf other)) := by
  have distinct : ((entriesOf other).map Prod.fst).Nodup := distinct_of_valid validOther
  have subset : ∀ key ∈ (entriesOf other).map Prod.fst,
      key ∈ (entriesOf (updateAll layout discipline map (keysOf other) (valuesOf other))).map Prod.fst := by
    intro key mem
    have := hasKey_updateAll_append layout discipline map other key
    rw [hasKey, containsB_eq_decide_mem, hasKey, hasKey, containsB_eq_decide_mem] at this
    have held : decide (key ∈ (entriesOf other).map Prod.fst) = true := decide_eq_true mem
    rw [held, Bool.true_or] at this
    exact decide_eq_true_eq.mp this
  have := length_le_of_nodup_subset distinct subset
  simp only [size, List.length_map] at this ⊢
  omega

/-- The entries a replaced key leaves valid, where replacing does not abort. -/
theorem valid_replaceKey (layout : Layout) {discipline : Discipline}
    {map old new : RuntimeValue} (ok : ¬AbortsReplaceKey layout discipline map old new) :
    Valid discipline (replaceKey layout map old new) :=
  Classical.byContradiction fun invalid => ok (Or.inr invalid)

/-! The abort conditions of the bulk roles at literal vectors, as their
lists: evaluated where a specification states them of literals. A symbolic
vector is a mapped array, which these leave as they are. -/

theorem AbortsNewFrom_toArray (keys values : List RuntimeValue) :
    AbortsNewFrom (.vector keys.toArray) (.vector values.toArray) ↔
      keys.length ≠ values.length ∨ ¬keys.Nodup := by
  simp [AbortsNewFrom]

theorem AbortsUpsertAll_toArray (keys values : List RuntimeValue) :
    AbortsUpsertAll (.vector keys.toArray) (.vector values.toArray) ↔
      keys.length ≠ values.length := by
  simp [AbortsUpsertAll]

theorem AbortsAddAll_toArray (map : RuntimeValue) (keys values : List RuntimeValue) :
    AbortsAddAll map (.vector keys.toArray) (.vector values.toArray) ↔
      keys.length ≠ values.length ∨ (∃ key ∈ keys, hasKey map key = true) ∨ ¬keys.Nodup := by
  simp [AbortsAddAll]

attribute [vir_denote_norm] List.nodup_cons List.nodup_nil List.mem_cons List.not_mem_nil

/-! ## Maps built from literals

A map built from vector literals over the empty map has literal entries
where the keys are literals: each step below takes only a decided
comparison, so a symbolic key leaves the step as it is. -/

/-- The entries with the keys of one list set to the values of another, in
order. -/
def setPairs (discipline : Discipline) (entries : Entries) :
    List RuntimeValue → List RuntimeValue → Entries
  | key :: keys, value :: values => setPairs discipline (discipline.set entries key value) keys values
  | _, _ => entries

theorem setPairs_cons_cons (discipline : Discipline) (entries : Entries)
    (key value : RuntimeValue) (keys values : List RuntimeValue) :
    setPairs discipline entries (key :: keys) (value :: values) =
      setPairs discipline (discipline.set entries key value) keys values := rfl

theorem setPairs_nil (discipline : Discipline) (entries : Entries)
    (values : List RuntimeValue) : setPairs discipline entries [] values = entries := rfl

theorem setPairs_cons_nil (discipline : Discipline) (entries : Entries)
    (key : RuntimeValue) (keys : List RuntimeValue) :
    setPairs discipline entries (key :: keys) [] = entries := rfl

theorem foldl_zip_set (discipline : Discipline) (entries : Entries) (keys values : List RuntimeValue) :
    (keys.zip values).foldl (fun entries entry => discipline.set entries entry.1 entry.2) entries =
      setPairs discipline entries keys values := by
  induction keys generalizing entries values with
  | nil => rfl
  | cons key keys ih =>
      cases values with
      | nil => rfl
      | cons value values => simp only [List.zip_cons_cons, List.foldl_cons, ih, setPairs]

/-- A map built from vector literals over the empty map. -/
theorem updateAll_empty_vector (layout : Layout) (discipline : Discipline)
    (keys values : List RuntimeValue) :
    updateAll layout discipline (empty layout) (.vector keys.toArray) (.vector values.toArray) =
      layout.build (layout.handleOf (empty layout)) (setPairs discipline [] keys values)
        layout.freshSlots := by
  simp only [updateAll, elementsOf_vector, List.toList_toArray, Layout.slotsOf_empty,
    foldl_zip_set]
  simp [empty, entriesOf_build]

attribute [vir_denote_norm] Discipline.set_ordered Discipline.set_sequence

theorem seqSet_of_absent {entries : Entries} {key : RuntimeValue}
    (value : RuntimeValue) (absent : containsB entries key = false) :
    seqSet entries key value = entries ++ [(key, value)] := by
  simp [seqSet, absent]

theorem seqSet_of_present {entries : Entries} {key : RuntimeValue}
    (value : RuntimeValue) (present : containsB entries key = true) :
    seqSet entries key value = replaceFirst key value entries := by
  simp [seqSet, present]

theorem replaceFirst_nil (key value : RuntimeValue) :
    replaceFirst key value [] = [] := rfl

theorem replaceFirst_cons_of_eq {other key : RuntimeValue}
    (value otherValue : RuntimeValue) (rest : Entries) (same : other = key) :
    replaceFirst key value ((other, otherValue) :: rest) = (key, value) :: rest := by
  simp [replaceFirst, same]

theorem replaceFirst_cons_of_ne {other key : RuntimeValue}
    (value otherValue : RuntimeValue) (rest : Entries) (differ : other ≠ key) :
    replaceFirst key value ((other, otherValue) :: rest) =
      (other, otherValue) :: replaceFirst key value rest := by
  simp [replaceFirst, differ]

theorem ordSet_nil (rank : ValueRanks) (key value : RuntimeValue) :
    ordSet rank key value [] = [(key, value)] := rfl

theorem ordSet_cons_lt (rank : ValueRanks) {key other : RuntimeValue}
    (value otherValue : RuntimeValue) (rest : Entries)
    (lt : RuntimeValue.order rank key other = .lt) :
    ordSet rank key value ((other, otherValue) :: rest) =
      (key, value) :: (other, otherValue) :: rest := by
  simp [ordSet, lt]

theorem ordSet_cons_eq (rank : ValueRanks) {key other : RuntimeValue}
    (value otherValue : RuntimeValue) (rest : Entries)
    (eq : RuntimeValue.order rank key other = .eq) :
    ordSet rank key value ((other, otherValue) :: rest) = (key, value) :: rest := by
  simp [ordSet, eq]

theorem ordSet_cons_gt (rank : ValueRanks) {key other : RuntimeValue}
    (value otherValue : RuntimeValue) (rest : Entries)
    (gt : RuntimeValue.order rank key other = .gt) :
    ordSet rank key value ((other, otherValue) :: rest) =
      (other, otherValue) :: ordSet rank key value rest := by
  simp [ordSet, gt]

theorem keyAt_build_nil (layout : Layout) (handle : RuntimeValue)
    (slots : List RuntimeValue) (index : Int) : keyAt (layout.build handle [] slots) index = .unit := by
  simp [keyAt]

theorem keyAt_build_cons_zero (layout : Layout) (handle key value : RuntimeValue)
    (rest : Entries) (slots : List RuntimeValue) :
    keyAt (layout.build handle ((key, value) :: rest) slots) 0 = key := by
  simp [keyAt]

theorem keyAt_build_cons_of_pos (layout : Layout) (handle key value : RuntimeValue)
    (rest : Entries) (slots : List RuntimeValue) {index : Int} (positive : 0 < index) :
    keyAt (layout.build handle ((key, value) :: rest) slots) index =
      keyAt (layout.build handle rest slots) (index - 1) := by
  obtain ⟨n, rfl⟩ : ∃ n : Nat, index = n + 1 := ⟨(index - 1).toNat, by omega⟩
  have : ((n : Int) + 1).toNat = n + 1 := by omega
  simp [keyAt, show ¬((n : Int) + 1 < 0) by omega, show ¬((n : Int) < 0) by omega, this]

theorem rank_build_nil (layout : Layout) (handle key : RuntimeValue)
    (slots : List RuntimeValue) : rank (layout.build handle [] slots) key = 0 := by
  simp [rank, indexOf]

theorem rank_build_cons_of_eq (layout : Layout) (handle : RuntimeValue)
    {other key : RuntimeValue} (value : RuntimeValue) (rest : Entries) (slots : List RuntimeValue)
    (same : other = key) :
    rank (layout.build handle ((other, value) :: rest) slots) key = 0 := by
  simp [rank, indexOf, same]

theorem rank_build_cons_of_ne (layout : Layout) (handle : RuntimeValue)
    {other key : RuntimeValue} (value : RuntimeValue) (rest : Entries) (slots : List RuntimeValue)
    (differ : other ≠ key) :
    rank (layout.build handle ((other, value) :: rest) slots) key =
      rank (layout.build handle rest slots) key + 1 := by
  simp [rank, indexOf, differ]

theorem get_nil (key : RuntimeValue) : get [] key = .unit := rfl

theorem get_cons_of_eq {other key : RuntimeValue} (value : RuntimeValue)
    (rest : Entries) (same : other = key) : get ((other, value) :: rest) key = value := by
  simp [get, same]

theorem get_cons_of_ne {other key : RuntimeValue} (value : RuntimeValue)
    (rest : Entries) (differ : other ≠ key) : get ((other, value) :: rest) key = get rest key := by
  simp [get, differ]

end VerexIR.Maps
