-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.GhostLocations

/-!
# Steps that continue

UNTRUSTED (checked by the kernel). What a step of the deterministic core does
to the state when it continues (`running` or `returned`), for any operands:
the semantics' helpers either fail or go wrong, which ends the execution, or
succeed with the stack shape their instruction implies. The ghost layer's
annotation of a step (`Proofs.GhostSteps`) follows these shapes.

Nothing here uses the verifier: D2 and D3 are properties of the semantics,
the native contracts and the transaction premises.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Step

theorem bind_ok' {ε α β : Type} {x : Except ε α} {f : α → Except ε β} {b : β} :
    (x >>= f) = .ok b ↔ ∃ a, x = .ok a ∧ f a = .ok b := by
  cases x <;> simp [bind, Except.bind]

/-- A partial step whose error, if any, ends the execution. -/
def Ends {α : Type} : Exec α → Prop
  | .ok _ => True
  | .error o => o.continues = false

theorem ends_wrong {α : Type} (r : WrongReason) : Ends (wrong r : Exec α) := rfl
theorem ends_fail {α : Type} (f : Failure) : Ends (fail f : Exec α) := rfl

theorem ends_need {α : Type} (x : Option α) (r : WrongReason) : Ends (need x r) := by
  cases x <;> first | rfl | trivial

theorem ends_pure {α : Type} (a : α) : Ends (pure a : Exec α) := trivial

theorem ends_bind {α β : Type} {x : Exec α} {g : α → Exec β} (hx : Ends x)
    (hg : ∀ a, x = .ok a → Ends (g a)) : Ends (x >>= g) := by
  cases h : x with
  | error o => rw [h] at hx; exact hx
  | ok a => exact hg a h

/-! ## Helpers -/

