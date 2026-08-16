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

  fun make : Move.Vector U64 := vector![10, 20, 30]

  spec make where
    ensures result = vector![10, 20, 30]

  verify make

  fun length : U64 := make.length

  fun middle : Action U64 := do
    let values := make
    let value ← &values[1]
    (*value)

  spec middle where
    requires True;
    ensures result = 20;
    aborts_if False

  verify middle

  fun replace : Action U64 := do
    let values := make
    let value ← &mut values[1]
    value := 42
    (*value)

  spec replace where
    requires True;
    ensures result = 42;
    aborts_if False

  verify replace

  fun insertMiddle : Action U64 := do
    let values : Move.Vector U64 := vector![10, 30]
    let valuesRef ← &mut values
    valuesRef.insert 1 20
    let updated ← *valuesRef
    let middle ← &updated[1]
    (*middle)

  spec insertMiddle where
    requires True;
    ensures result = 20;
    aborts_if False

  verify insertMiddle

  fun removeMiddle : Action U64 := do
    let values : Move.Vector U64 := vector![10, 20, 30]
    let valuesRef ← &mut values
    let removed ← valuesRef.remove 1
    let updated ← *valuesRef
    let shifted ← &updated[1]
    let shiftedValue ← *shifted
    pure (removed + shiftedValue)

  spec removeMiddle where
    requires True;
    ensures result = 50;
    aborts_if False

  /-- The concrete result used by this smoke test cannot overflow `u64`. -/
  @[grind .] private theorem removeMiddleResultFitsU64 : 50 < U64.size := by
    decide

  verify removeMiddle

  def compiled : MModule := move_module% "VectorsTest"

  private def run := Tests.run compiled

  #test run "make" [] [] = Tests.okRet [] [.vector [.u64 10, .u64 20, .u64 30]]
  #test run "length" [] [] = Tests.okRet [] [.u64 3]
  #test run "middle" [] [] = Tests.okRet [] [.u64 20]
  #test run "replace" [] [] = Tests.okRet [] [.u64 42]
  #test run "insertMiddle" [] [] = Tests.okRet [] [.u64 20]
  #test run "removeMiddle" [] [] = Tests.okRet [] [.u64 50]

end Tests.MovePrograms
