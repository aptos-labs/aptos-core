-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-!
# Impure functions in specifications

A function called in a specification is read as a value, which has no reading of
an effect on a reference. Each namespace is rejected at the call, wherever the
specification expression stands, and the message names what makes the callee
impure.
-/

-- A native is what its signature allows.
verex namespace impure::native_takes using rust where
  native fun advance(it : &mut u64) -> u64
  fun check(a : u64) -> u64 := a
  spec check where
    ensures result == advance(a)

verex namespace impure::native_returns using rust where
  native fun first(values : &Vector<u64>) -> &mut u64
  fun check(values : &Vector<u64>) -> u64 := 1
  spec check where
    ensures result == first(values)

-- A body is what it does, through the calls it makes.
verex namespace impure::chain using rust where
  native fun advance(it : &mut u64) -> u64
  fun middle(a : u64) -> u64 := do
    let mut it := a
    advance(&mut it)
  fun outer(a : u64) -> u64 := middle(a)
  fun check(a : u64) -> u64 := a
  spec check where
    ensures result == outer(a)

verex namespace impure::borrow using rust where
  fun borrows(a : u64) -> u64 := do
    let mut value := a
    let place := &mut value
    *place
  fun check(a : u64) -> u64 := a
  spec check where
    ensures result == borrows(a)

verex namespace impure::mutation using rust where
  fun set(place : &mut u64) -> Unit := *place := 1u64
  fun check(a : u64) -> u64 := a
  spec check where
    ensures set(a) == set(a)

verex namespace impure::field_write using rust where
  struct Point has Copy, Drop where
    x : u64
  fun set(point : &mut Point) -> Unit := point.x := 1u64
  fun check(point : Point) -> u64 := 1
  spec check where
    ensures set(point) == set(point)

verex namespace impure::dropped using rust where
  fun forgets(place : &mut u64) -> Unit := drop(place)
  fun check(a : u64) -> u64 := a
  spec check where
    ensures forgets(a) == forgets(a)

-- Mutual recursion ends at the effect on the way round.
verex namespace impure::cycle using rust where
  fun left(place : &mut u64, n : u64) -> u64 := if n == 0u64 then 0u64 else right(place, n - 1u64)
  fun right(place : &mut u64, n : u64) -> u64 := do
    *place := n
    left(place, n)
  fun check(a : u64) -> u64 := a
  spec check where
    ensures result == left(a, 1)

-- Every specification expression.
verex namespace impure::requires_clause using rust where
  native fun advance(it : &mut u64) -> u64
  fun check(a : u64) -> u64 := a
  spec check where
    requires advance(a) == 1

verex namespace impure::panics_clause using rust where
  native fun advance(it : &mut u64) -> u64
  fun check(a : u64) -> u64 := a
  spec check where
    panics_if advance(a) == 1

verex namespace impure::spec_function using rust where
  native fun advance(it : &mut u64) -> u64
  spec fun read_it(a : u64) : u64 := advance(a)
  fun check(a : u64) -> u64 := a
  spec check where
    ensures result == read_it(a)

verex namespace impure::loop_invariant using rust where
  native fun advance(it : &mut u64) -> u64
  fun count(n : u64) -> u64 := do
    let mut i := 0u64
    while i < n do
      i := i + 1u64
    i
  spec count where
    loop 1 where
      invariant advance(i) == i

verex namespace impure::struct_invariant using rust where
  native fun advance(it : &mut u64) -> u64
  struct Counter has Copy, Drop where
    value : u64
  spec Counter where
    invariant advance(value) == 0

verex namespace impure::lemma_statement using rust where
  native fun advance(it : &mut u64) -> u64
  spec lemma about(a : u64) where
    ensures advance(a) == a

-- In the Move profile too, through its receiver notation.
verex module 0x42::impure_receiver where
  use 0x1::vector
  fun check(values : Vector<u64>) -> u64 := 1
  spec check where
    ensures values.push_back(1) == values.push_back(1)

