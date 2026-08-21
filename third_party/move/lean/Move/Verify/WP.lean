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
@[simp, wp_norm] theorem wp_addSpec {W : Type} [Move.Width W] (lhs rhs : Move.UInt W)
    (ensures : Move.UInt W → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Checked.addSpec lhs rhs : Spec State (Move.UInt W)) ensures aborts initial ↔
      (lhs.toNat + rhs.toNat < (Move.widthOf W).size →
        ensures (Move.UInt.ofNat (lhs.toNat + rhs.toNat)) initial) ∧
      (¬lhs.toNat + rhs.toNat < (Move.widthOf W).size →
        aborts Checked.arithmeticAbortCode) := by
  rw [wp_total_iff (by simp [Checked.addSpec])]
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
@[simp, wp_norm] theorem wp_subSpec {W : Type} [Move.Width W] (lhs rhs : Move.UInt W)
    (ensures : Move.UInt W → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Checked.subSpec lhs rhs : Spec State (Move.UInt W)) ensures aborts initial ↔
      (rhs.toNat ≤ lhs.toNat →
        ensures (Move.UInt.ofNat (lhs.toNat - rhs.toNat)) initial) ∧
      (¬rhs.toNat ≤ lhs.toNat → aborts Checked.arithmeticAbortCode) := by
  rw [wp_total_iff (by simp [Checked.subSpec])]
  constructor
  · rintro ⟨normal, abnormal⟩
    exact ⟨fun safe => normal _ initial ⟨safe, rfl, rfl⟩,
      fun underflow => abnormal _ ⟨rfl, underflow⟩⟩
  · rintro ⟨normal, abnormal⟩
    constructor
    · rintro value final ⟨safe, rfl, rfl⟩
      exact normal safe
    · rintro code ⟨rfl, underflow⟩
      exact abnormal underflow

/-- Checked multiplication in one obligation. -/
@[simp, wp_norm] theorem wp_mulSpec {W : Type} [Move.Width W] (lhs rhs : Move.UInt W)
    (ensures : Move.UInt W → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Checked.mulSpec lhs rhs : Spec State (Move.UInt W)) ensures aborts initial ↔
      (lhs.toNat * rhs.toNat < (Move.widthOf W).size →
        ensures (Move.UInt.ofNat (lhs.toNat * rhs.toNat)) initial) ∧
      (¬lhs.toNat * rhs.toNat < (Move.widthOf W).size →
        aborts Checked.arithmeticAbortCode) := by
  rw [wp_total_iff (by simp [Checked.mulSpec])]
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

/-- Checked division in one obligation: the normal continuation under the
nonzero-divisor hypothesis, and the arithmetic abort otherwise. -/
@[simp, wp_norm] theorem wp_divSpec {W : Type} [Move.Width W] (lhs rhs : Move.UInt W)
    (ensures : Move.UInt W → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Checked.divSpec lhs rhs : Spec State (Move.UInt W)) ensures aborts initial ↔
      (rhs.toNat ≠ 0 →
        ensures (Move.UInt.ofNat (lhs.toNat / rhs.toNat)) initial) ∧
      (rhs.toNat = 0 → aborts Checked.arithmeticAbortCode) := by
  rw [wp_total_iff (by simp [Checked.divSpec])]
  constructor
  · rintro ⟨normal, abnormal⟩
    exact ⟨fun nonzero => normal _ initial ⟨nonzero, rfl, rfl⟩,
      fun zero => abnormal _ ⟨rfl, zero⟩⟩
  · rintro ⟨normal, abnormal⟩
    constructor
    · rintro value final ⟨nonzero, rfl, rfl⟩
      exact normal nonzero
    · rintro code ⟨rfl, zero⟩
      exact abnormal zero


