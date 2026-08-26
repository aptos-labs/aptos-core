-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: compiler fixtures.

import Move

/-! A Lean-authored Move library published at a non-zero address.  The address
regressions import it to check that a cross-module reference records the
callee's own address, which a fixed `0x0` would hide. -/

namespace Tests.MovePrograms.Modules

open Move
open scoped Move Move.Compiler Move.Spec

/- This package's named address, corresponding to a `Move.toml` `[addresses]`
entry. -/
address_alias registry_owner = 0x2

module Registry at registry_owner where

  public fun home : Address := @0x2

  spec home where
    ensures result = @0x2

  verify home

end Tests.MovePrograms.Modules
