-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: Move language.

import Move
import MoveModel.Tests.Common

/-! Native enum construction and exhaustive matching through Move. -/

namespace Tests.MovePrograms

open Move
open scoped Move Move.Compiler Move.Spec

module Enums where

  enum Action has Copy, Drop, Store where
    | Idle
    | Transfer (amount : U64)
    | Split (left right : U64)

  /-! ## Functions -/

  fun make_transfer (amount : U64) : Action := .Transfer amount

  spec make_transfer (amount : U64) where
    ensures result = .Transfer amount

  fun total (action : Action) : U64 :=
    match action with
    | .Idle => 0
    | .Transfer amount => amount
    | .Split left right => left + right

  spec total (action : Action) where
    ensures
      result =
        match action with
        | .Idle => 0
        | .Transfer amount => amount
        | .Split left right => left + right

  fun classify (action : Action) : U64 :=
    match action with
    | .Idle => 0
    | .Transfer _ => 1
    | .Split _ _ => 2

  fun is_transfer (action : Action) : Bool :=
    action is Action.Transfer

  fun mixed_match (action : Action) (flag : Bool) : U64 := do
    match action, flag with
    | .Idle, true => pure 1
    | .Transfer amount, true => pure amount
    | _, false => pure 0
    | .Split left right, true => pure (left + right)

  spec classify (action : Action) where
    ensures
      result =
        match action with
        | .Idle => 0
        | .Transfer _ => 1
        | .Split _ _ => 2

  /-! ## Proofs -/

  verify make_transfer
  verify total
  verify classify

  spec is_transfer (action : Action) where
    ensures result = match action with | .Transfer _ => true | _ => false

  verify is_transfer

  spec mixed_match (action : Action) (flag : Bool) where
    ensures result = if flag then
      match action with
      | .Idle => 1
      | .Transfer amount => amount
      | .Split left right => left + right
      else 0;
    aborts_if match action with
      | .Split left right => flag ∧ ¬left.toNat + right.toNat < U64.size
      | _ => False
      with Semantics.Checked.arithmeticAbortCode

  verify mixed_match by
    contract_intro
    rcases args with ⟨action, flag⟩
    cases action <;> cases flag <;> simp [wp_norm, move_norm]

  /-! ## Tests -/

  def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.Enums

  private def run := Tests.run compiled

  #test run "make_transfer" [] [.u64 7] = Tests.okRet [] [.variant 1 [.u64 7]]
  #test run "total" [] [.variant 0 []] = Tests.okRet [] [.u64 0]
  #test run "total" [] [.variant 1 [.u64 9]] = Tests.okRet [] [.u64 9]
  #test run "total" [] [.variant 2 [.u64 4, .u64 5]] = Tests.okRet [] [.u64 9]
  #test run "classify" [] [.variant 2 [.u64 4, .u64 5]] = Tests.okRet [] [.u64 2]
  #test run "is_transfer" [] [.variant 1 [.u64 8]] = Tests.okRet [] [.bool true]
  #test run "mixed_match" [] [.variant 1 [.u64 8], .bool true] = Tests.okU64 8
  #test run "mixed_match" [] [.variant 2 [.u64 3, .u64 4], .bool true] =
    Tests.okU64 7
  #test run "mixed_match" [] [.variant 0 [], .bool false] = Tests.okU64 0

end Tests.MovePrograms
