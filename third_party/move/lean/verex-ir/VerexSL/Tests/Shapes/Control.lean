-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport
import VerexSL.Tests.Shapes.Support

/-!
# A branch join and a loop carrying a value, at each type shape

A value of each shape reaches a join from both arms of a branch — from two
parameters, from a parameter and a construction, and as a mutable reference
written after the join — and a value of each shape is carried through a loop
by an invariant. A positive states the fact the shape's invariant gives of
the joined or carried value; the negative asks for a bound the invariant
does not give, or writes through the joined reference a value it refuses.
-/

namespace VerexSL.Tests.Shapes.Control

set_option Elab.async false

verex module 0x42::shapes_control where
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

  -- 9.1 a join of two parameters
  fun join_struct_bad(c : Bool, p : Percent, q : Percent) -> u64 := do
    let r := if c then p else q
    r.value
  spec join_struct_bad where
    aborts_if false
    ensures result <= 99

  fun join_struct_ok(c : Bool, p : Percent, q : Percent) -> u64 := do
    let r := if c then p else q
    r.value
  spec join_struct_ok where
    aborts_if false
    ensures result <= 100

  -- 9.2 a join of a parameter and a construction
  fun join_mixed_bad(c : Bool, p : Percent) -> u64 := do
    let r := if c then p else new Percent { value := 1 }
    r.value
  spec join_mixed_bad where
    aborts_if false
    ensures result <= 99

  fun join_mixed_ok(c : Bool, p : Percent) -> u64 := do
    let r := if c then p else new Percent { value := 1 }
    r.value
  spec join_mixed_ok where
    aborts_if false
    ensures result <= 100

  -- 9.3 a join at an enum with a per-variant invariant clause
  fun join_enum_bad(c : Bool, amount : u64) -> Payment :=
    if c then new Payment::Direct { amount } else new Payment::None {}
  spec join_enum_bad where
    requires amount > 0
    aborts_if false
    ensures result is Direct ==> result.amount > 1

  fun join_enum_ok(c : Bool, amount : u64) -> Payment :=
    if c then new Payment::Direct { amount } else new Payment::None {}
  spec join_enum_ok where
    requires amount > 0
    aborts_if false
    ensures result is Direct ==> result.amount > 0

  -- 9.4 a join at a generic struct instance
  fun join_instance_bad(c : Bool, a : Box<Percent>, b : Box<Percent>) -> u64 := do
    let r := if c then a else b
    r.item.value
  spec join_instance_bad where
    aborts_if false
    ensures result <= 99

  fun join_instance_ok(c : Bool, a : Box<Percent>, b : Box<Percent>) -> u64 := do
    let r := if c then a else b
    r.item.value
  spec join_instance_ok where
    aborts_if false
    ensures result <= 100

  -- 9.5 a join at the caller's own type parameter
  fun join_param_bad {T has Copy, Drop}(c : Bool, a : T, b : T) -> T := if c then a else b
  spec join_param_bad where
    aborts_if false
    ensures result != (if c then a else b)

  fun join_param_ok {T has Copy, Drop}(c : Bool, a : T, b : T) -> T := if c then a else b
  spec join_param_ok where
    aborts_if false
    ensures result == (if c then a else b)

  -- 9.6 a join of two mutable references, written after the join
  fun join_mut_bad(c : Bool, a : &mut Percent, b : &mut Percent) -> Unit := do
    let r := if c then a else b
    r.value := 101
  spec join_mut_bad where
    aborts_if false

  fun join_mut_ok(c : Bool, a : &mut Percent, b : &mut Percent, amount : u64) -> Unit := do
    let r := if c then a else b
    r.value := amount
  spec join_mut_ok where
    requires amount <= 100
    aborts_if false
    ensures (if c then a.value else b.value) == amount

  -- 9.7 a loop whose invariant carries a scalar read out of an
  -- invariant-bearing struct
  fun loop_scalar_bad(p : Percent) -> u64 := do
    let mut i : u64 := 0
    let mut acc : u64 := 0
    while i < 3 do
      acc := p.value
      i := i + 1
    where
      invariant i <= 3
      invariant acc <= 100
    acc
  spec loop_scalar_bad where
    aborts_if false
    ensures result <= 99

  fun loop_scalar_ok(p : Percent) -> u64 := do
    let mut i : u64 := 0
    let mut acc : u64 := 0
    while i < 3 do
      acc := p.value
      i := i + 1
    where
      invariant i <= 3
      invariant acc <= 100
    acc
  spec loop_scalar_ok where
    aborts_if false
    ensures result <= 100

  -- 9.8 a loop whose invariant carries an invariant-bearing struct
  fun loop_struct_bad(p : Percent, q : Percent) -> Percent := do
    let mut r := p
    let mut i : u64 := 0
    while i < 3 do
      r := q
      i := i + 1
    where
      invariant i <= 3
      invariant r.value <= 100
    r
  spec loop_struct_bad where
    aborts_if false
    ensures result.value <= 99

  fun loop_struct_ok(p : Percent, q : Percent) -> Percent := do
    let mut r := p
    let mut i : u64 := 0
    while i < 3 do
      r := q
      i := i + 1
    where
      invariant i <= 3
      invariant r.value <= 100
    r
  spec loop_struct_ok where
    aborts_if false
    ensures result.value <= 100

  -- 9.9 a loop reading an element of a vector of invariant-bearing structs
  fun loop_element_bad(v : Vector<Percent>) -> u64 := do
    let mut i : u64 := 0
    let mut acc : u64 := 0
    while i < 3 do
      if i < v.length then acc := v[i].value
      i := i + 1
    where
      invariant i <= 3
      invariant acc <= 100
    acc
  spec loop_element_bad where
    aborts_if false
    ensures result <= 99

  fun loop_element_ok(v : Vector<Percent>) -> u64 := do
    let mut i : u64 := 0
    let mut acc : u64 := 0
    while i < 3 do
      if i < v.length then acc := v[i].value
      i := i + 1
    where
      invariant i <= 3
      invariant acc <= 100
    acc
  spec loop_element_ok where
    aborts_if false
    ensures result <= 100

