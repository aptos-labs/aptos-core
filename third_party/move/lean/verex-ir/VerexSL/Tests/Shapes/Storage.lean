-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport
import VerexSL.Tests.Shapes.Support

/-!
# Publishing, reading, taking and testing a resource, at each type shape

The storage mechanisms that cross a value in or out of global memory:
`move_to`, a keyed read, `move_from`, and `exists`. Each cell states in
`ensures` the fact the resource's data invariant gives of the value it reads
out of memory, or the obligation the publish owes of the value it puts in.
The borrowing mechanisms are in `Shapes/StorageBorrows.lean`.

A resource is a struct or an enum with `Key`, so the scalar, tuple,
`&mut` and function-value shapes have no cell of their own here: a scalar
resource is the struct without an invariant, and a reference or a tuple is
not a resource type.
-/

namespace VerexSL.Tests.Shapes.Storage

set_option Elab.async false

verex module 0x42::shapes_storage where
  pragma verify = false

  struct Percent has Copy, Drop, Store where
    value : u64
  spec Percent where
    invariant value <= 100

  -- b. a resource with a data invariant, and one without
  struct Gauge has Key where
    level : u64
  spec Gauge where
    invariant level <= 100

  struct Plain has Key where
    level : u64

  -- c. an enum resource with a per-variant invariant clause
  enum Slot has Key where
    | Empty (val : u64)
    | Full (val : u64)
  spec Slot where
    invariant match this with
      | Slot::Empty { val := val } => val == 0
      | Slot::Full { val := val } => val > 0

  -- d. a resource holding an invariant-bearing struct, and one holding a
  -- vector of them
  struct Holder has Key where
    percent : Percent

  struct Shelf has Key where
    items : Vector<Percent>

  -- f. a generic resource at an instance and at a caller's own parameter
  struct Coin {C : phantom type} has Key where
    value : u64
  spec Coin where
    invariant value <= 100

  struct Gold has Copy, Drop, Store where
    dummy : Bool

  -- 6.1 publish
  fun publish_bad(s : &Signer) -> Unit := move_to<Gauge>(s, new Gauge { level := 101 })
  spec publish_bad where
    requires !exists<Gauge>(s.address)
    modifies global<Gauge>(s.address)
    aborts_if false

  fun publish_ok(s : &Signer, amount : u64) -> Unit :=
    move_to<Gauge>(s, new Gauge { level := amount })
  spec publish_ok where
    requires !exists<Gauge>(s.address) && amount <= 100
    modifies global<Gauge>(s.address)
    aborts_if false
    ensures global<Gauge>(s.address).level == amount

  fun publish_enum_bad(s : &Signer) -> Unit := move_to<Slot>(s, new Slot::Full { val := 0 })
  spec publish_enum_bad where
    requires !exists<Slot>(s.address)
    modifies global<Slot>(s.address)
    aborts_if false

  fun publish_enum_ok(s : &Signer, amount : u64) -> Unit :=
    move_to<Slot>(s, new Slot::Full { val := amount })
  spec publish_enum_ok where
    requires !exists<Slot>(s.address) && amount > 0
    modifies global<Slot>(s.address)
    aborts_if false
    ensures global<Slot>(s.address) is Full

  fun publish_nested_bad(s : &Signer) -> Unit :=
    move_to<Holder>(s, new Holder { percent := new Percent { value := 101 } })
  spec publish_nested_bad where
    requires !exists<Holder>(s.address)
    modifies global<Holder>(s.address)
    aborts_if false

  fun publish_nested_ok(s : &Signer, p : Percent) -> Unit :=
    move_to<Holder>(s, new Holder { percent := p })
  spec publish_nested_ok where
    requires !exists<Holder>(s.address)
    modifies global<Holder>(s.address)
    aborts_if false
    ensures global<Holder>(s.address).percent.value <= 100

  -- 6.2 a keyed read
  fun read_bad(addr : Address) -> u64 := Gauge[addr].level
  spec read_bad where
    aborts_if !exists<Gauge>(addr)
    ensures result <= 99

  fun read_ok(addr : Address) -> u64 := Gauge[addr].level
  spec read_ok where
    aborts_if !exists<Gauge>(addr)
    ensures result <= 100
    ensures result == global<Gauge>(addr).level

  fun read_plain_bad(addr : Address) -> u64 := Plain[addr].level
  spec read_plain_bad where
    aborts_if !exists<Plain>(addr)
    ensures result <= 100

  fun read_plain_ok(addr : Address) -> u64 := Plain[addr].level
  spec read_plain_ok where
    aborts_if !exists<Plain>(addr)
    ensures result == global<Plain>(addr).level

  fun read_enum_bad(addr : Address) -> u64 := Slot[addr].val
  spec read_enum_bad where
    aborts_if !exists<Slot>(addr)
    ensures result == global<Slot>(addr).val + 1

  fun read_enum_ok(addr : Address) -> u64 := Slot[addr].val
  spec read_enum_ok where
    aborts_if !exists<Slot>(addr)
    ensures result == global<Slot>(addr).val

  fun read_nested_bad(addr : Address) -> u64 := Holder[addr].percent.value
  spec read_nested_bad where
    aborts_if !exists<Holder>(addr)
    ensures result <= 99

  fun read_nested_ok(addr : Address) -> u64 := Holder[addr].percent.value
  spec read_nested_ok where
    aborts_if !exists<Holder>(addr)
    ensures result <= 100

  fun read_element_bad(addr : Address) -> u64 := Shelf[addr].items[0].value
  spec read_element_bad where
    requires 0 < global<Shelf>(addr).items.length
    aborts_if !exists<Shelf>(addr)
    ensures result <= 99

  fun read_element_ok(addr : Address) -> u64 := Shelf[addr].items[0].value
  spec read_element_ok where
    requires 0 < global<Shelf>(addr).items.length
    aborts_if !exists<Shelf>(addr)
    ensures result <= 100

  fun read_instance_bad(addr : Address) -> u64 := Coin<Gold>[addr].value
  spec read_instance_bad where
    aborts_if !exists<Coin<Gold> >(addr)
    ensures result <= 99

  fun read_instance_ok(addr : Address) -> u64 := Coin<Gold>[addr].value
  spec read_instance_ok where
    aborts_if !exists<Coin<Gold> >(addr)
    ensures result <= 100

  fun read_param_bad {C}(addr : Address) -> u64 := Coin<C>[addr].value
  spec read_param_bad where
    aborts_if !exists<Coin<C> >(addr)
    ensures result <= 99

  fun read_param_ok {C}(addr : Address) -> u64 := Coin<C>[addr].value
  spec read_param_ok where
    aborts_if !exists<Coin<C> >(addr)
    ensures result <= 100

  -- 6.3 take
  fun take_bad(addr : Address) -> u64 := do
    let Gauge { level := level } := move_from<Gauge>(addr)
    level
  spec take_bad where
    aborts_if !exists<Gauge>(addr)
    modifies global<Gauge>(addr)
    ensures result <= 99

  fun take_ok(addr : Address) -> u64 := do
    let Gauge { level := level } := move_from<Gauge>(addr)
    level
  spec take_ok where
    aborts_if !exists<Gauge>(addr)
    modifies global<Gauge>(addr)
    ensures result <= 100
    ensures !exists<Gauge>(addr)

  -- 6.4 presence
  fun exists_bad(addr : Address) -> Bool := exists<Gauge>(addr)
  spec exists_bad where
    aborts_if false
    ensures result == !exists<Gauge>(addr)

  fun exists_ok(addr : Address) -> Bool := exists<Gauge>(addr)
  spec exists_ok where
    aborts_if false
    ensures result == exists<Gauge>(addr)

  fun exists_instance_ok(addr : Address) -> Bool := exists<Coin<Gold> >(addr)
  spec exists_instance_ok where
    aborts_if false
    ensures result == exists<Coin<Gold> >(addr)

