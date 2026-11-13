-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
A field of a generic enum type, in a generic struct at the struct's own
parameter or at a concrete argument, in a non-generic struct, and as a
vector's elements, is viewed through the enum's native view at that spelling.
-/

namespace VerexSL.Tests.Check.Generics.GenericEnumFields

verex module 0x42::generic_enum_fields where
  enum Choice {T has Copy, Drop, Store} has Copy, Drop, Store where
    | None
    | Some (value : T)
  struct Holder {T has Copy, Drop, Store} has Copy, Drop, Store where
    item : Choice<T>
    count : u64
  struct Fixed has Copy, Drop, Store where
    item : Choice<u64>
  struct Many {T has Copy, Drop, Store} has Copy, Drop, Store where
    items : Vector<Choice<T> >
  struct Mixed {T has Copy, Drop, Store} has Copy, Drop, Store where
    items : Vector<T>
    cap : Choice<u64>

  fun is_some {T has Copy, Drop, Store}(h : &Holder<T>) -> Bool := h.item is Some
  spec is_some where
    aborts_if false
    ensures result == (h.item is Some)
  verify is_some

  fun fixed_value(f : &Fixed) -> u64 := if f.item is Some then 1u64 else 0u64
  spec fixed_value where
    aborts_if false
    ensures (f.item is Some) ==> result == 1
    ensures (f.item is None) ==> result == 0
  verify fixed_value

  fun many_length {T has Copy, Drop, Store}(m : &Many<T>) -> u64 := m.items.length
  spec many_length where
    aborts_if false
    ensures result == m.items.length
  verify many_length

  fun item_count {T has Copy, Drop, Store}(m : &Mixed<T>) -> u64 := m.items.length
  spec item_count where
    aborts_if false
    ensures result == m.items.length
  verify item_count

  -- A generic struct whose parameter is an enum, and an enum whose payload
  -- holds an enum through a struct.
  struct Pair {K has Copy, Drop, Store} {V has Copy, Drop, Store} has Copy, Drop, Store where
    key : K
    value : V
  struct Tree {K has Copy, Drop, Store} {V has Copy, Drop, Store} has Copy, Drop, Store where
    entries : Vector<Pair<K, Choice<V> > >
  struct Nest {T has Copy, Drop, Store} has Copy, Drop, Store where
    slot : Choice<Holder<T> >

  fun tree_size {K has Copy, Drop, Store} {V has Copy, Drop, Store}(t : &Tree<K, V>) -> u64 :=
    t.entries.length
  spec tree_size where
    aborts_if false
    ensures result == t.entries.length
  verify tree_size

  fun nest_full {T has Copy, Drop, Store}(n : &Nest<T>) -> Bool := n.slot is Some
  spec nest_full where
    aborts_if false
    ensures result == (n.slot is Some)
  verify nest_full

  fun wrong_fixed(f : &Fixed) -> u64 := if f.item is Some then 1u64 else 0u64
  spec wrong_fixed where
    pragma verify = false
    ensures result == 0

/--
error: the specification clause `ensures result == 0` is not established
---
error: verex verification failed: the automatic verification of `wrong_fixed` failed; provide a proof in the module:
  verify wrong_fixed by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::generic_enum_fields::wrong_fixed

end VerexSL.Tests.Check.Generics.GenericEnumFields
