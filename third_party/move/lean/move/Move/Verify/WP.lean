-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Verify.Contract
import Move.Semantics.Vector

/-!
# Weakest-precondition rules for source primitives

The rules here expose a primitive's normal and abort behavior in one
obligation.  They keep symbolic execution at the `wp` level and prevent
clients from re-opening the relational `ok` and `aborts` fields separately.
-/

namespace Move.Verify

open Move.Semantics

/-- Checked addition in one obligation: the normal continuation under the
no-overflow hypothesis, and the arithmetic abort otherwise. -/
@[simp, wp_norm] theorem wp_addSpec {S W : Type} [Move.Sign S] [Move.Width W]
    (lhs rhs : Move.MoveInt S W)
    (ensures : Move.MoveInt S W → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Checked.addSpec lhs rhs : Spec State (Move.MoveInt S W)) ensures aborts initial ↔
      (Checked.inRange (Move.numTypeOf S W) (lhs.toInt + rhs.toInt) →
        ensures (Move.MoveInt.ofInt (lhs.toInt + rhs.toInt)) initial) ∧
      (¬ Checked.inRange (Move.numTypeOf S W) (lhs.toInt + rhs.toInt) → aborts Checked.arithmeticAbortCode) := by
  rw [wp_total_iff (by simp [Checked.addSpec])]
  constructor
  · rintro ⟨normal, abnormal⟩
    exact ⟨fun h => normal _ initial ⟨h, rfl, rfl⟩,
      fun h => abnormal _ ⟨rfl, h⟩⟩
  · rintro ⟨normal, abnormal⟩
    constructor
    · rintro value final ⟨hs, rfl, rfl⟩
      exact normal hs
    · rintro code ⟨rfl, h⟩
      exact abnormal h
/-- Certifying the global state: the invariant is the obligation asserted at
this point, and the continuation may then assume it. -/
@[simp, wp_norm] theorem wp_certifyState (invariant : State → Prop)
    (ensures : Unit → State → Prop) (aborts : Nat → Prop) (initial : State) :
    wp (Spec.certifyState invariant) ensures aborts initial ↔
      invariant initial ∧ (invariant initial → ensures () initial) := by
  constructor
  · rintro ⟨normal, -, defined⟩
    have holds : invariant initial := Classical.byContradiction defined
    exact ⟨holds, fun _ => normal () initial ⟨holds, rfl, rfl⟩⟩
  · rintro ⟨holds, normal⟩
    refine ⟨?_, fun _ absurd => absurd.elim, fun contra => contra holds⟩
    rintro result final ⟨_, rfl, rfl⟩
    exact normal holds

/-- Certifying a state update: the relation between the pre-state and the
post-state of `op` is conjoined onto the post, so it is asserted exactly where
`op`'s result lands (and never assumed on entry). -/
@[simp, wp_norm] theorem wp_certifyUpdate (relation : State → State → Prop)
    (op : Spec State Result) (ensures : Result → State → Prop)
    (aborts : Nat → Prop) (initial : State) :
    wp (Spec.certifyUpdate relation op) ensures aborts initial ↔
      wp op (fun result final => relation initial final ∧ ensures result final)
        aborts initial := by
  simp only [wp, Spec.certifyUpdate_ok, Spec.certifyUpdate_aborts,
    Spec.certifyUpdate_undefined]
  constructor
  · rintro ⟨hok, habt, hundef⟩
    have hrel : ∀ result final, op.ok initial result final → relation initial final :=
      fun result final hopok => Classical.byContradiction fun hcontra =>
        hundef (Or.inr ⟨result, final, hopok, hcontra⟩)
    exact ⟨fun result final hopok =>
      ⟨hrel result final hopok, hok result final ⟨hopok, hrel result final hopok⟩⟩,
      habt, fun h => hundef (Or.inl h)⟩
  · rintro ⟨hok, habt, hundef⟩
    refine ⟨fun result final h => (hok result final h.1).2, habt, ?_⟩
    rintro (h | ⟨result, final, hopok, hnrel⟩)
    · exact hundef h
    · exact hnrel (hok result final hopok).1

/-- A source conditional splits the obligation; the branch condition is
what a proof case-splits on. -/
@[wp_norm] theorem wp_ite (c : Prop) [Decidable c] (a b : Spec State Result)
    (ensures : Result → State → Prop) (aborts : Nat → Prop) (initial : State) :
    wp (if c then a else b) ensures aborts initial ↔
      if c then wp a ensures aborts initial else wp b ensures aborts initial := by
  split <;> rfl

@[wp_norm] theorem wp_dite (c : Prop) [Decidable c] (a : c → Spec State Result)
    (b : ¬c → Spec State Result)
    (ensures : Result → State → Prop) (aborts : Nat → Prop) (initial : State) :
    wp (if h : c then a h else b h) ensures aborts initial ↔
      if h : c then wp (a h) ensures aborts initial
      else wp (b h) ensures aborts initial := by
  split <;> rfl

/-- Creating a certified value: the invariant is the obligation, and the
continuation may use it. -/
@[simp, wp_norm] theorem wp_certified {Invariant : Prop}
    (build : Invariant → α) (ensures : α → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Spec.certified build : Spec State α) ensures aborts initial ↔
      Invariant ∧ ∀ holds : Invariant, ensures (build holds) initial := by
  simp only [wp, Spec.certified]
  constructor
  · rintro ⟨normal, -, holds⟩
    have holds : Invariant := Classical.byContradiction holds
    exact ⟨holds, fun holds => normal _ initial ⟨holds, rfl, rfl⟩⟩
  · rintro ⟨holds, normal⟩
    refine ⟨?_, ?_, fun refuted => refuted holds⟩
    · rintro result final ⟨actual, rfl, rfl⟩
      exact normal actual
    · exact fun code absurd => absurd.elim

