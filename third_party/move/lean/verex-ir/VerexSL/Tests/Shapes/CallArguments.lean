-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport
import VerexSL.Tests.Shapes.Support

/-!
# Passing a value to a contract call, at each type shape

Each cell passes a value of its shape to an unverified `pragma opaque`
callee and states in `ensures` what the callee's contract and the argument's
own data invariant give together. The negative of each cell asks for more
than the two give, or hands over a value the invariant refuses.
-/

namespace VerexSL.Tests.Shapes.CallArguments

set_option Elab.async false

verex module 0x42::shapes_call_arguments where
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

  -- The opaque callees: a caller sees each only through its contract.
  fun succ(x : u64) -> u64 := x + 1
  spec succ where
    pragma opaque = true
    pragma verify = false
    aborts_if false
    ensures result == x + 1

  fun value_of(p : Percent) -> u64 := p.value
  spec value_of where
    pragma opaque = true
    pragma verify = false
    aborts_if false
    ensures result == p.value

  fun plain_value_of(p : Plain) -> u64 := p.value
  spec plain_value_of where
    pragma opaque = true
    pragma verify = false
    aborts_if false
    ensures result == p.value

  fun amount_of(p : Payment) -> u64 :=
    match p with
      | Payment::None {} => 0
      | Payment::Direct { amount := amount } => amount
  spec amount_of where
    pragma opaque = true
    pragma verify = false
    aborts_if false
    ensures result == (match p with
      | Payment::None {} => 0
      | Payment::Direct { amount := amount } => amount)

  fun first_of(p : Pair) -> u64 := p.first.value
  spec first_of where
    pragma opaque = true
    pragma verify = false
    aborts_if false
    ensures result == p.first.value

  fun head_of(b : Bag) -> u64 := b.items[0].value
  spec head_of where
    pragma opaque = true
    pragma verify = false
    requires 0 < b.items.length
    aborts_if false
    ensures result == b.items[0].value

  fun echo {T has Copy, Drop}(item : T) -> T := item
  spec echo where
    pragma opaque = true
    pragma verify = false
    aborts_if false
    ensures result == item

  fun unbox {T has Copy, Drop}(b : Box<T>) -> T := b.item
  spec unbox where
    pragma opaque = true
    pragma verify = false
    aborts_if false
    ensures result == b.item

  fun both_values(p : Percent, q : Percent) -> (u64, u64) := (p.value, q.value)
  spec both_values where
    pragma opaque = true
    pragma verify = false
    aborts_if false
    ensures spec.result[0] == p.value && spec.result[1] == q.value

  fun sum_values(p : Percent, q : Percent) -> u64 := p.value + q.value
  spec sum_values where
    pragma opaque = true
    pragma verify = false
    aborts_if false
    ensures result == p.value + q.value

  fun set_to(p : &mut Percent, amount : u64) -> Unit := p.value := amount
  spec set_to where
    pragma opaque = true
    pragma verify = false
    requires amount <= 100
    aborts_if false
    ensures p.value == amount

  fun run(f : Fn(u64) -> Percent has Copy, Drop, x : u64) -> Percent := invoke(f, x)
  spec run where
    pragma opaque = true
    pragma verify = false
    aborts_if false
    ensures ensures_of<f>(x, result)

  fun fifty(x : u64) -> Percent := new Percent { value := 50 }
  spec fifty where
    aborts_if false
    ensures result.value == 50

  -- a. scalar
  fun scalar_bad(x : u64) -> u64 := succ(x)
  spec scalar_bad where
    aborts_if false
    ensures result == x + 2

  fun scalar_ok(x : u64) -> u64 := succ(x)
  spec scalar_ok where
    aborts_if false
    ensures result == x + 1

  -- b. a struct with a data invariant, and one without: the bound of the
  -- result comes from the argument's invariant through the callee's `ensures`
  fun struct_bad(p : Percent) -> u64 := value_of(p)
  spec struct_bad where
    aborts_if false
    ensures result <= 99

  fun struct_ok(p : Percent) -> u64 := value_of(p)
  spec struct_ok where
    aborts_if false
    ensures result <= 100

  fun plain_bad(p : Plain) -> u64 := plain_value_of(p)
  spec plain_bad where
    aborts_if false
    ensures result <= 100

  fun plain_ok(p : Plain) -> u64 := plain_value_of(p)
  spec plain_ok where
    aborts_if false
    ensures result == p.value

  -- c. an enum with a per-variant invariant clause
  fun enum_bad(p : Payment) -> u64 := amount_of(p)
  spec enum_bad where
    requires p is Direct
    aborts_if false
    ensures result > 1

  fun enum_ok(p : Payment) -> u64 := amount_of(p)
  spec enum_ok where
    requires p is Direct
    aborts_if false
    ensures result > 0

  -- d. a struct holding an invariant-bearing struct, and a vector of them
  fun nested_bad(p : Pair) -> u64 := first_of(p)
  spec nested_bad where
    aborts_if false
    ensures result <= 99

  fun nested_ok(p : Pair) -> u64 := first_of(p)
  spec nested_ok where
    aborts_if false
    ensures result <= 100

  fun vector_bad(b : Bag) -> u64 := head_of(b)
  spec vector_bad where
    requires 0 < b.items.length
    aborts_if false
    ensures result <= 99

  fun vector_ok(b : Bag) -> u64 := head_of(b)
  spec vector_ok where
    requires 0 < b.items.length
    aborts_if false
    ensures result <= 100

  -- e. a type parameter: the argument is the caller's own parameter
  fun param_bad {T has Copy, Drop}(item : T) -> T := echo::<T>(item)
  spec param_bad where
    aborts_if false
    ensures result != item

  fun param_ok {T has Copy, Drop}(item : T) -> T := echo::<T>(item)
  spec param_ok where
    aborts_if false
    ensures result == item

  -- f. a generic struct at an instance
  fun instance_bad(b : Box<Percent>) -> Percent := unbox::<Percent>(b)
  spec instance_bad where
    aborts_if false
    ensures result.value <= 99

  fun instance_ok(b : Box<Percent>) -> Percent := unbox::<Percent>(b)
  spec instance_ok where
    aborts_if false
    ensures result.value <= 100

  -- g. a tuple of two invariant-bearing values
  fun tuple_bad(p : Percent, q : Percent) -> (u64, u64) := both_values(p, q)
  spec tuple_bad where
    aborts_if false
    ensures spec.result[0] <= 99 && spec.result[1] <= 100

  fun tuple_ok(p : Percent, q : Percent) -> (u64, u64) := both_values(p, q)
  spec tuple_ok where
    aborts_if false
    ensures spec.result[0] <= 100 && spec.result[1] <= 100

  -- g'. two invariant-bearing arguments of one call, the result a scalar
  fun two_args_bad(p : Percent, q : Percent) -> u64 := sum_values(p, q)
  spec two_args_bad where
    aborts_if false
    ensures result <= 199

  fun two_args_ok(p : Percent, q : Percent) -> u64 := sum_values(p, q)
  spec two_args_ok where
    aborts_if false
    ensures result <= 200

  -- h. a `&mut` of an invariant-bearing struct handed over to the call
  fun mut_bad(p : &mut Percent) -> Unit := do
    p.value := 101
    set_to(p, 50)
  spec mut_bad where
    aborts_if false

  fun mut_ok(p : &mut Percent) -> Unit := set_to(p, 50)
  spec mut_ok where
    aborts_if false
    ensures p.value == 50

  -- i. a function value whose result has an invariant
  fun fn_bad(x : u64) -> Percent := run(function[Fn(u64) -> Percent has Copy, Drop](fifty), x)
  spec fn_bad where
    aborts_if false
    ensures result.value == 40

  fun fn_ok(x : u64) -> Percent := run(function[Fn(u64) -> Percent has Copy, Drop](fifty), x)
  spec fn_ok where
    aborts_if false
    ensures result.value == 50

