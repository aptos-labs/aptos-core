-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.OwnBridge.Bridge

/-!
# The bridge of a table owner's own invariant

A generic declaration that owns a table, `Owner<K, V> { handle : address,
entries : Table<K, V> }`, whose own clause is the validity of its entries'
typed map under its discipline. A unit installs the clause at the frame the
type arguments induce, the keys read by that frame's typed read of the key
parameter (`installedRead`); the same clause with the identity key reader and
the map's type arguments spelled at the runtime family (`installed`) is a form
the laws cover alike. A caller's leaf reads the map of its own table with its
own typed key view: the typed read at a type parameter, an integer's value at
an integer key.

The bridge connects the installed clause, as it is, to the leaf's: for the
sequence discipline by the injectivity of both key readers, for an ordered
discipline by the agreement of the two key orders across the move.
-/

namespace VerexIR.Tests.OwnBridgeTable

open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

/-- The descriptor of `Owner<K, V>`. -/
def owner (κ ν : NTy) : NTy :=
  .struct ⟨⟨0⟩, 0⟩ (.cons κ (.cons ν .nil)) (.cons .address (.cons (.table κ ν) .nil))

/-- The sequence owner's clause with the identity key reader, the map's type
arguments at the runtime family's reading of the row, the entries at the frame
the row induces. -/
def installed {unit : Validation.ValidatedUnit} (base : FunctionCarriers unit) (θ : TypeArgs)
    (value : @NTy.carrier ((@Skolems.instantiate unit base θ (@Skolems.runtime unit base)).toCarriers).toSorts
      (owner (.param 0) (.param 1))) : Prop :=
  Obligation "owner" 100 200
    (@SpecMap.Valid
      (@NTy.carrier ((@Carriers.runtime unit base)).toSorts (θ.1.getD 0 (.param 0)).unmark)
      (@NTy.carrier ((@Carriers.runtime unit base)).toSorts (θ.1.getD 1 (.param 1)).unmark)
      .sequence
      (SpecMap.ofEntries (fun key => key) (fun value => value) value.snd.fst))

/-- The sequence owner's clause as a unit installs it: the map's keys read by
the induced frame's typed read of the key parameter, its types and its entries
at that one frame. -/
def installedRead {unit : Validation.ValidatedUnit} (base : FunctionCarriers unit) (θ : TypeArgs)
    (value : @NTy.carrier ((@Skolems.instantiate unit base θ (@Skolems.runtime unit base)).toCarriers).toSorts
      (owner (.param 0) (.param 1))) : Prop :=
  Obligation "owner" 100 200
    (SpecMap.Valid .sequence
      (SpecMap.ofEntries
        (@NTy.read ((@Skolems.instantiate unit base θ (@Skolems.runtime unit base)).toCarriers).toSorts
          (.param 0))
        (fun value => value) value.snd.fst))

/-- The clause at a caller's frame at a row with the entries `κ` and `ν`: the
caller's typed key view of its own table. -/
def callerAt {unit : Validation.ValidatedUnit} {functions : FunctionCarriers unit}
    (Θ : @Skolems unit functions) (κ ν : NTy)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (owner κ.unmark ν.unmark)) : Prop :=
  Obligation "owner" 100 200
    (SpecMap.Valid .sequence
      (SpecMap.ofEntries (@NTy.read (Θ.toCarriers).toSorts κ.unmark) (fun value => value) value.snd.fst))

/-- The clause as a leaf of a function generic in `K` and `V` reads it. -/
def caller {unit : Validation.ValidatedUnit} {functions : FunctionCarriers unit}
    (Θ : @Skolems unit functions)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (owner (.param 0) (.param 1))) : Prop :=
  Obligation "owner" 100 200
    (SpecMap.Valid .sequence
      (SpecMap.ofEntries (@NTy.read (Θ.toCarriers).toSorts (.param 0)) (fun value => value)
        value.snd.fst))

/-- The clause as a leaf reads it at the row `[u64, T]`. -/
def callerInteger {unit : Validation.ValidatedUnit} {functions : FunctionCarriers unit}
    (Θ : @Skolems unit functions)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (owner (.int 64 false) (.param 0))) : Prop :=
  Obligation "owner" 100 200
    (SpecMap.Valid .sequence
      (SpecMap.ofEntries SpecInt.val (fun value => value) value.snd.fst))

