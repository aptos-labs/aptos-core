-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

namespace LeanerLang.Tests.Check.Specifications.TableSnapshots

leaner module 0x42::table_snapshots where
  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address

  @[map_spec_has_key (Table)]
  opaque spec fun has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_spec_len (Table)]
  opaque spec fun size {K} {V}(t : Table<K, V>) : Int
  @[map_spec_get (Table)]
  opaque spec fun get {K} {V}(t : Table<K, V>, k : K) : V
  @[map_spec_set (Table)]
  opaque spec fun set {K} {V}(t : Table<K, V>, k : K, v : V) : Table<K, V>

  public fun unchanged(t : &mut Table<u64, u64>, k : u64) -> Unit := ()
  spec unchanged where
    ensures has(old(t), k) ==> size(old(t)) > 0
    ensures has(old(t), k) ==> size(t) > 0

  verify unchanged

  struct Holder has Store where
    table : Table<u64, u64>

  public fun nested_unchanged(t : &mut Holder, k : u64) -> Unit := ()
  spec nested_unchanged where
    ensures has(old(t.table), k) ==> size(t.table) > 0

  verify nested_unchanged

  spec fun nonempty {K} {V}(t : Table<K, V>, k : K) : Bool := has(t, k) ==> size(t) > 0

  public fun labelled(t : &Table<u64, u64>, k : u64) -> Unit := ()
  spec labelled where
    pragma opaque
    aborts_if false
    ensures ∀ (S : StateDomain), S |~ nonempty(t, k)

  verify labelled

  public fun caller(t : &Table<u64, u64>, k : u64) -> Unit := labelled(t, k)
  spec caller where
    ensures ∀ (S : StateDomain), S |~ nonempty(t, k)

  verify caller

  spec fun alternatives(t : Table<u64, u64>, k : u64) : Vector<Table<u64, u64> > :=
    vector<Table<u64, u64> >[t, set(t, k, 7)]

  public fun functional_snapshots(t : &Table<u64, u64>, k : u64) -> Unit := ()
  spec functional_snapshots where
    requires has(t, k)
    requires get(t, k) != 7
    ensures get(alternatives(t, k)[0], k) == get(t, k)
    ensures get(alternatives(t, k)[1], k) == 7
    ensures ∀ (index in range(alternatives(t, k))), 0 <= index && index < 2
    ensures alternatives(t, k)[0] == alternatives(t, k)[1]
    ensures get(alternatives(t, k)[0], k) != get(alternatives(t, k)[1], k)

  verify functional_snapshots

end LeanerLang.Tests.Check.Specifications.TableSnapshots

-- A shared reference has the same native carrier as its referent, but must
-- not give the Table's contents a second runtime storage key.
run_cmd do
  let some unit := LeanerLang.registeredUnit? (← Lean.getEnv) `«0x42».table_snapshots
    | throwError "missing Table snapshot module"
  let mut checked := false
  for n in [:unit.namespaces.size] do
    let ns := unit.namespaces[n]!
    for i in [:ns.tables.types.size] do
      let .reference reference := ns.tables.types[i]! | continue
      unless reference.kind == .shared do continue
      if (LeanerIR.Proofs.Denote.TableMemory.runtimeResourceOf unit ⟨n⟩ reference.referent).isSome then
        checked := true
        unless (LeanerIR.Proofs.Denote.TableMemory.runtimeResourceOf unit ⟨n⟩ ⟨i⟩).isNone do
          throwError "a shared Table reference incorrectly names native storage"
  unless checked do throwError "the Table storage-key regression checked no shared reference"
