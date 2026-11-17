-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport
import VerexSL.Tests.Shapes.Support

/-!
# The vector operations, at each element shape

Push, pop, swap, swap-remove, an element read and an element write, over a
vector of scalars, of invariant-bearing structs, of an enum with a
per-variant invariant, of a generic struct at an instance, and at the
caller's own type parameter. A positive states the fact the operation's
result owes, which for an invariant-bearing element is the element's own
invariant; the negative asks for one the operation does not give, or writes
a value the element invariant refuses.
-/

namespace VerexSL.Tests.Shapes.Vectors

set_option Elab.async false

verex module 0x42::shapes_vectors where
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

  -- 7.1 push
  fun push_scalar_bad(v : Vector<u64>, x : u64) -> Vector<u64> := core.prim.pushVector(v, x)
  spec push_scalar_bad where
    aborts_if false
    ensures result.length == v.length

  fun push_scalar_ok(v : Vector<u64>, x : u64) -> Vector<u64> := core.prim.pushVector(v, x)
  spec push_scalar_ok where
    aborts_if false
    ensures result.length == v.length + 1
    ensures result[v.length] == x

  fun push_struct_bad(v : Vector<Percent>) -> Vector<Percent> :=
    core.prim.pushVector(v, new Percent { value := 101 })
  spec push_struct_bad where
    aborts_if false

  fun push_struct_ok(v : Vector<Percent>, p : Percent) -> Vector<Percent> :=
    core.prim.pushVector(v, p)
  spec push_struct_ok where
    aborts_if false
    ensures result.length == v.length + 1
    ensures result[v.length] == p

  fun push_enum_bad(v : Vector<Payment>) -> Vector<Payment> :=
    core.prim.pushVector(v, new Payment::Direct { amount := 0 })
  spec push_enum_bad where
    aborts_if false

  fun push_enum_ok(v : Vector<Payment>, amount : u64) -> Vector<Payment> :=
    core.prim.pushVector(v, new Payment::Direct { amount })
  spec push_enum_ok where
    requires amount > 0
    aborts_if false
    ensures result.length == v.length + 1

  -- 7.2 pop: the popped element carries its own invariant
  fun pop_struct_bad(v : Vector<Percent>) -> u64 := do
    let (removed, rest) := core.prim.removeVector(v, v.length - 1)
    removed.value
  spec pop_struct_bad where
    requires 0 < v.length
    aborts_if false
    ensures result <= 99

  fun pop_struct_ok(v : Vector<Percent>) -> u64 := do
    let (removed, rest) := core.prim.removeVector(v, v.length - 1)
    removed.value
  spec pop_struct_ok where
    requires 0 < v.length
    aborts_if false
    ensures result <= 100

  -- 7.3 swap
  fun swap_struct_bad(v : Vector<Percent>, i : u64, j : u64) -> Vector<Percent> :=
    core.prim.swapVector(v, i, j)
  spec swap_struct_bad where
    requires i < v.length && j < v.length
    aborts_if false
    ensures result.length == v.length + 1

  fun swap_struct_ok(v : Vector<Percent>, i : u64, j : u64) -> Vector<Percent> :=
    core.prim.swapVector(v, i, j)
  spec swap_struct_ok where
    requires i < v.length && j < v.length
    aborts_if false
    ensures result.length == v.length
    ensures result[i] == v[j] && result[j] == v[i]

  -- 7.4 swap-remove, at the caller's own type parameter and at an instance
  fun swap_remove_param_bad {T has Copy, Drop}(v : Vector<T>, i : u64) -> (T, Vector<T>) := do
    let last := v.length - 1
    let swapped := core.prim.swapVector(v, i, last)
    core.prim.removeVector(swapped, last)
  spec swap_remove_param_bad where
    requires i < v.length
    aborts_if false
    ensures spec.result[1].length == v.length

  fun swap_remove_param_ok {T has Copy, Drop}(v : Vector<T>, i : u64) -> (T, Vector<T>) := do
    let last := v.length - 1
    let swapped := core.prim.swapVector(v, i, last)
    core.prim.removeVector(swapped, last)
  spec swap_remove_param_ok where
    requires i < v.length
    aborts_if false
    ensures spec.result[0] == v[i]
    ensures spec.result[1].length == v.length - 1

  fun swap_remove_struct_bad(v : Vector<Percent>, i : u64) -> Percent := do
    let last := v.length - 1
    let swapped := core.prim.swapVector(v, i, last)
    let (removed, rest) := core.prim.removeVector(swapped, last)
    removed
  spec swap_remove_struct_bad where
    requires i < v.length
    aborts_if false
    ensures result.value <= 99

  fun swap_remove_struct_ok(v : Vector<Percent>, i : u64) -> Percent := do
    let last := v.length - 1
    let swapped := core.prim.swapVector(v, i, last)
    let (removed, rest) := core.prim.removeVector(swapped, last)
    removed
  spec swap_remove_struct_ok where
    requires i < v.length
    aborts_if false
    ensures result.value <= 100

  -- 7.5 an element read: the element's invariant comes out of the container
  fun read_struct_bad(v : Vector<Percent>, i : u64) -> u64 := v[i].value
  spec read_struct_bad where
    requires i < v.length
    aborts_if false
    ensures result <= 99

  fun read_struct_ok(v : Vector<Percent>, i : u64) -> u64 := v[i].value
  spec read_struct_ok where
    requires i < v.length
    aborts_if false
    ensures result <= 100

  fun read_enum_bad(v : Vector<Payment>, i : u64) -> Payment := v[i]
  spec read_enum_bad where
    requires i < v.length
    aborts_if false
    ensures result is Direct ==> result.amount > 1

  fun read_enum_ok(v : Vector<Payment>, i : u64) -> Payment := v[i]
  spec read_enum_ok where
    requires i < v.length
    aborts_if false
    ensures result is Direct ==> result.amount > 0

  fun read_instance_bad(v : Vector<Box<Percent> >, i : u64) -> u64 := v[i].item.value
  spec read_instance_bad where
    requires i < v.length
    aborts_if false
    ensures result <= 99

  fun read_instance_ok(v : Vector<Box<Percent> >, i : u64) -> u64 := v[i].item.value
  spec read_instance_ok where
    requires i < v.length
    aborts_if false
    ensures result <= 100

  fun read_param_bad {T has Copy, Drop}(v : Vector<T>, i : u64) -> T := v[i]
  spec read_param_bad where
    requires i < v.length
    aborts_if false
    ensures result != v[i]

  fun read_param_ok {T has Copy, Drop}(v : Vector<T>, i : u64) -> T := v[i]
  spec read_param_ok where
    requires i < v.length
    aborts_if false
    ensures result == v[i]

  -- 7.6 an element write through a `&mut` of the vector
  fun write_struct_bad(v : &mut Vector<Percent>, i : u64) -> Unit := (*v)[i].value := 101
  spec write_struct_bad where
    requires i < v.length

  fun write_struct_ok(v : &mut Vector<Percent>, i : u64, amount : u64) -> Unit :=
    (*v)[i].value := amount
  spec write_struct_ok where
    requires i < v.length && amount <= 100
    aborts_if false
    ensures (*v)[i].value == amount

  fun write_param_bad {T has Copy, Drop}(v : &mut Vector<T>, i : u64, item : T) -> Unit :=
    (*v)[i] := item
  spec write_param_bad where
    requires i < v.length
    aborts_if false
    ensures (*v)[i] != item

  fun write_param_ok {T has Copy, Drop}(v : &mut Vector<T>, i : u64, item : T) -> Unit :=
    (*v)[i] := item
  spec write_param_ok where
    requires i < v.length
    aborts_if false
    ensures (*v)[i] == item

  -- 7.7 a removal through a mutable borrow of a generic holder's field
  struct Items {T has Copy, Drop, Store} has Copy, Drop, Store where
    items : Vector<T>

  fun remove_field_param_bad {T has Copy, Drop, Store}(w : &mut Items<T>, i : u64) -> T := do
    let items := &mut w.items
    let (removed, rest) := core.prim.removeVector(*items, i)
    *items := rest
    removed
  spec remove_field_param_bad where
    requires i < w.items.length
    aborts_if false
    ensures w.items.length == old(w).items.length

  fun remove_field_param_ok {T has Copy, Drop, Store}(w : &mut Items<T>, i : u64) -> T := do
    let items := &mut w.items
    let (removed, rest) := core.prim.removeVector(*items, i)
    *items := rest
    removed
  spec remove_field_param_ok where
    requires i < w.items.length
    aborts_if false
    ensures w.items.length + 1 == old(w).items.length

  -- 7.8 a generic enum built from the element popped out of a vector the
  -- body has tested, the emptied vector destroyed: the body of an optional
  -- value's construction from a vector, as a frontend lowers it
  enum Optional {Element} has Copy, Drop, Store where
    | Absent
    | Present (e : Element)

  fun from_vector_bad {Element}(mut vec : Vector<Element>) -> Optional<Element> := do
    assert!(vec.length <= 1, 1)
    if vec.length == 0 then
      core.prim.destroyEmptyVector(vec)
      new Optional<Element>::Absent {}
    else
      let e :=
        do
          let held := &mut vec
          if held.length == 0 then moveVectorError(2)
          let (last, rest) := core.prim.removeVector(*held, held.length - 1)
          *held := rest
          last
      core.prim.destroyEmptyVector(vec)
      new Optional<Element>::Present { e }
  spec from_vector_bad where
    aborts_if false

  fun from_vector_ok {Element}(mut vec : Vector<Element>) -> Optional<Element> := do
    assert!(vec.length <= 1, 1)
    if vec.length == 0 then
      core.prim.destroyEmptyVector(vec)
      new Optional<Element>::Absent {}
    else
      let e :=
        do
          let held := &mut vec
          if held.length == 0 then moveVectorError(2)
          let (last, rest) := core.prim.removeVector(*held, held.length - 1)
          *held := rest
          last
      core.prim.destroyEmptyVector(vec)
      new Optional<Element>::Present { e }
  spec from_vector_ok where
    aborts_if vec.length > 1

