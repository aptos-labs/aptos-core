-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: compiler integration.

import Move

/-! Low-level executable and theorem checks for the direct source semantics. -/

namespace Tests.SourceVerification

open Move
open Move.Semantics
open Move.Verify

example (left right : U64) :
    if Move.Verify.Source.logicalLE left right then
      left.toNat ≤ right.toNat
    else
      ¬left.toNat ≤ right.toNat := by
  move_cases ordered : Move.Verify.Source.logicalLE left right <;>
    assumption

example (n : U64) :
    if Move.Verify.Source.logicalLT 0 n then True else n = 0 := by
  move_cases positive : Move.Verify.Source.logicalLT 0 n

private def increment : ConcreteLoan Nat Unit := fun value => ((), value + 1)

example (initial final : Nat) :
    ProphecyLoan (liftConcrete increment) initial () final ↔ final = initial + 1 := by
  rw [prophecyLoan_liftConcrete_iff]
  simp [increment, eq_comm]

private structure Pair where
  left : Nat
  right : Nat
  deriving Repr

private def leftLens : Lens Pair Nat where
  get := Pair.left
  set := fun pair value => { pair with left := value }
  get_set := by intros; rfl
  set_get := by intro pair; cases pair; rfl
  set_set := by intros; rfl

example (parent : Mutation Pair) (future : Nat) :
    let split := parent.split leftLens future
    let written := split.child.write future
    parent.closeChild leftLens written = split.suspendedParent := by
  simp only
  apply Mutation.closeChild_eq_suspended
  rfl

private def setStateAndAbort : Txn Nat Unit := do
  Txn.set 99
  Txn.abort 7

example :
    Outcome.committedState 3 (setStateAndAbort 3) = 3 := by
  rfl

example : mutationWP 10 (assignBody 17) (fun _ final => final = 17) := by
  simp

example : mutationWP 10 readBody (fun result final => result = 10 ∧ final = 10) := by
  simp

example :
    (withMutation 10 (fun reference =>
      Spec.pure ((), reference.write 17))).ok () ((), 17) () := by
  refine ⟨17, { current := 17, prophecy := 17 }, ?_, rfl, rfl⟩
  exact ⟨rfl, rfl⟩

example : wp (withMutation 10 (assignSpecBody 17))
    (fun output finalState => output = ((), 17) ∧ finalState = ())
    (fun _ => False) () := by
  simp

example : wp (withMutation 10 readSpecBody)
    (fun output finalState => output = (10, 10) ∧ finalState = ())
    (fun _ => False) () := by
  simp

example :
    (Move.Semantics.Vector.insertSpec
      ({ current := Move.Vector.ofList [1], prophecy := Move.Vector.ofList [1] } :
        Mutation (Move.Vector U64)) (Move.UInt.ofNat 2) 7).aborts ()
      Move.Semantics.Vector.indexOutOfBounds := by
  simp [Move.Semantics.Vector.insertSpec, Move.Semantics.Vector.indexOutOfBounds,
    Move.Semantics.Mutation.read,
    Move.Vector.toList, Move.Vector.ofList,
    move_norm, Nat.reducePow]

example :
    (Move.Semantics.Vector.removeSpec
      ({ current := Move.Vector.ofList [1], prophecy := Move.Vector.ofList [1] } :
        Mutation (Move.Vector U64)) (Move.UInt.ofNat 1)).aborts ()
      Move.Semantics.Vector.indexOutOfBounds := by
  simp [Move.Semantics.Vector.removeSpec, Move.Semantics.Vector.indexOutOfBounds,
    Move.Semantics.Mutation.read,
    Move.Vector.toList, Move.Vector.ofList,
    move_norm, Nat.reducePow]

private def addFiveSpec (_ : Unit) : Spec Unit U64 :=
  Checked.addSpec (U64.ofNat 10) (U64.ofNat 5)

private def relationalAddContract : Contract Unit Unit U64 where
  requires := fun _ _ => True
  ensures := fun _ _ result _ => result = U64.ofNat 15
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False

example : Satisfies addFiveSpec relationalAddContract := by
  apply satisfies_of_wp
  intro args initial _
  cases args
  refine ⟨?_, ?_, ?_⟩
  · intro result final h
    rcases h with ⟨_, hresult, hfinal⟩
    exact ⟨fun _ => by
        -- this proof opens the relational spec on purpose, so it meets the
        -- generic `ofInt` value; `ofInt_intLit` bridges it to the unsigned
        -- view.  It is passed here rather than being a global `simp` lemma:
        -- its `no_index` numeral key is a discrimination-tree wildcard.
        simpa [relationalAddContract, move_norm, Move.UInt.ofInt_intLit,
            Nat.reducePow, Nat.reduceMod]
          using hresult,
      hfinal⟩
  · intro code h
    exact h.2 (by native_decide)
  · simp [addFiveSpec, Checked.addSpec]

private abbrev CounterWorld := Option U64 × Nat

