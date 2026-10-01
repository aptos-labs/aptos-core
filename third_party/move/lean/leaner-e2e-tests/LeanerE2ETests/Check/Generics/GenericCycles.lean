-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang
import LeanerE2ETests.CheckSupport

/-!
# Cycles of calls through generic functions

The members of a cycle of calls through a generic function are proved
together at every skolem family and type instantiation, each call to a member
assumed at its contract at the instantiation the call induces.
-/

namespace LeanerLang.Tests.Check.Generics.GenericCycles

leaner module 0x42::generic_cycles where
  public fun even_steps {T has Copy, Drop}(value : T, n : u64) -> T :=
    if n == 0 then value else odd_steps::<T>(value, n - 1)
  spec even_steps where
    ensures result == value
    aborts_if false

  public fun odd_steps {T has Copy, Drop}(value : T, n : u64) -> T :=
    if n == 0 then value else even_steps::<T>(value, n - 1)
  spec odd_steps where
    ensures result == value
    aborts_if false

  -- A cycle through a generic and a non-generic member.
  public fun count_down(n : u64) -> u64 :=
    if n == 0 then 0 else tagged_count::<Bool>(true, n - 1)
  spec count_down where
    ensures result == 0
    aborts_if false

  public fun tagged_count {T has Drop}(tag : T, n : u64) -> u64 := count_down(n)
  spec tagged_count where
    ensures result == 0
    aborts_if false

  -- A caller uses the members' contracts at its instantiation.
  public fun steps_u64(value : u64) -> u64 := even_steps::<u64>(value, 3)
  spec steps_u64 where
    ensures result == value
    aborts_if false

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  assertRuns `«0x42».generic_cycles #[
    ⟨"steps_u64", #[.integer 5], .returned #[.integer 5], {}⟩,
    ⟨"count_down", #[.integer 4], .returned #[.integer 0], {}⟩]

end LeanerLang.Tests.Check.Generics.GenericCycles
