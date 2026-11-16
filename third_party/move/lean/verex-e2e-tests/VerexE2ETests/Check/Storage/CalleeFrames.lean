-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# Callee frames

A caller reads the storage a callee's `modifies` clause leaves alone at its
value before the call: at another key of the modified resource and at
another resource.
-/

namespace VerexSL.Tests.Check.Storage.CalleeFrames

verex module 0x42::callee_frames where
  struct Counter has Key where
    value : u64

  struct Config has Key where
    limit : u64

  fun set_counter(addr : Address, value : u64) -> Unit := do
    let counter := &mut Counter[addr]
    counter.value := value
  spec set_counter where
    pragma opaque
    requires exists<Counter>(addr)
    ensures global<Counter>(addr).value == value
    aborts_if false
    modifies global<Counter>(addr)

  fun other_key(first : Address, second : Address) -> Unit := do
    set_counter(first, 1)
  spec other_key where
    requires first != second
    requires exists<Counter>(first) && exists<Counter>(second)
    ensures global<Counter>(second) == old(global<Counter>(second))
    ensures global<Counter>(first).value == 1
    modifies global<Counter>(first)

  fun other_resource(addr : Address) -> Unit := do
    set_counter(addr, 2)
  spec other_resource where
    requires exists<Counter>(addr) && exists<Config>(addr)
    ensures global<Config>(addr) == old(global<Config>(addr))
    modifies global<Counter>(addr)

  -- A publish after the call tests the resource at the memory the call
  -- left; the clause tests it before the call, and the callee's frame
  -- carries the one read to the other.
  fun publish_after_call(account : &Signer, addr : Address) -> Unit := do
    set_counter(addr, 3)
    move_to<Config>(account, new Config { limit := 1 })
  spec publish_after_call where
    requires exists<Counter>(addr)
    aborts_if exists<Config>(account.address)
    ensures exists<Config>(account.address)
    modifies global<Counter>(addr)
    modifies global<Config>(account.address)

end VerexSL.Tests.Check.Storage.CalleeFrames
