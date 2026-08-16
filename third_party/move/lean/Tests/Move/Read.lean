-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move
import Tests.Common

/-! Immutable borrow, returned values, and conditional aborts from Move. -/

namespace Tests.MovePrograms

open Move
open MoveModel.Frontend.XIR
open scoped Move Move.Compiler Move.Spec

move_module Read where

  @[move_struct]
  structure Reading where
    value : U64
    deriving Key

  def E_TOO_SMALL : U64 := 7

  fun read (addr : Address) : Action U64 := do
    let value ← &Reading[addr].value
    (*value)

  fun readAtLeast (addr : Address) (minimum : U64) : Action U64 := do
    let value ← &Reading[addr].value
    let current ← *value
    if current < minimum then
      abort E_TOO_SMALL
    pure current

  def compiled : MModule := move_module% "ReadTest"

  private def readingId := compiled.resourceId "Reading"
  private def memory (addr value : Nat) : MoveModel.IR.IMem :=
    [(readingId, addr, .struct [.u64 value])]
  private def run := Tests.run compiled

  #test run "read" (memory 4 99) [.address 4] =
    Tests.okRet (memory 4 99) [.u64 99]
  #test run "read" [] [.address 4] = Tests.aborted 0
  #test run "readAtLeast" (memory 4 10) [.address 4, .u64 10] =
    Tests.okRet (memory 4 10) [.u64 10]
  #test run "readAtLeast" (memory 4 9) [.address 4, .u64 10] =
    Tests.abortedIn (memory 4 9) 7

end Tests.MovePrograms
