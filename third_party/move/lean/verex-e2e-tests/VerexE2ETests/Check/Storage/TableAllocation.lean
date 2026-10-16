-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-! A table is allocated at a fresh handle and filled through its roles: the
caller reads the contents it built, and its frame leaves the allocation
history open while keeping every other resource
(`designs/intrinsic-maps.md`, "Owned Table values"). -/

verex module 0x42::table_allocation where
  pragma verify = false
  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address
  native fun host_new {K} {V}() -> Table<K, V>
  native fun host_add {K} {V}(t : &mut Table<K, V>, k : K, v : V) -> Unit
  native fun host_remove {K} {V}(t : &mut Table<K, V>, k : K) -> V
  @[map_new (Table)]
  public fun new {K} {V}() -> Table<K, V> := host_new()
  @[map_add_no_override (Table)]
  public fun add {K} {V}(t : &mut Table<K, V>, k : K, v : V) -> Unit := host_add(t, k, v)
  @[map_del_must_exist (Table)]
  public fun remove {K} {V}(t : &mut Table<K, V>, k : K) -> V := host_remove(t, k)
  @[map_spec_has_key (Table)]
  opaque spec fun has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_spec_get (Table)]
  opaque spec fun get {K} {V}(t : Table<K, V>, k : K) : V
  @[map_spec_len (Table)]
  opaque spec fun size {K} {V}(t : Table<K, V>) : Int

  public fun filled() -> Table<u8, u64> := do
    let mut t := new::<u8, u64>()
    add(&mut t, 1u8, 3u64)
    add(&mut t, 2u8, 4u64)
    t
  spec filled where
    ensures has(result, 1u8) && has(result, 2u8)
    ensures get(result, 1u8) == 3
    ensures get(result, 2u8) == 4
    ensures size(result) == 2
  verify filled

  public fun removed() -> Table<u8, u64> := do
    let mut t := filled()
    remove(&mut t, 1u8)
    t
  spec removed where
    ensures !has(result, 1u8) && has(result, 2u8)
    ensures size(result) == 1
  verify removed

  public fun duplicate(k1 : u8, k2 : u8) -> Table<u8, u64> := do
    let mut t := new::<u8, u64>()
    add(&mut t, k1, 2u64)
    add(&mut t, k2, 3u64)
    t
  spec duplicate where
    aborts_if k1 == k2
  verify duplicate

  public fun nested_values(x : u8) -> Table<u8, Vector<u8> > := do
    let mut t := new::<u8, Vector<u8> >()
    add(&mut t, 1u8, vector<u8>[x & 1])
    t
  spec nested_values where
    aborts_if false
    ensures get(result, 1u8) == vector<u8>[x & 1]
  verify nested_values

  -- A table held by a struct, keyed by a struct: the owner's twin bridges
  -- the entries through the key's.
  struct Key has Copy, Drop where
    v : Vector<u8>
  struct Holder has Store where
    t : Table<Key, u64>
  public fun held() -> Holder := do
    let mut t := new::<Key, u64>()
    add(&mut t, new Key { v := vector<u8>[1u8, 2u8] }, 22u64)
    new Holder { t := t }
  spec held where
    ensures size(result.t) == 1
    ensures get(result.t, new Key { v := vector<u8>[1u8, 2u8] }) == 22
  verify held

  public fun wrong_value() -> Table<u8, u64> := do
    let mut t := new::<u8, u64>()
    add(&mut t, 1u8, 3u64)
    t
  spec wrong_value where
    ensures get(result, 1u8) == 4

/--
error: the specification clause `ensures get(result, 1u8) == 4` is not established
---
error: verex verification failed: the automatic verification of `wrong_value` failed; provide a proof: `verify wrong_value by …` in the module (`verify wrong_value by verex_obligations` lists its obligations)
-/
#guard_msgs in
verify 0x42::table_allocation::wrong_value

-- The roles run in the interpreter: the runtime provides them over native
-- storage, allocating at a fresh handle and keeping the entries at the slot
-- of the owner's type and handle (`designs/intrinsic-maps.md`, "Tables in
-- the runtime semantics").
open Lean Elab Command VerexIR VerexE2ETests.CheckSupport in
run_cmd do
  let some unit := VerexSL.registeredUnit? (← getEnv) `«0x42».table_allocation
    | throwError "missing table allocation module"
  let executable ← match Validation.prepareExecution #[VerexIR.Move.semantics] unit with
    | .ok executable => pure executable
    | .error diagnostics => throwError "preparation failed: {repr diagnostics}"
  let run (function : String) (arguments : Array RuntimeValue) :
      CommandElabM (RuntimeState × Outcome) := do
    let some (namespaceIndex, _, index, _) := VerexSL.Contract.findFunction? unit function
      | throwError "missing function {function}"
    match Interpreter.run executable 256 ⟨⟨namespaceIndex⟩, ⟨index⟩⟩ arguments {} with
    | .error failure => throwError "{function} failed: {repr failure}"
    | .ok (state, outcome) => pure (state, outcome.value)
  let table (handle : String) : RuntimeValue := .nominal ⟨⟨0⟩, 0⟩ none #[.address handle]
  let entries (final : RuntimeState) (handle : String) : Option RuntimeValue :=
    (final.tables.contents.entries.find? (·.key.key == .address handle)).map (·.value)
  let check (function : String) (arguments : Array RuntimeValue) (expected : Outcome)
      (contents : Option RuntimeValue) (allocated : Array String := #["table-0"]) :
      CommandElabM Unit := do
    let (final, outcome) ← run function arguments
    unless outcome == expected do
      throwError "{function}: expected {repr expected}, got {repr outcome}"
    unless entries final "table-0" == contents do
      throwError "{function}: contents {repr final.tables.contents}"
    unless final.tables.allocated == allocated do
      throwError "{function}: history {repr final.tables.allocated}"
  check "filled" #[] (.returned #[table "table-0"])
    (some (.vector #[.tuple #[.integer 1, .integer 3], .tuple #[.integer 2, .integer 4]]))
  check "removed" #[] (.returned #[table "table-0"])
    (some (.vector #[.tuple #[.integer 2, .integer 4]]))
  check "duplicate" #[.integer 1, .integer 2] (.returned #[table "table-0"])
    (some (.vector #[.tuple #[.integer 1, .integer 2], .tuple #[.integer 2, .integer 3]]))
  -- An abort rolls the state back, the allocation included.
  check "duplicate" #[.integer 1, .integer 1] (.threw .abort #[.integer 100]) none #[]
  check "nested_values" #[.integer 3] (.returned #[table "table-0"])
    (some (.vector #[.tuple #[.integer 1, .vector #[.integer 1]]]))
  check "held" #[] (.returned #[.nominal ⟨⟨0⟩, 2⟩ none #[table "table-0"]])
    (some (.vector #[.tuple #[.nominal ⟨⟨0⟩, 1⟩ none #[.vector #[.integer 1, .integer 2]],
      .integer 22]]))
  check "wrong_value" #[] (.returned #[table "table-0"])
    (some (.vector #[.tuple #[.integer 1, .integer 3]]))

