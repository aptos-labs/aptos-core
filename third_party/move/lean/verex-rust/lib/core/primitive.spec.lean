-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A pointer-width integer has 64 bits, as the rendering's profile states.
mod primitive where
  impl «u8» where
    spec fun pow(x : Int, n : Int) : Int decreases n :=
      if n <= 0 then 1 else x * pow(x, n - 1)

    spec fun bits(x : Int, width : Int) : Int decreases width :=
      if width <= 0 then 0 else x % 2 + bits(x / 2, width - 1)

    spec fun count_ones(x : Int) : Int := bits(x, 8)

    spec pow where
      panics_if pow(_1, _2) > MAX_U8
      ensures result == pow(_1, _2)

    spec count_ones where
      ensures result == count_ones(_1)

    spec checked_div where
      ensures _2 != 0 ==> result is Some && result.0 == _1 / _2
      ensures _2 == 0 ==> result is None

    spec rem_euclid where
      panics_if _2 == 0
      ensures result == _1 % _2

    spec is_power_of_two where
      ensures result == (∃ (k in 0 .. 8), _1 == (1 << k))

    spec leading_zeros where
      ensures _1 == 0 ==> result == 8
      ensures _1 > 0 ==> (1 << (8 - 1 - result)) <= _1 && _1 < (1 << (8 - result))

    spec trailing_zeros where
      ensures _1 == 0 ==> result == 8
      ensures _1 != 0 ==> _1 % (1 << result) == 0 && _1 % (1 << (result + 1)) != 0

    spec checked_add where
      ensures 0 <= _1 + _2 && _1 + _2 <= MAX_U8 ==> result is Some && result.0 == _1 + _2
      ensures !(0 <= _1 + _2 && _1 + _2 <= MAX_U8) ==> result is None

    spec checked_sub where
      ensures 0 <= _1 - _2 && _1 - _2 <= MAX_U8 ==> result is Some && result.0 == _1 - _2
      ensures !(0 <= _1 - _2 && _1 - _2 <= MAX_U8) ==> result is None

    spec checked_mul where
      ensures 0 <= _1 * _2 && _1 * _2 <= MAX_U8 ==> result is Some && result.0 == _1 * _2
      ensures !(0 <= _1 * _2 && _1 * _2 <= MAX_U8) ==> result is None

    spec saturating_add where
      ensures result == (if _1 + _2 > MAX_U8 then MAX_U8 else if _1 + _2 < 0 then 0 else _1 + _2)

    spec saturating_sub where
      ensures result == (if _1 - _2 > MAX_U8 then MAX_U8 else if _1 - _2 < 0 then 0 else _1 - _2)

    spec wrapping_shl where
      ensures result == (_1 << (_2 % 8)) % (MAX_U8 + 1)

    spec wrapping_shr where
      ensures result == _1 >> (_2 % 8)

  impl «u16» where
    spec fun pow(x : Int, n : Int) : Int decreases n :=
      if n <= 0 then 1 else x * pow(x, n - 1)

    spec fun bits(x : Int, width : Int) : Int decreases width :=
      if width <= 0 then 0 else x % 2 + bits(x / 2, width - 1)

    spec fun count_ones(x : Int) : Int := bits(x, 16)

    spec pow where
      panics_if pow(_1, _2) > MAX_U16
      ensures result == pow(_1, _2)

    spec count_ones where
      ensures result == count_ones(_1)

    spec checked_div where
      ensures _2 != 0 ==> result is Some && result.0 == _1 / _2
      ensures _2 == 0 ==> result is None

    spec rem_euclid where
      panics_if _2 == 0
      ensures result == _1 % _2

    spec is_power_of_two where
      ensures result == (∃ (k in 0 .. 16), _1 == (1 << k))

    spec leading_zeros where
      ensures _1 == 0 ==> result == 16
      ensures _1 > 0 ==> (1 << (16 - 1 - result)) <= _1 && _1 < (1 << (16 - result))

    spec trailing_zeros where
      ensures _1 == 0 ==> result == 16
      ensures _1 != 0 ==> _1 % (1 << result) == 0 && _1 % (1 << (result + 1)) != 0

    spec checked_add where
      ensures 0 <= _1 + _2 && _1 + _2 <= MAX_U16 ==> result is Some && result.0 == _1 + _2
      ensures !(0 <= _1 + _2 && _1 + _2 <= MAX_U16) ==> result is None

    spec checked_sub where
      ensures 0 <= _1 - _2 && _1 - _2 <= MAX_U16 ==> result is Some && result.0 == _1 - _2
      ensures !(0 <= _1 - _2 && _1 - _2 <= MAX_U16) ==> result is None

    spec checked_mul where
      ensures 0 <= _1 * _2 && _1 * _2 <= MAX_U16 ==> result is Some && result.0 == _1 * _2
      ensures !(0 <= _1 * _2 && _1 * _2 <= MAX_U16) ==> result is None

    spec saturating_add where
      ensures result == (if _1 + _2 > MAX_U16 then MAX_U16 else if _1 + _2 < 0 then 0 else _1 + _2)

    spec saturating_sub where
      ensures result == (if _1 - _2 > MAX_U16 then MAX_U16 else if _1 - _2 < 0 then 0 else _1 - _2)

    spec wrapping_shl where
      ensures result == (_1 << (_2 % 16)) % (MAX_U16 + 1)

    spec wrapping_shr where
      ensures result == _1 >> (_2 % 16)

  impl «u32» where
    spec fun pow(x : Int, n : Int) : Int decreases n :=
      if n <= 0 then 1 else x * pow(x, n - 1)

    spec fun bits(x : Int, width : Int) : Int decreases width :=
      if width <= 0 then 0 else x % 2 + bits(x / 2, width - 1)

    spec fun count_ones(x : Int) : Int := bits(x, 32)

    spec pow where
      panics_if pow(_1, _2) > MAX_U32
      ensures result == pow(_1, _2)

    spec count_ones where
      ensures result == count_ones(_1)

    spec checked_div where
      ensures _2 != 0 ==> result is Some && result.0 == _1 / _2
      ensures _2 == 0 ==> result is None

    spec rem_euclid where
      panics_if _2 == 0
      ensures result == _1 % _2

    spec is_power_of_two where
      ensures result == (∃ (k in 0 .. 32), _1 == (1 << k))

    spec leading_zeros where
      ensures _1 == 0 ==> result == 32
      ensures _1 > 0 ==> (1 << (32 - 1 - result)) <= _1 && _1 < (1 << (32 - result))

    spec trailing_zeros where
      ensures _1 == 0 ==> result == 32
      ensures _1 != 0 ==> _1 % (1 << result) == 0 && _1 % (1 << (result + 1)) != 0

    spec checked_add where
      ensures 0 <= _1 + _2 && _1 + _2 <= MAX_U32 ==> result is Some && result.0 == _1 + _2
      ensures !(0 <= _1 + _2 && _1 + _2 <= MAX_U32) ==> result is None

    spec checked_sub where
      ensures 0 <= _1 - _2 && _1 - _2 <= MAX_U32 ==> result is Some && result.0 == _1 - _2
      ensures !(0 <= _1 - _2 && _1 - _2 <= MAX_U32) ==> result is None

    spec checked_mul where
      ensures 0 <= _1 * _2 && _1 * _2 <= MAX_U32 ==> result is Some && result.0 == _1 * _2
      ensures !(0 <= _1 * _2 && _1 * _2 <= MAX_U32) ==> result is None

    spec saturating_add where
      ensures result == (if _1 + _2 > MAX_U32 then MAX_U32 else if _1 + _2 < 0 then 0 else _1 + _2)

    spec saturating_sub where
      ensures result == (if _1 - _2 > MAX_U32 then MAX_U32 else if _1 - _2 < 0 then 0 else _1 - _2)

    spec wrapping_shl where
      ensures result == (_1 << (_2 % 32)) % (MAX_U32 + 1)

    spec wrapping_shr where
      ensures result == _1 >> (_2 % 32)

  impl «u64» where
    spec fun pow(x : Int, n : Int) : Int decreases n :=
      if n <= 0 then 1 else x * pow(x, n - 1)

    spec fun bits(x : Int, width : Int) : Int decreases width :=
      if width <= 0 then 0 else x % 2 + bits(x / 2, width - 1)

    spec fun count_ones(x : Int) : Int := bits(x, 64)

    spec pow where
      panics_if pow(_1, _2) > MAX_U64
      ensures result == pow(_1, _2)

    spec count_ones where
      ensures result == count_ones(_1)

    spec checked_div where
      ensures _2 != 0 ==> result is Some && result.0 == _1 / _2
      ensures _2 == 0 ==> result is None

    spec rem_euclid where
      panics_if _2 == 0
      ensures result == _1 % _2

    spec is_power_of_two where
      ensures result == (∃ (k in 0 .. 64), _1 == (1 << k))

    spec leading_zeros where
      ensures _1 == 0 ==> result == 64
      ensures _1 > 0 ==> (1 << (64 - 1 - result)) <= _1 && _1 < (1 << (64 - result))

    spec trailing_zeros where
      ensures _1 == 0 ==> result == 64
      ensures _1 != 0 ==> _1 % (1 << result) == 0 && _1 % (1 << (result + 1)) != 0

    spec checked_add where
      ensures 0 <= _1 + _2 && _1 + _2 <= MAX_U64 ==> result is Some && result.0 == _1 + _2
      ensures !(0 <= _1 + _2 && _1 + _2 <= MAX_U64) ==> result is None

    spec checked_sub where
      ensures 0 <= _1 - _2 && _1 - _2 <= MAX_U64 ==> result is Some && result.0 == _1 - _2
      ensures !(0 <= _1 - _2 && _1 - _2 <= MAX_U64) ==> result is None

    spec checked_mul where
      ensures 0 <= _1 * _2 && _1 * _2 <= MAX_U64 ==> result is Some && result.0 == _1 * _2
      ensures !(0 <= _1 * _2 && _1 * _2 <= MAX_U64) ==> result is None

    spec saturating_add where
      ensures result == (if _1 + _2 > MAX_U64 then MAX_U64 else if _1 + _2 < 0 then 0 else _1 + _2)

    spec saturating_sub where
      ensures result == (if _1 - _2 > MAX_U64 then MAX_U64 else if _1 - _2 < 0 then 0 else _1 - _2)

    spec wrapping_shl where
      ensures result == (_1 << (_2 % 64)) % (MAX_U64 + 1)

    spec wrapping_shr where
      ensures result == _1 >> (_2 % 64)

  impl «u128» where
    spec fun pow(x : Int, n : Int) : Int decreases n :=
      if n <= 0 then 1 else x * pow(x, n - 1)

    spec fun bits(x : Int, width : Int) : Int decreases width :=
      if width <= 0 then 0 else x % 2 + bits(x / 2, width - 1)

    spec fun count_ones(x : Int) : Int := bits(x, 128)

    spec pow where
      panics_if pow(_1, _2) > MAX_U128
      ensures result == pow(_1, _2)

    spec count_ones where
      ensures result == count_ones(_1)

    spec checked_div where
      ensures _2 != 0 ==> result is Some && result.0 == _1 / _2
      ensures _2 == 0 ==> result is None

    spec rem_euclid where
      panics_if _2 == 0
      ensures result == _1 % _2

    spec is_power_of_two where
      ensures result == (∃ (k in 0 .. 128), _1 == (1 << k))

    spec leading_zeros where
      ensures _1 == 0 ==> result == 128
      ensures _1 > 0 ==> (1 << (128 - 1 - result)) <= _1 && _1 < (1 << (128 - result))

    spec trailing_zeros where
      ensures _1 == 0 ==> result == 128
      ensures _1 != 0 ==> _1 % (1 << result) == 0 && _1 % (1 << (result + 1)) != 0

    spec checked_add where
      ensures 0 <= _1 + _2 && _1 + _2 <= MAX_U128 ==> result is Some && result.0 == _1 + _2
      ensures !(0 <= _1 + _2 && _1 + _2 <= MAX_U128) ==> result is None

    spec checked_sub where
      ensures 0 <= _1 - _2 && _1 - _2 <= MAX_U128 ==> result is Some && result.0 == _1 - _2
      ensures !(0 <= _1 - _2 && _1 - _2 <= MAX_U128) ==> result is None

    spec checked_mul where
      ensures 0 <= _1 * _2 && _1 * _2 <= MAX_U128 ==> result is Some && result.0 == _1 * _2
      ensures !(0 <= _1 * _2 && _1 * _2 <= MAX_U128) ==> result is None

    spec saturating_add where
      ensures result == (if _1 + _2 > MAX_U128 then MAX_U128 else if _1 + _2 < 0 then 0 else _1 + _2)

    spec saturating_sub where
      ensures result == (if _1 - _2 > MAX_U128 then MAX_U128 else if _1 - _2 < 0 then 0 else _1 - _2)

    spec wrapping_shl where
      ensures result == (_1 << (_2 % 128)) % (MAX_U128 + 1)

    spec wrapping_shr where
      ensures result == _1 >> (_2 % 128)

  impl «usize» where
    spec fun pow(x : Int, n : Int) : Int decreases n :=
      if n <= 0 then 1 else x * pow(x, n - 1)

    spec fun bits(x : Int, width : Int) : Int decreases width :=
      if width <= 0 then 0 else x % 2 + bits(x / 2, width - 1)

    spec fun count_ones(x : Int) : Int := bits(x, 64)

    spec pow where
      panics_if pow(_1, _2) > MAX_U64
      ensures result == pow(_1, _2)

    spec count_ones where
      ensures result == count_ones(_1)

    spec checked_div where
      ensures _2 != 0 ==> result is Some && result.0 == _1 / _2
      ensures _2 == 0 ==> result is None

    spec rem_euclid where
      panics_if _2 == 0
      ensures result == _1 % _2

    spec is_power_of_two where
      ensures result == (∃ (k in 0 .. 64), _1 == (1 << k))

    spec leading_zeros where
      ensures _1 == 0 ==> result == 64
      ensures _1 > 0 ==> (1 << (64 - 1 - result)) <= _1 && _1 < (1 << (64 - result))

    spec trailing_zeros where
      ensures _1 == 0 ==> result == 64
      ensures _1 != 0 ==> _1 % (1 << result) == 0 && _1 % (1 << (result + 1)) != 0

    spec checked_add where
      ensures 0 <= _1 + _2 && _1 + _2 <= MAX_U64 ==> result is Some && result.0 == _1 + _2
      ensures !(0 <= _1 + _2 && _1 + _2 <= MAX_U64) ==> result is None

    spec checked_sub where
      ensures 0 <= _1 - _2 && _1 - _2 <= MAX_U64 ==> result is Some && result.0 == _1 - _2
      ensures !(0 <= _1 - _2 && _1 - _2 <= MAX_U64) ==> result is None

    spec checked_mul where
      ensures 0 <= _1 * _2 && _1 * _2 <= MAX_U64 ==> result is Some && result.0 == _1 * _2
      ensures !(0 <= _1 * _2 && _1 * _2 <= MAX_U64) ==> result is None

    spec saturating_add where
      ensures result == (if _1 + _2 > MAX_U64 then MAX_U64 else if _1 + _2 < 0 then 0 else _1 + _2)

    spec saturating_sub where
      ensures result == (if _1 - _2 > MAX_U64 then MAX_U64 else if _1 - _2 < 0 then 0 else _1 - _2)

    spec wrapping_shl where
      ensures result == (_1 << (_2 % 64)) % (MAX_U64 + 1)

    spec wrapping_shr where
      ensures result == _1 >> (_2 % 64)

  impl «i8» where
    spec fun pow(x : Int, n : Int) : Int decreases n :=
      if n <= 0 then 1 else x * pow(x, n - 1)

    spec fun bits(x : Int, width : Int) : Int decreases width :=
      if width <= 0 then 0 else x % 2 + bits(x / 2, width - 1)

    -- The bits of a negative value are its two's complement.
    spec fun count_ones(x : Int) : Int :=
      bits(if 0 > x then x + 256 else x, 8)

    spec pow where
      panics_if pow(_1, _2) < MIN_I8 || pow(_1, _2) > MAX_I8
      ensures result == pow(_1, _2)

    spec count_ones where
      ensures result == count_ones(_1)

    spec abs where
      panics_if _1 == MIN_I8
      ensures result == (if _1 < 0 then -_1 else _1)

    spec checked_div where
      ensures _2 != 0 && !(_1 == MIN_I8 && _2 == -1) ==> result is Some && result.0 == _1 / _2
      ensures _2 == 0 || (_1 == MIN_I8 && _2 == -1) ==> result is None

    spec rem_euclid where
      panics_if _2 == 0 || (_1 == MIN_I8 && _2 == -1)
      ensures _1 % _2 >= 0 ==> result == _1 % _2
      ensures _1 % _2 < 0 && _2 > 0 ==> result == _1 % _2 + _2
      ensures _1 % _2 < 0 && _2 < 0 ==> result == _1 % _2 - _2

    spec leading_zeros where
      ensures _1 < 0 ==> result == 0
      ensures _1 == 0 ==> result == 8
      ensures _1 > 0 ==> (1 << (8 - 1 - result)) <= _1 && _1 < (1 << (8 - result))

    spec trailing_zeros where
      ensures _1 == 0 ==> result == 8
      ensures _1 != 0 ==> _1 % (1 << result) == 0 && _1 % (1 << (result + 1)) != 0

    spec checked_add where
      ensures MIN_I8 <= _1 + _2 && _1 + _2 <= MAX_I8 ==> result is Some && result.0 == _1 + _2
      ensures !(MIN_I8 <= _1 + _2 && _1 + _2 <= MAX_I8) ==> result is None

    spec checked_sub where
      ensures MIN_I8 <= _1 - _2 && _1 - _2 <= MAX_I8 ==> result is Some && result.0 == _1 - _2
      ensures !(MIN_I8 <= _1 - _2 && _1 - _2 <= MAX_I8) ==> result is None

    spec checked_mul where
      ensures MIN_I8 <= _1 * _2 && _1 * _2 <= MAX_I8 ==> result is Some && result.0 == _1 * _2
      ensures !(MIN_I8 <= _1 * _2 && _1 * _2 <= MAX_I8) ==> result is None

    spec saturating_add where
      ensures result == (if _1 + _2 > MAX_I8 then MAX_I8 else if _1 + _2 < MIN_I8 then MIN_I8 else _1 + _2)

    spec saturating_sub where
      ensures result == (if _1 - _2 > MAX_I8 then MAX_I8 else if _1 - _2 < MIN_I8 then MIN_I8 else _1 - _2)

    spec wrapping_shl where
      ensures result == (((_1 << (_2 % 8)) - MIN_I8) % (MAX_I8 - MIN_I8 + 1)) + MIN_I8

    spec wrapping_shr where
      ensures result == _1 >> (_2 % 8)

  impl «i16» where
    spec fun pow(x : Int, n : Int) : Int decreases n :=
      if n <= 0 then 1 else x * pow(x, n - 1)

    spec fun bits(x : Int, width : Int) : Int decreases width :=
      if width <= 0 then 0 else x % 2 + bits(x / 2, width - 1)

    -- The bits of a negative value are its two's complement.
    spec fun count_ones(x : Int) : Int :=
      bits(if 0 > x then x + 65536 else x, 16)

    spec pow where
      panics_if pow(_1, _2) < MIN_I16 || pow(_1, _2) > MAX_I16
      ensures result == pow(_1, _2)

    spec count_ones where
      ensures result == count_ones(_1)

    spec abs where
      panics_if _1 == MIN_I16
      ensures result == (if _1 < 0 then -_1 else _1)

    spec checked_div where
      ensures _2 != 0 && !(_1 == MIN_I16 && _2 == -1) ==> result is Some && result.0 == _1 / _2
      ensures _2 == 0 || (_1 == MIN_I16 && _2 == -1) ==> result is None

    spec rem_euclid where
      panics_if _2 == 0 || (_1 == MIN_I16 && _2 == -1)
      ensures _1 % _2 >= 0 ==> result == _1 % _2
      ensures _1 % _2 < 0 && _2 > 0 ==> result == _1 % _2 + _2
      ensures _1 % _2 < 0 && _2 < 0 ==> result == _1 % _2 - _2

    spec leading_zeros where
      ensures _1 < 0 ==> result == 0
      ensures _1 == 0 ==> result == 16
      ensures _1 > 0 ==> (1 << (16 - 1 - result)) <= _1 && _1 < (1 << (16 - result))

    spec trailing_zeros where
      ensures _1 == 0 ==> result == 16
      ensures _1 != 0 ==> _1 % (1 << result) == 0 && _1 % (1 << (result + 1)) != 0

    spec checked_add where
      ensures MIN_I16 <= _1 + _2 && _1 + _2 <= MAX_I16 ==> result is Some && result.0 == _1 + _2
      ensures !(MIN_I16 <= _1 + _2 && _1 + _2 <= MAX_I16) ==> result is None

    spec checked_sub where
      ensures MIN_I16 <= _1 - _2 && _1 - _2 <= MAX_I16 ==> result is Some && result.0 == _1 - _2
      ensures !(MIN_I16 <= _1 - _2 && _1 - _2 <= MAX_I16) ==> result is None

    spec checked_mul where
      ensures MIN_I16 <= _1 * _2 && _1 * _2 <= MAX_I16 ==> result is Some && result.0 == _1 * _2
      ensures !(MIN_I16 <= _1 * _2 && _1 * _2 <= MAX_I16) ==> result is None

    spec saturating_add where
      ensures result == (if _1 + _2 > MAX_I16 then MAX_I16 else if _1 + _2 < MIN_I16 then MIN_I16 else _1 + _2)

    spec saturating_sub where
      ensures result == (if _1 - _2 > MAX_I16 then MAX_I16 else if _1 - _2 < MIN_I16 then MIN_I16 else _1 - _2)

    spec wrapping_shl where
      ensures result == (((_1 << (_2 % 16)) - MIN_I16) % (MAX_I16 - MIN_I16 + 1)) + MIN_I16

    spec wrapping_shr where
      ensures result == _1 >> (_2 % 16)

  impl «i32» where
    spec fun pow(x : Int, n : Int) : Int decreases n :=
      if n <= 0 then 1 else x * pow(x, n - 1)

    spec fun bits(x : Int, width : Int) : Int decreases width :=
      if width <= 0 then 0 else x % 2 + bits(x / 2, width - 1)

    -- The bits of a negative value are its two's complement.
    spec fun count_ones(x : Int) : Int :=
      bits(if 0 > x then x + 4294967296 else x, 32)

    spec pow where
      panics_if pow(_1, _2) < MIN_I32 || pow(_1, _2) > MAX_I32
      ensures result == pow(_1, _2)

    spec count_ones where
      ensures result == count_ones(_1)

    spec abs where
      panics_if _1 == MIN_I32
      ensures result == (if _1 < 0 then -_1 else _1)

    spec checked_div where
      ensures _2 != 0 && !(_1 == MIN_I32 && _2 == -1) ==> result is Some && result.0 == _1 / _2
      ensures _2 == 0 || (_1 == MIN_I32 && _2 == -1) ==> result is None

    spec rem_euclid where
      panics_if _2 == 0 || (_1 == MIN_I32 && _2 == -1)
      ensures _1 % _2 >= 0 ==> result == _1 % _2
      ensures _1 % _2 < 0 && _2 > 0 ==> result == _1 % _2 + _2
      ensures _1 % _2 < 0 && _2 < 0 ==> result == _1 % _2 - _2

    spec leading_zeros where
      ensures _1 < 0 ==> result == 0
      ensures _1 == 0 ==> result == 32
      ensures _1 > 0 ==> (1 << (32 - 1 - result)) <= _1 && _1 < (1 << (32 - result))

    spec trailing_zeros where
      ensures _1 == 0 ==> result == 32
      ensures _1 != 0 ==> _1 % (1 << result) == 0 && _1 % (1 << (result + 1)) != 0

    spec checked_add where
      ensures MIN_I32 <= _1 + _2 && _1 + _2 <= MAX_I32 ==> result is Some && result.0 == _1 + _2
      ensures !(MIN_I32 <= _1 + _2 && _1 + _2 <= MAX_I32) ==> result is None

    spec checked_sub where
      ensures MIN_I32 <= _1 - _2 && _1 - _2 <= MAX_I32 ==> result is Some && result.0 == _1 - _2
      ensures !(MIN_I32 <= _1 - _2 && _1 - _2 <= MAX_I32) ==> result is None

    spec checked_mul where
      ensures MIN_I32 <= _1 * _2 && _1 * _2 <= MAX_I32 ==> result is Some && result.0 == _1 * _2
      ensures !(MIN_I32 <= _1 * _2 && _1 * _2 <= MAX_I32) ==> result is None

    spec saturating_add where
      ensures result == (if _1 + _2 > MAX_I32 then MAX_I32 else if _1 + _2 < MIN_I32 then MIN_I32 else _1 + _2)

    spec saturating_sub where
      ensures result == (if _1 - _2 > MAX_I32 then MAX_I32 else if _1 - _2 < MIN_I32 then MIN_I32 else _1 - _2)

    spec wrapping_shl where
      ensures result == (((_1 << (_2 % 32)) - MIN_I32) % (MAX_I32 - MIN_I32 + 1)) + MIN_I32

    spec wrapping_shr where
      ensures result == _1 >> (_2 % 32)

  impl «i64» where
    spec fun pow(x : Int, n : Int) : Int decreases n :=
      if n <= 0 then 1 else x * pow(x, n - 1)

    spec fun bits(x : Int, width : Int) : Int decreases width :=
      if width <= 0 then 0 else x % 2 + bits(x / 2, width - 1)

    -- The bits of a negative value are its two's complement.
    spec fun count_ones(x : Int) : Int :=
      bits(if 0 > x then x + 18446744073709551616 else x, 64)

    spec pow where
      panics_if pow(_1, _2) < MIN_I64 || pow(_1, _2) > MAX_I64
      ensures result == pow(_1, _2)

    spec count_ones where
      ensures result == count_ones(_1)

    spec abs where
      panics_if _1 == MIN_I64
      ensures result == (if _1 < 0 then -_1 else _1)

    spec checked_div where
      ensures _2 != 0 && !(_1 == MIN_I64 && _2 == -1) ==> result is Some && result.0 == _1 / _2
      ensures _2 == 0 || (_1 == MIN_I64 && _2 == -1) ==> result is None

    spec rem_euclid where
      panics_if _2 == 0 || (_1 == MIN_I64 && _2 == -1)
      ensures _1 % _2 >= 0 ==> result == _1 % _2
      ensures _1 % _2 < 0 && _2 > 0 ==> result == _1 % _2 + _2
      ensures _1 % _2 < 0 && _2 < 0 ==> result == _1 % _2 - _2

    spec leading_zeros where
      ensures _1 < 0 ==> result == 0
      ensures _1 == 0 ==> result == 64
      ensures _1 > 0 ==> (1 << (64 - 1 - result)) <= _1 && _1 < (1 << (64 - result))

    spec trailing_zeros where
      ensures _1 == 0 ==> result == 64
      ensures _1 != 0 ==> _1 % (1 << result) == 0 && _1 % (1 << (result + 1)) != 0

    spec checked_add where
      ensures MIN_I64 <= _1 + _2 && _1 + _2 <= MAX_I64 ==> result is Some && result.0 == _1 + _2
      ensures !(MIN_I64 <= _1 + _2 && _1 + _2 <= MAX_I64) ==> result is None

    spec checked_sub where
      ensures MIN_I64 <= _1 - _2 && _1 - _2 <= MAX_I64 ==> result is Some && result.0 == _1 - _2
      ensures !(MIN_I64 <= _1 - _2 && _1 - _2 <= MAX_I64) ==> result is None

    spec checked_mul where
      ensures MIN_I64 <= _1 * _2 && _1 * _2 <= MAX_I64 ==> result is Some && result.0 == _1 * _2
      ensures !(MIN_I64 <= _1 * _2 && _1 * _2 <= MAX_I64) ==> result is None

    spec saturating_add where
      ensures result == (if _1 + _2 > MAX_I64 then MAX_I64 else if _1 + _2 < MIN_I64 then MIN_I64 else _1 + _2)

    spec saturating_sub where
      ensures result == (if _1 - _2 > MAX_I64 then MAX_I64 else if _1 - _2 < MIN_I64 then MIN_I64 else _1 - _2)

    spec wrapping_shl where
      ensures result == (((_1 << (_2 % 64)) - MIN_I64) % (MAX_I64 - MIN_I64 + 1)) + MIN_I64

    spec wrapping_shr where
      ensures result == _1 >> (_2 % 64)

  impl «i128» where
    spec fun pow(x : Int, n : Int) : Int decreases n :=
      if n <= 0 then 1 else x * pow(x, n - 1)

    spec fun bits(x : Int, width : Int) : Int decreases width :=
      if width <= 0 then 0 else x % 2 + bits(x / 2, width - 1)

    -- The bits of a negative value are its two's complement.
    spec fun count_ones(x : Int) : Int :=
      bits(if 0 > x then x + 340282366920938463463374607431768211456 else x, 128)

    spec pow where
      panics_if pow(_1, _2) < MIN_I128 || pow(_1, _2) > MAX_I128
      ensures result == pow(_1, _2)

    spec count_ones where
      ensures result == count_ones(_1)

    spec abs where
      panics_if _1 == MIN_I128
      ensures result == (if _1 < 0 then -_1 else _1)

    spec checked_div where
      ensures _2 != 0 && !(_1 == MIN_I128 && _2 == -1) ==> result is Some && result.0 == _1 / _2
      ensures _2 == 0 || (_1 == MIN_I128 && _2 == -1) ==> result is None

    spec rem_euclid where
      panics_if _2 == 0 || (_1 == MIN_I128 && _2 == -1)
      ensures _1 % _2 >= 0 ==> result == _1 % _2
      ensures _1 % _2 < 0 && _2 > 0 ==> result == _1 % _2 + _2
      ensures _1 % _2 < 0 && _2 < 0 ==> result == _1 % _2 - _2

    spec leading_zeros where
      ensures _1 < 0 ==> result == 0
      ensures _1 == 0 ==> result == 128
      ensures _1 > 0 ==> (1 << (128 - 1 - result)) <= _1 && _1 < (1 << (128 - result))

    spec trailing_zeros where
      ensures _1 == 0 ==> result == 128
      ensures _1 != 0 ==> _1 % (1 << result) == 0 && _1 % (1 << (result + 1)) != 0

    spec checked_add where
      ensures MIN_I128 <= _1 + _2 && _1 + _2 <= MAX_I128 ==> result is Some && result.0 == _1 + _2
      ensures !(MIN_I128 <= _1 + _2 && _1 + _2 <= MAX_I128) ==> result is None

    spec checked_sub where
      ensures MIN_I128 <= _1 - _2 && _1 - _2 <= MAX_I128 ==> result is Some && result.0 == _1 - _2
      ensures !(MIN_I128 <= _1 - _2 && _1 - _2 <= MAX_I128) ==> result is None

    spec checked_mul where
      ensures MIN_I128 <= _1 * _2 && _1 * _2 <= MAX_I128 ==> result is Some && result.0 == _1 * _2
      ensures !(MIN_I128 <= _1 * _2 && _1 * _2 <= MAX_I128) ==> result is None

    spec saturating_add where
      ensures result == (if _1 + _2 > MAX_I128 then MAX_I128 else if _1 + _2 < MIN_I128 then MIN_I128 else _1 + _2)

    spec saturating_sub where
      ensures result == (if _1 - _2 > MAX_I128 then MAX_I128 else if _1 - _2 < MIN_I128 then MIN_I128 else _1 - _2)

    spec wrapping_shl where
      ensures result == (((_1 << (_2 % 128)) - MIN_I128) % (MAX_I128 - MIN_I128 + 1)) + MIN_I128

    spec wrapping_shr where
      ensures result == _1 >> (_2 % 128)

  impl «isize» where
    spec fun pow(x : Int, n : Int) : Int decreases n :=
      if n <= 0 then 1 else x * pow(x, n - 1)

    spec fun bits(x : Int, width : Int) : Int decreases width :=
      if width <= 0 then 0 else x % 2 + bits(x / 2, width - 1)

    -- The bits of a negative value are its two's complement.
    spec fun count_ones(x : Int) : Int :=
      bits(if 0 > x then x + 18446744073709551616 else x, 64)

    spec pow where
      panics_if pow(_1, _2) < MIN_I64 || pow(_1, _2) > MAX_I64
      ensures result == pow(_1, _2)

    spec count_ones where
      ensures result == count_ones(_1)

    spec abs where
      panics_if _1 == MIN_I64
      ensures result == (if _1 < 0 then -_1 else _1)

    spec checked_div where
      ensures _2 != 0 && !(_1 == MIN_I64 && _2 == -1) ==> result is Some && result.0 == _1 / _2
      ensures _2 == 0 || (_1 == MIN_I64 && _2 == -1) ==> result is None

    spec rem_euclid where
      panics_if _2 == 0 || (_1 == MIN_I64 && _2 == -1)
      ensures _1 % _2 >= 0 ==> result == _1 % _2
      ensures _1 % _2 < 0 && _2 > 0 ==> result == _1 % _2 + _2
      ensures _1 % _2 < 0 && _2 < 0 ==> result == _1 % _2 - _2

    spec leading_zeros where
      ensures _1 < 0 ==> result == 0
      ensures _1 == 0 ==> result == 64
      ensures _1 > 0 ==> (1 << (64 - 1 - result)) <= _1 && _1 < (1 << (64 - result))

    spec trailing_zeros where
      ensures _1 == 0 ==> result == 64
      ensures _1 != 0 ==> _1 % (1 << result) == 0 && _1 % (1 << (result + 1)) != 0

    spec checked_add where
      ensures MIN_I64 <= _1 + _2 && _1 + _2 <= MAX_I64 ==> result is Some && result.0 == _1 + _2
      ensures !(MIN_I64 <= _1 + _2 && _1 + _2 <= MAX_I64) ==> result is None

    spec checked_sub where
      ensures MIN_I64 <= _1 - _2 && _1 - _2 <= MAX_I64 ==> result is Some && result.0 == _1 - _2
      ensures !(MIN_I64 <= _1 - _2 && _1 - _2 <= MAX_I64) ==> result is None

    spec checked_mul where
      ensures MIN_I64 <= _1 * _2 && _1 * _2 <= MAX_I64 ==> result is Some && result.0 == _1 * _2
      ensures !(MIN_I64 <= _1 * _2 && _1 * _2 <= MAX_I64) ==> result is None

    spec saturating_add where
      ensures result == (if _1 + _2 > MAX_I64 then MAX_I64 else if _1 + _2 < MIN_I64 then MIN_I64 else _1 + _2)

    spec saturating_sub where
      ensures result == (if _1 - _2 > MAX_I64 then MAX_I64 else if _1 - _2 < MIN_I64 then MIN_I64 else _1 - _2)

    spec wrapping_shl where
      ensures result == (((_1 << (_2 % 64)) - MIN_I64) % (MAX_I64 - MIN_I64 + 1)) + MIN_I64

    spec wrapping_shr where
      ensures result == _1 >> (_2 % 64)
