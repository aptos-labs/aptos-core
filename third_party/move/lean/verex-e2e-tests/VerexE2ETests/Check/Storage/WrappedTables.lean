-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-! An intrinsic map holding a table and caching its size (`TableWithLength`)
is, in verification, its abstract map at the table's handle: its
specification roles and the contracts of its roles are the map model, and
the runtime runs its role bodies, which call the table's roles
(`doc/designs/intrinsic-maps.md`, "Abstract representation"). -/

verex module 0x42::wrapped_tables where
  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address
  native fun host_new {K} {V}() -> Table<K, V>
  native fun host_add {K} {V}(t : &mut Table<K, V>, k : K, v : V) -> Unit
  native fun host_remove {K} {V}(t : &mut Table<K, V>, k : K) -> V
  native fun host_contains {K} {V}(t : &Table<K, V>, k : K) -> Bool
  native fun host_borrow {K} {V}(t : &Table<K, V>, k : K) -> &V
  @[map_new (Table)]
  public fun new {K} {V}() -> Table<K, V> := host_new()
  @[map_add_no_override (Table)]
  public fun add {K} {V}(t : &mut Table<K, V>, k : K, v : V) -> Unit := host_add(t, k, v)
  @[map_del_must_exist (Table)]
  public fun remove {K} {V}(t : &mut Table<K, V>, k : K) -> V := host_remove(t, k)
  @[map_has_key (Table)]
  public fun contains {K} {V}(t : &Table<K, V>, k : K) -> Bool := host_contains(t, k)
  @[map_spec_has_key (Table)]
  opaque spec fun table_has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_borrow (Table)]
  public fun borrow {K} {V}(t : &Table<K, V>, k : K) -> &V := host_borrow(t, k)

  @[intrinsic_map]
  struct TableWithLength {K} {V} has Store where
    inner : Table<K, V>
    count : u64
  @[map_new (TableWithLength)]
  public fun wrapped_new {K} {V}() -> TableWithLength<K, V> :=
    new TableWithLength<K, V> { inner := new::<K, V>(), count := 0u64 }
  @[map_add_no_override (TableWithLength)]
  public fun wrapped_add {K} {V}(t : &mut TableWithLength<K, V>, k : K, v : V) -> Unit := do
    add(&mut t.inner, k, v)
    t.count := t.count + 1
  @[map_del_must_exist (TableWithLength)]
  public fun wrapped_remove {K} {V}(t : &mut TableWithLength<K, V>, k : K) -> V := do
    let v := remove(&mut t.inner, k)
    t.count := t.count - 1
    v
  @[map_has_key (TableWithLength)]
  public fun wrapped_contains {K} {V}(t : &TableWithLength<K, V>, k : K) -> Bool :=
    contains(&t.inner, k)
  @[map_borrow (TableWithLength)]
  public fun wrapped_borrow {K} {V}(t : &TableWithLength<K, V>, k : K) -> &V := borrow(&t.inner, k)
  @[map_len (TableWithLength)]
  public fun wrapped_length {K} {V}(t : &TableWithLength<K, V>) -> u64 := t.count
  @[map_spec_has_key (TableWithLength)]
  opaque spec fun has {K} {V}(t : TableWithLength<K, V>, k : K) : Bool
  @[map_spec_get (TableWithLength)]
  opaque spec fun get {K} {V}(t : TableWithLength<K, V>, k : K) : V
  @[map_spec_len (TableWithLength)]
  opaque spec fun size {K} {V}(t : TableWithLength<K, V>) : Int
  @[map_spec_set (TableWithLength)]
  opaque spec fun set {K} {V}(t : TableWithLength<K, V>, k : K, v : V) : TableWithLength<K, V>

  public fun filled() -> TableWithLength<u8, u64> := do
    let mut t := wrapped_new::<u8, u64>()
    wrapped_add(&mut t, 1u8, 3u64)
    wrapped_add(&mut t, 2u8, 4u64)
    t
  spec filled where
    ensures has(result, 1u8) && has(result, 2u8)
    ensures get(result, 2u8) == 4
    ensures size(result) == 2
  verify filled

  public fun insert(t : &mut TableWithLength<u64, u64>, k : u64, v : u64) -> Unit :=
    wrapped_add(t, k, v)
  spec insert where
    aborts_if has(t, k)
    ensures has(t, k) && get(t, k) == v
    ensures size(t) == size(old(t)) + 1
    ensures t == set(old(t), k, v)
  verify insert

  public fun take(t : &mut TableWithLength<u64, u64>, k : u64) -> u64 := wrapped_remove(t, k)
  spec take where
    aborts_if !has(t, k)
    ensures result == get(old(t), k)
    ensures !has(t, k)
    ensures size(t) == size(old(t)) - 1
  verify take

  public fun count(t : &TableWithLength<u64, u64>, k : u64) -> u64 :=
    if wrapped_contains(t, k) then wrapped_length(t) else 0u64
  spec count where
    aborts_if false
    ensures has(t, k) ==> result == size(t)
    ensures result <= size(t)
  verify count

  -- A value read from the map carries its type's range.
  public fun widened(t : &TableWithLength<u64, u128>, k : u64) -> u256 :=
    (*wrapped_borrow(t, k)) as u256 + 1
  spec widened where
    aborts_if !has(t, k)
    ensures result == get(t, k) + 1
  verify widened

  -- A generic owner of a table of values of its own parameter.
  struct Slot {T} has Copy, Drop, Store where
    item : T
  struct Slots {T} has Store where
    slots : Table<u64, Slot<T> >
  public fun slots_has {T}(s : &Slots<T>, k : u64) -> Bool := contains(&s.slots, k)
  spec slots_has where
    aborts_if false
    ensures result == table_has(s.slots, k)
  verify slots_has

  public fun wrong_size(t : &mut TableWithLength<u64, u64>, k : u64, v : u64) -> Unit :=
    wrapped_add(t, k, v)
  spec wrong_size where
    pragma verify = false
    ensures size(t) == size(old(t))

