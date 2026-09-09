-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: compiler fixtures.

import Move
import MoveModel.Tests.Common

namespace Tests.MovePrograms.Addresses

open Move
open scoped Move Move.Compiler Move.Spec

address_alias application = 0x42

module Aliased at application where

  public fun own_address : Address :=
    @application

  def compiled : MoveModel.IR.Module :=
    lowerToIR ``Tests.MovePrograms.Addresses.Aliased

  #test compiled.address = 0x42
  #test Tests.run compiled "own_address" [] [] = Tests.okVals [.address 0x42]

end Tests.MovePrograms.Addresses
