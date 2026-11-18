-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexIR.Tests.OwnBridgeMap
import VerexIR.Tests.OwnBridgeTable

/-!
# The bridge of a generated own invariant

The sorted map of `VerexIR.Tests.OwnBridgeMap` and the table owner of
`VerexIR.Tests.OwnBridgeTable` as source modules. Each unit's generated clause
(`dataOwn.case0`, at the base function values) is the installed clause the
test states by hand; so the bridge proved there is the bridge of the generated
constant, and of the declaration's fact at the unit's own function values. A
valid `Map<K, V>` of a function generic in `K` and `V` is one whose keys
ascend in the caller's family's order, read over the caller's own value, and
whose entries are valid. A valid `Table<K, V>` is one whose own table's map,
keyed by the typed read of `K`, is valid under the sequence discipline, and
whose fields are valid: the generated clause reads the keys by the induced
frame's typed read, the caller's side by the caller's.
-/

verex module 0x42::own_bridge_declared where
  struct Entry {K has Copy, Drop, Store} {V has Copy, Drop, Store} has Copy, Drop, Store where
    key : K
    value : V

  struct Map {K has Copy, Drop, Store} {V has Copy, Drop, Store} has Copy, Drop, Store where
    entries : Vector<Entry<K, V> >
  spec Map where
    invariant forall (i : Int), forall (j : Int), 0 <= i && i < j && j < entries.length ==>
      core.prim.compare(entries[i].key, entries[j].key) < 0

  fun size {K has Copy, Drop, Store} {V has Copy, Drop, Store}(map : &Map<K, V>) -> u64 :=
    map.entries.length
  spec size where
    aborts_if false

verex module 0x42::own_bridge_owner where
  pragma verify = false
  @[intrinsic_map]
  struct Table {K} {V} has Store where
    handle : Address
  native fun host_borrow {K} {V}(t : &Table<K, V>, k : K) -> &V
  @[map_borrow (Table)]
  public fun borrow {K} {V}(t : &Table<K, V>, k : K) -> &V := host_borrow(t, k)
  @[map_spec_has_key (Table)]
  opaque spec fun has {K} {V}(t : Table<K, V>, k : K) : Bool
  @[map_spec_get (Table)]
  opaque spec fun get {K} {V}(t : Table<K, V>, k : K) : V

  public fun lookup_generic {K has Copy, Drop} {V has Copy, Drop}
      (t : &Table<K, V>, k : K) -> V := *borrow(t, k)
  spec lookup_generic where
    requires has(t, k)
    aborts_if false
    ensures result == get(t, k)

verify 0x42::own_bridge_owner::lookup_generic

namespace VerexIR.Tests.OwnBridgeDeclared

open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote VerexIR.Tests.OwnBridgeMap

/-- The generated clause is the installed clause of the hand-stated
declaration, at the ranks the unit names. -/
theorem generated_installed :
    ∃ rank : ValueRanks, ∀ (θ : TypeArgs)
      (value : @NTy.carrier
        ((@Skolems.instantiate «0x42».own_bridge_declared.unit
          «0x42».own_bridge_declared.functionCarriersBase θ
          (@Skolems.runtime «0x42».own_bridge_declared.unit
            «0x42».own_bridge_declared.functionCarriersBase)).toCarriers).toSorts
        (map (.param 0) (.param 1))),
      «0x42».own_bridge_declared.dataOwn.case0 θ value ↔
        installed «0x42».own_bridge_declared.functionCarriersBase rank θ value := by
  refine ⟨?rank, fun θ value => ?same⟩
  case same =>
    unfold «0x42».own_bridge_declared.dataOwn.case0 installed Obligation
    exact Iff.rfl

/-- The unit's function values install the generated clause as the map's
`own`. -/
theorem installedOwn (arguments : NRow) (value : RuntimeValue) :
    @FunctionCarriers.own «0x42».own_bridge_declared.unit
        «0x42».own_bridge_declared.functionCarriers ⟨⟨0⟩, 1⟩ arguments value =
      OwnAtArguments «0x42».own_bridge_declared.unit
        «0x42».own_bridge_declared.functionCarriersBase (map (.param 0) (.param 1))
        «0x42».own_bridge_declared.dataOwn.case0 arguments value := by
  rw [«0x42».own_bridge_declared.ownEq]
  unfold «0x42».own_bridge_declared.dataOwn
  exact if_pos rfl

