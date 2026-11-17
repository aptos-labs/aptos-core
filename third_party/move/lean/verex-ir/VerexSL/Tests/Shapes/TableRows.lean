-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport
import VerexSL.Tests.Shapes.Support

/-!
# A table's rows at resolved type arguments, and tables behind an owner

A table's entries carry the data invariant of their value type at the type
arguments the table is used at: a generic struct with an invariant of its
own at the caller's parameter, a nested table, and a value whose invariant
is an opaque specification function at the type argument. Then a table
behind an owner: a generic struct holding a table of values over its own
parameter, and a resource built over a fresh table and published.

A positive states what the row's invariant gives of the value read, or what
the operation gives of the owner; the negative asks for more, or for the
invariant at another type argument.
-/

namespace VerexSL.Tests.Shapes.TableRows

set_option Elab.async false

verex module 0x42::shapes_table_rows where
  pragma verify = false

  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address

  native fun host_new {K} {V}() -> Table<K, V>
  native fun host_borrow {K} {V}(t : &Table<K, V>, k : K) -> &V
  native fun host_contains {K} {V}(t : &Table<K, V>, k : K) -> Bool

  @[map_new (Table)]
  public fun new {K} {V}() -> Table<K, V> := host_new()
  @[map_borrow (Table)]
  public fun borrow {K} {V}(t : &Table<K, V>, k : K) -> &V := host_borrow(t, k)
  @[map_has_key (Table)]
  public fun contains {K} {V}(t : &Table<K, V>, k : K) -> Bool := host_contains(t, k)

  @[map_spec_has_key (Table)]
  opaque spec fun has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_spec_get (Table)]
  opaque spec fun get {K} {V}(t : Table<K, V>, k : K) : V

  struct Payload has Copy, Drop, Store where
    value : u64
  spec Payload where
    invariant value <= 10

  struct Cell {T} has Copy, Drop, Store where
    entries : Vector<T>
  spec Cell where
    invariant entries.length <= 1

  opaque spec fun permitted {T}(value : u64) : Bool

  struct Tagged {T} has Copy, Drop, Store where
    value : u64
  spec Tagged where
    invariant permitted::<T>(value)

  struct Slot {T} has Copy, Drop, Store where
    item : T

  struct Slots {T} has Store where
    slots : Table<u64, Slot<T> >

  struct Registry has Key where
    entries : Table<Address, Payload>

  -- 7.15 a row whose value is a generic struct with an invariant of its own
  fun row_instance_bad {T has Copy, Drop, Store}(t : &Table<u64, Cell<T> >, k : u64) -> u64 :=
    borrow(t, k).entries.length
  spec row_instance_bad where
    requires has(t, k)
    aborts_if false
    ensures result < 1

  fun row_instance_ok {T has Copy, Drop, Store}(t : &Table<u64, Cell<T> >, k : u64) -> u64 :=
    borrow(t, k).entries.length
  spec row_instance_ok where
    requires has(t, k)
    aborts_if false
    ensures result <= 1

  -- 7.16 a nested table: the inner entries' invariant
  fun row_nested_bad(t : &Table<u64, Table<u64, Payload> >, a : u64, b : u64) -> u64 :=
    borrow(borrow(t, a), b).value
  spec row_nested_bad where
    requires has(t, a)
    requires has(get(t, a), b)
    aborts_if false
    ensures result < 10

  fun row_nested_ok(t : &Table<u64, Table<u64, Payload> >, a : u64, b : u64) -> u64 :=
    borrow(borrow(t, a), b).value
  spec row_nested_ok where
    requires has(t, a)
    requires has(get(t, a), b)
    aborts_if false
    ensures result <= 10

  -- 7.17 an invariant that is an opaque specification function at the type
  -- argument
  fun row_tagged_bad {T}(t : &Table<u64, Tagged<T> >, k : u64) -> u64 := borrow(t, k).value
  spec row_tagged_bad where
    requires has(t, k)
    aborts_if false
    ensures permitted::<Bool>(result)

  fun row_tagged_ok {T}(t : &Table<u64, Tagged<T> >, k : u64) -> u64 := borrow(t, k).value
  spec row_tagged_ok where
    requires has(t, k)
    aborts_if false
    ensures permitted::<T>(result)

  -- 7.18 a generic owner of a table of values over its own parameter
  fun owner_param_bad {T}(s : &Slots<T>, k : u64) -> Bool := contains(&s.slots, k)
  spec owner_param_bad where
    aborts_if false
    ensures result == !has(s.slots, k)

  fun owner_param_ok {T}(s : &Slots<T>, k : u64) -> Bool := contains(&s.slots, k)
  spec owner_param_ok where
    aborts_if false
    ensures result == has(s.slots, k)

  -- 7.19 a resource built over a fresh table and published
  fun publish_new_bad(s : &Signer) -> Unit :=
    move_to<Registry>(s, new Registry { entries := new::<Address, Payload>() })
  spec publish_new_bad where
    modifies global<Registry>(s.address)
    aborts_if exists<Registry>(s.address)
    ensures !exists<Registry>(s.address)

  fun publish_new_ok(s : &Signer) -> Unit :=
    move_to<Registry>(s, new Registry { entries := new::<Address, Payload>() })
  spec publish_new_ok where
    modifies global<Registry>(s.address)
    aborts_if exists<Registry>(s.address)
    ensures exists<Registry>(s.address)

/-! ## Negative controls -/

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_table_rows.row_instance_bad ["ensures result < 1"]
    (← `(command| verify 0x42::shapes_table_rows::row_instance_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_table_rows.row_nested_bad ["ensures result < 10"]
    (← `(command| verify 0x42::shapes_table_rows::row_nested_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_table_rows.row_tagged_bad
    ["ensures permitted::<Bool>(result)"]
    (← `(command| verify 0x42::shapes_table_rows::row_tagged_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_table_rows.owner_param_bad
    ["ensures result == !has(s.slots, k)"]
    (← `(command| verify 0x42::shapes_table_rows::owner_param_bad))

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».shapes_table_rows.publish_new_bad
    ["ensures !exists<Registry>(s.address)"]
    (← `(command| verify 0x42::shapes_table_rows::publish_new_bad))

/-! ## Positive controls -/

verify 0x42::shapes_table_rows::row_instance_ok
verify 0x42::shapes_table_rows::row_nested_ok
verify 0x42::shapes_table_rows::row_tagged_ok
verify 0x42::shapes_table_rows::owner_param_ok
verify 0x42::shapes_table_rows::publish_new_ok

end VerexSL.Tests.Shapes.TableRows
