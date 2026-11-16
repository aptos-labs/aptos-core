-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.OwnBridge.Bridge
import VerexIR.Proofs.Denote.Solve.Normal

/-!
# The bridge of a sorted map's own invariant

A generic declaration shaped like an ordered map,
`Map<K, V> { entries : Vector<Entry<K, V>> }` over
`Entry<K, V> { key : K, value : V }`, with the invariant that the keys of its
entries ascend. Its clause is stated twice: as a unit installs it, at the
frame a row of type arguments induces at the base function values, and as a
caller's leaf reads it, at the caller's own frame over the caller's own value.
The bridge between the two is assembled from the operator laws: the field of
the map, the size and the guarded reads of the vector, the field of the nested
generic entry, and the order at the key's type parameter.
-/

namespace VerexIR.Tests.OwnBridgeMap

open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

/-- The descriptor of `Entry<K, V>`. -/
def entry (κ ν : NTy) : NTy :=
  .struct ⟨⟨0⟩, 0⟩ (.cons κ (.cons ν .nil)) (.cons κ (.cons ν .nil))

/-- The descriptor of `Map<K, V>`. -/
def map (κ ν : NTy) : NTy :=
  .struct ⟨⟨0⟩, 1⟩ (.cons κ (.cons ν .nil)) (.cons (.vector (entry κ ν)) .nil)

/-- The declaration's clause as installed: at the frame the type arguments
induce over the runtime frame, at the base function values, over a value of
the declaration at its own parameters. -/
def installed {unit : Validation.ValidatedUnit} (base : FunctionCarriers unit) (rank : ValueRanks)
    (θ : TypeArgs)
    (value : @NTy.carrier (@Skolems.instantiate unit base θ (@Skolems.runtime unit base)).toCarriers
      (map (.param 0) (.param 1))) : Prop :=
  Obligation "map" 400 547
    (∀ i j : Int, (0 ≤ i ∧ i < j) ∧ j < ↑value.fst.values.size →
      orderValue
          (@NTy.order (@Skolems.instantiate unit base θ (@Skolems.runtime unit base)).toCarriers
            rank (.param 0)
            (value.fst.values[i.toNat]?.getD
              (@NTy.plainFallback
                (@Skolems.instantiate unit base θ (@Skolems.runtime unit base)).toCarriers
                (entry (.param 0) (.param 1)) rfl)).fst
            (value.fst.values[j.toNat]?.getD
              (@NTy.plainFallback
                (@Skolems.instantiate unit base θ (@Skolems.runtime unit base)).toCarriers
                (entry (.param 0) (.param 1)) rfl)).fst) <
        0)

/-- The clause at a caller's frame over a caller's value, at an instance row
with the entries `κ` and `ν`: the same source clause, its types the
declaration's at the row. -/
def callerAt {unit : Validation.ValidatedUnit} {functions : FunctionCarriers unit}
    (Θ : @Skolems unit functions) (rank : ValueRanks) (κ ν : NTy)
    (inhabitable : (entry κ.unmark ν.unmark).inhabitable = true)
    (value : @NTy.carrier Θ.toCarriers (map κ.unmark ν.unmark)) : Prop :=
  Obligation "map" 400 547
    (∀ i j : Int, (0 ≤ i ∧ i < j) ∧ j < ↑value.fst.values.size →
      orderValue
          (@NTy.order Θ.toCarriers rank κ.unmark
            (value.fst.values[i.toNat]?.getD
              (@NTy.plainFallback Θ.toCarriers (entry κ.unmark ν.unmark) inhabitable)).fst
            (value.fst.values[j.toNat]?.getD
              (@NTy.plainFallback Θ.toCarriers (entry κ.unmark ν.unmark) inhabitable)).fst) <
        0)

/-- The clause as a leaf of a function generic in `K` and `V` reads it: the
order is the caller's family's order of its first type parameter. -/
def caller {unit : Validation.ValidatedUnit} {functions : FunctionCarriers unit}
    (Θ : @Skolems unit functions) (rank : ValueRanks)
    (value : @NTy.carrier Θ.toCarriers (map (.param 0) (.param 1))) : Prop :=
  Obligation "map" 400 547
    (∀ i j : Int, (0 ≤ i ∧ i < j) ∧ j < ↑value.fst.values.size →
      orderValue
          ((@Carriers.orders Θ.toCarriers 0).order rank
            (value.fst.values[i.toNat]?.getD
              (@NTy.plainFallback Θ.toCarriers (entry (.param 0) (.param 1)) rfl)).fst
            (value.fst.values[j.toNat]?.getD
              (@NTy.plainFallback Θ.toCarriers (entry (.param 0) (.param 1)) rfl)).fst) <
        0)

