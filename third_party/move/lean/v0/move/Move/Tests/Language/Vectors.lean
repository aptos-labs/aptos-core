-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: Move language.

import Move
import MoveModel.Tests.Common

/-! Basic vector source syntax, lowering, verification, and execution. -/

namespace Tests.MovePrograms

open Move
open scoped Move Move.Compiler Move.Spec

module Vectors where

  /-! ## Functions -/

  fun make : Vector U64 := vector![10, 20, 30]

  spec make where
    ensures result = vector![10, 20, 30]

  fun length : U64 := make.length

  spec length where
    ensures result = 3

  fun middle : Action U64 := do
    let values := make
    let value ← &values[1]
    (*value)

  spec middle where
    ensures result = 20;
    aborts_if False

  fun replace : Action U64 := do
    let values := make
    let value ← &mut values[1]
    value := 42
    (*value)

  spec replace where
    ensures result = 42;
    aborts_if False

  fun insert_middle : Action U64 := do
    let values : Vector U64 := vector![10, 30]
    let valuesRef ← &mut values
    Move.Vector.insert valuesRef 1 20
    let updated ← *valuesRef
    let middle ← &updated[1]
    (*middle)

  spec insert_middle where
    ensures result = 20;
    aborts_if False

  fun remove_middle : Action U64 := do
    let values : Vector U64 := vector![10, 20, 30]
    let valuesRef ← &mut values
    let removed ← Move.Vector.remove valuesRef 1
    let updated ← *valuesRef
    let shifted ← &updated[1]
    let shiftedValue ← *shifted
    pure (removed + shiftedValue)

  spec remove_middle where
    ensures result = 50;
    aborts_if False

  /-! ## Proofs -/

  verify make

  verify length

  verify middle

  verify replace

  verify insert_middle

  verify remove_middle by
    contract_intro
    simp [move_spec, wp_norm, move_norm, Nat.reducePow,
      Move.Semantics.Mutation.read, Move.Semantics.Mutation.write,
      Move.Verify.wp, and_assoc]

  /-! ## Tests -/

  def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.Vectors

  private def run := Tests.run compiled

  #test run "make" [] [] = Tests.okRet [] [.vector [.u64 10, .u64 20, .u64 30]]
  #test run "length" [] [] = Tests.okRet [] [.u64 3]
  #test run "middle" [] [] = Tests.okRet [] [.u64 20]
  #test run "replace" [] [] = Tests.okRet [] [.u64 42]
  #test run "insert_middle" [] [] = Tests.okRet [] [.u64 20]
  #test run "remove_middle" [] [] = Tests.okRet [] [.u64 50]

end Tests.MovePrograms
