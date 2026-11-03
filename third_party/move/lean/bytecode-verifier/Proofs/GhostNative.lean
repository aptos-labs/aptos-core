-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.GhostInitial

/-!
# The ghost layer annotates every native call that returns

UNTRUSTED (checked by the kernel). A native call's annotation
(`Shadow.nativeStep`) passes on the argument nodes its flow names, creates
the result nodes the flow marks fresh, and consumes every argument node no
result node takes. Under the native contract (`NativesRespectOwnership`) the
annotation exists and establishes `StepGhost`: the argument nodes passed on
are kept, once each, and copied when passed on more than once (which the
contract allows only for types with `copy`); the others are consumed
legitimately; and creations and consumptions respect D3.

The counting is over *positions* of argument nodes (argument index and path):
distinct positions, each with its occurrence.
-/

namespace MoveBytecode.Proofs

/-! ## Counting positions -/

theorem count_replicates {α : Type} [BEq α] [LawfulBEq α] (g : α → Nat) (a : α) :
    ∀ {xs : List α}, xs.Nodup →
      (xs.flatMap fun b => List.replicate (g b) b).count a = if a ∈ xs then g a else 0
  | [], _ => by simp
  | b :: xs, h => by
    rw [List.nodup_cons] at h
    rw [List.flatMap_cons, List.count_append, count_replicates g a h.2, List.count_replicate]
    by_cases hab : b = a
    · subst hab
      simp [h.1]
    · have : (b == a) = false := by simpa using hab
      simp only [this, Bool.false_eq_true, ↓reduceIte, Nat.zero_add, List.mem_cons]
      by_cases ha : a ∈ xs
      · simp [ha]
      · simp [ha, Ne.symm hab]

/-- A list of positions `O`, all among the distinct positions `L`, is the
positions of `L` it contains, once each, and the repetitions. -/
theorem perm_positions {α : Type} [BEq α] [LawfulBEq α] {L O : List α} (hnd : L.Nodup)
    (hO : ∀ a ∈ O, a ∈ L) :
    O.Perm (L.filter (fun a => O.contains a) ++
      (L.filter (fun a => O.contains a)).flatMap fun a => List.replicate (O.count a - 1) a) := by
  have hnd' : (L.filter fun a => O.contains a).Nodup := hnd.sublist List.filter_sublist
  rw [List.perm_iff_count]
  intro a
  rw [List.count_append, hnd'.count, count_replicates _ a hnd']
  have hmem : a ∈ L.filter (fun a => O.contains a) ↔ a ∈ O := by
    simp only [List.mem_filter, List.contains_iff_mem]
    exact ⟨fun h => h.2, fun h => ⟨hO a h, h⟩⟩
  by_cases ha : a ∈ O
  · have := List.count_pos_iff.2 ha
    simp only [hmem.2 ha, ↓reduceIte]
    omega
  · simp only [mt hmem.1 ha, ↓reduceIte]
    exact List.count_eq_zero_of_not_mem ha