/-- Checked left shift in one obligation: the normal continuation under the
in-range shift-amount hypothesis, and the arithmetic abort otherwise. -/
@[simp, wp_norm] theorem wp_shlSpec {W : Type} [Move.Width W] (lhs : Move.UInt W)
    (amount : Move.UInt Move.W8)
    (ensures : Move.UInt W → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Checked.shlSpec lhs amount : Spec State (Move.UInt W))
        ensures aborts initial ↔
      (amount.toNat < (Move.widthOf W).bits → ensures (Move.UInt.shl lhs amount) initial) ∧
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
@[simp, wp_norm] theorem wp_shrSpec {W : Type} [Move.Width W] (lhs : Move.UInt W)
    (amount : Move.UInt Move.W8)
    (ensures : Move.UInt W → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Checked.shrSpec lhs amount : Spec State (Move.UInt W))
        ensures aborts initial ↔
      (amount.toNat < (Move.widthOf W).bits → ensures (Move.UInt.shr lhs amount) initial) ∧
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
@[simp, wp_norm] theorem wp_castSpec {W W' : Type} [Move.Width W]
    [Move.Width W'] (value : Move.UInt W)
    (ensures : Move.UInt W' → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Checked.castSpec value : Spec State (Move.UInt W'))
        ensures aborts initial ↔
      (value.toNat < (Move.widthOf W').size →
        ensures (Move.UInt.cast value) initial) ∧
      (¬value.toNat < (Move.widthOf W').size → aborts Checked.arithmeticAbortCode) := by
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
@[simp, wp_norm] theorem wp_modSpec {W : Type} [Move.Width W] (lhs rhs : Move.UInt W)
    (ensures : Move.UInt W → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Checked.modSpec lhs rhs : Spec State (Move.UInt W)) ensures aborts initial ↔
      (rhs.toNat ≠ 0 →
        ensures (Move.UInt.ofNat (lhs.toNat % rhs.toNat)) initial) ∧
      (rhs.toNat = 0 → aborts Checked.arithmeticAbortCode) := by
  rw [wp_total_iff (by simp [Checked.modSpec])]
  constructor
  · rintro ⟨normal, abnormal⟩
    exact ⟨fun nonzero => normal _ initial ⟨nonzero, rfl, rfl⟩,
      fun zero => abnormal _ ⟨rfl, zero⟩⟩
  · rintro ⟨normal, abnormal⟩
    constructor
    · rintro value final ⟨nonzero, rfl, rfl⟩
      exact normal nonzero
    · rintro code ⟨rfl, zero⟩
      exact abnormal zero

@[simp, wp_norm] theorem wp_borrowElemSpec (values : Move.Vector α)
    (index : Move.U64) (ensures : α → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Vector.borrowElemSpec (σ := State) values index) ensures aborts initial ↔
      (∀ value, values.toList[index.toNat]? = some value → ensures value initial) ∧
      (values.toList[index.toNat]? = none → aborts Resource.executionFailure) := by
  rw [wp_total_iff (by simp [Vector.borrowElemSpec])]
  constructor
  · rintro ⟨normal, abnormal⟩
    constructor
    · intro value present
      apply normal value initial
      change values.toList[index.toNat]? = some value ∧ initial = initial
      exact ⟨present, rfl⟩
    · intro missing
      apply abnormal Resource.executionFailure
      change values.toList[index.toNat]? = none ∧
        Resource.executionFailure = Resource.executionFailure
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
      (values.toList[index.toNat]? = none → aborts Resource.executionFailure) := by
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
      apply abnormal Resource.executionFailure
      change values.toList[index.toNat]? = none ∧
        Resource.executionFailure = Resource.executionFailure
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

@[simp, wp_norm] theorem wp_withBorrowElemMutSpec (values : Move.Vector α)
    (index : Move.U64) (body : Mutation α → Spec State (β × Mutation α))
    (ensures : (β × Move.Vector α) → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Vector.withBorrowElemMutSpec values index body) ensures aborts initial ↔
      match values.toList[index.toNat]? with
      | none => aborts Resource.executionFailure
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
