-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move
import Tests.Common

/-! Arithmetic abort behavior after lowering a Leaner resource program. -/

namespace Tests.MovePrograms

open Move
open MoveModel.Frontend.XIR
open scoped Move Move.Compiler Move.Spec

move_module Arithmetic where

  /-! ## Functions -/

  fun addValues (left right : U64) : Action U64 :=
    pure (left + right)

  spec addValues (left : U64) (right : U64) where
    ensures True;
    aborts_if ¬left.toNat + right.toNat < U64.size
      with Semantics.Checked.arithmeticAbortCode

  fun explicitAdd (left right : U64) : Action U64 :=
    pure (Move.U64.add left right)

  spec explicitAdd (left : U64) (right : U64) where
    ensures True;
    aborts_if ¬left.toNat + right.toNat < U64.size
      with Semantics.Checked.arithmeticAbortCode

  fun explicitDiv (left right : U64) : Action U64 :=
    pure (Move.U64.div left right)

  spec explicitDiv (left : U64) (right : U64) where
    ensures True;
    aborts_if right.toNat = 0 with Semantics.Checked.arithmeticAbortCode

  @[move_struct]
  structure Counter where
    value : U64
    deriving Key

  @[entry]
  fun multiply (addr : Address) (factor : U64) : Action Unit := do
    let value ← &mut Counter[addr].value
    let old ← *value
    value := old * factor

  spec multiply (addr : Address) (factor : U64) where
    requires exists<Counter>(addr);
    ensures
      Counter[addr].value = old(Counter[addr].value) * factor;
    aborts_if
      ¬old(Counter[addr].value).toNat * factor.toNat < U64.size
      with Semantics.Checked.arithmeticAbortCode

  @[entry]
  fun divide (addr : Address) (divisor : U64) : Action Unit := do
    let value ← &mut Counter[addr].value
    let old ← *value
    value := old / divisor

  spec divide (addr : Address) (divisor : U64) where
    requires exists<Counter>(addr);
    ensures
      Counter[addr].value = old(Counter[addr].value) / divisor;
    aborts_if divisor.toNat = 0 with Semantics.Checked.arithmeticAbortCode

  /-! ## Proofs -/

  verify addValues

  verify explicitAdd

  verify explicitDiv

  verify multiply

  verify divide

  /-! ## Tests -/

  def compiled : MModule := move_module% "ArithmeticTest"

  private def counterId := compiled.resourceId "Counter"
  private def memory (addr value : Nat) : MoveModel.IR.IMem :=
    [(counterId, addr, .struct [.u64 value])]
  private def run := Tests.run compiled

  #test run "addValues" [] [.u64 6, .u64 7] = Tests.okU64 13
  #test run "addValues" [] [.u64 18446744073709551615, .u64 1] = Tests.aborted 0
  #test run "explicitAdd" [] [.u64 6, .u64 7] = Tests.okU64 13
  #test run "explicitAdd" [] [.u64 18446744073709551615, .u64 1] = Tests.aborted 0
  #test run "explicitDiv" [] [.u64 17, .u64 5] = Tests.okU64 3
  #test run "explicitDiv" [] [.u64 17, .u64 0] = Tests.aborted 0
  #test run "multiply" (memory 2 6) [.address 2, .u64 7]
    = Tests.okRet (memory 2 42) []
  #test run "multiply" (memory 2 18446744073709551615) [.address 2, .u64 2]
    = Tests.abortedIn (memory 2 18446744073709551615) 0
  #test run "divide" (memory 2 17) [.address 2, .u64 5]
    = Tests.okRet (memory 2 3) []
  #test run "divide" (memory 2 17) [.address 2, .u64 0]
    = Tests.abortedIn (memory 2 17) 0
  #test run "divide" [] [.address 2, .u64 1] = Tests.aborted 0

  #emit_leaner_xir compiled

end Tests.MovePrograms