/-- An effect from occurrences: those kept, dropped and copied, and the
events' consumptions and creations, when the events borrow nothing. -/
theorem effect_of_occ {P : Program} {pre post : Occ} {evs : List Event} {f f' : Nat}
    (keep dropped copies : Occ)
    (hpre : pre.Perm (keep ++ dropped ++ consumedOf evs))
    (hpost : post.Perm (keep ++ copies ++ createdOf evs))
    (hiss : issuedOf evs = keys (createdOf evs))
    (hd : ∀ x ∈ dropped, (x.2.type.abilities P).drop = true)
    (hc : ∀ x ∈ copies, x ∈ keep ∧ (x.2.type.abilities P).copy = true)
    (hb : ∀ o ∈ createdOf evs, f ≤ o.1 ∧ o.1 < f') (hnd : ((createdOf evs).map (·.1)).Nodup)
    (hff : f ≤ f') :
    Effect P pre post evs f f' := by
  refine ⟨keys keep, keys dropped, keys copies, ?_, ?_, ?_, ?_, ?_, ?_, hff⟩
  · simpa [keys_append] using keys_perm hpre
  · rw [hiss]; simpa [keys_append] using keys_perm hpost
  · intro x hx
    obtain ⟨o, ho, -, hk⟩ := mem_keys_iff.1 hx
    rw [← hk, key_type]; exact hd o ho
  · intro x hx
    obtain ⟨o, ho, h1, hk⟩ := mem_keys_iff.1 hx
    obtain ⟨hk', hc'⟩ := hc o ho
    refine ⟨?_, by rw [← hk, key_type]; exact hc'⟩
    obtain ⟨x1, x2⟩ := x
    simp only at h1 hk
    subst h1 hk
    exact mem_keys hk'
  · rw [hiss]
    intro x hx
    obtain ⟨o, ho, h1, -⟩ := mem_keys_iff.1 hx
    rw [← h1]; exact hb o ho
  · rw [hiss]; simpa [keys, Function.comp_def] using hnd

/-- **The effect of a native call**, on positions: the argument nodes at
positions `AZ` (distinct), those passed on at positions `O`, the consumption
events `CE` of the others and the creation events `CR` of the new result
nodes. -/
theorem native_effect {α : Type} [BEq α] [LawfulBEq α] {P : Program}
    (AZ : List (α × (Nat × Value)))
    (O : List α) (occAt : α → Nat × Value) (hocc : ∀ x ∈ AZ, x.2 = occAt x.1)
    (hnd : (AZ.map (·.1)).Nodup) (hO : ∀ a ∈ O, a ∈ AZ.map (·.1))
    (hcopy : ∀ x ∈ AZ, (x.2.2.type.abilities P).copy = true ∨ O.count x.1 ≤ 1)
    {CE CR : List Event}
    (hCE : consumedOf CE = (AZ.filter fun x => !O.contains x.1).map (·.2))
    (hCE' : createdOf CE = []) (hCR : consumedOf CR = [])
    (hnb : ∀ i r p, Event.borrowed i r p ∉ CE ++ CR) {RO : Occ}
    (hRO : RO.Perm (O.map occAt ++ createdOf CR)) {f f' : Nat}
    (hb : ∀ o ∈ createdOf CR, f ≤ o.1 ∧ o.1 < f') (hnd' : ((createdOf CR).map (·.1)).Nodup)
    (hff : f ≤ f') :
    Effect P (AZ.map (·.2)) RO (CE ++ CR) f f' := by
  have hkeep : (AZ.filter fun x => O.contains x.1).map (·.2) =
      ((AZ.map (·.1)).filter fun a => O.contains a).map occAt := by
    rw [List.filter_map, List.map_map]
    exact List.map_congr_left fun x hx => hocc x (List.mem_filter.1 hx).1
  have hperm := perm_positions hnd hO
  refine effect_of_occ ((AZ.filter fun x => O.contains x.1).map (·.2)) []
    ((((AZ.map (·.1)).filter fun a => O.contains a).flatMap
      fun a => List.replicate (O.count a - 1) a).map occAt) ?_ ?_ (issuedOf_created hnb) (by simp)
    ?_ ?_ ?_ hff
  · rw [consumedOf_append, hCE, hCR, List.append_nil, List.append_nil, ← List.map_append]
    exact ((List.filter_append_perm _ AZ).map _).symm
  · rw [createdOf_append, hCE', List.nil_append, hkeep, ← List.map_append]
    exact hRO.trans ((hperm.map occAt).append_right _)
  · intro o ho
    obtain ⟨a, ha, rfl⟩ := List.mem_map.1 ho
    obtain ⟨b, hb', hab⟩ := List.mem_flatMap.1 ha
    rw [List.mem_replicate] at hab
    obtain ⟨hcnt, rfl⟩ := hab
    refine ⟨?_, ?_⟩
    · rw [hkeep]; exact List.mem_map_of_mem hb'
    · obtain ⟨hbL, hbO⟩ := List.mem_filter.1 hb'
      obtain ⟨x, hx, rfl⟩ := List.mem_map.1 hbL
      rcases hcopy x hx with hc | hc
      · rw [← hocc x hx]; exact hc
      · omega
  · rw [createdOf_append, hCE', List.nil_append]; exact hb
  · rw [createdOf_append, hCE', List.nil_append]; exact hnd'

/-! ## Nodes of fitting trees -/

theorem fitsAll_getElem?_right : ∀ {ts : List IdTree} {vs : List Value},
    IdTree.fitsAll ts vs = true → ∀ {j : Nat} {v : Value}, vs[j]? = some v →
      ∃ t, ts[j]? = some t ∧ t.fits v = true
  | [], [], _, _, _, hv => by simp at hv
  | t :: ts, v' :: vs, h, 0, v, hv => by
    simp only [List.getElem?_cons_zero, Option.some.injEq] at hv
    subst hv
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    exact ⟨t, rfl, h.1⟩
  | t :: ts, v' :: vs, h, j + 1, v, hv => by
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    simp only [List.getElem?_cons_succ] at hv ⊢
    exact fitsAll_getElem?_right h.2 hv
  | [], _ :: _, h, _, _, _ => by simp [IdTree.fitsAll] at h
  | _ :: _, [], h, _, _, _ => by simp [IdTree.fitsAll] at h

theorem fitsAll_getElem?_left : ∀ {ts : List IdTree} {vs : List Value},
    IdTree.fitsAll ts vs = true → ∀ {j : Nat} {t : IdTree}, ts[j]? = some t →
      ∃ v, vs[j]? = some v ∧ t.fits v = true
  | [], [], _, _, _, ht => by simp at ht
  | t' :: ts, v :: vs, h, 0, t, ht => by
    simp only [List.getElem?_cons_zero, Option.some.injEq] at ht
    subst ht
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    exact ⟨v, rfl, h.1⟩
  | t' :: ts, v :: vs, h, j + 1, t, ht => by
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    simp only [List.getElem?_cons_succ] at ht ⊢
    exact fitsAll_getElem?_left h.2 ht
  | [], _ :: _, h, _, _, _ => by simp [IdTree.fitsAll] at h
  | _ :: _, [], h, _, _, _ => by simp [IdTree.fitsAll] at h

/-- A tree that fits a value has a node wherever the value has one. -/
theorem at_fits : ∀ (q : Path) {t : IdTree} {v w : Value}, t.fits v = true →
    v.at? q = some w → ∃ t', t.at? q = some t' ∧ t'.fits w = true
  | [], t, v, w, h, hw => by
    simp only [Value.at?, Option.some.injEq] at hw
    subst hw
    exact ⟨t, rfl, h⟩
  | j :: q, .node i cs, v, w, h, hw => by
    simp only [Value.at?] at hw
    split at hw
    · next c hc =>
      simp only [IdTree.fits] at h
      obtain ⟨t', ht', hfit⟩ := fitsAll_getElem?_right h hc
      obtain ⟨t'', h1, h2⟩ := at_fits q hfit hw
      refine ⟨t'', ?_, h2⟩
      simp [IdTree.at?, IdTree.children, ht', h1]
    · simp at hw

theorem nodesFrom_append : ∀ (i : Nat) (xs ys : List Value),
    Value.nodesFrom i (xs ++ ys) = Value.nodesFrom i xs ++ Value.nodesFrom (i + xs.length) ys
  | i, [], ys => by simp [Value.nodesFrom]
  | i, x :: xs, ys => by
    simp only [List.cons_append, Value.nodesFrom, nodesFrom_append (i + 1) xs ys,
      List.append_assoc, List.length_cons]
    rw [show i + 1 + xs.length = i + (xs.length + 1) by omega]

theorem self_mem_nodes (v : Value) : ([], v) ∈ v.nodes := by
  cases v <;> simp [Value.nodes]

theorem mem_nodesFrom : ∀ {vs : List Value} {i j : Nat} {c : Value} {q : Path} {w : Value},
    vs[j]? = some c → (q, w) ∈ c.nodes → ((i + j) :: q, w) ∈ Value.nodesFrom i vs
  | [], _, _, _, _, _, hc, _ => by simp at hc
  | v :: vs, i, 0, c, q, w, hc, hm => by
    simp only [List.getElem?_cons_zero, Option.some.injEq] at hc
    subst hc
    simp only [Value.nodesFrom, Nat.add_zero, List.mem_append, List.mem_map]
    exact Or.inl ⟨(q, w), hm, rfl⟩
  | v :: vs, i, j + 1, c, q, w, hc, hm => by
    simp only [List.getElem?_cons_succ] at hc
    have := mem_nodesFrom (i := i + 1) hc hm
    simp only [Value.nodesFrom, List.mem_append]
    right
    rwa [show i + (j + 1) = i + 1 + j by omega]

/-- A node a value has at a path is one of its nodes. -/
theorem at_nodes : ∀ (q : Path) {v w : Value}, v.at? q = some w → (q, w) ∈ v.nodes
  | [], v, w, h => by
    simp only [Value.at?, Option.some.injEq] at h
    subst h
    exact self_mem_nodes v
  | j :: q, v, w, h => by
    simp only [Value.at?] at h
    split at h
    · next c hc =>
      have := mem_nodesFrom (i := 0) hc (at_nodes q h)
      simp only [Nat.zero_add] at this
      cases v <;> simp_all [Value.nodes, Value.children, Value.nodesFrom, nodesFrom_append]
    · simp at h

theorem nodesFrom_length_occ : ∀ {ts : List IdTree} {vs : List Value} (k : Nat),
    IdTree.fitsAll ts vs = true →
      (Value.nodesFrom k vs).length = (IdTree.occurrencesAll ts vs).length
  | [], [], _, _ => by simp [Value.nodesFrom, IdTree.occurrencesAll]
  | t :: ts, v :: vs, k, h => by
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    have := occurrences_snd t v h.1
    have h1 : (t.occurrences v).length = v.nodes.length := by
      have := congrArg List.length this; simpa [subvalues] using this
    simp [Value.nodesFrom, IdTree.occurrencesAll, nodesFrom_length_occ (k + 1) h.2, h1]
  | [], _ :: _, _, h => by simp [IdTree.fitsAll] at h
  | _ :: _, [], _, h => by simp [IdTree.fitsAll] at h

theorem occurrences_length {t : IdTree} {v : Value} (h : t.fits v = true) :
    (t.occurrences v).length = v.nodes.length := by
  have := congrArg List.length (occurrences_snd t v h)
  simpa [subvalues] using this

mutual
/-- The nodes of a value, paired with the occurrences of a tree that fits it:
each node's path leads to that node in the value, and to the node of the tree
whose identity the occurrence has. -/
theorem nodes_zip : ∀ (t : IdTree) (v : Value), t.fits v = true →
    ∀ x ∈ v.nodes.zip (t.occurrences v),
      v.at? x.1.1 = some x.1.2 ∧ (t.at? x.1.1).map (·.id) = some x.2.1 ∧ x.2.2 = x.1.2
  | .node i cs, .vector e vs, h, x, hx => by
    simp only [Value.nodes, IdTree.occurrences, Value.children, List.zip_cons_cons,
      List.mem_cons] at hx
    rcases hx with rfl | hx
    · simp [Value.at?, IdTree.at?, IdTree.id]
    · obtain ⟨j, q, hq, hv, ht, he⟩ := nodesFrom_zip 0 cs vs h x hx
      rw [hq, Nat.zero_add]
      refine ⟨?_, ?_, he⟩
      · simp only [Value.at?, Value.children]
        cases hj : vs[j]? <;> simp_all
      · simp only [IdTree.at?, IdTree.children]
        cases hj : cs[j]? <;> simp_all
  | .node i cs, .struct s fs, h, x, hx => by
    simp only [Value.nodes, IdTree.occurrences, Value.children, List.zip_cons_cons,
      List.mem_cons] at hx
    rcases hx with rfl | hx
    · simp [Value.at?, IdTree.at?, IdTree.id]
    · obtain ⟨j, q, hq, hv, ht, he⟩ := nodesFrom_zip 0 cs fs h x hx
      rw [hq, Nat.zero_add]
      refine ⟨?_, ?_, he⟩
      · simp only [Value.at?, Value.children]
        cases hj : fs[j]? <;> simp_all
      · simp only [IdTree.at?, IdTree.children]
        cases hj : cs[j]? <;> simp_all
  | .node i cs, .variant s tag fs, h, x, hx => by
    simp only [Value.nodes, IdTree.occurrences, Value.children, List.zip_cons_cons,
      List.mem_cons] at hx
    rcases hx with rfl | hx
    · simp [Value.at?, IdTree.at?, IdTree.id]
    · obtain ⟨j, q, hq, hv, ht, he⟩ := nodesFrom_zip 0 cs fs h x hx
      rw [hq, Nat.zero_add]
      refine ⟨?_, ?_, he⟩
      · simp only [Value.at?, Value.children]
        cases hj : fs[j]? <;> simp_all
      · simp only [IdTree.at?, IdTree.children]
        cases hj : cs[j]? <;> simp_all
  | .node i cs, .table s fs ks es, h, x, hx => by
    simp only [IdTree.fits, Value.children] at h
    have hlen := fitsAll_length h
    simp only [List.length_append] at hlen
    have hcs : cs = cs.take fs.length ++ cs.drop fs.length := (List.take_append_drop _ _).symm
    have hl1 : (cs.take fs.length).length = fs.length := by simp; omega
    rw [hcs, fitsAll_append hl1, Bool.and_eq_true] at h
    simp only [Value.nodes, IdTree.occurrences, Value.children, List.zip_cons_cons,
      List.mem_cons] at hx
    rw [hcs, occurrencesAll_append hl1,
      List.zip_append (nodesFrom_length_occ 0 h.1)] at hx
    rcases hx with rfl | hx
    · simp [Value.at?, IdTree.at?, IdTree.id]
    · rcases List.mem_append.1 hx with hx | hx
      · obtain ⟨j, q, hq, hv, ht, he⟩ := nodesFrom_zip 0 _ fs h.1 x hx
        rw [hq, Nat.zero_add]
        refine ⟨?_, ?_, he⟩
        · simp only [Value.at?, Value.children]
          cases hj : fs[j]? with
          | none => simp [hj] at hv
          | some c =>
            rw [List.getElem?_append_left (List.getElem?_eq_some_iff.1 hj).1, hj]
            simpa [hj] using hv
        · simp only [IdTree.at?, IdTree.children]
          cases hj : (cs.take fs.length)[j]? with
          | none => simp [hj] at ht
          | some c =>
            have hc : cs[j]? = some c := by
              rw [List.getElem?_take] at hj
              split at hj
              · exact hj
              · cases hj
            simpa [hc, hj] using ht
      · obtain ⟨j, q, hq, hv, ht, he⟩ := nodesFrom_zip fs.length _ es h.2 x hx
        rw [hq]
        refine ⟨?_, ?_, he⟩
        · simp only [Value.at?, Value.children]
          rw [List.getElem?_append_right (Nat.le_add_right _ _), Nat.add_sub_cancel_left]
          cases hj : es[j]? with
          | none => simp [hj] at hv
          | some c => simpa [hj] using hv
        · simp only [IdTree.at?, IdTree.children]
          have : cs[fs.length + j]? = (cs.drop fs.length)[j]? := by simp [List.getElem?_drop]
          rw [this]
          cases hj : (cs.drop fs.length)[j]? with
          | none => simp [hj] at ht
          | some c => simpa [hj] using ht
  | .node i cs, .closure f mask vs ps rs a, h, x, hx => by
    simp only [Value.nodes, IdTree.occurrences, Value.children, List.zip_cons_cons,
      List.mem_cons] at hx
    rcases hx with rfl | hx
    · simp [Value.at?, IdTree.at?, IdTree.id]
    · obtain ⟨j, q, hq, hv, ht, he⟩ := nodesFrom_zip 0 cs vs h x hx
      rw [hq, Nat.zero_add]
      refine ⟨?_, ?_, he⟩
      · simp only [Value.at?, Value.children]
        cases hj : vs[j]? <;> simp_all
      · simp only [IdTree.at?, IdTree.children]
        cases hj : cs[j]? <;> simp_all
  | .node i cs, .bool b, _, x, hx => by
    simp only [Value.nodes, IdTree.occurrences, Value.children] at hx
    cases cs <;> simp [IdTree.occurrencesAll] at hx <;> subst hx <;>
      simp [Value.at?, IdTree.at?, IdTree.id]
  | .node i cs, .int t' y, _, x, hx => by
    simp only [Value.nodes, IdTree.occurrences, Value.children] at hx
    cases cs <;> simp [IdTree.occurrencesAll] at hx <;> subst hx <;>
      simp [Value.at?, IdTree.at?, IdTree.id]
  | .node i cs, .address a, _, x, hx => by
    simp only [Value.nodes, IdTree.occurrences, Value.children] at hx
    cases cs <;> simp [IdTree.occurrencesAll] at hx <;> subst hx <;>
      simp [Value.at?, IdTree.at?, IdTree.id]
  | .node i cs, .signer a, h, x, hx => by
    simp only [IdTree.fits, Value.children] at h
    match cs, h with
    | [.node j []], _ =>
      simp only [Value.nodes, IdTree.occurrences, Value.children, IdTree.occurrencesAll,
        List.append_nil, List.zip_cons_cons, List.zip_nil_left, List.mem_cons, List.not_mem_nil,
        or_false] at hx
      rcases hx with rfl | rfl
      · simp [Value.at?, IdTree.at?, IdTree.id]
      · simp [Value.at?, IdTree.at?, IdTree.id, Value.children, IdTree.children]
    | [], h => simp [IdTree.fitsAll] at h
    | [.node j (_ :: _)], h => simp [IdTree.fitsAll, IdTree.fits, Value.children] at h
    | _ :: _ :: _, h => simp [IdTree.fitsAll] at h
  | .node i cs, .ref r, _, x, hx => by
    simp only [Value.nodes, IdTree.occurrences, Value.children] at hx
    cases cs <;> simp [IdTree.occurrencesAll] at hx <;> subst hx <;>
      simp [Value.at?, IdTree.at?, IdTree.id]

theorem nodesFrom_zip : ∀ (k : Nat) (ts : List IdTree) (vs : List Value),
    IdTree.fitsAll ts vs = true →
    ∀ x ∈ (Value.nodesFrom k vs).zip (IdTree.occurrencesAll ts vs),
      ∃ j q, x.1.1 = (k + j) :: q ∧ vs[j]?.bind (·.at? q) = some x.1.2 ∧
        (ts[j]?.bind (·.at? q)).map (·.id) = some x.2.1 ∧ x.2.2 = x.1.2
  | _, [], [], _, x, hx => by simp [Value.nodesFrom] at hx
  | k, t :: ts, v :: vs, h, x, hx => by
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    simp only [Value.nodesFrom, IdTree.occurrencesAll] at hx
    rw [List.zip_append (by simp [occurrences_length h.1]), List.mem_append] at hx
    rcases hx with hx | hx
    · rw [List.zip_map_left, List.mem_map] at hx
      obtain ⟨y, hy, rfl⟩ := hx
      obtain ⟨hv, ht, he⟩ := nodes_zip t v h.1 y hy
      exact ⟨0, y.1.1, by simp, by simpa using hv, by simpa using ht, by simpa using he⟩
    · obtain ⟨j, q, hq, hv, ht, he⟩ := nodesFrom_zip (k + 1) ts vs h.2 x hx
      exact ⟨j + 1, q, by rw [hq]; congr 1; omega, by simpa using hv, by simpa using ht, he⟩
  | _, [], _ :: _, h, _, _ => by simp [IdTree.fitsAll] at h
  | _, _ :: _, [], h, _, _ => by simp [IdTree.fitsAll] at h
end

mutual
/-- The paths of a value's nodes are distinct. -/
theorem nodes_nodup : ∀ (v : Value), (v.nodes.map (·.1)).Nodup
  | .vector e vs => by
    obtain ⟨hnd, hk⟩ := nodesFrom_nodup 0 vs
    simp only [Value.nodes, List.map_cons, List.nodup_cons]
    exact ⟨fun h => by obtain ⟨j, q, hjq, -⟩ := hk [] h; simp at hjq, hnd⟩
  | .struct s fs => by
    obtain ⟨hnd, hk⟩ := nodesFrom_nodup 0 fs
    simp only [Value.nodes, List.map_cons, List.nodup_cons]
    exact ⟨fun h => by obtain ⟨j, q, hjq, -⟩ := hk [] h; simp at hjq, hnd⟩
  | .variant s tag fs => by
    obtain ⟨hnd, hk⟩ := nodesFrom_nodup 0 fs
    simp only [Value.nodes, List.map_cons, List.nodup_cons]
    exact ⟨fun h => by obtain ⟨j, q, hjq, -⟩ := hk [] h; simp at hjq, hnd⟩
  | .closure _ _ cs _ _ _ => by
    obtain ⟨hnd, hk⟩ := nodesFrom_nodup 0 cs
    simp only [Value.nodes, List.map_cons, List.nodup_cons]
    exact ⟨fun h => by obtain ⟨j, q, hjq, -⟩ := hk [] h; simp at hjq, hnd⟩
  | .table s fs ks es => by
    obtain ⟨hnd₁, hk₁⟩ := nodesFrom_nodup 0 fs
    obtain ⟨hnd₂, hk₂⟩ := nodesFrom_nodup fs.length es
    simp only [Value.nodes, List.map_cons, List.map_append, List.nodup_cons, List.mem_append]
    refine ⟨fun h => ?_, List.nodup_append.2 ⟨hnd₁, hnd₂, ?_⟩⟩
    · rcases h with h | h
      · obtain ⟨j, q, hjq, -⟩ := hk₁ [] h; simp at hjq
      · obtain ⟨j, q, hjq, -⟩ := hk₂ [] h; simp at hjq
    · intro a ha b hb hab
      obtain ⟨j, q, rfl, hj⟩ := hk₁ a ha
      obtain ⟨j', q', rfl, -⟩ := hk₂ b hb
      simp only [List.cons.injEq] at hab
      omega
  | .bool _ => by simp [Value.nodes]
  | .int _ _ => by simp [Value.nodes]
  | .address _ => by simp [Value.nodes]
  | .signer _ => by simp [Value.nodes]
  | .ref _ => by simp [Value.nodes]

theorem nodesFrom_nodup : ∀ (k : Nat) (vs : List Value),
    ((Value.nodesFrom k vs).map (·.1)).Nodup ∧
      ∀ p ∈ (Value.nodesFrom k vs).map (·.1), ∃ j q, p = (k + j) :: q ∧ j < vs.length
  | _, [] => by simp [Value.nodesFrom]
  | k, v :: vs => by
    obtain ⟨hnd, hk⟩ := nodesFrom_nodup (k + 1) vs
    have hv := nodes_nodup v
    simp only [Value.nodesFrom, List.map_append, List.map_map]
    have hfirst : (v.nodes.map ((·.1) ∘ fun (p, w) => (k :: p, w))) =
        (v.nodes.map (·.1)).map (k :: ·) := by
      rw [List.map_map]; rfl
    rw [hfirst]
    refine ⟨List.nodup_append.2 ⟨List.Pairwise.map (k :: ·)
      (fun a b hab h => hab (List.cons.inj h).2) hv, hnd, ?_⟩, ?_⟩
    · intro a ha b hb hab
      obtain ⟨p, -, rfl⟩ := List.mem_map.1 ha
      obtain ⟨j, q, rfl, -⟩ := hk b hb
      simp only [List.cons.injEq] at hab
      omega
    · intro p hp
      rcases List.mem_append.1 hp with hp | hp
      · obtain ⟨q, -, rfl⟩ := List.mem_map.1 hp
        exact ⟨0, q, by simp, by simp⟩
      · obtain ⟨j, q, rfl, hj⟩ := hk p hp
        exact ⟨j + 1, q, by congr 1; omega, by simp; omega⟩
end

/-! ## Positions of argument nodes -/

/-- The nodes of arguments `vs` (numbered from `k`), whose shadows are `ts`,
at their positions (argument index, path), with their occurrences. -/
def argPos (k : Nat) : List IdTree → List Value → List ((Nat × Path) × (Nat × Value))
  | t :: ts, a :: as =>
    (a.nodes.zip (t.occurrences a)).map (fun y => ((k, y.1.1), y.2)) ++ argPos (k + 1) ts as
  | [], _ => []
  | _ :: _, [] => []

/-- The occurrence at a position of the arguments. -/
def argOcc (ts : List IdTree) (vs : List Value) (x : Nat × Path) : Nat × Value :=
  ((((ts[x.1]?).bind (·.at? x.2)).map (·.id)).getD 0,
    ((vs[x.1]?).bind (·.at? x.2)).getD (.bool false))

/-- The nodes of values `vs`, numbered from `k`, as `nodesOf` lists them. -/
def nodesOfFrom (k : Nat) (vs : List Value) : List (Nat × Path × Value) :=
  (vs.zipIdx k).flatMap fun x => x.1.nodes.map fun y => (x.2, y.1, y.2)

theorem nodesOf_eq (vs : List Value) : nodesOf vs = nodesOfFrom 0 vs := rfl

theorem argPos_snd : ∀ {k : Nat} {ts : List IdTree} {vs : List Value},
    IdTree.fitsAll ts vs = true → (argPos k ts vs).map (·.2) = IdTree.occurrencesAll ts vs
  | _, [], [], _ => by simp [argPos, IdTree.occurrencesAll]
  | k, t :: ts, a :: as, h => by
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    simp only [argPos, List.map_append, List.map_map, IdTree.occurrencesAll]
    rw [argPos_snd h.2]
    congr 1
    exact List.map_snd_zip (by simp [occurrences_length h.1])
  | _, [], _ :: _, h => by simp [IdTree.fitsAll] at h
  | _, _ :: _, [], h => by simp [IdTree.fitsAll] at h

theorem argPos_ge : ∀ {k : Nat} {ts : List IdTree} {vs : List Value},
    ∀ x ∈ argPos k ts vs, k ≤ x.1.1
  | _, [], _, _, hx => by simp [argPos] at hx
  | _, _ :: _, [], _, hx => by simp [argPos] at hx
  | k, t :: ts, a :: as, x, hx => by
    simp only [argPos, List.mem_append, List.mem_map] at hx
    rcases hx with ⟨y, -, rfl⟩ | hx
    · exact Nat.le_refl _
    · have := argPos_ge x hx; omega

theorem argPos_occ : ∀ {k : Nat} {ts : List IdTree} {vs : List Value},
    IdTree.fitsAll ts vs = true → ∀ x ∈ argPos k ts vs, x.2 = argOcc ts vs (x.1.1 - k, x.1.2)
  | _, [], [], _, _, hx => by simp [argPos] at hx
  | k, t :: ts, a :: as, h, x, hx => by
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    simp only [argPos, List.mem_append, List.mem_map] at hx
    rcases hx with ⟨y, hy, rfl⟩ | hx
    · obtain ⟨hv, ht, he⟩ := nodes_zip t a h.1 y hy
      simp only [Nat.sub_self, argOcc, List.getElem?_cons_zero, Option.bind_some, ht, hv,
        Option.getD_some]
      rw [← he]
    · have hk := argPos_ge x hx
      rw [argPos_occ h.2 x hx]
      simp only [argOcc]
      rw [show x.1.1 - k = (x.1.1 - (k + 1)) + 1 by omega]
      simp
  | _, [], _ :: _, h, _, _ => by simp [IdTree.fitsAll] at h
  | _, _ :: _, [], h, _, _ => by simp [IdTree.fitsAll] at h

theorem argPos_nodup : ∀ {k : Nat} {ts : List IdTree} {vs : List Value},
    IdTree.fitsAll ts vs = true → ((argPos k ts vs).map (·.1)).Nodup
  | _, [], [], _ => by simp [argPos]
  | k, t :: ts, a :: as, h => by
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    simp only [argPos, List.map_append, List.map_map]
    have hfirst : (a.nodes.zip (t.occurrences a)).map ((·.1) ∘ fun y => ((k, y.1.1), y.2)) =
        (a.nodes.map (·.1)).map (k, ·) := by
      have hz := List.map_fst_zip (l₁ := a.nodes) (l₂ := t.occurrences a)
        (by simp [occurrences_length h.1])
      conv => rhs; rw [← hz]
      simp [List.map_map, Function.comp_def]
    rw [hfirst]
    refine List.nodup_append.2 ⟨List.Pairwise.map (k, ·)
      (fun a b hab h => hab (Prod.mk.inj h).2) (nodes_nodup a), argPos_nodup h.2, ?_⟩
    intro x hx y hy hxy
    obtain ⟨q, -, rfl⟩ := List.mem_map.1 hx
    obtain ⟨z, hz, rfl⟩ := List.mem_map.1 hy
    have := argPos_ge z hz
    rw [← hxy] at this
    simp at this
    omega
  | _, [], _ :: _, h => by simp [IdTree.fitsAll] at h
  | _, _ :: _, [], h => by simp [IdTree.fitsAll] at h

theorem argPos_nodes : ∀ {k : Nat} {ts : List IdTree} {vs : List Value},
    IdTree.fitsAll ts vs = true →
      ∀ x ∈ argPos k ts vs, (x.1.1, x.1.2, x.2.2) ∈ nodesOfFrom k vs
  | _, [], [], _, _, hx => by simp [argPos] at hx
  | k, t :: ts, a :: as, h, x, hx => by
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    simp only [argPos, List.mem_append, List.mem_map] at hx
    simp only [nodesOfFrom, List.zipIdx_cons, List.flatMap_cons, List.mem_append, List.mem_map]
    rcases hx with ⟨y, hy, rfl⟩ | hx
    · obtain ⟨-, -, he⟩ := nodes_zip t a h.1 y hy
      left
      exact ⟨y.1, (List.of_mem_zip hy).1, by simp [he]⟩
    · right
      exact argPos_nodes h.2 x hx
  | _, [], _ :: _, h, _, _ => by simp [IdTree.fitsAll] at h
  | _, _ :: _, [], h, _, _ => by simp [IdTree.fitsAll] at h

theorem argPos_mem : ∀ {k : Nat} {ts : List IdTree} {vs : List Value},
    IdTree.fitsAll ts vs = true → ∀ {j : Nat} {q : Path} {w : Value},
      vs[j]?.bind (·.at? q) = some w → (k + j, q) ∈ (argPos k ts vs).map (·.1)
  | _, [], [], _, _, _, _, hw => by simp at hw
  | k, t :: ts, a :: as, h, 0, q, w, hw => by
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    simp only [List.getElem?_cons_zero, Option.bind_some] at hw
    have hn := at_nodes q hw
    rw [← List.map_fst_zip (l₁ := a.nodes) (l₂ := t.occurrences a)
      (by simp [occurrences_length h.1])] at hn
    obtain ⟨y, hy, hyq⟩ := List.mem_map.1 hn
    simp only [argPos, List.map_append, List.map_map, List.mem_append, List.mem_map]
    left
    exact ⟨y, hy, by simp [hyq]⟩
  | k, t :: ts, a :: as, h, j + 1, q, w, hw => by
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    simp only [List.getElem?_cons_succ] at hw
    have := argPos_mem (k := k + 1) h.2 hw
    simp only [argPos, List.map_append, List.mem_append]
    right
    rwa [show k + (j + 1) = k + 1 + j by omega]
  | _, [], _ :: _, h, _, _, _, _ => by simp [IdTree.fitsAll] at h
  | _, _ :: _, [], h, _, _, _, _ => by simp [IdTree.fitsAll] at h

/-! ## Consumed argument nodes -/

/-- The consumption events of a native call: for every argument node (from
argument `k` on) not at a position of `origins`, its identity and value, by
consumer `cons` at its position. -/
def consumedFrom (origins : List (Nat × Path)) (cons : Nat → Path → Consumer) (k : Nat)
    (ts : List IdTree) (vs : List Value) : List Event :=
  ((ts.zip vs).zipIdx k).flatMap fun x =>
    (x.1.2.nodes.zip (x.1.1.occurrences x.1.2)).filterMap fun y =>
      if origins.contains (x.2, y.1.1) then none
      else some (Event.consumed y.2.1 y.1.2 (cons x.2 y.1.1))

theorem consumedOf_filterMap {β : Type} (l : List β) (p : β → Bool) (i : β → Nat)
    (w : β → Value) (c : β → Consumer) :
    consumedOf (l.filterMap fun y =>
        if p y then none else some (Event.consumed (i y) (w y) (c y))) =
      (l.filter fun y => !p y).map fun y => (i y, w y) := by
  induction l with
  | nil => rfl
  | cons y l ih =>
    by_cases hp : p y
    · simp [hp, ih]
    · simp [hp, ih]

theorem createdOf_filterMap {β : Type} (l : List β) (p : β → Bool) (i : β → Nat)
    (w : β → Value) (c : β → Consumer) :
    createdOf (l.filterMap fun y =>
        if p y then none else some (Event.consumed (i y) (w y) (c y))) =
      [] := by
  induction l with
  | nil => rfl
  | cons y l ih =>
    by_cases hp : p y
    · simp [hp, ih]
    · simp [hp, ih]

theorem consumedFrom_created {origins : List (Nat × Path)} {cons : Nat → Path → Consumer} :
    ∀ {k : Nat} {ts : List IdTree} {vs : List Value},
      createdOf (consumedFrom origins cons k ts vs) = []
  | _, [], _ => by simp [consumedFrom]
  | _, _ :: _, [] => by simp [consumedFrom]
  | k, t :: ts, a :: as => by
    simp only [consumedFrom, List.zip_cons_cons, List.zipIdx_cons, List.flatMap_cons,
      createdOf_append]
    rw [createdOf_filterMap]
    exact consumedFrom_created (k := k + 1) (ts := ts) (vs := as)

theorem consumedFrom_consumed {origins : List (Nat × Path)} {cons : Nat → Path → Consumer} :
    ∀ {k : Nat} {ts : List IdTree} {vs : List Value}, IdTree.fitsAll ts vs = true →
      consumedOf (consumedFrom origins cons k ts vs) =
        ((argPos k ts vs).filter fun x => !origins.contains x.1).map (·.2)
  | _, [], [], _ => by simp [consumedFrom, argPos]
  | k, t :: ts, a :: as, h => by
    simp only [IdTree.fitsAll, Bool.and_eq_true] at h
    simp only [consumedFrom, List.zip_cons_cons, List.zipIdx_cons, List.flatMap_cons,
      consumedOf_append, argPos, List.filter_append, List.map_append]
    rw [consumedOf_filterMap]
    have := consumedFrom_consumed (origins := origins) (cons := cons) (k := k + 1) h.2
    simp only [consumedFrom] at this
    rw [this, List.filter_map, List.map_map]
    congr 1
    refine List.map_congr_left fun y hy => ?_
    obtain ⟨-, -, he⟩ := nodes_zip t a h.1 y (List.mem_filter.1 hy).1
    simp [← he]
  | _, [], _ :: _, h => by simp [IdTree.fitsAll] at h
  | _, _ :: _, [], h => by simp [IdTree.fitsAll] at h

theorem consumedFrom_mem {origins : List (Nat × Path)} {cons : Nat → Path → Consumer} :
    ∀ {k : Nat} {ts : List IdTree} {vs : List Value}, ∀ e ∈ consumedFrom origins cons k ts vs,
      ∃ k' q i w, e = .consumed i w (cons k' q) ∧ origins.contains (k', q) = false ∧
        (k', q, w) ∈ nodesOfFrom k vs
  | _, [], _, _, he => by simp [consumedFrom] at he
  | _, _ :: _, [], _, he => by simp [consumedFrom] at he
  | k, t :: ts, a :: as, e, he => by
    simp only [consumedFrom, List.zip_cons_cons, List.zipIdx_cons, List.flatMap_cons,
      List.mem_append] at he
    simp only [nodesOfFrom, List.zipIdx_cons, List.flatMap_cons, List.mem_append, List.mem_map]
    rcases he with he | he
    · obtain ⟨y, hy, hye⟩ := List.mem_filterMap.1 he
      by_cases hc : origins.contains (k, y.1.1) = true
      · rw [if_pos hc] at hye; exact absurd hye (by simp)
      · simp only [hc, Bool.false_eq_true, ↓reduceIte, Option.some.injEq] at hye
        exact ⟨k, y.1.1, y.2.1, y.1.2, hye.symm, by simpa using hc,
          Or.inl ⟨y.1, (List.of_mem_zip hy).1, rfl⟩⟩
    · obtain ⟨k', q, i, w, rfl, hc, hm⟩ :=
        consumedFrom_mem (k := k + 1) (ts := ts) (vs := as) e (by simpa [consumedFrom] using he)
      exact ⟨k', q, i, w, rfl, hc, Or.inr (by simpa [nodesOfFrom] using hm)⟩

/-! ## Result nodes -/

/-- Assigning identities to nodes: those `idOf` names are passed on, the
others created, with fresh identities. -/
theorem assign_spec (idOf : Path → Option Nat) (c : Creator) (p : Path) :
    ∀ (xs : List (Path × Value)) (n : Nat),
      (assign idOf c p xs n).1.Perm
        (xs.filterMap (fun y => (idOf (p ++ y.1)).map fun i => (i, y.2)) ++
          createdOf (assign idOf c p xs n).2.2) ∧
      consumedOf (assign idOf c p xs n).2.2 = [] ∧
      (∀ o ∈ createdOf (assign idOf c p xs n).2.2,
        n ≤ o.1 ∧ o.1 < (assign idOf c p xs n).2.1) ∧
      ((createdOf (assign idOf c p xs n).2.2).map (·.1)).Nodup ∧
      n ≤ (assign idOf c p xs n).2.1 ∧
      (∀ e ∈ (assign idOf c p xs n).2.2,
        ∃ y ∈ xs, idOf (p ++ y.1) = none ∧ ∃ i, e = .created i y.2 c)
  | [], n => by simp [assign]
  | (q, w) :: xs, n => by
    simp only [assign]
    cases hq : idOf (p ++ q) with
    | some i =>
      dsimp only
      obtain ⟨hp, hc, hb, hnd, hle, hev⟩ := assign_spec idOf c p xs n
      refine ⟨?_, hc, hb, hnd, hle, ?_⟩
      · simp only [List.filterMap_cons, hq, Option.map_some, List.cons_append]
        exact hp.cons _
      · intro e he
        obtain ⟨y, hy, hy'⟩ := hev e he
        exact ⟨y, List.mem_cons_of_mem _ hy, hy'⟩
    | none =>
      dsimp only
      obtain ⟨hp, hc, hb, hnd, hle, hev⟩ := assign_spec idOf c p xs (n + 1)
      refine ⟨?_, by simpa using hc, ?_, ?_, by omega, ?_⟩
      · simp only [List.filterMap_cons, hq, Option.map_none, createdOf_cons_created]
        exact (hp.cons _).trans List.perm_middle.symm
      · intro o ho
        simp only [createdOf_cons_created, List.mem_cons] at ho
        rcases ho with rfl | ho
        · simp; omega
        · have := hb o ho; omega
      · simp only [createdOf_cons_created, List.map_cons, List.nodup_cons]
        refine ⟨fun hn => ?_, hnd⟩
        obtain ⟨o, ho, hon⟩ := List.mem_map.1 hn
        have := (hb o ho).1
        omega
      · intro e he
        simp only [List.mem_cons] at he
        rcases he with rfl | he
        · exact ⟨(q, w), List.mem_cons_self .., hq, n, rfl⟩
        · obtain ⟨y, hy, hy'⟩ := hev e he
          exact ⟨y, List.mem_cons_of_mem _ hy, hy'⟩

/-- One step of annotating a native's results, as `Shadow.nativeStep` does. -/
def resultStep (idOf : Nat → Path → Option Nat) (c : Creator)
    (acc : List IdTree × Nat × List Event) (x : Value × Nat) :
    List IdTree × Nat × List Event :=
  (acc.1 ++ [(IdTree.build (idOf x.2) c [] x.1 acc.2.1).1],
    (IdTree.build (idOf x.2) c [] x.1 acc.2.1).2.1,
    acc.2.2 ++ (IdTree.build (idOf x.2) c [] x.1 acc.2.1).2.2)

theorem results_fold (idOf : Nat → Path → Option Nat) (c : Creator) :
    ∀ (Lz : List (Value × Nat)) (ts0 : List IdTree) (n0 : Nat) (evs0 : List Event),
      ∃ ts n evs, Lz.foldl (resultStep idOf c) (ts0, n0, evs0) = (ts0 ++ ts, n, evs0 ++ evs) ∧
        IdTree.fitsAll ts (Lz.map (·.1)) = true ∧
        (IdTree.occurrencesAll ts (Lz.map (·.1))).Perm
          (Lz.flatMap (fun x =>
              x.1.nodes.filterMap fun y => (idOf x.2 y.1).map fun i => (i, y.2)) ++
            createdOf evs) ∧
        consumedOf evs = [] ∧ (∀ o ∈ createdOf evs, n0 ≤ o.1 ∧ o.1 < n) ∧
        ((createdOf evs).map (·.1)).Nodup ∧ n0 ≤ n ∧
        (∀ e ∈ evs, ∃ x ∈ Lz, ∃ y ∈ x.1.nodes,
          idOf x.2 y.1 = none ∧ ∃ i, e = .created i y.2 c)
  | [], ts0, n0, evs0 => ⟨[], n0, [], by simp, by simp [IdTree.fitsAll],
      by simp [IdTree.occurrencesAll], rfl, by simp, by simp, Nat.le_refl _, by simp⟩
  | x :: Lz, ts0, n0, evs0 => by
    obtain ⟨hfit, hspec⟩ := build_spec (idOf x.2) c [] x.1 n0
    have hocc := congrArg (·.1) hspec
    have hn1 := congrArg (·.2.1) hspec
    have hevs := congrArg (·.2.2) hspec
    simp only at hocc hn1 hevs
    obtain ⟨hp, hc, hb, hnd, hle, hev⟩ := assign_spec (idOf x.2) c [] x.1.nodes n0
    rw [← hocc, ← hevs] at hp
    rw [← hevs] at hc hb hnd hev
    rw [← hn1] at hb hle
    obtain ⟨ts, n, evs, hf, hfit', hp', hc', hb', hnd', hle', hev'⟩ :=
      results_fold idOf c Lz (ts0 ++ [(IdTree.build (idOf x.2) c [] x.1 n0).1])
        (IdTree.build (idOf x.2) c [] x.1 n0).2.1
        (evs0 ++ (IdTree.build (idOf x.2) c [] x.1 n0).2.2)
    refine ⟨(IdTree.build (idOf x.2) c [] x.1 n0).1 :: ts, n,
      (IdTree.build (idOf x.2) c [] x.1 n0).2.2 ++ evs, ?_, ?_, ?_, ?_, ?_, ?_, by omega, ?_⟩
    · show List.foldl (resultStep idOf c) (resultStep idOf c (ts0, n0, evs0) x) Lz = _
      simp only [resultStep]
      rw [hf]
      simp
    · simp [IdTree.fitsAll, hfit, hfit']
    · simp only [List.map_cons, IdTree.occurrencesAll, List.flatMap_cons, createdOf_append]
      refine (hp.append hp').trans ?_
      simp only [List.nil_append, List.append_assoc]
      refine List.Perm.append_left _ ?_
      rw [← List.append_assoc, ← List.append_assoc]
      exact List.perm_append_comm.append_right _
    · rw [consumedOf_append, hc, hc']; rfl
    · intro o ho
      rw [createdOf_append, List.mem_append] at ho
      rcases ho with ho | ho
      · have := hb o ho; omega
      · have := hb' o ho; omega
    · rw [createdOf_append, List.map_append, List.nodup_append]
      refine ⟨hnd, hnd', fun a ha b hb'' hab => ?_⟩
      obtain ⟨o, ho, rfl⟩ := List.mem_map.1 ha
      obtain ⟨o', ho', rfl⟩ := List.mem_map.1 hb''
      have := (hb o ho).2
      have := (hb' o' ho').1
      omega
    · intro e he
      rcases List.mem_append.1 he with he | he
      · obtain ⟨y, hy, hy'⟩ := hev e he
        exact ⟨x, List.mem_cons_self .., y, hy, by simpa using hy'⟩
      · obtain ⟨x', hx', hy'⟩ := hev' e he
        exact ⟨x', List.mem_cons_of_mem _ hx', hy'⟩

/-! ## The annotation of a native call -/

/-- The identity of node `q` of argument `k`. -/
def argIdAt (argIds : List IdTree) (k : Nat) (q : Path) : Option Nat :=
  (argIds[k]?.bind (·.at? q)).map (·.id)

/-- The identity a result node keeps, if its flow passes an argument node on. -/
def resultIdOf (φ : Nat → Path → Origin) (argIds : List IdTree) (j : Nat) (p : Path) :
    Option Nat :=
  match φ j p with
  | .arg k q => argIdAt argIds k q
  | .fresh => none

/-- The shadows of a native's results, the fresh counter after them, and
their creation events. -/
def nativeResults (ν : NativeFlows) (f : FunctionRef) (args vals : List Value)
    (argIds : List IdTree) (m : ModuleId) (fresh : Nat) : List IdTree × Nat × List Event :=
  vals.zipIdx.foldl (resultStep (resultIdOf (ν f args vals) argIds) (.native f m args))
    ([], fresh, [])

/-- The shadow after a native call. -/
def nativePost (ν : NativeFlows) (f : FunctionRef) (args vals : List Value) (h : Shadow)
    (g : ShadowFrame) (hs : List ShadowFrame) (m : ModuleId) : ShadowPost :=
  let r := nativeResults ν f args vals (g.stack.take args.length).reverse m h.fresh
  .running { h with
    frames := { g with stack := r.1.reverse ++ g.stack.drop args.length } :: hs
    fresh := r.2.1 }

/-- The events of a native call. -/
def nativeEvents (ν : NativeFlows) (f : FunctionRef) (args vals : List Value) (g : ShadowFrame)
    (m : ModuleId) (fresh : Nat) : List Event :=
  consumedFrom (passedOn (ν f args vals) vals)
      (fun k q => .native f m (keepsNothingInside (passedOn (ν f args vals) vals) k q)) 0
      (g.stack.take args.length).reverse args ++
    (nativeResults ν f args vals (g.stack.take args.length).reverse m fresh).2.2

/-- The trees of what a native that takes values saw: its arguments' own. -/
theorem mapM_zip_values {F : Value × IdTree → Option IdTree}
    (hF : ∀ a t, (∀ r, a ≠ .ref r) → F (a, t) = some t) :
    ∀ {args : List Value} {ts : List IdTree}, (∀ v ∈ args, ∀ r, v ≠ .ref r) →
      args.length = ts.length → (args.zip ts).mapM F = some ts
  | [], [], _, _ => rfl
  | a :: args, t :: ts, hargs, hl => by
    have ih := mapM_zip_values hF (args := args) (ts := ts)
      (fun v hv => hargs v (List.mem_cons_of_mem _ hv)) (by simpa using hl)
    rw [List.zip_cons_cons, List.mapM_cons, ih, hF a t (hargs a List.mem_cons_self)]
    rfl
  | [], _ :: _, _, hl => by simp at hl
  | _ :: _, [], _, hl => by simp at hl

/-- A native that takes values has no reference arguments. -/
theorem filterMap_zip_values {β : Type} {F : IdTree × Value → Option β}
    (hF : ∀ t a, (∀ r, a ≠ .ref r) → F (t, a) = none) {args : List Value}
    (hargs : ∀ v ∈ args, ∀ r, v ≠ .ref r) (ts : List IdTree) : (ts.zip args).filterMap F = [] := by
  rw [List.filterMap_eq_nil_iff]
  intro x hx
  exact hF x.1 x.2 (hargs x.2 (List.of_mem_zip hx).2)

/-- A native that returns values has no reference results. -/
theorem filterMap_zip_valueTys {β : Type} {F : RuntimeType × Value → Option β}
    (hF : ∀ t v, t.isValue = true → F (t, v) = none) {tys : List RuntimeType}
    (hrv : ∀ t ∈ tys, t.isValue = true) (vs : List Value) : (tys.zip vs).filterMap F = [] := by
  rw [List.filterMap_eq_nil_iff]
  intro x hx
  exact hF x.1 x.2 (hrv x.1 (List.of_mem_zip hx).1)

/-- A fold that appends one tree per element appends as many as there are
elements. -/
theorem foldl_snoc_length {α γ ε : Type} (A : List γ × Nat × List ε → α → γ)
    (B : List γ × Nat × List ε → α → Nat) (C : List γ × Nat × List ε → α → List ε) :
    ∀ (L : List α) (acc : List γ × Nat × List ε),
      (L.foldl (fun acc x => (acc.1 ++ [A acc x], B acc x, acc.2.2 ++ C acc x)) acc).1.length =
        acc.1.length + L.length
  | [], _ => by simp
  | x :: L, acc => by
    rw [List.foldl_cons, foldl_snoc_length A B C L]
    simp; omega

/-- **The annotation of a call of a native that takes and returns values**
(`Program.nativeShape?` none, value arguments, nothing written back, no
places): it is the annotation of a call on values, `nativePost` and
`nativeEvents`, when its guards hold. -/
theorem nativeStep_values {P : Program} {ν : NativeFlows} {s : State} {f : FunctionRef}
    {args vals : List Value} {o : Outcome} {h : Shadow} {g : ShadowFrame} {hs : List ShadowFrame}
    {info : FunctionInfo} {ptys tys : List RuntimeType}
    (hframes : h.frames = g :: hs) (hinfo : P.function? f = some info)
    (hpar : P.paramTypes f = some ptys) (hret : P.returnTypes f = some tys)
    (hpv : ∀ t ∈ ptys, t.isValue = true) (hrv : ∀ t ∈ tys, t.isValue = true)
    (hvl : vals.length = tys.length) (hargs : ∀ v ∈ args, ∀ r, v ≠ .ref r) :
    Shadow.nativeStep P ν s f args args vals [] [] o h =
      if args.length ≤ g.stack.length ∧
        ((nodesOf vals).all fun x =>
          match ν f args vals x.1 x.2.1 with
          | .arg k q => (argIdAt (g.stack.take args.length).reverse k q).isSome
          | .fresh => true) = true ∧
        (nativePost ν f args vals h g hs info.moduleId).fits o = true
      then some (nativePost ν f args vals h g hs info.moduleId,
        nativeEvents ν f args vals g info.moduleId h.fresh)
      else none := by
  have hshared : ∀ k : Nat, (ptys[k]?).any (P.nativeKeeps f) = false := by
    intro k
    cases hk : ptys[k]? with
    | none => rfl
    | some t =>
      have := hpv t (List.mem_of_getElem? hk)
      cases t <;> simp_all [Program.nativeKeeps, RuntimeType.isValue]
  unfold Shadow.nativeStep
  simp only [hframes, hinfo, hret, hpar, Option.bind_eq_bind, Option.bind_some, List.append_nil]
  by_cases hlen : args.length ≤ g.stack.length
  case neg => simp [guard, hlen]
  have hargl : args.length = (List.take args.length g.stack).reverse.length := by
    simp; omega
  rw [mapM_zip_values (fun a t ha => by cases a <;> first | rfl | exact absurd rfl (ha _)) hargs hargl,
    filterMap_zip_values (fun t a ha => by cases a <;> first | rfl | exact absurd rfl (ha _)) hargs]
  simp only [guard, hlen, ↓reduceIte, Option.pure_def, Option.bind_some, true_and, ← hargl,
    beq_self_eq_true, List.filter_nil, List.length_nil]
  split
  case isFalse hg =>
    simp only [Option.bind, failure]
    rw [if_neg]
    intro hc
    exact hg hc.1
  case isTrue hg =>
  simp only [Option.bind_some]
  cases o with
  | running s'' =>
    simp only
    cases hfr : s''.frames with
    | nil =>
      simp only
      rw [if_neg]
      intro hc
      have hfit := hc.2
      simp [nativePost, ShadowPost.fits, Shadow.fits, hfr] at hfit
    | cons fr' tail =>
      simp only
      rw [filterMap_zip_valueTys (fun t v ht => by cases v <;> simp [ht]) hrv]
      simp only [List.length_nil, beq_self_eq_true, ↓reduceIte, Option.bind_some,
        List.zip_nil_right, List.zipIdx_nil, List.mapM_nil, List.map_nil, Option.pure_def]
      rw [nativeResultsIn_values hrv (by
        rw [List.length_take, foldl_snoc_length]; simp; omega)]
      simp only [Option.bind_some, List.zip_nil_left, List.foldlM_nil, Option.pure_def,
        Nat.add_zero]
      simp only [List.take_of_length_le, foldl_snoc_length, List.length_nil, List.length_zipIdx,
        Nat.zero_add, Nat.le_refl]
      split
      case isFalse hfit =>
        simp only [Option.bind, failure]
        rw [if_neg]
        intro hc
        exact hfit hc.2
      case isTrue hfit =>
      rw [if_pos ⟨hg, hfit⟩]
      simp only [Option.bind_some, Option.some.injEq, Prod.mk.injEq]
      refine ⟨rfl, ?_⟩
      simp only [hshared, Bool.false_eq_true, ↓reduceIte, List.nil_append]
      rfl
  | _ =>
    simp only
    rw [if_neg]
    intro hc
    have hfit := hc.2
    simp [nativePost, ShadowPost.fits] at hfit

/-! ## The annotation of any native call -/

/-- The identity trees of what a native saw of its arguments `args`, whose
own trees are `argIds`: a value argument's tree, or the tree at the location a
reference argument designates. -/
def seenTrees (h : Shadow) (s : State) (args : List Value) (argIds : List IdTree) :
    Option (List IdTree) :=
  (args.zip argIds).mapM fun (a, t) => match a with
    | .ref r => h.tree? s r.location
    | _ => some t

/-- The reference arguments, with their identities. -/
def refArgsOf (argIds : List IdTree) (args : List Value) : List (Nat × Reference) :=
  (argIds.zip args).filterMap fun (t, a) => match a with
    | .ref r => some (t.id, r)
    | _ => none

/-- The reference arguments a native writes back through. -/
def writtenOf (P : Program) (f : FunctionRef) (refArgs : List (Nat × Reference)) :
    List (Nat × Reference) :=
  refArgs.filter fun (_, r) => r.mutable && P.nativeShape? f == some .update

/-- The identity of node `q` of what the native saw of argument `k`. -/
def seenIdAt (seen : List IdTree) (k : Nat) (q : Path) : Option Nat :=
  (seen[k]?.bind (·.at? q)).map (·.id)

/-- The trees of a native's outputs, the fresh counter after them, and their
creation events. -/
def outTreesOf (φ : Nat → Path → Origin) (seen : List IdTree) (c : Creator) (outs : List Value)
    (fresh : Nat) : List IdTree × Nat × List Event :=
  outs.zipIdx.foldl (fun (ts, n, evs) (v, j) =>
      let idOf (p : Path) : Option Nat :=
        match φ j p with
        | .arg k q => seenIdAt seen k q
        | .fresh => none
      let (t, n, e) := IdTree.build idOf c [] v n
      (ts ++ [t], n, evs ++ e)) ([], fresh, [])

/-- The consumptions of a native call. -/
def consumedOfNative (P : Program) (f : FunctionRef) (m : ModuleId) (ptys : List RuntimeType)
    (seen : List IdTree) (views : List Value) (origins : List (Nat × Path)) : List Event :=
  (seen.zip views).zipIdx.flatMap fun ((t, a), k) =>
    if (ptys[k]?).any (P.nativeKeeps f) then [] else
      (a.nodes.zip (t.occurrences a)).filterMap fun ((q, w), (i, _)) =>
        if origins.contains (k, q) then none
        else some (Event.consumed i w (.native f m (keepsNothingInside origins k q)))

/-- The references among a native's results, read from its outcome's
operand stack. -/
def refResultsOf (tys : List RuntimeType) (results : List Value) : List Reference :=
  (tys.zip results).filterMap fun (t, v) => match v with
    | .ref r => if t.isValue then none else some r
    | _ => none

/-- The new references for a native's places, with fresh identities and the
identity of the reference argument each derives from. -/
def newRefsOf (refResults : List Reference) (places : List NativePlace) (argIds : List IdTree)
    (fresh : Nat) : Option (List (Nat × Reference × Nat)) :=
  (refResults.zip places).zipIdx.mapM fun ((r, p), i) => do
    pure (fresh + i, r, (← argIds[p.arg]?).id)

/-- The events of a native call. -/
def nativeEventsOf (refArgs written : List (Nat × Reference))
    (newRefs : List (Nat × Reference × Nat)) (consumed created : List Event) : List Event :=
  refArgs.map (fun (i, r) => Event.accessed .read r.location (some i)) ++
    written.map (fun (i, r) => Event.accessed .write r.location (some i)) ++
    refArgs.map (fun (i, r) => Event.used i r) ++
    newRefs.map (fun (i, r, parent) => Event.borrowed i r (some parent)) ++
    consumed ++ created

/-- **What a native call's annotation is**, when it exists, for any native:
its parts (`seenTrees`, `outTreesOf`, `consumedOfNative`, `newRefsOf`), the
shadow after it, with the results' trees on the caller's segment and the
written-back trees at the places the mutable arguments designate, and its
events (`nativeEventsOf`). -/
theorem nativeStep_spec {P : Program} {ν : NativeFlows} {s : State} {f : FunctionRef}
    {args views vals writes : List Value} {places : List NativePlace} {o : Outcome} {h : Shadow}
    {post : ShadowPost} {evs : List Event}
    (hn : Shadow.nativeStep P ν s f args views vals writes places o h = some (post, evs)) :
    ∃ g hs info tys ptys seen s' fr' tl newRefs resultTrees h₂,
      h.frames = g :: hs ∧ P.function? f = some info ∧ P.returnTypes f = some tys ∧
      P.paramTypes f = some ptys ∧ args.length ≤ g.stack.length ∧
      seenTrees h s args (g.stack.take args.length).reverse = some seen ∧
      seen.length = views.length ∧
      ((nodesOf (vals ++ writes)).all fun x =>
        match ν f views (vals ++ writes) x.1 x.2.1 with
        | .arg k q => (seenIdAt seen k q).isSome
        | .fresh => true) = true ∧
      (writtenOf P f (refArgsOf (g.stack.take args.length).reverse args)).length = writes.length ∧
      o = .running s' ∧ s'.frames = fr' :: tl ∧
      (refResultsOf tys (fr'.stack.take tys.length).reverse).length = places.length ∧
      newRefsOf (refResultsOf tys (fr'.stack.take tys.length).reverse) places
        (g.stack.take args.length).reverse
        (outTreesOf (ν f views (vals ++ writes)) seen (.native f info.moduleId views)
          (vals ++ writes) h.fresh).2.1 = some newRefs ∧
      nativeResultsIn tys
        ((outTreesOf (ν f views (vals ++ writes)) seen (.native f info.moduleId views)
          (vals ++ writes) h.fresh).1.take vals.length)
        (newRefs.map fun (i, _, _) => IdTree.node i []) = some resultTrees ∧
      ((writtenOf P f (refArgsOf (g.stack.take args.length).reverse args)).zip
        ((outTreesOf (ν f views (vals ++ writes)) seen (.native f info.moduleId views)
          (vals ++ writes) h.fresh).1.drop vals.length)).foldlM
        (fun h x => h.withTree? s x.1.2.location x.2)
        { h with
          frames := { g with stack := resultTrees.reverse ++ g.stack.drop args.length } :: hs
          fresh := (outTreesOf (ν f views (vals ++ writes)) seen (.native f info.moduleId views)
            (vals ++ writes) h.fresh).2.1 + newRefs.length } = some h₂ ∧
      post = .running h₂ ∧ ShadowPost.fits (.running h₂) o = true ∧
      evs = nativeEventsOf (refArgsOf (g.stack.take args.length).reverse args)
        (writtenOf P f (refArgsOf (g.stack.take args.length).reverse args)) newRefs
        (consumedOfNative P f info.moduleId ptys seen views
          (passedOn (ν f views (vals ++ writes)) (vals ++ writes)))
        (outTreesOf (ν f views (vals ++ writes)) seen (.native f info.moduleId views)
          (vals ++ writes) h.fresh).2.2 := by
  unfold Shadow.nativeStep at hn
  cases hh : h.frames with
  | nil => simp [hh] at hn
  | cons g hs =>
  simp only [hh, Option.bind_eq_bind, Option.bind_eq_some_iff] at hn
  obtain ⟨info, hinfo, hn⟩ := hn
  obtain ⟨tys, htys, hn⟩ := hn
  obtain ⟨_, hg1, hn⟩ := hn
  obtain ⟨seen, hseen, hn⟩ := hn
  obtain ⟨_, hg2, hn⟩ := hn
  obtain ⟨_, hg3, hn⟩ := hn
  obtain ⟨ptys, hptys, hn⟩ := hn
  obtain ⟨_, hg4, hn⟩ := hn
  cases o with
  | running s' =>
    cases hf' : s'.frames with
    | nil => simp [hf'] at hn
    | cons fr' tl =>
    simp only [hf', Option.bind_eq_some_iff] at hn
    obtain ⟨_, hg5, hn⟩ := hn
    obtain ⟨newRefs, hnew, hn⟩ := hn
    obtain ⟨resultTrees, hres, hn⟩ := hn
    obtain ⟨h₂, hh₂, hn⟩ := hn
    obtain ⟨_, hg6, hn⟩ := hn
    simp only [Option.pure_def, Option.some.injEq, Prod.mk.injEq] at hn
    obtain ⟨rfl, rfl⟩ := hn
    simp only [guard, Option.pure_def, failure] at hg1 hg2 hg3 hg4 hg5 hg6
    split at hg1 <;> split at hg2 <;> split at hg3 <;> split at hg4 <;> split at hg5 <;>
      split at hg6 <;> simp only [reduceCtorEq] at hg1 hg2 hg3 hg4 hg5 hg6
    rename_i hc1 hc2 hc3 hc4 hc5 hc6
    exact ⟨g, hs, info, tys, ptys, seen, s', fr', tl, newRefs, resultTrees, h₂, rfl, hinfo, htys,
      hptys, hc1, hseen, by simpa using hc2, hc3, beq_iff_eq.1 hc4, rfl, hf', beq_iff_eq.1 hc5,
      hnew, hres, hh₂, rfl, hc6, rfl⟩
  | _ => simp at hn

theorem flatMap_congr_mem {α β : Type} :
    ∀ {l : List α} {f g : α → List β}, (∀ x ∈ l, f x = g x) → l.flatMap f = l.flatMap g
  | [], _, _, _ => rfl
  | a :: l, f, g, h => by
    rw [List.flatMap_cons, List.flatMap_cons, h a (List.mem_cons_self ..),
      flatMap_congr_mem (fun x hx => h x (List.mem_cons_of_mem _ hx))]

theorem filterMap_congr_mem {α β : Type} :
    ∀ {l : List α} {f g : α → Option β}, (∀ x ∈ l, f x = g x) →
      l.filterMap f = l.filterMap g
  | [], _, _, _ => rfl
  | a :: l, f, g, h => by
    rw [List.filterMap_cons, List.filterMap_cons, h a (List.mem_cons_self ..),
      filterMap_congr_mem (fun x hx => h x (List.mem_cons_of_mem _ hx))]

/-- The result nodes a flow passes on have the identities, and the values,
of the argument nodes it names. -/
theorem passed_eq {φ : Nat → Path → Origin} {argIds : List IdTree} {args vals : List Value}
    (hguard : ∀ x ∈ nodesOf vals, ∀ k q, φ x.1 x.2.1 = .arg k q →
      (argIdAt argIds k q).isSome = true)
    (hhonest : ∀ x ∈ nodesOf vals, ∀ k q, φ x.1 x.2.1 = .arg k q →
      args[k]?.bind (·.at? q) = some x.2.2) :
    vals.zipIdx.flatMap (fun x => x.1.nodes.filterMap fun y =>
        (resultIdOf φ argIds x.2 y.1).map fun i => (i, y.2)) =
      (passedOn φ vals).map (argOcc argIds args) := by
  unfold passedOn
  rw [List.map_filterMap, nodesOf_eq, nodesOfFrom, List.filterMap_flatMap]
  refine flatMap_congr_mem fun x hx => ?_
  rw [List.filterMap_map]
  refine filterMap_congr_mem fun y hy => ?_
  have hmem : (x.2, y.1, y.2) ∈ nodesOf vals :=
    List.mem_flatMap.2 ⟨x, hx, List.mem_map_of_mem hy⟩
  simp only [Function.comp, resultIdOf]
  cases hφ : φ x.2 y.1 with
  | fresh => simp
  | arg k q =>
    have hs := hguard _ hmem k q hφ
    have hh := hhonest _ hmem k q hφ
    obtain ⟨i, hi⟩ := Option.isSome_iff_exists.1 hs
    simp only [hi, Option.map_some, argOcc]
    simp only [argIdAt] at hi
    simp [hi, hh]

theorem native_permits_respected {π : NativePolicy} {A : List StructId} {auth : List Address}
    {f : FunctionRef} {m : ModuleId} {args : List Value} {i : Nat} {w : Value}
    (h : (Creator.native f m args).Permits π [] [] w = true) :
    Encapsulation.structsRespected π A (.created i w (.native f m args)) = true ∧
      Encapsulation.signersRespected π auth (.created i w (.native f m args)) = true := by
  cases w <;> simp_all [Creator.Permits, Creator.MayCreateStruct, Creator.MayCreateSigner,
    Encapsulation.structsRespected, Encapsulation.signersRespected, Value.type]

/-- A native creates no closure (`Creator.MayCreateClosure`). -/
theorem native_closures {P : Program} {π : NativePolicy} {f : FunctionRef} {m : ModuleId}
    {args : List Value} {i : Nat} {w : Value}
    (h : (Creator.native f m args).Permits π [] [] w = true) :
    Encapsulation.closuresRespected P (.created i w (.native f m args)) = true := by
  cases w <;> simp_all [Creator.Permits, Creator.MayCreateClosure, Encapsulation.closuresRespected]

theorem consumer_permits {P : Program} {π : NativePolicy} {A : List StructId} {c : Consumer}
    {w : Value} {i : Nat} (h : c.Permits P π w = true) :
    ((w.type.abilities P).drop = true ∨ c.Legitimate π w = true) ∧
      Encapsulation.structsRespected π A (.consumed i w c) = true := by
  simp only [Consumer.Permits, Bool.and_eq_true, Bool.or_eq_true] at h
  exact ⟨h.1, h.2⟩

theorem mem_consumed_of {evs : List Event} {i : Nat} {w : Value} {c : Consumer}
    (h : Event.consumed i w c ∈ evs) : (i, w) ∈ consumedOf evs :=
  mem_consumedOf.2 ⟨c, h⟩

/-- **The annotation of a native call that returns**, under the native
contract: its guards hold and it establishes `FrameGhost` of the caller's
frame. -/
theorem native_ghost {P : Program} {π : NativePolicy} {A : List StructId} {auth : List Address}
    {ν : NativeFlows} {f : FunctionRef} {args vals : List Value} {info : FunctionInfo}
    {g : ShadowFrame} {fresh : Nat} {fr : Frame} {n : Nat} (hg : g.fits fr = true)
    (hn : n ≤ fr.stack.length) (hargs : args = (fr.stack.take n).reverse)
    (hresp : NativeCallRespects P π f info.moduleId args vals (ν f args vals) = true)
    (hns : ∀ k : Nat, ((P.paramTypes f).bind (·[k]?)).any (P.nativeKeeps f) = false)
    (hvals : ∀ v ∈ vals, shaped P v = true ∧ v.type.isValue = true) :
    args.length ≤ g.stack.length ∧
    ((nodesOf vals).all fun x =>
      match ν f args vals x.1 x.2.1 with
      | .arg k q => (argIdAt (g.stack.take args.length).reverse k q).isSome
      | .fresh => true) = true ∧
    FrameGhost σ P π A auth g fr fresh
      { g with stack := (nativeResults ν f args vals (g.stack.take args.length).reverse
        info.moduleId fresh).1.reverse ++ g.stack.drop args.length }
      { fr with stack := vals.reverse ++ fr.stack.drop n, pc := fr.pc + 1 }
      (nativeResults ν f args vals (g.stack.take args.length).reverse info.moduleId fresh).2.1
      (nativeEvents ν f args vals g info.moduleId fresh) := by
  -- The arguments and their shadows.
  have hfa := frame_stack_fits hg
  have hlen := fitsAll_length hfa
  obtain ⟨htake, hdropf⟩ := fitsAll_take_drop n hfa
  have hal : args.length = n := by rw [hargs]; simp; omega
  have hargfit : IdTree.fitsAll (g.stack.take args.length).reverse args = true := by
    rw [hal, hargs]; exact fitsAll_reverse htake
  generalize hφ : ν f args vals = φ at hresp
  generalize hids : (g.stack.take args.length).reverse = argIds at hargfit
  -- The native contract.
  simp only [NativeCallRespects, Bool.and_eq_true, List.all_eq_true] at hresp
  obtain ⟨H1, H2⟩ := hresp
  replace H2 : ∀ x ∈ nodesOf args, ((x.2.2.type.abilities P).copy ||
      decide ((passedOn φ vals).count (x.1, x.2.1) ≤ 1)) = true ∧
      ((passedOn φ vals).contains (x.1, x.2.1) ||
        (Consumer.native f info.moduleId (keepsNothingInside (passedOn φ vals) x.1 x.2.1)).Permits
          P π x.2.2) = true := by
    intro x hx
    have := H2 x hx
    obtain ⟨k, q, w⟩ := x
    simpa [hns k] using this
  have hhonest : ∀ x ∈ nodesOf vals, ∀ k q, φ x.1 x.2.1 = .arg k q →
      args[k]?.bind (·.at? q) = some x.2.2 := by
    intro x hx k q hxq
    have := H1 x hx
    obtain ⟨j, p, v⟩ := x
    simp only at hxq this ⊢
    rw [hxq] at this
    simpa using this
  have hcreate : ∀ x ∈ nodesOf vals, φ x.1 x.2.1 = .fresh →
      (Creator.native f info.moduleId args).Permits π [] [] x.2.2 = true := by
    intro x hx hxq
    have := H1 x hx
    obtain ⟨j, p, v⟩ := x
    simp only at hxq this ⊢
    rw [hxq] at this
    exact this
  have hguard' : ∀ x ∈ nodesOf vals, ∀ k q, φ x.1 x.2.1 = .arg k q →
      (argIdAt argIds k q).isSome = true := by
    intro x hx k q hxq
    have hh := hhonest x hx k q hxq
    cases ha : args[k]? with
    | none => rw [ha] at hh; simp at hh
    | some a =>
      rw [ha, Option.bind_some] at hh
      obtain ⟨t, ht, htf⟩ := fitsAll_getElem?_right hargfit ha
      obtain ⟨t', ht', -⟩ := at_fits q htf hh
      simp [argIdAt, ht, ht']
  have hguard : ((nodesOf vals).all fun x =>
      match φ x.1 x.2.1 with
      | .arg k q => (argIdAt argIds k q).isSome
      | .fresh => true) = true := by
    rw [List.all_eq_true]
    intro x hx
    cases hxq : φ x.1 x.2.1 with
    | arg k q => simpa using hguard' x hx k q hxq
    | fresh => rfl
  -- The results.
  obtain ⟨ts, fresh', evs, hfold, hfitR, hpermR, hcR, hbR, hndR, hleR, hevR⟩ :=
    results_fold (resultIdOf φ argIds) (.native f info.moduleId args) vals.zipIdx [] fresh []
  rw [List.zipIdx_map_fst] at hfitR hpermR
  have hR : nativeResults ν f args vals argIds info.moduleId fresh = (ts, fresh', evs) := by
    unfold nativeResults; rw [hφ, hfold]; simp
  have hevents : nativeEvents ν f args vals g info.moduleId fresh =
      consumedFrom (passedOn φ vals)
        (fun k q => .native f info.moduleId (keepsNothingInside (passedOn φ vals) k q)) 0
        argIds args ++ evs := by
    unfold nativeEvents; rw [hids, hR, hφ]
  have hnb : ∀ i r p, Event.borrowed i r p ∉ consumedFrom (passedOn φ vals)
      (fun k q => .native f info.moduleId (keepsNothingInside (passedOn φ vals) k q)) 0
      argIds args ++ evs := by
    intro i r p hb
    rcases List.mem_append.1 hb with hb | hb
    · obtain ⟨_, _, _, _, heq, -, -⟩ := consumedFrom_mem _ hb
      cases heq
    · obtain ⟨_, _, _, _, _, _, heq⟩ := hevR _ hb
      cases heq
  have hlenR := fitsAll_length hfitR
  have hfitPost : IdTree.fitsAll (ts.reverse ++ g.stack.drop n) (vals.reverse ++ fr.stack.drop n) =
      true := by
    rw [fitsAll_append (by simp [hlenR])]
    simp [fitsAll_reverse hfitR, hdropf]
  have hO : ∀ a ∈ passedOn φ vals, a ∈ (argPos 0 argIds args).map (·.1) := by
    intro a ha
    unfold passedOn at ha
    obtain ⟨x, hx, hxa⟩ := List.mem_filterMap.1 ha
    obtain ⟨j, p, v⟩ := x
    simp only at hxa
    split at hxa
    · next k q hxq =>
      simp only [Option.some.injEq] at hxa
      subst hxa
      have := argPos_mem (k := 0) hargfit (hhonest (j, p, v) hx k q hxq)
      simpa using this
    · simp at hxa
  rw [hevents, hR]
  dsimp only
  rw [hal]
  refine ⟨by omega, hguard, frame_fits_stack hg hfitPost _, ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩
  · -- The effect.
    refine effect_stack ?_
    rw [stack_split hfa n, occurrencesAll_append (by simp [hlenR])]
    refine Effect.append ?_ _
    have hcore : Effect P (IdTree.occurrencesAll argIds args) (IdTree.occurrencesAll ts vals)
        (consumedFrom (passedOn φ vals)
          (fun k q => .native f info.moduleId (keepsNothingInside (passedOn φ vals) k q)) 0
          argIds args ++ evs) fresh fresh' := by
      rw [← argPos_snd (k := 0) hargfit]
      refine native_effect (argPos 0 argIds args) (passedOn φ vals) (argOcc argIds args)
        (fun x hx => by simpa using argPos_occ (k := 0) hargfit x hx) (argPos_nodup hargfit)
        hO ?_ (consumedFrom_consumed hargfit) consumedFrom_created hcR hnb ?_ hbR hndR hleR
      · intro x hx
        have := H2 _ (argPos_nodes hargfit x hx)
        simp only [Bool.or_eq_true, decide_eq_true_eq] at this
        exact this.1
      · rw [← passed_eq hguard' hhonest]
        exact hpermR
    refine hcore.perm ?_ (occurrencesAll_reverse hfitR)
    rw [← hids, hal, hargs]
    exact (occurrencesAll_reverse htake).symm
  · -- Consumption is legitimate.
    intro i w c he
    rcases List.mem_append.1 he with he | he
    · obtain ⟨k', q, i', w', heq, hc, hm⟩ := consumedFrom_mem _ he
      simp only [Event.consumed.injEq] at heq
      obtain ⟨rfl, rfl, rfl⟩ := heq
      have := H2 _ hm
      simp only [Bool.or_eq_true] at this
      exact (consumer_permits (A := A) (i := i) (this.2.resolve_left (by simpa using hc))).1
    · have := mem_consumed_of he
      rw [hcR] at this
      simp at this
  · -- D3 for every event.
    intro e he
    rcases List.mem_append.1 he with he | he
    · obtain ⟨k', q, i', w', rfl, hc, hm⟩ := consumedFrom_mem _ he
      have := H2 _ hm
      simp only [Bool.or_eq_true] at this
      exact ⟨(consumer_permits (A := A) (i := i') (this.2.resolve_left (by simpa using hc))).2,
        rfl⟩
    · obtain ⟨x, hx, y, hy, hnone, i, rfl⟩ := hevR e he
      have hmem : (x.2, y.1, y.2) ∈ nodesOf vals :=
        List.mem_flatMap.2 ⟨x, hx, List.mem_map_of_mem hy⟩
      cases hxq : φ x.2 y.1 with
      | arg k q =>
        have := hguard' _ hmem k q hxq
        simp only [resultIdOf, hxq] at hnone
        rw [hnone] at this
        simp at this
      | fresh => exact native_permits_respected (hcreate _ hmem hxq)
  · -- The native's results hold no references.
    intro x hx
    rw [issuedOf_created hnb] at hx
    obtain ⟨o, ho, -, hok⟩ := mem_keys_iff.1 hx
    rw [createdOf_append, List.mem_append] at ho
    rcases ho with ho | ho
    · rw [consumedFrom_created] at ho
      simp at ho
    · obtain ⟨c, hc⟩ := mem_createdOf.1 ho
      obtain ⟨z, hz, y, hy, -, i, heq⟩ := hevR _ hc
      simp only [Event.created.injEq] at heq
      obtain ⟨-, hw, -⟩ := heq
      have hz1 : z.1 ∈ vals := by
        have := List.mem_map_of_mem (f := Prod.fst) hz
        rwa [List.zipIdx_map_fst] at this
      have hsub : o.2 ∈ subvalues z.1 := by
        rw [hw]; exact List.mem_map_of_mem (f := fun p => p.2) hy
      rw [← hok]
      exact container_ok (not_ref_of_isRef
        (shaped_noRef z.1 (hvals _ hz1).1 (hvals _ hz1).2 _ hsub))
  · -- Nor does it create one.
    intro i w c he
    have ho := (mem_createdOf (o := (i, w))).2 ⟨c, he⟩
    rw [createdOf_append, List.mem_append] at ho
    rcases ho with ho | ho
    · rw [consumedFrom_created] at ho
      simp at ho
    · obtain ⟨c, hc⟩ := mem_createdOf.1 ho
      obtain ⟨z, hz, y, hy, -, i, heq⟩ := hevR _ hc
      simp only [Event.created.injEq] at heq
      obtain ⟨-, hw, -⟩ := heq
      have hz1 : z.1 ∈ vals := by
        have := List.mem_map_of_mem (f := Prod.fst) hz
        rwa [List.zipIdx_map_fst] at this
      have hsub : w ∈ subvalues z.1 := by
        rw [hw]; exact List.mem_map_of_mem (f := fun p => p.2) hy
      exact shaped_noRef z.1 (hvals _ hz1).1 (hvals _ hz1).2 _ hsub
  · -- D3c: a native creates no closure.
    intro e he
    rcases List.mem_append.1 he with he | he
    · obtain ⟨k', q, i', w', rfl, -, -⟩ := consumedFrom_mem _ he
      rfl
    · obtain ⟨x, hx, y, hy, hnone, i, rfl⟩ := hevR e he
      have hmem : (x.2, y.1, y.2) ∈ nodesOf vals :=
        List.mem_flatMap.2 ⟨x, hx, List.mem_map_of_mem hy⟩
      cases hxq : φ x.2 y.1 with
      | arg k q =>
        have := hguard' _ hmem k q hxq
        simp only [resultIdOf, hxq] at hnone
        rw [hnone] at this
        simp at this
      | fresh => exact native_closures (hcreate _ hmem hxq)
  · -- The results' closures are the arguments' or created.
    refine kept_stack fun x hx hc => ?_
    rw [occurrencesAll_append (by simp [hlenR]), List.mem_append] at hx
    rcases hx with hx | hx
    · have hx' := (occurrencesAll_reverse hfitR).subset hx
      have hpR : (IdTree.occurrencesAll ts vals).Perm
          ((passedOn φ vals).map (argOcc argIds args) ++ createdOf evs) := by
        rw [← passed_eq hguard' hhonest]; exact hpermR
      rcases List.mem_append.1 (hpR.subset hx') with hx' | hx'
      · left
        obtain ⟨a, ha, rfl⟩ := List.mem_map.1 hx'
        obtain ⟨z, hz, rfl⟩ := List.mem_map.1 (hO a ha)
        have hzo : z.2 = argOcc argIds args z.1 := by
          simpa using argPos_occ (k := 0) hargfit z hz
        have hm : z.2 ∈ IdTree.occurrencesAll argIds args := by
          rw [← argPos_snd (k := 0) hargfit]; exact List.mem_map_of_mem hz
        rw [← hzo]
        rw [← hids, hal, hargs] at hm
        rw [stack_split hfa n]
        exact List.mem_append_left _ ((occurrencesAll_reverse htake).subset hm)
      · right
        obtain ⟨c, hc'⟩ := mem_createdOf.1 hx'
        exact ⟨c, List.mem_append_right _ hc'⟩
    · left
      rw [stack_split hfa n]
      exact List.mem_append_right _ hx

end MoveBytecode.Proofs
