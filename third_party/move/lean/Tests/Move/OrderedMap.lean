-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move
import Tests.Common

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
open MoveModel.Frontend.XIR
open scoped Move Move.Compiler Move.Spec

move_module OrderedMap where

  @[move_struct]
  structure Entry (K V : Type) where
    key : K
    value : V
    deriving Copy, Drop, Store

  @[move_struct]
  structure Map (K V : Type) where
    entries : Move.Vector (Entry K V)
    deriving Copy, Drop, Store

  /-! The contracts use this small mathematical model. Proofs connecting it
  to binary search and vector updates are kept in the proof section below. -/

  namespace Model

  def SortedEntries : List (Entry K V) → Prop
    | [] => True
    | entry :: rest =>
        (∀ next ∈ rest, entry.key < next.key) ∧ SortedEntries rest

  def Sorted (map : Map K V) : Prop :=
    SortedEntries map.entries.toList

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

  def add (map : Map K V) (key : K) (value : V) : Map K V :=
    { entries := Move.Vector.ofList (insert map.entries.toList key value) }

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
  def Window (entries : Move.Vector (Entry K V)) (key : K)
      (low high : U64) : Prop :=
    SortedEntries entries.toList ∧
      low.toNat ≤ lowerBoundList entries.toList key ∧
      lowerBoundList entries.toList key ≤ high.toNat ∧
      high.toNat ≤ entries.toList.length ∧
      entries.toList.length < U64.size

  end Search
  end Model

  @[move_public]
  fun empty {K V : Type} : Map K V :=
    { entries := Move.Vector.empty }

  spec empty {K : Type} {V : Type} where
    ensures (result : Map K V).entries.toList = [] ∧ Model.Sorted result

  /-- Index of the first entry whose key is not less than `key`. -/
  partial fun lowerBoundLoop {K V : Type} (entries : &Move.Vector (Entry K V))
      (key : &K) (low high : U64) : Action U64 := do
    if low < high then
      let middle := low + ((high - low) / 2)
      let entryKey ← &entries[middle].key
      if entryKey < key then
        continue lowerBoundLoop entries key (middle + 1) high
      else
        continue lowerBoundLoop entries key low middle
    else
      pure low

  spec lowerBoundLoop {K : Type} {V : Type} [Move.Compare.Total K]
      (entries : Move.Vector (Entry K V)) (key : K) (low : U64) (high : U64) where
    requires Model.Search.Window entries key low high;
    ensures result = U64.ofNat
      (Model.lowerBoundList entries.toList key) ∧ final = initial;
    aborts_if False

  fun lowerBound {K V : Type} (map : &Map K V) (key : &K) : Action U64 := do
    let entries ← &map.entries
    lowerBoundLoop entries key 0 entries.length

  spec lowerBound {K : Type} {V : Type} [Move.Compare.Total K]
      (map : Map K V) (key : K) where
    requires Model.Sorted map ∧ map.entries.toList.length < U64.size;
    ensures result = U64.ofNat
      (Model.lowerBoundList map.entries.toList key) ∧
      result.toNat ≤ map.entries.toList.length ∧ final = initial;
    aborts_if False

  @[move_public]
  fun length {K V : Type} (map : &Map K V) : Action U64 := do
    let entries ← &map.entries
    pure entries.length

  spec length {K : Type} {V : Type} (map : Map K V) where
    requires True;
    ensures result.toNat = map.entries.toList.length ∧ final = initial;
    aborts_if False

  /-- Borrow a key directly through the vector element and field places. -/
  fun borrowKeyAt {K V : Type} (map : &Map K V) (index : U64) : Action (&K) := do
    let entries ← &map.entries
    &entries[index].key

  @[move_public]
  fun contains {K V : Type} (map : &Map K V) (key : &K) : Action Bool := do
    let index ← lowerBound map key
    let entries ← &map.entries
    if index < entries.length then
      let entryKey ← &entries[index].key
      pure (entryKey == key)
    else
      pure false

  spec contains {K : Type} {V : Type} [Move.Compare.Total K]
      (map : Map K V) (key : K) where
    requires Model.Sorted map ∧ map.entries.toList.length < U64.size;
    ensures (result = true) ↔ Model.Contains map key;
    aborts_if False

  /-- Borrow the value stored under `key`, aborting when the key is absent. -/
  @[move_public]
  fun borrow {K V : Type} (map : &Map K V) (key : &K) : Action (&V) := do
    let index ← lowerBound map key
    let entries ← &map.entries
    if index < entries.length then
      let entryKey ← &entries[index].key
      if entryKey == key then
        &entries[index].value
      else
        abort 2
    else
      abort 2

  spec borrow {K : Type} {V : Type} [Move.Compare.Total K]
      (map : Map K V) (key : K) where
    requires Model.Sorted map ∧ map.entries.toList.length < U64.size;
    ensures Model.MapsTo map key result ∧ final = initial;
    aborts_if ¬Model.Contains map key with 2

  /-- Add a fresh key.  Abort code 1 matches `EKEY_ALREADY_EXISTS`. -/
  @[move_public]
  fun add {K V : Type} (map : &mut Map K V) (key : K) (value : V) :
      Action Unit := do
    let keyView ← &key
    let index ← lowerBound map keyView
    let entriesView ← &map.entries
    if index < entriesView.length then
      let entryKey ← &entriesView[index].key
      if entryKey == keyView then
        abort 1
    let entries ← &mut map.entries
    entries.insert index { key, value }

  spec add {K : Type} {V : Type} [Move.Compare.Total K]
      (map : &mut Map K V) (key : K) (value : V) where
    requires Model.Sorted map ∧
      map.entries.toList.length < U64.size;
    ensures map = Model.add (old(map)) key value ∧ Model.Sorted map;
    aborts_if Model.Contains map key with 1

  /-- Remove an existing key.  Abort code 2 matches `EKEY_NOT_FOUND`. -/
  @[move_public]
  fun remove {K V : Type} (map : &mut Map K V) (key : &K) : Action V := do
    let index ← lowerBound map key
    let entriesView ← &map.entries
    if index < entriesView.length then
      let entryKey ← &entriesView[index].key
      if entryKey == key then
        let entries ← &mut map.entries
        let removed ← entries.remove index
        pure removed.value
      else
        abort 2
    else
      abort 2

  spec remove {K : Type} {V : Type} [Move.Compare.Total K]
      (map : &mut Map K V) (key : K) where
    requires Model.Sorted map ∧
      map.entries.toList.length < U64.size;
    ensures Model.erase (old(map)).entries.toList key =
        some (map.entries.toList, result) ∧ Model.Sorted map;
    aborts_if ¬Model.Contains map key with 2

  /- Concrete scenarios keep the compiled tests readable while exercising
  generic bodies and type instantiation in the generated Move module. -/

  fun lookupScenario : Action U64 := do
    let map : Map U64 U64 := empty
    let mapRef ← &mut map
    add mapRef 30 300
    add mapRef 10 100
    add mapRef 20 200
    let key : U64 := 20
    let keyRef ← &key
    let valueRef ← borrow mapRef keyRef
    (*valueRef)

  fun removeScenario : Action U64 := do
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

  fun duplicateScenario : Action U64 := do
    let map : Map U64 U64 := empty
    let mapRef ← &mut map
    add mapRef 10 100
    add mapRef 10 999
    pure 0

  fun missingScenario : Action U64 := do
    let map : Map U64 U64 := empty
    let mapRef ← &mut map
    let key : U64 := 10
    let keyRef ← &key
    let _ ← remove mapRef keyRef
    pure 0

  fun orderingScenario : Action U64 := do
    let map : Map U64 U64 := empty
    let mapRef ← &mut map
    add mapRef 3 30
    add mapRef 1 10
    add mapRef 2 20
    let firstKeyRef ← borrowKeyAt mapRef 0
    let first ← *firstKeyRef
    let secondKeyRef ← borrowKeyAt mapRef 1
    let second ← *secondKeyRef
    let thirdKeyRef ← borrowKeyAt mapRef 2
    let third ← *thirdKeyRef
    pure (first * 100 + second * 10 + third)

  fun absentLookupScenario : Action U64 := do
    let map : Map U64 U64 := empty
    let mapRef ← &mut map
    add mapRef 10 100
    let key : U64 := 11
    let keyRef ← &key
    let present ← contains mapRef keyRef
    if present then pure 0 else pure 1

  fun boolKeyScenario : Action U64 := do
    let map : Map Bool U64 := empty
    let mapRef ← &mut map
    add mapRef true 10
    add mapRef false 20
    let key : Bool := false
    let keyRef ← &key
    let valueRef ← borrow mapRef keyRef
    (*valueRef)

  fun removeEdgesScenario : Action U64 := do
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

