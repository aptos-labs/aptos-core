-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: Move language.

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

  -- The checked addition is not evaluated when the left operand is false.
  -- In particular, `U64_MAX` does not overflow here.
  fun short_circuit_and (value : U64) : Action U64 := do
    if value == 0 && value + 1 == 2 then pure 1 else pure 0

  spec short_circuit_and (value : U64) where
    ensures result = 0;
    aborts_if False

  verify short_circuit_and

  -- Effects in a value conditional stay inside the selected branch.
  fun branch_effect (flag : Bool) (value : U64) : Action U64 := do
    pure (if flag then value + 1 else 0)

  spec branch_effect (flag : Bool) (value : U64) where
    ensures result = if flag then value + 1 else 0;
    aborts_if flag ∧ ¬value.toNat + 1 < U64.size
      with Semantics.Checked.arithmeticAbortCode

  verify branch_effect

  -- The same conditional sequencing applies independently to each match arm.
  fun match_effect (flag : Bool) (value : U64) : Action U64 := do
    pure (match flag with | true => value + 1 | false => 0)

  spec match_effect (flag : Bool) (value : U64) where
    ensures result = match flag with | true => value + 1 | false => 0;
    aborts_if flag ∧ ¬value.toNat + 1 < U64.size
      with Semantics.Checked.arithmeticAbortCode

  verify match_effect by
    contract_intro
    cases args.1 <;> simp [wp_norm, move_norm]

  fun match_two (left right : Bool) : Action U64 := do
    match left, right with
    | true, true => pure 2
    | true, false => pure 1
    | false, true => pure 0
    | false, false => pure 0

  spec match_two (left : Bool) (right : Bool) where
    ensures result = if left then if right then 2 else 1 else 0;
    aborts_if False

  verify match_two by
    contract_intro
    cases args.1 <;> cases args.2 <;> simp [wp_norm, move_norm]

  fun echo_flag (flag : Bool) : Action Bool := do
    pure flag

  spec echo_flag (flag : Bool) where
    ensures result = flag;
    aborts_if False

  verify echo_flag

  -- An action-valued `if let` scrutinee is sequenced before pattern choice.
  fun if_let_action (flag : Bool) : Action U64 := do
    if let true ← echo_flag flag then pure 1 else pure 0

  spec if_let_action (flag : Bool) where
    ensures result = if flag then 1 else 0;
    aborts_if False

  verify if_let_action by
    contract_intro
    cases args <;> simp [echo_flag.sourceSpec, wp_norm, move_norm]

  -- The dependent condition's hypothesis is in scope throughout the body.
  fun dependent_while (value : U64) : U64 := do
    while _h : value < 1 do
      break
    value

  spec dependent_while (value : U64) where
    ensures result = value;
    aborts_if False

  verify dependent_while

  fun checked_assert (flag : Bool) : Action U64 := do
    Move.assert flag 17
    pure 1

  spec checked_assert (flag : Bool) where
    ensures result = 1;
    aborts_if ¬flag with 17

  verify checked_assert by
    contract_intro
    cases args <;> simp [wp_norm, move_norm]

  fun checked_assert_syntax (flag : Bool) : Action U64 := do
    assert!(flag, 18)
    pure 10

  fun checked_assert_eq (left right : U64) : Action Unit := do
    assert_eq!(left, right, 19)

  fun checked_assert_ne (left right : U64) : Action Unit := do
    assert_ne!(left, right, 20)

  fun compound_local : U64 := do
    let mut value : U64 := 4
    value += 2
    value *= 3
    value

  spec compound_local where
    ensures result = 18

  verify compound_local

  fun compound_reference : Action U64 := do
    let value : U64 := 20
    let valueRef ← &mut value
    valueRef -= 5
    valueRef /= 3
    (*valueRef)

  spec compound_reference where
    ensures result = 5;
    aborts_if False

  verify compound_reference

  fun range_empty : Action U64 := do
    let mut value : U64 := 0
    for (index in 4 to 4) do
      value += index
    pure value

  spec range_empty where
    ensures result = 0;
    aborts_if False

  verify range_empty by
    contract_intro
    simp only [Move.Verify.wp, Move.Semantics.Spec.fix]
    constructor
    · intro result final execution
      rcases execution with ⟨fuel, execution⟩
      cases fuel with
      | zero => exact execution.elim
      | succ fuel => simpa [Move.Semantics.Spec.fixApprox, move_norm] using execution
    · constructor
      · intro code execution
        rcases execution with ⟨fuel, execution⟩
        cases fuel with
        | zero => exact execution.elim
        | succ fuel => simpa [Move.Semantics.Spec.fixApprox, move_norm] using execution
      · intro obligation
        rcases obligation with ⟨fuel, obligation⟩
        cases fuel with
        | zero => exact obligation.elim
        | succ fuel => simpa [Move.Semantics.Spec.fixApprox, move_norm] using obligation

  fun range_once_runtime : U64 := do
    let mut value : U64 := 0
    for (index in 4 to 5) do
      value += index
    value

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
  #test run "short_circuit_and" [] [.u64 0] = Tests.okU64 0
  #test run "short_circuit_and" [] [.u64 18446744073709551615] = Tests.okU64 0
  #test run "branch_effect" [] [.bool false, .u64 18446744073709551615] = Tests.okU64 0
  #test run "branch_effect" [] [.bool true, .u64 5] = Tests.okU64 6
  #test run "branch_effect" [] [.bool true, .u64 18446744073709551615] = Tests.aborted 0
  #test run "match_effect" [] [.bool false, .u64 18446744073709551615] = Tests.okU64 0
  #test run "match_effect" [] [.bool true, .u64 5] = Tests.okU64 6
  #test run "match_two" [] [.bool true, .bool true] = Tests.okU64 2
  #test run "match_two" [] [.bool true, .bool false] = Tests.okU64 1
  #test run "match_two" [] [.bool false, .bool true] = Tests.okU64 0
  #test run "if_let_action" [] [.bool true] = Tests.okU64 1
  #test run "if_let_action" [] [.bool false] = Tests.okU64 0
  #test run "dependent_while" [] [.u64 0] = Tests.okU64 0
  #test run "dependent_while" [] [.u64 7] = Tests.okU64 7
  #test run "checked_assert" [] [.bool true] = Tests.okU64 1
  #test run "checked_assert" [] [.bool false] = Tests.aborted 17
  #test run "checked_assert_syntax" [] [.bool true] = Tests.okU64 10
  #test run "checked_assert_syntax" [] [.bool false] = Tests.aborted 18
  #test run "checked_assert_eq" [] [.u64 3, .u64 3] = Tests.okRet [] []
  #test run "checked_assert_eq" [] [.u64 3, .u64 4] = Tests.aborted 19
  #test run "checked_assert_ne" [] [.u64 3, .u64 4] = Tests.okRet [] []
  #test run "checked_assert_ne" [] [.u64 3, .u64 3] = Tests.aborted 20
  #test run "compound_local" [] [] = Tests.okU64 18
  #test run "compound_reference" [] [] = Tests.okU64 5
  #test run "range_empty" [] [] = Tests.okU64 0
  #test run "range_once_runtime" [] [] = Tests.okU64 4
