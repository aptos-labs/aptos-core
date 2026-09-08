-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: specification and verification.

import Move
import MoveModel.Tests.Common

/-! Interpreter tests for a nested-resource Leaner account program. -/

namespace Tests.MovePrograms

open Move
open scoped Move Move.Compiler Move.Spec

module Account where

  /-! ## Functions -/

  struct BalanceValue has Copy, Drop, Store where
    value : U64

  struct Balance has Key where
    balance : BalanceValue

  def E_INSUFFICIENT_BALANCE : U64 := 1

  entry fun deposit (addr : Address) (amount : U64) : Action Unit := do
    let value ← &mut Balance[addr].balance.value
    value := *value + amount

  spec deposit (addr : Address) (amount : U64) where
    requires existsAt<Balance>(addr);
    modifies Balance[addr];
    ensures
      Balance[addr].balance.value =
        old(Balance[addr].balance.value) + amount;
    aborts_if
      ¬old(Balance[addr].balance.value).toNat + amount.toNat < U64.size
      with Semantics.Checked.arithmeticAbortCode

  entry fun withdraw (addr : Address) (amount : U64) : Action Unit := do
    let value ← &mut Balance[addr].balance.value
    let old ← *value
    if old < amount then
      abort E_INSUFFICIENT_BALANCE
    value := *value - amount

  spec withdraw (addr : Address) (amount : U64) where
    requires
      existsAt<Balance>(addr);
    modifies Balance[addr];
    ensures
      Balance[addr].balance.value =
        old(Balance[addr].balance.value) - amount;
    aborts_if
      old(Balance[addr].balance.value).toNat < amount.toNat
      with E_INSUFFICIENT_BALANCE

  /-! ## Proofs -/

  verify deposit

  verify withdraw

  /-! ## Tests -/

  def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.Account

  private def balanceId := compiled.resourceId "Balance"
  private def memory (addr value : Nat) : MoveModel.IR.IMem :=
    [(balanceId, addr, .struct [.struct [.u64 value]])]
  private def run := Tests.run compiled

  #test run "deposit" (memory 7 10) [.address 7, .u64 5]
    = Tests.okRet (memory 7 15) []
  #test run "withdraw" (memory 7 10) [.address 7, .u64 4]
    = Tests.okRet (memory 7 6) []
  #test run "withdraw" (memory 7 3) [.address 7, .u64 4]
    = Tests.abortedIn (memory 7 3) 1
  #test run "deposit" [] [.address 7, .u64 1] = Tests.aborted 0
  #test run "deposit" (memory 7 18446744073709551615) [.address 7, .u64 1]
    = Tests.abortedIn (memory 7 18446744073709551615) 0

end Tests.MovePrograms