section
variable {unit : Validation.ValidatedUnit} {functions base : FunctionCarriers unit}
  (sameCarrier : functions.carrier = base.carrier)
  (sameFunctions : HEq functions.functions base.functions)
  (sameNominals : functions.nominals = base.nominals)
  (Θ : @Skolems unit functions)

/-- The agreement of the two readings at a row with symbolic entries. -/
theorem agreeAt (rank : ValueRanks) (κ ν : NTy)
    (inhabitable : ((NRow.cons κ (.cons ν .nil)).map Θ.resolve).inhabitable = true)
    (fallback : (entry κ.unmark ν.unmark).inhabitable = true)
    (value : @NTy.carrier Θ.toCarriers (map κ.unmark ν.unmark)) :
    installed base rank ⟨(NRow.cons κ (.cons ν .nil)).map Θ.resolve, inhabitable⟩
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ
          (.cons κ (.cons ν .nil)) inhabitable (τ := map (.param 0) (.param 1)) rfl
          value) ↔
      callerAt Θ rank κ ν fallback value := by
  unfold installed callerAt
  apply Obligation.congr
  -- The laws, each instantiated at a descriptor by its handle, arguments and
  -- fields and stated at the descriptor's own spelling.
  have field :
      (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ
          (.cons κ (.cons ν .nil)) inhabitable (τ := map (.param 0) (.param 1)) rfl value).fst =
        Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ
          (.cons κ (.cons ν .nil)) inhabitable (τ := .vector (entry (.param 0) (.param 1))) rfl
          value.fst :=
    Skolems.reframeAtInstance_struct_fst sameCarrier sameFunctions sameNominals Θ
      (.cons κ (.cons ν .nil)) inhabitable (source := ⟨⟨0⟩, 1⟩)
      (arguments := .cons (.param 0) (.cons (.param 1) .nil))
      (τ := .vector (entry (.param 0) (.param 1))) (rest := .nil) rfl value
  have size := Skolems.reframeAtInstance_vector_size sameCarrier sameFunctions sameNominals Θ
    (.cons κ (.cons ν .nil)) inhabitable (element := entry (.param 0) (.param 1)) rfl value.fst
  have read := fun (index : Int) (within : index.toNat < value.fst.values.size) =>
    Skolems.reframeAtInstance_vector_read sameCarrier sameFunctions sameNominals Θ
      (.cons κ (.cons ν .nil)) inhabitable (element := entry (.param 0) (.param 1)) rfl
      value.fst index.toNat within
      (@NTy.plainFallback
        (Skolems.atArguments unit base ((NRow.cons κ (.cons ν .nil)).map Θ.resolve)
          inhabitable).toCarriers (entry (.param 0) (.param 1)) rfl)
      (@NTy.plainFallback Θ.toCarriers (entry κ.unmark ν.unmark) fallback)
  have key : ∀ element : @NTy.carrier Θ.toCarriers (entry κ.unmark ν.unmark),
      (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ
          (.cons κ (.cons ν .nil)) inhabitable (τ := entry (.param 0) (.param 1)) rfl
          element).fst =
        Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ
          (.cons κ (.cons ν .nil)) inhabitable (τ := .param 0) rfl element.fst :=
    fun element => Skolems.reframeAtInstance_struct_fst sameCarrier sameFunctions sameNominals Θ
      (.cons κ (.cons ν .nil)) inhabitable (source := ⟨⟨0⟩, 0⟩)
      (arguments := .cons (.param 0) (.cons (.param 1) .nil))
      (τ := .param 0) (rest := .cons (.param 1) .nil) rfl element
  have order := Skolems.order_reframeAtInstance sameCarrier sameFunctions sameNominals Θ
    (.cons κ (.cons ν .nil)) inhabitable (τ := .param 0) rfl rank
  -- The clause, operator by operator.
  refine forall_congr' fun i => forall_congr' fun j => imp_congr_ctx ?_ fun guard => ?_
  · rw [field, size]
    exact Iff.rfl
  · obtain ⟨⟨low, ascending⟩, high⟩ := guard
    rw [field, read i (by omega), read j (by omega), key, key, order]
    exact Iff.rfl

include sameCarrier sameFunctions sameNominals

