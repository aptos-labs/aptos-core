-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: specification and verification.

import Move
import MoveModel.Tests.Common

/-!
# Aptos `ordered_map` in Leaner Move

The framework's public type is currently a single-variant
`OrderedMap::SortedVectorMap` containing a vector of key/value entries in
strict key order.  This benchmark collapses that implementation-selection
enum to `Map`, but preserves its payload and invariant.  Lookup computes a
lower bound with binary search: `O(log n)` comparisons.  Insertion and removal
then shift the suffix and therefore remain `O(n)`.

Insertion and removal use Move's native `vector::insert` and
`vector::remove` through a mutable vector borrow. The abstract results and
abort behavior match the framework core. Iterators and bulk operations are
deliberately outside this benchmark.
-/

namespace Tests.MovePrograms

open Move
open scoped Move Move.Compiler Move.Spec

module OrderedMap where

  struct Entry (K V) has Copy, Drop, Store where
    key : K
    value : V

  /-! The contracts use this small mathematical model. Proofs connecting it
  to binary search and vector updates are kept in the proof section below.
  Sortedness comes first: it is the data invariant of the map itself. -/

  namespace Model

  def SortedEntries : List (Entry K V) → Prop
    | [] => True
    | entry :: rest =>
        (∀ next ∈ rest, entry.key < next.key) ∧ SortedEntries rest

  end Model

  struct Map (K V) has Copy, Drop, Store where
    entries : Vector (Entry K V)

  -- Every map is sorted, and carries the proof.  Operations therefore need
  -- no well-formedness precondition, and re-establish sortedness only where
  -- a map is created.
  spec Map {K} {V} where
    invariant Model.SortedEntries .entries.toList

  namespace Model

  def Contains (map : Map K V) (key : K) : Prop :=
    ∃ entry ∈ map.entries.toList, entry.key = key

  def MapsTo (map : Map K V) (key : K) (value : V) : Prop :=
    ∃ entry ∈ map.entries.toList,
      entry.key = key ∧ entry.value = value

  def lowerBoundList (entries : List (Entry K V)) (key : K) : Nat :=
    match entries with
    | [] => 0
    | entry :: rest =>
        if entry.key < key then lowerBoundList rest key + 1 else 0

  def insert (entries : List (Entry K V)) (key : K) (value : V) :
      List (Entry K V) :=
    match entries with
    | [] => [{ key, value }]
    | entry :: rest =>
        if entry.key < key then entry :: insert rest key value
        else { key, value } :: entry :: rest

  /-- The inserted entry list; the map-level result is pinned through
  `entries.toList` since only vectors within Move's length domain are
  representable. -/
  def add (map : Map K V) (key : K) (value : V) : List (Entry K V) :=
    insert map.entries.toList key value

  def erase (entries : List (Entry K V)) (key : K) :
      Option (List (Entry K V) × V) :=
    match entries with
    | [] => none
    | entry :: rest =>
        if entry.key < key then
          match erase rest key with
          | none => none
          | some (remaining, value) => some (entry :: remaining, value)
        else if key < entry.key then none
        else some (rest, entry.value)

  namespace Search

  /-- Binary-search invariant: the mathematical lower bound remains inside
  `[low, high]`, and all indices fit in `U64`. -/
  def Window (entries : Vector (Entry K V)) (key : K)
      (low high : U64) : Prop :=
    SortedEntries entries.toList ∧
      low.toNat ≤ lowerBoundList entries.toList key ∧
      lowerBoundList entries.toList key ≤ high.toNat ∧
      high.toNat ≤ entries.toList.length ∧
      entries.toList.length < U64.size

  end Search
  end Model

  /-! ## Functions -/

  public fun empty {K V} : Map K V :=
    { entries := Move.Vector.empty }

  spec empty {K} {V} where
    ensures (result : Map K V).entries.toList = []

  /-- Index of the first entry whose key is not less than `key`. -/
  partial fun lower_bound_loop {K V} (entries : &Vector (Entry K V))
      (key : &K) (low high : U64) : Action U64 := do
    if low < high then
      let middle := low + ((high - low) / 2)
      let entryKey ← &entries[middle].key
      if entryKey < key then
        continue lower_bound_loop entries key (middle + 1) high
      else
        continue lower_bound_loop entries key low middle
    else
      pure low

  spec lower_bound_loop {K} {V} [Compare.Total K]
      (entries : Vector (Entry K V)) (key : K) (low : U64) (high : U64) where
    requires Model.Search.Window entries key low high;
    ensures result = U64.ofNat
      (Model.lowerBoundList entries.toList key);
    aborts_if False

  fun lower_bound {K V} (map : &Map K V) (key : &K) : Action U64 := do
    let entries ← &map.entries
    lower_bound_loop entries key 0 entries.length

  spec lower_bound {K} {V} [Compare.Total K]
      (map : Map K V) (key : K) where
    ensures result = U64.ofNat
      (Model.lowerBoundList map.entries.toList key) ∧
      result.toNat ≤ map.entries.toList.length;
    aborts_if False

  public fun length {K V} (map : &Map K V) : Action U64 := do
    let entries ← &map.entries
    pure entries.length

  spec length {K} {V} (map : Map K V) where
    ensures result.toNat = map.entries.toList.length;
    aborts_if False

  /-- Borrow a key directly through the vector element and field places. -/
  fun borrow_key_at {K V} (map : &Map K V) (index : U64) : Action (&K) := do
    let entries ← &map.entries
    &entries[index].key

  spec borrow_key_at {K} {V} (map : Map K V) (index : U64) where
    ensures ∃ entry, map.entries.toList[index.toNat]? = some entry ∧
      result = entry.key;
    aborts_if map.entries.toList[index.toNat]? = none
      with Move.Semantics.Vector.indexOutOfBounds

  #guard borrow_key_at.borrowProgram.summary.returns.contains {
    parameter := 0
    path := #[.field "entries", .anyIndex, .field "key"]
    kind := .immutable }

  public fun contains {K V} (map : &Map K V) (key : &K) : Action Bool := do
    let index ← lower_bound map key
    let entries ← &map.entries
    if index < entries.length then
      let entryKey ← &entries[index].key
      pure (entryKey == key)
    else
      pure false

  spec contains {K} {V} [Compare.Total K]
      (map : Map K V) (key : K) where
    ensures (result = true) ↔ Model.Contains map key;
    aborts_if False

  /-- Borrow the value stored under `key`, aborting when the key is absent. -/
  public fun borrow {K V} (map : &Map K V) (key : &K) : Action (&V) := do
    let index ← lower_bound map key
    let entries ← &map.entries
    if index < entries.length then
      let entryKey ← &entries[index].key
      if entryKey == key then
        &entries[index].value
      else
        abort 2
    else
      abort 2

  spec borrow {K} {V} [Compare.Total K]
      (map : Map K V) (key : K) where
    ensures Model.MapsTo map key result;
    aborts_if ¬Model.Contains map key with 2

  #guard borrow.borrowProgram.summary.returns.contains {
    parameter := 0
    path := #[.field "entries", .anyIndex, .field "value"]
    kind := .immutable }

  /-- Add a fresh key.  Abort code 1 matches `EKEY_ALREADY_EXISTS`. -/
  public fun add {K V} (map : &mut Map K V) (key : K) (value : V) :
      Action Unit := do
    let keyView ← &key
    let index ← lower_bound map keyView
    let entriesView ← &map.entries
    if index < entriesView.length then
      let entryKey ← &entriesView[index].key
      if entryKey == keyView then
        abort 1
    let entries ← &mut map.entries
    Move.Vector.insert entries index { key, value }

  spec add {K} {V} [Compare.Total K]
      (map : &mut Map K V) (key : K) (value : V) where
    ensures map.entries.toList = Model.add (old(map)) key value;
    aborts_if Model.Contains map key with 1;
    aborts_if U64.size ≤ map.entries.toList.length + 1
      with Move.Semantics.Vector.indexOutOfBounds

  /-- Remove an existing key.  Abort code 2 matches `EKEY_NOT_FOUND`. -/
  public fun remove {K V} (map : &mut Map K V) (key : &K) : Action V := do
    let index ← lower_bound map key
    let entriesView ← &map.entries
    if index < entriesView.length then
      let entryKey ← &entriesView[index].key
      if entryKey == key then
        let entries ← &mut map.entries
        let removed ← Move.Vector.remove entries index
        pure removed.value
      else
        abort 2
    else
      abort 2

  spec remove {K} {V} [Compare.Total K]
      (map : &mut Map K V) (key : K) where
    ensures Model.erase (old(map)).entries.toList key =
        some (map.entries.toList, result);
    aborts_if ¬Model.Contains map key with 2

  /-! ## Proofs -/

  namespace Model

  /-! ### Mathematical proof library

  The proof dependencies are deliberately grouped by operation:

  - `Search.Window` is the binary-search loop invariant;
    `Search.contains_iff_lowerBound` turns its result into map membership.
  - `Insertion.insert_eq_take_lowerBound` identifies the concrete vector
    insertion, while `Insertion.add_sorted` preserves the map invariant.
  - `Removal.erase_eq_take_lowerBound` identifies the concrete vector removal,
    while `Removal.erase_sorted` preserves the map invariant.

  These lemmas are not additional assumptions: each is proved here and then
  cited explicitly by the source-verification proofs below.

  | Verified function | Mathematical facts it uses |
  |---|---|
  | `lower_bound_loop` | the two `Search` bound lemmas |
  | `lower_bound` | `lowerBoundList_le` and `lower_bound_loop.verified` |
  | `contains`, `borrow` | `contains_iff_lowerBound` and `lower_bound.verified` |
  | `add` | search correctness, `insert_eq_take_lowerBound`, `add_sorted` |
  | `remove` | search correctness, `erase_eq_take_lowerBound`, `erase_sorted` |

  The supporting invariant chains are `mem_insert → insert_sorted →
  add_sorted` and `erase_sublist → erase_sorted`.
  -/

  /-! ### Insertion proof chain

  `mem_insert` describes the new contents and `insert_sorted` proves invariant
  preservation. `add_sorted` lifts that fact from lists to maps. -/

  namespace Insertion

  /-- Membership characterization used internally by `insert_sorted`. -/
  theorem mem_insert {K V : Type} (entries : List (Entry K V))
      (key : K) (value : V) (candidate : Entry K V) :
      candidate ∈ insert entries key value ↔
        candidate = { key, value } ∨ candidate ∈ entries := by
    induction entries with
    | nil => simp [insert]
    | cons entry rest ih =>
        simp only [insert]
        split <;> simp_all only [List.mem_cons, or_assoc, or_comm]

  /-- The executable vector insertion point is exactly the abstract
  lower-bound position used by `insert`. This is the representation bridge in
  `add.verified`. -/
  theorem insert_eq_take_lowerBound {K V : Type}
      (entries : List (Entry K V)) (key : K) (value : V) :
      insert entries key value =
        entries.take (lowerBoundList entries key) ++
          { key, value } :: entries.drop (lowerBoundList entries key) := by
    induction entries with
    | nil => simp [insert, lowerBoundList]
    | cons entry rest induction =>
        simp only [insert, lowerBoundList]
        split
        · simp [induction]
        · simp

  /-- List-level invariant preservation, lifted to maps by `add_sorted`. -/
  theorem insert_sorted {K V : Type} [Compare.Total K]
      (entries : List (Entry K V)) (key : K) (value : V)
      (sorted : SortedEntries entries)
      (fresh : ¬∃ entry ∈ entries, entry.key = key) :
      SortedEntries (insert entries key value) := by
    induction entries with
    | nil => simp [insert, SortedEntries]
    | cons entry rest ih =>
        simp only [insert]
        split
        case isTrue entryLt =>
          simp only [SortedEntries] at sorted ⊢
          constructor
          · intro candidate member
            rw [mem_insert rest key value] at member
            rcases member with rfl | member
            · exact entryLt
            · exact sorted.1 candidate member
          · apply ih sorted.2
            intro present
            rcases present with ⟨candidate, member, same⟩
            exact fresh ⟨candidate, by simp [member], same⟩
        case isFalse notEntryLt =>
          have keyLt : key < entry.key := by
            rcases @Compare.Total.total K _ key entry.key with less | less | same
            · exact less
            · exact (notEntryLt less).elim
            · exact (fresh ⟨entry, by simp, same.symm⟩).elim
          simp only [SortedEntries] at sorted ⊢
          constructor
          · intro candidate member
            simp only [List.mem_cons] at member
            rcases member with rfl | member
            · exact keyLt
            · exact Compare.Lawful.trans keyLt (sorted.1 candidate member)
          · exact sorted

  /-- Final invariant theorem used by `add.verified`. -/
  theorem add_sorted {K V : Type} [Compare.Total K]
      (map : Map K V) (key : K) (value : V)
      (sorted : SortedEntries map.entries.toList)
      (fresh : ¬Contains map key) :
      SortedEntries (add map key value) := by
    exact insert_sorted map.entries.toList key value sorted fresh

  end Insertion

  /-! ### Removal proof chain

  `erase_sublist` proves that removal only deletes an entry. Pairwise order is
  inherited by sublists, yielding `erase_sorted`.
  `erase_eq_take_lowerBound` separately connects the abstract result to the
  concrete vector update performed by `remove`. -/

  namespace Removal

  /-- Rephrases the invariant so `List.Pairwise.sublist` can prove
  `erase_sorted`. -/
  theorem sortedEntries_iff_pairwise {K V : Type}
      (entries : List (Entry K V)) :
      SortedEntries entries ↔
        entries.Pairwise fun left right => left.key < right.key := by
    induction entries with
    | nil => simp [SortedEntries]
    | cons entry rest ih => simp [SortedEntries, ih, List.pairwise_cons]

  /-- The structural fact used by `erase_sorted`: erasure deletes exactly one
  entry and never reorders the remainder. -/
  theorem erase_sublist {K V : Type} (entries : List (Entry K V))
      (key : K) (remaining : List (Entry K V)) (value : V)
      (result : erase entries key = some (remaining, value)) :
      List.Sublist remaining entries := by
    induction entries generalizing remaining value with
    | nil => simp [erase] at result
    | cons entry rest ih =>
        simp only [erase] at result
        split at result
        case isTrue =>
          generalize erased : erase rest key = option at result
          cases option with
          | none => simp at result
          | some pair =>
              rcases pair with ⟨next, oldValue⟩
              simp only [Option.some.injEq, Prod.mk.injEq] at result
              rcases result with ⟨rfl, rfl⟩
              exact (ih next oldValue erased).cons_cons entry
        case isFalse =>
          split at result
          case isTrue => simp_all
          case isFalse =>
            simp only [Option.some.injEq, Prod.mk.injEq] at result
            rcases result with ⟨rfl, rfl⟩
            exact (List.Sublist.refl rest).cons entry

  /-- Final invariant theorem used by `remove.verified`. -/
  theorem erase_sorted {K V : Type}
      (entries : List (Entry K V)) (key : K)
      (remaining : List (Entry K V)) (value : V)
      (sorted : SortedEntries entries)
      (result : erase entries key = some (remaining, value)) :
      SortedEntries remaining := by
    rw [sortedEntries_iff_pairwise] at sorted ⊢
    exact sorted.sublist (erase_sublist entries key remaining value result)

  /-- Removing the key found at its lower bound has the same list update as
  Move's checked `vector::remove`. This is the representation bridge in
  `remove.verified`. -/
  theorem erase_eq_take_lowerBound {K V : Type} [Compare.Total K]
      (entries : List (Entry K V)) (key : K) (entry : Entry K V)
      (sorted : SortedEntries entries)
      (atTarget : entries[lowerBoundList entries key]? = some entry)
      (same : entry.key = key) :
      erase entries key = some
        (entries.take (lowerBoundList entries key) ++
          entries.drop (lowerBoundList entries key + 1), entry.value) := by
    induction entries with
    | nil => simp at atTarget
    | cons head rest induction =>
        simp only [SortedEntries] at sorted
        by_cases headLess : head.key < key
        · simp only [lowerBoundList, headLess, if_pos,
            List.getElem?_cons_succ] at atTarget
          rw [erase]
          simp only [headLess, if_pos]
          rw [induction sorted.2 atTarget]
          simp [lowerBoundList, headLess]
        · have entryEq : entry = head := by
            simpa [lowerBoundList, headLess] using atTarget.symm
          subst entry
          have notKeyLess : ¬key < head.key := by
            rw [same]
            exact fun less => Compare.Lawful.asymm less less
          simp [erase, lowerBoundList, headLess, notKeyLess]

  end Removal

  /-! ### Binary-search proof chain

  The bound lemmas place entries on the correct side of the lower bound;
  together they preserve `Window` across either recursive branch.
  `contains_iff_lowerBound` then characterizes lookup at the final index. -/

  namespace Search

  @[simp] theorem lowerBoundList_le (entries : List (Entry K V)) (key : K) :
      lowerBoundList entries key ≤ entries.length := by
    induction entries with
    | nil => simp [lowerBoundList]
    | cons entry rest ih =>
        simp only [lowerBoundList]
        split <;> simp_all

  /-- Preserves the high side of `Window` in the left search branch. -/
  theorem lowerBoundList_le_index_of_not_less
      (entries : List (Entry K V)) (key : K) (index : Nat)
      (entry : Entry K V) (atIndex : entries[index]? = some entry)
      (notLess : ¬entry.key < key) :
      lowerBoundList entries key ≤ index := by
    induction entries generalizing index with
    | nil => simp at atIndex
    | cons head rest induction =>
        cases index with
        | zero =>
            simp at atIndex
            subst entry
            simp [lowerBoundList, notLess]
        | succ index =>
            simp only [List.getElem?_cons_succ] at atIndex
            simp only [lowerBoundList]
            split
            · exact Nat.succ_le_succ
                (induction index atIndex)
            · omega

  /-- Preserves the low side of `Window` in the right search branch. -/
  theorem index_lt_lowerBoundList_of_less {K V : Type}
      [Compare.Lawful K] (entries : List (Entry K V)) (key : K)
      (index : Nat) (entry : Entry K V)
      (sorted : SortedEntries entries)
      (atIndex : entries[index]? = some entry)
      (less : entry.key < key) :
      index < lowerBoundList entries key := by
    induction entries generalizing index with
    | nil => simp at atIndex
    | cons head rest induction =>
        simp only [SortedEntries] at sorted
        cases index with
        | zero =>
            simp at atIndex
            subst entry
            simp [lowerBoundList, less]
        | succ index =>
            simp only [List.getElem?_cons_succ] at atIndex
            simp only [lowerBoundList]
            split
            case isTrue =>
              exact Nat.succ_lt_succ
                (induction index sorted.2 atIndex)
            case isFalse notHeadLess =>
              have headEntry := sorted.1 entry (List.mem_of_getElem? atIndex)
              exact (notHeadLess (Compare.Lawful.trans headEntry less)).elim

  /-- Converts the binary-search result into the abstract membership fact used
  by `contains`, `borrow`, `add`, and `remove`. -/
  theorem contains_iff_lowerBound {K V : Type} [Compare.Total K]
      (entries : List (Entry K V)) (key : K)
      (sorted : SortedEntries entries) :
      (∃ entry ∈ entries, entry.key = key) ↔
        ∃ entry, entries[lowerBoundList entries key]? = some entry ∧
          entry.key = key := by
    induction entries with
    | nil => simp [lowerBoundList]
    | cons entry rest induction =>
        simp only [SortedEntries] at sorted
        simp only [lowerBoundList]
        split
        case isTrue entryLess =>
          have notSame : entry.key ≠ key := by
            intro same
            subst key
            exact Compare.Lawful.asymm entryLess entryLess
          simpa [notSame] using induction sorted.2
        case isFalse notEntryLess =>
          have presentOnlyIfHead :
              (∃ candidate ∈ entry :: rest, candidate.key = key) →
                entry.key = key := by
            rintro ⟨candidate, membership, same⟩
            simp only [List.mem_cons] at membership
            rcases membership with rfl | membership
            · exact same
            · rcases Compare.Total.total key entry.key with
                keyLess | entryLess | equivalent
              · have entryCandidate := sorted.1 candidate membership
                rw [same] at entryCandidate
                exact (Compare.Lawful.asymm keyLess entryCandidate).elim
              · exact (notEntryLess entryLess).elim
              · exact equivalent.symm
          constructor
          · intro present
            exact ⟨entry, by simp, presentOnlyIfHead present⟩
          · rintro ⟨candidate, indexed, same⟩
            simp at indexed
            subst candidate
            exact ⟨entry, by simp, same⟩

  end Search

  end Model

  /-! ### Verification proofs

  The specs are attached to their functions above. Proofs are ordered by
  dependency: direct reductions first, then search, then mutations. Search
  proves where an operation acts; insertion and removal prove what the update
  means and that it preserves sortedness. -/

  /-! ### Direct proofs -/

  verify empty by
    simp [empty.contract, empty]

  /-! ### Search proofs

  `lower_bound_loop.verified` establishes the loop invariant. The remaining
  read-only proofs reuse it through `lower_bound.verified`. -/

  verify lower_bound_loop by
    contract_intro
    obtain ⟨entries, key, low, high⟩ := args
    replace permitted : Model.Search.Window entries key low high := permitted
    obtain ⟨sorted, lowTarget, targetHigh, highLength, lengthBound⟩ := permitted
    by_cases hloop : Move.Verify.Source.logicalLT low high
    · rw [if_pos hloop]
      rw [Move.Verify.Source.logicalLT_uint] at hloop
      have middleBound : low.toNat + (high.toNat - low.toNat) / 2 <
          entries.toList.length := by omega
      obtain ⟨middleEntry, atMiddle⟩ :
          ∃ entry, entries.toList[low.toNat + (high.toNat - low.toNat) / 2]? =
            some entry :=
        ⟨_, List.getElem?_eq_getElem middleBound⟩
      spec_norm
      by_cases goRight : middleEntry.key < key
      · rw [if_pos goRight]
        have targetAfterMiddle := Model.Search.index_lt_lowerBoundList_of_less
          entries.toList key _ middleEntry sorted atMiddle goRight
        exact Move.Verify.wp_of_satisfies recursiveVerified
          ⟨sorted, by u64_omega, targetHigh, highLength, lengthBound⟩
      · rw [if_neg goRight]
        have targetBeforeMiddle :=
          Model.Search.lowerBoundList_le_index_of_not_less
            entries.toList key _ middleEntry atMiddle goRight
        exact Move.Verify.wp_of_satisfies recursiveVerified
          ⟨sorted, lowTarget, by u64_omega, by u64_omega, lengthBound⟩
    · rw [if_neg hloop]
      rw [Move.Verify.Source.logicalLT_uint] at hloop
      rw [Move.Verify.wp_pure]
      exact ⟨by u64_omega, rfl⟩

  verify lower_bound by
    contract_intro
    obtain ⟨map, key⟩ := args
    have lengthToNat : (Move.Vector.length map.entries).toNat =
        map.entries.toList.length := by
      rw [Move.Vector.length_toNat]
    have window : Model.Search.Window map.entries key 0 map.entries.length := by
      refine ⟨map.invariant, by simp, ?_, ?_, map.entries.toList_length_lt⟩
      · rw [lengthToNat]
        exact Model.Search.lowerBoundList_le _ _
      · rw [lengthToNat]
        exact Nat.le_refl _
    refine Move.Verify.wp_mono
      (Move.Verify.wp_of_satisfies (lower_bound_loop.verified _) window) ?_ ?_
    · rintro result final ⟨established, rfl⟩
      dsimp only at established
      obtain ⟨rfl, -⟩ := established
      refine ⟨⟨rfl, ?_⟩, rfl⟩
      have targetLe := Model.Search.lowerBoundList_le map.entries.toList key
      have bounded := map.entries.toList_length_lt
      spec_norm
      omega
    · exact fun code h => h.elim

  verify length by
    unfold length.contract length.sourceSpec Move.Verify.Satisfies
    simp [Move.Vector.length_toNat]

  verify borrow_key_at by
    contract_intro
    obtain ⟨map, index⟩ := args
    rw [Move.Verify.wp_bind, Move.Verify.wp_borrowElemSpec]
    refine ⟨fun entry atIndex => ?_, fun missing => ⟨missing, rfl⟩⟩
    rw [Move.Verify.wp_pure]
    exact ⟨fun _ => ⟨entry, atIndex, rfl⟩, rfl⟩

  verify contains by
    contract_intro
    obtain ⟨map, key⟩ := args
    dsimp only
    wp_call (lower_bound.verified _) using permitted
    · rintro index middle ⟨⟨rfl, -⟩, rfl⟩
      have bounded : map.entries.toList.length < 18446744073709551616 := by
        have := map.entries.toList_length_lt
        simp only [move_norm, Nat.reducePow] at this
        exact this
      have targetLe := Model.Search.lowerBoundList_le map.entries.toList key
      move_step inBounds
      · -- In bounds: the entry at the lower bound decides membership.
        spec_norm at inBounds
        move_step entry atTarget
        spec_norm at atTarget
        move_step
        show (Move.Verify.Source.logicalBEq entry.key key = true) ↔
          Model.Contains map key
        rw [Move.Verify.Source.logicalBEq_move,
          Compare.equal_eq_true_iff]
        show _ ↔ ∃ candidate ∈ map.entries.toList, candidate.key = key
        rw [Model.Search.contains_iff_lowerBound _ _ map.invariant]
        constructor
        · exact fun same => ⟨entry, atTarget, same⟩
        · rintro ⟨candidate, atCandidate, same⟩
          rw [atTarget] at atCandidate
          cases atCandidate
          exact same
      · -- Out of bounds: no entry can match.
        spec_norm at inBounds
        move_step
        show (false = true) ↔ Model.Contains map key
        simp only [Bool.false_eq_true, false_iff]
        intro present
        obtain ⟨entry, atTarget, _⟩ :=
          (Model.Search.contains_iff_lowerBound _ _ map.invariant).mp present
        exact absurd (List.getElem?_eq_some_iff.mp atTarget).1 inBounds
    · exact fun code h => h.elim

  verify borrow by
    contract_intro
    obtain ⟨map, key⟩ := args
    dsimp only
    wp_call (lower_bound.verified _) using permitted
    · rintro index middle ⟨⟨rfl, -⟩, rfl⟩
      have bounded : map.entries.toList.length < 18446744073709551616 := by
        have := map.entries.toList_length_lt
        simp only [move_norm, Nat.reducePow] at this
        exact this
      have targetLe := Model.Search.lowerBoundList_le map.entries.toList key
      move_step inBounds
      · spec_norm at inBounds
        move_step entry atTarget
        spec_norm at atTarget
        move_step equal
        · -- The key matches: the entry is borrowed again for its value.
          move_step value atValue
          spec_norm at atValue
          rw [atTarget] at atValue
          cases atValue
          move_step
          have same := (Compare.equal_eq_true_iff entry.key key).mp
            (by rwa [Move.Verify.Source.logicalBEq_move] at equal)
          exact fun _ => ⟨entry, List.mem_of_getElem? atTarget, same, rfl⟩
        · -- Another key at the lower bound: the key is absent.
          abort_clause
          intro present
          obtain ⟨candidate, atCandidate, same⟩ :=
            (Model.Search.contains_iff_lowerBound _ _ map.invariant).mp
              present
          rw [atTarget] at atCandidate
          cases atCandidate
          exact equal (by
            rw [Move.Verify.Source.logicalBEq_move]
            exact (Compare.equal_eq_true_iff entry.key key).mpr same)
      · spec_norm at inBounds
        abort_clause
        intro present
        obtain ⟨entry, atTarget, _⟩ :=
          (Model.Search.contains_iff_lowerBound _ _ map.invariant).mp present
        exact absurd (List.getElem?_eq_some_iff.mp atTarget).1 inBounds
    · exact fun code h => h.elim

  /-! ### Mutation proofs

  `add` uses the insertion bridge and invariant theorem; `remove` uses the
  corresponding removal theorems. Both reuse search correctness to select the
  concrete vector index. -/

  verify add by
    contract_intro
    obtain ⟨map, key, value⟩ := args
    dsimp only
    rw [Move.Verify.wp_withMutation]
    intro future
    simp only [Move.Semantics.Mutation.read]
    wp_call (lower_bound.verified _) using trivial
    · rintro index middle ⟨⟨rfl, -⟩, rfl⟩
      have insertBound : Model.lowerBoundList map.entries.toList key ≤
          map.entries.toList.length := Model.Search.lowerBoundList_le _ _
      by_cases inBounds : Move.Verify.Source.logicalLT
        (U64.ofNat (Model.lowerBoundList map.entries.toList key))
        (Move.Vector.length map.entries)
      · rw [if_pos inBounds]
        have bounded : map.entries.toList.length < 18446744073709551616 := by
          have := map.entries.toList_length_lt
          simp only [move_norm, Nat.reducePow] at this
          exact this
        have targetLe := Model.Search.lowerBoundList_le map.entries.toList key
        simp only [Move.Verify.Source.logicalLT_uint, Move.UInt.toNat_ofNat,
          Move.Vector.length_toNat] at inBounds
        spec_norm at inBounds
        obtain ⟨entry, atTarget⟩ :
            ∃ entry, map.entries.toList[Model.lowerBoundList
              map.entries.toList key]? = some entry :=
          ⟨_, List.getElem?_eq_getElem inBounds⟩
        rw [Move.Semantics.Vector.borrowElemSpec_eq_pure
            (by
            simp only [move_norm, Nat.reducePow]
            rw [Nat.mod_eq_of_lt (by omega)]
            exact atTarget),
          Move.Semantics.Spec.pure_bind]
        by_cases equal : Move.Verify.Source.logicalBEq entry.key key = true
        · rw [if_pos equal]
          simp only [Move.Verify.wp_abort]
          abort_clause
          exact ⟨entry, List.mem_of_getElem? atTarget,
            (Compare.equal_eq_true_iff entry.key key).mp
              (by rwa [Move.Verify.Source.logicalBEq_move] at equal)⟩
        · rw [if_neg equal]
          have fresh : ¬Model.Contains map key := by
            intro present
            obtain ⟨candidate, atCandidate, same⟩ :=
              (Model.Search.contains_iff_lowerBound _ _ map.invariant).mp
                present
            rw [atTarget] at atCandidate
            cases atCandidate
            exact equal (by
              rw [Move.Verify.Source.logicalBEq_move]
              exact (Compare.equal_eq_true_iff entry.key key).mpr same)
          checked_cases room
          simp only [Move.UInt.toNat_ofNat_of_lt
              (W := Move.W64)
              (n := Model.lowerBoundList map.entries.toList key)
              (by
                simp only [move_norm, Nat.reducePow]
                exact Nat.lt_of_le_of_lt targetLe bounded),
            move_norm, Nat.reducePow,
            Move.Semantics.Mutation.write]
          refine ⟨?_, ?_⟩
          · simp only [Map.Invariant, Move.Vector.toList_ofList,
              ← Model.Insertion.insert_eq_take_lowerBound
                map.entries.toList key value]
            exact Model.Insertion.add_sorted map key value map.invariant fresh
          · rintro holds rfl
            refine ⟨?_, trivial⟩
            intro _noContains _hasRoom
            simp only [Move.Vector.toList_ofList, Model.add,
              ← Model.Insertion.insert_eq_take_lowerBound
                map.entries.toList key value]
      · rw [if_neg inBounds]
        have bounded : map.entries.toList.length < 18446744073709551616 := by
          have := map.entries.toList_length_lt
          simp only [move_norm, Nat.reducePow] at this
          exact this
        have targetLe := Model.Search.lowerBoundList_le map.entries.toList key
        simp only [Move.Verify.Source.logicalLT_uint, Move.UInt.toNat_ofNat,
          Move.Vector.length_toNat] at inBounds
        spec_norm at inBounds
        have fresh : ¬Model.Contains map key := by
          intro present
          obtain ⟨entry, atTarget, _⟩ :=
            (Model.Search.contains_iff_lowerBound _ _ map.invariant).mp
              present
          exact absurd (List.getElem?_eq_some_iff.mp atTarget).1 inBounds
        checked_cases room
        simp only [Move.UInt.toNat_ofNat_of_lt
            (W := Move.W64)
            (n := Model.lowerBoundList map.entries.toList key)
            (by
              simp only [move_norm, Nat.reducePow]
              exact Nat.lt_of_le_of_lt targetLe bounded),
          move_norm, Nat.reducePow, Move.Semantics.Mutation.write]
        refine ⟨?_, ?_⟩
        · simp only [Map.Invariant, Move.Vector.toList_ofList,
            ← Model.Insertion.insert_eq_take_lowerBound
              map.entries.toList key value]
          exact Model.Insertion.add_sorted map key value map.invariant fresh
        · rintro holds rfl
          refine ⟨?_, trivial⟩
          intro _noContains _hasRoom
          simp only [Move.Vector.toList_ofList, Model.add,
            ← Model.Insertion.insert_eq_take_lowerBound
              map.entries.toList key value]
    · exact fun code h => h.elim

  verify remove by
    contract_intro
    obtain ⟨map, key⟩ := args
    dsimp only
    rw [Move.Verify.wp_withMutation]
    intro future
    simp only [Move.Semantics.Mutation.read]
    wp_call (lower_bound.verified _) using permitted
    · rintro index middle ⟨⟨rfl, -⟩, rfl⟩
      by_cases inBounds : Move.Verify.Source.logicalLT
        (U64.ofNat (Model.lowerBoundList map.entries.toList key))
        (Move.Vector.length map.entries)
      · rw [if_pos inBounds]
        have bounded : map.entries.toList.length < 18446744073709551616 := by
          have := map.entries.toList_length_lt
          simp only [move_norm, Nat.reducePow] at this
          exact this
        have targetLe := Model.Search.lowerBoundList_le map.entries.toList key
        simp only [Move.Verify.Source.logicalLT_uint, Move.UInt.toNat_ofNat,
          Move.Vector.length_toNat] at inBounds
        spec_norm at inBounds
        obtain ⟨entry, atTarget⟩ :
            ∃ entry, map.entries.toList[Model.lowerBoundList
              map.entries.toList key]? = some entry :=
          ⟨_, List.getElem?_eq_getElem inBounds⟩
        rw [Move.Semantics.Vector.borrowElemSpec_eq_pure
            (by
            simp only [move_norm, Nat.reducePow]
            rw [Nat.mod_eq_of_lt (by omega)]
            exact atTarget),
          Move.Semantics.Spec.pure_bind]
        by_cases equal : Move.Verify.Source.logicalBEq entry.key key = true
        · rw [if_pos equal]
          have same := (Compare.equal_eq_true_iff entry.key key).mp
            (by rwa [Move.Verify.Source.logicalBEq_move] at equal)
          have erased := Model.Removal.erase_eq_take_lowerBound
            map.entries.toList key entry map.invariant atTarget same
          checked_cases removed
          simp only [Move.UInt.toNat_ofNat_of_lt
              (W := Move.W64) (n := Model.lowerBoundList map.entries.toList key)
              (by
                simp only [move_norm, Nat.reducePow]
                exact Nat.lt_of_le_of_lt targetLe bounded),
            move_norm, Move.Semantics.Mutation.write]
          intro lookup
          rw [atTarget] at lookup
          injection lookup with sameEntry
          subst sameEntry
          refine ⟨?_, ?_⟩
          · simp only [Map.Invariant, Move.Vector.toList_ofList]
            exact Model.Removal.erase_sorted map.entries.toList key _
              entry.value map.invariant (by simpa using erased)
          · rintro holds rfl
            refine ⟨?_, trivial⟩
            intro _present
            simpa using erased
        · rw [if_neg equal]
          simp only [Move.Verify.wp_abort]
          refine ⟨?_, trivial⟩
          intro present
          obtain ⟨candidate, atCandidate, same⟩ :=
            (Model.Search.contains_iff_lowerBound _ _ map.invariant).mp
              present
          rw [atTarget] at atCandidate
          cases atCandidate
          exact equal (by
            rw [Move.Verify.Source.logicalBEq_move]
            exact (Compare.equal_eq_true_iff entry.key key).mpr same)
      · rw [if_neg inBounds]
        have bounded : map.entries.toList.length < 18446744073709551616 := by
          have := map.entries.toList_length_lt
          simp only [move_norm, Nat.reducePow] at this
          exact this
        have targetLe := Model.Search.lowerBoundList_le map.entries.toList key
        simp only [Move.Verify.Source.logicalLT_uint, Move.UInt.toNat_ofNat,
          Move.Vector.length_toNat] at inBounds
        spec_norm at inBounds
        simp only [Move.Verify.wp_abort]
        refine ⟨?_, trivial⟩
        intro present
        obtain ⟨entry, atTarget, _⟩ :=
          (Model.Search.contains_iff_lowerBound _ _ map.invariant).mp present
        exact absurd (List.getElem?_eq_some_iff.mp atTarget).1 inBounds
    · exact fun code h => h.elim

  /-! ## Tests -/

  /- Concrete scenarios keep the compiled tests readable while exercising
  generic bodies and type instantiation in the generated Move module. -/

  fun lookup_scenario : Action U64 := do
    let map : Map U64 U64 := empty
    let mapRef ← &mut map
    add mapRef 30 300
    add mapRef 10 100
    add mapRef 20 200
    let key : U64 := 20
    let keyRef ← &key
    let valueRef ← borrow mapRef keyRef
    (*valueRef)

  fun remove_scenario : Action U64 := do
    let map : Map U64 U64 := empty
    let mapRef ← &mut map
    add mapRef 30 300
    add mapRef 10 100
    add mapRef 20 200
    let key : U64 := 20
    let keyRef ← &key
    let removed ← remove mapRef keyRef
    let absent ← contains mapRef keyRef
    if absent then pure 0 else pure removed

  fun duplicate_scenario : Action U64 := do
    let map : Map U64 U64 := empty
    let mapRef ← &mut map
    add mapRef 10 100
    add mapRef 10 999
    pure 0

  fun missing_scenario : Action U64 := do
    let map : Map U64 U64 := empty
    let mapRef ← &mut map
    let key : U64 := 10
    let keyRef ← &key
    let _ ← remove mapRef keyRef
    pure 0

  fun ordering_scenario : Action U64 := do
    let map : Map U64 U64 := empty
    let mapRef ← &mut map
    add mapRef 3 30
    add mapRef 1 10
    add mapRef 2 20
    let firstKeyRef ← borrow_key_at mapRef 0
    let first ← *firstKeyRef
    let secondKeyRef ← borrow_key_at mapRef 1
    let second ← *secondKeyRef
    let thirdKeyRef ← borrow_key_at mapRef 2
    let third ← *thirdKeyRef
    pure (first * 100 + second * 10 + third)

  fun absent_lookup_scenario : Action U64 := do
    let map : Map U64 U64 := empty
    let mapRef ← &mut map
    add mapRef 10 100
    let key : U64 := 11
    let keyRef ← &key
    let present ← contains mapRef keyRef
    if present then pure 0 else pure 1

  fun bool_key_scenario : Action U64 := do
    let map : Map Bool U64 := empty
    let mapRef ← &mut map
    add mapRef true 10
    add mapRef false 20
    let key : Bool := false
    let keyRef ← &key
    let valueRef ← borrow mapRef keyRef
    (*valueRef)

  fun remove_edges_scenario : Action U64 := do
    let map : Map U64 U64 := empty
    let mapRef ← &mut map
    add mapRef 2 20
    add mapRef 1 10
    add mapRef 3 30
    let firstKey : U64 := 1
    let firstKeyRef ← &firstKey
    let first ← remove mapRef firstKeyRef
    let lastKey : U64 := 3
    let lastKeyRef ← &lastKey
    let last ← remove mapRef lastKeyRef
    let middleKey : U64 := 2
    let middleKeyRef ← &middleKey
    let valueRef ← borrow mapRef middleKeyRef
    let value ← *valueRef
    pure (first + last + value)

  def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.OrderedMap

  private def run := Tests.run compiled

  #test run "lookup_scenario" [] [] = Tests.okU64 200
  #test run "remove_scenario" [] [] = Tests.okU64 200
  #test run "duplicate_scenario" [] [] = Tests.aborted 1
  #test run "missing_scenario" [] [] = Tests.aborted 2
  #test run "ordering_scenario" [] [] = Tests.okU64 123
  #test run "absent_lookup_scenario" [] [] = Tests.okU64 1
  #test run "bool_key_scenario" [] [] = Tests.okU64 20
  #test run "remove_edges_scenario" [] [] = Tests.okU64 60

  -- The `public fun` operations export public visibility.
  #guard ["empty", "length", "contains", "borrow", "add", "remove"].all
    fun name =>
      match compiled.funMeta? name with
      | some info => info.visibility == MoveModel.IR.Visibility.public_
      | none => false

end Tests.MovePrograms