/-! ## Negative controls -/

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_control.join_struct_bad ["ensures result <= 99"]
    (← `(command| verify 0x42::shapes_control::join_struct_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_control.join_mixed_bad ["ensures result <= 99"]
    (← `(command| verify 0x42::shapes_control::join_mixed_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_control.join_enum_bad
    ["ensures result is Direct ==> result.amount > 1"]
    (← `(command| verify 0x42::shapes_control::join_enum_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_control.join_instance_bad ["ensures result <= 99"]
    (← `(command| verify 0x42::shapes_control::join_instance_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_control.join_param_bad
    ["ensures result != (if c then a else b)"]
    (← `(command| verify 0x42::shapes_control::join_param_bad))

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  expectCuts `«0x42».shapes_control.join_mut_bad [(percent, exit)]
    (← `(command| verify 0x42::shapes_control::join_mut_bad)) (some [])

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_control.loop_scalar_bad ["ensures result <= 99"]
    (← `(command| verify 0x42::shapes_control::loop_scalar_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_control.loop_struct_bad ["ensures result.value <= 99"]
    (← `(command| verify 0x42::shapes_control::loop_struct_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_control.loop_element_bad ["ensures result <= 99"]
    (← `(command| verify 0x42::shapes_control::loop_element_bad))

/-! ## Positive controls -/

verify 0x42::shapes_control::join_struct_ok
verify 0x42::shapes_control::join_mixed_ok
verify 0x42::shapes_control::join_enum_ok
verify 0x42::shapes_control::join_instance_ok
verify 0x42::shapes_control::join_param_ok
verify 0x42::shapes_control::join_mut_ok
verify 0x42::shapes_control::loop_scalar_ok
verify 0x42::shapes_control::loop_struct_ok
verify 0x42::shapes_control::loop_element_ok

end VerexSL.Tests.Shapes.Control