/-- The bridge of the generated clause over the map's fact, at the unit's own
function values, for every frame of a function generic in `K` and `V`. -/
theorem declared :
    ∃ rank : ValueRanks,
      ∀ (Θ : @Skolems «0x42».own_bridge_declared.unit «0x42».own_bridge_declared.functionCarriers)
        (_ : ((NRow.cons (.param 0) (.cons (.param 1) .nil)).map Θ.resolve).inhabitable = true)
        (value : @NTy.carrier (Θ.toCarriers).toSorts (map (.param 0) (.param 1))),
        @NTy.Holds Θ.toCarriers (map (.param 0) (.param 1)) value ↔
          caller Θ rank value ∧
            @NTy.Holds Θ.toCarriers (.vector (entry (.param 0) (.param 1))) value.fst := by
  obtain ⟨rank, same⟩ := generated_installed
  refine ⟨rank, fun Θ inhabitable value => ?_⟩
  have opened := NTy.holds_struct_bridge
    (functions := «0x42».own_bridge_declared.functionCarriers)
    (base := «0x42».own_bridge_declared.functionCarriersBase) rfl HEq.rfl rfl Θ
    (.cons (.param 0) (.cons (.param 1) .nil)) inhabitable (source := ⟨⟨0⟩, 1⟩)
    (arguments := .cons (.param 0) (.cons (.param 1) .nil))
    (fields := .cons (.vector (entry (.param 0) (.param 1))) .nil) rfl rfl
    «0x42».own_bridge_declared.dataOwn.case0 installedOwn (caller Θ rank) value
    ((same _ _).trans
      (agreeAt (functions := «0x42».own_bridge_declared.functionCarriers)
        (base := «0x42».own_bridge_declared.functionCarriersBase) rfl HEq.rfl rfl Θ rank
        (.param 0) (.param 1) inhabitable rfl value))
  refine opened.trans (and_congr_right fun _ => ?_)
  rw [@HList.holds_cons Θ.toCarriers, @HList.holds_nil Θ.toCarriers, and_true]
  exact Iff.rfl

/-- The table owner's generated clause is the installed clause of the
hand-stated owner: its keys read by the induced frame's typed read. -/
theorem generated_installed_owner (θ : TypeArgs)
    (value : @NTy.carrier
      ((@Skolems.instantiate «0x42».own_bridge_owner.unit
        «0x42».own_bridge_owner.functionCarriersBase θ
        (@Skolems.runtime «0x42».own_bridge_owner.unit
          «0x42».own_bridge_owner.functionCarriersBase)).toCarriers).toSorts
      (OwnBridgeTable.owner (.param 0) (.param 1))) :
    «0x42».own_bridge_owner.dataOwn.case0 θ value ↔
      OwnBridgeTable.installedRead «0x42».own_bridge_owner.functionCarriersBase θ value := by
  unfold «0x42».own_bridge_owner.dataOwn.case0 OwnBridgeTable.installedRead Obligation
  exact Iff.rfl

/-- The unit's function values install the generated clause as the owner's
`own`. -/
theorem installedOwnOwner (arguments : NRow) (value : RuntimeValue) :
    @FunctionCarriers.own «0x42».own_bridge_owner.unit
        «0x42».own_bridge_owner.functionCarriers ⟨⟨0⟩, 0⟩ arguments value =
      OwnAtArguments «0x42».own_bridge_owner.unit
        «0x42».own_bridge_owner.functionCarriersBase (OwnBridgeTable.owner (.param 0) (.param 1))
        «0x42».own_bridge_owner.dataOwn.case0 arguments value := by
  rw [«0x42».own_bridge_owner.ownEq]
  unfold «0x42».own_bridge_owner.dataOwn
  exact if_pos rfl

/-- The bridge of the generated table clause over what the caller's frame
states of the owner, for every frame of a function generic in `K` and `V`:
the installed validity of the encoding's map, keyed by the induced frame's
typed read, is the validity of the caller's own table's map under the caller's
typed read of `K`. -/
theorem declaredOwner
    (Θ : @Skolems «0x42».own_bridge_owner.unit «0x42».own_bridge_owner.functionCarriers)
    (inhabitable :
      ((NRow.cons (.param 0) (.cons (.param 1) .nil)).map Θ.resolve).inhabitable = true)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (OwnBridgeTable.owner (.param 0) (.param 1))) :
    @Carriers.own Θ.toCarriers ⟨⟨0⟩, 0⟩ (.cons (.param 0) (.cons (.param 1) .nil))
        (@NTy.encode (Θ.toCarriers).toSorts (OwnBridgeTable.owner (.param 0) (.param 1)) value) ↔
      OwnBridgeTable.caller Θ value :=
  Carriers.own_bridge (functions := «0x42».own_bridge_owner.functionCarriers)
    (base := «0x42».own_bridge_owner.functionCarriersBase) rfl HEq.rfl rfl Θ
    (.cons (.param 0) (.cons (.param 1) .nil)) inhabitable
    (σ := OwnBridgeTable.owner (.param 0) (.param 1)) rfl
    «0x42».own_bridge_owner.dataOwn.case0 installedOwnOwner (OwnBridgeTable.caller Θ) value
    ((generated_installed_owner _ _).trans
      (OwnBridgeTable.agreeReadAt (functions := «0x42».own_bridge_owner.functionCarriers)
        (base := «0x42».own_bridge_owner.functionCarriersBase) rfl HEq.rfl rfl Θ
        (.param 0) (.param 1) inhabitable value))

open Lean Elab Command in
run_cmd do
  for name in [``generated_installed, ``installedOwn, ``declared, ``generated_installed_owner,
      ``installedOwnOwner, ``declaredOwner] do
    let axioms ← collectAxioms name
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {name}: {axioms}"

end VerexIR.Tests.OwnBridgeDeclared
