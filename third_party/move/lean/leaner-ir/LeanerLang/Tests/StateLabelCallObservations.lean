-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Reuse the call rule's observations before re-deriving its contract.
The symbolic quotient and nested labeled result must verify at the native
budget; a false postcondition at an opaque caller must still be rejected. -/
set_option maxHeartbeats 25000
set_option leaner.verifyHeartbeats 25000

leaner module 0x42::state_label_call_observations where

  fun fee(nav : u64) -> u64 := nav / 2

  spec fee where
    pragma opaque
    ensures result == nav / 2
    aborts_if false

  fun conv(amount : u64, denom : u64) -> u64 := amount / denom

  spec conv where
    pragma opaque
    ensures result == amount / denom
    aborts_if denom == 0

  -- Two opaque calls in sequence, so the contract has to speak about the
  -- state between them. An `aborts_if` naming that state defines the label
  -- `S1`, and the label definition is what the instrumenter needs. The
  -- condition itself must not be assumed along with it: on the success path
  -- it says the function aborted, which contradicts the arithmetic the body
  -- just completed and makes every postcondition provable.
  fun caller(shares : u64, nav : u64) -> u64 := do
    if nav == 0 then return 0;
    let f := fee(nav)
    let for_fee := shares * f / nav
    let rest := shares - for_fee
    conv(rest, nav)

  spec caller where
    pragma opaque
    ensures do
        let a :=
          do
            let b := (..S1 |~ result_of<function[Fn(u64) -> u64](fee)>(nav))
            S1.. |~ result_of<function[Fn(u64, u64) -> u64](conv)>(
              shares - shares * b / nav, nav
            )
        result == (if nav == 0 then 0 else a)
    aborts_if nav != 0
        && (do
          let a := (..S1 |~ result_of<function[Fn(u64) -> u64](fee)>(nav))
          S1.. |~ aborts_of<function[Fn(u64, u64) -> u64](conv)>(
            shares - shares * a / nav, nav
          ))
    aborts_if do
        let a := (..S1 |~ result_of<function[Fn(u64) -> u64](fee)>(nav))
        nav != 0 && shares < shares * a / nav
    aborts_if do
        let a := (..S1 |~ result_of<function[Fn(u64) -> u64](fee)>(nav))
        nav != 0 && shares * a > MAX_U64

  -- The opaque caller consumes the labeled contract without callee points.
  fun caller_again(shares : u64, nav : u64) -> u64 := caller(shares, nav)
  spec caller_again where
    requires nav == 0
    aborts_if false
    ensures result == 0

  -- An invocation-defined label must not make a false postcondition provable.
  fun wrong_result(shares : u64, nav : u64) -> u64 := caller(shares, nav)
  spec wrong_result where
    pragma verify = false
    requires nav == 0
    aborts_if false
    ensures result == 1

/--
error: the specification clause `ensures result == 1` is not established
---
error: leaner verification failed: the automatic verification of `wrong_result` failed; provide a proof: `verify wrong_result by …` in the module (`verify wrong_result by skip` shows the obligations it leaves)
-/
#guard_msgs in
verify 0x42::state_label_call_observations::wrong_result