variable {P : Program} {fr fr' : Frame}

theorem pop_ok {v : Value} (h : pop fr = .ok (v, fr')) :
    ∃ st, fr.stack = v :: st ∧ fr' = { fr with stack := st } := by
  unfold pop at h
  split at h
  · next v' st hst =>
    simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
    obtain ⟨rfl, rfl⟩ := h
    exact ⟨st, hst, rfl⟩
  · simp [wrong] at h

theorem pop_ends : Ends (pop fr) := by
  unfold pop; split <;> first | rfl | trivial

theorem popN_ok {n : Nat} {vals : List Value} (h : popN fr n = .ok (vals, fr')) :
    n ≤ fr.stack.length ∧ vals = (fr.stack.take n).reverse ∧
      fr' = { fr with stack := fr.stack.drop n } := by
  unfold popN at h
  split at h
  · simp [wrong] at h
  · next hn =>
    simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
    obtain ⟨rfl, rfl⟩ := h
    exact ⟨by omega, rfl, rfl⟩

theorem popN_ends {n : Nat} : Ends (popN fr n) := by
  unfold popN; split <;> first | rfl | trivial

theorem popBool_ok {b : Bool} (h : popBool fr = .ok (b, fr')) :
    ∃ st, fr.stack = .bool b :: st ∧ fr' = { fr with stack := st } := by
  unfold popBool at h
  rw [bind_ok'] at h
  obtain ⟨⟨v, fr₁⟩, hp, h⟩ := h
  obtain ⟨st, hst, rfl⟩ := pop_ok hp
  cases v <;> simp [pure, Except.pure, wrong] at h
  obtain ⟨rfl, rfl⟩ := h
  exact ⟨st, hst, rfl⟩

theorem popBool_ends : Ends (popBool fr) := by
  unfold popBool
  refine ends_bind pop_ends fun p _ => ?_
  rcases p with ⟨v, fr₁⟩
  cases v <;> first | rfl | trivial

theorem popInt_ok {t : IntType} {x : Int} (h : popInt fr = .ok (t, x, fr')) :
    ∃ st, fr.stack = .int t x :: st ∧ fr' = { fr with stack := st } := by
  unfold popInt at h
  rw [bind_ok'] at h
  obtain ⟨⟨v, fr₁⟩, hp, h⟩ := h
  obtain ⟨st, hst, rfl⟩ := pop_ok hp
  cases v <;> simp [pure, Except.pure, wrong] at h
  obtain ⟨rfl, rfl, rfl⟩ := h
  exact ⟨st, hst, rfl⟩

theorem popInt_ends : Ends (popInt fr) := by
  unfold popInt
  refine ends_bind pop_ends fun p _ => ?_
  rcases p with ⟨v, fr₁⟩
  cases v <;> first | rfl | trivial

/-- A boolean or an integer: what the arithmetic, comparison and boolean
instructions take. -/
def Primitive : Value → Bool
  | .bool _ | .int _ _ => true
  | .address _ | .signer _ | .vector _ _ | .struct _ _ | .variant _ _ _ | .ref _ => false
  | .closure .. | .table .. => false

/-- A step that pops `n` values and pushes `v` in their place, advancing. -/
def Replaces (fr fr' : Frame) (n : Nat) (v : Value) : Prop :=
  n ≤ fr.stack.length ∧ fr' = advance (push { fr with stack := fr.stack.drop n } v)

theorem intBinop_ok {allow : IntType → Bool} {op : IntType → Int → Int → Option Int}
    (h : intBinop fr allow op = .ok fr') :
    ∃ t r, Replaces fr fr' 2 (.int t r) ∧ ∀ w ∈ fr.stack.take 2, Primitive w = true := by
  unfold intBinop at h
  rw [bind_ok'] at h
  obtain ⟨⟨t', y, fr₁⟩, h1, h⟩ := h
  rw [bind_ok'] at h
  obtain ⟨⟨t, x, fr₂⟩, h2, h⟩ := h
  obtain ⟨st1, hst1, rfl⟩ := popInt_ok h1
  obtain ⟨st2, hst2, rfl⟩ := popInt_ok h2
  simp only at hst2
  subst hst2
  dsimp only at h
  split at h
  · simp [wrong] at h
  · split at h
    · next r _ =>
      simp only [pure, Except.pure, Except.ok.injEq] at h
      subst h
      exact ⟨t, r, ⟨by simp [hst1], by simp [hst1, advance, push]⟩,
        by simp [hst1, Primitive]⟩
    · simp [fail] at h

theorem intBinop_ends {allow : IntType → Bool} {op : IntType → Int → Int → Option Int} :
    Ends (intBinop fr allow op) := by
  unfold intBinop
  refine ends_bind popInt_ends fun p _ => ?_
  rcases p with ⟨_, _, _⟩
  refine ends_bind popInt_ends fun q _ => ?_
  rcases q with ⟨_, _, _⟩
  dsimp only
  split
  · rfl
  · split <;> first | rfl | trivial

theorem intCompare_ok {op : Int → Int → Bool} (h : intCompare fr op = .ok fr') :
    ∃ b, Replaces fr fr' 2 (.bool b) ∧ ∀ w ∈ fr.stack.take 2, Primitive w = true := by
  unfold intCompare at h
  rw [bind_ok'] at h
  obtain ⟨⟨t', y, fr₁⟩, h1, h⟩ := h
  rw [bind_ok'] at h
  obtain ⟨⟨t, x, fr₂⟩, h2, h⟩ := h
  obtain ⟨st1, hst1, rfl⟩ := popInt_ok h1
  obtain ⟨st2, hst2, rfl⟩ := popInt_ok h2
  simp only at hst2
  subst hst2
  dsimp only at h
  split at h
  · simp [wrong] at h
  · simp only [pure, Except.pure, Except.ok.injEq] at h
    subst h
    exact ⟨op x y, ⟨by simp [hst1], by simp [hst1, advance, push]⟩,
      by simp [hst1, Primitive]⟩

theorem intCompare_ends {op : Int → Int → Bool} : Ends (intCompare fr op) := by
  unfold intCompare
  refine ends_bind popInt_ends fun p _ => ?_
  rcases p with ⟨_, _, _⟩
  refine ends_bind popInt_ends fun q _ => ?_
  rcases q with ⟨_, _, _⟩
  dsimp only
  split <;> first | rfl | trivial

theorem boolBinop_ok {op : Bool → Bool → Bool} (h : boolBinop fr op = .ok fr') :
    ∃ b, Replaces fr fr' 2 (.bool b) ∧ ∀ w ∈ fr.stack.take 2, Primitive w = true := by
  unfold boolBinop at h
  rw [bind_ok'] at h
  obtain ⟨⟨b, fr₁⟩, h1, h⟩ := h
  rw [bind_ok'] at h
  obtain ⟨⟨a, fr₂⟩, h2, h⟩ := h
  obtain ⟨st1, hst1, rfl⟩ := popBool_ok h1
  obtain ⟨st2, hst2, rfl⟩ := popBool_ok h2
  simp only at hst2
  subst hst2
  dsimp only at h
  simp only [pure, Except.pure, Except.ok.injEq] at h
  subst h
  exact ⟨op a b, ⟨by simp [hst1], by simp [hst1, advance, push]⟩, by simp [hst1, Primitive]⟩

theorem boolBinop_ends {op : Bool → Bool → Bool} : Ends (boolBinop fr op) := by
  unfold boolBinop
  refine ends_bind popBool_ends fun p _ => ?_
  rcases p with ⟨_, _⟩
  refine ends_bind popBool_ends fun q _ => ?_
  rcases q with ⟨_, _⟩
  trivial

theorem shift_ok {op : IntType → Int → Nat → Option Int} (h : shift fr op = .ok fr') :
    ∃ t r, Replaces fr fr' 2 (.int t r) ∧ ∀ w ∈ fr.stack.take 2, Primitive w = true := by
  unfold shift at h
  rw [bind_ok'] at h
  obtain ⟨⟨ta, n, fr₁⟩, h1, h⟩ := h
  rw [bind_ok'] at h
  obtain ⟨⟨t, x, fr₂⟩, h2, h⟩ := h
  obtain ⟨st1, hst1, rfl⟩ := popInt_ok h1
  obtain ⟨st2, hst2, rfl⟩ := popInt_ok h2
  simp only at hst2
  subst hst2
  dsimp only at h
  split at h
  · simp [wrong] at h
  · split at h
    · next r _ =>
      simp only [pure, Except.pure, Except.ok.injEq] at h
      subst h
      exact ⟨t, r, ⟨by simp [hst1], by simp [hst1, advance, push]⟩,
        by simp [hst1, Primitive]⟩
    · simp [fail] at h

theorem shift_ends {op : IntType → Int → Nat → Option Int} : Ends (shift fr op) := by
  unfold shift
  refine ends_bind popInt_ends fun p _ => ?_
  rcases p with ⟨_, _, _⟩
  refine ends_bind popInt_ends fun q _ => ?_
  rcases q with ⟨_, _, _⟩
  dsimp only
  split
  · rfl
  · split <;> first | rfl | trivial

theorem cast_ok {target : IntType} (h : Step.cast fr target = .ok fr') :
    ∃ r, Replaces fr fr' 1 (.int target r) ∧ ∀ w ∈ fr.stack.take 1, Primitive w = true := by
  unfold Step.cast at h
  rw [bind_ok'] at h
  obtain ⟨⟨t, x, fr₁⟩, h1, h⟩ := h
  obtain ⟨st1, hst1, rfl⟩ := popInt_ok h1
  dsimp only at h
  split at h
  · next r _ =>
    simp only [pure, Except.pure, Except.ok.injEq] at h
    subst h
    exact ⟨r, ⟨by simp [hst1], by simp [hst1, advance, push]⟩, by simp [hst1, Primitive]⟩
  · simp [fail] at h

theorem cast_ends {target : IntType} : Ends (Step.cast fr target) := by
  unfold Step.cast
  refine ends_bind popInt_ends fun p _ => ?_
  rcases p with ⟨_, _, _⟩
  dsimp only
  split <;> first | rfl | trivial

theorem need_ok' {α : Type} {x : Option α} {r : WrongReason} {a : α} (h : need x r = .ok a) :
    x = some a := by
  cases x <;> simp_all [need, pure, Except.pure, wrong]

theorem deref_ok {s : State} {r : Reference} {v : Value} (h : deref s r = .ok v) :
    s.deref? r.location r.since = some v :=
  need_ok' h

theorem deref_ends {s : State} {r : Reference} : Ends (deref s r) := ends_need _ _

theorem equality_ok {s : State} {positive : Bool} (h : equality P s fr positive = .ok fr') :
    ∃ a b st, fr.stack = b :: a :: st ∧ (a.abilities P).drop = true ∧
      (b.abilities P).drop = true ∧ ∃ c, Replaces fr fr' 2 (.bool c) := by
  unfold equality at h
  rw [bind_ok'] at h
  obtain ⟨⟨b, fr₁⟩, h1, h⟩ := h
  rw [bind_ok'] at h
  obtain ⟨⟨a, fr₂⟩, h2, h⟩ := h
  obtain ⟨st1, hst1, rfl⟩ := pop_ok h1
  obtain ⟨st2, hst2, rfl⟩ := pop_ok h2
  simp only at hst2
  subst hst2
  dsimp only at h
  split at h
  · simp [wrong] at h
  · split at h
    · simp [wrong] at h
    · next hd =>
      simp only [Bool.or_eq_true, Bool.not_eq_true', not_or, Bool.not_eq_false] at hd
      refine ⟨a, b, st2, hst1, hd.1, hd.2, ?_⟩
      split at h
      · rw [bind_ok'] at h
        obtain ⟨x, -, h⟩ := h
        rw [bind_ok'] at h
        obtain ⟨y, -, h⟩ := h
        simp only [pure, Except.pure, Except.ok.injEq] at h
        subst h
        exact ⟨if positive then x == y else x != y, by simp [hst1], by simp [hst1, advance, push]⟩
      · simp only [pure, Except.pure, Except.ok.injEq] at h
        subst h
        exact ⟨if positive then a == b else a != b, by simp [hst1], by simp [hst1, advance, push]⟩

theorem equality_ends {s : State} {positive : Bool} : Ends (equality P s fr positive) := by
  unfold equality
  refine ends_bind pop_ends fun p _ => ?_
  rcases p with ⟨_, _⟩
  refine ends_bind pop_ends fun q _ => ?_
  rcases q with ⟨_, _⟩
  dsimp only
  split
  · rfl
  · split
    · rfl
    · split
      · exact ends_bind deref_ends fun _ _ => ends_bind deref_ends fun _ _ => trivial
      · trivial

theorem jump_ok {code : CodeUnit} {off : CodeOffset} (h : jump code fr off = .ok fr') :
    fr' = { fr with pc := off } := by
  unfold jump at h
  split at h
  · simp only [pure, Except.pure, Except.ok.injEq] at h
    exact h.symm
  · simp [wrong] at h

theorem jump_ends {code : CodeUnit} {off : CodeOffset} : Ends (jump code fr off) := by
  unfold jump; split <;> first | rfl | trivial

/-! ## References and global storage -/

theorem popRef_ok {r : Reference} (h : popRef fr = .ok (r, fr')) :
    ∃ st, fr.stack = .ref r :: st ∧ fr' = { fr with stack := st } := by
  unfold popRef at h
  rw [bind_ok'] at h
  obtain ⟨⟨v, fr₁⟩, hp, h⟩ := h
  obtain ⟨st, hst, rfl⟩ := pop_ok hp
  cases v <;> simp [pure, Except.pure, wrong] at h
  obtain ⟨rfl, rfl⟩ := h
  exact ⟨st, hst, rfl⟩

theorem popRef_ends : Ends (popRef fr) := by
  unfold popRef
  refine ends_bind pop_ends fun p _ => ?_
  rcases p with ⟨v, fr₁⟩
  cases v <;> first | rfl | trivial

theorem popRefTo_ok {t : RuntimeType} {mutable : Bool} {r : Reference}
    (h : popRefTo fr t mutable = .ok (r, fr')) :
    ∃ st, fr.stack = .ref r :: st ∧ fr' = { fr with stack := st } ∧ r.target = t ∧
      (mutable = true → r.mutable = true) := by
  unfold popRefTo at h
  rw [bind_ok'] at h
  obtain ⟨⟨r', fr₁⟩, hp, h⟩ := h
  obtain ⟨st, hst, rfl⟩ := popRef_ok hp
  dsimp only at h
  split at h
  · simp [wrong] at h
  · next ht =>
    split at h
    · simp [wrong] at h
    · next hm =>
      simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
      obtain ⟨rfl, rfl⟩ := h
      refine ⟨st, hst, rfl, by simpa using ht, fun hmut => ?_⟩
      simp only [hmut, Bool.true_and, Bool.not_eq_true'] at hm
      simpa using hm

theorem popRefTo_ends {t : RuntimeType} {mutable : Bool} : Ends (popRefTo fr t mutable) := by
  unfold popRefTo
  refine ends_bind popRef_ends fun p _ => ?_
  rcases p with ⟨_, _⟩
  dsimp only
  split
  · rfl
  · split <;> first | rfl | trivial

theorem popIndex_ok {i : Nat} (h : popIndex fr = .ok (i, fr')) :
    ∃ x st, fr.stack = .int .u64 x :: st ∧ fr' = { fr with stack := st } ∧ i = x.toNat := by
  unfold popIndex at h
  rw [bind_ok'] at h
  obtain ⟨⟨t, x, fr₁⟩, hp, h⟩ := h
  obtain ⟨st, hst, rfl⟩ := popInt_ok hp
  dsimp only at h
  split at h
  · simp [wrong] at h
  · next ht =>
    simp only [bne_iff_ne, ne_eq, Decidable.not_not] at ht
    subst ht
    simp only [pure, Except.pure, Except.ok.injEq, Prod.mk.injEq] at h
    obtain ⟨rfl, rfl⟩ := h
    exact ⟨x, st, hst, rfl, rfl⟩

theorem popIndex_ends : Ends (popIndex fr) := by
  unfold popIndex
  refine ends_bind popInt_ends fun p _ => ?_
  rcases p with ⟨_, _, _⟩
  dsimp only
  split <;> first | rfl | trivial

theorem popAddress_ok {a : Address} (h : popAddress fr = .ok (a, fr')) :
    ∃ st, fr.stack = .address a :: st ∧ fr' = { fr with stack := st } := by
  unfold popAddress at h
  rw [bind_ok'] at h
  obtain ⟨⟨v, fr₁⟩, hp, h⟩ := h
  obtain ⟨st, hst, rfl⟩ := pop_ok hp
  cases v <;> simp [pure, Except.pure, wrong] at h
  obtain ⟨rfl, rfl⟩ := h
  exact ⟨st, hst, rfl⟩

theorem popAddress_ends : Ends (popAddress fr) := by
  unfold popAddress
  refine ends_bind pop_ends fun p _ => ?_
  rcases p with ⟨v, fr₁⟩
  cases v <;> first | rfl | trivial

theorem derefVector_ok {s : State} {r : Reference} {elems : List Value}
    (h : derefVector s r = .ok elems) : ∃ t, s.deref? r.location r.since = some (.vector t elems) := by
  unfold derefVector at h
  rw [bind_ok'] at h
  obtain ⟨v, hv, h⟩ := h
  cases v <;> simp [pure, Except.pure, wrong] at h
  subst h
  exact ⟨_, deref_ok hv⟩

theorem derefVector_ends {s : State} {r : Reference} : Ends (derefVector s r) := by
  unfold derefVector
  refine ends_bind deref_ends fun v _ => ?_
  cases v <;> first | rfl | trivial

theorem update_ok {s s' : State} {l : Location} {v : Value} (h : update s l v = .ok s') :
    ∃ s₁, s.write? l v = some s₁ ∧ s' = s₁.invalidate (.mutated l) := by
  unfold update at h
  rw [bind_ok'] at h
  obtain ⟨s₁, h1, h⟩ := h
  simp only [pure, Except.pure, Except.ok.injEq] at h
  exact ⟨s₁, need_ok' h1, h.symm⟩

theorem update_ends {s : State} {l : Location} {v : Value} : Ends (update s l v) := by
  unfold update
  exact ends_bind (ends_need _ _) fun _ _ => trivial

theorem finish_ok {s : State} {vals : List Value} {r : StepResult} (h : finish s vals = .ok r) :
    ∃ fr rest, s.frames = fr :: rest ∧
      r = .done (.running { s with frames := advance (pushAll fr vals) :: rest }) := by
  unfold finish at h
  split at h
  · next fr rest hs =>
    simp only [pure, Except.pure, Except.ok.injEq] at h
    exact ⟨fr, rest, hs, h.symm⟩
  · simp [wrong] at h

theorem finish_ends {s : State} {vals : List Value} : Ends (finish s vals) := by
  unfold finish; split <;> first | rfl | trivial

theorem unlocked_ends {s : State} {sid : StructId} : Ends (unlocked s sid) := by
  unfold unlocked; split <;> first | rfl | trivial

theorem deref?_read {s : State} {l : Location} {since : Nat} {v : Value}
    (h : s.deref? l since = some v) : s.read? l = some v := by
  unfold State.deref? at h
  split at h
  · simp at h
  · exact h

end MoveBytecode.Proofs
