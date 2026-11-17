-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport
import VerexSL.Tests.Shapes.Support

/-!
# Receiving a call's result and using its invariant, at each type shape

Each cell calls a bodiless `native fun` or an unverified `pragma opaque`
callee whose result is of the cell's shape and states in `ensures` the fact
the result's data invariant gives, the callee's contract saying nothing of
it. One cell equates the result of a struct type with a specification
function's value, where the invariant must reach that value as well. The
negative of each cell asks for a bound the invariant does not give.
-/

namespace VerexSL.Tests.Shapes.CallResults

set_option Elab.async false

verex module 0x42::shapes_call_results where
  pragma verify = false

  struct Percent has Copy, Drop, Store where
    value : u64
  spec Percent where
    invariant value <= 100

  enum Payment has Copy, Drop, Store where
    | None
    | Direct (amount : u64)
  spec Payment where
    invariant match this with
      | Payment::None {} => true
      | Payment::Direct { amount := amount } => amount > 0

  struct Pair has Copy, Drop, Store where
    first : Percent
    second : u64

  struct Bag has Copy, Drop, Store where
    items : Vector<Percent>

  struct Box {T has Copy, Drop} has Copy, Drop where
    item : T

  opaque spec fun spec_percent() : Percent

  native fun make_u64() -> u64
  spec make_u64 where
    aborts_if false
    ensures result == 7

  native fun make_percent() -> Percent
  spec make_percent where
    aborts_if false

  native fun stated_percent() -> Percent
  spec stated_percent where
    aborts_if false
    ensures result == spec_percent()

  native fun make_payment() -> Payment
  spec make_payment where
    aborts_if false

  native fun make_pair() -> Pair
  spec make_pair where
    aborts_if false

  native fun make_bag() -> Bag
  spec make_bag where
    aborts_if false

  native fun wrap {T has Copy, Drop}(item : T) -> Box<T>
  spec wrap where
    aborts_if false
    ensures result.item == item

  native fun make_box_percent() -> Box<Percent>
  spec make_box_percent where
    aborts_if false

  native fun make_two() -> (Percent, Percent)
  spec make_two where
    aborts_if false

  native fun make_maker() -> Fn(u64) -> Percent has Copy, Drop
  spec make_maker where
    aborts_if false

  -- a. scalar: a scalar states no invariant, so the callee's `ensures` is all
  fun scalar_bad() -> u64 := make_u64()
  spec scalar_bad where
    aborts_if false
    ensures result == 8

  fun scalar_ok() -> u64 := make_u64()
  spec scalar_ok where
    aborts_if false
    ensures result == 7

  -- b. a struct with a data invariant, and the same result equated to a
  -- specification function's value
  fun struct_bad() -> Percent := make_percent()
  spec struct_bad where
    aborts_if false
    ensures result.value <= 99

  fun struct_ok() -> Percent := make_percent()
  spec struct_ok where
    aborts_if false
    ensures result.value <= 100

  fun stated_bad() -> Percent := stated_percent()
  spec stated_bad where
    aborts_if false
    ensures spec_percent().value <= 99

  fun stated_ok() -> Percent := stated_percent()
  spec stated_ok where
    aborts_if false
    ensures spec_percent().value <= 100

  -- c. an enum with a per-variant invariant clause
  fun enum_bad() -> Payment := make_payment()
  spec enum_bad where
    aborts_if false
    ensures result is Direct ==> result.amount > 1

  fun enum_ok() -> Payment := make_payment()
  spec enum_ok where
    aborts_if false
    ensures result is Direct ==> result.amount > 0

  -- d. a struct holding an invariant-bearing struct, and a vector of them
  fun nested_bad() -> Pair := make_pair()
  spec nested_bad where
    aborts_if false
    ensures result.first.value <= 99

  fun nested_ok() -> Pair := make_pair()
  spec nested_ok where
    aborts_if false
    ensures result.first.value <= 100

  fun vector_bad() -> Bag := make_bag()
  spec vector_bad where
    aborts_if false
    ensures 0 < result.items.length ==> result.items[0].value <= 99

  fun vector_ok() -> Bag := make_bag()
  spec vector_ok where
    aborts_if false
    ensures 0 < result.items.length ==> result.items[0].value <= 100

  -- e. a type parameter: a struct over the caller's own parameter
  fun param_bad {T has Copy, Drop}(item : T) -> Box<T> := wrap::<T>(item)
  spec param_bad where
    aborts_if false
    ensures result.item != item

  fun param_ok {T has Copy, Drop}(item : T) -> Box<T> := wrap::<T>(item)
  spec param_ok where
    aborts_if false
    ensures result.item == item

  -- f. a generic struct at an instance
  fun instance_bad() -> Box<Percent> := make_box_percent()
  spec instance_bad where
    aborts_if false
    ensures result.item.value <= 99

  fun instance_ok() -> Box<Percent> := make_box_percent()
  spec instance_ok where
    aborts_if false
    ensures result.item.value <= 100

  -- g. a tuple of two invariant-bearing values
  fun tuple_bad() -> (Percent, Percent) := make_two()
  spec tuple_bad where
    aborts_if false
    ensures spec.result[0].value <= 99

  fun tuple_ok() -> (Percent, Percent) := make_two()
  spec tuple_ok where
    aborts_if false
    ensures spec.result[0].value <= 100 && spec.result[1].value <= 100

  -- i. a function value whose result has an invariant, invoked by the caller
  fun fn_ok(x : u64) -> Percent := invoke(make_maker(), x)
  spec fn_ok where
    aborts_if false
    ensures result.value <= 100

