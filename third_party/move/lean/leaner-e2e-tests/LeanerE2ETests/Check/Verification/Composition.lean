-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Calls compose with the surrounding computation, independently of the
number or positions of the other locals. Each caller uses a proved summary. -/

namespace LeanerLang.Tests.VerificationComposition

set_option leaner.route "native"

leaner module 0x42::composition where
  fun identity(value : u64) -> u64 := value
  spec identity where
    ensures result == value
    aborts_if false
  verify identity

  fun beside_local(value : u64, untouched : u64) -> u64 := do
    let result := identity(value)
    result + untouched
  spec beside_local where
    requires value + untouched <= 18446744073709551615
    ensures result == value + untouched
    aborts_if false
  verify beside_local

  fun nested(value : u64) -> u64 := identity(identity(value))
  spec nested where
    ensures result == value
    aborts_if false
  verify nested

  fun set_seven(slot : &mut u64) -> Unit := *slot := 7
  spec set_seven where
    ensures *slot == 7
    aborts_if false
  verify set_seven

  fun local_beside_loan(untouched : &mut u64, value : u64) -> u64 := do
    let mut local := value
    set_seven(&mut local)
    local
  spec local_beside_loan where
    ensures result == 7 && *untouched == old(*untouched)
    aborts_if false
  verify local_beside_loan

  fun repeated_local(untouched : &mut u64, value : u64) -> u64 := do
    let mut local := value
    set_seven(&mut local)
    set_seven(&mut local)
    local
  spec repeated_local where
    ensures result == 7 && *untouched == old(*untouched)
    aborts_if false
  verify repeated_local

  fun checked(value : u64) -> u64 := do
    if value < 1 then abort(7)
    value
  spec checked where
    ensures result == value
    aborts_if value < 1 with 7
  verify checked

  fun forward(value : u64) -> u64 := checked(value)
  spec forward where
    ensures result == value
    aborts_if value < 1 with 7
  verify forward

  fun set_to(slot : &mut u64, value : u64) -> Unit := *slot := value
  spec set_to where
    ensures *slot == value
    aborts_if false
  verify set_to

  fun independent(left : &mut u64, right : &mut u64) -> Unit := do
    set_to(left, 3)
    set_to(right, 5)
  spec independent where
    ensures *left == 3 && *right == 5
    aborts_if false
  verify independent

end LeanerLang.Tests.VerificationComposition
