-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner module 0x42::state_labels where
  -- Copyright © Aptos Foundation
  -- SPDX-License-Identifier: Apache-2.0
  -- State labels: a label defined by a state-change predicate (`..S |~ publish`),
  -- read by behavioral predicates at a one-state (`S |~`) and a post-state
  -- (`S.. |~`) position, and labels quantified over the state domain
  -- (`exists S in *`) splitting a two-state specification function.
  struct Resource has Key where
    value : u64

  struct Counter has Copy, Drop, Store where
    value : u64

  fun read_resource(addr : Address) -> u64 := Resource[addr].value

  spec read_resource where
    pragma opaque
    ensures result == global<Resource>(addr).value
    aborts_if !exists<Resource>(addr)

  fun create_then_read(account : &Signer, addr : Address) -> u64 := do
    move_to<Resource>(account, new Resource { value := 42 })
    read_resource(addr)

  spec create_then_read where
    ensures ..S |~ publish<Resource>(
        account.address, new Resource { value := 42 }
      )
    ensures result
        == (S.. |~ result_of<function[Fn(Address) -> u64](read_resource)>(addr))
    aborts_if S.. |~ aborts_of<function[Fn(Address) -> u64](read_resource)>(
        addr
      )
    aborts_if exists<Resource>(account.address)
    modifies global<Resource>(account.address), *

  spec fun counter_increased(old_c : Counter, c : Counter) : Bool :=
    old_c.value < c.value

  fun inc(c : &mut Counter) -> Unit := c.value := c.value + 1

  fun inc_twice(c : &mut Counter) -> Unit := do
    inc(c)
    inc(c)

  spec inc_twice where
    ensures ∃ (S : StateDomain),
        (..S |~ counter_increased(old(c), c))
          && (S.. |~ counter_increased(old(c), c))
