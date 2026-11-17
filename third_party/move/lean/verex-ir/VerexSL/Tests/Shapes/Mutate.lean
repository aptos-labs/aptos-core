-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport
import VerexSL.Tests.Shapes.Support

/-!
# Mutating through a `&mut` and ending the mutation, at each type shape

One function per shape writes through a mutable reference and lets the
mutation end: a field write, a whole write, and an element write. The
negative of each cell writes a value the type's invariant refuses, so the
cut is owed where the mutation ends; the positive states the written value in
`ensures`, which the write's own equation gives.
-/

namespace VerexSL.Tests.Shapes.Mutate

set_option Elab.async false

verex module 0x42::shapes_mutate where
  pragma verify = false

  struct Percent has Copy, Drop, Store where
    value : u64
  spec Percent where
    invariant value <= 100

  struct Plain has Copy, Drop, Store where
    value : u64

  enum Payment has Copy, Drop, Store where
    | None
    | Direct (amount : u64)

  spec Payment where
    invariant match this with
      | Payment::None {} => true
      | Payment::Direct { amount := amount } => amount > 0

  struct Pair has Copy, Drop, Store where
    first : Percent
    second : Plain

  struct Bag has Copy, Drop, Store where
    items : Vector<Percent>

  struct Box {T has Copy, Drop} has Copy, Drop where
    item : T

  struct Op has Copy, Drop where
    f : Fn(u64) -> Percent has Copy, Drop

  fun fifty(x : u64) -> Percent := new Percent { value := 50 }
  spec fifty where
    aborts_if false
    ensures result.value == 50

  -- a. scalar, a whole write through `&mut u64`
  fun scalar_bad(slot : &mut u64) -> Unit := *slot := 7
  spec scalar_bad where
    aborts_if false
    ensures *slot == 8

  fun scalar_ok(slot : &mut u64) -> Unit := *slot := 7
  spec scalar_ok where
    aborts_if false
    ensures *slot == 7

  -- b. a struct with a data invariant: a field write and a whole write; and a
  -- struct without one, where no fact is owed at the mutation's end
  fun field_write_bad(p : &mut Percent) -> Unit := p.value := 101
  spec field_write_bad where
    aborts_if false

  fun field_write_ok(p : &mut Percent, amount : u64) -> Unit := p.value := amount
  spec field_write_ok where
    requires amount <= 100
    aborts_if false
    ensures p.value == amount

  fun whole_write_bad(p : &mut Percent) -> Unit := *p := new Percent { value := 101 }
  spec whole_write_bad where
    aborts_if false

  fun whole_write_ok(p : &mut Percent, amount : u64) -> Unit :=
    *p := new Percent { value := amount }
  spec whole_write_ok where
    requires amount <= 100
    aborts_if false
    ensures p.value == amount

  fun plain_write_bad(p : &mut Plain) -> Unit := p.value := 101
  spec plain_write_bad where
    aborts_if false
    ensures p.value <= 100

  fun plain_write_ok(p : &mut Plain) -> Unit := p.value := 101
  spec plain_write_ok where
    aborts_if false
    ensures p.value == 101

  -- c. an enum with a per-variant invariant clause: a whole write of a variant
  fun enum_write_bad(p : &mut Payment) -> Unit := *p := new Payment::Direct { amount := 0 }
  spec enum_write_bad where
    aborts_if false

  fun enum_write_ok(p : &mut Payment, amount : u64) -> Unit :=
    *p := new Payment::Direct { amount }
  spec enum_write_ok where
    requires amount > 0
    aborts_if false
    ensures *p is Direct

  -- d. a struct holding an invariant-bearing struct, and a vector of them
  fun nested_write_bad(p : &mut Pair) -> Unit := p.first.value := 101
  spec nested_write_bad where
    aborts_if false

  fun nested_write_ok(p : &mut Pair, amount : u64) -> Unit := p.first.value := amount
  spec nested_write_ok where
    requires amount <= 100
    aborts_if false
    ensures p.first.value == amount

  fun element_write_bad(b : &mut Bag) -> Unit := b.items[0].value := 101
  spec element_write_bad where
    requires 0 < b.items.length

  fun element_write_ok(b : &mut Bag, amount : u64) -> Unit := b.items[0].value := amount
  spec element_write_ok where
    requires 0 < b.items.length && amount <= 100
    aborts_if false
    ensures b.items[0].value == amount

  -- e. a type parameter
  fun param_write_bad {T has Copy, Drop}(slot : &mut T, item : T) -> Unit := *slot := item
  spec param_write_bad where
    aborts_if false
    ensures *slot != item

  fun param_write_ok {T has Copy, Drop}(slot : &mut T, item : T) -> Unit := *slot := item
  spec param_write_ok where
    aborts_if false
    ensures *slot == item

  -- f. a generic struct at an instance
  fun instance_write_bad(b : &mut Box<Percent>) -> Unit := b.item.value := 101
  spec instance_write_bad where
    aborts_if false

  fun instance_write_ok(b : &mut Box<Percent>, amount : u64) -> Unit := b.item.value := amount
  spec instance_write_ok where
    requires amount <= 100
    aborts_if false
    ensures b.item.value == amount

  -- g. a tuple of two invariant-bearing values: two mutations ending at the
  -- returned pair of references
  fun tuple_write_bad(a : &mut Percent, b : &mut Percent) -> (u64, u64) := do
    a.value := 101
    b.value := 1
    (a.value, b.value)
  spec tuple_write_bad where
    aborts_if false

  fun tuple_write_ok(a : &mut Percent, b : &mut Percent, amount : u64) -> (u64, u64) := do
    a.value := amount
    b.value := amount
    (a.value, b.value)
  spec tuple_write_ok where
    requires amount <= 100
    aborts_if false
    ensures spec.result[0] == amount && spec.result[1] == amount

  -- h. a `&mut` of an invariant-bearing struct, reborrowed to its field
  fun reborrow_write_bad(p : &mut Percent) -> Unit := do
    let slot := &mut p.value
    *slot := 101
  spec reborrow_write_bad where
    aborts_if false

  fun reborrow_write_ok(p : &mut Percent, amount : u64) -> Unit := do
    let slot := &mut p.value
    *slot := amount
  spec reborrow_write_ok where
    requires amount <= 100
    aborts_if false
    ensures p.value == amount

  -- i. a function-typed field, written whole
  fun fn_field_write_ok(o : &mut Op) -> Unit :=
    o.f := function[Fn(u64) -> Percent has Copy, Drop](fifty)
  spec fn_field_write_ok where
    aborts_if false

