-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# The results of a stored function value

A function value in a resource field whose result type states an invariant
carries, in the behavioral stored invariant, that its invocations return
valid results: a publish proves the fact of the value it stores. A field
whose result type states no invariant carries no such fact. An invocation
of a value read from such a field reads the field's clause from the stored
invariant at the reader's own frame, of the value read: it returns a valid
result and keeps the memory the field's frame says it keeps. What the
field's clause does not state, a reader cannot claim.
-/

namespace VerexSL.Tests.Check.Closures.StoredResultValidity

verex module 0x99::stored_result_validity where
  struct Percent has Copy, Drop, Store where
    value : u64
  spec Percent where
    invariant value <= 100

  struct Holder has Key where
    f : Fn(u64) -> Percent has Copy, Drop, Store

  struct PlainHolder has Key where
    g : Fn(u64) -> u64 has Copy, Drop, Store

  -- A holder at a type parameter: the result row of its field is the
  -- parameter's, the domain the rule for an unseen invocation supports.
  struct Box {T} has Key where
    h : Fn(u64) -> T has Copy, Drop, Store

  public fun fifty(x : u64) -> Percent := new Percent { value := 50 }
  spec fifty where
    aborts_if false
    ensures result == new Percent { value := 50 }

  -- A function whose results are valid only under its own precondition, so
  -- that a caller storing it cannot justify the fact.
  public fun at_most(x : u64) -> Percent := new Percent { value := x }
  spec at_most where
    requires x <= 100
    aborts_if false
    ensures result.value == x

  public fun identity(x : u64) -> u64 := x
  spec identity where
    aborts_if false
    ensures result == x

  -- A function the caller cannot justify: the publish owes that its
  -- invocations return valid results, and `at_most` returns one only at
  -- arguments it requires.
  fun publish_unjustified(s : &Signer) -> Unit :=
    move_to<Holder>(s, new Holder {
      f := function[Fn(u64) -> Percent has Copy, Drop, Store](at_most) })
  spec publish_unjustified where
    pragma aborts_if_is_partial
    modifies global<Holder>(s.address) -- error: `at_most` returns a valid result only under its precondition

  fun publish_total(s : &Signer) -> Unit :=
    move_to<Holder>(s, new Holder {
      f := function[Fn(u64) -> Percent has Copy, Drop, Store](fifty) })
  spec publish_total where
    pragma aborts_if_is_partial
    modifies global<Holder>(s.address)

  -- An invariant-free result: the field carries no fact, and a publish owes
  -- none.
  fun publish_plain(s : &Signer) -> Unit :=
    move_to<PlainHolder>(s, new PlainHolder {
      g := function[Fn(u64) -> u64 has Copy, Drop, Store](identity) })
  spec publish_plain where
    pragma aborts_if_is_partial
    modifies global<PlainHolder>(s.address)

  -- Returning the result of an invocation of the stored value owes the
  -- result's own data invariant, which the value carries in the stored
  -- invariant of its field. The invocation's rows are decided scalar from a
  -- function-typed parameter's typing assumption (`ScalarAt`), which the
  -- rule for an unseen invocation needs and only such a parameter states,
  -- so the reader takes one; the fact of the value's results is the clause
  -- of its field, read at the reader's frame.
  fun apply_stored {T}(owner : Address, probe : Fn(u64) -> T) -> T :=
    invoke(Box<T>[owner].h, 7)
  spec apply_stored where
    pragma aborts_if_is_partial

  -- The same after the value is moved out of storage.
  fun apply_moved {T}(owner : Address, probe : Fn(u64) -> T) -> T := do
    let box := move_from<Box<T> >(owner)
    invoke(box.h, 7)
  spec apply_moved where
    pragma aborts_if_is_partial
    modifies global<Box<T> >(owner)

  -- The field's clause states the value's results valid and the memory kept:
  -- nothing of whether an invocation aborts.
  fun apply_stored_total {T}(owner : Address, probe : Fn(u64) -> T) -> T :=
    invoke(Box<T>[owner].h, 7)
  spec apply_stored_total where
    requires exists<Box<T> >(owner)
    aborts_if false -- error: the stored value's invocation may abort

  -- Nor does it state which function the value is.
  fun apply_stored_same {T}(owner : Address, probe : Fn(u64) -> T) -> T :=
    invoke(Box<T>[owner].h, 7)
  spec apply_stored_same where
    pragma aborts_if_is_partial
    ensures result == result_of<probe>(7) -- error: the stored value is not the reader's parameter

  -- A publish at a type parameter owes the field's clause of the value it
  -- stores, at its own frame. A parameter that may modify memory does not
  -- keep the memory the field's frame says its value keeps.
  fun publish_box_writer {T}(s : &Signer, h : Fn(u64) -> T has Copy, Drop, Store) -> Unit :=
    move_to<Box<T> >(s, new Box<T> { h }) -- error: `h` may modify memory
  spec publish_box_writer where
    pragma aborts_if_is_partial
    modifies_of<h> *
    modifies global<Box<T> >(s.address)

  -- A parameter that declares no frame keeps memory, and its results are
  -- valid: the publish holds the field's clause of it.
  fun publish_box {T}(s : &Signer, h : Fn(u64) -> T has Copy, Drop, Store) -> Unit :=
    move_to<Box<T> >(s, new Box<T> { h })
  spec publish_box where
    pragma aborts_if_is_partial
    modifies global<Box<T> >(s.address)

  -- A holder without type parameters whose field states no fact of its
  -- results: the invocation of the value read needs none.
  fun apply_plain(owner : Address, probe : Fn(u64) -> u64) -> u64 :=
    invoke(PlainHolder[owner].g, 7)
  spec apply_plain where
    pragma aborts_if_is_partial

end VerexSL.Tests.Check.Closures.StoredResultValidity