/-- Checked subtraction in one obligation: the normal continuation under the
no-underflow hypothesis, and the arithmetic abort otherwise. -/
@[simp, wp_norm] theorem wp_subSpec {S W : Type} [Move.Sign S] [Move.Width W]
    (lhs rhs : Move.MoveInt S W)
    (ensures : Move.MoveInt S W → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Checked.subSpec lhs rhs : Spec State (Move.MoveInt S W)) ensures aborts initial ↔
      (Checked.inRange (Move.numTypeOf S W) (lhs.toInt - rhs.toInt) →
        ensures (Move.MoveInt.ofInt (lhs.toInt - rhs.toInt)) initial) ∧
      (¬ Checked.inRange (Move.numTypeOf S W) (lhs.toInt - rhs.toInt) → aborts Checked.arithmeticAbortCode) := by
  rw [wp_total_iff (by simp [Checked.subSpec])]
  constructor
  · rintro ⟨normal, abnormal⟩
    exact ⟨fun h => normal _ initial ⟨h, rfl, rfl⟩,
      fun h => abnormal _ ⟨rfl, h⟩⟩
  · rintro ⟨normal, abnormal⟩
    constructor
    · rintro value final ⟨hs, rfl, rfl⟩
      exact normal hs
    · rintro code ⟨rfl, h⟩
      exact abnormal h
/-- Checked multiplication in one obligation. -/
@[simp, wp_norm] theorem wp_mulSpec {S W : Type} [Move.Sign S] [Move.Width W]
    (lhs rhs : Move.MoveInt S W)
    (ensures : Move.MoveInt S W → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Checked.mulSpec lhs rhs : Spec State (Move.MoveInt S W)) ensures aborts initial ↔
      (Checked.inRange (Move.numTypeOf S W) (lhs.toInt * rhs.toInt) →
        ensures (Move.MoveInt.ofInt (lhs.toInt * rhs.toInt)) initial) ∧
      (¬ Checked.inRange (Move.numTypeOf S W) (lhs.toInt * rhs.toInt) → aborts Checked.arithmeticAbortCode) := by
  rw [wp_total_iff (by simp [Checked.mulSpec])]
  constructor
  · rintro ⟨normal, abnormal⟩
    exact ⟨fun h => normal _ initial ⟨h, rfl, rfl⟩,
      fun h => abnormal _ ⟨rfl, h⟩⟩
  · rintro ⟨normal, abnormal⟩
    constructor
    · rintro value final ⟨hs, rfl, rfl⟩
      exact normal hs
    · rintro code ⟨rfl, h⟩
      exact abnormal h
/-- Checked division in one obligation: the normal continuation under the
nonzero-divisor hypothesis, and the arithmetic abort otherwise. -/
@[simp, wp_norm] theorem wp_divSpec {S W : Type} [Move.Sign S] [Move.Width W]
    (lhs rhs : Move.MoveInt S W)
    (ensures : Move.MoveInt S W → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Checked.divSpec lhs rhs : Spec State (Move.MoveInt S W)) ensures aborts initial ↔
      ((rhs.toInt ≠ 0 ∧ Checked.inRange (Move.numTypeOf S W) (lhs.toInt.tdiv rhs.toInt)) →
        ensures (Move.MoveInt.ofInt (lhs.toInt.tdiv rhs.toInt)) initial) ∧
      ((rhs.toInt = 0 ∨ ¬ Checked.inRange (Move.numTypeOf S W) (lhs.toInt.tdiv rhs.toInt)) → aborts Checked.arithmeticAbortCode) := by
  rw [wp_total_iff (by simp [Checked.divSpec])]
  constructor
  · rintro ⟨normal, abnormal⟩
    exact ⟨fun h => normal _ initial ⟨h.1, h.2, rfl, rfl⟩,
      fun h => abnormal _ ⟨rfl, h⟩⟩
  · rintro ⟨normal, abnormal⟩
    constructor
    · rintro value final ⟨hz, hs, rfl, rfl⟩
      exact normal ⟨hz, hs⟩
    · rintro code ⟨rfl, h⟩
      exact abnormal h
/-- Checked left shift in one obligation: the normal continuation under the
in-range shift-amount hypothesis, and the arithmetic abort otherwise. -/
@[simp, wp_norm] theorem wp_shlSpec {S W : Type} [Move.Sign S] [Move.Width W] (lhs : Move.MoveInt S W)
    (amount : Move.UInt Move.W8)
    (ensures : Move.MoveInt S W → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Checked.shlSpec lhs amount : Spec State (Move.MoveInt S W))
        ensures aborts initial ↔
      (amount.toNat < (Move.widthOf W).bits → ensures (Move.MoveInt.shl lhs amount) initial) ∧
      (¬amount.toNat < (Move.widthOf W).bits → aborts Checked.arithmeticAbortCode) := by
  rw [wp_total_iff (by simp [Checked.shlSpec])]
  constructor
  · rintro ⟨normal, abnormal⟩
    exact ⟨fun safe => normal _ initial ⟨safe, rfl, rfl⟩,
      fun overflow => abnormal _ ⟨rfl, overflow⟩⟩
  · rintro ⟨normal, abnormal⟩
    constructor
    · rintro value final ⟨safe, rfl, rfl⟩
      exact normal safe
    · rintro code ⟨rfl, overflow⟩
      exact abnormal overflow

/-- Checked right shift in one obligation. -/
@[simp, wp_norm] theorem wp_shrSpec {S W : Type} [Move.Sign S] [Move.Width W] (lhs : Move.MoveInt S W)
    (amount : Move.UInt Move.W8)
    (ensures : Move.MoveInt S W → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Checked.shrSpec lhs amount : Spec State (Move.MoveInt S W))
        ensures aborts initial ↔
      (amount.toNat < (Move.widthOf W).bits → ensures (Move.MoveInt.shr lhs amount) initial) ∧
      (¬amount.toNat < (Move.widthOf W).bits → aborts Checked.arithmeticAbortCode) := by
  rw [wp_total_iff (by simp [Checked.shrSpec])]
  constructor
  · rintro ⟨normal, abnormal⟩
    exact ⟨fun safe => normal _ initial ⟨safe, rfl, rfl⟩,
      fun overflow => abnormal _ ⟨rfl, overflow⟩⟩
  · rintro ⟨normal, abnormal⟩
    constructor
    · rintro value final ⟨safe, rfl, rfl⟩
      exact normal safe
    · rintro code ⟨rfl, overflow⟩
      exact abnormal overflow

