-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-!
# Ordered map

The framework's sorted-vector map
over generic keys and values, with strictly increasing keys as a data
invariant. Lookup is a recursive binary search for the lower bound; insertion
and removal shift the suffix through `insertVector`/`removeVector`. `empty`,
`lower_bound`, `length`, and `borrow_key_at` close automatically; the binary
search, `contains`, `borrow`, `add`, and `remove` leave the order-theoretic
residue (transitivity across the search window, absence and presence at the
lower bound, sortedness after an insertion or removal) to lemmas over arrays,
theorems of the module next to the functions whose proofs use them.
Eight execution scenarios run at `u64` and `Bool` keys.
-/

namespace VerexSL.Tests.Check.Examples.OrderedMap

verex module 0x42::ordered_map where
  -- ## Representation

  struct Entry {K has Copy, Drop, Store} {V has Copy, Drop, Store} has Copy, Drop, Store where
    key : K
    value : V

  struct Map {K has Copy, Drop, Store} {V has Copy, Drop, Store} has Copy, Drop, Store where
    entries : Vector<Entry<K, V> >

  spec Map where
    invariant sorted_keys(entries)

  -- Keys strictly increase.
  spec fun sorted_keys {K has Copy, Drop, Store} {V has Copy, Drop, Store}(entries : Vector<Entry<K, V> >) : Bool :=
    forall (i : Int), forall (j : Int), 0 <= i && i < j && j < entries.length ==>
      core.prim.compare(entries[i].key, entries[j].key) < 0

  public fun empty {K has Copy, Drop, Store} {V has Copy, Drop, Store}() -> Map<K, V> :=
    new Map<K, V> { entries := vector<Entry<K, V> >[] }
  spec empty where
    ensures result.entries.length == 0
    aborts_if false

  -- ## Lookup

  -- Index of the first entry whose key is not less than `key`.
  fun lower_bound_loop {K has Copy, Drop, Store} {V has Copy, Drop, Store}
      (entries : &Vector<Entry<K, V> >, key : &K, low : u64, high : u64) -> u64 :=
    if low < high then do
      let middle := low + (high - low) / 2
      if core.prim.compare(&entries[middle].key, key) < 0 then
        lower_bound_loop::<K, V>(entries, key, middle + 1, high)
      else
        lower_bound_loop::<K, V>(entries, key, low, middle)
    else low
  spec lower_bound_loop where
    requires sorted_keys(entries) && low <= high && high <= entries.length
    requires forall (i : Int), 0 <= i && i < low ==> core.prim.compare(entries[i].key, key) < 0
    requires forall (i : Int), high <= i && i < entries.length ==>
      core.prim.compare(entries[i].key, key) >= 0
    ensures low <= result && result <= high
    ensures forall (i : Int), 0 <= i && i < result ==> core.prim.compare(entries[i].key, key) < 0
    ensures forall (i : Int), result <= i && i < entries.length ==>
      core.prim.compare(entries[i].key, key) >= 0
    aborts_if false

  /-- An entry left of the middle, whose key is below the needle, is below it. -/
  theorem search_below {α : Type} (order : α → α → Ordering)
      (key : Int → α) (needle : α) (size : Int) (low middle i : Int)
      (transitive : Std.TransCmp order)
      (sorted : ∀ i j : Int, 0 ≤ i → i < j → j < size → order (key i) (key j) = .lt)
      (below : ∀ i : Int, 0 ≤ i → i < low → order (key i) needle = .lt)
      (atMiddle : order (key middle) needle = .lt) (hmiddle : middle < size)
      (h0 : 0 ≤ i) (hi : i < middle + 1) : order (key i) needle = .lt := by
    by_cases left : i < low
    · exact below i h0 left
    by_cases same : i = middle
    · subst same; exact atMiddle
    exact Std.TransCmp.lt_trans (sorted i middle h0 (by omega) hmiddle) atMiddle

  /-- An entry right of the middle, whose key is not below the needle, is not
  below it. -/
  theorem search_above {α : Type} (order : α → α → Ordering)
      (key : Int → α) (needle : α) (size : Int) (high middle i : Int)
      (transitive : Std.TransCmp order)
      (sorted : ∀ i j : Int, 0 ≤ i → i < j → j < size → order (key i) (key j) = .lt)
      (above : ∀ i : Int, high ≤ i → i < size → ¬order (key i) needle = .lt)
      (atMiddle : ¬order (key middle) needle = .lt) (h0 : 0 ≤ middle)
      (hi : middle ≤ i) (hsize : i < size) : ¬order (key i) needle = .lt := by
    by_cases right : high ≤ i
    · exact above i right hsize
    by_cases same : i = middle
    · subst same; exact atMiddle
    intro lt
    exact atMiddle (Std.TransCmp.lt_trans (sorted middle i h0 (by omega) hsize) lt)

  verify! lower_bound_loop by
    verex_obligations
    all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
    case requires_of_lower_bound_loop_1 =>
      apply search_above (order := VerexIR.Proofs.Denote.TypedOrder.order _ _) (i := i) (high := high.val)
        (middle := low.val + (high.val - low.val) / 2)
      all_goals first
        | assumption
        | omega
        | infer_instance
        | (rw [‹(_ : Array _)[_]? = some _›]; assumption)
    case requires_of_lower_bound_loop_7 =>
      apply search_below (order := VerexIR.Proofs.Denote.TypedOrder.order _ _) (i := i) (low := low.val)
        (middle := low.val + (high.val - low.val) / 2)
      all_goals first
        | assumption
        | omega
        | infer_instance
        | (rw [‹(_ : Array _)[_]? = some _›]; assumption)

  fun lower_bound {K has Copy, Drop, Store} {V has Copy, Drop, Store}(map : &Map<K, V>, key : &K) ->
      u64 := do
    let entries := &map.entries
    lower_bound_loop::<K, V>(entries, key, 0, (*entries).length)
  spec lower_bound where
    ensures result <= map.entries.length
    ensures forall (i : Int), 0 <= i && i < result ==>
      core.prim.compare(map.entries[i].key, key) < 0
    ensures forall (i : Int), result <= i && i < map.entries.length ==>
      core.prim.compare(map.entries[i].key, key) >= 0
    aborts_if false

  public fun length {K has Copy, Drop, Store} {V has Copy, Drop, Store}(map : &Map<K, V>) -> u64 :=
    do
    let entries := &map.entries
    (*entries).length
  spec length where
    ensures result == map.entries.length
    aborts_if false

  -- Borrow a key through the vector element and field places.
  fun borrow_key_at {K has Copy, Drop, Store} {V has Copy, Drop, Store}(map : &Map<K, V>,
      index : u64) -> &K := do
    let entries := &map.entries
    &entries[index].key
  spec borrow_key_at where
    ensures result == map.entries[index].key
    aborts_if index >= map.entries.length

  public fun contains {K has Copy, Drop, Store} {V has Copy, Drop, Store}(map : &Map<K, V>,
      key : &K) -> Bool := do
    let index := lower_bound::<K, V>(map, key)
    let entries := &map.entries
    if index < (*entries).length then
      core.prim.equal(&entries[index].key, key)
    else false
  spec contains where
    ensures result == has_key(map.entries, key)
    aborts_if false

  spec fun has_key {K has Copy, Drop, Store} {V has Copy, Drop, Store}(entries : Vector<Entry<K, V> >, key : K) : Bool :=
    exists (i : Int), 0 <= i && i < entries.length && entries[i].key == key

  /-- Nothing below the lower bound carries the needle; past the end nothing
  is left. -/
  theorem absent_past_end {α : Type} (order : α → α → Ordering)
      (key : Int → α) (needle : α) (size : Int) (index : Int)
      (transitive : Std.TransCmp order)
      (below : ∀ i : Int, 0 ≤ i → i < index → order (key i) needle = .lt)
      (past : size ≤ index) :
      ∀ i : Int, 0 ≤ i → i < size → ¬key i = needle := by
    intro i i0 i1 i2
    have lt := below i i0 (by omega)
    rw [i2, Std.ReflCmp.compare_self (cmp := order)] at lt
    cases lt

  /-- At the lower bound of a sorted range the needle is present exactly when
  the entry there carries it. -/
  theorem present_at_bound {α : Type} (order : α → α → Ordering)
      (key : Int → α) (needle : α) (size : Int) (index : Int)
      (value : α) (transitive : Std.TransCmp order)
      (sorted : ∀ i j : Int, 0 ≤ i → i < j → j < size → order (key i) (key j) = .lt)
      (below : ∀ i : Int, 0 ≤ i → i < index → order (key i) needle = .lt)
      (above : ∀ i : Int, index ≤ i → i < size → ¬order (key i) needle = .lt)
      (h0 : 0 ≤ index) (hi : index < size) (atIndex : key index = value) :
      value = needle ↔ ∃ i : Int, 0 ≤ i ∧ i < size ∧ key i = needle := by
    subst atIndex
    refine ⟨fun h => ⟨index, h0, hi, h⟩, fun ⟨i, i0, i1, i2⟩ => ?_⟩
    rcases Int.lt_trichotomy i index with lt | eq | gt
    · have lt := below i i0 lt
      rw [i2, Std.ReflCmp.compare_self (cmp := order)] at lt
      cases lt
    · subst eq; exact i2
    · have h := sorted index i h0 gt i1
      rw [i2] at h
      exact absurd h (above index (Int.le_refl _) hi)

  verify contains by
    verex_obligations
    all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
    case ensures_1_1 =>
      apply absent_past_end (order := VerexIR.Proofs.Denote.TypedOrder.order _ _)
      all_goals first
        | assumption
        | omega
        | infer_instance
    case ensures_1_2 =>
      apply present_at_bound (order := VerexIR.Proofs.Denote.TypedOrder.order _ _)
      all_goals first
        | assumption
        | omega
        | infer_instance
        | (rw [‹(_ : Array _)[_]? = some _›]; rfl)

  -- Borrow the value stored under `key`; abort code 2 is `EKEY_NOT_FOUND`.
  public fun borrow {K has Copy, Drop, Store} {V has Copy, Drop, Store}(map : &Map<K, V>,
      key : &K) -> &V := do
    let index := lower_bound::<K, V>(map, key)
    let entries := &map.entries
    if index < (*entries).length then
      if core.prim.equal(&entries[index].key, key) then &entries[index].value
      else abort(2)
    else abort(2)
  spec borrow where
    ensures exists (i : Int), 0 <= i && i < map.entries.length && map.entries[i].key == key &&
      map.entries[i].value == result
    aborts_if !has_key(map.entries, key) with 2

  /-- An entry found at a position is the lookup there. -/
  theorem entryAt_found {α β : Type} (project : α → β) (xs : Array α) (d : α) (i : Int)
      (e : α) (found : xs[i.toNat]? = some e) :
      project (xs[i.toNat]?.getD d) = project e := by
    simp [found]

  /-- An entry found at a position carries its key there. -/
  theorem present {α β : Type} (key : α → β) (xs : Array α) (d : α) (i : Int) (e : α)
      (found : xs[i.toNat]? = some e) (h0 : 0 ≤ i) (hi : i < xs.size) :
      ∃ a : Int, 0 ≤ a ∧ a < xs.size ∧ key (xs[a.toNat]?.getD d) = key e :=
    ⟨i, h0, hi, entryAt_found key xs d i e found⟩

  /-- An entry found at a position carries its key and value there. -/
  theorem present_with {α β γ : Type} (key : α → β) (value : α → γ) (xs : Array α) (d : α)
      (i : Int) (e : α) (found : xs[i.toNat]? = some e) (h0 : 0 ≤ i) (hi : i < xs.size) :
      ∃ a : Int, 0 ≤ a ∧ a < xs.size ∧ key (xs[a.toNat]?.getD d) = key e ∧
        value (xs[a.toNat]?.getD d) = value e :=
    ⟨i, h0, hi, entryAt_found key xs d i e found, entryAt_found value xs d i e found⟩

  /-- An entry found at a position whose key is not the needle. -/
  theorem found_ne {α β : Type} (key : α → β) (xs : Array α) (d : α) (i : Int) (e : α)
      (needle : β) (found : xs[i.toNat]? = some e) (different : ¬key e = needle) :
      key (xs[i.toNat]?.getD d) ≠ needle := by
    rw [found]; exact different

  /-- Past the end, nothing below the lower bound carries the needle. -/
  theorem absent_past {α β : Type} (order : β → β → Ordering)
      (key : α → β) (transitive : Std.TransCmp order) (xs : Array α) (d : α)
      (index : Int) (needle : β)
      (below : ∀ a : Int, 0 ≤ a → a < index → order (key (xs[a.toNat]?.getD d)) needle = .lt)
      (past : xs.size ≤ index) :
      ∀ a : Int, 0 ≤ a → a < xs.size → ¬key (xs[a.toNat]?.getD d) = needle := by
    intro a a0 a1 a2
    have lt := below a a0 (by omega)
    rw [a2, Std.ReflCmp.compare_self (cmp := order)] at lt
    cases lt

  /-- At the lower bound of increasing keys, an entry without the needle
  means the needle is absent. -/
  theorem absent_at {α β : Type} (order : β → β → Ordering)
      (key : α → β) (transitive : Std.TransCmp order) (xs : Array α) (d : α)
      (index : Int) (needle : β)
      (sorted : ∀ a b : Int, 0 ≤ a → a < b → b < xs.size →
        order (key (xs[a.toNat]?.getD d)) (key (xs[b.toNat]?.getD d)) = .lt)
      (below : ∀ a : Int, 0 ≤ a → a < index → order (key (xs[a.toNat]?.getD d)) needle = .lt)
      (above : ∀ a : Int, index ≤ a → a < xs.size →
        ¬order (key (xs[a.toNat]?.getD d)) needle = .lt)
      (h0 : 0 ≤ index) (hi : index < xs.size)
      (miss : key (xs[index.toNat]?.getD d) ≠ needle) :
      ∀ a : Int, 0 ≤ a → a < xs.size → ¬key (xs[a.toNat]?.getD d) = needle := by
    intro a a0 a1 a2
    rcases Int.lt_trichotomy a index with lt | eq | gt
    · have lt := below a a0 lt
      rw [a2, Std.ReflCmp.compare_self (cmp := order)] at lt
      cases lt
    · exact miss (eq ▸ a2)
    · have step := sorted index a h0 gt a1
      rw [a2] at step
      exact above index (Int.le_refl _) hi step

  verify borrow by
    verex_obligations
    all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
    all_goals first
      | (apply absent_past (order := VerexIR.Proofs.Denote.TypedOrder.order _ _) (key := Prod.fst) <;>
          first | assumption | omega | infer_instance)
      | (apply absent_at (order := VerexIR.Proofs.Denote.TypedOrder.order _ _) (key := Prod.fst) <;>
          first | assumption | omega | infer_instance |
            (apply found_ne Prod.fst <;> assumption))

  -- ## Update

  -- Add a fresh key; abort code 1 is `EKEY_ALREADY_EXISTS`.
  public fun add {K has Copy, Drop, Store} {V has Copy, Drop, Store}(map : &mut Map<K, V>,
      key : K, value : V) -> Unit := do
    let index := lower_bound::<K, V>(map, &key)
    let entries := &map.entries
    if index < (*entries).length then
      if core.prim.equal(&entries[index].key, &key) then abort(1)
    let entries := &mut map.entries
    *entries := core.prim.insertVector(*entries, index, new Entry<K, V> { key := key, value := value })
  spec add where
    ensures map.entries.length == old(map).entries.length + 1
    ensures exists (p : Int), 0 <= p && p <= old(map).entries.length &&
      map.entries[p].key == key && map.entries[p].value == value &&
      (forall (i : Int), 0 <= i && i < p ==> map.entries[i] == old(map).entries[i]) &&
      (forall (i : Int), p < i && i < map.entries.length ==> map.entries[i] == old(map).entries[i - 1])
    aborts_if has_key(map.entries, key) with 1

  /-- A lookup after an insertion. -/
  theorem entryAt_insert {α β : Type} (project : α → β) (xs : Array α) (d : α) (i : Int)
      (e : α) (h : i.toNat ≤ xs.size) (hi : 0 ≤ i) (k : Int) (hk : 0 ≤ k) :
      project ((xs.insertIdx i.toNat e h)[k.toNat]?.getD d) =
        if k < i then project (xs[k.toNat]?.getD d)
        else if k = i then project e
        else project (xs[(k - 1).toNat]?.getD d) := by
    simp only [Array.getElem?_insertIdx]
    by_cases lt : k < i
    · rw [if_pos (by omega), if_pos lt]
    · rw [if_neg (by omega), if_neg lt]
      by_cases eq : k = i
      · rw [if_pos (by omega), if_pos eq, if_pos (by omega)]; rfl
      · rw [if_neg (by omega), if_neg eq, show k.toNat - 1 = (k - 1).toNat by omega]

  /-- A key not below the needle and different from it is above it. -/
  theorem above_of_not_below {β : Type} (order : β → β → Ordering)
      (transitive : Std.TransCmp order) (lawful : Std.LawfulEqCmp order)
      (a b : β) (notBelow : ¬order a b = .lt) (different : a ≠ b) :
      order b a = .lt := by
    cases e : order a b
    · exact absurd e notBelow
    · exact absurd (Std.LawfulEqCmp.eq_of_compare e) different
    · exact Std.OrientedCmp.gt_iff_lt.mp e

  /-- Inserting a key at its lower bound keeps the keys increasing. -/
  theorem sorted_insert {α β : Type} (order : β → β → Ordering)
      (key : α → β) (transitive : Std.TransCmp order)
      (lawful : Std.LawfulEqCmp order) (xs : Array α) (d : α) (i : Int) (e : α)
      (h : i.toNat ≤ xs.size) (hi : 0 ≤ i)
      (sorted : ∀ a b : Int, 0 ≤ a → a < b → b < xs.size →
        order (key (xs[a.toNat]?.getD d)) (key (xs[b.toNat]?.getD d)) = .lt)
      (below : ∀ a : Int, 0 ≤ a → a < i → order (key (xs[a.toNat]?.getD d)) (key e) = .lt)
      (above : ∀ a : Int, i ≤ a → a < xs.size →
        ¬order (key (xs[a.toNat]?.getD d)) (key e) = .lt)
      (absent : ∀ a : Int, 0 ≤ a → a < xs.size → ¬key (xs[a.toNat]?.getD d) = key e) :
      ∀ a b : Int, 0 ≤ a → a < b → b < ((xs.size + 1 : Nat) : Int) →
        order (key ((xs.insertIdx i.toNat e h)[a.toNat]?.getD d))
          (key ((xs.insertIdx i.toNat e h)[b.toNat]?.getD d)) = .lt := by
    have greater : ∀ a : Int, i ≤ a → a < xs.size →
        order (key e) (key (xs[a.toNat]?.getD d)) = .lt :=
      fun a h1 h2 => above_of_not_below order transitive lawful _ _ (above a h1 h2)
        fun same => absent a (by omega) h2 same
    intro a b h0 hab hb
    rw [entryAt_insert key xs d i e h hi a h0, entryAt_insert key xs d i e h hi b (by omega)]
    by_cases a1 : a < i
    · rw [if_pos a1]
      by_cases b1 : b < i
      · rw [if_pos b1]; exact sorted a b h0 hab (by omega)
      · rw [if_neg b1]
        by_cases b2 : b = i
        · rw [if_pos b2]; exact below a h0 a1
        · rw [if_neg b2]; exact sorted a (b - 1) h0 (by omega) (by omega)
    · rw [if_neg a1, if_neg (show ¬b < i by omega)]
      by_cases a2 : a = i
      · rw [if_pos a2, if_neg (show ¬b = i by omega)]
        exact greater (b - 1) (by omega) (by omega)
      · rw [if_neg a2, if_neg (show ¬b = i by omega)]
        exact sorted (a - 1) (b - 1) (by omega) (by omega) (by omega)

  /-- An insertion at a position: the new entry there, the entries before it
  in place, those after it shifted up by one. -/
  theorem insert_at {α β γ : Type} (key : α → β) (value : α → γ) (xs : Array α) (d : α)
      (i : Int) (e : α) (h : i.toNat ≤ xs.size) (hi : 0 ≤ i) :
      ∃ p : Int, 0 ≤ p ∧ p ≤ xs.size ∧
        key ((xs.insertIdx i.toNat e h)[p.toNat]?.getD d) = key e ∧
        value ((xs.insertIdx i.toNat e h)[p.toNat]?.getD d) = value e ∧
        (∀ k : Int, 0 ≤ k → k < p →
          (xs.insertIdx i.toNat e h)[k.toNat]?.getD d = xs[k.toNat]?.getD d) ∧
        (∀ k : Int, p < k → k < ((xs.size + 1 : Nat) : Int) →
          (xs.insertIdx i.toNat e h)[k.toNat]?.getD d = xs[(k - 1).toNat]?.getD d) := by
    refine ⟨i, hi, by omega, ?_, ?_, fun k h0 hk => ?_, fun k hk _ => ?_⟩
    · rw [entryAt_insert key xs d i e h hi i hi, if_neg (Int.lt_irrefl _), if_pos rfl]
    · rw [entryAt_insert value xs d i e h hi i hi, if_neg (Int.lt_irrefl _), if_pos rfl]
    · have step := entryAt_insert id xs d i e h hi k h0
      simp only [id] at step
      rw [step, if_pos hk]
    · have step := entryAt_insert id xs d i e h hi k (by omega)
      simp only [id] at step
      rw [step, if_neg (by omega), if_neg (by omega)]

  verify add by
    verex_obligations
    all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
    case ensures_2_1 | ensures_2_2 =>
      apply insert_at Prod.fst (Prod.fst ∘ Prod.snd) <;> first | assumption | omega
    all_goals first
      | (apply absent_past (order := VerexIR.Proofs.Denote.TypedOrder.order _ _) (key := Prod.fst) <;>
          first | assumption | omega | infer_instance)
      | (apply absent_at (order := VerexIR.Proofs.Denote.TypedOrder.order _ _) (key := Prod.fst) <;>
          first | assumption | omega | infer_instance |
            (apply found_ne Prod.fst <;> assumption))
      | (apply present Prod.fst <;> first | assumption | omega)
      | (apply sorted_insert (order := VerexIR.Proofs.Denote.TypedOrder.order _ _) (key := Prod.fst) <;>
          first | assumption | omega | infer_instance)

  -- Remove an existing key; abort code 2 is `EKEY_NOT_FOUND`.
  public fun remove {K has Copy, Drop, Store} {V has Copy, Drop, Store}(map : &mut Map<K, V>,
      key : &K) -> V := do
    let index := lower_bound::<K, V>(map, key)
    let entries := &map.entries
    if index < (*entries).length then
      if core.prim.equal(&entries[index].key, key) then do
        let entries := &mut map.entries
        let (removed, rest) := core.prim.removeVector(*entries, index)
        *entries := rest
        removed.value
      else abort(2)
    else abort(2)
  spec remove where
    ensures map.entries.length + 1 == old(map).entries.length
    ensures exists (p : Int), 0 <= p && p < old(map).entries.length &&
      old(map).entries[p].key == key && old(map).entries[p].value == result &&
      (forall (i : Int), 0 <= i && i < p ==> map.entries[i] == old(map).entries[i]) &&
      (forall (i : Int), p <= i && i < map.entries.length ==> map.entries[i] == old(map).entries[i + 1])
    aborts_if !has_key(map.entries, key) with 2

  /-- A lookup after a removal. -/
  theorem entryAt_erase {α β : Type} (project : α → β) (xs : Array α) (d : α) (i : Int)
      (h : i.toNat < xs.size) (k : Int) (hk : 0 ≤ k) :
      project ((xs.eraseIdx i.toNat h)[k.toNat]?.getD d) =
        if k < i then project (xs[k.toNat]?.getD d)
        else project (xs[(k + 1).toNat]?.getD d) := by
    simp only [Array.getElem?_eraseIdx]
    by_cases lt : k < i
    · rw [if_pos (by omega), if_pos lt]
    · rw [if_neg (by omega), if_neg lt, show k.toNat + 1 = (k + 1).toNat by omega]

  /-- Removing an entry keeps the keys increasing. -/
  theorem sorted_erase {α β : Type} (order : β → β → Ordering)
      (key : α → β) (xs : Array α) (d : α) (i : Int) (h : i.toNat < xs.size)
      (sorted : ∀ a b : Int, 0 ≤ a → a < b → b < xs.size →
        order (key (xs[a.toNat]?.getD d)) (key (xs[b.toNat]?.getD d)) = .lt) :
      ∀ a b : Int, 0 ≤ a → a < b → b < ((xs.size - 1 : Nat) : Int) →
        order (key ((xs.eraseIdx i.toNat h)[a.toNat]?.getD d))
          (key ((xs.eraseIdx i.toNat h)[b.toNat]?.getD d)) = .lt := by
    intro a b h0 hab hb
    rw [entryAt_erase key xs d i h a h0, entryAt_erase key xs d i h b (by omega)]
    split <;> split
    · exact sorted a b h0 hab (by omega)
    · exact sorted a (b + 1) h0 (by omega) (by omega)
    · omega
    · exact sorted (a + 1) (b + 1) (by omega) (by omega) (by omega)

  /-- A removal at a position holding an entry: the entry was there, the
  entries before it stay, those after it shift down by one. -/
  theorem erase_at {α β γ : Type} (key : α → β) (value : α → γ) (xs : Array α) (d : α)
      (i : Int) (e : α) (h : i.toNat < xs.size) (hi : 0 ≤ i) (found : xs[i.toNat]? = some e) :
      ∃ p : Int, 0 ≤ p ∧ p < xs.size ∧ key (xs[p.toNat]?.getD d) = key e ∧
        value (xs[p.toNat]?.getD d) = value e ∧
        (∀ k : Int, 0 ≤ k → k < p → (xs.eraseIdx i.toNat h)[k.toNat]?.getD d = xs[k.toNat]?.getD d) ∧
        (∀ k : Int, p ≤ k → k < ((xs.size - 1 : Nat) : Int) →
          (xs.eraseIdx i.toNat h)[k.toNat]?.getD d = xs[(k + 1).toNat]?.getD d) := by
    refine ⟨i, hi, by omega, entryAt_found key xs d i e found, entryAt_found value xs d i e found,
      fun k h0 hk => ?_, fun k hk _ => ?_⟩
    · have step := entryAt_erase id xs d i h k h0
      simp only [id] at step
      rw [step, if_pos hk]
    · have step := entryAt_erase id xs d i h k (by omega)
      simp only [id] at step
      rw [step, if_neg (by omega)]

  verify remove by
    verex_obligations
    all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
    case ensures_2 =>
      have found := ‹(_ : Array _)[_]? = some _›
      erw [(Array.getElem_of_getElem? found).choose_spec]
      apply erase_at Prod.fst (Prod.fst ∘ Prod.snd) _ _ _ _ _ _ found <;> first | assumption | omega
    all_goals first
      | (apply absent_past (order := VerexIR.Proofs.Denote.TypedOrder.order _ _) (key := Prod.fst) <;>
          first | assumption | omega | infer_instance)
      | (apply absent_at (order := VerexIR.Proofs.Denote.TypedOrder.order _ _) (key := Prod.fst) <;>
          first | assumption | omega | infer_instance |
            (apply found_ne Prod.fst <;> assumption))
      | (apply sorted_erase (key := Prod.fst) <;> first | assumption | omega)

  -- ## Scenarios

  -- Concrete scenarios exercise the generic bodies at their instantiations.
  fun lookup_scenario() -> u64 := do
    let mut map := empty::<u64, u64>()
    add::<u64, u64>(&mut map, 30, 300)
    add::<u64, u64>(&mut map, 10, 100)
    add::<u64, u64>(&mut map, 20, 200)
    let key : u64 := 20
    *borrow::<u64, u64>(&map, &key)

  fun remove_scenario() -> u64 := do
    let mut map := empty::<u64, u64>()
    add::<u64, u64>(&mut map, 30, 300)
    add::<u64, u64>(&mut map, 10, 100)
    add::<u64, u64>(&mut map, 20, 200)
    let key : u64 := 20
    let removed := remove::<u64, u64>(&mut map, &key)
    if contains::<u64, u64>(&map, &key) then 0 else removed
  -- Every insertion and removal is computed on the concrete map, its order
  -- owed where each mutation ends.
  spec remove_scenario where
    pragma heartbeats = 300

  fun duplicate_scenario() -> u64 := do
    let mut map := empty::<u64, u64>()
    add::<u64, u64>(&mut map, 10, 100)
    add::<u64, u64>(&mut map, 10, 999)
    0

  fun missing_scenario() -> u64 := do
    let mut map := empty::<u64, u64>()
    let key : u64 := 10
    remove::<u64, u64>(&mut map, &key)

  fun ordering_scenario() -> u64 := do
    let mut map := empty::<u64, u64>()
    add::<u64, u64>(&mut map, 3, 30)
    add::<u64, u64>(&mut map, 1, 10)
    add::<u64, u64>(&mut map, 2, 20)
    let first := *borrow_key_at::<u64, u64>(&map, 0)
    let second := *borrow_key_at::<u64, u64>(&map, 1)
    let third := *borrow_key_at::<u64, u64>(&map, 2)
    first * 100 + second * 10 + third

  fun absent_lookup_scenario() -> u64 := do
    let mut map := empty::<u64, u64>()
    add::<u64, u64>(&mut map, 10, 100)
    let key : u64 := 11
    if contains::<u64, u64>(&map, &key) then 0 else 1

  fun bool_key_scenario() -> u64 := do
    let mut map := empty::<Bool, u64>()
    add::<Bool, u64>(&mut map, true, 10)
    add::<Bool, u64>(&mut map, false, 20)
    let key := false
    *borrow::<Bool, u64>(&map, &key)

  fun remove_edges_scenario() -> u64 := do
    let mut map := empty::<u64, u64>()
    add::<u64, u64>(&mut map, 2, 20)
    add::<u64, u64>(&mut map, 1, 10)
    add::<u64, u64>(&mut map, 3, 30)
    let first_key : u64 := 1
    let first := remove::<u64, u64>(&mut map, &first_key)
    let last_key : u64 := 3
    let last := remove::<u64, u64>(&mut map, &last_key)
    let middle_key : u64 := 2
    let middle := *borrow::<u64, u64>(&map, &middle_key)
    first + last + middle
  spec remove_edges_scenario where
    pragma heartbeats = 300

-- Run the functions on concrete inputs in the interpreter and compare the
-- outcomes at `u64` and `Bool` keys.
open VerexIR VerexE2ETests.CheckSupport in
run_cmd do
  assertRuns `«0x42».ordered_map #[
    ⟨"lookup_scenario", #[], .returned #[.integer 200], {}⟩,
    ⟨"remove_scenario", #[], .returned #[.integer 200], {}⟩,
    ⟨"duplicate_scenario", #[], .threw .abort #[.integer 1], {}⟩,
    ⟨"missing_scenario", #[], .threw .abort #[.integer 2], {}⟩,
    ⟨"ordering_scenario", #[], .returned #[.integer 123], {}⟩,
    ⟨"absent_lookup_scenario", #[], .returned #[.integer 1], {}⟩,
    ⟨"bool_key_scenario", #[], .returned #[.integer 20], {}⟩,
    ⟨"remove_edges_scenario", #[], .returned #[.integer 60], {}⟩]

end VerexSL.Tests.Check.Examples.OrderedMap
