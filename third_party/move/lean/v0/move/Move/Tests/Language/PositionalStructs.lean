-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: Move language.

import Move
import MoveModel.Tests.Common

namespace Tests.MovePrograms

open Move
open scoped Move Move.Compiler Move.Spec

module PositionalStructs where

  struct Pair(U64, Bool) has Copy, Drop, Store

  fun make : Pair := Pair.mk 7 true

  fun first (pair : Pair) : U64 :=
    pair._0

  fun destructure (pair : Pair) : U64 := do
    let ⟨left, _⟩ := pair
    left

  fun destructure_move_spelling (pair : Pair) : U64 := do
    let Pair(left, right) := pair
    if right then left else 0

  spec first (pair : Pair) where
    ensures result = pair._0

  verify first

  spec destructure_move_spelling (pair : Pair) where
    ensures result = if pair._1 then pair._0 else 0;
    aborts_if False

  verify destructure_move_spelling

  def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.PositionalStructs
  private def run := Tests.run compiled

  #test run "make" [] [] = Tests.okVals [.struct [.u64 7, .bool true]]
  #test run "first" [] [.struct [.u64 9, .bool false]] = Tests.okU64 9
  #test run "destructure" [] [.struct [.u64 11, .bool true]] = Tests.okU64 11
  #test run "destructure_move_spelling" [] [.struct [.u64 12, .bool true]] =
    Tests.okU64 12

end Tests.MovePrograms