/-! ## Negative controls -/

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_vectors.push_scalar_bad
    ["ensures result.length == v.length"]
    (← `(command| verify 0x42::shapes_vectors::push_scalar_bad))

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  expectCuts `«0x42».shapes_vectors.push_struct_bad [(percent, construction)]
    (← `(command| verify 0x42::shapes_vectors::push_struct_bad)) (some [])

open LoanDeathControls in
run_cmd do
  let payment := "invariant match this with\n" ++
    "      | Payment::None {} => true\n" ++
    "      | Payment::Direct { amount := amount } => amount > 0"
  expectCuts `«0x42».shapes_vectors.push_enum_bad [(payment, construction)]
    (← `(command| verify 0x42::shapes_vectors::push_enum_bad)) (some [])

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_vectors.pop_struct_bad ["ensures result <= 99"]
    (← `(command| verify 0x42::shapes_vectors::pop_struct_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_vectors.swap_struct_bad
    ["ensures result.length == v.length + 1"]
    (← `(command| verify 0x42::shapes_vectors::swap_struct_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_vectors.read_struct_bad ["ensures result <= 99"]
    (← `(command| verify 0x42::shapes_vectors::read_struct_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_vectors.read_enum_bad
    ["ensures result is Direct ==> result.amount > 1"]
    (← `(command| verify 0x42::shapes_vectors::read_enum_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_vectors.read_instance_bad ["ensures result <= 99"]
    (← `(command| verify 0x42::shapes_vectors::read_instance_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_vectors.read_param_bad ["ensures result != v[i]"]
    (← `(command| verify 0x42::shapes_vectors::read_param_bad))

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  expectCuts `«0x42».shapes_vectors.write_struct_bad [(percent, exit)]
    (← `(command| verify 0x42::shapes_vectors::write_struct_bad)) (some [])

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_vectors.write_param_bad ["ensures (*v)[i] != item"]
    (← `(command| verify 0x42::shapes_vectors::write_param_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_vectors.swap_remove_param_bad
    ["ensures spec.result[1].length == v.length"]
    (← `(command| verify 0x42::shapes_vectors::swap_remove_param_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_vectors.swap_remove_struct_bad ["ensures result.value <= 99"]
    (← `(command| verify 0x42::shapes_vectors::swap_remove_struct_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_vectors.remove_field_param_bad
    ["ensures w.items.length == old(w).items.length"]
    (← `(command| verify 0x42::shapes_vectors::remove_field_param_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_vectors.from_vector_bad ["aborts_if false"]
    (← `(command| verify 0x42::shapes_vectors::from_vector_bad))

/-! ## Positive controls -/

verify 0x42::shapes_vectors::push_scalar_ok
verify 0x42::shapes_vectors::swap_struct_ok
verify 0x42::shapes_vectors::read_struct_ok
verify 0x42::shapes_vectors::read_enum_ok
verify 0x42::shapes_vectors::read_instance_ok
verify 0x42::shapes_vectors::read_param_ok
verify 0x42::shapes_vectors::write_struct_ok
verify 0x42::shapes_vectors::write_param_ok
verify 0x42::shapes_vectors::push_struct_ok
verify 0x42::shapes_vectors::push_enum_ok
verify 0x42::shapes_vectors::pop_struct_ok
verify 0x42::shapes_vectors::swap_remove_struct_ok
verify 0x42::shapes_vectors::swap_remove_param_ok
verify 0x42::shapes_vectors::remove_field_param_ok
verify 0x42::shapes_vectors::from_vector_ok

end VerexSL.Tests.Shapes.Vectors
