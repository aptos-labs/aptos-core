-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: source verification.

import Move

/-! Loop invariants: `invariant P` at the head of a `loop`/`while` body is
the loop's invariant, a proposition over the loop's state (locals and the
current referents of live mutable references).  The automatic proof
establishes it on entry and preserves it per iteration; the code after the
loop starts from the invariant and the exit condition. -/

namespace Tests.MovePrograms.LoopInvariants

open Move
open scoped Move Move.Spec

module LoopInvariants where

  struct Bits has Copy, Drop, Store where
    length : U64
    bit_field : Vector Bool

  -- A `loop` with its exit test.
  fun count_to (n : U64) : U64 := do
    let mut i : U64 := 0
    loop
      invariant i ≤ n
      if !(i < n) then break
      i := i + 1
    i

  spec count_to (n : U64) where
    ensures result = n;
    aborts_if False

  -- A `while` form; the invariant still holds before the condition.
  fun sum_ones (n : U64) : U64 := do
    let mut i : U64 := 0
    let mut total : U64 := 0
    while i < n do
      invariant i ≤ n ∧ total = i
      total := total + 1
      i := i + 1
    total

  spec sum_ones (n : U64) where
    ensures result = n;
    aborts_if False

  -- A loop writing through a `&mut` parameter: the referent travels with
  -- the loop state, so the invariant can speak about it.  An invariant
  -- relates to the past through values snapshotted before the loop.
  fun clear (self : &mut Bits) : Action Unit := do
    let field ← &self.bit_field
    let len := field.length
    let ref ← &self.length
    let length0 ← *ref
    let mut i : U64 := 0
    loop
      invariant i ≤ len ∧ self.bit_field.toList.length = len.toNat ∧
        self.length = length0
      if !(i < len) then break
      let bits ← &mut self.bit_field
      let bit ← &mut bits[i]
      bit := false
      i := i + 1

  spec clear (self : &mut Bits) where
    ensures self.length = old(self).length ∧
      self.bit_field.toList.length = old(self).bit_field.toList.length;
    aborts_if False

  verify count_to
  verify sum_ones
  verify clear

end Tests.MovePrograms.LoopInvariants
