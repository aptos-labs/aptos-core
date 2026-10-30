-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-! An intrinsic map owner is its abstract map in verification
(`doc/designs/intrinsic-maps.md`, "Abstract representation"): a function of the
owner's module reading its representation has no denotation and is used
through its specification; its own body is not verified. Only the owner's
module can read its representation, by Move's visibility. -/

namespace VerexSL.Tests.Check.Specifications.AbstractOwners

verex module 0x42::abstract_owners where
  struct Entry {K} {V} has Copy, Drop, Store where
    key : K
    value : V

  @[intrinsic_map]
  struct Map {K} {V} has Copy, Drop, Store where
    entries : Vector<Entry<K, V> >

  @[map_spec_get (Map)]
  opaque spec fun get {K} {V}(m : Map<K, V>, k : K) : V
  @[map_spec_set (Map)]
  opaque spec fun set {K} {V}(m : Map<K, V>, k : K, v : V) : Map<K, V>
  @[map_spec_del (Map)]
  opaque spec fun del {K} {V}(m : Map<K, V>, k : K) : Map<K, V>
  @[map_spec_has_key (Map)]
  opaque spec fun has {K} {V}(m : Map<K, V>, k : K) : Bool
  @[map_spec_len (Map)]
  opaque spec fun size {K} {V}(m : Map<K, V>) : Int

  native fun host_add {K} {V}(m : &mut Map<K, V>, k : K, v : V) -> Unit
  @[map_new (Map)]
  public fun new {K} {V}() -> Map<K, V> := new Map<K, V> { entries := vector<Entry<K, V> >[] }
  @[map_add_no_override (Map)]
  public fun add {K} {V}(m : &mut Map<K, V>, k : K, v : V) -> Unit := host_add(m, k, v)

  -- Reads the representation: used through its specification.
  public fun count {K} {V}(m : &Map<K, V>) -> u64 := m.entries.length
  spec count where
    aborts_if false
    ensures result == size(m)

  public fun two() -> u64 := do
    let mut m := new::<u64, u64>()
    add(&mut m, 1u64, 10u64)
    add(&mut m, 2u64, 20u64)
    count(&m)
  spec two where
    aborts_if false
    ensures result == 2
  verify two

  -- The same, under a leaf budget too small for its leaves.
  public fun two_budgeted() -> u64 := do
    let mut m := new::<u64, u64>()
    add(&mut m, 1u64, 10u64)
    add(&mut m, 2u64, 20u64)
    count(&m)
  spec two_budgeted where
    pragma verify = false
    pragma leaf_heartbeats = 100
    aborts_if false
    ensures result == 2

/--
error: `count` reads the representation of the intrinsic map `Map`; a function of an intrinsic map's module reading its representation is used through its specification, and its body is not verified
-/
#guard_msgs in
verify 0x42::abstract_owners::count

-- Stated contract facts close the invariant and call precondition even at
-- this budget; the remaining clauses are reported for the proof file.
/--
error: the specification clause `aborts_if false` is not established
---
error: the specification clause `ensures result == 2` is not established
---
error: verex verification failed: the automatic verification of `two_budgeted` failed; provide a proof in the module:
  verify two_budgeted by
    verex_obligations
    verex_on
    | aborts_if_1 => verex_prepare; sorry
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::abstract_owners::two_budgeted

-- The map laws under the module's specification function names.
/--
info: «0x42».abstract_owners.get_set {K V : Type} [DecidableEq K] [Inhabited V] (discipline : VerexIR.SpecMap.Discipline K)
  (map : VerexIR.SpecMap K V) (key : K) (value : V) (probe : K) :
  (VerexIR.SpecMap.update discipline map key value).valueAt probe = if probe = key then value else map.valueAt probe
-/
#guard_msgs in
#check «0x42».abstract_owners.get_set

end VerexSL.Tests.Check.Specifications.AbstractOwners