namespace OrderedMap

  namespace Model

  /-! # Mathematical proof library

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
  | `lowerBoundLoop` | the two `Search` bound lemmas |
  | `lowerBound` | `lowerBoundList_le` and `lowerBoundLoop.verified` |
  | `contains`, `borrow` | `contains_iff_lowerBound` and `lowerBound.verified` |
  | `add` | search correctness, `insert_eq_take_lowerBound`, `add_sorted` |
  | `remove` | search correctness, `erase_eq_take_lowerBound`, `erase_sorted` |

  The supporting invariant chains are `mem_insert → insert_sorted →
  add_sorted` and `erase_sublist → erase_sorted`.
  -/

  /-! ## Insertion proof chain

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
  theorem insert_sorted {K V : Type} [Move.Compare.Total K]
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
            rcases @Move.Compare.Total.total K _ key entry.key with less | less | same
            · exact less
            · exact (notEntryLt less).elim
            · exact (fresh ⟨entry, by simp, same.symm⟩).elim
          simp only [SortedEntries] at sorted ⊢
          constructor
          · intro candidate member
            simp only [List.mem_cons] at member
            rcases member with rfl | member
            · exact keyLt
            · exact Move.Compare.Lawful.trans keyLt (sorted.1 candidate member)
          · exact sorted

  /-- Final invariant theorem used by `add.verified`. -/
  theorem add_sorted {K V : Type} [Move.Compare.Total K]
      (map : Map K V) (key : K) (value : V)
      (sorted : Sorted map) (fresh : ¬Contains map key) :
      Sorted (add map key value) := by
    exact insert_sorted map.entries.toList key value sorted fresh

  end Insertion

  /-! ## Removal proof chain

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
  theorem erase_eq_take_lowerBound {K V : Type} [Move.Compare.Total K]
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
            exact fun less => Move.Compare.Lawful.asymm less less
          simp [erase, lowerBoundList, headLess, notKeyLess]

  end Removal

  /-! ## Binary-search proof chain

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
      [Move.Compare.Lawful K] (entries : List (Entry K V)) (key : K)
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
              exact (notHeadLess (Move.Compare.Lawful.trans headEntry less)).elim

  /-- Converts the binary-search result into the abstract membership fact used
  by `contains`, `borrow`, `add`, and `remove`. -/
  theorem contains_iff_lowerBound {K V : Type} [Move.Compare.Total K]
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
            exact Move.Compare.Lawful.asymm entryLess entryLess
          simpa [notSame] using induction sorted.2
        case isFalse notEntryLess =>
          have presentOnlyIfHead :
              (∃ candidate ∈ entry :: rest, candidate.key = key) →
                entry.key = key := by
            rintro ⟨candidate, membership, same⟩
            simp only [List.mem_cons] at membership
            rcases membership with rfl | membership
            · exact same
            · rcases Move.Compare.Total.total key entry.key with
                keyLess | entryLess | equivalent
              · have entryCandidate := sorted.1 candidate membership
                rw [same] at entryCandidate
                exact (Move.Compare.Lawful.asymm keyLess entryCandidate).elim
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

  /-! # Verification proofs

  The specs are attached to their functions above. Proofs are ordered by
  dependency: direct reductions first, then search, then mutations. Search
  proves where an operation acts; insertion and removal prove what the update
  means and that it preserves sortedness. -/

  /-! ## Direct proofs -/

  verify empty by
    simp [empty.contract, empty, Model.Sorted, Model.SortedEntries]

  verify length by
    unfold length.contract length.sourceSpec Move.Verify.Satisfies
    simp [Move.Vector.length_toNat]

  /-! ## Search proofs

  `lowerBoundLoop.verified` establishes the loop invariant. The remaining
  read-only proofs reuse it through `lowerBound.verified`. -/

  verify lowerBoundLoop by
    unfold lowerBoundLoop.contract lowerBoundLoop.sourceSpec
    intro K _ V _ _ State
    apply Move.Verify.satisfies_fix
    intro recursive recursiveVerified
    unfold lowerBoundLoop.bodySpec Move.Verify.Satisfies at *
    rintro ⟨entries, key, low, high⟩ initial window
    change Model.Search.Window entries key low high at window
    rcases window with ⟨sorted, lowTarget, targetHigh, highLength, lengthBound⟩
    by_cases loop : low.toNat < high.toNat
    · let middleNat := low.toNat + (high.toNat - low.toNat) / 2
      have lowHigh : low.toNat ≤ high.toNat := Nat.le_of_lt loop
      have middleLtHigh : middleNat < high.toNat := by
        dsimp [middleNat]
        omega
      have middleLtLength : middleNat < entries.toList.length := by omega
      have middleLtSize : middleNat < U64.size := by omega
      have nextLtSize : middleNat + 1 < U64.size := by omega
      have twoNonzero : (2 : U64).toNat ≠ 0 := by decide
      have twoNat : (2 : U64).toNat = 2 := by decide
      let middleEntry := entries.toList[middleNat]'middleLtLength
      have atMiddle : entries.toList[middleNat]? = some middleEntry := by
        simp [middleEntry, middleLtLength]
      have midpointSafe :
          low.toNat +
              (U64.ofNat
                ((high.toNat - low.toNat) / (2 : U64).toNat)).toNat <
            U64.size := by
        change low.toNat + (high.toNat - low.toNat) / 2 < U64.size
        exact middleLtSize
      have midpointSpec :
          (Move.Semantics.Checked.addSpec low
              (U64.ofNat
                ((high.toNat - low.toNat) / (2 : U64).toNat)) :
            Move.Semantics.Spec State U64) =
            Move.Semantics.Spec.pure (U64.ofNat middleNat) := by
        calc
          _ = Move.Semantics.Spec.pure
                (U64.ofNat
                  (low.toNat +
                    (U64.ofNat
                      ((high.toNat - low.toNat) / (2 : U64).toNat)).toNat)) :=
              Move.Semantics.Checked.addSpec_eq_pure midpointSafe
          _ = Move.Semantics.Spec.pure (U64.ofNat middleNat) := by
            simp [middleNat, twoNat]
      have borrowMiddleSpec :
          (Move.Semantics.Vector.borrowElemSpec entries
              (U64.ofNat middleNat) :
            Move.Semantics.Spec State (Entry K V)) =
            Move.Semantics.Spec.pure middleEntry := by
        apply Move.Semantics.Vector.borrowElemSpec_eq_pure
        simpa using atMiddle
      have nextSpec :
          (Move.Semantics.Checked.addSpec (U64.ofNat middleNat) 1 :
            Move.Semantics.Spec State U64) =
            Move.Semantics.Spec.pure (U64.ofNat (middleNat + 1)) := by
        have nextSafe :
            (U64.ofNat middleNat).toNat + (1 : U64).toNat < U64.size := by
          change middleNat + 1 < U64.size
          exact nextLtSize
        calc
          _ = Move.Semantics.Spec.pure
                (U64.ofNat
                  ((U64.ofNat middleNat).toNat + (1 : U64).toNat)) :=
              Move.Semantics.Checked.addSpec_eq_pure nextSafe
          _ = Move.Semantics.Spec.pure (U64.ofNat (middleNat + 1)) := by
            rfl
      by_cases goRight : middleEntry.key < key
      · have targetAfterMiddle := Model.Search.index_lt_lowerBoundList_of_less
          entries.toList key middleNat middleEntry sorted atMiddle goRight
        have nextWindow : Model.Search.Window entries key
            (U64.ofNat (middleNat + 1)) high := by
          exact ⟨sorted, by simp; omega, targetHigh, highLength, lengthBound⟩
        have recursiveResult := recursiveVerified
          (entries, key, U64.ofNat (middleNat + 1), high) initial nextWindow
        constructor
        · intro result final execution
          apply recursiveResult.1 result final
          simpa [loop, goRight, middleNat, middleEntry, lowHigh,
            middleLtLength, middleLtSize, nextLtSize, twoNonzero,
            atMiddle, midpointSpec, borrowMiddleSpec, nextSpec] using execution
        · intro code execution
          apply recursiveResult.2 code
          simpa [loop, goRight, middleNat, middleEntry, lowHigh,
            middleLtLength, middleLtSize, nextLtSize, twoNonzero,
            atMiddle, midpointSpec, borrowMiddleSpec, nextSpec] using execution
      · have targetBeforeMiddle := Model.Search.lowerBoundList_le_index_of_not_less
          entries.toList key middleNat middleEntry atMiddle goRight
        have nextWindow : Model.Search.Window entries key low
            (U64.ofNat middleNat) := by
          exact ⟨sorted, lowTarget, by simp; omega, by simp; omega, lengthBound⟩
        have recursiveResult := recursiveVerified
          (entries, key, low, U64.ofNat middleNat) initial nextWindow
        constructor
        · intro result final execution
          apply recursiveResult.1 result final
          simpa [loop, goRight, middleNat, middleEntry, lowHigh,
            middleLtLength, middleLtSize, twoNonzero, atMiddle,
            midpointSpec, borrowMiddleSpec] using execution
        · intro code execution
          apply recursiveResult.2 code
          simpa [loop, goRight, middleNat, middleEntry, lowHigh,
            middleLtLength, middleLtSize, twoNonzero, atMiddle,
            midpointSpec, borrowMiddleSpec] using execution
    · have targetEq : Model.lowerBoundList entries.toList key = low.toNat := by
        omega
      constructor
      · intro result final execution
        simp [loop, Move.Semantics.Spec.pure] at execution
        rcases execution with ⟨rfl, rfl⟩
        constructor
        · apply U64.ext
          simp [targetEq]
        · rfl
      · intro code execution
        simp [loop, Move.Semantics.Spec.pure] at execution

  verify lowerBound by
    unfold lowerBound.contract Move.Verify.Satisfies
    intro K _ V _ _ State
    rintro ⟨map, key⟩ initial ⟨sorted, lengthBound⟩
    have initialWindow : Model.Search.Window map.entries key 0
        map.entries.length := by
      refine ⟨sorted, ?_, Model.Search.lowerBoundList_le _ _, ?_, lengthBound⟩
      · change 0 ≤ Model.lowerBoundList map.entries.toList key
        omega
      · change map.entries.toList.length ≤ map.entries.toList.length
        exact Nat.le_refl _
    have loopResult := lowerBoundLoop.verified State
      (map.entries, key, 0, map.entries.length) initial initialWindow
    constructor
    · intro result final execution
      have loopExecution :
          (lowerBoundLoop.sourceSpec
            (map.entries, key, 0, map.entries.length)).ok
              initial result final := by
        simpa [lowerBound.sourceSpec] using execution
      rcases loopResult.1 result final loopExecution with ⟨resultEq, finalEq⟩
      constructor
      · exact resultEq
      · constructor
        · rw [resultEq]
          exact Model.Search.lowerBoundList_le map.entries.toList key
        · exact finalEq
    · intro code execution
      apply loopResult.2 code
      simpa [lowerBound.sourceSpec] using execution

  verify contains by
    unfold contains.contract Move.Verify.Satisfies
    intro K _ V _ _ State
    rintro ⟨map, key⟩ initial permitted
    have lowerResult := lowerBound.verified State (map, key) initial permitted
    constructor
    · intro result final execution
      simp only [contains.sourceSpec, Move.Semantics.Spec.pure_bind] at execution
      rcases execution with ⟨index, middle, indexExecution, restExecution⟩
      rcases lowerResult.1 index middle indexExecution with
        ⟨indexEq, indexBound, _⟩
      subst index
      let target := Model.lowerBoundList map.entries.toList key
      by_cases inBounds : target < map.entries.toList.length
      · let entry := map.entries.toList[target]'inBounds
        have atTarget : map.entries.toList[target]? = some entry := by
          simp [entry, inBounds]
        simp [target, inBounds] at restExecution
        rcases restExecution with ⟨rfl, rfl⟩
        change ((entry.key == key) = true ↔
          ∃ candidate ∈ map.entries.toList, candidate.key = key)
        rw [Model.Search.contains_iff_lowerBound map.entries.toList key permitted.1]
        constructor
        · intro equal
          change Move.Compare.equal entry.key key = true at equal
          exact ⟨entry, atTarget,
            (Move.Compare.equal_eq_true_iff entry.key key).mp equal⟩
        · rintro ⟨candidate, atCandidate, equal⟩
          rw [atTarget] at atCandidate
          injection atCandidate with same
          subst candidate
          change Move.Compare.equal entry.key key = true
          exact (Move.Compare.equal_eq_true_iff entry.key key).mpr equal
      · have targetEq : target = map.entries.toList.length := by
          change target ≤ map.entries.toList.length at indexBound
          omega
        simp [target, inBounds] at restExecution
        rcases restExecution with ⟨rfl, rfl⟩
        change (false = true ↔ Model.Contains map key)
        constructor
        · simp
        · intro present
          change (∃ entry ∈ map.entries.toList, entry.key = key) at present
          rw [Model.Search.contains_iff_lowerBound map.entries.toList key permitted.1]
            at present
          rcases present with ⟨entry, atTarget, _⟩
          change map.entries.toList[target]? = some entry at atTarget
          rw [targetEq] at atTarget
          simp at atTarget
    · intro code execution
      simp only [contains.sourceSpec, Move.Semantics.Spec.pure_bind] at execution
      rcases execution with lowerAbort | continuationAbort
      · exact lowerResult.2 code lowerAbort
      · rcases continuationAbort with
          ⟨index, middle, indexExecution, restAbort⟩
        rcases lowerResult.1 index middle indexExecution with
          ⟨indexEq, indexBound, _⟩
        subst index
        let target := Model.lowerBoundList map.entries.toList key
        by_cases inBounds : target < map.entries.toList.length
        · let entry := map.entries.toList[target]'inBounds
          have atTarget : map.entries.toList[target]? = some entry := by
            simp [entry, inBounds]
          simp [target, inBounds] at restAbort
        · simp [target, inBounds] at restAbort

  verify borrow by
    unfold borrow.contract Move.Verify.Satisfies
    intro K _ V _ _ State
    rintro ⟨map, key⟩ initial permitted
    have lowerResult := lowerBound.verified State (map, key) initial permitted
    constructor
    · intro result final execution
      simp only [borrow.sourceSpec, Move.Semantics.Spec.pure_bind] at execution
      rcases execution with ⟨index, middle, indexExecution, restExecution⟩
      rcases lowerResult.1 index middle indexExecution with
        ⟨indexEq, indexBound, middleEq⟩
      subst index
      let target := Model.lowerBoundList map.entries.toList key
      by_cases inBounds : target < map.entries.toList.length
      · let entry := map.entries.toList[target]'inBounds
        have atTarget : map.entries.toList[target]? = some entry := by
          simp [entry, inBounds]
        have borrowAtTarget :
            (Move.Semantics.Vector.borrowElemSpec map.entries
              (U64.ofNat target) : Move.Semantics.Spec State (Entry K V)) =
                Move.Semantics.Spec.pure entry := by
          apply Move.Semantics.Vector.borrowElemSpec_eq_pure
          simpa [target] using atTarget
        by_cases equal : Move.Compare.equal entry.key key = true
        · have same := (Move.Compare.equal_eq_true_iff entry.key key).mp equal
          have equalBeq : (entry.key == key) = true := equal
          simp only [target, Move.Verify.Source.logicalLT_u64,
            Move.U64.toNat_ofNat, Move.Vector.length_toNat, inBounds,
            if_pos] at restExecution
          rw [borrowAtTarget] at restExecution
          simp only [Move.Semantics.Spec.pure_bind] at restExecution
          rw [if_pos equalBeq] at restExecution
          simp [Move.Semantics.Spec.pure] at restExecution
          rcases restExecution with ⟨rfl, rfl⟩
          constructor
          · exact ⟨entry, List.mem_of_getElem? atTarget, same, rfl⟩
          · exact middleEq
        · have unequalBeq : ¬((entry.key == key) = true) := equal
          simp only [target, Move.Verify.Source.logicalLT_u64,
            Move.U64.toNat_ofNat, Move.Vector.length_toNat, inBounds,
            if_pos] at restExecution
          rw [borrowAtTarget] at restExecution
          simp only [Move.Semantics.Spec.pure_bind] at restExecution
          rw [if_neg unequalBeq] at restExecution
          simp [Move.Semantics.Spec.abort] at restExecution
      · simp [target, inBounds, Move.Semantics.Spec.abort] at restExecution
    · intro code execution
      simp only [borrow.sourceSpec, Move.Semantics.Spec.pure_bind] at execution
      rcases execution with lowerAbort | continuationAbort
      · exact (lowerResult.2 code lowerAbort).elim
      · rcases continuationAbort with
          ⟨index, middle, indexExecution, restAbort⟩
        rcases lowerResult.1 index middle indexExecution with
          ⟨indexEq, indexBound, _⟩
        subst index
        let target := Model.lowerBoundList map.entries.toList key
        by_cases inBounds : target < map.entries.toList.length
        · let entry := map.entries.toList[target]'inBounds
          have atTarget : map.entries.toList[target]? = some entry := by
            simp [entry, inBounds]
          have borrowAtTarget :
              (Move.Semantics.Vector.borrowElemSpec map.entries
                (U64.ofNat target) : Move.Semantics.Spec State (Entry K V)) =
                  Move.Semantics.Spec.pure entry := by
            apply Move.Semantics.Vector.borrowElemSpec_eq_pure
            simpa [target] using atTarget
          by_cases equal : Move.Compare.equal entry.key key = true
          · have equalBeq : (entry.key == key) = true := equal
            simp only [target, Move.Verify.Source.logicalLT_u64,
              Move.U64.toNat_ofNat, Move.Vector.length_toNat, inBounds,
              if_pos] at restAbort
            rw [borrowAtTarget] at restAbort
            simp only [Move.Semantics.Spec.pure_bind] at restAbort
            rw [if_pos equalBeq] at restAbort
            simp [Move.Semantics.Spec.pure] at restAbort
          · have unequalBeq : ¬((entry.key == key) = true) := equal
            simp only [target, Move.Verify.Source.logicalLT_u64,
              Move.U64.toNat_ofNat, Move.Vector.length_toNat, inBounds,
              if_pos] at restAbort
            rw [borrowAtTarget] at restAbort
            simp only [Move.Semantics.Spec.pure_bind] at restAbort
            rw [if_neg unequalBeq] at restAbort
            simp [Move.Semantics.Spec.abort] at restAbort
            rcases restAbort with rfl
            constructor
            · intro present
              change (∃ candidate ∈ map.entries.toList,
                candidate.key = key) at present
              rw [Model.Search.contains_iff_lowerBound map.entries.toList key permitted.1]
                at present
              rcases present with ⟨candidate, atCandidate, same⟩
              rw [atTarget] at atCandidate
              injection atCandidate with candidateEq
              subst candidate
              exact equal
                ((Move.Compare.equal_eq_true_iff entry.key key).mpr same)
            · rfl
        · simp [target, inBounds] at restAbort
          rcases restAbort with rfl
          constructor
          · intro present
            change (∃ entry ∈ map.entries.toList, entry.key = key) at present
            rw [Model.Search.contains_iff_lowerBound map.entries.toList key permitted.1]
              at present
            rcases present with ⟨entry, atTarget, _⟩
            change target ≤ map.entries.toList.length at indexBound
            have targetEq : target = map.entries.toList.length := by omega
            change map.entries.toList[target]? = some entry at atTarget
            rw [targetEq] at atTarget
            simp at atTarget
          · rfl

  /-! ## Mutation proofs

  `add` uses the insertion bridge and invariant theorem; `remove` uses the
  corresponding removal theorems. Both reuse search correctness to select the
  concrete vector index. -/

  verify add by
    unfold add.contract add.sourceSpec Move.Verify.Satisfies
    intro K _ V _ _ State
    rintro ⟨map, key, value⟩ initial permitted
    constructor
    · intro output final execution
      simp only [Move.Semantics.withMutation,
        Move.Semantics.Spec.pure_bind] at execution
      rcases execution with
        ⟨future, reference, bodyExecution, referenceCurrent, outputFinal⟩
      rcases bodyExecution with
        ⟨index, middle, indexExecution, restExecution⟩
      have lowerResult := lowerBound.verified State
        (map, key) initial permitted
      rcases lowerResult.1 index middle indexExecution with
        ⟨indexEq, indexBound, middleEq⟩
      subst index
      subst middle
      let target := Model.lowerBoundList map.entries.toList key
      by_cases present : Model.Contains map key
      · have found :=
          (Model.Search.contains_iff_lowerBound map.entries.toList key permitted.1).mp
            present
        rcases found with ⟨entry, atTarget, same⟩
        have inBounds : target < map.entries.toList.length := by
          change map.entries.toList[target]? = some entry at atTarget
          exact (List.getElem?_eq_some_iff.mp atTarget).1
        have borrowAtTarget :
            (Move.Semantics.Vector.borrowElemSpec map.entries
              (U64.ofNat target) : Move.Semantics.Spec State (Entry K V)) =
                Move.Semantics.Spec.pure entry := by
          apply Move.Semantics.Vector.borrowElemSpec_eq_pure
          simpa [target] using atTarget
        have equalBeq : (entry.key == key) = true := by
          change Move.Compare.equal entry.key key = true
          exact (Move.Compare.equal_eq_true_iff entry.key key).mpr same
        simp only [Move.Semantics.Mutation.read] at restExecution
        simp only [target, Move.Verify.Source.logicalLT_u64,
          Move.U64.toNat_ofNat, Move.Vector.length_toNat, inBounds,
          if_pos] at restExecution
        rw [borrowAtTarget] at restExecution
        simp only [Move.Semantics.Spec.pure_bind] at restExecution
        rw [if_pos equalBeq] at restExecution
        simp [Move.Semantics.Spec.abort, Move.Semantics.Spec.bind]
          at restExecution
      · by_cases inBounds : target < map.entries.toList.length
        · let entry := map.entries.toList[target]'inBounds
          have atTarget : map.entries.toList[target]? = some entry := by
            simp [entry, inBounds]
          have borrowAtTarget :
              (Move.Semantics.Vector.borrowElemSpec map.entries
                (U64.ofNat target) : Move.Semantics.Spec State (Entry K V)) =
                  Move.Semantics.Spec.pure entry := by
            apply Move.Semantics.Vector.borrowElemSpec_eq_pure
            simpa [target] using atTarget
          have unequalBeq : ¬((entry.key == key) = true) := by
            intro equal
            apply present
            exact ⟨entry, List.mem_of_getElem? atTarget,
              (Move.Compare.equal_eq_true_iff entry.key key).mp equal⟩
          simp only [Move.Semantics.Mutation.read] at restExecution
          simp only [target, Move.Verify.Source.logicalLT_u64,
            Move.U64.toNat_ofNat, Move.Vector.length_toNat, inBounds,
            if_pos] at restExecution
          rw [borrowAtTarget] at restExecution
          simp only [Move.Semantics.Spec.pure_bind] at restExecution
          rw [if_neg unequalBeq] at restExecution
          simp only [Move.Semantics.Spec.pure_bind] at restExecution
          have insertBound : target ≤ map.entries.toList.length := by
            change target ≤ map.entries.toList.length at indexBound
            exact indexBound
          change (Move.Semantics.Spec.bind
            (Move.Semantics.withMutation map.entries fun entries =>
              Move.Semantics.Spec.bind
                (Move.Semantics.Vector.insertSpec entries
                  (U64.ofNat target) { key, value })
                (fun vectorOutput =>
                  Move.Semantics.Spec.pure ((), vectorOutput.2)))
            (fun fieldOutput =>
              Move.Semantics.Spec.pure
                (fieldOutput.1,
                  Move.Semantics.Mutation.write
                    ({ current := map, prophecy := future } :
                      Move.Semantics.Mutation (Map K V))
                    { entries := fieldOutput.2 }))).ok
              initial (output.1, reference) final at restExecution
          rw [Move.Verify.withMutation_insertSpec_eq_pure
            map.entries (U64.ofNat target) { key, value }
            insertBound] at restExecution
          simp only [Move.Semantics.Spec.pure_bind] at restExecution
          simp [Move.Semantics.Spec.pure, Move.Semantics.Mutation.write]
            at restExecution
          rcases restExecution with ⟨outputFirst, referenceEq, finalEq⟩
          subst reference
          have futureEq : future = Model.add map key value := by
            simpa [Model.add, Model.Insertion.insert_eq_take_lowerBound, target] using
              referenceCurrent.symm
          constructor
          · exact outputFinal.trans futureEq
          · rw [outputFinal, futureEq]
            exact Model.Insertion.add_sorted map key value permitted.1 present
        · simp only [Move.Semantics.Mutation.read] at restExecution
          simp only [target, Move.Verify.Source.logicalLT_u64,
            Move.U64.toNat_ofNat, Move.Vector.length_toNat, inBounds,
            if_false] at restExecution
          have insertBound : target ≤ map.entries.toList.length := by
            change target ≤ map.entries.toList.length at indexBound
            exact indexBound
          change (Move.Semantics.Spec.bind
            (Move.Semantics.withMutation map.entries fun entries =>
              Move.Semantics.Spec.bind
                (Move.Semantics.Vector.insertSpec entries
                  (U64.ofNat target) { key, value })
                (fun vectorOutput =>
                  Move.Semantics.Spec.pure ((), vectorOutput.2)))
            (fun fieldOutput =>
              Move.Semantics.Spec.pure
                (fieldOutput.1,
                  Move.Semantics.Mutation.write
                    ({ current := map, prophecy := future } :
                      Move.Semantics.Mutation (Map K V))
                    { entries := fieldOutput.2 }))).ok
              initial (output.1, reference) final at restExecution
          rw [Move.Verify.withMutation_insertSpec_eq_pure
            map.entries (U64.ofNat target) { key, value }
            insertBound] at restExecution
          simp only [Move.Semantics.Spec.pure_bind] at restExecution
          simp [Move.Semantics.Spec.pure, Move.Semantics.Mutation.write]
            at restExecution
          rcases restExecution with ⟨outputFirst, referenceEq, finalEq⟩
          subst reference
          have futureEq : future = Model.add map key value := by
            simpa [Model.add, Model.Insertion.insert_eq_take_lowerBound, target] using
              referenceCurrent.symm
          constructor
          · exact outputFinal.trans futureEq
          · rw [outputFinal, futureEq]
            exact Model.Insertion.add_sorted map key value permitted.1 present
    · intro code execution
      simp only [Move.Semantics.withMutation,
        Move.Semantics.Spec.pure_bind] at execution
      rcases execution with ⟨future, bodyAbort⟩
      rcases bodyAbort with lowerAbort | continuationAbort
      · have lowerResult := lowerBound.verified State
          (map, key) initial permitted
        exact (lowerResult.2 code lowerAbort).elim
      · rcases continuationAbort with
          ⟨index, middle, indexExecution, restAbort⟩
        have lowerResult := lowerBound.verified State
          (map, key) initial permitted
        rcases lowerResult.1 index middle indexExecution with
          ⟨indexEq, indexBound, middleEq⟩
        subst index
        subst middle
        let target := Model.lowerBoundList map.entries.toList key
        by_cases inBounds : target < map.entries.toList.length
        · let entry := map.entries.toList[target]'inBounds
          have atTarget : map.entries.toList[target]? = some entry := by
            simp [entry, inBounds]
          have borrowAtTarget :
              (Move.Semantics.Vector.borrowElemSpec map.entries
                (U64.ofNat target) : Move.Semantics.Spec State (Entry K V)) =
                  Move.Semantics.Spec.pure entry := by
            apply Move.Semantics.Vector.borrowElemSpec_eq_pure
            simpa [target] using atTarget
          simp only [Move.Semantics.Mutation.read] at restAbort
          simp only [target, Move.Verify.Source.logicalLT_u64,
            Move.U64.toNat_ofNat, Move.Vector.length_toNat, inBounds,
            if_pos] at restAbort
          rw [borrowAtTarget] at restAbort
          simp only [Move.Semantics.Spec.pure_bind] at restAbort
          split at restAbort
          case isTrue equal =>
            simp [Move.Semantics.Spec.abort, Move.Semantics.Spec.bind]
              at restAbort
            constructor
            · exact ⟨entry, List.mem_of_getElem? atTarget,
                (Move.Compare.equal_eq_true_iff entry.key key).mp equal⟩
            · exact restAbort
          case isFalse equal =>
            simp only [Move.Semantics.Spec.pure_bind] at restAbort
            have insertBound : target ≤ map.entries.toList.length := by
              change target ≤ map.entries.toList.length at indexBound
              exact indexBound
            change (Move.Semantics.Spec.bind
              (Move.Semantics.withMutation map.entries fun entries =>
                Move.Semantics.Spec.bind
                  (Move.Semantics.Vector.insertSpec entries
                    (U64.ofNat target) { key, value })
                  (fun vectorOutput =>
                    Move.Semantics.Spec.pure ((), vectorOutput.2)))
              (fun fieldOutput =>
                Move.Semantics.Spec.pure
                  (fieldOutput.1,
                    Move.Semantics.Mutation.write
                      ({ current := map, prophecy := future } :
                        Move.Semantics.Mutation (Map K V))
                      { entries := fieldOutput.2 }))).aborts
                initial code at restAbort
            rw [Move.Verify.withMutation_insertSpec_eq_pure
              map.entries (U64.ofNat target) { key, value }
              insertBound] at restAbort
            simp at restAbort
        · simp only [Move.Semantics.Mutation.read] at restAbort
          simp only [target, Move.Verify.Source.logicalLT_u64,
            Move.U64.toNat_ofNat, Move.Vector.length_toNat, inBounds,
            if_false] at restAbort
          have insertBound : target ≤ map.entries.toList.length := by
            change target ≤ map.entries.toList.length at indexBound
            exact indexBound
          change (Move.Semantics.Spec.bind
            (Move.Semantics.withMutation map.entries fun entries =>
              Move.Semantics.Spec.bind
                (Move.Semantics.Vector.insertSpec entries
                  (U64.ofNat target) { key, value })
                (fun vectorOutput =>
                  Move.Semantics.Spec.pure ((), vectorOutput.2)))
            (fun fieldOutput =>
              Move.Semantics.Spec.pure
                (fieldOutput.1,
                  Move.Semantics.Mutation.write
                    ({ current := map, prophecy := future } :
                      Move.Semantics.Mutation (Map K V))
                    { entries := fieldOutput.2 }))).aborts
              initial code at restAbort
          rw [Move.Verify.withMutation_insertSpec_eq_pure
            map.entries (U64.ofNat target) { key, value }
            insertBound] at restAbort
          simp at restAbort

  verify remove by
    unfold remove.contract remove.sourceSpec Move.Verify.Satisfies
    intro K _ V _ _ State
    rintro ⟨map, key⟩ initial permitted
    constructor
    · intro output final execution
      simp only [Move.Semantics.withMutation,
        Move.Semantics.Spec.pure_bind] at execution
      rcases execution with
        ⟨future, reference, bodyExecution, referenceCurrent, outputFinal⟩
      rcases bodyExecution with
        ⟨index, middle, indexExecution, restExecution⟩
      have lowerResult := lowerBound.verified State
        (map, key) initial permitted
      rcases lowerResult.1 index middle indexExecution with
        ⟨indexEq, indexBound, middleEq⟩
      subst index
      subst middle
      let target := Model.lowerBoundList map.entries.toList key
      by_cases inBounds : target < map.entries.toList.length
      · let entry := map.entries.toList[target]'inBounds
        have atTarget : map.entries.toList[target]? = some entry := by
          simp [entry, inBounds]
        have borrowAtTarget :
            (Move.Semantics.Vector.borrowElemSpec map.entries
              (U64.ofNat target) : Move.Semantics.Spec State (Entry K V)) =
                Move.Semantics.Spec.pure entry := by
          apply Move.Semantics.Vector.borrowElemSpec_eq_pure
          simpa [target] using atTarget
        simp only [Move.Semantics.Mutation.read] at restExecution
        simp only [target, Move.Verify.Source.logicalLT_u64,
          Move.U64.toNat_ofNat, Move.Vector.length_toNat, inBounds,
          if_pos] at restExecution
        rw [borrowAtTarget] at restExecution
        simp only [Move.Semantics.Spec.pure_bind] at restExecution
        split at restExecution
        case isFalse =>
          simp [Move.Semantics.Spec.abort] at restExecution
        case isTrue equal =>
          have same : entry.key = key :=
            (Move.Compare.equal_eq_true_iff entry.key key).mp equal
          change (Move.Semantics.Spec.bind
            (Move.Semantics.withMutation map.entries fun entries =>
              Move.Semantics.Spec.bind
                (Move.Semantics.Vector.removeSpec entries (U64.ofNat target))
                (fun vectorOutput =>
                  Move.Semantics.Spec.pure
                    (vectorOutput.1.value, vectorOutput.2)))
            (fun fieldOutput =>
              Move.Semantics.Spec.pure
                (fieldOutput.1,
                  Move.Semantics.Mutation.write
                    ({ current := map, prophecy := future } :
                      Move.Semantics.Mutation (Map K V))
                    { entries := fieldOutput.2 }))).ok
              initial (output.1, reference) final at restExecution
          rw [Move.Verify.withMutation_removeSpec_eq_pure
            map.entries (U64.ofNat target) entry
            (fun candidate : Entry K V => candidate.value)
            (by simpa [target] using atTarget)] at restExecution
          simp only [Move.Semantics.Spec.pure_bind] at restExecution
          simp [Move.Semantics.Spec.pure, Move.Semantics.Mutation.write]
            at restExecution
          rcases restExecution with
            ⟨⟨outputValue, referenceEq⟩, finalEq⟩
          subst reference
          let remaining := map.entries.toList.take target ++
            map.entries.toList.drop (target + 1)
          have futureEq : future =
              ({ entries := Move.Vector.ofList remaining } : Map K V) := by
            simpa [remaining] using referenceCurrent.symm
          have erased := Model.Removal.erase_eq_take_lowerBound
            map.entries.toList key entry permitted.1 atTarget same
          constructor
          · calc
              Model.erase map.entries.toList key =
                  some (remaining, entry.value) := by
                    simpa [remaining, target] using erased
              _ = some (output.2.entries.toList, output.1) := by
                rw [outputFinal, futureEq, outputValue]
                simp [remaining]
          · rw [outputFinal, futureEq]
            exact Model.Removal.erase_sorted map.entries.toList key remaining
              entry.value permitted.1 (by simpa [remaining, target] using erased)
      · simp only [Move.Semantics.Mutation.read] at restExecution
        simp [target, inBounds, Move.Semantics.Spec.abort] at restExecution
    · intro code execution
      simp only [Move.Semantics.withMutation,
        Move.Semantics.Spec.pure_bind] at execution
      rcases execution with ⟨future, bodyAbort⟩
      rcases bodyAbort with lowerAbort | continuationAbort
      · have lowerResult := lowerBound.verified State
          (map, key) initial permitted
        exact (lowerResult.2 code lowerAbort).elim
      · rcases continuationAbort with
          ⟨index, middle, indexExecution, restAbort⟩
        have lowerResult := lowerBound.verified State
          (map, key) initial permitted
        rcases lowerResult.1 index middle indexExecution with
          ⟨indexEq, indexBound, middleEq⟩
        subst index
        subst middle
        let target := Model.lowerBoundList map.entries.toList key
        by_cases inBounds : target < map.entries.toList.length
        · let entry := map.entries.toList[target]'inBounds
          have atTarget : map.entries.toList[target]? = some entry := by
            simp [entry, inBounds]
          have borrowAtTarget :
              (Move.Semantics.Vector.borrowElemSpec map.entries
                (U64.ofNat target) : Move.Semantics.Spec State (Entry K V)) =
                  Move.Semantics.Spec.pure entry := by
            apply Move.Semantics.Vector.borrowElemSpec_eq_pure
            simpa [target] using atTarget
          simp only [Move.Semantics.Mutation.read] at restAbort
          simp only [target, Move.Verify.Source.logicalLT_u64,
            Move.U64.toNat_ofNat, Move.Vector.length_toNat, inBounds,
            if_pos] at restAbort
          rw [borrowAtTarget] at restAbort
          simp only [Move.Semantics.Spec.pure_bind] at restAbort
          split at restAbort
          case isTrue equal =>
            change (Move.Semantics.Spec.bind
              (Move.Semantics.withMutation map.entries fun entries =>
                Move.Semantics.Spec.bind
                  (Move.Semantics.Vector.removeSpec entries (U64.ofNat target))
                  (fun vectorOutput =>
                    Move.Semantics.Spec.pure
                      (vectorOutput.1.value, vectorOutput.2)))
              (fun fieldOutput =>
                Move.Semantics.Spec.pure
                  (fieldOutput.1,
                    Move.Semantics.Mutation.write
                      ({ current := map, prophecy := future } :
                        Move.Semantics.Mutation (Map K V))
                      { entries := fieldOutput.2 }))).aborts
                initial code at restAbort
            rw [Move.Verify.withMutation_removeSpec_eq_pure
              map.entries (U64.ofNat target) entry
              (fun candidate : Entry K V => candidate.value)
              (by simpa [target] using atTarget)] at restAbort
            simp at restAbort
          case isFalse unequal =>
            simp [Move.Semantics.Spec.abort] at restAbort
            constructor
            · intro present
              change (∃ candidate ∈ map.entries.toList,
                candidate.key = key) at present
              rw [Model.Search.contains_iff_lowerBound map.entries.toList key permitted.1]
                at present
              rcases present with ⟨candidate, atCandidate, same⟩
              rw [atTarget] at atCandidate
              injection atCandidate with candidateEq
              subst candidate
              exact unequal
                ((Move.Compare.equal_eq_true_iff entry.key key).mpr same)
            · exact restAbort
        · simp only [Move.Semantics.Mutation.read] at restAbort
          simp [target, inBounds, Move.Semantics.Spec.abort] at restAbort
          constructor
          · intro present
            change (∃ entry ∈ map.entries.toList, entry.key = key) at present
            rw [Model.Search.contains_iff_lowerBound map.entries.toList key permitted.1]
              at present
            rcases present with ⟨entry, atTarget, _⟩
            change target ≤ map.entries.toList.length at indexBound
            have targetEq : target = map.entries.toList.length := by omega
            change map.entries.toList[target]? = some entry at atTarget
            rw [targetEq] at atTarget
            simp at atTarget
          · exact restAbort
  def compiled : MModule := move_module% "OrderedMapTest"

  private def run := Tests.run compiled

  #test run "lookupScenario" [] [] = Tests.okU64 200
  #test run "removeScenario" [] [] = Tests.okU64 200
  #test run "duplicateScenario" [] [] = Tests.aborted 1
  #test run "missingScenario" [] [] = Tests.aborted 2
  #test run "orderingScenario" [] [] = Tests.okU64 123
  #test run "absentLookupScenario" [] [] = Tests.okU64 1
  #test run "boolKeyScenario" [] [] = Tests.okU64 20
  #test run "removeEdgesScenario" [] [] = Tests.okU64 60

end OrderedMap

end Tests.MovePrograms
