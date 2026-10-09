-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Sequential writes may alias. An opaque caller observes the final value
through the labeled contract alone, without the callee's program points.
The frame may refer to its own poststate and must not become a cyclic rewrite. -/
set_option maxHeartbeats 25000
set_option leaner.verifyHeartbeats 25000

leaner module 0x42::state_label_memory_equality where
  pragma verify = false
  struct Counter has Key where
    value : u64

  fun write_twice(a1 : Address, a2 : Address, v1 : u64, v2 : u64) -> Unit := do
    Counter[a1].value := v1
    Counter[a2].value := v2
  spec write_twice where
    pragma opaque
    ensures ..S |~ update<Counter>(a1, core.data.updateField(old(global<Counter>(a1)), value, v1))
    ensures S.. |~ update<Counter>(a2, core.data.updateField((..S |~ global<Counter>(a2)), value, v2))
    aborts_if !exists<Counter>(a1)
    aborts_if !exists<Counter>(a2)
    modifies global<Counter>(a1), *
    modifies global<Counter>(a2), *

  fun caller(a1 : Address, a2 : Address, v1 : u64, v2 : u64) -> Unit :=
    write_twice(a1, a2, v1, v2)
  spec caller where
    ensures global<Counter>(a2).value == v2
    aborts_if !exists<Counter>(a1)
    aborts_if !exists<Counter>(a2)
    modifies global<Counter>(a1), *
    modifies global<Counter>(a2), *

  fun wrong_update(addr : Address) -> Unit := do
    Counter[addr].value := 1
    Counter[addr].value := 2
  spec wrong_update where
    ensures ..S |~ update<Counter>(addr, new Counter { value := 1 })
    ensures S.. |~ update<Counter>(addr, new Counter { value := 3 })
    aborts_if !exists<Counter>(addr)
    modifies global<Counter>(addr), *

verify 0x42::state_label_memory_equality::write_twice
verify 0x42::state_label_memory_equality::caller
/--
error: the specification clause `ensures S.. |~ update<Counter>(addr, new Counter { value := 3 })` is not established
---
error: leaner verification failed: the automatic verification of `wrong_update` failed; provide a proof: `verify wrong_update by …` in the module (`verify wrong_update by skip` shows the obligations it leaves)
-/
#guard_msgs in
verify 0x42::state_label_memory_equality::wrong_update