/-- Checked integer cast in one obligation: the normal continuation under the
fits-in-target hypothesis, and the arithmetic abort otherwise. -/
@[simp, wp_norm] theorem wp_castSpec {S W S' W' : Type} [Move.Sign S]
    [Move.Width W] [Move.Sign S'] [Move.Width W'] (value : Move.MoveInt S W)
    (ensures : Move.MoveInt S' W' → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Checked.castSpec value : Spec State (Move.MoveInt S' W'))
        ensures aborts initial ↔
      (Checked.inRange (Move.numTypeOf S' W') value.toInt →
        ensures (Move.MoveInt.cast value) initial) ∧
      (¬ Checked.inRange (Move.numTypeOf S' W') value.toInt →
        aborts Checked.arithmeticAbortCode) := by
  rw [wp_total_iff (by simp [Checked.castSpec])]
  constructor
  · rintro ⟨normal, abnormal⟩
    exact ⟨fun safe => normal _ initial ⟨safe, rfl, rfl⟩,
      fun overflow => abnormal _ ⟨rfl, overflow⟩⟩
  · rintro ⟨normal, abnormal⟩
    constructor
    · rintro result final ⟨safe, rfl, rfl⟩
      exact normal safe
    · rintro code ⟨rfl, overflow⟩
      exact abnormal overflow

/-- Checked remainder in one obligation. -/
@[simp, wp_norm] theorem wp_modSpec {S W : Type} [Move.Sign S] [Move.Width W]
    (lhs rhs : Move.MoveInt S W)
    (ensures : Move.MoveInt S W → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Checked.modSpec lhs rhs : Spec State (Move.MoveInt S W)) ensures aborts initial ↔
      (rhs.toInt ≠ 0 →
        ensures (Move.MoveInt.ofInt (lhs.toInt.tmod rhs.toInt)) initial) ∧
      (rhs.toInt = 0 → aborts Checked.arithmeticAbortCode) := by
  rw [wp_total_iff (by simp [Checked.modSpec])]
  constructor
  · rintro ⟨normal, abnormal⟩
    exact ⟨fun h => normal _ initial ⟨h, rfl, rfl⟩,
      fun h => abnormal _ ⟨rfl, h⟩⟩
  · rintro ⟨normal, abnormal⟩
    constructor
    · rintro value final ⟨hs, rfl, rfl⟩
      exact normal hs
    · rintro code ⟨rfl, h⟩
      exact abnormal h

/-! ## The unsigned view of the checked-arithmetic rules

The generic rules above state every obligation in the neutral `Int` domain,
because that is the domain the unified `Checked` specifications compute in.
Unsigned source code, however, is specified in `Nat` — `toNat`, `ofNat`, and a
`< size` bound.  Left to itself, every unsigned proof would first rewrite the
generic obligation and then translate it back, paying a second normalization
pass over the whole goal.

These rules state the unsigned obligation *directly* in that view, so it is
what the proof sees in the first place.  They are keyed on `MoveInt Unsigned W`,
a constant head, so `simp`'s discrimination tree picks the view in one step;
`simp high` puts them ahead of the generic rules, which then serve only the
signed view (whose native domain already is `Int`). -/

@[simp high, wp_norm high] theorem wp_addSpec_unsigned {W : Type} [Move.Width W]
    (lhs rhs : Move.UInt W)
    (ensures : Move.UInt W → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Checked.addSpec lhs rhs : Spec State (Move.UInt W)) ensures aborts initial ↔
      (lhs.toNat + rhs.toNat < (Move.widthOf W).size →
        ensures (Move.UInt.ofNat (lhs.toNat + rhs.toNat)) initial) ∧
      (¬lhs.toNat + rhs.toNat < (Move.widthOf W).size →
        aborts Checked.arithmeticAbortCode) := by
  rw [wp_addSpec, Checked.inRange_add, Move.UInt.ofInt_add]

@[simp high, wp_norm high] theorem wp_mulSpec_unsigned {W : Type} [Move.Width W]
    (lhs rhs : Move.UInt W)
    (ensures : Move.UInt W → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Checked.mulSpec lhs rhs : Spec State (Move.UInt W)) ensures aborts initial ↔
      (lhs.toNat * rhs.toNat < (Move.widthOf W).size →
        ensures (Move.UInt.ofNat (lhs.toNat * rhs.toNat)) initial) ∧
      (¬lhs.toNat * rhs.toNat < (Move.widthOf W).size →
        aborts Checked.arithmeticAbortCode) := by
  rw [wp_mulSpec, Checked.inRange_mul, Move.UInt.ofInt_mul]

@[simp high, wp_norm high] theorem wp_subSpec_unsigned {W : Type} [Move.Width W]
    (lhs rhs : Move.UInt W)
    (ensures : Move.UInt W → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Checked.subSpec lhs rhs : Spec State (Move.UInt W)) ensures aborts initial ↔
      (rhs.toNat ≤ lhs.toNat →
        ensures (Move.UInt.ofNat (lhs.toNat - rhs.toNat)) initial) ∧
      (¬rhs.toNat ≤ lhs.toNat → aborts Checked.arithmeticAbortCode) := by
  rw [wp_subSpec, Checked.inRange_sub]
  refine and_congr ?_ Iff.rfl
  constructor <;> intro h safe
  · rw [← Move.UInt.ofInt_sub lhs rhs safe]; exact h safe
  · rw [Move.UInt.ofInt_sub lhs rhs safe]; exact h safe

@[simp high, wp_norm high] theorem wp_divSpec_unsigned {W : Type} [Move.Width W]
    (lhs rhs : Move.UInt W)
    (ensures : Move.UInt W → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Checked.divSpec lhs rhs : Spec State (Move.UInt W)) ensures aborts initial ↔
      (rhs.toNat ≠ 0 →
        ensures (Move.UInt.ofNat (lhs.toNat / rhs.toNat)) initial) ∧
      (rhs.toNat = 0 → aborts Checked.arithmeticAbortCode) := by
  have range := Checked.inRange_tdiv lhs rhs
  rw [wp_divSpec, Move.UInt.ofInt_tdiv]
  constructor
  · rintro ⟨normal, abnormal⟩
    refine ⟨fun nonzero => normal ⟨?_, range⟩, fun zero => abnormal (Or.inl ?_)⟩
    · rw [Checked.toInt_ne_zero_iff]; exact nonzero
    · rw [Checked.toInt_eq_zero_iff]; exact zero
  · rintro ⟨normal, abnormal⟩
    refine ⟨fun h => normal ?_, fun h => abnormal ?_⟩
    · rw [← Checked.toInt_ne_zero_iff]; exact h.1
    · rcases h with h | h
      · rw [← Checked.toInt_eq_zero_iff]; exact h
      · exact absurd range h