verex module 0x42::impure_mutation where
  fun set(place : &mut u64) -> Unit := *place := 1u64
  fun check(a : u64) -> u64 := a
  spec check where
    ensures set(a) == set(a)

-- A write of global memory is an effect, which `pureness_checker` does not treat
-- as one. Each claim would hold were the write read as nothing.
verex module 0x42::impure_taken where
  struct R has Key, Drop where
    value : u64
  fun removed(a : Address) -> Bool := do
    move_from<R>(a);
    exists<R>(a)
  fun check(a : Address) -> Bool := true
  spec check where
    ensures removed(a) == exists<R>(a)

verex module 0x42::impure_taken_value where
  struct R has Key, Drop where
    value : u64
  fun removed(a : Address) -> Bool := do
    let r := move_from<R>(a);
    exists<R>(a)
  fun check(a : Address) -> Bool := true
  spec check where
    ensures removed(a) == exists<R>(a)

verex module 0x42::impure_published where
  struct R has Key, Drop where
    value : u64
  fun added(account : &Signer) -> Bool := do
    move_to<R>(account, new R { value := 1 });
    exists<R>(account.address)
  fun check(account : &Signer) -> Bool := true
  spec check where
    ensures added(account) == exists<R>(account.address)

verex module 0x42::impure_global_borrow where
  struct R has Key, Drop where
    value : u64
  fun bumped(a : Address) -> u64 := do
    let place := &mut R[a];
    place.value := 1;
    R[a].value
  fun check(a : Address) -> u64 := 1
  spec check where
    requires exists<R>(a)
    ensures result == bumped(a)

verex module 0x42::impure_storage_write where
  struct R has Key, Drop where
    value : u64
  fun bumped(a : Address) -> u64 := do
    R[a].value := 1;
    R[a].value
  fun check(a : Address) -> u64 := 1
  spec check where
    requires exists<R>(a)
    ensures result == bumped(a)

verex module 0x42::impure_global_chain where
  struct R has Key, Drop where
    value : u64
  fun inner(a : Address) -> Bool := do
    move_from<R>(a);
    true
  fun outer(a : Address) -> Bool := inner(a)
  fun check(a : Address) -> Bool := true
  spec check where
    ensures outer(a)

-- A function value may do either.
verex module 0x42::impure_function_value where
  fun apply(f : Fn(u64) -> u64, a : u64) -> u64 := invoke(f, a)
  fun check(f : Fn(u64) -> u64, a : u64) -> u64 := a
  spec check where
    ensures result == apply(f, a)

-- Reads of global memory, and a specification's own state operations, which are
-- values over labelled states, are no effect.
verex module 0x42::pure_global_reads where
  struct R has Key, Drop where
    value : u64
  fun held(a : Address) -> Bool := exists<R>(a)
  fun read_value(a : Address) -> u64 := R[a].value
  spec fun published(a : Address, v : u64) : Bool := publish<R>(a, new R { value := v })
  fun check(a : Address) -> Bool := exists<R>(a)
  spec check where
    ensures result == held(a)
  fun reader(a : Address) -> u64 := R[a].value
  spec reader where
    requires exists<R>(a)
    ensures result == read_value(a)
  fun put(account : &Signer) -> Unit := move_to<R>(account, new R { value := 7 })
  spec put where
    aborts_if exists<R>(account.address)
    ensures ..S |~ publish<R>(account.address, new R { value := 7 })
    ensures S |~ exists<R>(account.address)
    ensures published(account.address, 7)
    modifies global<R>(account.address)

verex module 0x42::impure_borrow_global where
  struct R has Key, Drop where
    value : u64
  fun bumped(a : Address) -> u64 := do
    let place := borrow_global_mut<R>(a);
    place.value := 1;
    place.value
  fun check(a : Address) -> u64 := 1
  spec check where
    requires exists<R>(a)
    ensures result == bumped(a)
