-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Conditional observations of two opaque calls share a Boolean when their
memory is unchanged. Cover both branches without assuming either one. -/
set_option maxHeartbeats 25000
set_option leaner.verifyHeartbeats 25000

leaner module 0x42::state_label_boolean_observations where
  pragma verify = false
  struct Counter has Key where
    value : u64
  struct Config has Key where
    active : Bool

  fun read(addr : Address) -> u64 := Counter[addr].value
  spec read where
    pragma opaque
    ensures result == global<Counter>(addr).value
    aborts_if !exists<Counter>(addr)

  fun conditional(addr : Address) -> u64 :=
    if Config[addr].active then Counter[addr].value else 0
  spec conditional where
    pragma opaque
    pragma aborts_if_is_partial
    ensures global<Config>(addr).active ==> result == global<Counter>(addr).value
    ensures !global<Config>(addr).active ==> result == 0

  fun twice(addr : Address) -> (u64, u64) := do
    let a := conditional(addr)
    let b := conditional(addr)
    (a, b)
  spec twice where
    pragma aborts_if_is_partial
    ensures global<Config>(addr) == old(global<Config>(addr))
    ensures result == spec.result[1]

  -- The unconditional read may differ from the conditional read when inactive.
  fun different_readers(addr : Address) -> (u64, u64) := do
    let a := read(addr)
    let b := conditional(addr)
    (a, b)
  spec different_readers where
    pragma aborts_if_is_partial
    ensures result == spec.result[1]

verify 0x42::state_label_boolean_observations::read
verify 0x42::state_label_boolean_observations::conditional
verify 0x42::state_label_boolean_observations::twice
/--
error: the specification clause `ensures result == spec.result[1]` is not established
---
error: leaner verification failed: the automatic verification of `different_readers` failed; provide a proof: `verify different_readers by …` in the module (`verify different_readers by skip` shows the obligations it leaves)
-/
#guard_msgs in
verify 0x42::state_label_boolean_observations::different_readers
