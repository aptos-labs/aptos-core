-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! Bit-vector decisions: under `pragma bv`, a leaf over bit operations
on unsigned values is restated over bit vectors and decided there. Without
the pragma the same leaf is left to integer arithmetic. -/

leaner module 0x48::flags where
  fun set(flags : &mut Vector<u8>, flag : u64, include : Bool) -> Unit := do
    let byte_index := flag / 8
    let bit_mask := 1u8 << (flag % 8) as u8
    while flags.length <= byte_index do
      *flags := core.prim.pushVector(*flags, 0u8)
    if include then
      let byte := &mut flags[byte_index]
      *byte := *byte | bit_mask
    else
      let cleared := 255u8 ^ bit_mask
      let byte := &mut flags[byte_index]
      *byte := *byte & cleared

  spec set where
    pragma bv = b"0"
    aborts_if false
    ensures flags.length > flag / 8
    ensures include == spec_contains(flags, flag)

  fun contains(flags : &Vector<u8>, flag : u64) -> Bool := do
    let byte_index := flag / 8
    let bit_mask := 1u8 << (flag % 8) as u8
    flags.length > byte_index && flags[byte_index] & bit_mask != 0u8

  spec contains where
    pragma bv = b"0"
    aborts_if false
    ensures result == spec_contains(flags, flag)

  spec fun spec_contains(flags : Vector<u8>, flag : Int) : Bool :=
    (1 << flag % 8) % 256 & flags[flag / 8] > 0 && flags.length > flag / 8

  fun toggled(byte : u8, bit : u8) -> u8 := byte ^ (1u8 << bit % 8u8)

  spec toggled where
    pragma bv = b"0"
    ensures result & (1 << bit % 8) % 256 != byte & (1 << bit % 8) % 256
    ensures result == byte -- error: toggling changes the byte

  fun contains_without_bv(flags : &Vector<u8>, flag : u64) -> Bool := do
    let byte_index := flag / 8
    let bit_mask := 1u8 << (flag % 8) as u8
    flags.length > byte_index && flags[byte_index] & bit_mask != 0u8

  spec contains_without_bv where
    aborts_if false
    ensures result == spec_contains(flags, flag) -- error: needs `pragma bv`
