-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
Defines a fixed-point numeric type with a 32-bit integer part and
a 32-bit fractional part.
-/
leaner module 0x1::fixed_point32 where
  pragma aborts_if_is_strict

  /--
  Define a fixed-point numeric type with 32 fractional bits.
  This is just a u64 integer but it is wrapped in a struct to
  make a unique type. This is a binary representation, so decimal
  values may not be exactly representable, but it provides more
  than 9 decimal digits of precision both before and after the
  decimal point (18 digits total). For comparison, double precision
  floating-point has less than 16 decimal digits of precision, so
  be careful about using floating-point to convert these values to
  decimal.
  -/
  struct FixedPoint32 has Copy, Drop, Store where
    value : u64

  const MAX_U64 : u128 := 18446744073709551615u128

  /--
  The denominator provided was zero
  -/
  const EDENOMINATOR : u64 := 65537

  /--
  The quotient value would be too large to be held in a `u64`
  -/
  const EDIVISION : u64 := 131074

  /--
  The multiplied value would be too large to be held in a `u64`
  -/
  const EMULTIPLICATION : u64 := 131075

  /--
  A division by zero was encountered
  -/
  const EDIVISION_BY_ZERO : u64 := 65540

  /--
  The computed ratio when converting to a `FixedPoint32` would be unrepresentable
  -/
  const ERATIO_OUT_OF_RANGE : u64 := 131077

  /--
  Multiply a u64 integer by a fixed-point number, truncating any
  fractional part of the product. This will abort if the product
  overflows.
  -/
  public fun multiply_u64(val : u64, multiplier : FixedPoint32) -> u64 := do
    let unscaled_product := val as u128 * (multiplier.value as u128)
    let product := unscaled_product >> 32u8
    assert!(product <= MAX_U64, EMULTIPLICATION)
    return product as u64

  spec multiply_u64 where
    pragma opaque
    aborts_if spec_multiply_u64(val, multiplier) > MAX_U64 with EMULTIPLICATION
    ensures result == spec_multiply_u64(val, multiplier)

  -- The product of two 64 bit values has 128 bits, so perform the
  -- multiplication with u128 types and keep the full 128 bit product
  -- to avoid losing accuracy.
  -- The unscaled product has 32 fractional bits (from the multiplier)
  -- so rescale it by shifting away the low bits.
  -- Check whether the value is too large.
  spec fun spec_multiply_u64(val : Int, multiplier : FixedPoint32) : Int :=
    val * multiplier.value >> 32

  /--
  Divide a u64 integer by a fixed-point number, truncating any
  fractional part of the quotient. This will abort if the divisor
  is zero or if the quotient overflows.
  -/
  public fun divide_u64(val : u64, divisor : FixedPoint32) -> u64 := do
    assert!(divisor.value != 0, EDIVISION_BY_ZERO)
    let scaled_value := val as u128 << 32u8
    let quotient := scaled_value / (divisor.value as u128)
    assert!(quotient <= MAX_U64, EDIVISION)
    return quotient as u64

  spec divide_u64 where
    pragma opaque
    aborts_if divisor.value == 0 with EDIVISION_BY_ZERO
    aborts_if spec_divide_u64(val, divisor) > MAX_U64 with EDIVISION
    ensures result == spec_divide_u64(val, divisor)

  -- Check for division by zero.
  -- First convert to 128 bits and then shift left to
  -- add 32 fractional zero bits to the dividend.
  -- Check whether the value is too large.
  -- the value may be too large, which will cause the cast to fail
  -- with an arithmetic error.
  spec fun spec_divide_u64(val : Int, divisor : FixedPoint32) : Int :=
    (val << 32) / divisor.value

  /--
  Create a fixed-point value from a rational number specified by its
  numerator and denominator. Calling this function should be preferred
  for using `Self::create_from_raw_value` which is also available.
  This will abort if the denominator is zero. It will also
  abort if the numerator is nonzero and the ratio is not in the range
  2^-32 .. 2^32-1. When specifying decimal fractions, be careful about
  rounding errors: if you round to display N digits after the decimal
  point, you can use a denominator of 10^N to avoid numbers where the
  very small imprecision in the binary representation could change the
  rounding, e.g., 0.0125 will round down to 0.012 instead of up to 0.013.
  -/
  public fun create_from_rational(
    numerator : u64, denominator : u64
  ) -> FixedPoint32 := do
    let scaled_numerator := numerator as u128 << 64u8
    let scaled_denominator := denominator as u128 << 32u8
    assert!(scaled_denominator != 0u128, EDENOMINATOR)
    let quotient := scaled_numerator / scaled_denominator
    assert!(quotient != 0u128 || numerator == 0, ERATIO_OUT_OF_RANGE)
    assert!(quotient <= MAX_U64, ERATIO_OUT_OF_RANGE)
    return new FixedPoint32 { value := quotient as u64 }

  spec create_from_rational where
    pragma opaque
    let_pre scaled_numerator := numerator << 64
    let_pre scaled_denominator := denominator << 32
    let_pre quotient := scaled_numerator / scaled_denominator
    aborts_if scaled_denominator == 0 with EDENOMINATOR
    aborts_if quotient == 0 && scaled_numerator != 0 with ERATIO_OUT_OF_RANGE
    aborts_if quotient > MAX_U64 with ERATIO_OUT_OF_RANGE
    ensures result == spec_create_from_rational(numerator, denominator)

  -- If the denominator is zero, this will abort.
  -- Scale the numerator to have 64 fractional bits and the denominator
  -- to have 32 fractional bits, so that the quotient will have 32
  -- fractional bits.
  -- Return the quotient as a fixed-point number. We first need to check whether the cast
  -- can succeed.
  spec fun spec_create_from_rational(
    numerator : Int, denominator : Int
  ) : FixedPoint32 :=
    new FixedPoint32 { value := (numerator << 64) / (denominator << 32) }

  /--
  Create a fixedpoint value from a raw value.
  -/
  public fun create_from_raw_value(value : u64) -> FixedPoint32 :=
    new FixedPoint32 { value }

  spec create_from_raw_value where
    pragma opaque
    aborts_if false
    ensures result.value == value

  /--
  Accessor for the raw u64 value. Other less common operations, such as
  adding or subtracting FixedPoint32 values, can be done using the raw
  values directly.
  -/
  public fun get_raw_value(self : FixedPoint32) -> u64 := self.value

  /--
  Returns true if the ratio is zero.
  -/
  public fun is_zero(self : FixedPoint32) -> Bool := self.value == 0

  /--
  Returns the smaller of the two FixedPoint32 numbers.
  -/
  public fun min(num1 : FixedPoint32, num2 : FixedPoint32) -> FixedPoint32 :=
    if num1.value < num2.value then num1 else num2

  spec min where
    pragma opaque
    aborts_if false
    ensures result == spec_min(num1, num2)

  spec fun spec_min(num1 : FixedPoint32, num2 : FixedPoint32) : FixedPoint32 :=
    if num1.value < num2.value then num1 else num2

  /--
  Returns the larger of the two FixedPoint32 numbers.
  -/
  public fun max(num1 : FixedPoint32, num2 : FixedPoint32) -> FixedPoint32 :=
    if num1.value > num2.value then num1 else num2

  spec max where
    pragma opaque
    aborts_if false
    ensures result == spec_max(num1, num2)

  spec fun spec_max(num1 : FixedPoint32, num2 : FixedPoint32) : FixedPoint32 :=
    if num1.value > num2.value then num1 else num2

  /--
  Create a fixedpoint value from a u64 value.
  -/
  public fun create_from_u64(val : u64) -> FixedPoint32 := do
    let value := val as u128 << 32u8
    assert!(value <= MAX_U64, ERATIO_OUT_OF_RANGE)
    return new FixedPoint32 { value := value as u64 }

  spec create_from_u64 where
    pragma opaque
    let_pre scaled_value := val << 32
    aborts_if scaled_value > MAX_U64
    ensures result == spec_create_from_u64(val)

  spec fun spec_create_from_u64(val : Int) : FixedPoint32 :=
    new FixedPoint32 { value := val << 32 }

  /--
  Returns the largest integer less than or equal to a given number.
  -/
  public fun floor(self : FixedPoint32) -> u64 := self.value >> 32u8

  spec floor where
    pragma opaque
    aborts_if false
    ensures result == spec_floor(self)

  spec fun spec_floor(self : FixedPoint32) : Int := self.value >> 32

  -- Right-shifting discards the lower 32 bits unconditionally;
  -- the original conditional was redundant since both branches equal self.value >> 32.
  /--
  Rounds up the given FixedPoint32 to the next largest integer.
  -/
  public fun ceil(self : FixedPoint32) -> u64 := do
    let floored_num := self.floor() << 32u8
    if self.value == floored_num then return floored_num >> 32u8;
    let val := floored_num as u128 + (1u128 << 32u8)
    return (val >> 32u8) as u64

  spec ceil where
    pragma opaque
    aborts_if false
    ensures result == spec_ceil(self)

  spec fun spec_ceil(self : FixedPoint32) : Int := do
    let floor_val := self.value >> 32
    return if self.value == floor_val << 32 then floor_val else floor_val + 1

  -- Expressed in terms of floor_val to avoid modulo: the else branch
  -- (self.value - fractional + 2^32) >> 32 = floor_val + 1, and
  -- fractional == 0 iff self.value == floor_val << 32.
  /--
  Returns the value of a FixedPoint32 to the nearest integer.
  -/
  public fun round(self : FixedPoint32) -> u64 := do
    let floored_num := self.floor() << 32u8
    let boundary := floored_num + (1 << 32u8) / 2
    return if self.value < boundary then floored_num >> 32u8 else self.ceil()

  spec round where
    pragma opaque
    aborts_if false
    ensures result == spec_round(self)

  spec fun spec_round(self : FixedPoint32) : Int := do
    let floor_val := self.value >> 32
    return if self.value < (floor_val << 32) + (1 << 31) then floor_val
    else floor_val + 1

  -- Expressed in terms of floor_val to avoid modulo: both result branches
  -- equal floor_val or floor_val + 1, and the boundary condition
  -- fractional < 2^31 is equivalent to self.value < floor_val<<32 + 2^31.
  -- **************** SPECIFICATIONS ****************
  -- switch documentation context to module level
