-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

/-! Small pass-independent inversion lemmas used by IR proofs. -/

namespace MoveModel.IR

/-- Invert an exception-map which pairs a successful value with fixed
metadata. -/
theorem Except.map_pair_ok_inv {ε α σ : Type} {x : Except ε α}
    {s s' : σ} {a : α}
    (h : x.map (·, s) = .ok (a, s')) : x = .ok a ∧ s' = s := by
  cases hx : x with
  | error e => rw [hx] at h; cases h
  | ok value =>
      rw [hx] at h
      simp only [Except.map, Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl⟩ := h
      exact ⟨rfl, rfl⟩

/-- Invert a successful exceptional bind into successful input and continuation. -/
theorem Except.bind_ok_inv {e α β : Type} {x : Except e α}
    {f : α → Except e β} {b : β} (h : x >>= f = .ok b) :
    ∃ a, x = .ok a ∧ f a = .ok b := by
  cases x with
  | error err => simp [Except.bind, bind] at h
  | ok a => exact ⟨a, rfl, by simpa [Except.bind, bind] using h⟩

/-- Successful exceptional mapping preserves corresponding indexed
elements. -/
theorem Except.mapM_getElem? {e α β : Type} {f : α → Except e β} :
    ∀ {xs : List α} {ys : List β}, xs.mapM f = .ok ys →
      ∀ {i : Nat} {x : α}, xs[i]? = some x →
        ∃ y, ys[i]? = some y ∧ f x = .ok y
  | [], _, h, i, x, hx => by simp at hx
  | _ :: _, _, h, 0, x, hx => by
      simp only [List.mapM_cons] at h
      obtain ⟨y, hy, h⟩ := Except.bind_ok_inv h
      obtain ⟨ys, -, h⟩ := Except.bind_ok_inv h
      simp only [pure, Except.pure, Except.ok.injEq] at h
      subst h
      simp at hx ⊢
      subst x
      exact hy
  | _ :: xs, _, h, i + 1, x, hx => by
      simp only [List.mapM_cons] at h
      obtain ⟨y, -, h⟩ := Except.bind_ok_inv h
      obtain ⟨ys, hys, h⟩ := Except.bind_ok_inv h
      simp only [pure, Except.pure, Except.ok.injEq] at h
      subst h
      exact Except.mapM_getElem? hys (by simpa using hx)

/-- Decompose `mapM` over `Option` on a cons cell. -/
theorem mapM_cons_eq_some {α β : Type} {f : α → Option β} {x : α}
    {xs : List α} {ys : List β} :
    (x :: xs).mapM f = some ys ↔
      ∃ y ys', f x = some y ∧ xs.mapM f = some ys' ∧ ys = y :: ys' := by
  simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff,
    Option.pure_def, Option.some.injEq]
  constructor
  · rintro ⟨y, hy, ys', hys, rfl⟩
    exact ⟨y, ys', hy, hys, rfl⟩
  · rintro ⟨y, ys', hy, hys, rfl⟩
    exact ⟨y, hy, ys', hys, rfl⟩

/-- Successful optional mapping preserves corresponding indexed elements. -/
theorem mapM_getElem? {α β : Type} {f : α → Option β} :
    ∀ {xs : List α} {ys : List β}, xs.mapM f = some ys →
      ∀ {i : Nat} {x : α}, xs[i]? = some x →
        ∃ y, ys[i]? = some y ∧ f x = some y
  | [], _, h, i, x, hx => by simp at hx
  | _ :: _, _, h, 0, x, hx => by
      rw [mapM_cons_eq_some] at h
      obtain ⟨y, ys, hy, -, rfl⟩ := h
      simp at hx ⊢
      subst x
      exact hy
  | _ :: xs, _, h, i + 1, x, hx => by
      rw [mapM_cons_eq_some] at h
      obtain ⟨y, ys, -, hys, rfl⟩ := h
      exact mapM_getElem? hys (by simpa using hx)

/-- Recover a source element corresponding to an element of a mapped result. -/
theorem getElem?_of_mapM {α β : Type} {f : α → Option β} :
    ∀ {xs : List α} {ys : List β}, xs.mapM f = some ys →
      ∀ {i : Nat} {y : β}, ys[i]? = some y →
        ∃ x, xs[i]? = some x ∧ f x = some y
  | [], ys, h, i, y, hy => by
      simp at h
      subst ys
      simp at hy
  | _ :: _, _, h, 0, y, hy => by
      rw [mapM_cons_eq_some] at h
      obtain ⟨y', ys, hx, -, rfl⟩ := h
      simp at hy ⊢
      subst y'
      exact hx
  | _ :: xs, _, h, i + 1, y, hy => by
      rw [mapM_cons_eq_some] at h
      obtain ⟨y', ys, -, hys, rfl⟩ := h
      exact getElem?_of_mapM hys (by simpa using hy)

/-- A member of the input or accumulator survives a deduplicating insertion fold. -/
theorem mem_foldl_dedupInsert {α : Type} [BEq α] [LawfulBEq α]
    {x : α} : ∀ {ns acc : List α},
    x ∈ ns.foldl (fun a n => if a.contains n then a else n :: a) acc ↔
      x ∈ ns ∨ x ∈ acc
  | [], acc => by simp
  | n :: ns, acc => by
      simp only [List.foldl_cons]
      split
      · next hn =>
          simp only [List.contains_iff_mem] at hn
          rw [mem_foldl_dedupInsert]
          simp only [List.mem_cons]
          constructor
          · rintro (h | h)
            · exact Or.inl (Or.inr h)
            · exact Or.inr h
          · rintro ((rfl | h) | h)
            · exact Or.inr hn
            · exact Or.inl h
            · exact Or.inr h
      · rw [mem_foldl_dedupInsert]
        simp only [List.mem_cons]
        constructor
        · rintro (h | rfl | h)
          · exact Or.inl (Or.inr h)
          · exact Or.inl (Or.inl rfl)
          · exact Or.inr h
        · rintro ((rfl | h) | h)
          · exact Or.inr (Or.inl rfl)
          · exact Or.inl h
          · exact Or.inr (Or.inr h)

end MoveModel.IR
