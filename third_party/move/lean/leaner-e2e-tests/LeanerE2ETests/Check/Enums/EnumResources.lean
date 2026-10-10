-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! An enum resource, whose variants share a field: a read and a write of
the shared field through a mutable reference, and a published value read
from storage. -/

leaner module 0x42::enum_resources where
  enum Slot has Key where
    | Empty (val : u64)
    | Full (val : u64)

  public fun read_val(self : &mut Slot) -> u64 := self.val
  spec read_val where
    aborts_if false
    ensures result == self.val

  public fun write_val(self : &mut Slot, v : u64) -> Unit := self.val := v
  spec write_val where
    aborts_if false
    ensures self.val == v

  public fun stored_val(addr : Address) -> u64 := Slot[addr].val
  spec stored_val where
    aborts_if !exists<Slot>(addr)
    ensures result == global<Slot>(addr).val
