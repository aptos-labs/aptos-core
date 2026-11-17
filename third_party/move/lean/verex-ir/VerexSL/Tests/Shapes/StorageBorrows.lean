-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport
import VerexSL.Tests.Shapes.Support

/-!
# Writing a resource in place, and reading memory after a write

A mutable borrow of a resource, a write through it and the end of the loan,
and then the reads that follow a write: of the written resource at the same
key, of the same resource type at another key, and of another resource type.
A read after a write is of the memory the write made; the data invariant of
the value it reads has to reach it there, since the body did not write that
value. The same reads follow a publish and a call that modifies memory.

A positive states the fact the invariant gives of the value read, or the
value the write stored; the negative asks for more than the invariant, or
writes a value the invariant refuses.
-/

namespace VerexSL.Tests.Shapes.StorageBorrows

set_option Elab.async false

verex module 0x42::shapes_storage_borrows where
  pragma verify = false

  struct Tally has Key where
    count : u64
  spec Tally where
    invariant count >= 10

  struct Other has Key where
    n : u64
  spec Other where
    invariant n >= 10

  -- 6.5 a write through a mutable borrow, the loan ended
  fun write_bad(addr : Address) -> Unit := do
    let t := &mut Tally[addr]
    t.count := 9
  spec write_bad where
    requires exists<Tally>(addr)
    modifies global<Tally>(addr)
    aborts_if false

  fun write_ok(addr : Address, amount : u64) -> Unit := do
    let t := &mut Tally[addr]
    t.count := amount
  spec write_ok where
    requires exists<Tally>(addr) && amount >= 10
    modifies global<Tally>(addr)
    aborts_if false
    ensures global<Tally>(addr).count == amount

  -- 6.6 a read of the written resource at the same key
  fun read_same_key_bad(addr : Address) -> u64 := do
    let t := &mut Tally[addr]
    t.count := 12
    Tally[addr].count
  spec read_same_key_bad where
    requires exists<Tally>(addr)
    modifies global<Tally>(addr)
    aborts_if false
    ensures result == 13

  fun read_same_key_ok(addr : Address) -> u64 := do
    let t := &mut Tally[addr]
    t.count := 12
    Tally[addr].count
  spec read_same_key_ok where
    requires exists<Tally>(addr)
    modifies global<Tally>(addr)
    aborts_if false
    ensures result == 12

  -- 6.7 a read of the written resource type at another key
  fun read_other_key_bad(addr : Address, other : Address) -> u64 := do
    let t := &mut Tally[addr]
    t.count := 12
    Tally[other].count
  spec read_other_key_bad where
    pragma aborts_if_is_partial
    modifies global<Tally>(addr)
    ensures result >= 11

  fun read_other_key_ok(addr : Address, other : Address) -> u64 := do
    let t := &mut Tally[addr]
    t.count := 12
    Tally[other].count
  spec read_other_key_ok where
    pragma aborts_if_is_partial
    modifies global<Tally>(addr)
    ensures result >= 10

  -- 6.8 a read of another resource type after the write
  fun read_other_resource_bad(addr : Address) -> u64 := do
    let t := &mut Tally[addr]
    t.count := 12
    Other[addr].n
  spec read_other_resource_bad where
    pragma aborts_if_is_partial
    modifies global<Tally>(addr)
    ensures result >= 11

  fun read_other_resource_ok(addr : Address) -> u64 := do
    let t := &mut Tally[addr]
    t.count := 12
    Other[addr].n
  spec read_other_resource_ok where
    pragma aborts_if_is_partial
    modifies global<Tally>(addr)
    ensures result >= 10

  -- 6.9 a read after a publish
  fun read_after_publish_bad(s : &Signer, addr : Address) -> u64 := do
    move_to<Tally>(s, new Tally { count := 10 })
    Other[addr].n
  spec read_after_publish_bad where
    pragma aborts_if_is_partial
    modifies global<Tally>(s.address)
    ensures result >= 11

  fun read_after_publish_ok(s : &Signer, addr : Address) -> u64 := do
    move_to<Tally>(s, new Tally { count := 10 })
    Other[addr].n
  spec read_after_publish_ok where
    pragma aborts_if_is_partial
    modifies global<Tally>(s.address)
    ensures result >= 10

  -- 6.10 a read after a call that modifies memory
  fun bump(addr : Address) -> Unit := do
    let t := &mut Tally[addr]
    t.count := 12
  spec bump where
    pragma opaque
    pragma aborts_if_is_partial
    modifies global<Tally>(addr)

  fun read_after_call_bad(addr : Address, other : Address) -> u64 := do
    bump(addr)
    Tally[other].count
  spec read_after_call_bad where
    pragma aborts_if_is_partial
    modifies global<Tally>(addr)
    ensures result >= 11

  fun read_after_call_ok(addr : Address, other : Address) -> u64 := do
    bump(addr)
    Tally[other].count
  spec read_after_call_ok where
    pragma aborts_if_is_partial
    modifies global<Tally>(addr)
    ensures result >= 10

/-! ## Negative controls -/

open LoanDeathControls in
run_cmd do
  expectCuts `«0x42».shapes_storage_borrows.write_bad
    [("invariant count >= 10", mutationEnd)]
    (← `(command| verify 0x42::shapes_storage_borrows::write_bad)) (some [])

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_storage_borrows.read_same_key_bad
    ["ensures result == 13"]
    (← `(command| verify 0x42::shapes_storage_borrows::read_same_key_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_storage_borrows.read_other_key_bad
    ["ensures result >= 11"]
    (← `(command| verify 0x42::shapes_storage_borrows::read_other_key_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_storage_borrows.read_other_resource_bad
    ["ensures result >= 11"]
    (← `(command| verify 0x42::shapes_storage_borrows::read_other_resource_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_storage_borrows.read_after_publish_bad
    ["ensures result >= 11"]
    (← `(command| verify 0x42::shapes_storage_borrows::read_after_publish_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_storage_borrows.read_after_call_bad
    ["ensures result >= 11"]
    (← `(command| verify 0x42::shapes_storage_borrows::read_after_call_bad))

/-! ## Positive controls -/

verify 0x42::shapes_storage_borrows::write_ok
verify 0x42::shapes_storage_borrows::read_same_key_ok
verify 0x42::shapes_storage_borrows::read_other_key_ok
verify 0x42::shapes_storage_borrows::read_other_resource_ok
verify 0x42::shapes_storage_borrows::read_after_publish_ok
verify 0x42::shapes_storage_borrows::read_after_call_ok

end VerexSL.Tests.Shapes.StorageBorrows
