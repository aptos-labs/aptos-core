-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# A generic callee's conditions at the caller's row

A call of a generic function whose body states conditions of its own is
inlined as that body compiled at the call's type arguments
(`doc/plan/one-spelling.md`, D2): the loop invariants and the precondition of
the callee are built at the row, and keyed by the sites the row's own site
space gives them, so two readings of one body never share a condition.

The caller therefore owes the callee's `requires` where the body starts, as
it owes an inlined callee's: `counted_bool` establishes it, `counted_u64`
does not and is told so. While a callee inlined at a substitution was walked
without its conditions, no diagnostic reported that call.
-/

namespace VerexSL.Tests.Check.Generics.GenericRowConditions

verex module 0x42::generic_row_conditions where
  struct Box {T has Copy, Drop, Store} has Copy, Drop, Store where
    value : T
    count : u64

  -- A generic callee with a precondition and a loop of its own, whose
  -- parameter reads its type parameter.
  fun counted {T has Copy, Drop, Store}(box : Box<T>) -> u64 := do
    let mut i := 0
    while i < box.count do
      i := i + 1
    where
      invariant i <= box.count
    i
  spec counted where
    requires box.count < 10
    ensures result == box.count
    aborts_if false

  -- The caller establishes the callee's precondition at its row.
  fun counted_bool(box : Box<Bool>) -> u64 := core.call counted::<Bool>(box)
  spec counted_bool where
    requires box.count < 10
    ensures result == box.count
    aborts_if false

  -- The caller does not establish it, and is told at the call.
  fun counted_u64(box : Box<u64>) -> u64 := core.call counted::<u64>(box)
  spec counted_u64 where
    ensures result == box.count
    aborts_if false

end VerexSL.Tests.Check.Generics.GenericRowConditions