/-! ## Negative controls -/

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_call_results.scalar_bad ["ensures result == 8"]
    (← `(command| verify 0x42::shapes_call_results::scalar_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_call_results.struct_bad ["ensures result.value <= 99"]
    (← `(command| verify 0x42::shapes_call_results::struct_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_call_results.stated_bad ["ensures spec_percent().value <= 99"]
    (← `(command| verify 0x42::shapes_call_results::stated_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_call_results.enum_bad
    ["ensures result is Direct ==> result.amount > 1"]
    (← `(command| verify 0x42::shapes_call_results::enum_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_call_results.nested_bad ["ensures result.first.value <= 99"]
    (← `(command| verify 0x42::shapes_call_results::nested_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_call_results.vector_bad
    ["ensures 0 < result.items.length ==> result.items[0].value <= 99"]
    (← `(command| verify 0x42::shapes_call_results::vector_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_call_results.param_bad ["ensures result.item != item"]
    (← `(command| verify 0x42::shapes_call_results::param_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_call_results.instance_bad
    ["ensures result.item.value <= 99"]
    (← `(command| verify 0x42::shapes_call_results::instance_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_call_results.tuple_bad
    ["ensures spec.result[0].value <= 99"]
    (← `(command| verify 0x42::shapes_call_results::tuple_bad))

/-! ## Positive controls -/

verify 0x42::shapes_call_results::scalar_ok
verify 0x42::shapes_call_results::struct_ok
verify 0x42::shapes_call_results::enum_ok
verify 0x42::shapes_call_results::nested_ok
verify 0x42::shapes_call_results::param_ok
verify 0x42::shapes_call_results::instance_ok
verify 0x42::shapes_call_results::tuple_ok
verify 0x42::shapes_call_results::stated_ok

-- Open cells
--
-- `vector_ok`: the element invariant of a returned vector of
-- invariant-bearing structs is not available to the caller.
-- `fn_ok`: a call whose result is a function value leaves the continuation
-- after the call unestablished; the cell has no negative, which is written
-- once the continuation holds.

run_cmd do
  expectOpen `«0x42».shapes_call_results.vector_ok
    "the specification clause `ensures 0 < result.items.length ==> result.items[0].value <= 100` is not established"
    (← `(command| verify 0x42::shapes_call_results::vector_ok))

run_cmd do
  expectOpen `«0x42».shapes_call_results.fn_ok
    "the continuation after call 1 of `make_maker` is not established"
    (← `(command| verify 0x42::shapes_call_results::fn_ok))

end VerexSL.Tests.Shapes.CallResults
