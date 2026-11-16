-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# A leaf reads a declaration's own invariant at its own frame

The data invariant of a generic declaration, in a generic function at the
declaration's own parameters and at a closed instance, in the shapes of a
clause: an order and an equality over two fields of the parameter's type, over
fields of a nested generic struct, and quantified over a vector of the
parameter's type and of a nested generic struct. A function that is handed a
value reads the clause it needs from the value's fact; a function that makes a
value owes the clause, which it has from its precondition. Each shape has a
function that does not verify: it asks more than the clause states, or makes a
value without what the clause needs.
-/

verex module 0x42::own_bridge_leaves where
  struct Entry {K has Copy, Drop, Store} has Copy, Drop, Store where
    key : K

  -- An order over two fields of the parameter's type.
  struct Two {K has Copy, Drop, Store} has Copy, Drop, Store where
    first : K
    second : K
  spec Two where
    invariant core.prim.compare(first, second) < 0

  fun two {K has Copy, Drop, Store}(value : &Two<K>) -> Bool :=
    core.prim.compare(&value.first, &value.second) < 0
  spec two where
    ensures result
    aborts_if false

  -- The same over fields of a nested generic struct.
  struct Nested {K has Copy, Drop, Store} has Copy, Drop, Store where
    first : Entry<K>
    second : Entry<K>
  spec Nested where
    invariant core.prim.compare(first.key, second.key) < 0

  fun nested {K has Copy, Drop, Store}(value : &Nested<K>) -> Bool :=
    core.prim.compare(&value.first.key, &value.second.key) < 0
  spec nested where
    ensures result
    aborts_if false

  -- A quantified order over a vector of the parameter's type.
  struct Keys {K has Copy, Drop, Store} has Copy, Drop, Store where
    keys : Vector<K>
  spec Keys where
    invariant forall (i : Int), forall (j : Int), 0 <= i && i < j && j < keys.length ==>
      core.prim.compare(keys[i], keys[j]) < 0

  fun keys {K has Copy, Drop, Store}(value : &Keys<K>, i : u64, j : u64) -> Bool :=
    core.prim.compare(&value.keys[i], &value.keys[j]) < 0
  spec keys where
    requires i < j && j < value.keys.length
    ensures result
    aborts_if false

  -- The same over a vector of a nested generic struct.
  struct Entries {K has Copy, Drop, Store} has Copy, Drop, Store where
    entries : Vector<Entry<K> >
  spec Entries where
    invariant forall (i : Int), forall (j : Int), 0 <= i && i < j && j < entries.length ==>
      core.prim.compare(entries[i].key, entries[j].key) < 0

  fun entries {K has Copy, Drop, Store}(value : &Entries<K>, i : u64, j : u64) -> Bool :=
    core.prim.compare(&value.entries[i].key, &value.entries[j].key) < 0
  spec entries where
    requires i < j && j < value.entries.length
    ensures result
    aborts_if false

  -- Controls: the same four shapes with equality in place of the order.
  struct TwoSame {K has Copy, Drop, Store} has Copy, Drop, Store where
    first : K
    second : K
  spec TwoSame where
    invariant first == second

  fun two_same {K has Copy, Drop, Store}(value : &TwoSame<K>) -> Bool :=
    core.prim.equal(&value.first, &value.second)
  spec two_same where
    ensures result
    aborts_if false

  struct NestedSame {K has Copy, Drop, Store} has Copy, Drop, Store where
    first : Entry<K>
    second : Entry<K>
  spec NestedSame where
    invariant first.key == second.key

  fun nested_same {K has Copy, Drop, Store}(value : &NestedSame<K>) -> Bool :=
    core.prim.equal(&value.first.key, &value.second.key)
  spec nested_same where
    ensures result
    aborts_if false

  struct KeysSame {K has Copy, Drop, Store} has Copy, Drop, Store where
    keys : Vector<K>
  spec KeysSame where
    invariant forall (i : Int), forall (j : Int), 0 <= i && i < j && j < keys.length ==>
      keys[i] == keys[j]

  fun keys_same {K has Copy, Drop, Store}(value : &KeysSame<K>, i : u64, j : u64) -> Bool :=
    core.prim.equal(&value.keys[i], &value.keys[j])
  spec keys_same where
    requires i < j && j < value.keys.length
    ensures result
    aborts_if false

  struct EntriesSame {K has Copy, Drop, Store} has Copy, Drop, Store where
    entries : Vector<Entry<K> >
  spec EntriesSame where
    invariant forall (i : Int), forall (j : Int), 0 <= i && i < j && j < entries.length ==>
      entries[i].key == entries[j].key

  fun entries_same {K has Copy, Drop, Store}(value : &EntriesSame<K>, i : u64, j : u64) -> Bool :=
    core.prim.equal(&value.entries[i].key, &value.entries[j].key)
  spec entries_same where
    requires i < j && j < value.entries.length
    ensures result
    aborts_if false

  -- Controls: the order at a closed instance, and as a precondition in place
  -- of the invariant.
  fun two_closed(value : &Two<u64>) -> Bool :=
    core.prim.compare(&value.first, &value.second) < 0
  spec two_closed where
    ensures result
    aborts_if false

  struct Free {K has Copy, Drop, Store} has Copy, Drop, Store where
    first : K
    second : K

  fun two_required {K has Copy, Drop, Store}(value : &Free<K>) -> Bool :=
    core.prim.compare(&value.first, &value.second) < 0
  spec two_required where
    requires core.prim.compare(value.first, value.second) < 0
    ensures result
    aborts_if false

  -- A value made owes the declaration's clause at the leaf's frame.
  fun make_two {K has Copy, Drop, Store}(first : K, second : K) -> Two<K> :=
    new Two<K> { first := first, second := second }
  spec make_two where
    requires core.prim.compare(first, second) < 0
    aborts_if false

  fun make_nested_same {K has Copy, Drop, Store}(key : K) -> NestedSame<K> :=
    new NestedSame<K> { first := new Entry<K> { key := key }, second := new Entry<K> { key := key } }
  spec make_nested_same where
    aborts_if false

