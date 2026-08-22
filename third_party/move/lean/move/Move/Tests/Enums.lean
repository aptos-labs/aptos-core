-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

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

  /-! ## Tests -/

  def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.Enums

  private def run := Tests.run compiled

  #test run "make_transfer" [] [.u64 7] = Tests.okRet [] [.variant 1 [.u64 7]]
  #test run "total" [] [.variant 0 []] = Tests.okRet [] [.u64 0]
  #test run "total" [] [.variant 1 [.u64 9]] = Tests.okRet [] [.u64 9]
  #test run "total" [] [.variant 2 [.u64 4, .u64 5]] = Tests.okRet [] [.u64 9]
  #test run "classify" [] [.variant 2 [.u64 4, .u64 5]] = Tests.okRet [] [.u64 2]

end Tests.MovePrograms
