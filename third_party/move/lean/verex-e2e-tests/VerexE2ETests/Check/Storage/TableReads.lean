-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-! Read-only Table roles are explicit intrinsic-contract assumptions over a
table's owned contents, including through caller-side contracts, a nested
table, and a variant's field of a value read from the map. -/

verex module 0x42::table_reads where
  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address

  native fun host_borrow {K} {V}(t : &Table<K, V>, k : K) -> &V
  native fun host_contains {K} {V}(t : &Table<K, V>, k : K) -> Bool

  @[map_borrow (Table)]
  public fun borrow {K} {V}(t : &Table<K, V>, k : K) -> &V := host_borrow(t, k)
  @[map_has_key (Table)]
  public fun contains {K} {V}(t : &Table<K, V>, k : K) -> Bool := host_contains(t, k)

  @[map_spec_has_key (Table)]
  opaque spec fun has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_spec_get (Table)]
  opaque spec fun get {K} {V}(t : Table<K, V>, k : K) : V

  public fun membership(t : &Table<u64, u64>, k : u64) -> Bool := contains(t, k)
  spec membership where
    aborts_if false
    ensures result == has(t, k)
  verify membership

  public fun lookup(t : &Table<u64, u64>, k : u64) -> u64 := *borrow(t, k)
  spec lookup where
    pragma opaque
    aborts_if !has(t, k)
    ensures result == get(t, k)
  verify lookup

  public fun caller(t : &Table<u64, u64>, k : u64) -> u64 := lookup(t, k)
  spec caller where
    requires has(t, k)
    aborts_if false
    ensures result == old(get(t, k))
  verify caller

  public fun lookup_generic {K has Copy, Drop} {V has Copy, Drop}
      (t : &Table<K, V>, k : K) -> V := *borrow(t, k)
  spec lookup_generic where
    aborts_if !has(t, k)
    ensures result == get(t, k)
  verify lookup_generic

  public fun nested(t : &Table<u64, Table<u64, u64> >, outer : u64, inner : u64) -> u64 :=
    *borrow(borrow(t, outer), inner)
  -- A table of tables: a type argument holding a table.
  spec nested where
    requires has(t, outer)
    requires has(get(t, outer), inner)
    aborts_if false
    ensures result == get(get(t, outer), inner)
  verify nested

  struct Transaction has Copy, Drop, Store where
    payload : Vector<Vector<u8> >
    sequence : u64

  -- A borrowed aggregate and the specification's snapshot must project to
  -- the same value, without assuming any invariant about its vector length.
  public fun payload(t : &Table<u64, Transaction>, k : u64) -> Vector<Vector<u8> > :=
    borrow(t, k).payload
  spec payload where
    aborts_if !has(t, k)
    ensures result == get(t, k).payload
    ensures result.length == get(t, k).payload.length
  verify payload

  enum Link {T has Store} has Store where
    | Occupied (value : T)
    | Vacant (next : u64)

  -- A variant's field of a value read from the map.
  public fun occupied(t : &Table<u64, Link<u64> >, k : u64) -> u64 := borrow(t, k).value
  spec occupied where
    aborts_if !has(t, k)
    aborts_if !(get(t, k) is Occupied)
    ensures result == get(t, k).value
  verify occupied

  public fun wrong_payload_bound(t : &Table<u64, Transaction>, k : u64) -> Vector<Vector<u8> > :=
    borrow(t, k).payload
  spec wrong_payload_bound where
    pragma verify = false
    requires has(t, k)
    aborts_if false
    ensures result.length <= 1

  public fun wrong_lookup(t : &Table<u64, u64>, k : u64) -> u64 := *borrow(t, k)
  spec wrong_lookup where
    pragma verify = false
    requires has(t, k)
    aborts_if false
    ensures result != get(t, k)

/--
error: the specification clause `ensures result != get(t, k)` is not established
---
error: verex verification failed: the automatic verification of `wrong_lookup` failed; provide a proof in the module:
  verify wrong_lookup by
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::table_reads::wrong_lookup

/--
error: the specification clause `ensures result.length <= 1` is not established
---
error: verex verification failed: the automatic verification of `wrong_payload_bound` failed; provide a proof in the module:
  verify wrong_payload_bound by
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::table_reads::wrong_payload_bound

-- The read roles run in the interpreter on the table's value, its entries in
-- the owner's last field (`designs/intrinsic-maps.md`, "Tables in the runtime
-- semantics").
open Lean Elab Command VerexIR VerexIR.SemanticOperations VerexE2ETests.CheckSupport in
run_cmd do
  let some unit := VerexSL.registeredUnit? (← getEnv) `«0x42».table_reads
    | throwError "missing table reads module"
  let executable ← match Validation.prepareExecution #[VerexIR.Move.semantics] unit with
    | .ok executable => pure executable
    | .error diagnostics => throwError "preparation failed: {repr diagnostics}"
  let table : RuntimeValue :=
    .nominal ⟨⟨0⟩, 0⟩ none #[.address "t", .vector #[.tuple #[.integer 7, .integer 70]]]
  let initial : RuntimeState := { tableHandles := #["t"] }
  let run (function : String) (arguments : Array RuntimeValue) : CommandElabM Outcome := do
    let some (namespaceIndex, _, index, _) := VerexSL.Contract.findFunction? unit function
      | throwError "missing function {function}"
    match Interpreter.run executable 256 ⟨⟨namespaceIndex⟩, ⟨index⟩⟩ arguments initial with
    | .error failure => throwError "{function} failed: {repr failure}"
    | .ok (_, outcome) => pure outcome.value
  let check (function : String) (arguments : Array RuntimeValue) (expected : Outcome) :
      CommandElabM Unit := do
    let outcome ← run function arguments
    unless outcome == expected do
      throwError "{function}: expected {repr expected}, got {repr outcome}"
  check "membership" #[table, .integer 7] (.returned #[.bool true])
  check "membership" #[table, .integer 8] (.returned #[.bool false])
  check "lookup" #[table, .integer 7] (.returned #[.integer 70])
  check "lookup" #[table, .integer 8] (.threw .abort #[.integer 25863])
  check "caller" #[table, .integer 7] (.returned #[.integer 70])
