-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport
import VerexSL.Tests.Shapes.Support

/-!
# Closures a proof sees, and function values in storage

An invocation of a closure whose target and captures are in sight is a call
of its target through the target's contract: a capture in the leading
position, a generic target at a concrete type argument and at the caller's
own parameter, a closure held in a struct field and in a vector, and a
two-argument function value passed to a generic higher-order function. Then
a function value held by an enum resource: taken out of storage and invoked,
and published, under the frame the function claims.

A positive states the target's own `ensures` of the invocation, or the
frame; the negative states a different result, or leaves the take out of
the frame. The module of the stored function values is verified where it is
elaborated, as the closure fixtures of `verex-e2e-tests` are; `apply_state`,
which invokes a function value out of a matched variant, is not verified
here (`Closures.stored_ok` is the open cell of that invocation).
-/

namespace VerexSL.Tests.Shapes.KnownClosures

set_option Elab.async false

verex module 0x42::shapes_known_closures where
  pragma verify = false

  struct Op has Copy, Drop where
    f : Fn(u64) -> u64 has Copy, Drop

  fun add(x : u64, y : u64) -> u64 := x + y
  spec add where
    ensures result == x + y
    aborts_if x + y > MAX_U64

  fun pick {T has Copy, Drop}(first : Bool, a : T, b : T) -> T :=
    if first then a else b
  spec pick where
    ensures result == (if first then a else b)
    aborts_if false

  fun choose(flag : Bool, y : u64) -> u64 := if flag then y else 0
  spec choose where
    ensures result == (if flag then y else 0)

  fun apply_both {A} {B} {R}(f : Fn(A, B) -> R, x : A, y : B) -> R := invoke(f, x, y)
  spec apply_both where
    ensures ensures_of<f>(x, y, result)

  -- 8.8 a capture in the leading position
  fun capture_bad(x : u64, y : u64) -> u64 := do
    let f := function[Fn(u64) -> u64 has Copy, Drop](add, x)
    invoke(f, y)
  spec capture_bad where
    ensures result == x + y + 1
    aborts_if x + y > MAX_U64

  fun capture_ok(x : u64, y : u64) -> u64 := do
    let f := function[Fn(u64) -> u64 has Copy, Drop](add, x)
    invoke(f, y)
  spec capture_ok where
    ensures result == x + y
    aborts_if x + y > MAX_U64

  -- 8.9 a generic target at a concrete type argument, and at the caller's
  -- own parameter
  fun generic_bad(first : Bool, a : u64, b : u64) -> u64 := do
    let f := function[Fn(u64, u64) -> u64 has Copy, Drop](pick::<u64>, first, _, _)
    invoke(f, a, b)
  spec generic_bad where
    ensures result == (if first then b else a)
    aborts_if false

  fun generic_ok(first : Bool, a : u64, b : u64) -> u64 := do
    let f := function[Fn(u64, u64) -> u64 has Copy, Drop](pick::<u64>, first, _, _)
    invoke(f, a, b)
  spec generic_ok where
    ensures result == (if first then a else b)
    aborts_if false

  fun generic_param_ok {T has Copy, Drop}(first : Bool, a : T, b : T) -> T := do
    let f := function[Fn(T, T) -> T has Copy, Drop](pick::<T>, first, _, _)
    invoke(f, a, b)
  spec generic_param_ok where
    ensures result == (if first then a else b)
    aborts_if false

  -- 8.10 a closure held in a struct field, and in a vector
  fun field_capture_ok(x : u64, y : u64) -> u64 := do
    let op := new Op { f := function[Fn(u64) -> u64 has Copy, Drop](add, x) }
    invoke(op.f, y)
  spec field_capture_ok where
    ensures result == x + y
    aborts_if x + y > MAX_U64

  fun element_ok(x : u64, y : u64) -> u64 := do
    let fs := vector<Fn(u64) -> u64 has Copy, Drop>[function[Fn(u64) -> u64 has Copy, Drop](add, x)]
    invoke(fs[0], y)
  spec element_ok where
    ensures result == x + y
    aborts_if x + y > MAX_U64

  -- 8.11 a two-argument function value through a generic higher-order function
  fun two_arguments_bad(flag : Bool, x : u64) -> u64 :=
    apply_both(function[Fn(Bool, u64) -> u64](choose), flag, x)
  spec two_arguments_bad where
    ensures result == x

  fun two_arguments_ok(flag : Bool, x : u64) -> u64 :=
    apply_both(function[Fn(Bool, u64) -> u64](choose), flag, x)
  spec two_arguments_ok where
    ensures flag ==> result == x
    ensures !flag ==> result == 0

verex module 0x42::shapes_stored_closures where
  enum State has Key, Copy, Drop where
    | Empty
    | Pending (f : Fn(u64) -> u64 has Copy, Drop, Store)

  public fun identity(x : u64) -> u64 := x
  spec identity where
    pragma opaque
    aborts_if false
    ensures result == x

  fun apply_state(s : State, x : u64) -> u64 :=
    match s with
    | State::Empty {} => x
    | State::Pending { f := f } => invoke(f, x)
  spec apply_state where
    pragma verify = false
    pragma aborts_if_is_partial

  -- 8.12 a function value taken out of storage and invoked, under the frame
  -- of the take
  fun consume_ok(owner : Address, x : u64) -> u64 := do
    let state := move_from<State>(owner)
    apply_state(state, x)
  spec consume_ok where
    pragma aborts_if_is_partial
    modifies global<State>(owner)

  fun consume_bad(owner : Address, x : u64) -> u64 := do
    let state := move_from<State>(owner)
    apply_state(state, x)
  spec consume_bad where
    pragma verify = false
    pragma aborts_if_is_partial

  -- 8.13 a function value published
  fun publish_ok(s : &Signer) -> Unit := move_to<State>(s, new State::Pending {
    f := function[Fn(u64) -> u64 has Copy, Drop, Store](identity) })
  spec publish_ok where
    pragma aborts_if_is_partial
    modifies global<State>(s.address)

/-! ## Negative controls -/

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_known_closures.capture_bad ["ensures result == x + y + 1"]
    (← `(command| verify 0x42::shapes_known_closures::capture_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_known_closures.generic_bad
    ["ensures result == (if first then b else a)"]
    (← `(command| verify 0x42::shapes_known_closures::generic_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_known_closures.two_arguments_bad ["ensures result == x"]
    (← `(command| verify 0x42::shapes_known_closures::two_arguments_bad))

-- The take is outside the frame `consume_bad` claims; the failure names no
-- source clause.
run_cmd do
  expectResidual `«0x42».shapes_stored_closures.consume_bad
    (← `(command| verify 0x42::shapes_stored_closures::consume_bad))

/-! ## Positive controls

`consume_ok` and `publish_ok` are verified where their module is elaborated. -/

verify 0x42::shapes_known_closures::capture_ok
verify 0x42::shapes_known_closures::generic_ok
verify 0x42::shapes_known_closures::generic_param_ok
verify 0x42::shapes_known_closures::field_capture_ok
verify 0x42::shapes_known_closures::element_ok
verify 0x42::shapes_known_closures::two_arguments_ok

end VerexSL.Tests.Shapes.KnownClosures
