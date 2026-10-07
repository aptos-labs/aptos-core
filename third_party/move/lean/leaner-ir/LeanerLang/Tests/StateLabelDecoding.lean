-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Decoding a state-label update uses the caller's known stored value and
its arithmetic bounds. Opaque callers have no program point for the label. -/

set_option maxHeartbeats 25000
set_option leaner.verifyHeartbeats 25000

leaner module 0x42::state_label_decoding where
  pragma verify = false
  struct Counter has Key where
    value : u64

  spec fun increased(addr : Address) : Bool :=
    old(global<Counter>(addr).value) < global<Counter>(addr).value

  fun increment(addr : Address) -> Unit :=
    Counter[addr].value := Counter[addr].value + 1
  spec increment where
    pragma opaque
    ensures increased(addr)
    ensures global<Counter>(addr).value == old(global<Counter>(addr).value) + 1
    aborts_if !exists<Counter>(addr)
    aborts_if global<Counter>(addr).value + 1 > MAX_U64
    modifies global<Counter>(addr)

  fun twice(addr : Address) -> Unit := do
    increment(addr)
    increment(addr)
  spec twice where
    pragma opaque
    pragma aborts_if_is_partial
    ensures ..S |~ update<Counter>(addr, core.data.updateField(old(global<Counter>(addr)), value, old(global<Counter>(addr).value) + 1))
    ensures S.. |~ increased(addr)
    modifies global<Counter>(addr)

  fun caller(addr : Address) -> Unit := twice(addr)
  spec caller where
    pragma aborts_if_is_partial
    ensures old(global<Counter>(addr).value) + 1 < global<Counter>(addr).value
    modifies global<Counter>(addr)

  -- A state-change definition must not fabricate a missing resource.
  fun missing_label(addr : Address) -> Unit := ()
  spec missing_label where
    requires !exists<Counter>(addr)
    aborts_if false
    ensures ..S |~ update<Counter>(addr, new Counter { value := 1 })

  fun overflow_label(addr : Address) -> Unit := ()
  spec overflow_label where
    requires exists<Counter>(addr)
    requires global<Counter>(addr).value == MAX_U64
    aborts_if false
    ensures ..S |~ update<Counter>(addr, core.data.updateField(old(global<Counter>(addr)), value, old(global<Counter>(addr).value) + 1))

verify 0x42::state_label_decoding::increment
verify 0x42::state_label_decoding::twice
verify 0x42::state_label_decoding::caller
/--
error: the specification clause `ensures ..S |~ update<Counter>(addr, new Counter { value := 1 })` is not established
---
error: leaner verification failed: the automatic verification of `missing_label` failed; provide a proof: `verify missing_label by …` in the module (`verify missing_label by skip` shows the obligations it leaves)
-/
#guard_msgs in
verify 0x42::state_label_decoding::missing_label
/--
error: the specification clause `ensures ..S |~ update<Counter>(addr, core.data.updateField(old(global<Counter>(addr)), value, old(global<Counter>(addr).value) + 1))` is not established
---
error: leaner verification failed: the automatic verification of `overflow_label` failed; provide a proof: `verify overflow_label by …` in the module (`verify overflow_label by skip` shows the obligations it leaves)
-/
#guard_msgs in
verify 0x42::state_label_decoding::overflow_label
