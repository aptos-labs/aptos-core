-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move
import Tests.Common

/-! Basic vector source syntax, lowering, verification, and execution. -/

namespace Tests.MovePrograms

open Move
open MoveModel.Frontend.XIR
open scoped Move Move.Compiler Move.Spec

move_module Vectors where

  /-! ## Functions -/

  fun make : Move.Vector U64 := vector![10, 20, 30]

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

  fun insertMiddle : Action U64 := do
    let values : Move.Vector U64 := vector![10, 30]
    let valuesRef ← &mut values
    Move.Vector.insert valuesRef 1 20
    let updated ← *valuesRef
    let middle ← &updated[1]
    (*middle)

  spec insertMiddle where
    ensures result = 20;
    aborts_if False

  fun removeMiddle : Action U64 := do
    let values : Move.Vector U64 := vector![10, 20, 30]
    let valuesRef ← &mut values
    let removed ← Move.Vector.remove valuesRef 1
    let updated ← *valuesRef
    let shifted ← &updated[1]
    let shiftedValue ← *shifted
    pure (removed + shiftedValue)

  spec removeMiddle where
    ensures result = 50;
    aborts_if False

  /-! ## Proofs -/

  verify make

  @[grind .] private theorem makeLength :
      (((Move.Vector.empty.push (10 : U64)).push 20).push 30).length = 3 := by
    decide

  verify length

  verify middle

  verify replace

  verify insertMiddle

  /-- The concrete result used by this smoke test cannot overflow `u64`. -/
  @[grind .] private theorem removeMiddleResultFitsU64 : 50 < U64.size := by
    decide

  verify removeMiddle

  /-! ## Tests -/

  def compiled : MModule := move_module% "VectorsTest"

  private def run := Tests.run compiled

  #test run "make" [] [] = Tests.okRet [] [.vector [.u64 10, .u64 20, .u64 30]]
  #test run "length" [] [] = Tests.okRet [] [.u64 3]
  #test run "middle" [] [] = Tests.okRet [] [.u64 20]
  #test run "replace" [] [] = Tests.okRet [] [.u64 42]
  #test run "insertMiddle" [] [] = Tests.okRet [] [.u64 20]
  #test run "removeMiddle" [] [] = Tests.okRet [] [.u64 50]

end Tests.MovePrograms
