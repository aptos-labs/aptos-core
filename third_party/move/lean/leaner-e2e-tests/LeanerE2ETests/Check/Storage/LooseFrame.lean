-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! A mixed wildcard in `modifies` opens
unlisted families, not the other keys of a listed family. -/

namespace LeanerLang.Tests.Check.Storage.LooseFrame

leaner module 0x42::loose_frame where
  struct Counter has Key where
    value : u64
  struct Ledger has Key where
    total : u64

  fun bump(addr : Address) -> Unit := do
    let value := &mut Counter[addr].value
    *value := *value + 1
  spec bump where
    requires exists<Counter>(addr) && old(global<Counter>(addr).value) + 1 <= MAX_U64
    modifies global<Counter>(addr), *
    ensures global<Counter>(addr).value == old(global<Counter>(addr).value) + 1
    aborts_if false

  fun guarded_bump(addr : Address, limit : u64) -> Unit := do
    let value := &mut Counter[addr].value
    let current := *value
    assert!(current < limit, 7)
    *value := *value + 1
  spec guarded_bump where
    requires exists<Counter>(addr)
    modifies *
    ensures global<Counter>(addr).value == old(global<Counter>(addr).value) + 1
    aborts_if limit <= old(global<Counter>(addr).value) with 7

  -- The wildcard really permits an unlisted family to change.
  fun bump_with_ledger(addr : Address) -> Unit := do
    *(&mut Counter[addr].value) := 7
    *(&mut Ledger[addr].total) := 9
  spec bump_with_ledger where
    requires exists<Counter>(addr) && exists<Ledger>(addr)
    modifies global<Counter>(addr), *
    ensures global<Counter>(addr).value == 7 && global<Ledger>(addr).total == 9
    aborts_if false

-- The mixed wildcard must not silently turn the listed family fully open.
example [LeanerIR.Proofs.Denote.Skolems] (initial final : LeanerIR.RuntimeState)
    (addr other : String)
    (different : other ≠ addr)
    (frame : «0x42».loose_frame.bump.typedContract.frame (addr, ()) initial final) :
    final.globals.lookup («0x42».loose_frame.Counter.key (.address other)) =
      initial.globals.lookup («0x42».loose_frame.Counter.key (.address other)) := by
  apply frame (.address other)
  intro same
  exact different (LeanerIR.StorageKey.address.inj same)

end LeanerLang.Tests.Check.Storage.LooseFrame
