-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexRust.Profile

/-!
Rust's `as` wraps and rustc spells no checked cast, so the exporter never reaches
`Term.cast` and `Term.charOfChecked`. This unit spells them.
-/

verex namespace differential_casts where
  fun to_i8(a : u8) -> i8 := core.prim.checkedCast[panic, i8](a)

  fun to_u8(a : i8) -> u8 := core.prim.checkedCast[panic, u8](a)

  fun to_u16(a : i8) -> u16 := core.prim.checkedCast[panic, u16](a)

  fun narrow(a : u32) -> u8 := core.prim.checkedCast[panic, u8](a)

  fun signed_narrow(a : i128) -> i64 := core.prim.checkedCast[panic, i64](a)

  fun to_char(a : u8) -> Char := core.prim.checkedCast[panic, Char](a)