verex module 0x42::own_bridge_leaf_negatives where
  pragma verify = false
  struct Entry {K has Copy, Drop, Store} has Copy, Drop, Store where
    key : K

  struct Two {K has Copy, Drop, Store} has Copy, Drop, Store where
    first : K
    second : K
  spec Two where
    invariant core.prim.compare(first, second) < 0

  -- The clause orders the first field below the second, not the reverse.
  fun two_reversed {K has Copy, Drop, Store}(value : &Two<K>) -> Bool :=
    core.prim.compare(&value.second, &value.first) < 0
  spec two_reversed where
    ensures result -- error: the invariant states the other order
    aborts_if false

  struct NestedSame {K has Copy, Drop, Store} has Copy, Drop, Store where
    first : Entry<K>
    second : Entry<K>
  spec NestedSame where
    invariant first.key == second.key

  fun nested_differ {K has Copy, Drop, Store}(value : &NestedSame<K>) -> Bool :=
    !core.prim.equal(&value.first.key, &value.second.key)
  spec nested_differ where
    ensures result -- error: the invariant states the keys equal
    aborts_if false

  struct KeysSame {K has Copy, Drop, Store} has Copy, Drop, Store where
    keys : Vector<K>
  spec KeysSame where
    invariant forall (i : Int), forall (j : Int), 0 <= i && i < j && j < keys.length ==>
      keys[i] == keys[j]

  fun keys_differ {K has Copy, Drop, Store}(value : &KeysSame<K>, i : u64, j : u64) -> Bool :=
    !core.prim.equal(&value.keys[i], &value.keys[j])
  spec keys_differ where
    requires i < j && j < value.keys.length
    ensures result -- error: the invariant states the keys equal
    aborts_if false

  -- A value made without the order its declaration states.
  fun make_two_unordered {K has Copy, Drop, Store}(first : K, second : K) -> Two<K> :=
    new Two<K> { first := first, second := second }
  spec make_two_unordered where
    aborts_if false

/--
error: the specification clause `ensures result` is not established
---
error: verex verification failed: the automatic verification of `two_reversed` failed; provide a proof in the module:
  verify two_reversed by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::own_bridge_leaf_negatives::two_reversed

/--
error: the specification clause `ensures result` is not established
---
error: verex verification failed: the automatic verification of `nested_differ` failed; provide a proof in the module:
  verify nested_differ by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::own_bridge_leaf_negatives::nested_differ

/--
error: the specification clause `ensures result` is not established
---
error: verex verification failed: the automatic verification of `keys_differ` failed; provide a proof in the module:
  verify keys_differ by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::own_bridge_leaf_negatives::keys_differ

-- The clause owed is the declaration's, under its own source range.
/--
error: the data invariant `invariant core.prim.compare(first, second) < 0` does not hold where a value is constructed
---
error: verex verification failed: the automatic verification of `make_two_unordered` failed; provide a proof in the module:
  verify make_two_unordered by lean =>
    verex_obligations
    verex_on
    | data_invariant_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::own_bridge_leaf_negatives::make_two_unordered
