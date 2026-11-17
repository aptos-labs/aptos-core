-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport
import VerexSL.Tests.Shapes.Support

/-!
# Function values, at each result shape

Invoking a function-typed parameter whose target a proof cannot see, passing
a named function and a literal closure with a capture, passing a
function-typed parameter through to another higher-order function, and
invoking a closure held in a struct field and in global storage. The result
shapes are a value whose type states an invariant and one whose type states
none, at a concrete type and at the caller's own type parameter.

The positives are verified where the module is elaborated, as the
closure fixtures under `verex-e2e-tests/VerexE2ETests/Check/Closures` are:
a contract that reads `ensures_of` of a function-typed parameter is not
established through a later `verify` command of a module that carries
`pragma verify = false` (`README.md`). The negatives, which need that
command, are in a second module below.
-/

namespace VerexSL.Tests.Shapes.Closures

set_option Elab.async false

verex module 0x42::shapes_closures where
  struct Percent has Copy, Drop, Store where
    value : u64
  spec Percent where
    invariant value <= 100

  fun fifty(x : u64) -> Percent := new Percent { value := 50 }
  spec fifty where
    aborts_if false
    ensures result.value == 50

  fun add(x : u64, y : u64) -> u64 := x + y
  spec add where
    aborts_if x + y > MAX_U64
    ensures result == x + y

  fun apply {R}(f : Fn(u64) -> R, x : u64) -> R := invoke(f, x)
  spec apply where
    ensures ensures_of<f>(x, result)

  fun apply_both {A} {B} {R}(f : Fn(A, B) -> R, x : A, y : B) -> R := invoke(f, x, y)
  spec apply_both where
    ensures ensures_of<f>(x, y, result)

  -- 8.1 invoking an unseen target whose result type states no invariant
  fun unseen_plain_ok(f : Fn(u64) -> u64, x : u64) -> u64 := invoke(f, x)
  spec unseen_plain_ok where
    ensures ensures_of<f>(x, result)

  -- 8.2 passing a named function to a higher-order function, the result
  -- type stating an invariant
  fun named_ok(x : u64) -> Percent := apply::<Percent>(function[Fn(u64) -> Percent](fifty), x)
  spec named_ok where
    ensures result.value == 50

  -- 8.3 passing a literal closure with a capture
  fun captured_ok(x : u64, y : u64) -> u64 := do
    let f := function[Fn(u64) -> u64 has Copy, Drop](add, x)
    invoke(f, y)
  spec captured_ok where
    ensures result == x + y
    aborts_if x + y > MAX_U64

  -- 8.4 passing a function-typed parameter through, at the caller's own
  -- type parameter
  fun through_ok {R}(f : Fn(u64) -> R, x : u64) -> R := apply::<R>(f, x)
  spec through_ok where
    ensures ensures_of<f>(x, result)

/-! ## Negative controls, and the cells this tree leaves open -/

verex module 0x42::shapes_closures_controls where
  pragma verify = false

  struct Percent has Copy, Drop, Store where
    value : u64
  spec Percent where
    invariant value <= 100

  struct Op has Copy, Drop, Store where
    f : Fn(u64) -> Percent has Copy, Drop, Store

  struct Holder has Key where
    op : Op

  fun fifty(x : u64) -> Percent := new Percent { value := 50 }
  spec fifty where
    aborts_if false
    ensures result.value == 50

  fun add(x : u64, y : u64) -> u64 := x + y
  spec add where
    aborts_if x + y > MAX_U64
    ensures result == x + y

  fun apply {R}(f : Fn(u64) -> R, x : u64) -> R := invoke(f, x)
  spec apply where
    ensures ensures_of<f>(x, result)

  -- The negatives of the cells above.
  fun named_bad(x : u64) -> Percent := apply::<Percent>(function[Fn(u64) -> Percent](fifty), x)
  spec named_bad where
    ensures result.value == 40

  fun captured_bad(x : u64, y : u64) -> u64 := do
    let f := function[Fn(u64) -> u64 has Copy, Drop](add, x)
    invoke(f, y)
  spec captured_bad where
    ensures result == x + y + 1
    aborts_if x + y > MAX_U64

  -- 8.5 invoking an unseen target whose result type states an invariant
  fun unseen_ok(f : Fn(u64) -> Percent, x : u64) -> Percent := invoke(f, x)
  spec unseen_ok where
    ensures result.value <= 100

  -- 8.6 a closure held in a struct field: the result's invariant, and the
  -- field's own behaviour
  fun field_ok(o : Op, x : u64) -> Percent := invoke(o.f, x)
  spec field_ok where
    ensures result.value <= 100

  fun field_behavior_ok(o : Op, x : u64) -> Percent := invoke(o.f, x)
  spec field_behavior_ok where
    ensures ensures_of<o.f>(x, result)

  -- 8.7 a closure held in global storage
  fun stored_ok(addr : Address, x : u64) -> Percent := invoke(Holder[addr].op.f, x)
  spec stored_ok where
    requires exists<Holder>(addr)
    ensures result.value <= 100

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_closures_controls.named_bad
    ["ensures result.value == 40"]
    (← `(command| verify 0x42::shapes_closures_controls::named_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_closures_controls.captured_bad
    ["ensures result == x + y + 1"]
    (← `(command| verify 0x42::shapes_closures_controls::captured_bad))

-- Open cells
--
-- `unseen_ok`: the data invariant of the result type of a function value a
-- proof cannot see is not available to the caller of its invocation.
-- `field_ok`, `field_behavior_ok`: neither the result's invariant nor the
-- behaviour of a closure held in a struct field reaches the caller.
-- `stored_ok`: the same of a closure held by a resource.
-- None of the four has a negative while it is open: a false clause beside
-- the residual obligation would not be the only reason the cell fails.

run_cmd do
  expectOpen `«0x42».shapes_closures_controls.unseen_ok
    "verification failed with a residual obligation"
    (← `(command| verify 0x42::shapes_closures_controls::unseen_ok))

run_cmd do
  expectOpen `«0x42».shapes_closures_controls.field_ok
    "verification failed with a residual obligation"
    (← `(command| verify 0x42::shapes_closures_controls::field_ok))

run_cmd do
  expectOpen `«0x42».shapes_closures_controls.field_behavior_ok
    "verification failed with a residual obligation"
    (← `(command| verify 0x42::shapes_closures_controls::field_behavior_ok))

run_cmd do
  expectOpen `«0x42».shapes_closures_controls.stored_ok
    "verification failed with a residual obligation"
    (← `(command| verify 0x42::shapes_closures_controls::stored_ok))

end VerexSL.Tests.Shapes.Closures
