-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-!
# Recursive specification functions

A recursive `spec fun` names a `decreases` measure and becomes a Lean
definition over its bundled arguments (`count.spec`), with the unfolding
theorem `count.spec.unfold`. Lemmas about it are items of the module:
they are elaborated once the module is registered, and `verify` after them.
Termination is proved at definition time from the conditions on the path
to each recursive call.
-/

leaner module 0x42::recursive_spec where
  fun first_is(values : Vector<u64>, needle : u64) -> Bool :=
    values.length > 0 && values[0] == needle
  spec first_is where
    ensures result ==> count(values, needle, values.length) >= 1
    aborts_if false

  spec fun count(values : Vector<u64>, needle : Int, n : Int) : Int decreases n :=
    if n <= 0 then 0
    else count(values, needle, n - 1) + (if values[n - 1] == needle then 1 else 0)

  open LeanerIR in
  /-- A vector whose first element is the needle counts it at least once. -/
  theorem count_first (values : RuntimeValue) (needle : Int) :
      ∀ (k : Nat) (n : Int), n.toNat = k → 1 ≤ n → (values.field 0).asInt = needle →
        1 ≤ count.spec (values, needle, n, ()) := by
    intro k
    induction k using Nat.strongRecOn with
    | _ k ih =>
      intro n hk hn hfirst
      rw [count.spec.unfold]
      simp only
      split
      · omega
      · by_cases one : n = 1
        · subst one
          rw [count.spec.unfold]
          simp [hfirst]
        · have := ih (n - 1).toNat (by omega) (n - 1) rfl (by omega) hfirst
          split <;> omega

  verify first_is by
    intro first
    refine count_first _ _ _ _ rfl (by omega) ?_
    simp [LeanerIR.RuntimeValue.field, ‹values.values[0]? = some _›, first]
