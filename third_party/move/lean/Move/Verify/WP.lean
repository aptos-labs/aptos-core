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

@[simp, wp_norm] theorem wp_borrowElemSpec (values : Move.Vector α)
    (index : Move.U64) (ensures : α → State → Prop) (aborts : Nat → Prop)
    (initial : State) :
    wp (Vector.borrowElemSpec (σ := State) values index) ensures aborts initial ↔
      (∀ value, values.toList[index.toNat]? = some value → ensures value initial) ∧
      (values.toList[index.toNat]? = none → aborts Resource.executionFailure) := by
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
  · rintro ⟨normal, abnormal⟩ future
    constructor
    · intro output final execution current
      apply normal (output.1, future) final
      exact ⟨future, output.2, execution, current, rfl⟩
    · intro code execution
      exact abnormal code ⟨future, execution⟩
  · intro bodyWP
    constructor
    · rintro ⟨result, finalOwner⟩ final
        ⟨future, reference, execution, current, ownerEq⟩
      change finalOwner = future at ownerEq
      subst finalOwner
      exact (bodyWP future).1 (result, reference) final execution current
    · rintro code ⟨future, execution⟩
      exact (bodyWP future).2 code execution

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
      if index.toNat ≤ reference.current.toList.length then
        ensures
          ((), reference.write (Move.Vector.ofList
            (reference.current.toList.take index.toNat ++
              value :: reference.current.toList.drop index.toNat)))
          initial
      else
        aborts Vector.indexOutOfBounds := by
  by_cases inBounds : index.toNat ≤ reference.current.toList.length
  · simp only [Vector.insertSpec, Mutation.read, inBounds, if_pos]
    exact wp_pure _ _ _ _
  · simp only [Vector.insertSpec, Mutation.read, inBounds]
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
                reference.current.toList.drop (index.toNat + 1))))
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
  · rintro ⟨normal, abnormal⟩
    constructor
    · intro value present
      constructor
      · intro output bodyWorld execution
        apply normal output.1 (resource.insert bodyWorld key output.2)
        exact ⟨value, bodyWorld, output.2, present, execution, rfl⟩
      · intro code execution
        exact abnormal code (.inr ⟨value, present, execution⟩)
    · intro missing
      apply abnormal Resource.executionFailure
      exact .inl ⟨missing, rfl⟩
  · rintro ⟨bodyWP, missingWP⟩
    constructor
    · rintro result finalWorld
        ⟨value, bodyWorld, finalValue, present, execution, finalEq⟩
      subst finalWorld
      exact (bodyWP value present).1 (result, finalValue) bodyWorld execution
    · rintro code (missing | bodyAbort)
      · rcases missing with ⟨notPresent, codeEq⟩
        subst code
        exact missingWP notPresent
      · rcases bodyAbort with ⟨value, present, execution⟩
        exact (bodyWP value present).2 code execution

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
