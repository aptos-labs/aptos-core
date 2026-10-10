-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: specification and verification.

import Move
import MoveModel.Tests.Common

/-! Source contracts compose typed resource families without a module-specific
`World` record. -/

namespace Tests.MovePrograms

open Move
open scoped Move Move.Compiler Move.Spec

module ResourceComposition where

  /-! ## Functions -/

  struct Debit has Key where
    value : U64

  struct Credit has Key where
    value : U64

  fun shift (addr : Address) (amount : U64) : Action Unit := do
    let debit ← &mut Debit[addr].value
    debit := *debit - amount
    let credit ← &mut Credit[addr].value
    credit := *credit + amount

  spec shift (addr : Address) (amount : U64) where
    requires
      existsAt<Debit>(addr) ∧
      existsAt<Credit>(addr) ∧
      amount.toNat ≤ old(Debit[addr].value).toNat ∧
      old(Credit[addr].value).toNat + amount.toNat < U64.size;
    modifies Debit[addr], Credit[addr];
    ensures
      Debit[addr].value = old(Debit[addr].value) - amount ∧
      Credit[addr].value = old(Credit[addr].value) + amount;
    aborts_if False

  /-! ## Proofs -/

  verify shift

  /-! ## Tests -/

  def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.ResourceComposition

  private def debitId := compiled.resourceId "Debit"
  private def creditId := compiled.resourceId "Credit"
  private def memory (addr debit credit : Nat) : MoveModel.IR.IMem :=
    [(creditId, addr, .struct [.u64 credit]),
     (debitId, addr, .struct [.u64 debit])]

  #test Tests.run compiled "shift" (memory 3 10 4) [.address 3, .u64 3]
    = Tests.okRet (memory 3 7 7) []

end Tests.MovePrograms
