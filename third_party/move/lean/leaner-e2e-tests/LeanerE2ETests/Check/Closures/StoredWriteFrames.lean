-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! A stored closure's addressed write frame is established at construction
and preserved through opaque calls, while wildcard frames impose no restriction. -/
leaner module 0x99::stored_write_frames where
  struct Counter has Key where
    value : u64
  struct Config has Key where
    value : u64
  struct Stored has Copy, Drop, Store where
    action : Fn(Address, u64) -> u64 has Copy, Drop, Store
  spec Stored where
    modifies_of<action>(owner : Address, value : u64) global<Counter>(owner)

  public fun write(owner : Address, value : u64) -> u64 := do
    let target := &mut Counter[owner].value
    *target := value
    value
  spec write where
    pragma opaque
    modifies global<Counter>(owner)
    aborts_if !exists<Counter>(owner)
    ensures result == value

  fun make() -> Stored := new Stored {
    action := function[Fn(Address, u64) -> u64 has Copy, Drop, Store](write) }
  spec make where
    pragma opaque
    aborts_if false

  fun call(stored : Stored, owner : Address, value : u64) -> u64 :=
    invoke(stored.action, owner, value)
  spec call where
    pragma aborts_if_is_partial
    modifies global<Counter>(owner)
    ensures global<Config>(owner) == old(global<Config>(owner))

  fun via_opaque(owner : Address, value : u64) -> u64 := call(make(), owner, value)
  spec via_opaque where
    pragma aborts_if_is_partial
    modifies global<Counter>(owner)

  struct AnyWriter has Copy, Drop, Store where
    action : Fn(Address, u64) -> u64 has Copy, Drop, Store
  spec AnyWriter where
    modifies_of<action> *

  fun make_any() -> AnyWriter := new AnyWriter {
    action := function[Fn(Address, u64) -> u64 has Copy, Drop, Store](write) }
  fun call_any(stored : AnyWriter, owner : Address, value : u64) -> u64 :=
    invoke(stored.action, owner, value)
  spec call_any where
    pragma aborts_if_is_partial
    modifies *

  struct GenericCounter {T : phantom type} has Key where
    value : u64
  struct GenericStored {T : phantom type} has Copy, Drop, Store where
    action : Fn(Address, u64) -> u64 has Copy, Drop, Store
  spec GenericStored where
    modifies_of<action>(owner : Address, value : u64) global<GenericCounter<T> >(owner)

  fun generic_call {T}(stored : GenericStored<T>, owner : Address, value : u64) -> u64 :=
    invoke(stored.action, owner, value)
  spec generic_call where
    pragma opaque
    pragma aborts_if_is_partial
    modifies global<GenericCounter<T> >(owner)

  fun concrete_call(stored : GenericStored<Bool>, owner : Address, value : u64) -> u64 :=
    generic_call(stored, owner, value)
  spec concrete_call where
    pragma aborts_if_is_partial
    modifies global<GenericCounter<Bool> >(owner)
    ensures global<GenericCounter<u64> >(owner) == old(global<GenericCounter<u64> >(owner))

  struct BoundWriter has Copy, Drop, Store where
    owner : Address
    action : Fn(u64) -> u64 has Copy, Drop, Store
  spec BoundWriter where
    modifies_of<action>(value : u64) global<Counter>(owner)

  fun bind_owner(owner : Address) -> BoundWriter := new BoundWriter {
    owner := owner,
    action := function[Fn(u64) -> u64 has Copy, Drop, Store](write, owner) }
  spec bind_owner where
    pragma opaque
    aborts_if false
    ensures result.owner == owner

  fun call_bound(stored : BoundWriter, value : u64, other : Address) -> u64 :=
    invoke(stored.action, value)
  spec call_bound where
    pragma aborts_if_is_partial
    requires other != stored.owner
    modifies global<Counter>(stored.owner)
    ensures global<Counter>(other) == old(global<Counter>(other))