/-! ## The function a cell passes as a value

A call that passes a named function to a callee whose function-typed
parameter returns a type with a data invariant pays the validity of that
function's results from its theorem. -/

verify 0x42::shapes_call_arguments::fifty

/-! ## Negative controls -/

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_call_arguments.scalar_bad ["ensures result == x + 2"]
    (← `(command| verify 0x42::shapes_call_arguments::scalar_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_call_arguments.struct_bad ["ensures result <= 99"]
    (← `(command| verify 0x42::shapes_call_arguments::struct_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_call_arguments.plain_bad ["ensures result <= 100"]
    (← `(command| verify 0x42::shapes_call_arguments::plain_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_call_arguments.enum_bad ["ensures result > 1"]
    (← `(command| verify 0x42::shapes_call_arguments::enum_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_call_arguments.nested_bad ["ensures result <= 99"]
    (← `(command| verify 0x42::shapes_call_arguments::nested_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_call_arguments.vector_bad ["ensures result <= 99"]
    (← `(command| verify 0x42::shapes_call_arguments::vector_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_call_arguments.param_bad ["ensures result != item"]
    (← `(command| verify 0x42::shapes_call_arguments::param_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_call_arguments.instance_bad ["ensures result.value <= 99"]
    (← `(command| verify 0x42::shapes_call_arguments::instance_bad))

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  expectCuts `«0x42».shapes_call_arguments.mut_bad [(percent, handOver)]
    (← `(command| verify 0x42::shapes_call_arguments::mut_bad)) (some [])

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_call_arguments.fn_bad ["ensures result.value == 40"]
    (← `(command| verify 0x42::shapes_call_arguments::fn_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_call_arguments.tuple_bad
    ["ensures spec.result[0] <= 99 && spec.result[1] <= 100"]
    (← `(command| verify 0x42::shapes_call_arguments::tuple_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_call_arguments.two_args_bad ["ensures result <= 199"]
    (← `(command| verify 0x42::shapes_call_arguments::two_args_bad))

/-! ## Positive controls -/

verify 0x42::shapes_call_arguments::scalar_ok
verify 0x42::shapes_call_arguments::struct_ok
verify 0x42::shapes_call_arguments::plain_ok
verify 0x42::shapes_call_arguments::enum_ok
verify 0x42::shapes_call_arguments::nested_ok
verify 0x42::shapes_call_arguments::param_ok
verify 0x42::shapes_call_arguments::instance_ok
verify 0x42::shapes_call_arguments::mut_ok
verify 0x42::shapes_call_arguments::fn_ok
verify 0x42::shapes_call_arguments::two_args_ok
verify 0x42::shapes_call_arguments::tuple_ok

-- Open cells
--
-- `vector_ok`: the element invariant of a `Vector<Percent>` held by a struct
-- passed to the call does not reach the callee's `ensures` about it.

run_cmd do
  expectOpen `«0x42».shapes_call_arguments.vector_ok
    "the specification clause `ensures result <= 100` is not established"
    (← `(command| verify 0x42::shapes_call_arguments::vector_ok))

end VerexSL.Tests.Shapes.CallArguments
