-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-! Read-only Table roles are explicit intrinsic-contract assumptions over a
table's owned contents, including through caller-side contracts. A nested
table is read through a role at a table-bearing type argument, which the
denotation does not carry (`designs/intrinsic-maps.md`, "Stages"). -/

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
  spec nested where
    pragma verify = false
    requires has(t, outer)
    requires has(get(t, outer), inner)
    aborts_if false
    ensures result == get(get(t, outer), inner)

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
error: no denotation for `nested`: a type argument holding a map's contents is not carried by the denotation
-/
#guard_msgs in
verify 0x42::table_reads::nested

/--
error: the specification clause `ensures result != get(t, k)` is not established
---
error: verex verification failed: the automatic verification of `wrong_lookup` failed; provide a proof: `verify wrong_lookup by …` in the module (`verify wrong_lookup by verex_obligations` lists its obligations)
-/
#guard_msgs in
verify 0x42::table_reads::wrong_lookup

/--
error: the specification clause `ensures result.length <= 1` is not established
---
error: verex verification failed: the automatic verification of `wrong_payload_bound` failed; provide a proof: `verify wrong_payload_bound by …` in the module (`verify wrong_payload_bound by verex_obligations` lists its obligations)
-/
#guard_msgs in
verify 0x42::table_reads::wrong_payload_bound

-- The read roles run in the interpreter over a table held in native storage
-- at the slot of its owner's closed type, the one the role resolves at its
-- frame (`designs/intrinsic-maps.md`, "Tables in the runtime semantics").
open Lean Elab Command VerexIR VerexIR.SemanticOperations VerexE2ETests.CheckSupport in
run_cmd do
  let some unit := VerexSL.registeredUnit? (← getEnv) `«0x42».table_reads
    | throwError "missing table reads module"
  let executable ← match Validation.prepareExecution #[VerexIR.Move.semantics] unit with
    | .ok executable => pure executable
    | .error diagnostics => throwError "preparation failed: {repr diagnostics}"
  let ns := unit.namespaces[0]!
  -- The slot of `Table<u64, u64>`: the parameter type of `membership`.
  let some (_, _, membership, declaration) := VerexSL.Contract.findFunction? unit "membership"
    | throwError "missing membership"
  let some parameter := declaration.signature.parameters[0]? | throwError "membership has no parameter"
  let referent := match ns.tables.types[parameter.typeUse.typeId.index]? with
    | some (.reference reference) => reference.referent
    | _ => parameter.typeUse.typeId
  let some typeId := canonicalTypeId? unit referent | throwError "the table type is open"
  let slot := tableSlot ⟨0⟩ typeId "t"
  let table : RuntimeValue := .nominal ⟨⟨0⟩, 0⟩ none #[.address "t"]
  let initial : RuntimeState := { tables := {
    contents := ⟨#[⟨slot, .vector #[.tuple #[.integer 7, .integer 70]]⟩]⟩
    allocated := #["t"] } }
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
  let _ := membership
  check "membership" #[table, .integer 7] (.returned #[.bool true])
  check "membership" #[table, .integer 8] (.returned #[.bool false])
  check "lookup" #[table, .integer 7] (.returned #[.integer 70])
  check "lookup" #[table, .integer 8] (.threw .abort #[.integer 101])
  check "caller" #[table, .integer 7] (.returned #[.integer 70])