/--
error: the specification clause `ensures size(t) == size(old(t))` is not established
---
error: verex verification failed: the automatic verification of `wrong_size` failed; provide a proof in the module:
  verify wrong_size by
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::wrapped_tables::wrong_size

-- The runtime runs the wrapper's role bodies, which call the table's roles
-- the runtime provides.
open Lean Elab Command VerexIR VerexE2ETests.CheckSupport in
run_cmd do
  let some unit := VerexSL.registeredUnit? (← getEnv) `«0x42».wrapped_tables
    | throwError "missing wrapped tables module"
  let executable ← match Validation.prepareExecution #[VerexIR.Move.semantics] unit with
    | .ok executable => pure executable
    | .error diagnostics => throwError "preparation failed: {repr diagnostics}"
  let run (function : String) (arguments : Array RuntimeValue) (initial : RuntimeState) :
      CommandElabM (RuntimeState × Outcome) := do
    let some (namespaceIndex, _, index, _) := VerexSL.Contract.findFunction? unit function
      | throwError "missing function {function}"
    match Interpreter.run executable 256 ⟨⟨namespaceIndex⟩, ⟨index⟩⟩ arguments initial with
    | .error failure => throwError "{function} failed: {repr failure}"
    | .ok (state, outcome) => pure (state, outcome.value)
  let entry (key value : Int) : RuntimeValue := .tuple #[.integer key, .integer value]
  let wrapped (entries : Array RuntimeValue) : RuntimeValue :=
    .nominal ⟨⟨0⟩, 1⟩ none
      #[.nominal ⟨⟨0⟩, 0⟩ none #[.address "table-0", .vector entries], .integer entries.size]
  let (state, outcome) ← run "filled" #[] {}
  unless outcome == .returned #[wrapped #[entry 1 3, entry 2 4]] do
    throwError "filled: got {repr outcome}"
  unless state.tableHandles == #["table-0"] do
    throwError "filled: handles {repr state.tableHandles}"
  let held : RuntimeState := { tableHandles := #["table-0"] }
  let (_, outcome) ← run "count" #[wrapped #[entry 1 3, entry 2 4], .integer 2] held
  unless outcome == .returned #[.integer 2] do
    throwError "count: got {repr outcome}"
  let (_, outcome) ← run "count" #[wrapped #[entry 1 3], .integer 2] held
  unless outcome == .returned #[.integer 0] do
    throwError "count: got {repr outcome}"
