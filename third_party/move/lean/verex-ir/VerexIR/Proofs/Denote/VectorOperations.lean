-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Term

/-!
# Certified vector operation normalization

Swapping and removing elements cannot exceed an already certified vector's
length. Rewrite their denotations to one bounds check and a certified result,
without re-checking representability or splitting redundant optional reads.
The map lemmas preserve generic caller/callee transport of these results.
A program's read of an element (`withVectorElem`) keeps the vector it reads
until the walk splits the read.
-/

namespace VerexIR.Proofs

/-- Split a dependent computation while retaining its branch evidence. -/
theorem wp_dite_rule {σ ε α : Type} (condition : Prop) [Decidable condition]
    (yes : condition → Spec σ ε α) (no : ¬condition → Spec σ ε α)
    (ensures : α → σ → Prop) (aborts : ε → Prop) (state : σ) :
    wp (if h : condition then yes h else no h) ensures aborts state ↔
      ((∀ h : condition, wp (yes h) ensures aborts state) ∧
        (∀ h : ¬condition, wp (no h) ensures aborts state)) := by
  by_cases h : condition <;> simp [h]
end VerexIR.Proofs

namespace VerexIR.Proofs.Denote
variable {unit : Validation.ValidatedUnit}

/-- Executable unsigned indices cannot take a negative-index branch. -/
@[vir_denote_norm] theorem unsigned_val_not_negative {width : Nat}
    (value : SpecInt (.bits width) false) : ¬value.val < 0 := by
  have := (SpecInt.unsigned_bounds value).1
  omega

/-- The read of a vector's element as a step of a program: the continuation
at the element the vector holds at a position, or the alternative where it
holds none there. It takes the vector and not its array, so the vector
reaches the split of this read as the program computed it, and the split can
state the element's fact from the vector's. The read itself states nothing of
the element: a vector whose fact is not known is read all the same. -/
def withVectorElem [Carriers] {σ ε β : Type} (τ : NTy) (values : (NTy.vector τ).carrier)
    (index : Nat) (found : τ.carrier → Spec σ ε β) (missing : Spec σ ε β) : Spec σ ε β :=
  match values.values[index]? with
  | some element => found element
  | none => missing

/-- The read as the match on the optional read of the vector's array, which
the walk splits by cases. -/
theorem withVectorElem_eq_match [Carriers] {σ ε β : Type} (τ : NTy)
    (values : (NTy.vector τ).carrier) (index : Nat) (found : τ.carrier → Spec σ ε β)
    (missing : Spec σ ε β) :
    withVectorElem τ values index found missing =
      match values.values[index]? with
      | some element => found element
      | none => missing := rfl

/-- A read the array decides takes its branch without a split. -/
theorem withVectorElem_of_some [Carriers] {σ ε β : Type} {τ : NTy}
    {values : (NTy.vector τ).carrier} {index : Nat} (found : τ.carrier → Spec σ ε β)
    (missing : Spec σ ε β) {element : τ.carrier} (read : values.values[index]? = some element) :
    withVectorElem τ values index found missing = found element := by
  rw [withVectorElem_eq_match, read]

theorem withVectorElem_of_none [Carriers] {σ ε β : Type} {τ : NTy}
    {values : (NTy.vector τ).carrier} {index : Nat} (found : τ.carrier → Spec σ ε β)
    (missing : Spec σ ε β) (read : values.values[index]? = none) :
    withVectorElem τ values index found missing = missing := by
  rw [withVectorElem_eq_match, read]

/-- The fact of an element a program read out of a vector, owed to the fact
of that vector: what the split of a read keeps of the read where the fact of
its vector is not there yet. It is evidence and no fact: it gives the
element's fact only to who has the vector's (`ElementFactOf.holds`), and it
is no implication a step could split or take a premise of. -/
def ElementFactOf [Carriers] (τ : NTy) (container : (NTy.vector τ).carrier)
    (element : τ.carrier) : Prop :=
  NTy.Holds (.vector τ) container → NTy.Holds τ element

