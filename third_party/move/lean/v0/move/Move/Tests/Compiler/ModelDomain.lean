-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: compiler integration.

import Move

/-! Regression coverage for source-value projections into specification domains. -/

namespace Tests.ModelDomain

open Move
open scoped Move

/-- Values without a specialized projection retain their logical identity. -/
structure Pair where
  left : U64
  right : Bool

example (value : U64) : value↑ = value.toNat := rfl
example (value : I64) : value↑ = value.toInt := rfl
example (left right : U64) :
    ((left)^) + ((right)^) = left.toNat + right.toNat := rfl

example (values : Vector U64) :
    values↑ = values.toList.map (fun value => value.toNat) := by
  simp

example (value : Ref U64) : value↑ = value.get.toNat := rfl
example (value : MutRef I32) : value↑ = value.get.toInt := rfl
example (value : Bool) : value↑ = value := rfl
example (value : Address) : value↑ = value := rfl
example (value : Pair) : value↑ = value := rfl

end Tests.ModelDomain
