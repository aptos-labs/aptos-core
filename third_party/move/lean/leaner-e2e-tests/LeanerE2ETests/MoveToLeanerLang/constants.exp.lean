-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner module 0x42::constants where
  const ENABLED : Bool := true

  const OWNER : Address := @0x42

  const NAME : Vector<u8> := b"constants"

  const LIMIT : u8 := 200u8

  const BIG : u128 := 1267650600228229401496703205376u128

  spec fun int2bv_u64(value : Int) : Int := int_to_bit_vector(value + 1)

  spec fun int2bv_u128(value : Int) : Int := int_to_bit_vector(value + 1)

  spec fun int2bv_and_u64(left : Int, right : Int) : Int := left & right

  public fun owner() -> Address := OWNER

  public fun enabled() -> Bool := ENABLED

  public fun name() -> Vector<u8> := NAME

  public fun within(x : u8) -> Bool := x < LIMIT

  spec within where
    ensures result == (x < LIMIT)

  public fun big() -> u128 := BIG
