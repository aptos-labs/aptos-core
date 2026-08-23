-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: specification and verification.

import Move
import MoveModel.Tests.Common
open Move
open scoped Move Move.Compiler Move.Spec

/-! Calls from verified bodies: effectful callees taking a mutable reference
(the caller's live `&mut` is passed and its final value written back),
including a recursive one; and pure callees, whose relational semantics is
generated on demand from their retained source when they carry no `spec`. -/

module Callees where
  struct Counter has Key where
    value : U64

  struct PairValues has Copy, Drop, Store where
    left : U64
    right : U64

  /-! ## Callees with a mutable-reference parameter -/

  fun bump (slot : &mut U64) : Action Unit := do
    slot := *slot + 1

  spec bump (slot : &mut U64) where
    ensures slot = old(slot) + 1;
    aborts_if ¬slot.toNat + 1 < U64.size
      with Semantics.Checked.arithmeticAbortCode

  verify bump

  -- The live `&mut` parameter is passed on twice.
  fun bump_twice (slot : &mut U64) : Action Unit := do
    bump slot
    bump slot

  spec bump_twice (slot : &mut U64) where
    ensures slot = old(slot) + 2;
    aborts_if ¬slot.toNat + 2 < U64.size
      with Semantics.Checked.arithmeticAbortCode

  verify bump_twice

  -- A global field borrow is passed to the callee.
  entry fun bump_counter (addr : Address) : Action Unit := do
    let value ← &mut Counter[addr].value
    bump value

  spec bump_counter (addr : Address) where
    requires existsAt<Counter>(addr);
    modifies Counter[addr];
    ensures Counter[addr].value = old(Counter[addr].value) + 1;
    aborts_if ¬old(Counter[addr].value).toNat + 1 < U64.size
      with Semantics.Checked.arithmeticAbortCode

  verify bump_counter

  -- The caller observes the reference before and after the call.
  fun take_and_bump (slot : &mut U64) : Action U64 := do
    let before ← *slot
    bump slot
    pure before

  spec take_and_bump (slot : &mut U64) where
    ensures result = old(slot) ∧ slot = old(slot) + 1;
    aborts_if ¬slot.toNat + 1 < U64.size
      with Semantics.Checked.arithmeticAbortCode

  verify take_and_bump

  -- Multiple mutable parameters have independent prophecies.  The callee's
  -- source semantics returns both final referents in declaration order, and
  -- a caller writes both values back before continuing.
  fun set_pair (left : &mut U64) (right : &mut U64) : Action Unit := do
    left := 10
    right := 20

  spec set_pair (left : &mut U64) (right : &mut U64) where
    ensures left = 10 ∧ right = 20;
    aborts_if False

  verify set_pair

  fun forward_set_pair (left : &mut U64) (right : &mut U64) : Action Unit := do
    set_pair left right

  spec forward_set_pair (left : &mut U64) (right : &mut U64) where
    ensures left = 10 ∧ right = 20;
    aborts_if False

  verify forward_set_pair

  -- A recursive callee with a mutable-reference parameter: the fixed point
  -- carries the reference's value through each call.
  partial fun drain (slot : &mut U64) : Action Unit := do
    let current ← *slot
    if current == 0 then return ()
    slot := current - 1
    continue drain slot

  spec drain (slot : &mut U64) where
    ensures slot = 0;
    aborts_if False

  verify drain by
    contract_intro
    simp [wp_norm, move_norm]
    intro future
    split
    · rename_i hzero
      intro hfuture
      subst hfuture
      exact Move.UInt.ext (by simpa using hzero)
    · rename_i hnonzero
      refine ⟨fun _ => ?_, hnonzero⟩
      refine Move.Verify.wp_mono
        (Move.Verify.wp_of_satisfies recursiveVerified trivial) ?_ ?_
      · intro result final established hfuture
        obtain ⟨hvalue, hfinal⟩ := established
        exact ⟨by rw [← hfuture]; exact hvalue, hfinal⟩
      · intro code h
        exact h

  -- Once a recursive callee is verified, automatic caller verification uses
  -- that contract without unfolding the callee's fixed point.
  fun call_drain (slot : &mut U64) : Action Unit := do
    drain slot

  spec call_drain (slot : &mut U64) where
    ensures slot = 0;
    aborts_if False

  verify call_drain

  /-! ## Mutually recursive effectful callees -/

  mutual
    partial fun mutual_ping (value : U64) : Action U64 := do
      if value == 0 then return 0
      mutual_pong (value - 1)

    partial fun mutual_pong (value : U64) : Action U64 := do
      if value == 0 then return 1
      mutual_ping (value - 1)
  end

  spec mutual_ping (value : U64) where
    ensures True;
    aborts_if False

  spec mutual_pong (value : U64) where
    ensures True;
    aborts_if False

  verify mutual_ping by
    contract_intro
    all_goals
      rw [Move.Verify.wp_ite]
      split <;> simp [wp_norm, move_norm]
    all_goals
      rename_i hzero
      constructor
      · intro _
        first
        | simpa [mutual_pingMutualArgs, mutual_pingMutualResult,
            mutual_ping.contractSpec, mutual_pong.contractSpec] using
            (Move.Verify.wp_of_satisfies
              (recursiveVerified mutual_pingMutualIndex.member0)
              (by simp [mutual_ping.contractSpec, mutual_pong.contractSpec])
              (by simp [mutual_ping.contractSpec, mutual_pong.contractSpec]))
        | simpa [mutual_pingMutualArgs, mutual_pingMutualResult,
            mutual_ping.contractSpec, mutual_pong.contractSpec] using
            (Move.Verify.wp_of_satisfies
              (recursiveVerified mutual_pingMutualIndex.member1)
              (by simp [mutual_ping.contractSpec, mutual_pong.contractSpec])
              (by simp [mutual_ping.contractSpec, mutual_pong.contractSpec]))
      · rw [Move.Verify.Source.logicalBEq_uint] at hzero
        simpa using hzero

  verify mutual_pong by
    contract_intro
    all_goals
      rw [Move.Verify.wp_ite]
      split <;> simp [wp_norm, move_norm]
    all_goals
      rename_i hzero
      constructor
      · intro _
        first
        | simpa [mutual_pingMutualArgs, mutual_pingMutualResult,
            mutual_ping.contractSpec, mutual_pong.contractSpec] using
            (Move.Verify.wp_of_satisfies
              (recursiveVerified mutual_pingMutualIndex.member0)
              (by simp [mutual_ping.contractSpec, mutual_pong.contractSpec])
              (by simp [mutual_ping.contractSpec, mutual_pong.contractSpec]))
        | simpa [mutual_pingMutualArgs, mutual_pingMutualResult,
            mutual_ping.contractSpec, mutual_pong.contractSpec] using
            (Move.Verify.wp_of_satisfies
              (recursiveVerified mutual_pingMutualIndex.member1)
              (by simp [mutual_ping.contractSpec, mutual_pong.contractSpec])
              (by simp [mutual_ping.contractSpec, mutual_pong.contractSpec]))
      · rw [Move.Verify.Source.logicalBEq_uint] at hzero
        simpa using hzero

  -- The family index is dependent: members may have different arguments and
  -- results. Merely declaring both contracts typechecks the heterogeneous
  -- projections generated from the shared fixed point.
  mutual
    partial fun heterogeneous_flag (value : U64) : Action Bool := do
      if value == 0 then return true
      let _ ← heterogeneous_count false
      return false

    partial fun heterogeneous_count (flag : Bool) : Action U64 := do
      if flag then
        let _ ← heterogeneous_flag 0
        return 1
      return 0
  end

  spec heterogeneous_flag (value : U64) where
    ensures True;
    aborts_if False

  spec heterogeneous_count (flag : Bool) where
    ensures True;
    aborts_if False

  /-! ## Pure callees -/

  fun plus_one (value : U64) : U64 := value + 1

  fun calls_pure_helper (value : U64) : Action U64 :=
    pure (plus_one value)

  spec calls_pure_helper (value : U64) where
    ensures result = value + 1;
    aborts_if ¬value.toNat + 1 < U64.size
      with Semantics.Checked.arithmeticAbortCode

  verify calls_pure_helper

  fun pure_predicate (value : U64) : Bool := value == 0

  -- A pure callee in a condition.
  fun helper_condition (value : U64) : Action U64 := do
    if pure_predicate value then pure 1 else pure 2

  spec helper_condition (value : U64) where
    ensures result = if value.toNat = 0 then 1 else 2;
    aborts_if False

  verify helper_condition

  -- Nested pure calls in an embedded position.
  fun embedded_helper (value : U64) : Action U64 := do
    let doubled := plus_one (plus_one value)
    pure doubled

  spec embedded_helper (value : U64) where
    ensures result = value + 2;
    aborts_if ¬value.toNat + 2 < U64.size
      with Semantics.Checked.arithmeticAbortCode

  verify embedded_helper

  -- A recursive pure callee: its semantics is a fixed point, which an
  -- automatic caller proof does not unfold; such a caller is proved by hand
  -- from the callee's verified contract.  Here the call is only compiled.
  partial fun sum_down (value : U64) : U64 :=
    if value < 1 then 0 else value + sum_down (value - 1)

  fun calls_recursive (value : U64) : Action U64 :=
    pure (sum_down value)

  fun run_set_pair : Action U64 := do
    let pair : PairValues := { left := 1, right := 2 }
    let pairRef ← &mut pair
    let left ← &mut pairRef.left
    let right ← &mut pairRef.right
    set_pair left right
    let leftValue ← *left
    let rightValue ← *right
    pure (leftValue + rightValue)

  /-! ## Tests -/

  def compiled : MoveModel.IR.Module := lowerToIR ``Callees

  private def counterId := compiled.resourceId "Counter"
  private def memory (addr value : Nat) : MoveModel.IR.IMem :=
    [(counterId, addr, .struct [.u64 value])]
  private def run := Tests.run compiled

  #test run "bump_counter" (memory 7 9) [.address 7] = Tests.okRet (memory 7 10) []
  #test run "calls_pure_helper" [] [.u64 3] = Tests.okU64 4
  #test run "helper_condition" [] [.u64 0] = Tests.okU64 1
  #test run "helper_condition" [] [.u64 3] = Tests.okU64 2
  #test run "embedded_helper" [] [.u64 3] = Tests.okU64 5
  #test run "calls_recursive" [] [.u64 3] = Tests.okU64 6
  #test run "run_set_pair" [] [] = Tests.okU64 30
  #test run "mutual_ping" [] [.u64 4] = Tests.okU64 0
  #test run "mutual_pong" [] [.u64 4] = Tests.okU64 1