/-- The bridge, once for the declaration: at a row whose entries are symbolic,
for a symbolic caller frame. -/
theorem bridgeAt (rank : ValueRanks) (κ ν : NTy)
    (inhabitable : ((NRow.cons κ (.cons ν .nil)).map Θ.resolve).inhabitable = true)
    (fallback : (entry κ.unmark ν.unmark).inhabitable = true)
    (value : @NTy.carrier Θ.toCarriers (map κ.unmark ν.unmark)) :
    OwnAtArguments unit base (map (.param 0) (.param 1)) (installed base rank)
        ((NRow.cons κ (.cons ν .nil)).map Θ.resolve)
        (@NTy.encode Θ.toCarriers (map κ.unmark ν.unmark) value) ↔
      callerAt Θ rank κ ν fallback value :=
  OwnAtArguments.bridge sameCarrier sameFunctions sameNominals Θ (.cons κ (.cons ν .nil))
    inhabitable (σ := map (.param 0) (.param 1)) rfl (installed base rank)
    (callerAt Θ rank κ ν fallback) value
    (agreeAt sameCarrier sameFunctions sameNominals Θ rank κ ν inhabitable fallback value)

/-- The bridge at the declaration's own parameters, in the form a leaf of a
function generic in `K` and `V` reads: the instance of the declaration's
bridge at that row, whose right-hand side converts to the leaf's form. -/
theorem bridge (rank : ValueRanks)
    (inhabitable :
      ((NRow.cons (.param 0) (.cons (.param 1) .nil)).map Θ.resolve).inhabitable = true)
    (value : @NTy.carrier Θ.toCarriers (map (.param 0) (.param 1))) :
    OwnAtArguments unit base (map (.param 0) (.param 1)) (installed base rank)
        ((NRow.cons (.param 0) (.cons (.param 1) .nil)).map Θ.resolve)
        (@NTy.encode Θ.toCarriers (map (.param 0) (.param 1)) value) ↔
      caller Θ rank value :=
  bridgeAt sameCarrier sameFunctions sameNominals Θ rank (.param 0) (.param 1) inhabitable rfl
    value

/-- The clause as a leaf reads it at the row `[u64, T]`: the keys are
integers, ordered by their values. -/
def callerInteger {unit : Validation.ValidatedUnit} {functions : FunctionCarriers unit}
    (Θ : @Skolems unit functions)
    (value : @NTy.carrier Θ.toCarriers (map (.int 64 false) (.param 0))) : Prop :=
  Obligation "map" 400 547
    (∀ i j : Int, (0 ≤ i ∧ i < j) ∧ j < ↑value.fst.values.size →
      orderValue
          (compare
            (value.fst.values[i.toNat]?.getD
              (@NTy.plainFallback Θ.toCarriers (entry (.int 64 false) (.param 0)) rfl)).fst.val
            (value.fst.values[j.toNat]?.getD
              (@NTy.plainFallback Θ.toCarriers (entry (.int 64 false) (.param 0)) rfl)).fst.val) <
        0)

/-- The bridge at the row `[u64, T]`: the same instance of the declaration's
bridge, converting to that row's form. -/
theorem bridgeInteger (rank : ValueRanks)
    (inhabitable :
      ((NRow.cons (.int 64 false) (.cons (.param 0) .nil)).map Θ.resolve).inhabitable = true)
    (value : @NTy.carrier Θ.toCarriers (map (.int 64 false) (.param 0))) :
    OwnAtArguments unit base (map (.param 0) (.param 1)) (installed base rank)
        ((NRow.cons (.int 64 false) (.cons (.param 0) .nil)).map Θ.resolve)
        (@NTy.encode Θ.toCarriers (map (.int 64 false) (.param 0)) value) ↔
      callerInteger Θ value :=
  bridgeAt sameCarrier sameFunctions sameNominals Θ rank (.int 64 false) (.param 0) inhabitable
    rfl value

/-- The bridge over the map's fact at its own parameters: a map is valid
exactly where the clause holds at the caller's frame and its entries are
valid. `installedOwn` is the unit's installation of the clause as the
declaration's `own`. -/
theorem holds_bridge (rank : ValueRanks)
    (installedOwn : ∀ arguments value,
      @FunctionCarriers.own unit functions ⟨⟨0⟩, 1⟩ arguments value =
        OwnAtArguments unit base (map (.param 0) (.param 1)) (installed base rank) arguments
          value)
    (inhabitable :
      ((NRow.cons (.param 0) (.cons (.param 1) .nil)).map Θ.resolve).inhabitable = true)
    (value : @NTy.carrier Θ.toCarriers (map (.param 0) (.param 1))) :
    @NTy.Holds Θ.toCarriers (map (.param 0) (.param 1)) value ↔
      caller Θ rank value ∧
        @NTy.Holds Θ.toCarriers (.vector (entry (.param 0) (.param 1))) value.fst := by
  have opened := NTy.holds_struct_bridge sameCarrier sameFunctions sameNominals Θ
    (.cons (.param 0) (.cons (.param 1) .nil)) inhabitable (source := ⟨⟨0⟩, 1⟩)
    (arguments := .cons (.param 0) (.cons (.param 1) .nil))
    (fields := .cons (.vector (entry (.param 0) (.param 1))) .nil) rfl rfl
    (installed base rank) installedOwn (caller Θ rank) value
    (agreeAt sameCarrier sameFunctions sameNominals Θ rank (.param 0) (.param 1) inhabitable rfl
      value)
  refine opened.trans (and_congr_right fun _ => ?_)
  rw [@HList.holds_cons Θ.toCarriers, @HList.holds_nil Θ.toCarriers, and_true]
  exact Iff.rfl