@[simp high, wp_norm high] theorem wp_modSpec_unsigned {W : Type} [Move.Width W]
    (lhs rhs : Move.UInt W)
    (ensures : Move.UInt W → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Checked.modSpec lhs rhs : Spec State (Move.UInt W)) ensures aborts initial ↔
      (rhs.toNat ≠ 0 →
        ensures (Move.UInt.ofNat (lhs.toNat % rhs.toNat)) initial) ∧
      (rhs.toNat = 0 → aborts Checked.arithmeticAbortCode) := by
  rw [wp_modSpec, Move.UInt.ofInt_tmod, Checked.toInt_ne_zero_iff,
    Checked.toInt_eq_zero_iff]

@[simp high, wp_norm high] theorem wp_castSpec_unsigned {W W' : Type} [Move.Width W]
    [Move.Width W'] (value : Move.UInt W)
    (ensures : Move.UInt W' → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Checked.castSpec value : Spec State (Move.UInt W'))
        ensures aborts initial ↔
      (value.toNat < (Move.widthOf W').size →
        ensures (Move.MoveInt.cast value) initial) ∧
      (¬value.toNat < (Move.widthOf W').size →
        aborts Checked.arithmeticAbortCode) := by
  rw [wp_castSpec, Checked.inRange_cast]

@[simp, wp_norm] theorem wp_borrowElemSpec (values : Move.Vector α)
    (index : Move.U64) (ensures : α → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Vector.borrowElemSpec (σ := State) values index) ensures aborts initial ↔
      (∀ value, values.toList[index.toNat]? = some value → ensures value initial) ∧
      (values.toList[index.toNat]? = none → aborts Vector.indexOutOfBounds) := by
  rw [wp_total_iff (by simp [Vector.borrowElemSpec])]
  constructor
  · rintro ⟨normal, abnormal⟩
    constructor
    · intro value present
      apply normal value initial
      change values.toList[index.toNat]? = some value ∧ initial = initial
      exact ⟨present, rfl⟩
    · intro missing
      apply abnormal Vector.indexOutOfBounds
      change values.toList[index.toNat]? = none ∧
        Vector.indexOutOfBounds = Vector.indexOutOfBounds
      exact ⟨missing, rfl⟩
  · rintro ⟨normal, abnormal⟩
    constructor
    · rintro value final ⟨present, finalEq⟩
      subst final
      exact normal value present
    · rintro code ⟨missing, codeEq⟩
      subst code
      exact abnormal missing

@[simp, wp_norm] theorem wp_setSpec (values : Move.Vector α)
    (index : Move.U64) (value : α)
    (ensures : Move.Vector α → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Vector.setSpec (σ := State) values index value) ensures aborts initial ↔
      ((∃ old, values.toList[index.toNat]? = some old) →
        ensures (Move.Vector.set values index value) initial) ∧
      (values.toList[index.toNat]? = none → aborts Vector.indexOutOfBounds) := by
  rw [wp_total_iff (by simp [Vector.setSpec])]
  constructor
  · rintro ⟨normal, abnormal⟩
    constructor
    · intro present
      apply normal (Move.Vector.set values index value) initial
      change (∃ old, values.toList[index.toNat]? = some old) ∧
        Move.Vector.set values index value = Move.Vector.set values index value ∧
        initial = initial
      exact ⟨present, rfl, rfl⟩
    · intro missing
      apply abnormal Vector.indexOutOfBounds
      change values.toList[index.toNat]? = none ∧
        Vector.indexOutOfBounds = Vector.indexOutOfBounds
      exact ⟨missing, rfl⟩
  · rintro ⟨normal, abnormal⟩
    constructor
    · rintro result final ⟨present, resultEq, finalEq⟩
      subst result
      subst final
      exact normal present
    · rintro code ⟨missing, codeEq⟩
      subst code
      exact abnormal missing

/-- Opening a mutation scope means proving the body at every possible
prophecy, with the body result reconciled to that prophecy on normal return.
The body itself stays abstract, so callers do not destructure the scope's
relational representation. -/
@[wp_norm] theorem wp_withMutation (owner : α)
    (body : Mutation α → Spec State (β × Mutation α))
    (ensures : (β × α) → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (withMutation owner body) ensures aborts initial ↔
      ∀ future,
        wp (body { current := owner, prophecy := future })
          (fun output final =>
            output.2.current = future →
            ensures (output.1, future) final)
          aborts initial := by
  constructor
  · rintro ⟨normal, abnormal, defined⟩ future
    refine ⟨?_, ?_, ?_⟩
    · intro output final execution current
      apply normal (output.1, future) final
      exact ⟨future, output.2, execution, current, rfl⟩
    · intro code execution
      exact abnormal code ⟨future, execution⟩
    · intro obligation
      exact defined ⟨future, obligation⟩
  · intro bodyWP
    refine ⟨?_, ?_, ?_⟩
    · rintro ⟨result, finalOwner⟩ final
        ⟨future, reference, execution, current, ownerEq⟩
      change finalOwner = future at ownerEq
      subst finalOwner
      exact (bodyWP future).1 (result, reference) final execution current
    · rintro code ⟨future, execution⟩
      exact (bodyWP future).2.1 code execution
    · rintro ⟨future, obligation⟩
      exact (bodyWP future).2.2 obligation

@[wp_norm] theorem wp_withMutations2 (first : α) (second : β)
    (body : Mutation α → Mutation β →
      Spec State (γ × (Mutation α × Mutation β)))
    (ensures : (γ × (α × β)) → State → Prop)
    (aborts : Nat → Prop) (initial : State) :
    wp (withMutations2 first second body) ensures aborts initial ↔
      ∀ firstFuture secondFuture,
        wp (body { current := first, prophecy := firstFuture }
          { current := second, prophecy := secondFuture })
          (fun output final =>
            output.2.1.current = firstFuture →
            output.2.2.current = secondFuture →
            ensures (output.1, (firstFuture, secondFuture)) final)
          aborts initial := by
  constructor
  · rintro ⟨normal, abnormal, defined⟩ firstFuture secondFuture
    refine ⟨?_, ?_, ?_⟩
    · intro output final execution firstCurrent secondCurrent
      apply normal (output.1, (firstFuture, secondFuture)) final
      exact ⟨firstFuture, secondFuture, output.2.1, output.2.2,
        execution, firstCurrent, secondCurrent, rfl⟩
    · intro code execution
      exact abnormal code ⟨firstFuture, secondFuture, execution⟩
    · intro obligation
      exact defined ⟨firstFuture, secondFuture, obligation⟩
  · intro bodyWP
    refine ⟨?_, ?_, ?_⟩
    · rintro ⟨result, finalFirst, finalSecond⟩ final
        ⟨firstFuture, secondFuture, firstReference, secondReference,
          execution, firstCurrent, secondCurrent, finals⟩
      change (finalFirst, finalSecond) = (firstFuture, secondFuture) at finals
      cases finals
      exact (bodyWP finalFirst finalSecond).1
        (result, (firstReference, secondReference)) final execution
        firstCurrent secondCurrent
    · rintro code ⟨firstFuture, secondFuture, execution⟩
      exact (bodyWP firstFuture secondFuture).2.1 code execution
    · rintro ⟨firstFuture, secondFuture, obligation⟩
      exact (bodyWP firstFuture secondFuture).2.2 obligation

@[simp, wp_norm] theorem wp_withBorrowElemMutSpec (values : Move.Vector α)
    (index : Move.U64) (body : Mutation α → Spec State (β × Mutation α))
    (ensures : (β × Move.Vector α) → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Vector.withBorrowElemMutSpec values index body) ensures aborts initial ↔
      match values.toList[index.toNat]? with
      | none => aborts Vector.indexOutOfBounds
      | some value =>
          wp (withMutation value body)
            (fun output final =>
              ensures (output.1, Move.Vector.set values index output.2) final)
            aborts initial := by
  cases present : values.toList[index.toNat]? with
  | none =>
      simp only [Vector.withBorrowElemMutSpec, present]
      exact wp_abort _ _ _ _
  | some borrowed =>
      simp only [Vector.withBorrowElemMutSpec, present]
      change wp
        (Spec.bind (withMutation borrowed body) fun output =>
          Spec.pure (output.1, Move.Vector.set values index output.2))
        ensures aborts initial ↔ _
      rw [wp_bind]
      simp only [wp_pure]

@[simp, wp_norm] theorem wp_insertSpec (reference : Mutation (Move.Vector α))
    (index : Move.U64) (value : α)
    (ensures : (Unit × Mutation (Move.Vector α)) → State → Prop)
    (aborts : Nat → Prop) (initial : State) :
    wp (Vector.insertSpec reference index value) ensures aborts initial ↔
      if room : index.toNat ≤ reference.current.toList.length ∧
          reference.current.toList.length + 1 < Move.U64.size then
        ensures
          ((), reference.write (Move.Vector.ofList
            (reference.current.toList.take index.toNat ++
              value :: reference.current.toList.drop index.toNat)
            (by
              have inBounds := room.1
              have hasRoom : reference.current.toList.length + 1 <
                  MoveModel.IR.IntWidth.size .w64 := room.2
              simp only [List.length_append, List.length_cons,
                List.length_take, List.length_drop]
              omega)))
          initial
      else
        aborts Vector.indexOutOfBounds := by
  by_cases room : index.toNat ≤ reference.current.toList.length ∧
      reference.current.toList.length + 1 < Move.U64.size
  · simp only [Vector.insertSpec, Mutation.read]
    rw [dif_pos room, dif_pos room]
    exact wp_pure _ _ _ _
  · simp only [Vector.insertSpec, Mutation.read]
    rw [dif_neg room, dif_neg room]
    exact wp_abort _ _ _ _

@[simp, wp_norm] theorem wp_removeSpec (reference : Mutation (Move.Vector α))
    (index : Move.U64)
    (ensures : (α × Mutation (Move.Vector α)) → State → Prop)
    (aborts : Nat → Prop) (initial : State) :
    wp (Vector.removeSpec reference index) ensures aborts initial ↔
      match reference.current.toList[index.toNat]? with
      | none => aborts Vector.indexOutOfBounds
      | some removed =>
          ensures
            (removed, reference.write (Move.Vector.ofList
              (reference.current.toList.take index.toNat ++
                reference.current.toList.drop (index.toNat + 1))
              (by
                have bounded : reference.current.toList.length <
                    MoveModel.IR.IntWidth.size .w64 :=
                  reference.current.toList_length_lt
                simp only [List.length_append, List.length_take,
                  List.length_drop]
                omega)))
            initial := by
  cases present : reference.current.toList[index.toNat]? with
  | none =>
      simp only [Vector.removeSpec, Mutation.read, present]
      exact wp_abort _ _ _ _
  | some removed =>
      simp only [Vector.removeSpec, Mutation.read, present]
      exact wp_pure _ _ _ _

@[simp, wp_norm] theorem wp_popBackSpec (reference : Mutation (Move.Vector α))
    (ensures : (α × Mutation (Move.Vector α)) → State → Prop)
    (aborts : Nat → Prop) (initial : State) :
    wp (Vector.popBackSpec reference) ensures aborts initial ↔
      match reference.current.toList.getLast? with
      | none => aborts Vector.indexOutOfBounds
      | some removed =>
          ensures
            (removed, reference.write (Move.Vector.ofList
              reference.current.toList.dropLast (by
                have bounded := reference.current.toList_length_lt
                simpa only [List.length_dropLast] using
                  Nat.lt_of_le_of_lt
                    (Nat.sub_le reference.current.toList.length 1) bounded)))
            initial := by
  cases present : reference.current.toList.getLast? with
  | none =>
      simp only [Vector.popBackSpec, Mutation.read, present]
      exact wp_abort _ _ _ _
  | some removed =>
      simp only [Vector.popBackSpec, Mutation.read, present]
      exact wp_pure _ _ _ _

@[simp, wp_norm] theorem wp_swapSpec (reference : Mutation (Move.Vector α))
    (i j : Move.U64)
    (ensures : (Unit × Mutation (Move.Vector α)) → State → Prop)
    (aborts : Nat → Prop) (initial : State) :
    wp (Vector.swapSpec reference i j) ensures aborts initial ↔
      match reference.current.toList[i.toNat]?,
          reference.current.toList[j.toNat]? with
      | some vi, some vj =>
          ensures ((), reference.write (Move.Vector.set
            (Move.Vector.set reference.current i vj) j vi)) initial
      | _, _ => aborts Vector.indexOutOfBounds := by
  cases first : reference.current.toList[i.toNat]? <;>
    cases second : reference.current.toList[j.toNat]?
  <;> simp only [Vector.swapSpec, Mutation.read, first, second, wp_pure, wp_abort]

@[simp, wp_norm] theorem wp_swapRemoveSpec
    (reference : Mutation (Move.Vector α)) (index : Move.U64)
    (ensures : (α × Mutation (Move.Vector α)) → State → Prop)
    (aborts : Nat → Prop) (initial : State) :
    wp (Vector.swapRemoveSpec reference index) ensures aborts initial ↔
      match reference.current.toList[index.toNat]?,
          reference.current.toList.getLast? with
      | some removed, some last =>
          ensures
            (removed, reference.write (Move.Vector.ofList
              (reference.current.toList.set index.toNat last).dropLast
              (by
                change (reference.current.toList.set index.toNat last).dropLast.length <
                  Move.U64.size
                simp only [List.length_dropLast, List.length_set]
                exact Nat.lt_of_le_of_lt (Nat.sub_le _ _)
                  reference.current.toList_length_lt)))
            initial
      | _, _ => aborts Vector.indexOutOfBounds := by
  cases present : reference.current.toList[index.toNat]? <;>
    cases last : reference.current.toList.getLast?
  <;> simp only [Vector.swapRemoveSpec, Mutation.read, present, last,
    wp_pure, wp_abort]

@[simp, wp_norm] theorem wp_appendSpec (reference : Mutation (Move.Vector α))
    (other : Move.Vector α)
    (ensures : (Unit × Mutation (Move.Vector α)) → State → Prop)
    (aborts : Nat → Prop) (initial : State) :
    wp (Vector.appendSpec reference other) ensures aborts initial ↔
      if room : reference.current.toList.length + other.toList.length <
          Move.U64.size then
        ensures
          ((), reference.write (Move.Vector.ofList
            (reference.current.toList ++ other.toList) (by simpa using room)))
          initial
      else aborts Vector.indexOutOfBounds := by
  by_cases room : reference.current.toList.length + other.toList.length <
      Move.U64.size
  · simp only [Vector.appendSpec, Mutation.read]
    rw [dif_pos room, dif_pos room]
    exact wp_pure _ _ _ _
  · simp only [Vector.appendSpec, Mutation.read]
    rw [dif_neg room, dif_neg room]
    exact wp_abort _ _ _ _

@[simp, wp_norm] theorem wp_reverseSpec (reference : Mutation (Move.Vector α))
    (ensures : (Unit × Mutation (Move.Vector α)) → State → Prop)
    (aborts : Nat → Prop) (initial : State) :
    wp (Vector.reverseSpec reference) ensures aborts initial ↔
      ensures
        ((), reference.write (Move.Vector.ofList reference.current.toList.reverse
          (by simpa using reference.current.toList_length_lt))) initial := by
  simp only [Vector.reverseSpec, Mutation.read]
  exact wp_pure _ _ _ _

@[simp, wp_norm] theorem wp_reverseSliceSpec
    (reference : Mutation (Move.Vector α)) (left right : Move.U64)
    (ensures : (Unit × Mutation (Move.Vector α)) → State → Prop)
    (aborts : Nat → Prop) (initial : State) :
    wp (Vector.reverseSliceSpec reference left right) ensures aborts initial ↔
      if range : left.toNat ≤ right.toNat ∧
          right.toNat ≤ reference.current.toList.length then
        ensures
          ((), reference.write (Move.Vector.ofList
            (reference.current.toList.take left.toNat ++
              ((reference.current.toList.drop left.toNat).take
                (right.toNat - left.toNat)).reverse ++
              reference.current.toList.drop right.toNat)
            (by
              simp only [List.length_append, List.length_take,
                List.length_reverse, List.length_drop]
              rw [Nat.min_eq_left (Nat.le_trans range.1 range.2)]
              rw [Nat.min_eq_left (by omega : right.toNat - left.toNat ≤
                reference.current.toList.length - left.toNat)]
              have bounded := reference.current.toList_length_lt
              change reference.current.toList.length <
                MoveModel.IR.IntWidth.size .w64 at bounded
              omega))) initial
      else aborts 0x20001 := by
  by_cases range : left.toNat ≤ right.toNat ∧
      right.toNat ≤ reference.current.toList.length
  · simp only [Vector.reverseSliceSpec, Mutation.read]
    rw [dif_pos range, dif_pos range]
    exact wp_pure _ _ _ _
  · simp only [Vector.reverseSliceSpec, Mutation.read]
    rw [dif_neg range, dif_neg range]
    exact wp_abort _ _ _ _

@[simp, wp_norm] theorem wp_trimSpec (reference : Mutation (Move.Vector α))
    (newLen : Move.U64)
    (ensures : (Move.Vector α × Mutation (Move.Vector α)) → State → Prop)
    (aborts : Nat → Prop) (initial : State) :
    wp (Vector.trimSpec reference newLen) ensures aborts initial ↔
      if bound : newLen.toNat ≤ reference.current.toList.length then
        ensures
          (Move.Vector.ofList (reference.current.toList.drop newLen.toNat) (by
              simp only [List.length_drop]
              exact Nat.lt_of_le_of_lt (Nat.sub_le _ _)
                reference.current.toList_length_lt),
            reference.write (Move.Vector.ofList
              (reference.current.toList.take newLen.toNat)
              (Nat.lt_of_le_of_lt (List.length_take_le ..) newLen.toNat_lt)))
          initial
      else aborts Vector.indexOutOfBounds := by
  by_cases bound : newLen.toNat ≤ reference.current.toList.length
  · simp only [Vector.trimSpec, Mutation.read]
    rw [dif_pos bound, dif_pos bound]
    exact wp_pure _ _ _ _
  · simp only [Vector.trimSpec, Mutation.read]
    rw [dif_neg bound, dif_neg bound]
    exact wp_abort _ _ _ _

@[simp, wp_norm] theorem wp_trimReverseSpec
    (reference : Mutation (Move.Vector α)) (newLen : Move.U64)
    (ensures : (Move.Vector α × Mutation (Move.Vector α)) → State → Prop)
    (aborts : Nat → Prop) (initial : State) :
    wp (Vector.trimReverseSpec reference newLen) ensures aborts initial ↔
      if bound : newLen.toNat ≤ reference.current.toList.length then
        ensures
          (Move.Vector.ofList (reference.current.toList.drop newLen.toNat).reverse
              (by
                simp only [List.length_reverse]
                simp only [List.length_drop]
                exact Nat.lt_of_le_of_lt (Nat.sub_le _ _)
                  reference.current.toList_length_lt),
            reference.write (Move.Vector.ofList
              (reference.current.toList.take newLen.toNat)
              (Nat.lt_of_le_of_lt (List.length_take_le ..) newLen.toNat_lt)))
          initial
      else aborts Vector.indexOutOfBounds := by
  by_cases bound : newLen.toNat ≤ reference.current.toList.length
  · simp only [Vector.trimReverseSpec, Mutation.read]
    rw [dif_pos bound, dif_pos bound]
    exact wp_pure _ _ _ _
  · simp only [Vector.trimReverseSpec, Mutation.read]
    rw [dif_neg bound, dif_neg bound]
    exact wp_abort _ _ _ _

@[simp, wp_norm] theorem wp_rotateSliceSpec
    (reference : Mutation (Move.Vector α)) (left rot right : Move.U64)
    (ensures : (Move.U64 × Mutation (Move.Vector α)) → State → Prop)
    (aborts : Nat → Prop) (initial : State) :
    wp (Vector.rotateSliceSpec reference left rot right) ensures aborts initial ↔
      if range : (left.toNat ≤ rot.toNat ∧ rot.toNat ≤ right.toNat) ∧
          right.toNat ≤ reference.current.toList.length then
        ensures
          (Move.U64.ofNat (left.toNat + (right.toNat - rot.toNat)),
            reference.write (Move.Vector.ofList
              (Vector.rotateSliceValues reference.current.toList
                left.toNat rot.toNat right.toNat)
              (by
                simp only [Vector.rotateSliceValues, List.length_append,
                  List.length_take, List.length_drop]
                rw [Nat.min_eq_left (by omega : left.toNat ≤
                  reference.current.toList.length)]
                rw [Nat.min_eq_left (by omega : right.toNat - rot.toNat ≤
                  reference.current.toList.length - rot.toNat)]
                rw [Nat.min_eq_left (by omega : rot.toNat - left.toNat ≤
                  reference.current.toList.length - left.toNat)]
                have bounded := reference.current.toList_length_lt
                change reference.current.toList.length <
                  MoveModel.IR.IntWidth.size .w64 at bounded
                omega))) initial
      else aborts 0x20001 := by
  by_cases range : (left.toNat ≤ rot.toNat ∧ rot.toNat ≤ right.toNat) ∧
      right.toNat ≤ reference.current.toList.length
  · simp only [Vector.rotateSliceSpec, Mutation.read]
    rw [dif_pos range, dif_pos range]
    exact wp_pure _ _ _ _
  · simp only [Vector.rotateSliceSpec, Mutation.read]
    rw [dif_neg range, dif_neg range]
    exact wp_abort _ _ _ _

@[simp, wp_norm] theorem wp_rotateSpec
    (reference : Mutation (Move.Vector α)) (rot : Move.U64)
    (ensures : (Move.U64 × Mutation (Move.Vector α)) → State → Prop)
    (aborts : Nat → Prop) (initial : State) :
    wp (Vector.rotateSpec reference rot) ensures aborts initial ↔
      wp (Vector.rotateSliceSpec reference 0 rot
        (Move.Vector.length reference.current)) ensures aborts initial := by
  rfl

@[simp, wp_norm] theorem wp_destroyEmptySpec (values : Move.Vector α)
    (ensures : Unit → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Vector.destroyEmptySpec values) ensures aborts initial ↔
      if values.toList.isEmpty then ensures () initial
      else aborts Vector.indexOutOfBounds := by
  by_cases empty : values.toList.isEmpty
  · simp only [Vector.destroyEmptySpec]
    rw [if_pos empty, if_pos empty]
    exact wp_pure _ _ _ _
  · simp only [Vector.destroyEmptySpec]
    rw [if_neg empty, if_neg empty]
    exact wp_abort _ _ _ _

@[simp, wp_norm] theorem wp_containsSpec (resource : Resource World Key Value)
    (key : Key) (ensures : Bool → World → Prop) (aborts : Nat → Prop)
    (initial : World) :
    wp (resource.containsSpec key) ensures aborts initial ↔
      ensures (resource.lookup initial key).isSome initial := by
  rw [wp_total_iff (by simp [Resource.containsSpec])]
  constructor
  · rintro ⟨normal, _⟩
    apply normal _ initial
    exact ⟨rfl, rfl⟩
  · intro normal
    constructor
    · rintro result final ⟨resultEq, finalEq⟩
      subst result
      subst final
      exact normal
    · intro code impossible
      exact False.elim impossible

@[simp, wp_norm] theorem wp_borrowSpec (resource : Resource World Key Value)
    (key : Key) (ensures : Value → World → Prop) (aborts : Nat → Prop)
    (initial : World) :
    wp (resource.borrowSpec key) ensures aborts initial ↔
      (∀ value, resource.lookup initial key = some value → ensures value initial) ∧
      (resource.lookup initial key = none → aborts Resource.executionFailure) := by
  rw [wp_total_iff (by simp [Resource.borrowSpec])]
  constructor
  · rintro ⟨normal, abnormal⟩
    constructor
    · intro value present
      apply normal value initial
      exact ⟨present, rfl⟩
    · intro missing
      apply abnormal Resource.executionFailure
      exact ⟨missing, rfl⟩
  · rintro ⟨normal, abnormal⟩
    constructor
    · rintro value final ⟨present, finalEq⟩
      subst final
      exact normal value present
    · rintro code ⟨missing, codeEq⟩
      subst code
      exact abnormal missing

@[simp, wp_norm] theorem wp_moveFromSpec (resource : Resource World Key Value)
    (key : Key) (ensures : Value → World → Prop) (aborts : Nat → Prop)
    (initial : World) :
    wp (resource.moveFromSpec key) ensures aborts initial ↔
      (∀ value, resource.lookup initial key = some value →
        ensures value (resource.erase initial key)) ∧
      (resource.lookup initial key = none → aborts Resource.executionFailure) := by
  rw [wp_total_iff (by simp [Resource.moveFromSpec])]
  constructor
  · rintro ⟨normal, abnormal⟩
    constructor
    · intro value present
      apply normal value (resource.erase initial key)
      exact ⟨present, rfl⟩
    · intro missing
      apply abnormal Resource.executionFailure
      exact ⟨missing, rfl⟩
  · rintro ⟨normal, abnormal⟩
    constructor
    · rintro value final ⟨present, finalEq⟩
      subst final
      exact normal value present
    · rintro code ⟨missing, codeEq⟩
      subst code
      exact abnormal missing

@[simp, wp_norm] theorem wp_moveToSpec (resource : Resource World Key Value)
    (key : Key) (value : Value) (ensures : Unit → World → Prop)
    (aborts : Nat → Prop) (initial : World) :
    wp (resource.moveToSpec key value) ensures aborts initial ↔
      (resource.lookup initial key = none →
        ensures () (resource.insert initial key value)) ∧
      ((∃ old, resource.lookup initial key = some old) →
        aborts Resource.executionFailure) := by
  rw [wp_total_iff (by simp [Resource.moveToSpec])]
  constructor
  · rintro ⟨normal, abnormal⟩
    constructor
    · intro missing
      apply normal () (resource.insert initial key value)
      exact ⟨missing, rfl, rfl⟩
    · intro present
      apply abnormal Resource.executionFailure
      exact ⟨present, rfl⟩
  · rintro ⟨normal, abnormal⟩
    constructor
    · rintro result final ⟨missing, resultEq, finalEq⟩
      subst result
      subst final
      exact normal missing
    · rintro code ⟨present, codeEq⟩
      subst code
      exact abnormal present

/-- WP rule for a scoped global mutable borrow. A successful body result is
written back once, while missing-resource and body aborts are handled in the
same predicate. -/
@[wp_norm] theorem wp_withBorrowMutSpec (resource : Resource World Key Value)
    (key : Key) (body : Value → Spec World (Result × Value))
    (ensures : Result → World → Prop) (aborts : Nat → Prop)
    (initial : World) :
    wp (resource.withBorrowMutSpec key body) ensures aborts initial ↔
      (∀ value, resource.lookup initial key = some value →
        wp (body value)
          (fun output bodyWorld =>
            ensures output.1 (resource.insert bodyWorld key output.2))
          aborts initial) ∧
      (resource.lookup initial key = none → aborts Resource.executionFailure) := by
  constructor
  · rintro ⟨normal, abnormal, defined⟩
    refine ⟨?_, ?_⟩
    · intro value present
      refine ⟨?_, ?_, ?_⟩
      · intro output bodyWorld execution
        apply normal output.1 (resource.insert bodyWorld key output.2)
        exact ⟨value, bodyWorld, output.2, present, execution, rfl⟩
      · intro code execution
        exact abnormal code (.inr ⟨value, present, execution⟩)
      · intro obligation
        exact defined ⟨value, present, obligation⟩
    · intro missing
      apply abnormal Resource.executionFailure
      exact .inl ⟨missing, rfl⟩
  · rintro ⟨bodyWP, missingWP⟩
    refine ⟨?_, ?_, ?_⟩
    · rintro result finalWorld
        ⟨value, bodyWorld, finalValue, present, execution, finalEq⟩
      subst finalWorld
      exact (bodyWP value present).1 (result, finalValue) bodyWorld execution
    · rintro code (missing | bodyAbort)
      · rcases missing with ⟨notPresent, codeEq⟩
        subst code
        exact missingWP notPresent
      · rcases bodyAbort with ⟨value, present, execution⟩
        exact (bodyWP value present).2.1 code execution
    · rintro ⟨value, present, obligation⟩
      exact (bodyWP value present).2.2 obligation

@[wp_norm] theorem wp_withBorrowMutFocusSpec
    (resource : Resource World Key Owner) (key : Key)
    (get : Owner → Focus) (set : Owner → Focus → Owner)
    (body : Mutation Focus → Spec World (Result × Mutation Focus))
    (ensures : Result → World → Prop) (aborts : Nat → Prop)
    (initial : World) :
    wp (resource.withBorrowMutFocusSpec key get set body) ensures aborts initial ↔
      (∀ owner, resource.lookup initial key = some owner →
        wp (withMutation (get owner) body)
          (fun output bodyWorld =>
            ensures output.1
              (resource.insert bodyWorld key (set owner output.2)))
          aborts initial) ∧
      (resource.lookup initial key = none → aborts Resource.executionFailure) := by
  unfold Resource.withBorrowMutFocusSpec
  rw [wp_withBorrowMutSpec]
  constructor
  · rintro ⟨normal, missing⟩
    refine ⟨?_, missing⟩
    intro owner present
    have bodyWP := normal owner present
    change wp
      (Spec.bind (withMutation (get owner) body) fun output =>
        Spec.pure (output.1, set owner output.2))
      _ aborts initial at bodyWP
    rw [wp_bind] at bodyWP
    simpa only [wp_pure] using bodyWP
  · rintro ⟨normal, missing⟩
    refine ⟨?_, missing⟩
    intro owner present
    have bodyWP := normal owner present
    change wp
      (Spec.bind (withMutation (get owner) body) fun output =>
        Spec.pure (output.1, set owner output.2))
      _ aborts initial
    rw [wp_bind]
    simpa only [wp_pure] using bodyWP

end Move.Verify
