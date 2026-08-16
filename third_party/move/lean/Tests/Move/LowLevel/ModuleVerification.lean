-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move
import Tests.Common

/-!
# Low-level module source-semantics fixtures

This fixture exercises the foundational prophecy and relational APIs directly.
End-user declarative `Action` contracts live beside the actual programs in
`Tests.Move.Account` and `Tests.Move.Arithmetic`.
-/

open scoped Move Move.Compiler Move.Spec

move_module SourceVerification where

  @[move_enum]
  inductive Choice where
    | fallback
    | chosen (value : U64)
    deriving Copy, Drop, Store

  /-- A pure Move function can be unfolded and proved about directly. -/
  fun choose (fallback : U64) (choice : Choice) : U64 :=
    match choice with
    | .fallback => fallback
    | .chosen value => value

  spec choose (fallback : U64) (choice : Choice) where
    ensures
      result = match choice with
        | .fallback => fallback
        | .chosen value => value

  -- Keep one fixture for the explicit-proof form; source modules normally use
  -- the automatic `verify choose` command.
  verify choose by
    intro fallback choice
    cases choice <;> rfl

  theorem choose_fallback (fallback : U64) :
      choose fallback .fallback = fallback := by
    rfl

  theorem choose_chosen (fallback value : U64) :
      choose fallback (.chosen value) = value := by
    rfl

  /-- The executable definition still uses the concise source operator. -/
  fun increment (value : U64) : U64 := value + 1

  /-- Relational interpretation of `increment`, including Move overflow. -/
  def incrementSpec (value : U64) : Semantics.Spec Unit U64 :=
    Semantics.Checked.addSpec value 1

  def incrementContract : Verify.Contract Unit U64 U64 := Verify.Contract.mk
    (fun _ _ => True)
    (fun value initial result final =>
      value.toNat + 1 < U64.size ∧
        result = U64.ofNat (value.toNat + 1) ∧ final = initial)
    (fun value _ code =>
      code = Semantics.Checked.arithmeticAbortCode ∧
        ¬value.toNat + 1 < U64.size)

  /-- Both successful execution and overflow are covered by one contract. -/
  theorem increment_verified :
      Verify.Satisfies incrementSpec incrementContract := by
    intro value initial _
    constructor
    · intro result final execution
      exact execution
    · intro code execution
      exact execution

  /-- Mutable source syntax lowers normally; its proof semantics uses a
  prophecy loan and returns the owner value at loan death. -/
  fun replace (replacement : U64) : Action U64 := do
    let values : Vector U64 := vector![3]
    let value ← &mut values[0]
    value := replacement
    (*value)

  def replaceSpec (replacement : U64) : Semantics.Spec Unit U64 :=
    Semantics.Spec.bind
      (Semantics.withMutation (U64.ofNat 3)
        (Verify.assignSpecBody replacement))
      (fun output => Semantics.Spec.pure output.2)

  def replaceContract : Verify.Contract Unit U64 U64 := Verify.Contract.mk
    (fun _ _ => True)
    (fun replacement initial result final =>
      result = replacement ∧ final = initial)
    (fun _ _ _ => False)

  theorem replace_verified :
      Verify.Satisfies replaceSpec replaceContract := by
    apply Verify.satisfies_of_wp
    intro args initial _
    unfold replaceSpec
    rw [Verify.wp_bind]
    simp [replaceContract]

  /-- This proof-only value also checks that ordinary `def` declarations are
  not selected as Move functions by the enclosing `move_module`. -/
  def compiledForTest : MoveModel.Frontend.XIR.MModule :=
    move_module% "SourceVerificationTest"

  #test Tests.run compiledForTest "choose" []
      [.u64 3, .variant 1 [.u64 9]] = Tests.okRet [] [.u64 9]
  #test Tests.run compiledForTest "increment" [] [.u64 41] =
      Tests.okRet [] [.u64 42]
  #test Tests.run compiledForTest "replace" [] [.u64 9] =
      Tests.okRet [] [.u64 9]
