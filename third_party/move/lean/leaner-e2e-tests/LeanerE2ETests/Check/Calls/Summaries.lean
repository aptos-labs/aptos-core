-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Contract summaries

A native has no body: it
runs as the runtime implements it, and a caller's theorems assume that the
native satisfies its `spec`. A function specified `pragma opaque` is
summarized for its callers the same way; its own body is still verified
against the contract. A caller of a verified function assumes the natives
that function assumes (`octuple`).
-/

leaner module 0x99::summaries where
  const E_LIMIT : u64 := 7

  -- A native: modeled by its contract.
  native fun host_double(value : u64) -> u64
  spec host_double where
    pragma aborts_if_is_partial
    ensures result == 2 * value

  public fun quadruple(value : u64) -> u64 := host_double(host_double(value))
  spec quadruple where
    pragma aborts_if_is_partial
    ensures result == 4 * value

  public fun octuple(value : u64) -> u64 := quadruple(host_double(value))
  spec octuple where
    pragma aborts_if_is_partial
    ensures result == 8 * value

  -- An opaque function: callers see the contract; the body is verified.
  public fun checked_succ(value : u64) -> u64 := do
    if value >= E_LIMIT then abort(E_LIMIT)
    value + 1
  spec checked_succ where
    pragma opaque
    ensures result == value + 1
    aborts_if value >= E_LIMIT with E_LIMIT

  public fun succ_twice(value : u64) -> u64 := do
    let once := checked_succ(value)
    checked_succ(once)
  spec succ_twice where
    pragma aborts_if_is_partial
    ensures result == value + 2
    aborts_if value >= E_LIMIT with E_LIMIT

  -- A native over references: callers see its contract.
  native fun swap {T} (left : &mut T, right : &mut T) -> Unit
  spec swap where
    ensures right == old(left) && left == old(right)
    aborts_if false

  public fun replace {T} (ref : &mut T, new : T) -> T := do
    let mut value := new
    let slot := &mut value
    swap::<T>(ref, slot)
    *slot
  spec replace where
    pragma opaque
    ensures result == old(ref) && ref == new
    aborts_if false
