-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: Move language.

import Move
import MoveModel.Tests.Common

open Move
open scoped Move Move.Compiler Move.Spec

/-! Transient tuples and Move multiple returns. Products are flattened at
function and call boundaries and are never admitted as storable Move types. -/

module Tuples where

  struct Pair has Copy, Drop, Store where
    first : U64
    second : Bool

  fun pure_pair (value : U64) : U64 × Bool :=
    (value, true)

  spec pure_pair (value : U64) where
    ensures result.1 = value ∧ result.2 = true;
    aborts_if False

  verify pure_pair

  fun effect_pair (value : U64) : Action (U64 × Bool) := do
    pure (value, true)

  spec effect_pair (value : U64) where
    ensures result.1 = value ∧ result.2 = true;
    aborts_if False

  verify effect_pair

  fun destructure_pure (value : U64) : U64 := do
    let (first, _) := pure_pair value
    pure first

  spec destructure_pure (value : U64) where
    ensures result = value;
    aborts_if False

  verify destructure_pure

  fun destructure_effect (value : U64) : Action U64 := do
    let (first, _) ← effect_pair value
    pure first

  spec destructure_effect (value : U64) where
    ensures result = value;
    aborts_if False

  verify destructure_effect

  fun triple (value : U64) : U64 × Bool × U8 :=
    (value, true, 3)

  spec triple (value : U64) where
    ensures result.1 = value ∧ result.2.1 = true ∧ result.2.2 = 3;
    aborts_if False

  verify triple

  fun destructure_triple (value : U64) : U8 := do
    let (_, _, third) := triple value
    pure third

  spec destructure_triple (value : U64) where
    ensures result = 3;
    aborts_if False

  verify destructure_triple

  fun destructure_local (value : U64) : U64 := do
    let pair := (value, false)
    let (first, _) := pair
    pure first

  spec destructure_local (value : U64) where
    ensures result = value;
    aborts_if False

  verify destructure_local

  fun destructure_struct (value : U64) : U64 := do
    let pair : Pair := { first := value, second := true }
    let ⟨first, _⟩ := pair
    pure first

  spec destructure_struct (value : U64) where
    ensures result = value;
    aborts_if False

  verify destructure_struct

  fun destructure_struct_partial (value : U64) : U64 := do
    let pair : Pair := { first := value, second := true }
    let { first, .. } := pair
    pure first

  spec destructure_struct_partial (value : U64) where
    ensures result = value;
    aborts_if False

  verify destructure_struct_partial

  fun destructure_struct_move_spelling (value : U64) : U64 := do
    let pair : Pair := { first := value, second := true }
    let Pair { first, second } := pair
    if second then first else 0

  spec destructure_struct_move_spelling (value : U64) where
    ensures result = value;
    aborts_if False

  verify destructure_struct_move_spelling

  def compiled : MoveModel.IR.Module := lowerToIR ``Tuples

  private def run := Tests.run compiled

  #guard (compiled.funDecl? "pure_pair").map (·.returns) ==
    some [.u64, .bool]
  #guard (compiled.funDecl? "triple").map (·.returns) ==
    some [.u64, .bool, .uint .w8]

  #test run "pure_pair" [] [.u64 7] = Tests.okVals [.u64 7, .bool true]
  #test run "effect_pair" [] [.u64 7] = Tests.okVals [.u64 7, .bool true]
  #test run "destructure_pure" [] [.u64 7] = Tests.okU64 7
  #test run "destructure_effect" [] [.u64 7] = Tests.okU64 7
  #test run "triple" [] [.u64 7] = Tests.okVals [.u64 7, .bool true, .int 3]
  #test run "destructure_triple" [] [.u64 7] = Tests.okVals [.int 3]
  #test run "destructure_local" [] [.u64 7] = Tests.okU64 7
  #test run "destructure_struct" [] [.u64 7] = Tests.okU64 7
  #test run "destructure_struct_partial" [] [.u64 7] = Tests.okU64 7
  #test run "destructure_struct_move_spelling" [] [.u64 7] = Tests.okU64 7