/-- An ordered owner's clause with the identity key reader under the
structural order of the keys. -/
def installedOrdered {unit : Validation.ValidatedUnit} (base : FunctionCarriers unit)
    (rank : ValueRanks) (θ : TypeArgs)
    (value : @NTy.carrier ((@Skolems.instantiate unit base θ (@Skolems.runtime unit base)).toCarriers).toSorts
      (owner (.param 0) (.param 1))) : Prop :=
  Obligation "owner" 100 200
    (SpecMap.Valid
      (.ordered (@NTy.keyOrder
        ((@Skolems.instantiate unit base θ (@Skolems.runtime unit base)).toCarriers).toSorts rank (.param 0)))
      (SpecMap.ofEntries (fun key => key) (fun value => value) value.snd.fst))

/-- The ordered clause at a caller's frame, under a key reader and a key order
the caller names. -/
def callerOrderedAt {unit : Validation.ValidatedUnit} {functions : FunctionCarriers unit}
    (Θ : @Skolems unit functions) (κ ν : NTy) {K : Type}
    (readKey : @NTy.carrier (Θ.toCarriers).toSorts κ.unmark → K) (order : SpecMap.KeyOrder K)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (owner κ.unmark ν.unmark)) : Prop :=
  Obligation "owner" 100 200
    (SpecMap.Valid (.ordered order)
      (SpecMap.ofEntries readKey (fun value => value) value.snd.fst))

section
variable {unit : Validation.ValidatedUnit} {functions base : FunctionCarriers unit}
  (sameCarrier : functions.carrier = base.carrier)
  (sameFunctions : HEq functions.functions base.functions)
  (sameNominals : functions.nominals = base.nominals)
  (Θ : @Skolems unit functions)

/-- The table of a moved owner is the moved table. -/
theorem entries_moved (κ ν : NTy)
    (inhabitable : ((NRow.cons κ (.cons ν .nil)).map Θ.resolve).inhabitable = true)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (owner κ.unmark ν.unmark)) :
    (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ
        (.cons κ (.cons ν .nil)) inhabitable (τ := owner (.param 0) (.param 1)) rfl
        value).snd.fst =
      Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ
        (.cons κ (.cons ν .nil)) inhabitable (τ := .table (.param 0) (.param 1)) rfl
        value.snd.fst := by
  have rest := Skolems.reframeAtInstance_struct_snd sameCarrier sameFunctions sameNominals Θ
    (.cons κ (.cons ν .nil)) inhabitable (source := ⟨⟨0⟩, 0⟩)
    (arguments := .cons (.param 0) (.cons (.param 1) .nil)) (τ := .address)
    (rest := .cons (.table (.param 0) (.param 1)) .nil) rfl value
  have field := Skolems.reframeAtInstance_struct_fst sameCarrier sameFunctions sameNominals Θ
    (.cons κ (.cons ν .nil)) inhabitable (source := ⟨⟨0⟩, 0⟩)
    (arguments := .cons (.param 0) (.cons (.param 1) .nil)) (τ := .table (.param 0) (.param 1))
    (rest := .nil) rfl value.snd
  exact (congrArg Prod.fst rest).trans field

/-- The agreement of the installed sequence clause, identity key reader and
all, with the caller's clause under the caller's typed key view. -/
theorem agreeAt (κ ν : NTy)
    (inhabitable : ((NRow.cons κ (.cons ν .nil)).map Θ.resolve).inhabitable = true)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (owner κ.unmark ν.unmark)) :
    installed base ⟨(NRow.cons κ (.cons ν .nil)).map Θ.resolve, inhabitable⟩
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ
          (.cons κ (.cons ν .nil)) inhabitable (τ := owner (.param 0) (.param 1)) rfl value) ↔
      callerAt Θ κ ν value := by
  unfold installed callerAt
  apply Obligation.congr
  have moved := entries_moved sameCarrier sameFunctions sameNominals Θ κ ν inhabitable value
  have valid := Skolems.valid_sequence_reframeAtInstance_read sameCarrier sameFunctions
    sameNominals Θ (.cons κ (.cons ν .nil)) inhabitable (key := .param 0) (value := .param 1) rfl
    value.snd.fst (fun value => value) (fun value => value)
  rw [← moved] at valid
  exact valid

