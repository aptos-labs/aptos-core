-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! # Data invariants where a mutation ends

A mutation owes the data invariant of what it changed where its loan dies,
whether the loan is of a local or of global memory. A borrow of global
memory has no lending local, so the invariant is owed through the borrowing
reference, of the current value its resolution ties to the stored prophecy;
a narrowed borrow (`&mut R[a].f`) owes the resource's invariant, the root of
the borrow being the resource. A function that only mutates such a resource
is verified for the invariant without a specification of its own, and a
caller takes the referent's invariant from the callee's contract.

The obligation is the invariant at every depth, owed where the borrow's
root is written back and at the death of a `&mut` parameter's loan, as the
Move Prover owes it (`memory_instrumentation.rs`, `PackRefDeep` at a
`LocalRoot` or `GlobalRoot` write-back and at a parameter's death; an
intermediate reference node owes nothing of its own). A value therefore may
be broken through a reborrow and repaired before its root is written back,
but it cannot be read by a callee meanwhile: the callee assumes its
referent's invariant at entry, which the caller owes where it hands the
reference over.
-/

namespace VerexE2ETests.Check.Storage.MutationEnds

verex module 0x42::mutation_ends where
  struct Tally has Key where
    count : u64
  spec Tally where
    invariant count > 0

  struct Local has Copy, Drop where
    value : u64
  spec Local where
    invariant value > 0

  fun bump_local() -> Local := do
    let mut l := new Local { value := 1 }
    let r := &mut l
    r.value := r.value + 1
    l

  fun zero_local() -> Local := do
    let mut l := new Local { value := 1 }
    let r := &mut l
    r.value := 0 -- error: the invariant fails where the mutation ends
    l

  fun bump(address : Address) -> Unit := do
    let t := &mut Tally[address]
    t.count := t.count + 1
  spec bump where
    pragma aborts_if_is_partial
    modifies global<Tally>(address)

  fun zero(address : Address) -> Unit := do
    let t := &mut Tally[address]
    t.count := 0 -- error: the invariant fails where the mutation ends
  spec zero where
    pragma aborts_if_is_partial
    modifies global<Tally>(address)

  fun zero_field(address : Address) -> Unit := do
    let count := &mut Tally[address].count
    *count := 0 -- error: the invariant fails where the mutation ends
  spec zero_field where
    pragma aborts_if_is_partial
    modifies global<Tally>(address)

  -- No specification of its own: selected for verification all the same.
  fun zero_spec_less(address : Address) -> Unit := do
    let t := &mut Tally[address]
    t.count := 0 -- error: the invariant fails where the mutation ends

  fun one() -> u64 := 1

  fun zero_of() -> u64 := 0

  -- The loan dies after a call made while it is live.
  fun bump_across_call(address : Address) -> Unit := do
    let t := &mut Tally[address]
    t.count := one()
  spec bump_across_call where
    pragma aborts_if_is_partial
    modifies global<Tally>(address)

  fun zero_across_call(address : Address) -> Unit := do
    let t := &mut Tally[address]
    t.count := zero_of() -- error: the invariant fails where the mutation ends
  spec zero_across_call where
    pragma aborts_if_is_partial
    modifies global<Tally>(address)

  -- A callee's referent: owed at the callee's exit, and the caller works
  -- from the callee's contract, the loan being consumed by the call.
  fun set_one(t : &mut Tally) -> Unit := t.count := 1
  spec set_one where
    aborts_if false
    ensures t.count == 1

  fun set_zero(t : &mut Tally) -> Unit :=
    t.count := 0 -- error: the referent's invariant fails at the exit

  fun calls_set_one(address : Address) -> u64 := do
    set_one(&mut Tally[address])
    Tally[address].count
  spec calls_set_one where
    pragma aborts_if_is_partial
    modifies global<Tally>(address)
    ensures result > 0

  -- Nested invariants, owed at every depth where the root is written back.
  struct Inner has Copy, Drop where
    v : u64
  spec Inner where
    invariant v > 0

  struct Outer has Copy, Drop where
    inner : Inner
  spec Outer where
    invariant inner.v > 1

  fun observe(x : &mut Outer) -> u64 := x.inner.v
  spec observe where
    pragma opaque
    aborts_if false
    ensures result > 1

  -- Broken through a reborrow and repaired before the root is written back:
  -- the root's obligation holds, and no intermediate level owes its own.
  fun repaired_before_root() -> Outer := do
    let mut outer := new Outer { inner := new Inner { v := 2 } }
    let inner := &mut outer.inner
    inner.v := 0
    inner.v := 2
    outer

  -- The same break cannot reach a callee: the call owes the referent's
  -- invariant, which the callee assumes at entry.
  fun escapes_to_callee() -> u64 := do
    let mut outer := new Outer { inner := new Inner { v := 2 } }
    let r := &mut outer
    let inner := &mut r.inner
    inner.v := 0
    let observed := observe(r) -- error: the referent's invariant is owed here
    r.inner.v := 2
    observed

end VerexE2ETests.Check.Storage.MutationEnds
