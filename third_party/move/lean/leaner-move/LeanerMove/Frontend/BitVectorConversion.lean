-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove.Frontend.Xast

/-!
Specification arithmetic is mathematical. The Move Prover's bit-vector
representation (`pragma bv`, `bv_ret`, `bv_internal`, and the classification
bitwise operations induce) is a backend encoding and is not modeled; see G14 in
`designs/prover-test-problems.md`. The explicit conversions are: `int2bv(e)`
wraps `e` into the fixed-width type of its result, two's complement for a signed
type, and `bv2int(e)` reads the value back unchanged.
-/

namespace LeanerMove.Frontend.BitVectorConversion

open Xast

private def numericCall (loc : Loc) (operation : Operation) (arguments : List Exp) : Exp :=
  .mk .num loc (.call operation [] arguments none)

private def number (loc : Loc) (value : Int) : Exp :=
  .mk .num loc (.value (.number value) none false)

private def asNumber (value : Exp) : Exp :=
  if value.ty == .num then value else numericCall value.loc .cast [value]

/-- The width and signedness of a fixed-width integer type. -/
def fixedWidth? : Ty → Option (Nat × Bool)
  | .u8 => some (8, false) | .u16 => some (16, false) | .u32 => some (32, false)
  | .u64 => some (64, false) | .u128 => some (128, false) | .u256 => some (256, false)
  | .i8 => some (8, true) | .i16 => some (16, true) | .i32 => some (32, true)
  | .i64 => some (64, true) | .i128 => some (128, true) | .i256 => some (256, true)
  | _ => none

/-- `value` wrapped into `bits` bits, two's complement when `signed`. -/
def wrap (bits : Nat) (signed : Bool) (value : Int) : Int :=
  let modulus : Int := 2 ^ bits
  if signed then (value + 2 ^ (bits - 1)) % modulus - 2 ^ (bits - 1) else value % modulus

/-- The least nonnegative residue modulo `modulus`, for every integer: the two
remainders are also exact where the target's remainder truncates toward zero. -/
private def residue (modulus : Int) (value : Exp) : Exp :=
  let loc := value.loc
  let remainder := numericCall loc .mod [asNumber value, number loc modulus]
  numericCall loc .mod [numericCall loc .add [remainder, number loc modulus], number loc modulus]

private def booleanCall (loc : Loc) (operation : Operation) (arguments : List Exp) : Exp :=
  .mk .bool loc (.call operation [] arguments none)

/-- `int2bv(value)` at the fixed-width type `type`, as a mathematical expression.
A value already in range is kept as it is, so a proof meets the residue only
where the conversion changes the value. -/
def int2bv (type : Ty) (value : Exp) : Except String Exp := do
  let some (bits, signed) := fixedWidth? type
    | throw "`int2bv` needs a fixed-width integer result type; a generic or `num` one has no width"
  let loc := value.loc
  -- A parameter holds a value of its type.
  if value.ty == type && value.node matches .param _ then return value
  let wrapped := match value.node with
    | .value (.number constant) _ _ => number loc (wrap bits signed constant)
    | _ =>
      let modulus : Int := 2 ^ bits
      let low : Int := if signed then -(2 ^ (bits - 1)) else 0
      let integer := asNumber value
      let inRange := booleanCall loc .and [booleanCall loc .le [number loc low, integer],
        booleanCall loc .lt [integer, number loc (low + modulus)]]
      let residue :=
        if signed then numericCall loc .add
          [residue modulus (numericCall loc .sub [integer, number loc low]), number loc low]
        else residue modulus integer
      .mk .num loc (.ite inRange integer residue)
  return .mk type loc (.call .cast [] [wrapped] none)

end LeanerMove.Frontend.BitVectorConversion