private def counter : Resource CounterWorld Unit U64 where
  lookup := fun world _ => world.1
  insert := fun world _ value => (some value, world.2)
  erase := fun world _ => (none, world.2)

private def addToCounter (amount : U64) : Txn CounterWorld Unit :=
  counter.withBorrowMut () fun current => do
    let final ← Checked.addM current amount
    Txn.modify fun world => (world.1, world.2 + 1)
    pure ((), final)

private def addToCounterSpec (amount : U64) : Spec CounterWorld Unit :=
  counter.withBorrowMutSpec () fun current => do
    let final ← Checked.addSpec current amount
    Spec.modify fun world => (world.1, world.2 + 1)
    pure ((), final)

example : addToCounter (U64.ofNat 5) (some (U64.ofNat 10), 0) =
    .ok () (some (U64.ofNat 15), 1) := by
  native_decide

example : addToCounter (U64.ofNat 1)
    (some (U64.ofNat 18446744073709551615), 0) =
      .abort Checked.arithmeticAbortCode := by
  native_decide

example : addToCounter (U64.ofNat 1) (none, 0) =
    .abort Resource.executionFailure := by
  native_decide

example : (addToCounterSpec (U64.ofNat 5)).ok
    (some (U64.ofNat 10), 0) () (some (U64.ofNat 15), 1) := by
  apply (Resource.withBorrowMutSpec_present_ok
    (resource := counter) (key := ())
    (initial := (some (U64.ofNat 10), 0))
    (finalWorld := (some (U64.ofNat 15), 1))
    (body := fun current => do
      let final ← Checked.addSpec current (U64.ofNat 5)
      Spec.modify fun world => (world.1, world.2 + 1)
      pure ((), final))
    (value := U64.ofNat 10) (result := ()) rfl).2
  refine ⟨(some (U64.ofNat 10), 1), U64.ofNat 15, ?_, rfl⟩
  refine ⟨U64.ofNat 15, (some (U64.ofNat 10), 0), ?_, ?_⟩
  · exact ⟨by native_decide, rfl, rfl⟩
  · change (Spec.bind
      (Spec.modify fun world : CounterWorld => (world.1, world.2 + 1))
      (fun _ => Spec.pure ((), U64.ofNat 15))).ok
        (some (U64.ofNat 10), 0) ((), U64.ofNat 15)
        (some (U64.ofNat 10), 1)
    exact ⟨(), (some (U64.ofNat 10), 1), ⟨rfl, rfl⟩, ⟨rfl, rfl⟩⟩

example : (addToCounterSpec (U64.ofNat 1)).aborts
    (some (U64.ofNat 18446744073709551615), 0)
    Checked.arithmeticAbortCode := by
  apply (Resource.withBorrowMutSpec_present_aborts
    (resource := counter) (key := ())
    (initial := (some (U64.ofNat 18446744073709551615), 0))
    (body := fun current => do
      let final ← Checked.addSpec current (U64.ofNat 1)
      Spec.modify fun world => (world.1, world.2 + 1)
      pure ((), final))
    (value := U64.ofNat 18446744073709551615)
    (code := Checked.arithmeticAbortCode) rfl).2
  exact .inl ⟨rfl, by native_decide⟩

example : (addToCounterSpec (U64.ofNat 1)).aborts
    (none, 0) Resource.executionFailure := by
  exact (Resource.withBorrowMutSpec_missing_aborts
    (resource := counter) (key := ()) (initial := (none, 0))
    (body := fun current => do
      let final ← Checked.addSpec current (U64.ofNat 1)
      Spec.modify fun world => (world.1, world.2 + 1)
      pure ((), final))
    (code := Resource.executionFailure) rfl).2 rfl

private def addFiveContract : Contract CounterWorld Unit Unit where
  requires := fun _ initial => initial = (some (U64.ofNat 10), 0)
  ensures := fun _ _ result final =>
    result = () ∧ final = (some (U64.ofNat 15), 1)
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False
  -- This function does change the counter world, so it frames nothing. A
  -- generated contract would say the same through its `modifies` clause.
  frame := fun _ _ _ => True

example : Satisfies (fun _ => Spec.ofTxn (addToCounter (U64.ofNat 5)))
    addFiveContract := by
  apply satisfies_of_txnWP
  intro args initial hinitial
  cases args
  subst initial
  unfold txnWP
  rw [show addToCounter (U64.ofNat 5) (some (U64.ofNat 10), 0) =
    .ok () (some (U64.ofNat 15), 1) by native_decide]
  exact ⟨⟨rfl, rfl⟩, trivial⟩

private def alwaysAbortContract : Contract Nat Unit Unit where
  requires := fun _ _ => True
  ensures := fun _ _ _ _ => False
  aborts := fun _ initial code => initial = 3 → code = 7
  mayAbort := fun _ _ => True

example : Satisfies (fun _ => Spec.ofTxn setStateAndAbort) alwaysAbortContract := by
  apply satisfies_of_txnWP
  intro _ initial _
  simp [setStateAndAbort, txnWP]
  intro h
  subst initial
  rfl

end Tests.SourceVerification
