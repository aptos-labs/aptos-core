-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.OwnBridge.Bridge

/-!
# Controls of the own-invariant bridge

The operator laws at the shapes a leaf meets, each as the statement a clause
uses: an order at a type parameter, a field of a nested generic struct, a
guarded read under two binders. Then the negatives: a right-hand side chosen
without agreement has no bridge; a read without a bound agrees only under a
premise about the two frames' fallbacks, which no law provides; a declaration's
clause does not certify a value whose field is not valid; and validity read at
the base function values is not validity at the unit's.
-/

namespace VerexIR.Tests.OwnBridgeControls

open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

/-- The descriptor of `Entry<K> { key : K }`. -/
def entry (κ : NTy) : NTy := .struct ⟨⟨0⟩, 0⟩ (.cons κ .nil) (.cons κ .nil)

/-- The descriptor of `Nested<K> { first : Entry<K>, second : Entry<K> }`. -/
def nested (κ : NTy) : NTy :=
  .struct ⟨⟨0⟩, 1⟩ (.cons κ .nil) (.cons (entry κ) (.cons (entry κ) .nil))

/-- The descriptor of `Keys<K> { keys : Vector<K> }`. -/
def keys (κ : NTy) : NTy := .struct ⟨⟨0⟩, 2⟩ (.cons κ .nil) (.cons (.vector κ) .nil)

/-- The descriptor of `Box<K> { content : K }`. -/
def box (κ : NTy) : NTy := .struct ⟨⟨0⟩, 3⟩ (.cons κ .nil) (.cons κ .nil)

section Shapes
variable {unit : Validation.ValidatedUnit} {functions base : FunctionCarriers unit}
  (sameCarrier : functions.carrier = base.carrier)
  (sameFunctions : HEq functions.functions base.functions)
  (sameNominals : functions.nominals = base.nominals)
  (Θ : @Skolems unit functions)
  (inhabitable : ((NRow.cons (.param 0) .nil).map Θ.resolve).inhabitable = true)

/-- An order at a type parameter, with the same ranks: the declaration's
reading over moved values is the caller's family's order of the values, the
form every reader of the caller's leaf uses. -/
theorem order_parameter (rank : ValueRanks) (left right : @NTy.carrier Θ.toCarriers (.param 0))
    (outcome : Ordering) :
    @NTy.order (Skolems.atArguments unit base ((NRow.cons (.param 0) .nil).map Θ.resolve)
          inhabitable).toCarriers rank (.param 0)
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ (.cons (.param 0) .nil)
          inhabitable (τ := .param 0) rfl left)
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ (.cons (.param 0) .nil)
          inhabitable (τ := .param 0) rfl right) = outcome ↔
      (@Carriers.orders Θ.toCarriers 0).order rank left right = outcome := by
  rw [Skolems.order_reframeAtInstance sameCarrier sameFunctions sameNominals Θ
    (.cons (.param 0) .nil) inhabitable (τ := .param 0) rfl rank left right]
  exact Iff.rfl

/-- Equality at a type parameter, likewise: the caller's family's equality of
the values. -/
theorem equal_parameter (left right : @NTy.carrier Θ.toCarriers (.param 0)) (outcome : Bool) :
    @NTy.equal (Skolems.atArguments unit base ((NRow.cons (.param 0) .nil).map Θ.resolve)
          inhabitable).toCarriers false (.param 0)
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ (.cons (.param 0) .nil)
          inhabitable (τ := .param 0) rfl left)
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ (.cons (.param 0) .nil)
          inhabitable (τ := .param 0) rfl right) = outcome ↔
      (@Carriers.erasures Θ.toCarriers 0).same left right = outcome := by
  rw [Skolems.equal_reframeAtInstance sameCarrier sameFunctions sameNominals Θ
    (.cons (.param 0) .nil) inhabitable (τ := .param 0) rfl left right]
  exact Iff.rfl

/-- A field of a field of a nested generic struct: the key of the first entry
of a moved value is the moved key, through two descriptors. -/
theorem field_nested (value : @NTy.carrier Θ.toCarriers (nested (.param 0))) :
    (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ (.cons (.param 0) .nil)
        inhabitable (τ := nested (.param 0)) rfl value).fst.fst =
      Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ (.cons (.param 0) .nil)
        inhabitable (τ := .param 0) rfl value.fst.fst := by
  have outer := Skolems.reframeAtInstance_struct_fst sameCarrier sameFunctions sameNominals Θ
    (.cons (.param 0) .nil) inhabitable (source := ⟨⟨0⟩, 1⟩) (arguments := .cons (.param 0) .nil)
    (τ := entry (.param 0)) (rest := .cons (entry (.param 0)) .nil) rfl value
  have inner := Skolems.reframeAtInstance_struct_fst sameCarrier sameFunctions sameNominals Θ
    (.cons (.param 0) .nil) inhabitable (source := ⟨⟨0⟩, 0⟩) (arguments := .cons (.param 0) .nil)
    (τ := .param 0) (rest := .nil) rfl value.fst
  exact (congrArg Prod.fst outer).trans inner