/-! ## Negative controls -/

open LoanDeathControls in
run_cmd do
  let gauge := "invariant level <= 100"
  expectCuts `«0x42».shapes_storage.publish_bad [(gauge, construction)]
    (← `(command| verify 0x42::shapes_storage::publish_bad)) (some [])

open LoanDeathControls in
run_cmd do
  let slot := "invariant match this with\n" ++
    "      | Slot::Empty { val := val } => val == 0\n" ++
    "      | Slot::Full { val := val } => val > 0"
  expectCuts `«0x42».shapes_storage.publish_enum_bad [(slot, construction)]
    (← `(command| verify 0x42::shapes_storage::publish_enum_bad)) (some [])

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  expectCuts `«0x42».shapes_storage.publish_nested_bad [(percent, construction)]
    (← `(command| verify 0x42::shapes_storage::publish_nested_bad)) (some [])

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_storage.read_bad ["ensures result <= 99"]
    (← `(command| verify 0x42::shapes_storage::read_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_storage.read_plain_bad ["ensures result <= 100"]
    (← `(command| verify 0x42::shapes_storage::read_plain_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_storage.read_nested_bad ["ensures result <= 99"]
    (← `(command| verify 0x42::shapes_storage::read_nested_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_storage.read_element_bad ["ensures result <= 99"]
    (← `(command| verify 0x42::shapes_storage::read_element_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_storage.read_instance_bad ["ensures result <= 99"]
    (← `(command| verify 0x42::shapes_storage::read_instance_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_storage.read_param_bad ["ensures result <= 99"]
    (← `(command| verify 0x42::shapes_storage::read_param_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_storage.take_bad ["ensures result <= 99"]
    (← `(command| verify 0x42::shapes_storage::take_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_storage.exists_bad
    ["ensures result == !exists<Gauge>(addr)"]
    (← `(command| verify 0x42::shapes_storage::exists_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_storage.read_enum_bad
    ["ensures result == global<Slot>(addr).val + 1"]
    (← `(command| verify 0x42::shapes_storage::read_enum_bad))

/-! ## Positive controls -/

verify 0x42::shapes_storage::publish_ok
verify 0x42::shapes_storage::publish_enum_ok
verify 0x42::shapes_storage::publish_nested_ok
verify 0x42::shapes_storage::read_ok
verify 0x42::shapes_storage::read_plain_ok
verify 0x42::shapes_storage::read_nested_ok
verify 0x42::shapes_storage::read_instance_ok
verify 0x42::shapes_storage::read_param_ok
verify 0x42::shapes_storage::take_ok
verify 0x42::shapes_storage::exists_ok
verify 0x42::shapes_storage::exists_instance_ok
verify 0x42::shapes_storage::read_enum_ok

end VerexSL.Tests.Shapes.Storage
