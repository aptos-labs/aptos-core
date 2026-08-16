-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move
import Tests.Common

/-! Source contracts compose typed resource families without a module-specific
`World` record. -/

namespace Tests.MovePrograms

open Move
open MoveModel.Frontend.XIR
open scoped Move Move.Compiler Move.Spec

move_module ResourceComposition where

  @[move_struct]
  structure Debit where
    value : U64
    deriving Key

  @[move_struct]
  structure Credit where
    value : U64
    deriving Key

  fun shift (addr : Address) (amount : U64) : Action Unit := do
    let debit ← &mut Debit[addr].value
    debit := *debit - amount
    let credit ← &mut Credit[addr].value
    credit := *credit + amount

  spec shift (addr : Address) (amount : U64) where
    requires
      exists<Debit>(addr) ∧
      exists<Credit>(addr) ∧
      amount.toNat ≤ old(Debit[addr].value).toNat ∧
      old(Credit[addr].value).toNat + amount.toNat < U64.size;
    ensures
      Debit[addr].value = old(Debit[addr].value) - amount ∧
      Credit[addr].value = old(Credit[addr].value) + amount;
    aborts_if False

  verify shift

  def compiled : MModule := move_module% "ResourceCompositionTest"

  private def debitId := compiled.resourceId "Debit"
  private def creditId := compiled.resourceId "Credit"
  private def memory (addr debit credit : Nat) : MoveModel.IR.IMem :=
    [(creditId, addr, .struct [.u64 credit]),
     (debitId, addr, .struct [.u64 debit])]

  #test Tests.run compiled "shift" (memory 3 10 4) [.address 3, .u64 3]
    = Tests.okRet (memory 3 7 7) []

  #emit_leaner_xir compiled

end Tests.MovePrograms
