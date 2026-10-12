-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner module 0x99::stored_write_frame_errors where
  struct Counter has Key where
    value : u64
  struct Config has Key where
    value : u64
  struct Stored has Copy, Drop, Store where
    action : Fn(Address, u64) -> u64 has Copy, Drop, Store
  spec Stored where
    modifies_of<action>(owner : Address, value : u64) global<Counter>(owner)

  public fun write_config(owner : Address, value : u64) -> u64 := do
    let target := &mut Config[owner].value
    *target := value
    value
  spec write_config where
    modifies global<Config>(owner)
    aborts_if !exists<Config>(owner)
    ensures result == value

  -- Its actual write falls outside the field's declared frame.
  fun invalid() -> Stored := new Stored {
    action := function[Fn(Address, u64) -> u64 has Copy, Drop, Store](write_config) }

  fun invalid_replacement(stored : &mut Stored) -> Unit := do
    let action := &mut stored.action
    *action := function[Fn(Address, u64) -> u64 has Copy, Drop, Store](write_config)

  struct AnyWriter has Copy, Drop, Store where
    action : Fn(Address, u64) -> u64 has Copy, Drop, Store
  spec AnyWriter where
    modifies_of<action> *

  fun wrong_frame(stored : AnyWriter, owner : Address, value : u64) -> u64 :=
    invoke(stored.action, owner, value)
  spec wrong_frame where
    pragma aborts_if_is_partial
    modifies *
    ensures global<Config>(owner) == old(global<Config>(owner))

  -- The slot explicitly allowed to change is not preserved either.
  fun wrong_slot(stored : Stored, owner : Address, value : u64) -> u64 :=
    invoke(stored.action, owner, value)
  spec wrong_slot where
    pragma aborts_if_is_partial
    modifies global<Counter>(owner)
    ensures global<Counter>(owner) == old(global<Counter>(owner))
