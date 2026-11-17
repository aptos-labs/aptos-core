-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport
import VerexSL.Tests.Shapes.Support

/-!
# Constructing and returning a value, at each type shape

One function per shape constructs a value of that shape and returns it. The
negative of each cell constructs a value its type's invariant refuses, or
claims of the constructed value what the construction does not give; the
positive states in `ensures` the fact the construction owes.

Shapes: a scalar, a struct with a data invariant and one without, an enum
with a per-variant invariant clause, a struct holding an invariant-bearing
struct, a vector of invariant-bearing structs, a type parameter, a generic
struct at an instance, a tuple of two invariant-bearing values, and a
function value whose result has an invariant and one whose result has none.
A `&mut` shape has no cell here: a reference is borrowed, never constructed.
-/

namespace VerexSL.Tests.Shapes.Construct

set_option Elab.async false

verex module 0x42::shapes_construct where
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

  fun fifty(x : u64) -> Percent := new Percent { value := 50 }
  spec fifty where
    aborts_if false
    ensures result.value == 50

  fun double(x : u64) -> u64 := x * 2
  spec double where
    aborts_if x * 2 > MAX_U64
    ensures result == x * 2

  -- a. scalar
  fun scalar_bad() -> u64 := 42
  spec scalar_bad where
    aborts_if false
    ensures result == 43

  fun scalar_ok() -> u64 := 42
  spec scalar_ok where
    aborts_if false
    ensures result == 42

  -- b. a struct with a data invariant, and one without
  fun percent_bad() -> Percent := new Percent { value := 101 }
  spec percent_bad where
    aborts_if false

  fun percent_ok(amount : u64) -> Percent := new Percent { value := amount }
  spec percent_ok where
    requires amount <= 100
    aborts_if false
    ensures result.value == amount

  fun plain_bad() -> Plain := new Plain { value := 101 }
  spec plain_bad where
    aborts_if false
    ensures result.value <= 100

  fun plain_ok() -> Plain := new Plain { value := 101 }
  spec plain_ok where
    aborts_if false
    ensures result.value == 101

  -- c. an enum with a per-variant invariant clause
  fun payment_bad() -> Payment := new Payment::Direct { amount := 0 }
  spec payment_bad where
    aborts_if false

  fun payment_ok(amount : u64) -> Payment := new Payment::Direct { amount }
  spec payment_ok where
    requires amount > 0
    aborts_if false
    ensures result is Direct

  -- d. a struct holding an invariant-bearing struct, and a vector of them
  fun pair_bad(p : Percent) -> Pair :=
    new Pair { first := new Percent { value := 101 }, second := new Plain { value := 0 } }
  spec pair_bad where
    aborts_if false

  fun pair_ok(p : Percent) -> Pair :=
    new Pair { first := p, second := new Plain { value := 0 } }
  spec pair_ok where
    aborts_if false
    ensures result.first.value <= 100

  fun bag_bad(p : Percent) -> Bag :=
    new Bag { items := vector<Percent>[p, new Percent { value := 101 }] }
  spec bag_bad where
    aborts_if false

  fun bag_ok(p : Percent, q : Percent) -> Bag := new Bag { items := vector<Percent>[p, q] }
  spec bag_ok where
    aborts_if false
    ensures result.items.length == 2

  -- e. a type parameter
  fun box_param_bad {T has Copy, Drop}(item : T) -> Box<T> := new Box<T> { item }
  spec box_param_bad where
    aborts_if false
    ensures result.item != item

  fun box_param_ok {T has Copy, Drop}(item : T) -> Box<T> := new Box<T> { item }
  spec box_param_ok where
    aborts_if false
    ensures result.item == item

  -- f. a generic struct at an instance
  fun box_percent_bad() -> Box<Percent> := new Box<Percent> { item := new Percent { value := 101 } }
  spec box_percent_bad where
    aborts_if false

  fun box_percent_ok(amount : u64) -> Box<Percent> :=
    new Box<Percent> { item := new Percent { value := amount } }
  spec box_percent_ok where
    requires amount <= 100
    aborts_if false
    ensures result.item.value == amount

  -- g. a tuple of two invariant-bearing values
  fun tuple_bad(p : Percent) -> (Percent, Percent) := (p, new Percent { value := 101 })
  spec tuple_bad where
    aborts_if false

  fun tuple_ok(p : Percent, q : Percent) -> (Percent, Percent) := (q, p)
  spec tuple_ok where
    aborts_if false
    ensures spec.result[0] == q && spec.result[1] == p

  -- i. a function value whose result has an invariant, and one whose has none
  fun fn_percent_bad() -> Fn(u64) -> Percent has Copy, Drop :=
    function[Fn(u64) -> Percent has Copy, Drop](fifty)
  spec fn_percent_bad where
    aborts_if false
    ensures false

  fun fn_percent_ok() -> Fn(u64) -> Percent has Copy, Drop :=
    function[Fn(u64) -> Percent has Copy, Drop](fifty)
  spec fn_percent_ok where
    aborts_if false

  fun fn_plain_bad() -> Fn(u64) -> u64 has Copy, Drop :=
    function[Fn(u64) -> u64 has Copy, Drop](double)
  spec fn_plain_bad where
    aborts_if false
    ensures false

  fun fn_plain_ok() -> Fn(u64) -> u64 has Copy, Drop :=
    function[Fn(u64) -> u64 has Copy, Drop](double)
  spec fn_plain_ok where
    aborts_if false

  -- d'. a generic struct whose invariant bounds its vector field, built from
  -- a vector the body has tested
  struct Maybe {T has Copy, Drop} has Copy, Drop where
    vec : Vector<T>
  spec Maybe where
    invariant vec.length <= 1

  fun bounded_vector_bad {T has Copy, Drop}(v : Vector<T>) -> Maybe<T> := do
    assert!(v.length <= 2, 1)
    new Maybe<T> { vec := v }
  spec bounded_vector_bad where
    aborts_if v.length > 2

  fun bounded_vector_ok {T has Copy, Drop}(v : Vector<T>) -> Maybe<T> := do
    assert!(v.length <= 1, 1)
    new Maybe<T> { vec := v }
  spec bounded_vector_ok where
    aborts_if v.length > 1
    ensures result.vec == v

