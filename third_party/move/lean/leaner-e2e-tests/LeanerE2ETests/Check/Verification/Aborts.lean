-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# The two directions of `aborts_if`, verified from LIR

The first verification tests copied from the reference stack
(`move/Move/Tests/Verification/AbortDirections.lean`), restricted to its
state-free functions: the generated contracts read the declared `aborts_if`
clauses with their codes and the two abort pragmas, and the in-module
`verify` items prove them by scripted symbolic execution over the
validated unit, as in the reference stack.

A declared condition is *sufficient* (the function must fail where it holds)
and, without `pragma aborts_if_is_partial`, the declared conditions are also
*necessary*: every failure matches a clause, with its code when one is
declared.  `pragma aborts_if_is_strict` makes an empty clause list mean
"never fails"; on its own an empty list leaves failure behavior
uninterpreted, so the postcondition is owed on every successful execution
and nothing is claimed about failures.
-/

leaner module 0x99::abort_directions where
  const E_ZERO : u64 := 1
  const E_LARGE : u64 := 2

  -- Both directions: the clause is exactly where the function aborts.
  public fun halve(value : u64) -> u64 := do
    if value < 1 then abort(E_ZERO)
    value / 2
  spec halve where
    ensures result == value / 2
    aborts_if value < 1 with E_ZERO
  verify halve

  -- Partial: the clause names one sufficient condition; the other abort
  -- (the overflow of `value + 1`) is permitted without being declared.
  public fun bump_checked(value : u64, limit : u64) -> u64 := do
    if limit <= value then abort(E_LARGE)
    value + 1
  spec bump_checked where
    pragma aborts_if_is_partial
    ensures result == value + 1
    aborts_if limit <= value with E_LARGE
  verify bump_checked

  -- Strict without clauses: never aborts.
  public fun identity(value : u64) -> u64 := value
  spec identity where
    pragma aborts_if_is_strict
    ensures result == value
  verify identity

  -- Uninterpreted abort behavior: the postcondition is owed on every
  -- successful execution and nothing is claimed about the overflow.
  public fun successor(value : u64) -> u64 := value + 1
  spec successor where
    ensures result == value + 1
  verify successor

  struct Counter has Key where
    value : u64

  -- The storage form: an `assert!` guards a subtraction through a borrow,
  -- and the missing resource is a second, code-free abort direction.
  public entry fun withdraw(addr : Address, amount : u64) -> Unit := do
    let value := &mut Counter[addr].value
    let current := *value
    assert!(current >= amount, E_LARGE)
    *value := *value - amount
  spec withdraw where
    modifies global<Counter>(addr)
    ensures global<Counter>(addr).value == old(global<Counter>(addr).value) - amount
    aborts_if !exists<Counter>(addr)
    aborts_if old(global<Counter>(addr).value) < amount with E_LARGE
  verify withdraw

  -- A literal abort code pins the failure outcome directly.
  public fun explicit_abort(value : u64) -> u64 := do
    if value < 10 then abort(4)
    value
  spec explicit_abort where
    ensures result == value
    aborts_if value < 10 with 4
  verify explicit_abort

  -- The same guard as an `assert!`: the throw sits on the else-arm.
  public fun assert_floor(value : u64) -> u64 := do
    assert!(value >= 1, E_ZERO)
    value - 1
  spec assert_floor where
    ensures result == value - 1
    aborts_if value < 1 with E_ZERO
  verify assert_floor

#leaner_require_native 0x99::abort_directions::halve
#leaner_require_native 0x99::abort_directions::bump_checked
#leaner_require_native 0x99::abort_directions::identity
#leaner_require_native 0x99::abort_directions::successor
#leaner_require_native 0x99::abort_directions::explicit_abort
#leaner_require_native 0x99::abort_directions::assert_floor

/-! ## A deliberately wrong body fails at the specification range

The body adds two where the unchanged contract promises one; the failure is
reported at the authored `ensures` clause, not as an interpreter state. -/

leaner module 0x99::abort_directions_negative where
  public fun off_by_one(value : u64) -> u64 := value + 2
  spec off_by_one where
    pragma aborts_if_is_partial
    ensures result == value + 1

/-! The failure is attributed to the authored clause. -/

#leaner_verify 0x99::abort_directions_negative::off_by_one
