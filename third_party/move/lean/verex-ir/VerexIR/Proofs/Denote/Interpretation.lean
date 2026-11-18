-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Types

/-!
# Carrier and state interpretations

The local checking operation depends on a carrier family and a state type,
not on a total transport to runtime values. The shared term interpreter in
`Term` extends this interface with explicit external operations. A given frame
alone does not transport a generic theorem to runtime; that requires explicit
relational evidence.
-/

namespace VerexIR.Proofs.Denote

/-- The carrier family and state of a typed computation. -/
structure Interpretation where
  carriers : Carriers
  State : Type

/-- A computation at an interpretation, before choosing a runtime store. -/
abbrev Interpretation.Comp (interpretation : Interpretation) (α : Type) :=
  Spec interpretation.State Failure α

/-- Checking needs no runtime-frame bijection. Declared invariants and
parameter predicates are still supplied by the chosen carrier family. -/
def Interpretation.checking (interpretation : Interpretation)
    (τ : NTy) (value : @NTy.carrier (interpretation.carriers).toSorts τ) :
    interpretation.Comp PUnit :=
  Spec.certified (Invariant := @NTy.Holds interpretation.carriers τ value) (fun _ => ())

/-- The existing concrete denotation's interpretation. Its state remains
certified runtime memory; its family retains every concrete invariant. -/
@[reducible] def Interpretation.concrete (unit : Validation.ValidatedUnit)
    [FunctionCarriers unit] [Θ : Skolems unit] : Interpretation where
  carriers := Θ.toCarriers
  State := Memory unit

/-- The concrete adapter is exactly the concrete checking computation. -/
theorem Interpretation.checking_concrete {unit : Validation.ValidatedUnit}
    [FunctionCarriers unit] [Skolems unit] (τ : NTy) (value : τ.carrier) :
    (Interpretation.concrete unit).checking τ value =
      (Spec.certified (Invariant := NTy.Holds τ value) (fun _ => ()) :
        VerexIR.Proofs.Denote.Comp unit PUnit) := rfl

/-- A given parameter is one uninterpreted sort. A generic body neither opens
nor constructs a parameter value. Concrete parameter views and nominal ADT
invariants are separate and are related to this frame explicitly. -/
structure GivenFrame where
  operations : Carriers

/-- Specifications observe a given parameter at its one abstract sort. The
runtime read-back keeps the supplied interpretation, but no second typed sort
is introduced into generic clauses. -/
@[reducible] def GivenFrame.parameterReads (frame : GivenFrame) (index : Nat) :
    TypedReads (frame.operations.carrier index) (frame.operations.codec index).encode where
  typed := frame.operations.carrier index
  read := id
  decEq := frame.operations.decEq index
  inhabited := ⟨(frame.operations.reads index).unread
    (frame.operations.reads index).inhabited.default⟩
  write := (frame.operations.codec index).encode
  back := fun raw => (frame.operations.reads index).unread
    ((frame.operations.reads index).back raw)
  back_write := fun value => by
    rw [← (frame.operations.reads index).write_read value,
      (frame.operations.reads index).back_write,
      (frame.operations.reads index).unread_read]
  back_unit := congrArg (frame.operations.reads index).unread
    (frame.operations.reads index).back_unit
  write_read := fun _ => rfl
  unread := id
  unread_read := fun _ => rfl
  order := fun rank => by
    letI : Carriers := frame.operations
    exact { compare := (Sorts.orders index).order rank,
            lawful := inferInstance, trans := inferInstance }
  order_encode := fun rank left right =>
    ((frame.operations.orders index).order_encode rank left right).symm

/-- Choose the canonical representation of each given parameter. While the
Given route is parked (`U2`), parameter well-formedness in the frame stays
`True`; the relational transfer boundary (`RelationalTransfer.lean`,
`GivenTransfer.lean`) still requires explicit `validArgs`/`validResult`
evidence before exporting a concrete theorem. -/
@[reducible] def GivenFrame.toCarriers (frame : GivenFrame) : Carriers :=
  { frame.operations with
    reads := frame.parameterReads
    holds := fun _ _ => True }

/-- A given interpretation can choose its state independently of runtime
memory. Relating that state to concrete memory is separate work. -/
@[reducible] def GivenFrame.interpretation (frame : GivenFrame) (state : Type) : Interpretation where
  carriers := frame.toCarriers
  State := state

/-- Parameter well-formedness has no obligation in a parked given frame. -/
theorem GivenFrame.deepHolds_param (frame : GivenFrame) (index : Nat)
    (value : @NTy.carrier (frame.toCarriers).toSorts (.param index)) :
    @NTy.deepHolds frame.toCarriers (.param index) value = True := rfl

@[simp] theorem GivenFrame.holds_param (frame : GivenFrame) (index : Nat)
    (value : @NTy.carrier (frame.toCarriers).toSorts (.param index)) :
    @NTy.Holds frame.toCarriers (.param index) value :=
  letI : Carriers := frame.toCarriers
  ⟨trivial⟩

/-- Choosing given parameter sorts does not erase an ADT's declared invariant. -/
theorem GivenFrame.own_eq (frame : GivenFrame) (source : StructHandle)
    (arguments : NRow) (value : RuntimeValue) :
    @Carriers.own frame.toCarriers source arguments value =
      frame.operations.own source arguments value := rfl

/-- A parameter check asks for no parameter invariant in a parked given frame. -/
theorem GivenFrame.checking_param (frame : GivenFrame) (state : Type) (index : Nat)
    (value : @NTy.carrier (frame.toCarriers).toSorts (.param index)) :
    (frame.interpretation state).checking (.param index) value =
      Spec.pure () := by
  letI : Carriers := frame.toCarriers
  simp only [Interpretation.checking, Spec.certified, Spec.pure, Spec.mk.injEq]
  refine ⟨?_, trivial, ?_⟩
  · funext initial result final
    exact propext ⟨fun ⟨_, result_eq, final_eq⟩ => ⟨result_eq, final_eq⟩,
      fun ⟨result_eq, final_eq⟩ => ⟨⟨trivial⟩, result_eq, final_eq⟩⟩
  · funext _
    exact propext ⟨fun fails => fails ⟨trivial⟩, False.elim⟩

end VerexIR.Proofs.Denote
