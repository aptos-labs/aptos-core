-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.Rendered.Support

/-!
# `table_stored_resource` as a frontend renders it

The module below is the rendering of
`VerexE2ETests/SourceVerify/table_stored_resource.move` that the
source-verification driver writes to `.verex/table_stored_resource.move.lean`,
its `verex sources at` command dropped; the diagnostics are those of
`table_stored_resource.exp`. Neither is edited here:
`scripts/prover-smoke-renderings.sh --write` writes this file again, and
without `--write` tells whether it is what the frontend renders.
-/

/-!
A unit declaring ghost state, proved over every family because it routes
generic callees: a stored table's validity holds where it is read and written,
an `[abstract]` clause is what a caller sees of an opaque callee, and the memory
after an allocation reads as before it, with its stored invariants.
-/

/--
error: the specification clause `ensures result` is not established
---
error: verex verification failed: the automatic verification of `read_flag_false` failed; provide a proof in `VerexE2ETests/SourceVerify/table_stored_resource.proof.lean`:
  verify read_flag_false by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
---
error: the specification clause `ensures result` is not established
---
error: verex verification failed: the automatic verification of `read_false` failed; provide a proof in `VerexE2ETests/SourceVerify/table_stored_resource.proof.lean`:
  verify read_false by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
---
error: the specification clause `aborts_if false` is not established
---
error: verex verification failed: the automatic verification of `publish_false` failed; provide a proof in `VerexE2ETests/SourceVerify/table_stored_resource.proof.lean`:
  verify publish_false by lean =>
    verex_obligations
    verex_on
    | aborts_if_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_rendering in
verex module 0x1::table_stored_resource where
  pragma proof_file = "VerexE2ETests/SourceVerify/table_stored_resource.proof.lean"

  -- Copyright © Aptos Foundation
  -- SPDX-License-Identifier: Apache-2.0
  @[intrinsic_map]
  struct Table {K : phantom type has Copy, Drop} {V : phantom type} has Store where
    handle : Address

  spec Table where
    pragma intrinsic = map

  native fun new_table_handle {K} {V}() -> Address

  @[map_new (Table)]
  public fun new {K has Copy, Drop} {V}() -> Table<K, V> :=
    new Table<K, V> { handle := new_table_handle::<K, V>() }

  spec new where
    pragma intrinsic

  native fun add_entry {K has Copy, Drop} {V}(
    self : &mut Table<K, V>, key : K, val : V
  ) -> Unit

  @[map_add_no_override (Table)]
  public fun add {K has Copy, Drop} {V}(
    self : &mut Table<K, V>, key : K, val : V
  ) -> Unit := do
    self.add_entry(key, val)

  spec add where
    pragma intrinsic

  native fun has_entry {K has Copy, Drop} {V}(
    self : &Table<K, V>, key : K
  ) -> Bool

  @[map_has_key (Table)]
  public fun contains {K has Copy, Drop} {V}(
    self : &Table<K, V>, key : K
  ) -> Bool := self.has_entry(key)

  spec contains where
    pragma intrinsic

  @[map_spec_has_key (Table)]
  opaque spec fun spec_contains {K} {V}(t : Table<K, V>, k : K) : Bool

  @[map_spec_set (Table)]
  opaque spec fun spec_set {K} {V}(t : Table<K, V>, k : K, v : V) : Table<K, V>

  @[map_spec_get (Table)]
  opaque spec fun spec_get {K} {V}(t : Table<K, V>, k : K) : V

  @[map_spec_del (Table)]
  opaque spec fun spec_remove {K} {V}(t : Table<K, V>, k : K) : Table<K, V>

  struct Holder has Key where
    entries : Table<Address, u64>

  struct «Ghost$draws» has Copy, Drop, Store, Key where
    v : u64

  spec «Ghost$draws» where
    pragma ghost_memory

  fun flag() -> Bool := true

  spec flag where
    pragma opaque
    aborts_if [abstract] false
    ensures [abstract] result == spec_flag()

  opaque spec fun spec_flag() : Bool

  fun read_flag() -> Bool := flag()

  spec read_flag where
    aborts_if false
    ensures result == spec_flag()

  fun read_flag_false() -> Bool := flag()

  spec read_flag_false where
    aborts_if false
    ensures result

  -- error: only the opaque specification function is known
  fun read(key : Address) -> Bool := do
    let entries := &Holder[@0x1].entries
    entries.contains(key)

  spec read where
    aborts_if !exists<Holder>(@0x1)
    ensures result == spec_contains(global<Holder>(@0x1).entries, key)

  fun read_false(key : Address) -> Bool := do
    let entries := &Holder[@0x1].entries
    entries.contains(key)

  spec read_false where
    aborts_if !exists<Holder>(@0x1)
    ensures result

  -- error: the key may be absent
  fun write(key : Address, value : u64) -> Unit := do
    let entries := &mut Holder[@0x1].entries
    entries.add(key, value)

  spec write where
    aborts_if !exists<Holder>(@0x1)
    aborts_if spec_contains(global<Holder>(@0x1).entries, key)
    ensures spec_contains(global<Holder>(@0x1).entries, key)
    modifies global<Holder>(@0x1), *

  fun publish(account : &Signer) -> Unit := do
    move_to<Holder>(account, new Holder { entries := new::<Address, u64>() })

  spec publish where
    aborts_if exists<Holder>(account.address)
    ensures exists<Holder>(account.address)
    modifies global<Holder>(account.address), *

  fun publish_false(account : &Signer) -> Unit := do
    move_to<Holder>(account, new Holder { entries := new::<Address, u64>() })

  spec publish_false where
    aborts_if false
    modifies global<Holder>(account.address), *

  -- error: the address may hold one already
