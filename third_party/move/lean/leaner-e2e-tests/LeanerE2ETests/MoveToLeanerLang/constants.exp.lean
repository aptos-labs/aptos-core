-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner module 0x42::constants where
  const ENABLED : Bool := true

  const OWNER : Address := @0x42

  const NAME : Vector<u8> := b"constants"

  const LIMIT : u8 := 200u8

  const BIG : u128 := 1267650600228229401496703205376u128

  spec fun int2bv_u64(value : Int) : Int :=
    if 0 <= value && value <= MAX_U64 then
      if 0 <= value + 1 && value + 1 < 18446744073709551616 then value + 1
      else
        ((value + 1) % 18446744073709551616 + 18446744073709551616)
          % 18446744073709551616
    else abort()

  spec fun int2bv_u128(value : Int) : Int :=
    if 0 <= value && value <= MAX_U128 then
      if 0 <= value + 1 && value + 1 < 18446744073709551616 then value + 1
      else
        ((value + 1) % 18446744073709551616 + 18446744073709551616)
          % 18446744073709551616
    else abort()

  spec fun int2bv_and_u64(left : Int, right : Int) : Int :=
    if 0 <= left && left <= MAX_U64 && (0 <= right && right <= MAX_U64) then
      (if 0 <= left && left < 18446744073709551616 then left
      else
        (left % 18446744073709551616 + 18446744073709551616)
          % 18446744073709551616)
        & right
    else abort()

  public fun owner() -> Address := OWNER

  public fun enabled() -> Bool := ENABLED

  public fun name() -> Vector<u8> := NAME

  public fun within(x : u8) -> Bool := x < LIMIT

  spec within where
    ensures result == (x < LIMIT)

  public fun big() -> u128 := BIG
