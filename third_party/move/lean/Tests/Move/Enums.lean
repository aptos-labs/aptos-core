-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move
import Tests.Common

/-! Native enum construction and exhaustive matching through Move. -/

namespace Tests.MovePrograms

open Move
open MoveModel.Frontend.XIR
open scoped Move Move.Compiler Move.Spec

move_module Enums where

  enum Action where
    | idle
    | transfer (amount : U64)
    | split (left right : U64)
    deriving Copy, Drop, Store

  /-! ## Functions -/

  fun makeTransfer (amount : U64) : Action := .transfer amount

  spec makeTransfer (amount : U64) where
    ensures result = .transfer amount

  fun total (action : Action) : U64 :=
    match action with
    | .idle => 0
    | .transfer amount => amount
    | .split left right => left + right

  spec total (action : Action) where
    ensures
      result =
        match action with
        | .idle => 0
        | .transfer amount => amount
        | .split left right => left + right

  fun classify (action : Action) : U64 :=
    match action with
    | .idle => 0
    | .transfer _ => 1
    | .split _ _ => 2

  spec classify (action : Action) where
    ensures
      result =
        match action with
        | .idle => 0
        | .transfer _ => 1
        | .split _ _ => 2

  /-! ## Proofs -/

  verify makeTransfer
  verify total
  verify classify

  /-! ## Tests -/

  def compiled : MModule := move_module% "EnumsTest"

  private def run := Tests.run compiled

  #test run "makeTransfer" [] [.u64 7] = Tests.okRet [] [.variant 1 [.u64 7]]
  #test run "total" [] [.variant 0 []] = Tests.okRet [] [.u64 0]
  #test run "total" [] [.variant 1 [.u64 9]] = Tests.okRet [] [.u64 9]
  #test run "total" [] [.variant 2 [.u64 4, .u64 5]] = Tests.okRet [] [.u64 9]
  #test run "classify" [] [.variant 2 [.u64 4, .u64 5]] = Tests.okRet [] [.u64 2]

end Tests.MovePrograms
