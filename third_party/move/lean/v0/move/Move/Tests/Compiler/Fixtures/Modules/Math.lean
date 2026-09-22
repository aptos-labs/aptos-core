-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: compiler fixtures.

import Move

/-! A Lean-authored Move library imported by another Lean-authored module. -/

namespace Tests.MovePrograms.Modules

open Move
open scoped Move Move.Compiler Move.Spec

module Math where

  public fun identity {T} (value : T) : T := value

  spec identity {T} [Inhabited T] (value : T) where
    ensures result = value

  verify identity

end Tests.MovePrograms.Modules
