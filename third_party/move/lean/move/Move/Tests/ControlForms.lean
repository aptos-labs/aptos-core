-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move
import MoveModel.Tests.Common
open Move
open scoped Move Move.Compiler Move.Spec

/-! Control and expression forms verified from retained source: `return`
inside a loop, a dependent `if`, an `if`/`else` statement with a continuation,
and checked arithmetic in conditions, indices, and embedded positions — each
sequenced before the term it feeds, so its abort stays observable. -/

module ControlForms where
  struct Box has Copy, Drop, Store where
    value : U64

  -- `return` from inside `while`: the loop's fixed point exits early.
  fun return_in_loop (n : U64) : U64 := do
    let mut n := n
    while 0 < n do
      if n == 3 then return 1
      n := n - 1
    n

  spec return_in_loop (n : U64) where
    ensures result.toNat ≤ 1;
    aborts_if False

  verify return_in_loop by
    contract_intro
    move_cases hloop : Move.Verify.Source.logicalLT 0 args
    · move_cases hthree : Move.Verify.Source.logicalBEq args 3 = true
      · simp [wp_norm, move_norm, Nat.reducePow]
      · rw [Move.Semantics.Checked.subSpec_one_eq_pure_of_pos hloop,
          Move.Semantics.Spec.pure_bind]
        exact Move.Verify.wp_of_satisfies recursiveVerified trivial
    · subst args
      simp [Move.Verify.wp, Move.Semantics.Spec.pure]

  -- A dependent `if` in statement position; the hypothesis is unused here.
  fun clamp (value : U64) : Action U64 := do
    let mut result := value
    if _h : value < 10 then
      result := value
    else
      result := 10
    pure result

  spec clamp (value : U64) where
    ensures result.toNat ≤ 10;
    aborts_if False

  verify clamp

  -- `if`/`else` as a statement, followed by a continuation that observes both
  -- branches.
  fun then_else (flag : Bool) : Action U64 := do
    let mut value : U64 := 0
    if flag then
      value := 1
    else
      value := 2
    pure (value + 1)

  spec then_else (flag : Bool) where
    ensures result = if flag then 2 else 3;
    aborts_if False

  verify then_else

  -- Arithmetic in an `if` condition: evaluated (and able to abort) before the
  -- branch is chosen.
  fun arithmetic_condition (value : U64) : Action U64 := do
    if value + 1 < 2 then
      pure 1
    else
      pure 0

  spec arithmetic_condition (value : U64) where
    ensures result = if value.toNat = 0 then 1 else 0;
    aborts_if ¬value.toNat + 1 < U64.size
      with Semantics.Checked.arithmeticAbortCode

  verify arithmetic_condition

  -- The same with the operation spelled as its primitive.
  fun explicit_arithmetic_condition (value : U64) : Action U64 := do
    if Move.UInt.add value 1 < 2 then pure 1 else pure 0

  spec explicit_arithmetic_condition (value : U64) where
    ensures result = if value.toNat = 0 then 1 else 0;
    aborts_if ¬value.toNat + 1 < U64.size
      with Semantics.Checked.arithmeticAbortCode

  verify explicit_arithmetic_condition

  -- Arithmetic in an index position.
  fun index_arithmetic (base : U64) : Action U64 := do
    let values : Vector U64 := vector![10, 20, 30]
    let value ← &values[base + 1]
    (*value)

  spec index_arithmetic (base : U64) where
    requires base.toNat = 1;
    ensures result = 30;
    aborts_if False

  verify index_arithmetic

  -- Arithmetic embedded in a structure literal.
  fun embedded (value : U64) : Action Box := do
    pure { value := value + 1 }

  spec embedded (value : U64) where
    ensures result.value = value + 1;
    aborts_if ¬value.toNat + 1 < U64.size
      with Semantics.Checked.arithmeticAbortCode

  verify embedded

  /-! ## Tests -/

  def compiled : MoveModel.IR.Module := lowerToIR ``ControlForms

  private def run := Tests.run compiled

  #test run "return_in_loop" [] [.u64 5] = Tests.okU64 1
  #test run "return_in_loop" [] [.u64 2] = Tests.okU64 0
  #test run "clamp" [] [.u64 50] = Tests.okU64 10
  #test run "clamp" [] [.u64 4] = Tests.okU64 4
  #test run "then_else" [] [.bool true] = Tests.okU64 2
  #test run "then_else" [] [.bool false] = Tests.okU64 3
  #test run "arithmetic_condition" [] [.u64 0] = Tests.okU64 1
  #test run "arithmetic_condition" [] [.u64 5] = Tests.okU64 0
  #test run "explicit_arithmetic_condition" [] [.u64 0] = Tests.okU64 1
  #test run "index_arithmetic" [] [.u64 1] = Tests.okU64 30
  #test run "embedded" [] [.u64 4] = Tests.okVals [.struct [.u64 5]]
