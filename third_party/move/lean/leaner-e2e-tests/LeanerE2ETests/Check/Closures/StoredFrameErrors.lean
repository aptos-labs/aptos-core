-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang
leaner module 0x99::stored_frame_negative where
  struct Counter has Key where
    value : u64
  struct Stored has Copy, Drop, Store where
    action : Fn(Address, u64) -> u64 has Copy, Drop, Store
  public fun write(owner : Address, x : u64) -> u64 := do
    let value := &mut Counter[owner].value
    *value := x
    x
  spec write where
    modifies global<Counter>(owner)
    aborts_if !exists<Counter>(owner)
    ensures result == x
  fun bad() -> Stored := new Stored {
    action := function[Fn(Address, u64) -> u64 has Copy, Drop, Store](write) }

  -- Replacing a field must re-establish the same frame checked at packing.
  fun bad_replacement(stored : &mut Stored) -> Unit := do
    let action := &mut stored.action
    *action := function[Fn(Address, u64) -> u64 has Copy, Drop, Store](write)