/-- A guarded read under two binders: the quantified order over the keys of a
moved value is the quantified order over the keys of the value, with each
side's own fallback and no move left under the binders. -/
theorem quantified_reads (rank : ValueRanks)
    (value : @NTy.carrier Θ.toCarriers (keys (.param 0)))
    (fallback : @NTy.carrier (Skolems.atArguments unit base
      ((NRow.cons (.param 0) .nil).map Θ.resolve) inhabitable).toCarriers (.param 0))
    (fallback' : @NTy.carrier Θ.toCarriers (.param 0)) :
    (∀ i j : Int, 0 ≤ i → i < j →
        j < ↑(Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ
          (.cons (.param 0) .nil) inhabitable (τ := keys (.param 0)) rfl value).fst.values.size →
        @NTy.order (Skolems.atArguments unit base ((NRow.cons (.param 0) .nil).map Θ.resolve)
            inhabitable).toCarriers rank (.param 0)
          ((Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ
            (.cons (.param 0) .nil) inhabitable (τ := keys (.param 0)) rfl
            value).fst.values[i.toNat]?.getD fallback)
          ((Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ
            (.cons (.param 0) .nil) inhabitable (τ := keys (.param 0)) rfl
            value).fst.values[j.toNat]?.getD fallback) = .lt) ↔
      ∀ i j : Int, 0 ≤ i → i < j → j < ↑value.fst.values.size →
        (@Carriers.orders Θ.toCarriers 0).order rank
          (value.fst.values[i.toNat]?.getD fallback')
          (value.fst.values[j.toNat]?.getD fallback') = .lt := by
  have field :
      (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ (.cons (.param 0) .nil)
          inhabitable (τ := keys (.param 0)) rfl value).fst =
        Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ (.cons (.param 0) .nil)
          inhabitable (τ := .vector (.param 0)) rfl value.fst :=
    Skolems.reframeAtInstance_struct_fst sameCarrier sameFunctions sameNominals Θ
      (.cons (.param 0) .nil) inhabitable (source := ⟨⟨0⟩, 2⟩) (arguments := .cons (.param 0) .nil)
      (τ := .vector (.param 0)) (rest := .nil) rfl value
  have size := Skolems.reframeAtInstance_vector_size sameCarrier sameFunctions sameNominals Θ
    (.cons (.param 0) .nil) inhabitable (element := .param 0) rfl value.fst
  have read := fun (index : Int) (within : index.toNat < value.fst.values.size) =>
    Skolems.reframeAtInstance_vector_read sameCarrier sameFunctions sameNominals Θ
      (.cons (.param 0) .nil) inhabitable (element := .param 0) rfl value.fst index.toNat within
      fallback fallback'
  have order := Skolems.order_reframeAtInstance sameCarrier sameFunctions sameNominals Θ
    (.cons (.param 0) .nil) inhabitable (τ := .param 0) rfl rank
  rw [field, size]
  refine forall_congr' fun i => forall_congr' fun j => imp_congr_right fun low =>
    imp_congr_right fun ascending => imp_congr_right fun high => ?_
  rw [read i (by omega), read j (by omega), order]
  exact Iff.rfl

/-! ## A read without a bound -/

/-- A read of a moved vector at any index is the moved read where the caller's
fallback moves to the declaration's. -/
theorem read_of_fallback {element : NTy} (bounded)
    (vector : @NTy.carrier Θ.toCarriers (.vector (element.subst (.cons (.param 0) .nil))))
    (index : Nat)
    (fallback : @NTy.carrier (Skolems.atArguments unit base
      ((NRow.cons (.param 0) .nil).map Θ.resolve) inhabitable).toCarriers element)
    (fallback' : @NTy.carrier Θ.toCarriers (element.subst (.cons (.param 0) .nil)))
    (moves : Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ
      (.cons (.param 0) .nil) inhabitable (τ := element)
      (by simpa only [NTy.paramsAll] using bounded) fallback' = fallback) :
    (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ (.cons (.param 0) .nil)
        inhabitable (τ := .vector element) bounded vector).values[index]?.getD fallback =
      Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ (.cons (.param 0) .nil)
        inhabitable (τ := element) (by simpa only [NTy.paramsAll] using bounded)
        (vector.values[index]?.getD fallback') := by
  rw [Skolems.reframeAtInstance_vector_read?]
  cases vector.values[index]? with
  | none => exact moves.symm
  | some _ => rfl

/-- A clause that reads the first key with no bound on the vector's size
agrees across the move under the premise that the fallback moves (`moves`):
the premise is sufficient. That is all this control shows. It is not a
countermodel: it does not refute an agreement of the two fallbacks, and where
the element's carrier does not read the frame, or the clause's own context
bounds the read, the agreement is proved without the premise. -/
theorem unbounded_read (rank : ValueRanks)
    (value : @NTy.carrier Θ.toCarriers (keys (.param 0)))
    (probe : @NTy.carrier Θ.toCarriers (.param 0))
    (moves : Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ
        (.cons (.param 0) .nil) inhabitable (τ := .param 0) rfl
        (@NTy.plainFallback Θ.toCarriers (.param 0) rfl) =
      @NTy.plainFallback (Skolems.atArguments unit base
        ((NRow.cons (.param 0) .nil).map Θ.resolve) inhabitable).toCarriers (.param 0) rfl) :
    @NTy.order (Skolems.atArguments unit base ((NRow.cons (.param 0) .nil).map Θ.resolve)
          inhabitable).toCarriers rank (.param 0)
        ((Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ
          (.cons (.param 0) .nil) inhabitable (τ := keys (.param 0)) rfl
          value).fst.values[0]?.getD
            (@NTy.plainFallback (Skolems.atArguments unit base
              ((NRow.cons (.param 0) .nil).map Θ.resolve) inhabitable).toCarriers (.param 0) rfl))
        (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ
          (.cons (.param 0) .nil) inhabitable (τ := .param 0) rfl probe) = .lt ↔
      (@Carriers.orders Θ.toCarriers 0).order rank
        (value.fst.values[0]?.getD (@NTy.plainFallback Θ.toCarriers (.param 0) rfl)) probe =
          .lt := by
  have field :
      (Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ (.cons (.param 0) .nil)
          inhabitable (τ := keys (.param 0)) rfl value).fst =
        Skolems.reframeAtInstance sameCarrier sameFunctions sameNominals Θ (.cons (.param 0) .nil)
          inhabitable (τ := .vector (.param 0)) rfl value.fst :=
    Skolems.reframeAtInstance_struct_fst sameCarrier sameFunctions sameNominals Θ
      (.cons (.param 0) .nil) inhabitable (source := ⟨⟨0⟩, 2⟩) (arguments := .cons (.param 0) .nil)
      (τ := .vector (.param 0)) (rest := .nil) rfl value
  -- The guarded law needs a bound on the index, and the clause states none.
  fail_if_success
    have := Skolems.reframeAtInstance_vector_read sameCarrier sameFunctions sameNominals Θ
      (.cons (.param 0) .nil) inhabitable (element := .param 0) rfl value.fst 0 (by omega)
      (@NTy.plainFallback (Skolems.atArguments unit base
        ((NRow.cons (.param 0) .nil).map Θ.resolve) inhabitable).toCarriers (.param 0) rfl)
      (@NTy.plainFallback Θ.toCarriers (.param 0) rfl)
  rw [field, read_of_fallback sameCarrier sameFunctions sameNominals Θ inhabitable
      (element := .param 0) rfl value.fst 0 _ _ moves,
    Skolems.order_reframeAtInstance sameCarrier sameFunctions sameNominals Θ
      (.cons (.param 0) .nil) inhabitable (τ := .param 0) rfl rank]
  exact Iff.rfl

/-! ## A right-hand side without agreement -/

/-- The equivalence does not hold of a right-hand side chosen independently of
the installed clauses: with the agreement premise dropped, the statement of
the bridge is refuted by a pair of clauses that disagree. -/
theorem no_bridge_without_agreement (value : @NTy.carrier Θ.toCarriers (box (.param 0))) :
    ¬(OwnAtArguments unit base (box (.param 0)) (fun _ _ => True)
        ((NRow.cons (.param 0) .nil).map Θ.resolve)
        (@NTy.encode Θ.toCarriers (box (.param 0)) value) ↔ False) :=
  fun bridge => bridge.mp fun _ _ _ => trivial

end Shapes

/-! ## Validity is not the clause -/

/-- The ground family with no valid value of a type parameter. -/
@[reducible] def invalidParameters : Carriers :=
  { Carriers.ground with holds := fun _ _ => False }

/-- A declaration's clause does not certify a value whose field is not valid:
at a family whose type parameter has no valid value, the clause of a box holds
of every box and no box is valid. Certifying a constructed value therefore
owes the fields' validity beside the clause, which is what the bridge's
statement of a struct's fact lists (`NTy.holds_struct_bridge`). -/
theorem clause_does_not_certify (value : @NTy.carrier invalidParameters (box (.param 0))) :
    @Carriers.own invalidParameters ⟨⟨0⟩, 3⟩ (.cons (.param 0) .nil)
        (@NTy.encode invalidParameters (box (.param 0)) value) ∧
      ¬@NTy.Holds invalidParameters (box (.param 0)) value := by
  refine ⟨trivial, fun valid => ?_⟩
  have opened := (@NTy.holds_struct invalidParameters ⟨⟨0⟩, 3⟩ (.cons (.param 0) .nil)
    (.cons (.param 0) .nil) value).mp valid
  rw [@HList.holds_cons invalidParameters, @NTy.holds_param invalidParameters] at opened
  exact opened.2.1

/-- The ground family with a declaration's invariant that no value satisfies,
and the same family at the base function values, where the invariant is not
read. -/
@[reducible] def strictOwn : Carriers :=
  { Carriers.ground with own := fun _ _ _ => False }

/-- Validity at the base function values is not validity at the unit's: a box
is valid where no declaration's invariant is read and invalid where the
declaration's is. A binder of an installed clause restricted to the valid
values of a declared type therefore ranges over more values than the caller's,
and only its opening half transfers
(`Skolems.forall_holds_reframeAtInstance_mp`). -/
theorem validity_reads_own (value : @NTy.carrier Carriers.ground (box (.param 0))) :
    @NTy.Holds Carriers.ground (box (.param 0)) value ∧
      ¬@NTy.Holds strictOwn (box (.param 0)) value := by
  constructor
  · refine (@NTy.holds_struct Carriers.ground ⟨⟨0⟩, 3⟩ (.cons (.param 0) .nil)
      (.cons (.param 0) .nil) value).mpr ⟨trivial, ?_⟩
    rw [@HList.holds_cons Carriers.ground, @NTy.holds_param Carriers.ground]
    exact ⟨trivial, trivial⟩
  · intro valid
    exact ((@NTy.holds_struct strictOwn ⟨⟨0⟩, 3⟩ (.cons (.param 0) .nil)
      (.cons (.param 0) .nil) value).mp valid).1

open Lean Elab Command in
run_cmd do
  for name in [``order_parameter, ``equal_parameter, ``field_nested, ``quantified_reads, ``read_of_fallback,
      ``unbounded_read, ``no_bridge_without_agreement, ``clause_does_not_certify,
      ``validity_reads_own,
      -- The library's laws and statements.
      ``Skolems.reframeAtInstance_inj, ``Skolems.reframeAtInstance_surjective,
      ``Skolems.order_reframeAtInstance, ``Skolems.equal_reframeAtInstance,
      ``Skolems.reframeAtInstance_struct_fst, ``Skolems.reframeAtInstance_struct_snd,
      ``Skolems.reframeAtInstance_tuple_fst, ``Skolems.reframeAtInstance_tuple_snd,
      ``Skolems.reframeAtInstance_enum_inl, ``Skolems.reframeAtInstance_enum_inr,
      ``Skolems.reframeAtInstance_ghost, ``Skolems.reframeAtInstance_of_encode,
      ``Skolems.reframeAtInstance_bool, ``Skolems.reframeAtInstance_int,
      ``Skolems.reframeAtInstance_address,
      ``Skolems.reframeAtInstance_vector_values, ``Skolems.reframeAtInstance_vector_size,
      ``Skolems.reframeAtInstance_vector_read?, ``Skolems.reframeAtInstance_vector_read,
      ``Skolems.reframeAtInstance_vector_read_of_fallback, ``VerexIR.SpecInt.val_injective,
      ``Skolems.reframeAtInstance_table_entries,
      ``Skolems.valid_sequence_reframeAtInstance_of_agree,
      ``Skolems.valid_sequence_reframeAtInstance, ``Skolems.valid_sequence_reframeAtInstance_read,
      ``Skolems.valid_ordered_reframeAtInstance, ``Skolems.valid_ordered_reframeAtInstance_keyOrder,
      ``OwnAtArguments.bridge, ``Carriers.own_bridge, ``NTy.holds_struct_bridge,
      ``NTy.holds_enum_bridge,
      ``Skolems.forall_reframeAtInstance, ``Skolems.exists_reframeAtInstance,
      ``Skolems.forall_mem_reframeAtInstance, ``Skolems.exists_mem_reframeAtInstance,
      ``Skolems.forall_holds_reframeAtInstance, ``Skolems.forall_holds_reframeAtInstance_mp,
      ``VerexIR.Proofs.Obligation.congr] do
    let axioms ← collectAxioms name
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {name}: {axioms}"

end VerexIR.Tests.OwnBridgeControls