/-! ## Negative controls -/

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_construct.scalar_bad ["ensures result == 43"]
    (← `(command| verify 0x42::shapes_construct::scalar_bad))

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  expectCuts `«0x42».shapes_construct.percent_bad [(percent, construction)]
    (← `(command| verify 0x42::shapes_construct::percent_bad)) (some [])

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_construct.plain_bad ["ensures result.value <= 100"]
    (← `(command| verify 0x42::shapes_construct::plain_bad))

open LoanDeathControls in
run_cmd do
  let payment := "invariant match this with\n" ++
    "      | Payment::None {} => true\n" ++
    "      | Payment::Direct { amount := amount } => amount > 0"
  expectCuts `«0x42».shapes_construct.payment_bad [(payment, construction)]
    (← `(command| verify 0x42::shapes_construct::payment_bad)) (some [])

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  expectCuts `«0x42».shapes_construct.pair_bad [(percent, construction)]
    (← `(command| verify 0x42::shapes_construct::pair_bad)) (some [])

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  expectCuts `«0x42».shapes_construct.bag_bad [(percent, construction)]
    (← `(command| verify 0x42::shapes_construct::bag_bad)) (some [])

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_construct.box_param_bad ["ensures result.item != item"]
    (← `(command| verify 0x42::shapes_construct::box_param_bad))

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  expectCuts `«0x42».shapes_construct.box_percent_bad [(percent, construction)]
    (← `(command| verify 0x42::shapes_construct::box_percent_bad)) (some [])

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  expectCuts `«0x42».shapes_construct.tuple_bad [(percent, construction)]
    (← `(command| verify 0x42::shapes_construct::tuple_bad)) (some [])

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_construct.fn_percent_bad ["ensures false"]
    (← `(command| verify 0x42::shapes_construct::fn_percent_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_construct.fn_plain_bad ["ensures false"]
    (← `(command| verify 0x42::shapes_construct::fn_plain_bad))

open LoanDeathControls in
run_cmd do
  expectCuts `«0x42».shapes_construct.bounded_vector_bad
    [("invariant vec.length <= 1", construction)]
    (← `(command| verify 0x42::shapes_construct::bounded_vector_bad)) (some [])

/-! ## Positive controls -/

verify 0x42::shapes_construct::scalar_ok
verify 0x42::shapes_construct::percent_ok
verify 0x42::shapes_construct::plain_ok
verify 0x42::shapes_construct::payment_ok
verify 0x42::shapes_construct::pair_ok
verify 0x42::shapes_construct::box_param_ok
verify 0x42::shapes_construct::box_percent_ok
verify 0x42::shapes_construct::tuple_ok
verify 0x42::shapes_construct::fn_percent_ok
verify 0x42::shapes_construct::fn_plain_ok
verify 0x42::shapes_construct::bag_ok
verify 0x42::shapes_construct::bounded_vector_ok

end VerexSL.Tests.Shapes.Construct
