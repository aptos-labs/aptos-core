-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: Move language.

import Move
import MoveModel.Tests.Common

/-! Signed integer (`I8` … `I256`) arithmetic, literals, negation, comparison,
and resource updates, verified against the checked signed semantics.  Signed
arithmetic aborts when the mathematical result leaves the two's-complement
range `[-halfSize, halfSize)`; division truncates toward zero and additionally
overflows at `minInt / -1`. -/

namespace Tests.MovePrograms

open Move
open scoped Move Move.Compiler Move.Spec

module Signed where

  /-! ## Pure arithmetic -/

  fun add_values (left : I64) (right : I64) : Action I64 :=
    pure (Move.SInt.add left right)

  spec add_values (left : I64) (right : I64) where
    ensures result = Move.SInt.ofInt (left.toInt + right.toInt);
    aborts_if ¬ Move.Semantics.Checked.inRange (Move.numTypeOf Move.Signed W64)
        (left.toInt + right.toInt)
      with Semantics.Checked.arithmeticAbortCode

  verify add_values

  fun sub_values (left : I32) (right : I32) : Action I32 :=
    pure (Move.SInt.sub left right)

  spec sub_values (left : I32) (right : I32) where
    ensures result = Move.SInt.ofInt (left.toInt - right.toInt);
    aborts_if ¬ Move.Semantics.Checked.inRange (Move.numTypeOf Move.Signed W32)
        (left.toInt - right.toInt)
      with Semantics.Checked.arithmeticAbortCode

  verify sub_values

  fun mul_values (left : I16) (right : I16) : Action I16 :=
    pure (Move.SInt.mul left right)

  spec mul_values (left : I16) (right : I16) where
    ensures result = Move.SInt.ofInt (left.toInt * right.toInt);
    aborts_if ¬ Move.Semantics.Checked.inRange (Move.numTypeOf Move.Signed W16)
        (left.toInt * right.toInt)
      with Semantics.Checked.arithmeticAbortCode

  verify mul_values

  /-! Signed division truncates toward zero and aborts on a zero divisor or the
  `minInt / -1` overflow. -/

  fun div_values (left : I32) (right : I32) : Action I32 :=
    pure (Move.SInt.div left right)

  spec div_values (left : I32) (right : I32) where
    ensures True;
    aborts_if (right.toInt = 0 ∨
        ¬ Move.Semantics.Checked.inRange (Move.numTypeOf Move.Signed W32) (left.toInt.tdiv right.toInt))
      with Semantics.Checked.arithmeticAbortCode

  verify div_values

  fun mod_values (left : I32) (right : I32) : Action I32 :=
    pure (Move.SInt.mod left right)

  spec mod_values (left : I32) (right : I32) where
    ensures result = Move.SInt.ofInt (left.toInt.tmod right.toInt);
    aborts_if right.toInt = 0 with Semantics.Checked.arithmeticAbortCode

  verify mod_values

  /-! ## Literals, negation, comparison (these compile; markers are lowered to
  signed loads, `0 - x`, and the shared ordering primitive). -/

  fun positive_literal : Action I8 :=
    pure (100 : I8)

  fun negative_literal : Action I32 :=
    pure (-5 : I32)

  fun negate_value (value : I64) : Action I64 :=
    pure (-value)

  fun below (left : I32) (right : I32) : Action Bool :=
    pure (Move.SInt.less left right)

  /-! ## A signed resource -/

  struct Balance has Key where
    amount : I64

  entry fun credit (addr : Address) (delta : I64) : Action Unit := do
    let value ← &mut Balance[addr].amount
    let current ← *value
    value := Move.SInt.add current delta

  spec credit (addr : Address) (delta : I64) where
    requires existsAt<Balance>(addr);
    modifies Balance[addr];
    ensures Balance[addr].amount =
      Move.SInt.ofInt (old(Balance[addr].amount).toInt + delta.toInt);
    aborts_if ¬ Move.Semantics.Checked.inRange (Move.numTypeOf Move.Signed W64)
        (old(Balance[addr].amount).toInt + delta.toInt)
      with Semantics.Checked.arithmeticAbortCode

  verify credit

end Tests.MovePrograms
