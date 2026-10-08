-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove.Frontend.BitVectorConversion

open LeanerMove.Frontend
open BitVectorConversion

-- The residue expansion is valid for every integer, including negative
-- values; it does not rely on a finite collection of overflow examples.
example (value modulus : Int) (positive : 0 < modulus) :
    (value.tmod modulus + modulus).tmod modulus = value % modulus := by
  have lower := Int.lt_tmod_of_pos value positive
  rw [Int.tmod_eq_emod_of_nonneg (by omega)]
  rw [Int.tmod_eq_emod]
  split
  · simp
  · rw [Int.natAbs_of_nonneg (by omega : 0 ≤ modulus)]
    simp

private def symbol : Xast.Exp :=
  .mk .num ⟨0, 0, 0⟩ (.«local» "x")

-- Interpret a generated expression at `x := input`, with the truncating
-- remainder of the neutral integer operations; both branches are checked
-- because the samples include values in and out of range.
private partial def evaluate (input : Int) : Xast.Exp → Except String Int
  | .mk _ _ (.value (.number value) _ _) => pure value
  | .mk _ _ (.«local» "x") => pure input
  | .mk _ _ (.call .cast _ [value] _) => evaluate input value
  | .mk _ _ (.ite condition yes no) => do
      evaluate input (if (← holds input condition) then yes else no)
  | .mk _ _ (.call operation _ [left, right] _) => do
      let left ← evaluate input left
      let right ← evaluate input right
      match operation with
      | .add => pure (left + right)
      | .sub => pure (left - right)
      | .mod => pure (left.tmod right)
      | _ => throw "unexpected generated operation"
  | _ => throw "unexpected generated expression"
where
  holds (input : Int) : Xast.Exp → Except String Bool
    | .mk _ _ (.call .and _ [left, right] _) => return (← holds input left) && (← holds input right)
    | .mk _ _ (.call .le _ [left, right] _) => return (← evaluate input left) ≤ (← evaluate input right)
    | .mk _ _ (.call .lt _ [left, right] _) => return (← evaluate input left) < (← evaluate input right)
    | _ => throw "unexpected generated condition"

private def widths : List (Xast.Ty × Nat × Bool) :=
  [(.u8, 8, false), (.u16, 16, false), (.u32, 32, false), (.u64, 64, false),
   (.u128, 128, false), (.u256, 256, false), (.i8, 8, true), (.i16, 16, true),
   (.i32, 32, true), (.i64, 64, true), (.i128, 128, true), (.i256, 256, true)]

-- Symbolic and literal conversions both agree with `BitVec`.
private def agreesWithBitVec (type : Xast.Ty) (bits : Nat) (signed : Bool) : Bool := Id.run do
  let edge : Int := 2 ^ bits
  let samples : List Int := [0, 1, -1, 7, 255, 256, -256, edge - 1, edge, edge + 1, -edge,
    -edge - 1, edge / 2, edge / 2 - 1, -(edge / 2), -(edge / 2) - 1, 3 * edge + 5, -3 * edge - 5]
  for input in samples do
    let expected := if signed then (BitVec.ofInt bits input).toInt else (BitVec.ofInt bits input).toNat
    let .ok symbolic := int2bv type symbol | return false
    unless (evaluate input symbolic).toOption == some expected do return false
    let .ok literal := int2bv type (.mk .num ⟨0, 0, 0⟩ (.value (.number input) none false))
      | return false
    unless (evaluate 0 literal).toOption == some expected do return false
    unless wrap bits signed input == expected do return false
  return true

#guard widths.all fun (type, bits, signed) => agreesWithBitVec type bits signed

-- A conversion has the width of its fixed-width result type; a `num` one wraps
-- at `u64`, and a type parameter has no width.
#guard [0, 1, -1, 2 ^ 64 - 1, 2 ^ 64, -(2 ^ 64), 5 * 2 ^ 64 + 3].all fun input =>
  match int2bv .num symbol with
  | .ok converted@(.mk .num _ _) =>
      (evaluate input converted).toOption == some ((BitVec.ofInt 64 input).toNat : Int)
  | _ => false
#guard (int2bv (.typeParam 0) symbol).toOption.isNone

-- A parameter of the result type is already in range.
#guard match int2bv .u8 (.mk .u8 ⟨0, 0, 0⟩ (.param 0)) with
  | .ok (.mk .u8 _ (.param 0)) => true
  | _ => false
