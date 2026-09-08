-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: source verification.

import Move

/-! Mutable loans in automatic source specifications: a loan of an owned
local while a `&mut` parameter is live (independent of it), and a nested
element loan inside a loop body (the loan is a closed computation; the loop
carries on after it). -/

namespace Tests.MovePrograms.Loans

open Move
open scoped Move Move.Spec

module Loans where

  struct Buffer has Copy, Drop, Store where
    bytes : Vector U8

  struct Bits has Copy, Drop, Store where
    length : U64
    bit_field : Vector Bool

  fun extend (self : &mut Vector U8) (value : U8) : Action Unit := do
    let current ← *self
    self := current.push value

  -- `push` is bounded by the vector length limit: the contract assumes room.
  spec extend (self : &mut Vector U8) (value : U8) where
    requires self.toList.length + 1 < U64.size;
    ensures self.toList = old(self).toList ++ [value];
    aborts_if False

  -- `front` is loaned while `self` (a `&mut` parameter) is live: the loan
  -- is independent of it, and the owner is rebound to the loan's final value
  -- before the code after the loan writes `self`.
  fun splice (self : &mut Buffer) (a : U8) (b : U8) : Action Unit := do
    let mut front : Vector U8 := Move.Vector.empty
    let r ← &mut front
    extend r a
    front ← *r
    extend r b
    front ← *r
    self := { bytes := front }

  spec splice (self : &mut Buffer) (a : U8) (b : U8) where
    ensures self.bytes.toList = [a, b];
    aborts_if False

  /-- An element of an owned vector is an independent loan while `self` is
  live. The element reconciles into the vector before the continuation writes
  the unrelated mutable parameter. -/
  fun independent_element (self : &mut Buffer) : Action Unit := do
    let mut values : Vector U8 := vector![0]
    let element ← &mut values[0]
    element := 7
    self := { bytes := values }

  spec independent_element (self : &mut Buffer) where
    ensures self.bytes.toList = [7];
    aborts_if False

  -- A nested element loan inside a loop: the loan body is a closed
  -- computation, and the iteration carries on after it.
  fun clear (self : &mut Bits) : Action Unit := do
    let field ← &self.bit_field
    let len := field.length
    let mut i : U64 := 0
    loop
      if !(i < len) then break
      let bits ← &mut self.bit_field
      let bit ← &mut bits[i]
      bit := false
      i := i + 1

  -- The semantics elaborates (the loop carries `self` on); its proof needs
  -- a loop invariant.
  spec clear (self : &mut Bits) where
    ensures self.length = old(self).length;
    aborts_if False

  verify extend
  verify splice
  verify independent_element

end Tests.MovePrograms.Loans
