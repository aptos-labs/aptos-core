-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport
import VerexSL.Tests.Shapes.Support

/-!
# Calling a generic callee, at the caller's own parameter and at an instance

Each cell calls a generic `pragma opaque` callee: once from a generic caller
at its own type parameter, and once at an instance, `Vector<T>` and a struct
over `T`. Two cells carry a recursive specification function in the callee's
contract, one at a type parameter and one at an instance. The negative of
each cell asks of the result what the callee's contract and the type
argument's invariant do not give.
-/

namespace VerexSL.Tests.Shapes.Generics

set_option Elab.async false

verex module 0x42::shapes_generics where
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

  struct Box {T has Copy, Drop} has Copy, Drop where
    item : T

  spec fun length_upto {T}(v : Vector<T>, end : Int) : Int :=
    if end == 0 then 0 else 1 + length_upto::<T>(v, end - 1)

  spec fun total(v : Vector<u64>, end : Int) : Int :=
    if end == 0 then 0 else total(v, end - 1) + v[end - 1]

  -- The generic opaque callees.
  fun first {T has Copy, Drop}(v : Vector<T>) -> T := v[0]
  spec first where
    pragma opaque = true
    pragma verify = false
    requires 0 < v.length
    aborts_if false
    ensures result == v[0]

  fun two {T has Copy, Drop}(v : Vector<T>) -> (T, T) := (v[0], v[1])
  spec two where
    pragma opaque = true
    pragma verify = false
    requires 1 < v.length
    aborts_if false
    ensures spec.result[0] == v[0] && spec.result[1] == v[1]

  fun unbox {T has Copy, Drop}(b : Box<T>) -> T := b.item
  spec unbox where
    pragma opaque = true
    pragma verify = false
    aborts_if false
    ensures result == b.item

  fun replace {T has Copy, Drop}(slot : &mut T, item : T) -> Unit := *slot := item
  spec replace where
    pragma opaque = true
    pragma verify = false
    aborts_if false
    ensures *slot == item

  fun counted {T has Copy, Drop}(v : Vector<T>, n : u64) -> u64 := n
  spec counted where
    pragma opaque = true
    pragma verify = false
    requires n <= v.length
    aborts_if false
    ensures result == length_upto::<T>(v, n)

  fun summed(v : Vector<u64>, n : u64) -> u64 := 0
  spec summed where
    pragma opaque = true
    pragma verify = false
    requires n <= v.length
    aborts_if false
    ensures result == total(v, n)

  fun apply {T has Copy, Drop}(f : Fn(u64) -> T has Copy, Drop, x : u64) -> T := invoke(f, x)
  spec apply where
    pragma opaque = true
    pragma verify = false
    aborts_if false
    ensures ensures_of<f>(x, result)

  fun fifty(x : u64) -> Percent := new Percent { value := 50 }
  spec fifty where
    aborts_if false
    ensures result.value == 50

  -- a. at the instance `Vector<u64>`
  fun scalar_bad(v : Vector<u64>) -> u64 := first::<u64>(v)
  spec scalar_bad where
    requires 0 < v.length
    aborts_if false
    ensures result != v[0]

  fun scalar_ok(v : Vector<u64>) -> u64 := first::<u64>(v)
  spec scalar_ok where
    requires 0 < v.length
    aborts_if false
    ensures result == v[0]

  -- b. at the instance `Vector<Percent>`: the element's invariant
  fun struct_bad(v : Vector<Percent>) -> Percent := first::<Percent>(v)
  spec struct_bad where
    requires 0 < v.length
    aborts_if false
    ensures result.value <= 99

  fun struct_ok(v : Vector<Percent>) -> Percent := first::<Percent>(v)
  spec struct_ok where
    requires 0 < v.length
    aborts_if false
    ensures result.value <= 100

  -- c. at the instance `Vector<Payment>`
  fun enum_bad(v : Vector<Payment>) -> Payment := first::<Payment>(v)
  spec enum_bad where
    requires 0 < v.length
    aborts_if false
    ensures result is Direct ==> result.amount > 1

  fun enum_ok(v : Vector<Payment>) -> Payment := first::<Payment>(v)
  spec enum_ok where
    requires 0 < v.length
    aborts_if false
    ensures result is Direct ==> result.amount > 0

  -- d. at a nested instance `Vector<Box<Percent> >`
  fun nested_bad(v : Vector<Box<Percent> >) -> Box<Percent> := first::<Box<Percent> >(v)
  spec nested_bad where
    requires 0 < v.length
    aborts_if false
    ensures result.item.value <= 99

  fun nested_ok(v : Vector<Box<Percent> >) -> Box<Percent> := first::<Box<Percent> >(v)
  spec nested_ok where
    requires 0 < v.length
    aborts_if false
    ensures result.item.value <= 100

  -- e. at the caller's own type parameter
  fun param_bad {T has Copy, Drop}(v : Vector<T>) -> T := first::<T>(v)
  spec param_bad where
    requires 0 < v.length
    aborts_if false
    ensures result != v[0]

  fun param_ok {T has Copy, Drop}(v : Vector<T>) -> T := first::<T>(v)
  spec param_ok where
    requires 0 < v.length
    aborts_if false
    ensures result == v[0]

  -- f. a struct over `T`, at the caller's parameter and at an instance
  fun box_param_bad {T has Copy, Drop}(b : Box<T>) -> T := unbox::<T>(b)
  spec box_param_bad where
    aborts_if false
    ensures result != b.item

  fun box_param_ok {T has Copy, Drop}(b : Box<T>) -> T := unbox::<T>(b)
  spec box_param_ok where
    aborts_if false
    ensures result == b.item

  fun box_instance_bad(b : Box<Percent>) -> Percent := unbox::<Percent>(b)
  spec box_instance_bad where
    aborts_if false
    ensures result.value <= 99

  fun box_instance_ok(b : Box<Percent>) -> Percent := unbox::<Percent>(b)
  spec box_instance_ok where
    aborts_if false
    ensures result.value <= 100

  -- g. a tuple of two invariant-bearing values at an instance
  fun tuple_bad(v : Vector<Percent>) -> (Percent, Percent) := two::<Percent>(v)
  spec tuple_bad where
    requires 1 < v.length
    aborts_if false
    ensures spec.result[0].value <= 99

  fun tuple_ok(v : Vector<Percent>) -> (Percent, Percent) := two::<Percent>(v)
  spec tuple_ok where
    requires 1 < v.length
    aborts_if false
    ensures spec.result[0].value <= 100 && spec.result[1].value <= 100

  -- h. a `&mut` of an invariant-bearing struct at a generic callee
  fun mut_bad(p : &mut Percent, q : Percent) -> Unit := replace::<Percent>(p, q)
  spec mut_bad where
    aborts_if false
    ensures p.value <= 99

  fun mut_ok(p : &mut Percent, q : Percent) -> Unit := replace::<Percent>(p, q)
  spec mut_ok where
    aborts_if false
    ensures p.value == q.value

  -- i. a function value whose result has an invariant, at a generic callee.
  -- The cell is open (`Open cells` below), so `fn_bad` has no control: its
  -- false clause would not be the only reason it fails.
  fun fn_bad(x : u64) -> Percent :=
    apply::<Percent>(function[Fn(u64) -> Percent has Copy, Drop](fifty), x)
  spec fn_bad where
    aborts_if false
    ensures result.value == 40

  fun fn_ok(x : u64) -> Percent :=
    apply::<Percent>(function[Fn(u64) -> Percent has Copy, Drop](fifty), x)
  spec fn_ok where
    aborts_if false
    ensures result.value == 50

  -- A recursive specification function in the callee's contract, at the
  -- caller's own type parameter and at an instance.
  fun recursive_param_bad {T has Copy, Drop}(v : Vector<T>) -> u64 := counted::<T>(v, 0)
  spec recursive_param_bad where
    aborts_if false
    ensures result == 1

  fun recursive_param_ok {T has Copy, Drop}(v : Vector<T>) -> u64 := counted::<T>(v, 0)
  spec recursive_param_ok where
    aborts_if false
    ensures result == length_upto::<T>(v, 0)

  fun recursive_instance_bad(v : Vector<u64>) -> u64 := summed(v, 0)
  spec recursive_instance_bad where
    aborts_if false
    ensures result == 1

  fun recursive_instance_ok(v : Vector<u64>) -> u64 := summed(v, 0)
  spec recursive_instance_ok where
    aborts_if false
    ensures result == total(v, 0)

