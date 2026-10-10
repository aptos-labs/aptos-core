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
to each recursive call. Without a proof, the verifier unfolds a definition
once at each application a leaf holds whose guard the context decides, as
a solver instantiates a definitional axiom: a loop summing a vector keeps
an invariant over the sum up to its index (`sum_probe`). Mutually recursive
functions are defined together, and lemmas about them recurse through
each other (`mutual_spec`). Boolean parameters are Booleans, of a function
and of a lemma alike (`boolean_parameters`). A definition reading storage
takes the memory and the family its reads resolve types at
(`stateful_recursion`).
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

leaner module 0x42::sum_probe where
  spec fun sum_upto(v : Vector<u64>, n : Int) : Int :=
    if n <= 0 then 0 else sum_upto(v, n - 1) + v[n - 1]

  fun total(v : Vector<u64>) -> u64 := do
    let sum := 0
    let i := 0
    while i < v.length do
      sum := sum + v[i]
      i := i + 1
    where
      invariant i <= v.length
      invariant sum == sum_upto(v, i)
    sum
  spec total where
    ensures result == sum_upto(v, v.length)

leaner module 0x42::mutual_spec where
  spec fun f(n : Int) : Int := if n <= 0 then 1 else f(n - 1) + g(n - 1)
  spec fun g(n : Int) : Int := if n <= 0 then 0 else f(n - 1)

  spec lemma f_pos(n : Int) where
    requires 0 <= n
    ensures f(n) >= 1
    proof
      split n > 0
      apply n > 0 ==> f_pos(n - 1)
      apply n > 0 ==> g_nonneg(n - 1)

  spec lemma g_nonneg(n : Int) where
    requires 0 <= n
    ensures g(n) >= 0
    proof
      split n > 0
      apply n > 0 ==> f_pos(n - 1)

  fun one() -> u64 := 1
  spec one where
    ensures result == f(0)
    ensures g(1) == result

leaner module 0x42::boolean_parameters where
  spec fun count_if(n : Int, p : Bool, q : Bool) : Int :=
    if n <= 0 then 0 else (if p == q then 1 else 0) + count_if(n - 1, p, q)

  spec lemma same_counts(n : Int, p : Bool) where
    requires 0 <= n
    ensures count_if(n, p, p) == n
    proof
      split n > 0
      apply n > 0 ==> same_counts(n - 1, p)

  fun one(b : Bool) -> u64 := 1
  spec one where
    ensures count_if(1, b, b) == result

leaner module 0x42::stateful_recursion where
  struct R has Key where
    v : u64

  spec fun get_v(a : Address) : Int := global<R>(a).v

  spec fun rf(a : Address, n : Int) : Bool :=
    if n <= 0 then false else rg(a, n - 1) || get_v(a) == 1

  spec fun rg(a : Address, n : Int) : Bool :=
    if n <= 0 then false else rf(a, n - 1)

  fun read_v(a : Address) -> u64 := R[a].v
  spec read_v where
    requires exists<R>(a)
    requires !rf(a, 1)
    ensures result != 1
