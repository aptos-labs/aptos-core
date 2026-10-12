-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-!
# Comparing mutable references

Equality and inequality of mutable references compare their referents, as
Move does: two references to equal values in distinct places are equal.
No comparison reads a loan's identity.
-/

namespace LeanerLang.Tests.Check.References.ReferenceComparisons

leaner module 0x42::reference_comparisons where
  fun same(a : &mut u64, b : &mut u64) -> Bool := a == b
  spec same where
    pragma verify = false

  fun differ(a : &mut u64, b : &mut u64) -> Bool := a != b
  spec differ where
    pragma verify = false

  fun run_equal() -> Bool := do
    let mut x := 3
    let mut y := 3
    same(&mut x, &mut y)
  spec run_equal where
    pragma verify = false

  fun run_unequal() -> Bool := do
    let mut x := 3
    let mut y := 4
    same(&mut x, &mut y)
  spec run_unequal where
    pragma verify = false

  fun run_differ() -> Bool := do
    let mut x := 3
    let mut y := 3
    differ(&mut x, &mut y)
  spec run_differ where
    pragma verify = false

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd assertRuns `«0x42».reference_comparisons #[
  ⟨"run_equal", #[], .returned #[.bool true], {}⟩,
  ⟨"run_unequal", #[], .returned #[.bool false], {}⟩,
  ⟨"run_differ", #[], .returned #[.bool false], {}⟩]

end LeanerLang.Tests.Check.References.ReferenceComparisons
