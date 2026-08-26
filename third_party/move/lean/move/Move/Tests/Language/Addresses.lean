-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: Move language.

import Move.Tests.Language.Addresses.Accounts
import Move.Tests.Compiler.Fixtures.Modules.AliasedAddress
import MoveModel.Tests.Common

namespace Tests.MovePrograms.Addresses

open Move
open scoped Move Move.Compiler Move.Spec

module LiteralAddress at 0x43 where

  fun call_aliased : Address :=
    Aliased.own_address

  fun framework_address : Address :=
    @aptos_framework

  def compiled : MoveModel.IR.Module :=
    lowerToIR ``Tests.MovePrograms.Addresses.LiteralAddress

  #test compiled.address = 0x43
  #test compiled.externalFuns.map (fun reference =>
    (reference.address, reference.moduleName, reference.functionName)) =
      [(0x42, "Aliased", "own_address")]
  #test Tests.run compiled "framework_address" [] [] = Tests.okVals [.address 0x1]

end Tests.MovePrograms.Addresses

open Move
open scoped Move Move.Compiler

/--
error: unknown Move address alias `missing_address`
-/
#guard_msgs in
#register_module_identity_at missing_address, "UnknownAddressAlias"

/--
error: address alias value does not fit in 256 bits
-/
#guard_msgs in
address_alias oversized_address =
  0x10000000000000000000000000000000000000000000000000000000000000000