/-! ## Negative controls -/

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_mutate.scalar_bad ["ensures *slot == 8"]
    (← `(command| verify 0x42::shapes_mutate::scalar_bad))

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  expectCuts `«0x42».shapes_mutate.field_write_bad [(percent, exit)]
    (← `(command| verify 0x42::shapes_mutate::field_write_bad)) (some [])

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  expectCuts `«0x42».shapes_mutate.whole_write_bad [(percent, construction)]
    (← `(command| verify 0x42::shapes_mutate::whole_write_bad)) (some [])

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_mutate.plain_write_bad ["ensures p.value <= 100"]
    (← `(command| verify 0x42::shapes_mutate::plain_write_bad))

open LoanDeathControls in
run_cmd do
  let payment := "invariant match this with\n" ++
    "      | Payment::None {} => true\n" ++
    "      | Payment::Direct { amount := amount } => amount > 0"
  expectCuts `«0x42».shapes_mutate.enum_write_bad [(payment, construction)]
    (← `(command| verify 0x42::shapes_mutate::enum_write_bad)) (some [])

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  expectCuts `«0x42».shapes_mutate.nested_write_bad [(percent, exit)]
    (← `(command| verify 0x42::shapes_mutate::nested_write_bad)) (some [])

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  expectCuts `«0x42».shapes_mutate.element_write_bad [(percent, exit)]
    (← `(command| verify 0x42::shapes_mutate::element_write_bad)) (some [])

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_mutate.param_write_bad ["ensures *slot != item"]
    (← `(command| verify 0x42::shapes_mutate::param_write_bad))

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  expectCuts `«0x42».shapes_mutate.instance_write_bad [(percent, exit)]
    (← `(command| verify 0x42::shapes_mutate::instance_write_bad)) (some [])

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  expectCuts `«0x42».shapes_mutate.tuple_write_bad [(percent, exit)]
    (← `(command| verify 0x42::shapes_mutate::tuple_write_bad)) (some [])

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  expectCuts `«0x42».shapes_mutate.reborrow_write_bad [(percent, exit)]
    (← `(command| verify 0x42::shapes_mutate::reborrow_write_bad)) (some [])

/-! ## Positive controls -/

verify 0x42::shapes_mutate::scalar_ok
verify 0x42::shapes_mutate::field_write_ok
verify 0x42::shapes_mutate::whole_write_ok
verify 0x42::shapes_mutate::plain_write_ok
verify 0x42::shapes_mutate::enum_write_ok
verify 0x42::shapes_mutate::nested_write_ok
verify 0x42::shapes_mutate::param_write_ok
verify 0x42::shapes_mutate::instance_write_ok
verify 0x42::shapes_mutate::tuple_write_ok
verify 0x42::shapes_mutate::reborrow_write_ok

-- Open cells
--
-- `element_write_ok`: an element write through a `&mut` of a struct holding a
-- vector of invariant-bearing structs leaves the element invariant owed where
-- the parameter's loan ends, although the written value meets it.
-- `fn_field_write_ok`: a whole write of a function-typed field leaves the
-- field's own function type owed as a clause. This cell has no negative: a
-- false clause beside that one would not be the only reason the cell fails,
-- and the negative is written once the cell is established.

run_cmd do
  expectOpen `«0x42».shapes_mutate.element_write_ok
    "the data invariant `invariant value <= 100` does not hold where a mutable parameter's loan ends, at the function's exit"
    (← `(command| verify 0x42::shapes_mutate::element_write_ok))

run_cmd do
  expectOpen `«0x42».shapes_mutate.fn_field_write_ok
    "the specification clause `f : Fn(u64) -> Percent has Copy, Drop` is not established"
    (← `(command| verify 0x42::shapes_mutate::fn_field_write_ok))

end VerexSL.Tests.Shapes.Mutate