/-! ## The function a cell passes as a value

A call that passes a named function to a callee whose function-typed
parameter returns a type with a data invariant pays the validity of that
function's results from its theorem. -/

verify 0x42::shapes_generics::fifty

/-! ## Negative controls -/

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_generics.scalar_bad ["ensures result != v[0]"]
    (← `(command| verify 0x42::shapes_generics::scalar_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_generics.struct_bad ["ensures result.value <= 99"]
    (← `(command| verify 0x42::shapes_generics::struct_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_generics.enum_bad
    ["ensures result is Direct ==> result.amount > 1"]
    (← `(command| verify 0x42::shapes_generics::enum_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_generics.nested_bad ["ensures result.item.value <= 99"]
    (← `(command| verify 0x42::shapes_generics::nested_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_generics.param_bad ["ensures result != v[0]"]
    (← `(command| verify 0x42::shapes_generics::param_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_generics.box_param_bad ["ensures result != b.item"]
    (← `(command| verify 0x42::shapes_generics::box_param_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_generics.box_instance_bad ["ensures result.value <= 99"]
    (← `(command| verify 0x42::shapes_generics::box_instance_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_generics.tuple_bad
    ["ensures spec.result[0].value <= 99"]
    (← `(command| verify 0x42::shapes_generics::tuple_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_generics.mut_bad ["ensures p.value <= 99"]
    (← `(command| verify 0x42::shapes_generics::mut_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_generics.recursive_param_bad ["ensures result == 1"]
    (← `(command| verify 0x42::shapes_generics::recursive_param_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_generics.recursive_instance_bad ["ensures result == 1"]
    (← `(command| verify 0x42::shapes_generics::recursive_instance_bad))

/-! ## Positive controls -/

verify 0x42::shapes_generics::scalar_ok
verify 0x42::shapes_generics::struct_ok
verify 0x42::shapes_generics::enum_ok
verify 0x42::shapes_generics::nested_ok
verify 0x42::shapes_generics::param_ok
verify 0x42::shapes_generics::box_param_ok
verify 0x42::shapes_generics::box_instance_ok
verify 0x42::shapes_generics::tuple_ok
verify 0x42::shapes_generics::mut_ok
verify 0x42::shapes_generics::recursive_param_ok
verify 0x42::shapes_generics::recursive_instance_ok

-- Open cells
--
-- `fn_ok`: a function value whose result type states an invariant, passed to
-- a generic opaque higher-order callee at a declared type. The call owes
-- three facts of `apply`'s contract and establishes none:
-- 1. `ScalarAt` of the row `T`: the contract of a generic function states it
--    for each row of a function-typed parameter that mentions a type
--    parameter, because the rule for invoking a function value a proof does
--    not see is stated for rows of scalar types. It is false at `Percent`, so
--    no caller pays it. This is a limit of the rule and of the contract
--    generated from the signature, not a property every higher-order
--    function needs.
-- 2. the frame of the function literal at the frame the callee's type
--    arguments induce (`Check/Closures/ResultValidity`,
--    `apply_opaque_double`): the step moves the fact to the base frame and
--    then does not recognize the literal in the moved statement.
-- 3. the validity of the literal's results at that frame: the step that reads
--    it from the target's theorem does not move the fact to the base frame
--    first and declines ("the closure and its target theorem use different
--    type families").
-- The caller's own `ensures` is established from the theorem of `fifty`.
-- The cell has no negative while it is open.

run_cmd do
  expectOpen `«0x42».shapes_generics.fn_ok
    "the precondition of `apply` at call 1 is not established"
    (← `(command| verify 0x42::shapes_generics::fn_ok))

end VerexSL.Tests.Shapes.Generics