/-- A read of a vector owes its element's fact to the vector's. -/
theorem ElementFactOf.of_read [Carriers] {τ : NTy} {container : (NTy.vector τ).carrier}
    {index : Nat} {element : τ.carrier} (read : container.values[index]? = some element) :
    ElementFactOf τ container element :=
  fun held => held.of_getElem? read

/-- The element's fact, once the vector's is there. -/
theorem ElementFactOf.holds [Carriers] {τ : NTy} {container : (NTy.vector τ).carrier}
    {element : τ.carrier} (owed : ElementFactOf τ container element)
    (held : NTy.Holds (.vector τ) container) : NTy.Holds τ element :=
  owed held

/-- An element read keeps its vector until the walk splits the read. The
index is rejected below zero, and a position the vector does not hold
aborts. -/
@[vir_denote ↓ high] theorem denote_index_direct [C : Carriers] {State : Type} {ρ : ResultShape}
    {Γ : NRow} {τ : NTy} {width : Nat} {signed : Bool} (site : Nat)
    (vector : Term unit ρ Γ (.vector τ)) (position : Term unit ρ Γ (.int width signed))
    (meanings : InterpreterOps unit C State) (env : HEnv Γ) :
    (Term.index site vector position).denoteIn meanings env =
      Flow.bind (vector.denoteIn meanings env) fun values env =>
        Flow.bind (position.denoteIn meanings env) fun i env =>
          opAt site (if i.val < 0 then abortAt site (Failure.ofInts .abort #[i.val])
          else withVectorElem τ values i.val.toNat
            (fun element => Spec.pure (.value element env))
            (abortAt site (Failure.ofInts .abort #[i.val]))) := by
  rw [Term.denoteIn]
  rfl

/-- Swapping preserves the representable length without a fresh runtime check. -/
def swapBounded {α : Type} (v : SpecVector α) (i j : Nat) : SpecVector α :=
  ⟨v.values.swapIfInBounds i j, by simpa only [Array.size_swapIfInBounds] using v.bounded⟩

@[vir_denote ↓ high] theorem denote_swap_direct [C : Carriers] {State : Type} {ρ : ResultShape} {Γ : NRow} {τ : NTy}
    {width : Nat} {signed : Bool} (site : Nat)
    (vector : Term unit ρ Γ (.vector τ)) (left right : Term unit ρ Γ (.int width signed))
    (meanings : InterpreterOps unit C State) (env : HEnv Γ) :
    (Term.swap site vector left right).denoteIn meanings env =
      Flow.bind (vector.denoteIn meanings env) fun values env =>
        Flow.bind (left.denoteIn meanings env) fun i env =>
          Flow.bind (right.denoteIn meanings env) fun j env =>
            if i.val < 0 ∨ j.val < 0 ∨ values.values.size ≤ i.val.toNat ∨
                values.values.size ≤ j.val.toNat then
              abortAt site (Failure.ofInts .abort #[i.val, j.val])
            else Spec.pure (.value (swapBounded values i.val.toNat j.val.toNat) env) := by
  rw [Term.denoteIn]
  congr 1
  funext values env
  congr 1
  funext i env
  congr 1
  funext j env
  by_cases h : i.val < 0 ∨ j.val < 0 ∨ values.values.size ≤ i.val.toNat ∨
      values.values.size ≤ j.val.toNat
  · simp only [if_pos h]
  · have hi : i.val.toNat < values.values.size := by omega
    have hj : j.val.toNat < values.values.size := by omega
    simp only [if_neg h, Array.getElem?_eq_getElem hi, Array.getElem?_eq_getElem hj]
    simp only [SpecVector.ofArray?, Array.size_set!, dif_pos values.bounded]
    simp only [swapBounded, Array.swapIfInBounds_def, Array.swap_def, dif_pos,
      Array.set!_eq_setIfInBounds, Array.setIfInBounds, hi, hj, Array.size_set]

@[vir_denote_norm] theorem swapBounded_values {α : Type} (v : SpecVector α) (i j : Nat) :
    (swapBounded v i j).values = v.values.swapIfInBounds i j := rfl

theorem NTy.Holds.swapBounded [Carriers] {τ : NTy} {vector : (NTy.vector τ).carrier}
    (held : NTy.Holds (.vector τ) vector) (i j : Nat) :
    NTy.Holds (.vector τ) (Denote.swapBounded vector i j) := by
  rw [NTy.holds_vector, swapBounded_values]
  intro _ member
  by_cases hi : i < vector.values.size
  · by_cases hj : j < vector.values.size
    · rw [Array.swapIfInBounds_def, dif_pos hi, dif_pos hj] at member
      obtain ⟨k, hk, rfl⟩ := Array.getElem_of_mem member
      rw [Array.getElem_swap]
      split
      · exact held.of_getElem j hj
      · split
        · exact held.of_getElem i hi
        · exact held.of_getElem k (by simpa only [Array.size_swap] using hk)
    · rw [Array.swapIfInBounds_def, dif_pos hi, dif_neg hj] at member
      exact held.of_mem member
  · rw [Array.swapIfInBounds_def, dif_neg hi] at member
    exact held.of_mem member

/-- Removing an element preserves representability, even for an empty vector. -/
def eraseBounded {α : Type} (v : SpecVector α) (i : Nat) : SpecVector α :=
  ⟨v.values.eraseIdxIfInBounds i, by
    have bounded := v.bounded
    rw [Array.eraseIdxIfInBounds_eq]
    split
    · simp only [Array.size_eraseIdx]; omega
    · exact bounded⟩

-- Keep the result carrier explicit: inferred partially expanded HLists
-- obstruct later normalization across generic caller/callee frames.
@[vir_denote ↓ high] theorem denote_remove_direct [C : Carriers] {State : Type} {ρ : ResultShape} {Γ : NRow} {τ : NTy}
    {width : Nat} {signed : Bool} (site : Nat)
    (vector : Term unit ρ Γ (.vector τ)) (position : Term unit ρ Γ (.int width signed))
    (meanings : InterpreterOps unit C State) (env : HEnv Γ) :
    (Term.remove site vector position).denoteIn meanings env =
      Flow.bind (β := NTy.carrier (.tuple (.cons τ (.cons (.vector τ) .nil))))
        (vector.denoteIn meanings env) fun values env =>
        Flow.bind (β := NTy.carrier (.tuple (.cons τ (.cons (.vector τ) .nil))))
          (position.denoteIn meanings env) fun i env =>
          opAt site (if i.val < 0 then
            (abortAt site (Failure.ofInts .abort #[i.val]) :
              Spec State Failure (Flow ρ Γ (NTy.carrier (.tuple (.cons τ (.cons (.vector τ) .nil))))))
          else withVectorElem τ values i.val.toNat
            (fun element => Spec.pure (@Flow.value C ρ Γ
                (NTy.carrier (.tuple (.cons τ (.cons (.vector τ) .nil))))
                ((element, (eraseBounded values i.val.toNat, ())) :
                  HList (.cons τ (.cons (.vector τ) .nil))) env))
            (abortAt site (Failure.ofInts .abort #[i.val]))) := by
  rw [Term.denoteIn]
  congr 1
  funext values env
  congr 1
  funext i env
  congr 1
  by_cases low : i.val < 0
  · simp only [if_pos low]
  · simp only [if_neg low, withVectorElem_eq_match]
    cases read : values.values[i.val.toNat]? with
    | none => rfl
    | some element =>
      have bounded := (eraseBounded values i.val.toNat).bounded
      change (values.values.eraseIdxIfInBounds i.val.toNat).size < 2 ^ 64 at bounded
      simp only [SpecVector.ofArray?, dif_pos bounded]
      rfl

@[vir_denote_norm] theorem eraseBounded_values {α : Type} (v : SpecVector α) (i : Nat) :
    (eraseBounded v i).values = v.values.eraseIdxIfInBounds i := rfl

theorem NTy.Holds.eraseBounded [Carriers] {τ : NTy} {vector : (NTy.vector τ).carrier}
    (held : NTy.Holds (.vector τ) vector) (i : Nat) :
    NTy.Holds (.vector τ) (Denote.eraseBounded vector i) := by
  rw [NTy.holds_vector, eraseBounded_values]
  exact held.of_eraseIdxIfInBounds i

/-- Inserting an element can exceed the representable length, so the result
carries the certificate the construction decided. -/
def insertBounded {α : Type} (v : SpecVector α) (i : Nat) (e : α)
    (bounded : (v.values.insertIdxIfInBounds i e).size < 2 ^ 64) : SpecVector α :=
  ⟨v.values.insertIdxIfInBounds i e, bounded⟩

-- Keep the result carrier explicit, as the removal does. The position is
-- rejected below zero and past the length, the length itself included, and
-- the representability of the result stays a branch of its own.
@[vir_denote ↓ high] theorem denote_insert_direct [C : Carriers] {State : Type} {ρ : ResultShape}
    {Γ : NRow} {τ : NTy} {width : Nat} {signed : Bool} (site : Nat)
    (vector : Term unit ρ Γ (.vector τ)) (position : Term unit ρ Γ (.int width signed))
    (element : Term unit ρ Γ τ)
    (meanings : InterpreterOps unit C State) (env : HEnv Γ) :
    (Term.insert site vector position element).denoteIn meanings env =
      Flow.bind (β := NTy.carrier (.vector τ)) (vector.denoteIn meanings env) fun values env =>
        Flow.bind (β := NTy.carrier (.vector τ)) (position.denoteIn meanings env) fun i env =>
          Flow.bind (β := NTy.carrier (.vector τ)) (element.denoteIn meanings env) fun e env =>
            opAt site (if i.val < 0 ∨ values.values.size < i.val.toNat then
              (abortAt site (Failure.ofInts .abort #[i.val]) :
                Spec State Failure (Flow ρ Γ (NTy.carrier (.vector τ))))
            else if bounded : (values.values.insertIdxIfInBounds i.val.toNat e).size < 2 ^ 64 then
              Spec.pure (@Flow.value C ρ Γ (NTy.carrier (.vector τ))
                (insertBounded values i.val.toNat e bounded) env)
            else Spec.bottom) := by
  rw [Term.denoteIn]
  congr 1
  funext values env
  congr 1
  funext i env
  congr 1
  funext e env
  congr 1
  by_cases outside : i.val < 0 ∨ values.values.size < i.val.toNat
  · simp only [if_pos outside]
  · simp only [if_neg outside, SpecVector.ofArray?]
    by_cases bounded : (values.values.insertIdxIfInBounds i.val.toNat e).size < 2 ^ 64
    · simp only [dif_pos bounded]
      rfl
    · simp only [dif_neg bounded]

@[vir_denote_norm] theorem insertBounded_values {α : Type} (v : SpecVector α) (i : Nat) (e : α)
    (bounded : (v.values.insertIdxIfInBounds i e).size < 2 ^ 64) :
    (insertBounded v i e bounded).values = v.values.insertIdxIfInBounds i e := rfl

theorem NTy.Holds.insertBounded [Carriers] {τ : NTy} {vector : (NTy.vector τ).carrier}
    {element : τ.carrier} (held : NTy.Holds (.vector τ) vector) (elementHeld : τ.Holds element)
    (i : Nat) (bounded : (vector.values.insertIdxIfInBounds i element).size < 2 ^ 64) :
    NTy.Holds (.vector τ) (Denote.insertBounded vector i element bounded) :=
  NTy.Holds.of_insert (index := i) held elementHeld (by
    simp only [SpecVector.ofArray?, dif_pos bounded]
    rfl)

-- Appending an element can exceed the representable length: there the
-- program has no result, and within it the result is the vector a
-- specification states of the append (`SpecVector.push`), so a program's
-- result and a clause's value are one term.
@[vir_denote ↓ high] theorem denote_push_direct [C : Carriers] {State : Type} {ρ : ResultShape}
    {Γ : NRow} {τ : NTy} (vector : Term unit ρ Γ (.vector τ)) (element : Term unit ρ Γ τ)
    (meanings : InterpreterOps unit C State) (env : HEnv Γ) :
    (Term.push vector element).denoteIn meanings env =
      Flow.bind (β := NTy.carrier (.vector τ)) (vector.denoteIn meanings env) fun values env =>
        Flow.bind (β := NTy.carrier (.vector τ)) (element.denoteIn meanings env) fun e env =>
          if values.values.size + 1 < 2 ^ 64 then
            Spec.pure (@Flow.value C ρ Γ (NTy.carrier (.vector τ)) (SpecVector.push values e) env)
          else Spec.bottom := by
  rw [Term.denoteIn]
  congr 1
  funext values env
  congr 1
  funext e env
  by_cases bounded : values.values.size + 1 < 2 ^ 64
  · have built : (values.values.push e).size < 2 ^ 64 := by
      simpa only [Array.size_push] using bounded
    simp only [if_pos bounded, SpecVector.ofArray?, dif_pos built, SpecVector.push, dif_pos bounded]
  · have full : ¬(values.values.push e).size < 2 ^ 64 := by
      simpa only [Array.size_push] using bounded
    simp only [if_neg bounded, SpecVector.ofArray?, dif_neg full]

/-- An append holds where the vector and the appended element do, also where
the vector is full and the append leaves it as it is. -/
theorem NTy.Holds.vector_push [Carriers] {τ : NTy} {vector : (NTy.vector τ).carrier}
    {element : τ.carrier} (held : NTy.Holds (.vector τ) vector) (elementHeld : τ.Holds element) :
    NTy.Holds (.vector τ) (SpecVector.push vector element) := by
  by_cases bounded : vector.values.size + 1 < 2 ^ 64
  · refine NTy.Holds.of_push (result := SpecVector.push vector element) held elementHeld ?_
    have built : (vector.values.push element).size < 2 ^ 64 := by
      simpa only [Array.size_push] using bounded
    simp only [SpecVector.ofArray?, dif_pos built, SpecVector.push, dif_pos bounded]
  · have same : SpecVector.push vector element = vector := by
      simp only [SpecVector.push, dif_neg bounded]
    rw [same]
    exact held

/-- Concatenating two vectors can exceed the representable length, so the
result carries the certificate the construction decided. -/
def concatBounded {α : Type} (left right : SpecVector α)
    (bounded : (left.values ++ right.values).size < 2 ^ 64) : SpecVector α :=
  ⟨left.values ++ right.values, bounded⟩

-- A concatenation keeps both of its vectors until its result is asked for
-- its fact. The representability of the result stays a branch of its own:
-- past the representable length the program has no result.
@[vir_denote ↓ high] theorem denote_concat_direct [C : Carriers] {State : Type} {ρ : ResultShape}
    {Γ : NRow} {τ : NTy} (left right : Term unit ρ Γ (.vector τ))
    (meanings : InterpreterOps unit C State) (env : HEnv Γ) :
    (Term.concat left right).denoteIn meanings env =
      Flow.bind (β := NTy.carrier (.vector τ)) (left.denoteIn meanings env) fun l env =>
        Flow.bind (β := NTy.carrier (.vector τ)) (right.denoteIn meanings env) fun r env =>
          if bounded : (l.values ++ r.values).size < 2 ^ 64 then
            Spec.pure (@Flow.value C ρ Γ (NTy.carrier (.vector τ)) (concatBounded l r bounded) env)
          else Spec.bottom := by
  rw [Term.denoteIn]
  congr 1
  funext l env
  congr 1
  funext r env
  by_cases bounded : (l.values ++ r.values).size < 2 ^ 64
  · simp only [SpecVector.ofArray?, dif_pos bounded]
    rfl
  · simp only [SpecVector.ofArray?, dif_neg bounded]

@[vir_denote_norm] theorem concatBounded_values {α : Type} (left right : SpecVector α)
    (bounded : (left.values ++ right.values).size < 2 ^ 64) :
    (concatBounded left right bounded).values = left.values ++ right.values := rfl

/-- A concatenation holds where both of its vectors do. -/
theorem NTy.Holds.concatBounded [Carriers] {τ : NTy} {left right : (NTy.vector τ).carrier}
    (leftHeld : NTy.Holds (.vector τ) left) (rightHeld : NTy.Holds (.vector τ) right)
    (bounded : (left.values ++ right.values).size < 2 ^ 64) :
    NTy.Holds (.vector τ) (Denote.concatBounded left right bounded) :=
  NTy.Holds.of_concat leftHeld rightHeld (by
    simp only [SpecVector.ofArray?, dif_pos bounded]
    rfl)

-- A slice within its range never exceeds the representable length: its result
-- is the vector a specification states of the range (`SpecVector.extract`),
-- which keeps the vector it is of. A range outside the vector aborts as before.
@[vir_denote ↓ high] theorem denote_slice_direct [C : Carriers] {State : Type} {ρ : ResultShape}
    {Γ : NRow} {τ : NTy} {width : Nat} {signed : Bool} (site : Nat)
    (vector : Term unit ρ Γ (.vector τ)) (start stop : Term unit ρ Γ (.int width signed))
    (meanings : InterpreterOps unit C State) (env : HEnv Γ) :
    (Term.slice site vector start stop).denoteIn meanings env =
      Flow.bind (vector.denoteIn meanings env) fun values env =>
        Flow.bind (start.denoteIn meanings env) fun a env =>
          Flow.bind (stop.denoteIn meanings env) fun b env =>
            if a.val < 0 ∨ b.val < a.val ∨ values.values.size < b.val.toNat then
              abortAt site (Failure.ofInts .abort #[a.val, b.val])
            else Spec.pure (.value (SpecVector.extract values a.val.toNat b.val.toNat) env) := by
  rw [Term.denoteIn]
  congr 1
  funext values env
  congr 1
  funext a env
  congr 1
  funext b env
  by_cases outside : a.val < 0 ∨ b.val < a.val ∨ values.values.size < b.val.toNat
  · simp only [if_pos outside]
  · have bounded := (SpecVector.extract values a.val.toNat b.val.toNat).bounded
    rw [SpecVector.values_extract] at bounded
    simp only [if_neg outside, SpecVector.ofArray?, dif_pos bounded]
    rfl

/-- Reversing a range preserves the representable length without a fresh
runtime check. -/
def reverseSliceBounded {α : Type} (v : SpecVector α) (left right : Nat) : SpecVector α :=
  ⟨reverseSlice v.values left right, by simpa only [size_reverseSlice] using v.bounded⟩

-- A range outside the vector aborts as before.
@[vir_denote ↓ high] theorem denote_reverseSlice_direct [C : Carriers] {State : Type}
    {ρ : ResultShape} {Γ : NRow} {τ : NTy} {width : Nat} {signed : Bool} (site : Nat)
    (vector : Term unit ρ Γ (.vector τ)) (start stop : Term unit ρ Γ (.int width signed))
    (meanings : InterpreterOps unit C State) (env : HEnv Γ) :
    (Term.reverseSlice site vector start stop).denoteIn meanings env =
      Flow.bind (vector.denoteIn meanings env) fun values env =>
        Flow.bind (start.denoteIn meanings env) fun a env =>
          Flow.bind (stop.denoteIn meanings env) fun b env =>
            if a.val < 0 ∨ b.val < a.val ∨ values.values.size < b.val.toNat then
              abortAt site (Failure.ofInts .abort #[a.val, b.val])
            else Spec.pure (.value (reverseSliceBounded values a.val.toNat b.val.toNat) env) := by
  rw [Term.denoteIn]
  congr 1
  funext values env
  congr 1
  funext a env
  congr 1
  funext b env
  by_cases outside : a.val < 0 ∨ b.val < a.val ∨ values.values.size < b.val.toNat
  · simp only [if_pos outside]
  · have bounded := (reverseSliceBounded values a.val.toNat b.val.toNat).bounded
    change (reverseSlice values.values a.val.toNat b.val.toNat).size < 2 ^ 64 at bounded
    simp only [if_neg outside, SpecVector.ofArray?, dif_pos bounded]
    rfl

@[vir_denote_norm] theorem reverseSliceBounded_values {α : Type} (v : SpecVector α)
    (left right : Nat) :
    (reverseSliceBounded v left right).values = reverseSlice v.values left right := rfl

theorem NTy.Holds.reverseSliceBounded [Carriers] {τ : NTy} {vector : (NTy.vector τ).carrier}
    (held : NTy.Holds (.vector τ) vector) (left right : Nat) :
    NTy.Holds (.vector τ) (Denote.reverseSliceBounded vector left right) :=
  NTy.Holds.reverseSlice (start := left) (stop := right) held (by
    have bounded := (Denote.reverseSliceBounded vector left right).bounded
    change (Denote.reverseSlice vector.values left right).size < 2 ^ 64 at bounded
    simp only [SpecVector.ofArray?, dif_pos bounded]
    rfl)

-- Keep the certified constructors folded; their projections have normalization rules.
-- The element read stays folded until its split or its decided branch.
attribute [irreducible] swapBounded eraseBounded insertBounded concatBounded reverseSliceBounded withVectorElem ElementFactOf
attribute [vir_denote_norm] Array.size_swapIfInBounds

/-- An optional read of a swapped array at one of the two positions the swap
moves, both within the array: the array before the swap, read at the other
one. As for the bounded read (`getElem_swapIfInBounds_left`), a position
the swap does not decide is no instance of these. -/
theorem getElem?_swapIfInBounds_left {α : Type} {xs : Array α} {i j : Nat} (hi : i < xs.size)
    (hj : j < xs.size) : (xs.swapIfInBounds i j)[i]? = xs[j]? := by
  have bound : i < (xs.swapIfInBounds i j).size := by
    simpa only [Array.size_swapIfInBounds] using hi
  rw [Array.getElem?_eq_getElem bound, getElem_swapIfInBounds_left hj bound,
    Array.getElem?_eq_getElem hj]

theorem getElem?_swapIfInBounds_right {α : Type} {xs : Array α} {i j : Nat} (hi : i < xs.size)
    (hj : j < xs.size) : (xs.swapIfInBounds i j)[j]? = xs[i]? := by
  have bound : j < (xs.swapIfInBounds i j).size := by
    simpa only [Array.size_swapIfInBounds] using hj
  rw [Array.getElem?_eq_getElem bound, getElem_swapIfInBounds_right hi bound,
    Array.getElem?_eq_getElem hi]

/-- An optional read of a swapped array at a position the swap leaves alone:
the array before the swap, read there. The swap moves the two positions it
names and nothing else, and it keeps the array's size, so the read needs no
bound of its own (as `Array.getElem?_setIfInBounds_ne` for a write). -/
theorem getElem?_swapIfInBounds_ne {α : Type} {xs : Array α} {i j k : Nat}
    (atFirst : k ≠ i) (atSecond : k ≠ j) : (xs.swapIfInBounds i j)[k]? = xs[k]? := by
  by_cases inside : k < xs.size
  · have moved : k < (xs.swapIfInBounds i j).size := by
      simpa only [Array.size_swapIfInBounds] using inside
    rw [Array.getElem?_eq_getElem moved, Array.getElem?_eq_getElem inside,
      Array.getElem_swapIfInBounds moved, dif_neg (fun both => atFirst both.1),
      dif_neg (fun both => atSecond both.1)]
  · have moved : ¬k < (xs.swapIfInBounds i j).size := by
      simpa only [Array.size_swapIfInBounds] using inside
    rw [Array.getElem?_eq_none (by omega), Array.getElem?_eq_none (by omega)]

open Lean Meta in
/-- A read whose array holds, in the normal form of its optional read, a
known element or none takes that branch, as a match on the read does. -/
simproc [vir_denote, vir_denote_norm] decidedVectorElem (@withVectorElem ?carriers _ _ _ _ _ _ _ _) := fun e => do
  unless e.isAppOfArity ``withVectorElem 9 do return .continue
  let arguments := e.getAppArgs
  let missed := mkAppN (mkConst ``withVectorElem_of_none) arguments
  let .forallE _ readsNone _ _ ← inferType missed | return .continue
  let some (_, read, _) := readsNone.eq? | return .continue
  let result ← Simp.simp read
  let value := result.expr.consumeMData
  if value.isAppOfArity ``Option.none 1 then
    return .visit { expr := arguments[8]!, proof? := some (mkApp missed (← result.getProof)) }
  if value.isAppOfArity ``Option.some 2 then
    let element := value.getArg! 1
    let taken := mkAppN (mkConst ``withVectorElem_of_some)
      (arguments ++ #[element, ← result.getProof])
    return .visit { expr := (mkApp arguments[7]! element).headBeta, proof? := some taken }
  return .continue

@[vir_denote_norm] theorem swapIfInBounds_map {α β : Type} (xs : Array α) (f : α → β) (i j : Nat) :
    (xs.map f).swapIfInBounds i j = (xs.swapIfInBounds i j).map f := by
  by_cases hi : i < xs.size <;> by_cases hj : j < xs.size
  all_goals simp [Array.swapIfInBounds_def, hi, hj, Array.swap_def]
private theorem list_map_erase {α β : Type} (xs : List α) (f : α → β) (i : Nat) :
    (xs.map f).eraseIdx i = (xs.eraseIdx i).map f := by
  induction xs generalizing i with
  | nil => simp
  | cons x xs ih => cases i <;> simp [ih]
@[vir_denote_norm] theorem eraseIdxIfInBounds_map {α β : Type} (xs : Array α) (f : α → β) (i : Nat) :
    (xs.map f).eraseIdxIfInBounds i = (xs.eraseIdxIfInBounds i).map f := by
  apply Array.toList_inj.mp
  simp only [Array.toList_eraseIdxIfInBounds, Array.toList_map]
  exact list_map_erase xs.toList f i

private theorem list_map_insertIdx {α β : Type} (xs : List α) (f : α → β) (i : Nat) (e : α) :
    (xs.map f).insertIdx i (f e) = (xs.insertIdx i e).map f := by
  induction xs generalizing i with
  | nil => cases i <;> simp
  | cons x xs ih => cases i <;> simp [ih]
@[vir_denote_norm] theorem insertIdxIfInBounds_map {α β : Type} (xs : Array α) (f : α → β)
    (i : Nat) (e : α) :
    (xs.map f).insertIdxIfInBounds i (f e) = (xs.insertIdxIfInBounds i e).map f := by
  by_cases within : i ≤ xs.size
  · simp only [Array.insertIdxIfInBounds, Array.size_map, dif_pos within]
    apply Array.toList_inj.mp
    simp only [Array.toList_insertIdx, Array.toList_map]
    exact list_map_insertIdx xs.toList f i e
  · simp only [Array.insertIdxIfInBounds, Array.size_map, dif_neg within]

@[vir_denote_norm] theorem findIndex?_getD_lt_iff {α : Type} (eq : α → α → Bool)
    (elements : Array α) (needle : α) (count index : Nat) :
    (findIndex? eq elements needle count index).getD 0 < elements.size ↔ 0 < elements.size := by
  constructor
  · intro h; omega
  · intro h
    cases found : findIndex? eq elements needle count index with
    | none => simpa only [Option.getD_none]
    | some position =>
      simpa only [Option.getD_some] using findIndex?_lt eq elements needle count index position found
@[vir_denote_norm] theorem size_le_findIndex?_getD_iff {α : Type} (eq : α → α → Bool)
    (elements : Array α) (needle : α) (count index : Nat) :
    elements.size ≤ (findIndex? eq elements needle count index).getD 0 ↔ elements.size = 0 := by
  have := findIndex?_getD_lt_iff eq elements needle count index
  omega
end VerexIR.Proofs.Denote