/-- Opening: a valid map's clause, at the caller's frame over the caller's
value. -/
theorem opened (rank : ValueRanks)
    (installedOwn : ∀ arguments value,
      @FunctionCarriers.own unit functions ⟨⟨0⟩, 1⟩ arguments value =
        OwnAtArguments unit base (map (.param 0) (.param 1)) (installed base rank) arguments
          value)
    (inhabitable :
      ((NRow.cons (.param 0) (.cons (.param 1) .nil)).map Θ.resolve).inhabitable = true)
    (value : @NTy.carrier Θ.toCarriers (map (.param 0) (.param 1)))
    (valid : @NTy.Holds Θ.toCarriers (map (.param 0) (.param 1)) value) :
    caller Θ rank value :=
  ((holds_bridge sameCarrier sameFunctions sameNominals Θ rank installedOwn inhabitable
    value).mp valid).1

/-- Certifying: a constructed or written map is valid where its clause holds
at the caller's frame and its entries are valid. Nothing is assumed of the
map's own validity. -/
theorem certified (rank : ValueRanks)
    (installedOwn : ∀ arguments value,
      @FunctionCarriers.own unit functions ⟨⟨0⟩, 1⟩ arguments value =
        OwnAtArguments unit base (map (.param 0) (.param 1)) (installed base rank) arguments
          value)
    (inhabitable :
      ((NRow.cons (.param 0) (.cons (.param 1) .nil)).map Θ.resolve).inhabitable = true)
    (value : @NTy.carrier Θ.toCarriers (map (.param 0) (.param 1)))
    (clause : caller Θ rank value)
    (entries : @NTy.Holds Θ.toCarriers (.vector (entry (.param 0) (.param 1))) value.fst) :
    @NTy.Holds Θ.toCarriers (map (.param 0) (.param 1)) value :=
  (holds_bridge sameCarrier sameFunctions sameNominals Θ rank installedOwn inhabitable
    value).mpr ⟨clause, entries⟩

omit sameCarrier sameFunctions sameNominals in
/-- The leaf's normalization of the opened clause is the form a leaf's readers
hold, the caller's family's order of the keys read from the caller's own
vector, and it is stable: a second pass changes nothing. -/
theorem opened_normal (rank : ValueRanks)
    (value : @NTy.carrier Θ.toCarriers (map (.param 0) (.param 1)))
    (clause : caller Θ rank value) :
    ∀ i j : Int, 0 ≤ i → i < j → j < ↑value.fst.values.size →
      (@Carriers.orders Θ.toCarriers 0).order rank
        (value.fst.values[i.toNat]?.getD
          (@NTy.plainFallback Θ.toCarriers (entry (.param 0) (.param 1)) rfl)).fst
        (value.fst.values[j.toNat]?.getD
          (@NTy.plainFallback Θ.toCarriers (entry (.param 0) (.param 1)) rfl)).fst = .lt := by
  unfold caller Obligation at clause
  simp only [vir_denote_norm] at clause
  fail_if_success (simp only [vir_denote_norm] at clause)
  exact clause

end

open Lean Elab Command in
run_cmd do
  for name in [``agreeAt, ``bridgeAt, ``bridge, ``bridgeInteger, ``holds_bridge, ``opened,
      ``certified, ``opened_normal] do
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
  for name in [``callerAt, ``caller, ``callerInteger] do
    let body := (← getConstInfo name).value!
    for constant in foreign do
      if (body.find? (·.isConstOf constant)).isSome then
        throwError "the caller's clause {name} names {constant}"
  let body := (← getConstInfo ``installed).value!
  unless (body.find? (·.isConstOf ``Skolems.instantiate)).isSome do
    throwError "the installed clause does not name the induced frame"

end VerexIR.Tests.OwnBridgeMap