/-- The agreement of the clause a unit installs, keyed by the induced frame's
typed read, with the caller's clause under the caller's typed key view: both
reads are injective. -/
theorem agreeReadAt (κ ν : NTy)
    (inhabitable : ((NRow.cons κ (.cons ν .nil)).map Θ.resolve).inhabitable = true)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (owner κ.unmark ν.unmark)) :
    installedRead base ⟨(NRow.cons κ (.cons ν .nil)).map Θ.resolve, inhabitable⟩
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ
          (.cons κ (.cons ν .nil)) inhabitable (τ := owner (.param 0) (.param 1)) rfl value) ↔
      callerAt Θ κ ν value := by
  unfold installedRead callerAt
  apply Obligation.congr
  have moved := entries_moved sameCarrier sameFunctions sameNominals Θ κ ν inhabitable value
  have valid := Skolems.valid_sequence_reframeAtInstance sameCarrier sameFunctions
    sameNominals Θ (.cons κ (.cons ν .nil)) inhabitable (key := .param 0) (value := .param 1) rfl
    value.snd.fst
    (@NTy.read ((Skolems.atArguments unit base ((NRow.cons κ (.cons ν .nil)).map Θ.resolve)
      inhabitable).toCarriers).toSorts (.param 0))
    (fun value => value) (@NTy.read (Θ.toCarriers).toSorts κ.unmark) (fun value => value)
    (@NativeMaps.NTy.read_injective
      (Skolems.atArguments unit base ((NRow.cons κ (.cons ν .nil)).map Θ.resolve)
        inhabitable).toCarriers.toSorts (.param 0))
    (@NativeMaps.NTy.read_injective Θ.toCarriers.toSorts κ.unmark)
  rw [← moved] at valid
  exact valid

include sameCarrier sameFunctions sameNominals

/-- The sequence owner's bridge, once for the declaration. -/
theorem bridgeAt (κ ν : NTy)
    (inhabitable : ((NRow.cons κ (.cons ν .nil)).map Θ.resolve).inhabitable = true)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (owner κ.unmark ν.unmark)) :
    OwnAtArguments unit base (owner (.param 0) (.param 1)) (installed base)
        ((NRow.cons κ (.cons ν .nil)).map Θ.resolve)
        (@NTy.encode (Θ.toCarriers).toSorts (owner κ.unmark ν.unmark) value) ↔
      callerAt Θ κ ν value :=
  OwnAtArguments.bridge sameCarrier sameFunctions sameNominals Θ (.cons κ (.cons ν .nil))
    inhabitable (σ := owner (.param 0) (.param 1)) rfl (installed base) (callerAt Θ κ ν) value
    (agreeAt sameCarrier sameFunctions sameNominals Θ κ ν inhabitable value)

/-- At the declaration's own parameters: the leaf's map is keyed by the typed
read of its first type parameter, as the leaf's own reads of the table are. -/
theorem bridge
    (inhabitable :
      ((NRow.cons (.param 0) (.cons (.param 1) .nil)).map Θ.resolve).inhabitable = true)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (owner (.param 0) (.param 1))) :
    OwnAtArguments unit base (owner (.param 0) (.param 1)) (installed base)
        ((NRow.cons (.param 0) (.cons (.param 1) .nil)).map Θ.resolve)
        (@NTy.encode (Θ.toCarriers).toSorts (owner (.param 0) (.param 1)) value) ↔
      caller Θ value :=
  bridgeAt sameCarrier sameFunctions sameNominals Θ (.param 0) (.param 1) inhabitable value

/-- At the row `[u64, T]`: the leaf's map is keyed by the integers' values. -/
theorem bridgeInteger
    (inhabitable :
      ((NRow.cons (.int 64 false) (.cons (.param 0) .nil)).map Θ.resolve).inhabitable = true)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (owner (.int 64 false) (.param 0))) :
    OwnAtArguments unit base (owner (.param 0) (.param 1)) (installed base)
        ((NRow.cons (.int 64 false) (.cons (.param 0) .nil)).map Θ.resolve)
        (@NTy.encode (Θ.toCarriers).toSorts (owner (.int 64 false) (.param 0)) value) ↔
      callerInteger Θ value :=
  bridgeAt sameCarrier sameFunctions sameNominals Θ (.int 64 false) (.param 0) inhabitable value

