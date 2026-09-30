-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-!
# Loans ending between operands

A loan whose holders no later operand and no continuation uses dies before
the next operand of the same operation, unless the value of an earlier
operand carries it. A tuple can read through a reference and then move the
place the reference borrows.
-/

namespace LeanerLang.Tests.Check.References.OperandLoans

leaner module 0x42::operand_loans where
  struct S has Drop where
    v : u64

  fun read_then_move(s : S) -> u64 := do
    let r := &s.v
    let (a, b) := (*r, s)
    a + b.v
  spec read_then_move where
    pragma aborts_if_is_partial
    ensures result == s.v + s.v

  fun write_then_move(s : S) -> u64 := do
    let r := &mut s.v
    *r := 5
    let (a, b) := (*r, s)
    a + b.v
  spec write_then_move where
    ensures result == 10

  fun write_then_move_incorrect(s : S) -> u64 := do
    let r := &mut s.v
    *r := 5
    let (a, b) := (*r, s)
    a + b.v
  spec write_then_move_incorrect where
    ensures result == 5 + s.v -- error: the write reaches the moved value

  fun consume(a : u64, s : S) -> u64 := a + s.v

  fun read_then_pass(s : S) -> u64 := do
    let r := &s.v
    consume(*r, s)
  spec read_then_pass where
    pragma aborts_if_is_partial
    ensures result == s.v + s.v

end LeanerLang.Tests.Check.References.OperandLoans
