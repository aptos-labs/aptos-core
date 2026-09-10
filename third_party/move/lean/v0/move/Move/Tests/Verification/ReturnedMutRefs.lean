-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: specification and verification.

import Move

/-! Mutable references returned across the relational call boundary. -/

namespace Tests.MovePrograms

open Move
open scoped Move Move.Compiler Move.Spec

module ReturnedMutRefs where

  struct Cell has Copy, Drop, Store where
    value : U64

  struct Pair has Copy, Drop, Store where
    left : U64
    right : U64

  struct ExitResource has Key where
    value : U64

  enum Maybe has Copy, Drop, Store where
    | Empty
    | Filled (value : U64)

  enum Duo has Copy, Drop, Store where
    | None
    | Both (left : U64) (right : U64)

  fun return_pair (left : &mut U64) (right : &mut U64) :
      Action ((&mut U64) × (&mut U64)) := do
    pure (left, right)

  spec return_pair (left : &mut U64) (right : &mut U64) where
    ensures result.1 = old(left) ∧ result.2 = old(right) ∧
      left = old(left) ∧ right = old(right);
    aborts_if False

  verify return_pair by
    contract_intro
    unfold return_pair.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]

  fun write_pair (left : &mut U64) (right : &mut U64) : Action Unit := do
    let (returnedLeft, returnedRight) ← return_pair left right
    returnedLeft := 7
    returnedRight := 9

  spec write_pair (left : &mut U64) (right : &mut U64) where
    ensures left = 7 ∧ right = 9;
    aborts_if False

  verify write_pair by
    contract_intro
    unfold return_pair.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]

  fun write_pair_then_resume (left : &mut U64) (right : &mut U64) : Action Unit := do
    let (returnedLeft, returnedRight) ← return_pair left right
    returnedLeft := 7
    returnedRight := 9
    left := 11
    right := 13

  spec write_pair_then_resume (left : &mut U64) (right : &mut U64) where
    ensures left = 11 ∧ right = 13;
    aborts_if False

  verify write_pair_then_resume by
    contract_intro
    unfold return_pair.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]

  fun write_then_forward_pair (left : &mut U64) (right : &mut U64) :
      Action ((&mut U64) × (&mut U64)) := do
    let (returnedLeft, returnedRight) ← return_pair left right
    returnedLeft := 5
    returnedRight := 6
    pure (returnedRight, returnedLeft)

  spec write_then_forward_pair (left : &mut U64) (right : &mut U64) where
    ensures result = (6, 5) ∧ left = 5 ∧ right = 6;
    aborts_if False

  verify write_then_forward_pair by
    contract_intro
    unfold write_then_forward_pair.mutationSpec return_pair.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]

  fun write_after_forwarded_pair (left : &mut U64) (right : &mut U64) :
      Action Unit := do
    let (first, second) ← write_then_forward_pair left right
    first := 31
    second := 37

  spec write_after_forwarded_pair (left : &mut U64) (right : &mut U64) where
    ensures left = 37 ∧ right = 31;
    aborts_if False

  verify write_after_forwarded_pair by
    contract_intro
    unfold write_then_forward_pair.mutationSpec return_pair.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]

  fun return_pair_fields (pair : &mut Pair) :
      Action ((&mut U64) × (&mut U64)) := do
    let left ← &mut pair.left
    let right ← &mut pair.right
    left := 2
    right := 3
    pure (left, right)

  spec return_pair_fields (pair : &mut Pair) where
    ensures result = (2, 3) ∧ pair = Pair.mk 2 3;
    aborts_if False

  verify return_pair_fields by
    contract_intro
    unfold return_pair_fields.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]

  fun write_pair_fields (pair : &mut Pair) : Action Unit := do
    let (left, right) ← return_pair_fields pair
    left := 41
    right := 43

  spec write_pair_fields (pair : &mut Pair) where
    ensures pair = Pair.mk 41 43;
    aborts_if False

  verify write_pair_fields by
    contract_intro
    unfold return_pair_fields.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]

  fun order_pair (swap : Bool) (left : &mut U64) (right : &mut U64) :
      Action ((&mut U64) × (&mut U64)) := do
    if swap then pure (right, left) else pure (left, right)

  spec order_pair (swap : Bool) (left : &mut U64) (right : &mut U64) where
    ensures result = (if swap then (old(right), old(left))
      else (old(left), old(right))) ∧ left = old(left) ∧ right = old(right);
    aborts_if False

  verify order_pair by
    contract_intro
    unfold order_pair.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]
    intro leftFuture rightFuture
    split <;> simp_all

  fun write_ordered_pair (swap : Bool) (left : &mut U64) (right : &mut U64) :
      Action Unit := do
    let (first, second) ← order_pair swap left right
    first := 19
    second := 23

  spec write_ordered_pair (swap : Bool) (left : &mut U64) (right : &mut U64) where
    ensures if swap then left = 23 ∧ right = 19 else left = 19 ∧ right = 23;
    aborts_if False

  verify write_ordered_pair by
    contract_intro
    unfold order_pair.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]
    intro leftFuture rightFuture
    split <;> simp_all

  fun return_mixed (left : &mut U64) (flag : Bool) (right : &mut U64) :
      Action ((&mut U64) × Bool × (&mut U64)) := do
    pure (left, flag, right)

  spec return_mixed (left : &mut U64) (flag : Bool) (right : &mut U64) where
    ensures result.1 = old(left) ∧ result.2.1 = flag ∧
      result.2.2 = old(right) ∧ left = old(left) ∧ right = old(right);
    aborts_if False

  verify return_mixed by
    contract_intro
    unfold return_mixed.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]

  fun write_mixed (left : &mut U64) (right : &mut U64) : Action Bool := do
    let (returnedLeft, flag, returnedRight) ← return_mixed left true right
    returnedLeft := 13
    returnedRight := 17
    pure flag

  spec write_mixed (left : &mut U64) (right : &mut U64) where
    ensures result = true ∧ left = 13 ∧ right = 17;
    aborts_if False

  verify write_mixed by
    contract_intro
    unfold return_mixed.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]

  fun identity (slot : &mut U64) : Action (&mut U64) := do
    pure slot

  spec identity (slot : &mut U64) where
    ensures result = old(slot) ∧ slot = old(slot);
    aborts_if False

  verify identity by
    contract_intro
    unfold identity.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]

  /-- A returned loan can cross another function boundary without exposing
  its origin. The inner call's updated lender carrier becomes the outer
  mutation-level result's carrier directly. -/
  fun forward_identity (slot : &mut U64) : Action (&mut U64) := do
    let returned ← identity slot
    pure returned

  spec forward_identity (slot : &mut U64) where
    ensures result = old(slot) ∧ slot = old(slot);
    aborts_if False

  verify forward_identity by
    contract_intro
    unfold forward_identity.mutationSpec identity.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]

  /-- Forwarding preserves the returned mutation's prophecy after local use;
  it is not restricted to an immediate syntactic return. -/
  fun write_then_forward (slot : &mut U64) : Action (&mut U64) := do
    let returned ← identity slot
    returned := 13
    pure returned

  spec write_then_forward (slot : &mut U64) where
    ensures result = 13 ∧ slot = 13;
    aborts_if False

  verify write_then_forward by
    contract_intro
    unfold write_then_forward.mutationSpec identity.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]

  fun write_after_nontrivial_forward (slot : &mut U64) : Action Unit := do
    let returned ← write_then_forward slot
    returned := 17

  spec write_after_nontrivial_forward (slot : &mut U64) where
    ensures slot = 17;
    aborts_if False

  verify write_after_nontrivial_forward by
    contract_intro
    unfold write_then_forward.mutationSpec identity.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]

  fun write_forwarded (slot : &mut U64) : Action Unit := do
    let returned ← forward_identity slot
    returned := 11

  spec write_forwarded (slot : &mut U64) where
    ensures slot = 11;
    aborts_if False

  verify write_forwarded by
    contract_intro
    unfold forward_identity.mutationSpec identity.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]

  partial fun recursive_identity (done : Bool) (slot : &mut U64) :
      Action (&mut U64) := do
    if done then
      pure slot
    else
      let returned ← recursive_identity true slot
      pure returned

  spec recursive_identity (done : Bool) (slot : &mut U64) where
    ensures result = old(slot) ∧ slot = old(slot);
    aborts_if False

  verify recursive_identity by
    contract_intro
    let mutationContract : Move.Verify.Contract _moveSpecState
        (Bool × Move.Semantics.Mutation U64)
        (Move.Semantics.Mutation U64 × Move.Semantics.Mutation U64) := {
      «requires» := fun _ _ => True
      «ensures» := fun args _ output _ =>
        output.1.current = args.2.current ∧
        output.2.current = output.1.prophecy ∧
        output.2.prophecy = args.2.prophecy
      «aborts» := fun _ _ _ => False
      mayAbort := fun _ _ => False }
    have mutationVerified : Move.Verify.Satisfies
        recursive_identity.mutationSpec mutationContract := by
      unfold recursive_identity.mutationSpec
      apply Move.Verify.satisfies_fix_of_wp
      intro recursive recursiveVerified args initial _
      rcases args with ⟨done, slot⟩
      cases done with
      | false =>
          unfold recursive_identity.bodySpec
          simpa [wp_norm, mutationContract] using
            (Move.Verify.wp_of_satisfies
              (args := (true, slot)) (initial := initial)
              recursiveVerified trivial
              (noAbort := by simp [mutationContract]))
      | true =>
          unfold recursive_identity.bodySpec
          simp [wp_norm, mutationContract]
    rcases args with ⟨done, slot⟩
    simp [wp_norm]
    intro future
    apply Move.Verify.wp_mono
      (Move.Verify.wp_of_satisfies
        (args := (done, { current := slot, prophecy := future }))
        (initial := initial) mutationVerified trivial
        (noAbort := by simp [mutationContract]))
    · rintro ⟨returned, lender⟩ final
        ⟨returnedCurrent, lenderCurrent, lenderProphecy, finalState⟩
      simp only [Move.Semantics.Mutation.Finished,
        Move.Semantics.Mutation.read]
      intro returnedFinished lenderFinished
      change returned.current = slot ∧
        lender.current = returned.prophecy ∧
        lender.prophecy = future at returnedCurrent
      simp_all
    · simp [mutationContract]

  mutual
    partial fun mutual_return_left (done : Bool) (slot : &mut U64) :
        Action (&mut U64) := do
      if done then
        pure slot
      else
        mutual_return_right true slot

    partial fun mutual_return_right (done : Bool) (slot : &mut U64) :
        Action (&mut U64) := do
      if done then
        pure slot
      else
        let returned ← mutual_return_left true slot
        pure returned
  end

  spec mutual_return_left (done : Bool) (slot : &mut U64) where
    ensures result = old(slot) ∧ slot = old(slot);
    aborts_if False

  spec mutual_return_right (done : Bool) (slot : &mut U64) where
    ensures result = old(slot) ∧ slot = old(slot);
    aborts_if False

  private def mutualReturnMutationContract (State : Type) :
      Move.Verify.Contract State
        (Bool × Move.Semantics.Mutation U64)
        (Move.Semantics.Mutation U64 × Move.Semantics.Mutation U64) := {
    «requires» := fun _ _ => True
    «ensures» := fun args _ output _ =>
      output.1.current = args.2.current ∧
      output.2.current = output.1.prophecy ∧
      output.2.prophecy = args.2.prophecy
    «aborts» := fun _ _ _ => False
    mayAbort := fun _ _ => False }

  private def mutualReturnMutationContracts (State : Type) :
      (index : mutual_return_leftMutualIndex) →
        Move.Verify.Contract State
          (mutual_return_leftMutualArgs index)
          (mutual_return_leftMutualResult index)
    | .member0 => mutualReturnMutationContract State
    | .member1 => mutualReturnMutationContract State

  private theorem mutualReturnMutationVerified {State : Type} :
      ∀ index, Move.Verify.Satisfies
        (@mutual_return_leftMutualSourceSpec State index)
        (mutualReturnMutationContracts State index) := by
    unfold mutual_return_leftMutualSourceSpec
    apply Move.Verify.satisfies_fixFamily_of_wp
    intro recursive recursiveVerified index
    cases index
    all_goals
      simp only [mutual_return_leftMutualArgs,
        mutual_return_leftMutualResult,
        mutualReturnMutationContracts] at recursiveVerified ⊢
      intro args initial _
    · rcases args with ⟨done, slot⟩
      cases done with
      | false =>
          unfold mutual_return_leftMutualBody
          simpa [wp_norm, mutual_return_leftMutualResult,
              mutualReturnMutationContract] using
            (Move.Verify.wp_of_satisfies
              (args := (true, slot)) (initial := initial)
              (recursiveVerified .member1) trivial
              (noAbort := by simp [mutualReturnMutationContract]))
      | true =>
          unfold mutual_return_leftMutualBody
          simp [wp_norm, mutualReturnMutationContract]
    · rcases args with ⟨done, slot⟩
      cases done with
      | false =>
          unfold mutual_return_leftMutualBody
          simpa [wp_norm, mutual_return_leftMutualResult,
              mutualReturnMutationContract] using
            (Move.Verify.wp_of_satisfies
              (args := (true, slot)) (initial := initial)
              (recursiveVerified .member0) trivial
              (noAbort := by simp [mutualReturnMutationContract]))
      | true =>
          unfold mutual_return_leftMutualBody
          simp [wp_norm, mutualReturnMutationContract]

  private theorem mutualReturnLeftMutationVerified {State : Type} :
      Move.Verify.Satisfies (@mutual_return_left.mutationSpec State)
        (mutualReturnMutationContract State) := by
    unfold mutual_return_left.mutationSpec
    simpa [mutual_return_leftMutualArgs, mutual_return_leftMutualResult,
      mutualReturnMutationContracts] using
      (mutualReturnMutationVerified (State := State) .member0)

  private theorem mutualReturnRightMutationVerified {State : Type} :
      Move.Verify.Satisfies (@mutual_return_right.mutationSpec State)
        (mutualReturnMutationContract State) := by
    unfold mutual_return_right.mutationSpec
    simpa [mutual_return_leftMutualArgs, mutual_return_leftMutualResult,
      mutualReturnMutationContracts] using
      (mutualReturnMutationVerified (State := State) .member1)

  verify mutual_return_left by
    contract_intro
    rcases args with ⟨done, slot⟩
    simp [wp_norm]
    intro future
    apply Move.Verify.wp_mono
      (Move.Verify.wp_of_satisfies
        (args := (done, { current := slot, prophecy := future }))
        (initial := initial)
        mutualReturnLeftMutationVerified trivial
        (noAbort := by simp [mutualReturnMutationContract]))
    · rintro ⟨returned, lender⟩ final
        ⟨returnedCurrent, lenderCurrent, lenderProphecy, finalState⟩
      simp only [Move.Semantics.Mutation.Finished,
        Move.Semantics.Mutation.read]
      intro returnedFinished lenderFinished
      change returned.current = slot ∧
        lender.current = returned.prophecy ∧
        lender.prophecy = future at returnedCurrent
      simp_all
    · simp [mutualReturnMutationContract]

  verify mutual_return_right by
    contract_intro
    rcases args with ⟨done, slot⟩
    simp [wp_norm]
    intro future
    apply Move.Verify.wp_mono
      (Move.Verify.wp_of_satisfies
        (args := (done, { current := slot, prophecy := future }))
        (initial := initial)
        mutualReturnRightMutationVerified trivial
        (noAbort := by simp [mutualReturnMutationContract]))
    · rintro ⟨returned, lender⟩ final
        ⟨returnedCurrent, lenderCurrent, lenderProphecy, finalState⟩
      simp only [Move.Semantics.Mutation.Finished,
        Move.Semantics.Mutation.read]
      intro returnedFinished lenderFinished
      change returned.current = slot ∧
        lender.current = returned.prophecy ∧
        lender.prophecy = future at returnedCurrent
      simp_all
    · simp [mutualReturnMutationContract]

  /-- Move signatures carry no lifetime identifying the selected input.  The
  mutation relation can retain the branch-sensitive prophecy equations, while
  a caller must conservatively suspend both inputs. -/
  fun choose (takeLeft : Bool) (left : &mut U64) (right : &mut U64) :
      Action (&mut U64) := do
    if takeLeft then pure left else pure right

  spec choose (takeLeft : Bool) (left : &mut U64) (right : &mut U64) where
    ensures result = (if takeLeft then old(left) else old(right)) ∧
      left = old(left) ∧ right = old(right);
    aborts_if False

  verify choose by
    contract_intro
    unfold choose.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]
    intro firstFuture secondFuture
    split <;> simp_all

  fun forward_choose (takeLeft : Bool) (left : &mut U64) (right : &mut U64) :
      Action (&mut U64) := do
    let returned ← choose takeLeft left right
    pure returned

  spec forward_choose (takeLeft : Bool) (left : &mut U64) (right : &mut U64) where
    ensures result = (if takeLeft then old(left) else old(right)) ∧
      left = old(left) ∧ right = old(right);
    aborts_if False

  verify forward_choose by
    contract_intro
    unfold forward_choose.mutationSpec choose.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]
    intro firstFuture secondFuture
    split <;> simp_all

  fun write_identity (slot : &mut U64) : Action Unit := do
    let returned ← identity slot
    returned := 9

  spec write_identity (slot : &mut U64) where
    ensures slot = 9;
    aborts_if False

  verify write_identity by
    contract_intro
    unfold identity.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]

  /-- The input is revived from its prophecy once the returned loan reaches
  its last use, so it can be written again in the continuation. -/
  fun write_then_resume (slot : &mut U64) : Action Unit := do
    let returned ← identity slot
    returned := 9
    slot := 10

  spec write_then_resume (slot : &mut U64) where
    ensures slot = 10;
    aborts_if False

  verify write_then_resume by
    contract_intro
    unfold identity.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]

  /-- Each returned loan ends inside the iteration. The outer mutation then
  resumes with its enclosing prophecy still open, so it can be passed again
  on the next iteration and written after the loop. -/
  fun repeat_returned_loan (slot : &mut U64) (count : U64) : Action Unit := do
    let mut i : U64 := 0
    while i < count do
      invariant i ≤ count
      let returned ← identity slot
      returned := i
      i := i + 1
    slot := 10

  spec repeat_returned_loan (slot : &mut U64) (count : U64) where
    ensures slot = 10;
    aborts_if False

  verify repeat_returned_loan

  /-- A returned mutation may itself remain live across loop iterations. Its
  possible lender stays suspended until the loop and the result's last use
  finish, then resumes for the continuation. -/
  fun loop_carried_return (slot : &mut U64) (count : U64) : Action Unit := do
    let returned ← identity slot
    let mut i : U64 := 0
    while i < count do
      invariant i ≤ count
      returned := i
      i := i + 1
    slot := 10

  spec loop_carried_return (slot : &mut U64) (count : U64) where
    ensures slot = 10;
    aborts_if False

  verify loop_carried_return

  /-- A loop exit unwinds a returned loan before running the post-loop
  continuation, so the conservatively poisoned input is revived there. -/
  fun break_with_returned_loan (slot : &mut U64) (stop : Bool) : Action Unit := do
    let mut i : U64 := 0
    while i < 2 do
      invariant i ≤ 2
      let returned ← identity slot
      if stop then break
      returned := 7
      i := i + 1
    slot := 10

  spec break_with_returned_loan (slot : &mut U64) (stop : Bool) where
    ensures slot = 10;
    aborts_if False

  verify break_with_returned_loan

  /-- Every component of a returned-reference tuple is resolved before a
  loop exit revives the shared conservative lender set. -/
  fun break_with_returned_pair (left : &mut U64) (right : &mut U64)
      (stop : Bool) : Action Unit := do
    let mut i : U64 := 0
    while i < 2 do
      invariant i ≤ 2
      let (returnedLeft, returnedRight) ← return_pair left right
      if stop then break
      returnedLeft := 7
      returnedRight := 8
      i := i + 1
    left := 10
    right := 11

  spec break_with_returned_pair (left : &mut U64) (right : &mut U64)
      (stop : Bool) where
    ensures left = 10 ∧ right = 11;
    aborts_if False

  verify break_with_returned_pair

  /-- `continue` likewise reconciles a returned loan before invoking the next
  loop approximation; the next iteration receives the revived root. -/
  fun continue_with_returned_loan (slot : &mut U64) (count : U64)
      (skipWrite : Bool) : Action Unit := do
    let mut i : U64 := 0
    while i < count do
      invariant i ≤ count
      let returned ← identity slot
      i := i + 1
      if skipWrite then continue
      returned := i
    slot := 10

  spec continue_with_returned_loan (slot : &mut U64) (count : U64)
      (skipWrite : Bool) where
    ensures slot = 10;
    aborts_if False

  verify continue_with_returned_loan

  /-- An early function return also reconciles the loan and skips the normal
  post-loan continuation. -/
  fun return_with_returned_loan (slot : &mut U64) (early : Bool) : Action Unit := do
    let returned ← identity slot
    if early then return ()
    returned := 1
    slot := 2

  spec return_with_returned_loan (slot : &mut U64) (early : Bool) where
    ensures if early then slot = old(slot) else slot = 2;
    aborts_if False

  verify return_with_returned_loan by
    contract_intro
    unfold identity.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]
    intro future returnedFuture
    split <;> simp_all

  /-- A break targeting a loop created inside the loan does not unwind the
  loan; the returned reference remains usable after that inner loop. -/
  fun inner_break_with_returned_loan (slot : &mut U64) : Action Unit := do
    let returned ← identity slot
    let mut i : U64 := 0
    while i < 2 do
      invariant i ≤ 2
      if i == 1 then break
      i := i + 1
    returned := 3
    slot := 4

  spec inner_break_with_returned_loan (slot : &mut U64) where
    ensures slot = 4;
    aborts_if False

  verify inner_break_with_returned_loan

  /-- Exiting through nested returned loans unwinds them inside-out before the
  loop continuation sees the original root again. -/
  fun break_with_nested_returned_loans (slot : &mut U64) (stop : Bool) :
      Action Unit := do
    let mut i : U64 := 0
    while i < 2 do
      invariant i ≤ 2
      let outer ← identity slot
      let inner ← identity outer
      if stop then break
      inner := 7
      i := i + 1
    slot := 10

  spec break_with_nested_returned_loans (slot : &mut U64) (stop : Bool) where
    ensures slot = 10;
    aborts_if False

  verify break_with_nested_returned_loans

  /-- The same unwind rule applies to a direct loan of an owned local. -/
  fun break_with_local_loan (stop : Bool) : Action U64 := do
    let mut owner : U64 := 0
    let mut i : U64 := 0
    while i < 2 do
      invariant i ≤ 2
      let borrowed ← &mut owner
      if stop then break
      borrowed := 7
      i := i + 1
    owner := 10
    pure owner

  spec break_with_local_loan (stop : Bool) where
    ensures result = 10;
    aborts_if False

  verify break_with_local_loan

  /-- A structural child loan rebuilds its parent mutation before leaving the
  loop, so a later borrow observes a live complete owner. -/
  fun break_with_field_loan (cell : &mut Cell) (stop : Bool) : Action Unit := do
    let mut i : U64 := 0
    while i < 2 do
      invariant i ≤ 2
      let value ← &mut cell.value
      if stop then break
      value := 7
      i := i + 1
    let finalValue ← &mut cell.value
    finalValue := 10

  spec break_with_field_loan (cell : &mut Cell) (stop : Bool) where
    ensures cell.value = 10;
    aborts_if False

  verify break_with_field_loan

  /-- Vector-element loan reconciliation updates the vector mutation before
  control leaves the loop. -/
  fun break_with_vector_element_loan (values : &mut Vector U64) (stop : Bool) :
      Action Unit := do
    let mut i : U64 := 0
    while i < 2 do
      invariant i ≤ 2 ∧ 0 < values.toList.length
      let first ← &mut values[0]
      if stop then break
      first := 7
      i := i + 1
    let finalFirst ← &mut values[0]
    finalFirst := 10

  spec break_with_vector_element_loan (values : &mut Vector U64) (stop : Bool) where
    requires 0 < values.toList.length;
    ensures values.toList[0]? = some 10;
    aborts_if False

  verify break_with_vector_element_loan

  /-- A global resource is restored to its store before an early return; the
  normal post-loan continuation is skipped on that path. -/
  fun return_with_global_loan (address : Address) (early : Bool) : Action Unit := do
    let borrowed ← &mut ExitResource[address].value
    if early then return ()
    borrowed := 7
    let finalValue ← &mut ExitResource[address].value
    finalValue := 10

  spec return_with_global_loan (address : Address) (early : Bool) where
    requires existsAt<ExitResource>(address);
    modifies ExitResource[address];
    ensures ExitResource[address].value =
      if early then old(ExitResource[address].value) else 10;
    aborts_if False

  verify return_with_global_loan

  /-- A disjoint outer mutation may be written while an owned-local loan is
  live; the post-loan closure transports both updates. -/
  fun write_outer_during_local_loan (outer : &mut U64) : Action U64 := do
    let mut owner : U64 := 0
    let borrowed ← &mut owner
    outer := 5
    borrowed := 7
    let result ← *borrowed
    pure result

  spec write_outer_during_local_loan (outer : &mut U64) where
    ensures outer = 5 ∧ result = 7;
    aborts_if False

  verify write_outer_during_local_loan

  fun set_five (value : &mut U64) : Action Unit := do
    value := 5

  /-- Updates performed by callees must also cross an unrelated live loan;
  inspecting only source assignments would miss this case. -/
  fun call_outer_during_local_loan (outer : &mut U64) : Action U64 := do
    let mut owner : U64 := 0
    let borrowed ← &mut owner
    set_five outer
    borrowed := 7
    let result ← *borrowed
    pure result

  spec call_outer_during_local_loan (outer : &mut U64) where
    ensures outer = 5 ∧ result = 7;
    aborts_if False

  verify call_outer_during_local_loan

  /-- Structural reconciliation and a disjoint mutation update are both
  retained when the child loan dies. -/
  fun write_outer_during_field_loan (cell : &mut Cell) (outer : &mut U64) :
      Action Unit := do
    let value ← &mut cell.value
    outer := 5
    value := 7

  spec write_outer_during_field_loan (cell : &mut Cell) (outer : &mut U64) where
    ensures cell.value = 7 ∧ outer = 5;
    aborts_if False

  verify write_outer_during_field_loan

  /-- An element loan similarly transports writes to an independent mutable
  parameter while rebuilding its vector owner. -/
  fun write_outer_during_vector_loan (values : &mut Vector U64)
      (outer : &mut U64) : Action Unit := do
    let first ← &mut values[0]
    outer := 5
    first := 7

  spec write_outer_during_vector_loan (values : &mut Vector U64)
      (outer : &mut U64) where
    requires 0 < values.toList.length;
    ensures values.toList[0]? = some 7 ∧ outer = 5;
    aborts_if False

  verify write_outer_during_vector_loan by
    contract_intro
    simp [wp_norm]
    intro secondFuture
    cases elements : args.fst.toList with
    | nil => simp [elements] at permitted
    | cons head tail =>
      simp
      intro finalVector
      rw [← finalVector]
      simp [elements]

  /-- A global loan keeps disjoint mutable parameters live in its returned
  continuation while the resource is checked out of the store. -/
  fun write_outer_during_global_loan (outer : &mut U64) (address : Address) :
      Action Unit := do
    let value ← &mut ExitResource[address].value
    outer := 5
    value := 7

  spec write_outer_during_global_loan (outer : &mut U64) (address : Address) where
    requires existsAt<ExitResource>(address);
    modifies ExitResource[address];
    ensures outer = 5 ∧ ExitResource[address].value = 7;
    aborts_if False

  verify write_outer_during_global_loan

  /-- A returned loan poisons only its possible lenders. Updates to another
  mutable parameter are captured until those lenders resume. -/
  fun write_outer_during_returned_loan (slot : &mut U64) (outer : &mut U64) :
      Action Unit := do
    let returned ← identity slot
    outer := 5
    returned := 7

  spec write_outer_during_returned_loan (slot : &mut U64) (outer : &mut U64) where
    ensures slot = 7 ∧ outer = 5;
    aborts_if False

  verify write_outer_during_returned_loan by
    contract_intro
    unfold identity.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]

  /-- Multiple returned loans also retain an independent outer mutation. -/
  fun write_outer_during_returned_pair (left : &mut U64) (right : &mut U64)
      (outer : &mut U64) : Action Unit := do
    let (returnedLeft, returnedRight) ← return_pair left right
    outer := 5
    returnedLeft := 7
    returnedRight := 9

  spec write_outer_during_returned_pair (left : &mut U64) (right : &mut U64)
      (outer : &mut U64) where
    ensures left = 7 ∧ right = 9 ∧ outer = 5;
    aborts_if False

  verify write_outer_during_returned_pair by
    contract_intro
    unfold return_pair.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]

  /-- Enum payload loans rebuild the variant while retaining a write to a
  disjoint mutable parameter. -/
  fun write_outer_during_payload_loan (choice : &mut Maybe) (outer : &mut U64) :
      Action Unit := do
    match choice with
    | .Empty => abort 7
    | .Filled value =>
      outer := 5
      value := 7

  spec write_outer_during_payload_loan (choice : &mut Maybe) (outer : &mut U64) where
    requires choice is Maybe.Filled;
    ensures choice = Maybe.Filled 7 ∧ outer = 5;
    aborts_if False

  verify write_outer_during_payload_loan by
    contract_intro
    rcases args with ⟨choice, outer⟩
    cases choice <;> simp_all [wp_norm]

  fun write_chosen (takeLeft : Bool) (left : &mut U64) (right : &mut U64) :
      Action Unit := do
    let returned ← forward_choose takeLeft left right
    returned := 9

  spec write_chosen (takeLeft : Bool) (left : &mut U64) (right : &mut U64) where
    ensures if takeLeft then left = 9 ∧ right = old(right)
      else left = old(left) ∧ right = 9;
    aborts_if False

  verify write_chosen by
    contract_intro
    unfold forward_choose.mutationSpec choose.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]
    intro leftFuture rightFuture
    split <;> simp_all

  /-- The modular call rule suspends every one of three possible lenders;
  after the result resolves, all three carriers resume in parameter order. -/
  fun choose_three (takeFirst : Bool) (takeSecond : Bool)
      (first : &mut U64) (second : &mut U64)
      (third : &mut U64) : Action (&mut U64) := do
    if takeFirst then pure first
    else if takeSecond then pure second
    else pure third

  spec choose_three (takeFirst : Bool) (takeSecond : Bool)
      (first : &mut U64) (second : &mut U64)
      (third : &mut U64) where
    ensures result = (if takeFirst then old(first)
      else if takeSecond then old(second) else old(third)) ∧
      first = old(first) ∧ second = old(second) ∧ third = old(third);
    aborts_if False

  verify choose_three by
    contract_intro
    unfold choose_three.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]
    intro firstFuture secondFuture thirdFuture
    split <;> simp_all

  fun write_chosen_three (takeFirst : Bool) (takeSecond : Bool)
      (first : &mut U64) (second : &mut U64)
      (third : &mut U64) : Action Unit := do
    let returned ← choose_three takeFirst takeSecond first second third
    returned := 31

  spec write_chosen_three (takeFirst : Bool) (takeSecond : Bool)
      (first : &mut U64) (second : &mut U64)
      (third : &mut U64) where
    ensures if takeFirst then first = 31 ∧ second = old(second) ∧ third = old(third)
      else if takeSecond then first = old(first) ∧ second = 31 ∧ third = old(third)
      else first = old(first) ∧ second = old(second) ∧ third = 31;
    aborts_if False

  verify write_chosen_three by
    contract_intro
    unfold choose_three.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]
    intro firstFuture secondFuture thirdFuture
    split <;> simp_all

  /-- Mutable parameter opening is arity-independent. This regression crosses
  the former three-parameter limit and also checks that every unrelated lender
  resumes unchanged after the returned loan is resolved. -/
  fun return_one_of_four (first : &mut U64) (_second : &mut U64)
      (_third : &mut U64) (_fourth : &mut U64) : Action (&mut U64) := do
    pure first

  spec return_one_of_four (first : &mut U64) (second : &mut U64)
      (third : &mut U64) (fourth : &mut U64) where
    ensures result = old(first) ∧ first = old(first) ∧
      second = old(second) ∧ third = old(third) ∧ fourth = old(fourth);
    aborts_if False

  verify return_one_of_four by
    contract_intro
    unfold return_one_of_four.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]

  fun write_first_of_four (first : &mut U64) (second : &mut U64)
      (third : &mut U64) (fourth : &mut U64) : Action Unit := do
    let returned ← return_one_of_four first second third fourth
    returned := 41

  spec write_first_of_four (first : &mut U64) (second : &mut U64)
      (third : &mut U64) (fourth : &mut U64) where
    ensures first = 41 ∧ second = old(second) ∧
      third = old(third) ∧ fourth = old(fourth);
    aborts_if False

  verify write_first_of_four by
    contract_intro
    unfold return_one_of_four.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]

  fun borrow_value (cell : &mut Cell) : Action (&mut U64) := do
    let value ← &mut cell.value
    pure value

  spec borrow_value (cell : &mut Cell) where
    ensures result = old(cell).value ∧ cell = old(cell);
    aborts_if False

  verify borrow_value by
    contract_intro
    unfold borrow_value.mutationSpec
    simp [wp_norm]
    intro future
    cases args
    simp_all [Move.Semantics.Mutation.Finished]

  /-- A field selected for return on one loop path is reconciled when another
  path breaks and selects a fresh loan after the loop. -/
  fun borrow_value_after_break (stop : Bool) (cell : &mut Cell) :
      Action (&mut U64) := do
    loop
      invariant True
      let value ← &mut cell.value
      if stop then break
      return value
    let afterValue ← &mut cell.value
    pure afterValue

  spec borrow_value_after_break (stop : Bool) (cell : &mut Cell) where
    ensures result = old(cell).value ∧ cell = old(cell);
    aborts_if False

  /-- Continuing an iteration also resolves the temporary child before the
  next fixed-point invocation reopens it. The last iteration transfers it. -/
  fun borrow_value_after_continue (count : U64) (cell : &mut Cell) :
      Action (&mut U64) := do
    let mut i : U64 := 0
    while i < count do
      invariant i ≤ count
      let value ← &mut cell.value
      i := i + 1
      if i < count then continue
      if i == count then return value
      -- Retain a syntactic loop tail for executable lowering. The invariant
      -- and loop guard make this edge unreachable.
      let current ← *value
      value := current
    let afterValue ← &mut cell.value
    pure afterValue

  spec borrow_value_after_continue (count : U64) (cell : &mut Cell) where
    ensures result = old(cell).value ∧ cell = old(cell);
    aborts_if False

  /-- A labeled exit crosses both the returned field loan and its inner loop,
  then reopens the field in the named loop's continuation. -/
  fun borrow_value_after_labeled_break (stop : Bool) (cell : &mut Cell) :
      Action (&mut U64) := do
    loop@outer
      invariant True
      loop
        invariant True
        let value ← &mut cell.value
        if stop then break@outer
        return value
    let afterValue ← &mut cell.value
    pure afterValue

  spec borrow_value_after_labeled_break (stop : Bool) (cell : &mut Cell) where
    ensures result = old(cell).value ∧ cell = old(cell);
    aborts_if False

  /-- Returning a different mutable input abandons the selected structural
  child: its prophecy is resolved before the other root crosses the boundary. -/
  fun borrow_value_or_slot (takeSlot : Bool) (cell : &mut Cell)
      (slot : &mut U64) : Action (&mut U64) := do
    let value ← &mut cell.value
    if takeSlot then return slot
    pure value

  spec borrow_value_or_slot (takeSlot : Bool) (cell : &mut Cell)
      (slot : &mut U64) where
    ensures True;
    aborts_if False

  fun write_value_or_slot (takeSlot : Bool) (cell : &mut Cell)
      (slot : &mut U64) : Action Unit := do
    let selected ← borrow_value_or_slot takeSlot cell slot
    selected := 79

  spec write_value_or_slot (takeSlot : Bool) (cell : &mut Cell)
      (slot : &mut U64) where
    ensures if takeSlot then cell = old(cell) ∧ slot = 79
      else cell.value = 79 ∧ slot = old(slot);
    aborts_if False

  verify write_value_or_slot by
    contract_intro
    unfold borrow_value_or_slot.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]
    intro firstFuture secondFuture future
    rcases args with ⟨takeSlot, ⟨cellValue⟩, slot⟩
    cases takeSlot <;> cases firstFuture <;> simp_all

  /-- The forwarding boundary remains path-free even when the inner result was
  derived from a structure field. Only the updated root carrier crosses each
  call boundary. -/
  fun forward_value (cell : &mut Cell) : Action (&mut U64) := do
    let value ← borrow_value cell
    pure value

  spec forward_value (cell : &mut Cell) where
    ensures result = old(cell).value ∧ cell = old(cell);
    aborts_if False

  verify forward_value by
    contract_intro
    unfold forward_value.mutationSpec borrow_value.mutationSpec
    simp [wp_norm]
    intro future
    cases args
    simp_all [Move.Semantics.Mutation.Finished]

  fun write_then_forward_value (cell : &mut Cell) : Action (&mut U64) := do
    let value ← borrow_value cell
    value := 29
    pure value

  spec write_then_forward_value (cell : &mut Cell) where
    ensures result = 29 ∧ cell.value = 29;
    aborts_if False

  verify write_then_forward_value by
    contract_intro
    unfold write_then_forward_value.mutationSpec borrow_value.mutationSpec
    simp [wp_norm]
    intro future
    cases args
    simp_all [Move.Semantics.Mutation.Finished]

  fun write_after_nontrivial_value_forward (cell : &mut Cell) : Action Unit := do
    let value ← write_then_forward_value cell
    value := 31

  spec write_after_nontrivial_value_forward (cell : &mut Cell) where
    ensures cell.value = 31;
    aborts_if False

  verify write_after_nontrivial_value_forward by
    contract_intro
    unfold write_then_forward_value.mutationSpec borrow_value.mutationSpec
    simp [wp_norm]
    intro future
    simp_all [Move.Semantics.Mutation.Finished]

  fun write_forwarded_value (cell : &mut Cell) : Action Unit := do
    let value ← forward_value cell
    value := 19

  spec write_forwarded_value (cell : &mut Cell) where
    ensures cell.value = 19;
    aborts_if False

  verify write_forwarded_value by
    contract_intro
    unfold forward_value.mutationSpec borrow_value.mutationSpec
    simp [wp_norm]
    intro future
    simp_all [Move.Semantics.Mutation.Finished]

  fun write_value (cell : &mut Cell) : Action Unit := do
    let value ← borrow_value cell
    value := 17

  spec write_value (cell : &mut Cell) where
    ensures cell.value = 17;
    aborts_if False

  verify write_value by
    contract_intro
    unfold borrow_value.mutationSpec
    simp [wp_norm]
    intro future
    simp_all [Move.Semantics.Mutation.Finished]

  fun borrow_first (values : &mut Vector U64) : Action (&mut U64) := do
    let first ← &mut values[0]
    pure first

  spec borrow_first (values : &mut Vector U64) where
    requires 0 < values.toList.length;
    ensures True;
    aborts_if False

  verify borrow_first by
    contract_intro
    unfold borrow_first.mutationSpec
    simp [wp_norm]
    intro empty
    simp [empty] at permitted

  /-- A vector-element transfer is resolved on the break path before the
  element is borrowed again after the loop. -/
  fun borrow_first_after_break (stop : Bool) (values : &mut Vector U64) :
      Action (&mut U64) := do
    loop
      invariant 0 < values.toList.length
      let first ← &mut values[0]
      if stop then break
      return first
    let afterFirst ← &mut values[0]
    pure afterFirst

  spec borrow_first_after_break (stop : Bool) (values : &mut Vector U64) where
    requires 0 < values.toList.length;
    ensures True;
    aborts_if False

  fun write_first (values : &mut Vector U64) : Action Unit := do
    let first ← borrow_first values
    first := 23

  spec write_first (values : &mut Vector U64) where
    requires 0 < values.toList.length;
    ensures values.toList[0]? = some 23;
    aborts_if False

  verify write_first by
    contract_intro
    unfold borrow_first.mutationSpec
    simp [wp_norm]
    intro outerFuture
    constructor
    · intro value atFirst returnFuture finished finalOwner
      subst outerFuture
      simp [Move.Semantics.Mutation.Finished] at finished
      subst returnFuture
      cases elements : args.toList with
      | nil => simp [elements] at atFirst
      | cons head tail => simp [elements] at atFirst ⊢
    · intro empty
      simp [empty] at permitted

  fun return_vector_element_and_slot (values : &mut Vector U64)
      (slot : &mut U64) : Action ((&mut U64) × (&mut U64)) := do
    let first ← &mut values[0]
    pure (first, slot)

  spec return_vector_element_and_slot (values : &mut Vector U64)
      (slot : &mut U64) where
    requires 0 < values.toList.length;
    ensures True;
    aborts_if False

  verify return_vector_element_and_slot by
    contract_intro
    unfold return_vector_element_and_slot.mutationSpec
    simp [wp_norm]
    intro empty
    simp [empty] at permitted

  fun write_vector_element_and_slot (values : &mut Vector U64)
      (slot : &mut U64) : Action Unit := do
    let (first, returnedSlot) ← return_vector_element_and_slot values slot
    first := 47
    returnedSlot := 53

  spec write_vector_element_and_slot (values : &mut Vector U64)
      (slot : &mut U64) where
    requires 0 < values.toList.length;
    ensures values.toList[0]? = some 47 ∧ slot = 53;
    aborts_if False

  verify write_vector_element_and_slot by
    contract_intro
    unfold return_vector_element_and_slot.mutationSpec
    simp [wp_norm]
    intro outerFuture slotFuture
    constructor
    · intro value atFirst returnFuture finished finalOwner
      intro finalSlot finalVector finalSlotFuture
      simp [Move.Semantics.Mutation.Finished] at finalOwner finalSlot
      subst returnFuture
      subst finished
      subst outerFuture
      subst slotFuture
      cases elements : args.fst.toList with
      | nil => simp [elements] at atFirst
      | cons head tail => simp [elements] at atFirst ⊢
    · intro empty
      simp [empty] at permitted

  fun borrow_payload (slot : &mut Maybe) : Action (&mut U64) := do
    match slot with
    | .Empty => abort 7
    | .Filled value => pure value

  spec borrow_payload (slot : &mut Maybe) where
    pragma aborts_if_is_partial;
    ensures match old(slot) with
      | .Empty => True
      | .Filled value => result = value;
    aborts_if slot is Maybe.Empty with 7

  verify borrow_payload by
    contract_intro
    unfold borrow_payload.mutationSpec
    simp [wp_norm]
    intro future
    cases args <;> simp_all [Move.Semantics.Mutation.Finished]
    all_goals simp [wp_norm]

  /-- A variant payload transfer is resolved and its owner rebuilt on the
  break path before a second payload match returns a fresh loan. -/
  fun borrow_payload_after_break (stop : Bool) (slot : &mut Maybe) :
      Action (&mut U64) := do
    loop
      invariant slot is Maybe.Filled
      match slot with
      | .Empty => abort 7
      | .Filled value =>
        if stop then break
        return value
    match slot with
    | .Empty => abort 7
    | .Filled value => pure value

  spec borrow_payload_after_break (stop : Bool) (slot : &mut Maybe) where
    requires slot is Maybe.Filled;
    ensures True;
    aborts_if False

  /-- Every transferred payload child is resolved inside-out on the break
  edge; the post-loop match can then return a fresh pair. -/
  fun borrow_payload_pair_after_break (stop : Bool) (duo : &mut Duo) :
      Action ((&mut U64) × (&mut U64)) := do
    loop
      invariant ∃ left right, duo = Duo.Both left right
      match duo with
      | .None => abort 9
      | .Both left right =>
        if stop then break
        return (left, right)
    match duo with
    | .None => abort 9
    | .Both left right => pure (left, right)

  spec borrow_payload_pair_after_break (stop : Bool) (duo : &mut Duo) where
    requires ∃ left right, duo = Duo.Both left right;
    ensures True;
    aborts_if False

  fun write_payload (slot : &mut Maybe) : Action Unit := do
    let value ← borrow_payload slot
    value := 29

  spec write_payload (slot : &mut Maybe) where
    requires slot is Maybe.Filled;
    ensures slot = Maybe.Filled 29;
    aborts_if False

  verify write_payload by
    contract_intro
    unfold borrow_payload.mutationSpec
    simp [wp_norm]
    intro future
    cases args <;> simp_all [Move.Semantics.Mutation.Finished]
    all_goals simp [wp_norm]
    intro finalOwner
    exact finalOwner.symm

  fun return_payload_and_slot (choice : &mut Maybe) (slot : &mut U64) :
      Action ((&mut U64) × (&mut U64)) := do
    match choice with
    | .Empty => abort 7
    | .Filled value => pure (value, slot)

  spec return_payload_and_slot (choice : &mut Maybe) (slot : &mut U64) where
    requires choice is Maybe.Filled;
    ensures True;
    aborts_if False

  verify return_payload_and_slot by
    contract_intro
    unfold return_payload_and_slot.mutationSpec
    simp [wp_norm]
    intro choiceFuture slotFuture
    rcases args with ⟨choice, slot⟩
    cases choice <;> simp_all [Move.Semantics.Mutation.Finished]
    all_goals simp [wp_norm]

  fun write_payload_and_slot (choice : &mut Maybe) (slot : &mut U64) :
      Action Unit := do
    let (value, returnedSlot) ← return_payload_and_slot choice slot
    value := 59
    returnedSlot := 61

  spec write_payload_and_slot (choice : &mut Maybe) (slot : &mut U64) where
    requires choice is Maybe.Filled;
    ensures choice = Maybe.Filled 59 ∧ slot = 61;
    aborts_if False

  verify write_payload_and_slot by
    contract_intro
    unfold return_payload_and_slot.mutationSpec
    simp [wp_norm]
    intro choiceFuture slotFuture
    rcases args with ⟨choice, slot⟩
    cases choice <;> simp_all [Move.Semantics.Mutation.Finished]
    all_goals simp [wp_norm]
    intro finalChoice finalSlot
    exact ⟨finalChoice.symm, finalSlot.symm⟩

  fun return_payload_pair (duo : &mut Duo) :
      Action ((&mut U64) × (&mut U64)) := do
    match duo with
    | .None => abort 9
    | .Both left right => pure (left, right)

  spec return_payload_pair (duo : &mut Duo) where
    requires ∃ left right, duo = Duo.Both left right;
    ensures True;
    aborts_if False

  verify return_payload_pair by
    contract_intro
    unfold return_payload_pair.mutationSpec
    simp [wp_norm]
    intro future
    cases args <;> simp_all [Move.Semantics.Mutation.Finished]
    all_goals simp [wp_norm]

  fun write_payload_pair (duo : &mut Duo) : Action Unit := do
    let (left, right) ← return_payload_pair duo
    left := 67
    right := 71

  spec write_payload_pair (duo : &mut Duo) where
    requires ∃ left right, duo = Duo.Both left right;
    ensures duo = Duo.Both 67 71;
    aborts_if False

  verify write_payload_pair by
    contract_intro
    unfold return_payload_pair.mutationSpec
    simp [wp_norm]
    intro future
    cases args <;> simp_all [Move.Semantics.Mutation.Finished]
    all_goals simp [wp_norm]
    intro finalOwner
    exact finalOwner.symm

  /-! A native or body-less function may participate in returned-reference
  calls only by exporting this full prophecy relation. Its ordinary contract
  remains the separate, reference-erased client view. -/

  native fun summarized_native (slot : &mut U64) : Action (&mut U64)

  noncomputable def summarized_native.mutationSpec {State : Type} :
      Move.Semantics.Mutation U64 →
        Move.Semantics.Spec State
          (Move.Semantics.Mutation U64 × Move.Semantics.Mutation U64) :=
    fun slot => Move.Semantics.transferMutation slot

  spec summarized_native (slot : &mut U64) where
    ensures True;
    aborts_if False

  fun call_summarized_native (slot : &mut U64) : Action Unit := do
    let returned ← summarized_native slot
    returned := 37

  spec call_summarized_native (slot : &mut U64) where
    ensures slot = 37;
    aborts_if False

  verify call_summarized_native by
    contract_intro
    unfold summarized_native.mutationSpec
    simp [wp_norm, Move.Semantics.Mutation.Finished]

end Tests.MovePrograms