/-- The ordered owner's agreement: the caller's key reader and key order
compare two read keys as the caller's structural order compares the keys
(`reads`). The premise is the caller's side alone; the move is discharged by
the law. -/
theorem agreeOrderedAt (rank : ValueRanks) (κ ν : NTy) {K : Type}
    (readKey : @NTy.carrier (Θ.toCarriers).toSorts κ.unmark → K) (order : SpecMap.KeyOrder K)
    (reads : ∀ a b, order.compare (readKey a) (readKey b) =
      @NTy.order (Θ.toCarriers).toSorts rank κ.unmark a b)
    (inhabitable : ((NRow.cons κ (.cons ν .nil)).map Θ.resolve).inhabitable = true)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (owner κ.unmark ν.unmark)) :
    installedOrdered base rank ⟨(NRow.cons κ (.cons ν .nil)).map Θ.resolve, inhabitable⟩
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ
          (.cons κ (.cons ν .nil)) inhabitable (τ := owner (.param 0) (.param 1)) rfl value) ↔
      callerOrderedAt Θ κ ν readKey order value := by
  unfold installedOrdered callerOrderedAt
  apply Obligation.congr
  have moved := entries_moved sameCarrier sameFunctions sameNominals Θ κ ν inhabitable value
  have valid := Skolems.valid_ordered_reframeAtInstance_keyOrder sameCarrier sameFunctions
    sameNominals Θ (.cons κ (.cons ν .nil)) inhabitable (key := .param 0) (value := .param 1) rfl
    value.snd.fst rank (fun value => value) readKey (fun value => value) order reads
  rw [← moved] at valid
  exact valid

/-- At an integer key the leaf's order of the keys' values is the structural
order: the premise holds by computation. -/
theorem agreeOrderedInteger (rank : ValueRanks)
    (inhabitable :
      ((NRow.cons (.int 64 false) (.cons (.param 0) .nil)).map Θ.resolve).inhabitable = true)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (owner (.int 64 false) (.param 0))) :
    installedOrdered base rank
        ⟨(NRow.cons (.int 64 false) (.cons (.param 0) .nil)).map Θ.resolve, inhabitable⟩
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ
          (.cons (.int 64 false) (.cons (.param 0) .nil)) inhabitable
          (τ := owner (.param 0) (.param 1)) rfl value) ↔
      callerOrderedAt Θ (.int 64 false) (.param 0) SpecInt.val SpecMap.intOrder value :=
  agreeOrderedAt sameCarrier sameFunctions sameNominals Θ rank (.int 64 false) (.param 0)
    SpecInt.val SpecMap.intOrder (fun _ _ => rfl) inhabitable value

/-- At a key the leaf reads as it is, under the structural order of the keys,
the premise holds by computation at every entry of the row. -/
theorem agreeOrderedStructural (rank : ValueRanks) (κ ν : NTy)
    (inhabitable : ((NRow.cons κ (.cons ν .nil)).map Θ.resolve).inhabitable = true)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (owner κ.unmark ν.unmark)) :
    installedOrdered base rank ⟨(NRow.cons κ (.cons ν .nil)).map Θ.resolve, inhabitable⟩
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ
          (.cons κ (.cons ν .nil)) inhabitable (τ := owner (.param 0) (.param 1)) rfl value) ↔
      callerOrderedAt Θ κ ν (fun key => key) (@NTy.keyOrder (Θ.toCarriers).toSorts rank κ.unmark) value :=
  agreeOrderedAt sameCarrier sameFunctions sameNominals Θ rank κ ν (fun key => key)
    (@NTy.keyOrder (Θ.toCarriers).toSorts rank κ.unmark) (fun _ _ => rfl) inhabitable value

