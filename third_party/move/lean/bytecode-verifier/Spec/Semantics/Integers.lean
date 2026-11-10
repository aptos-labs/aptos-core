-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

/-!
# Integer types and operations

TRUSTED. Move has unsigned integers `u8`–`u256` and signed integers
`i8`–`i256`. A value is a mathematical integer tagged with its type and kept in
the type's range. Every operation returns `none` exactly when the language
defines it to fail with an arithmetic error (catalogue entry R6): overflow,
division or remainder by zero, a shift by at least the width, a cast out of
range, or negating the minimum signed value.

Which operations apply to which types is part of the language: bitwise
operations and shifts apply to unsigned types only, `Negate` to signed types
only. Applying an operation to a type it does not apply to is not an
arithmetic error; the semantics treats it as `wrong`.

Language decision (R6, confirmed by the maintainers on 2026-10-02): signed
division and remainder both fail when the dividend is the minimum value and
the divisor is `-1`, for remainder too, although the mathematical remainder
`0` is representable. Both VMs agree. The Move book's correction
(`documentation/book/src/primitive-types.md`, "Arithmetic") lands
separately, on branch `move-book-signed-arithmetic`.
-/

namespace MoveBytecode

inductive IntType where
  | u8 | u16 | u32 | u64 | u128 | u256
  | i8 | i16 | i32 | i64 | i128 | i256
  deriving DecidableEq, Repr, Inhabited, Hashable

namespace IntType

def width : IntType → Nat
  | u8 | i8 => 8
  | u16 | i16 => 16
  | u32 | i32 => 32
  | u64 | i64 => 64
  | u128 | i128 => 128
  | u256 | i256 => 256

def isSigned : IntType → Bool
  | u8 | u16 | u32 | u64 | u128 | u256 => false
  | i8 | i16 | i32 | i64 | i128 | i256 => true

/-- The least value of the type. -/
def min (t : IntType) : Int :=
  if t.isSigned then -((2 : Int) ^ (t.width - 1)) else 0

/-- The greatest value of the type. -/
def max (t : IntType) : Int :=
  if t.isSigned then (2 : Int) ^ (t.width - 1) - 1 else (2 : Int) ^ t.width - 1

def inRange (t : IntType) (v : Int) : Bool :=
  decide (t.min ≤ v) && decide (v ≤ t.max)

/-- `v` if it is in range, otherwise the arithmetic failure `none`. -/
def checked (t : IntType) (v : Int) : Option Int :=
  if t.inRange v then some v else none

def add (t : IntType) (a b : Int) : Option Int := t.checked (a + b)

def sub (t : IntType) (a b : Int) : Option Int := t.checked (a - b)

def mul (t : IntType) (a b : Int) : Option Int := t.checked (a * b)

/-- Division rounding toward zero. -/
def div (t : IntType) (a b : Int) : Option Int :=
  if b = 0 then none else t.checked (a.tdiv b)

/-- Remainder with the sign of the dividend. Fails where division fails
(see the spec decision above). -/
def rem (t : IntType) (a b : Int) : Option Int :=
  if b = 0 then none
  else if (t.checked (a.tdiv b)).isNone then none
  else t.checked (a.tmod b)

/-- Negation, for signed types. -/
def negate (t : IntType) (a : Int) : Option Int := t.checked (-a)

/-- Bitwise operations, for unsigned types: both operands are in
`[0, 2^width)`, and so is the result. -/
def bitOr (a b : Int) : Int := Int.ofNat (a.toNat ||| b.toNat)

def bitAnd (a b : Int) : Int := Int.ofNat (a.toNat &&& b.toNat)

def bitXor (a b : Int) : Int := Int.ofNat (a.toNat ^^^ b.toNat)

/-- Left shift, for unsigned types: bits shifted beyond the width are lost.
Fails when the shift amount is at least the width. -/
def shl (t : IntType) (a : Int) (n : Nat) : Option Int :=
  if n < t.width then some (Int.ofNat ((a.toNat <<< n) % 2 ^ t.width)) else none

/-- Right shift, for unsigned types. Fails when the shift amount is at least
the width. -/
def shr (t : IntType) (a : Int) (n : Nat) : Option Int :=
  if n < t.width then some (Int.ofNat (a.toNat >>> n)) else none

/-- A cast to type `t`: the value is kept and must be in `t`'s range. -/
def cast (t : IntType) (a : Int) : Option Int := t.checked a

end IntType

/-- Interpret a bit vector literal of the given type. -/
def IntType.ofBitVec (t : IntType) {n : Nat} (v : BitVec n) : Int :=
  if t.isSigned then v.toInt else Int.ofNat v.toNat

end MoveBytecode
