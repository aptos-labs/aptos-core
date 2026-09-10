-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: Move language.

import Move.Tests.Compiler.Fixtures.Modules.Registry
import MoveModel.Tests.Common

/-! Literal and named addresses used in values, storage locations, module
identities, and cross-module references. -/

namespace Tests.MovePrograms

open Move
open scoped Move Move.Compiler Move.Spec

example : (@0x1 : Address) ≠ @0x2 := by decide
example : (@0xa550c18 : Address) = @0xA550C18 := by decide

address_alias accounts_admin = 0xA550C18

module Accounts at 0xCAFE where

  struct Vault has Key where
    balance : U64

  fun owner : Address := @0xCAFE

  fun framework : Address := @aptos_framework

  fun the_admin : Address := @accounts_admin

  fun is_admin (addr : Address) : Bool := addr == @accounts_admin

  spec is_admin (addr : Address) where
    ensures result = (addr == @accounts_admin)

  fun balance_of_owner : Action U64 := do
    let value ← &Vault[@0xCAFE].balance
    (*value)

  spec balance_of_owner where
    requires existsAt<Vault>(@0xCAFE);
    ensures result = old(Vault[@0xCAFE].balance);
    aborts_if False

  fun registry_home : Address := Modules.Registry.home

  verify is_admin

  verify balance_of_owner by
    contract_intro
    rcases Option.isSome_iff_exists.mp permitted with ⟨vault, lookup⟩
    rw [Move.Verify.wp_bind, Move.Verify.wp_borrowSpec]
    simp [Move.Semantics.ResourceStore.get, lookup]

  def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.Accounts

namespace Accounts

private def run := Tests.run compiled

#guard compiled.address = 0xCAFE

#test compiled.externalFuns.map
    (fun reference =>
      (reference.address, reference.moduleName, reference.functionName)) =
  [(2, "Registry", "home")]

#test run "owner" [] [] = Tests.okVals [.address 0xCAFE]
#test run "framework" [] [] = Tests.okVals [.address 0x1]
#test run "the_admin" [] [] = Tests.okVals [.address 0xA550C18]
#test run "is_admin" [] [.address 0xA550C18] = Tests.okVals [.bool true]
#test run "is_admin" [] [.address 0xCAFE] = Tests.okVals [.bool false]

private def vaultId := compiled.resourceId "Vault"

#test run "balance_of_owner"
    [(MoveModel.IR.resourceKey vaultId [], 0xCAFE, .struct [.u64 7])] [] =
  Tests.okRet
    [(MoveModel.IR.resourceKey vaultId [], 0xCAFE, .struct [.u64 7])] [.u64 7]

end Accounts

end Tests.MovePrograms