/-- At a type parameter's key under the leaf's typed view (the family's read
and the family's order of the typed keys) the premise is that the family
orders two read keys as it orders the keys: stated here as a hypothesis, and
discharged by the family's coherence law in `agreeOrderedParameterCoherent`. -/
theorem agreeOrderedParameter (rank : ValueRanks)
    (readsOrder : ∀ a b : @Sorts.carrier (Θ.toCarriers).toSorts 0,
      ((@Sorts.reads (Θ.toCarriers).toSorts 0).order rank).compare
          ((@Sorts.reads (Θ.toCarriers).toSorts 0).read a) ((@Sorts.reads (Θ.toCarriers).toSorts 0).read b) =
        (@Sorts.orders (Θ.toCarriers).toSorts 0).order rank a b)
    (inhabitable :
      ((NRow.cons (.param 0) (.cons (.param 1) .nil)).map Θ.resolve).inhabitable = true)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (owner (.param 0) (.param 1))) :
    installedOrdered base rank
        ⟨(NRow.cons (.param 0) (.cons (.param 1) .nil)).map Θ.resolve, inhabitable⟩
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ
          (.cons (.param 0) (.cons (.param 1) .nil)) inhabitable
          (τ := owner (.param 0) (.param 1)) rfl value) ↔
      callerOrderedAt Θ (.param 0) (.param 1) (@NTy.read (Θ.toCarriers).toSorts (.param 0))
        (@NTy.readOrder (Θ.toCarriers).toSorts rank (.param 0)) value :=
  agreeOrderedAt sameCarrier sameFunctions sameNominals Θ rank (.param 0) (.param 1)
    (@NTy.read (Θ.toCarriers).toSorts (.param 0)) (@NTy.readOrder (Θ.toCarriers).toSorts rank (.param 0)) readsOrder
    inhabitable value

/-- A family's typed reads compare as its values do
(`NTy.readOrder_read_order`, from `TypedReads.order_encode` and
`TypedOrder.order_encode`), so the ordered owner's bridge holds at a type
parameter's key under the typed view with no premise left. -/
theorem agreeOrderedParameterCoherent (rank : ValueRanks)
    (inhabitable :
      ((NRow.cons (.param 0) (.cons (.param 1) .nil)).map Θ.resolve).inhabitable = true)
    (value : @NTy.carrier (Θ.toCarriers).toSorts (owner (.param 0) (.param 1))) :
    installedOrdered base rank
        ⟨(NRow.cons (.param 0) (.cons (.param 1) .nil)).map Θ.resolve, inhabitable⟩
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ
          (.cons (.param 0) (.cons (.param 1) .nil)) inhabitable
          (τ := owner (.param 0) (.param 1)) rfl value) ↔
      callerOrderedAt Θ (.param 0) (.param 1) (@NTy.read (Θ.toCarriers).toSorts (.param 0))
        (@NTy.readOrder (Θ.toCarriers).toSorts rank (.param 0)) value :=
  agreeOrderedParameter sameCarrier sameFunctions sameNominals Θ rank
    (fun left right => @NTy.readOrder_read_order (Θ.toCarriers).toSorts rank (.param 0) left right)
    inhabitable value

end

open Lean Elab Command in
run_cmd do
  for name in [``entries_moved, ``agreeAt, ``agreeReadAt, ``bridgeAt, ``bridge, ``bridgeInteger,
      ``agreeOrderedAt, ``agreeOrderedInteger, ``agreeOrderedStructural,
      ``agreeOrderedParameter, ``agreeOrderedParameterCoherent] do
    let axioms ← collectAxioms name
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {name}: {axioms}"

-- The output form of the right-hand sides: a caller's clause names no move
-- and no frame but the caller's, while the installed clause names the induced
-- frame at the base function values.
open Lean Elab Command in
run_cmd do
  let foreign := [``Skolems.reframeAtInstance, ``Skolems.reframeAtBase, ``Skolems.reframe,
    ``Skolems.atArguments, ``Skolems.instantiate, ``Skolems.runtime, ``Carriers.runtime,
    ``Carriers.instantiate, ``FunctionCarriers.runtime]
  for name in [``callerAt, ``caller, ``callerInteger, ``callerOrderedAt] do
    let body := (← getConstInfo name).value!
    for constant in foreign do
      if (body.find? (·.isConstOf constant)).isSome then
        throwError "the caller's clause {name} names {constant}"
  let body := (← getConstInfo ``installed).value!
  unless (body.find? (·.isConstOf ``Skolems.instantiate)).isSome do
    throwError "the installed clause does not name the induced frame"

end VerexIR.Tests.OwnBridgeTable
