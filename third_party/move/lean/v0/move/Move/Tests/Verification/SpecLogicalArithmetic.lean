-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: specification elaboration.

import Move

/-! Contract clauses use mathematical integer and vector syntax without
exposing the bounded source representation.  These examples intentionally mix
`U64` values, `Int` quantifiers, arithmetic results, and vector lengths and
indices in the same expressions. -/

namespace Tests.MovePrograms.SpecLogicalArithmetic

open Move
open scoped Move Move.Spec

module SpecLogicalArithmetic where

  struct Bits has Copy, Drop, Store where
    length : U64
    values : Vector Bool
  spec Bits where
    invariant .length = .values.length

  spec fun is_set (bits : Bits) (index : Int) : Prop :=
    0 ≤ index ∧ index < bits.values.length ∧ bits.values[index]!

  spec fun logical_length (bits : Bits) : Int := bits.length

  /-- A mathematical summary for checked source arithmetic.  The specification
  and its proof below deliberately contain no bounded-integer projections. -/
  spec fun logical_successor (value : Int) : Int := value + 1

  fun checked_successor (value : U64) : Action U64 := do
    pure (value + 1)

  spec checked_successor (value : U64) where
    requires value < 18446744073709551615;
    ensures result = logical_successor value;
    aborts_if False

  verify checked_successor by
    contract_intro
    checked_cases inRange
    simp only [logical_successor] at *
    spec_norm at *
    exact ⟨by rw [Nat.mod_eq_of_lt inRange, Int.natCast_add,
      Int.natCast_one], trivial⟩

  fun below (left right : U64) : Bool := left < right

  fun scan (bits : &Bits) (start : U64) : Action U64 := do
    let lengthRef ← &bits.length
    let length ← *lengthRef
    let mut index := start
    loop
      invariant index = start ∨ is_set bits (index - 1)
      invariant index = start ∨ index - 1 < bits.values.length
      invariant index = start ∨ below (index - 1) length
      invariant ∀ (j : Int), start ≤ j ∧ j < index → is_set bits j
      if !(index < length) then break
      index := index + 1
    pure index

  spec scan (bits : Bits) (start : U64) where
    ensures start ≤ result ∧ result ≤ logical_length bits ∧
      (result = start ∨ below (result - 1) (logical_length bits));
    aborts_if False

end Tests.MovePrograms.SpecLogicalArithmetic

/-! The shared proof normalizer lowers the clean surface in one step.  These
regressions make sure a proof does not need to unfold each helper or insert
each bounded-integer projection by hand. -/

namespace Tests.SpecLogicalArithmeticProofSurface

open Move

example (left right : U64) :
    Move.Spec.logicalLT (Move.Spec.intSub left 1) right ↔
      left.toInt - 1 < right.toInt := by
  simp only [move_norm]

example (left right : U64) :
    Move.Spec.logicalEq (Move.Spec.intAdd left 1) right ↔
      left.toInt + 1 = right.toInt := by
  simp only [move_norm]

end Tests.SpecLogicalArithmeticProofSurface
