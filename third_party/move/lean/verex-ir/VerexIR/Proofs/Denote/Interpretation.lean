-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Types

/-!
# Carrier and state interpretations

The local certifying operation depends on a carrier family and a state type,
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

/-- Certification needs no runtime-frame bijection. Declared invariants and
structural certificates are still supplied by the chosen carrier family. -/
noncomputable def Interpretation.certifying (interpretation : Interpretation)
    (τ : NTy) (value : @NTy.plain interpretation.carriers τ) :
    interpretation.Comp (@NTy.carrier interpretation.carriers τ) :=
  Spec.certified (Invariant := @NTy.deepHolds interpretation.carriers τ value)
    (@NTy.ofDeepHolds interpretation.carriers τ value)

/-- The existing concrete denotation's interpretation. Its state remains
certified runtime memory; its family retains every concrete invariant. -/
@[reducible] def Interpretation.concrete (unit : Validation.ValidatedUnit)
    [FunctionCarriers unit] [Θ : Skolems unit] : Interpretation where
  carriers := Θ.toCarriers
  State := Memory unit

/-- The concrete adapter is exactly the existing certifying computation. -/
theorem Interpretation.certifying_concrete {unit : Validation.ValidatedUnit}
    [FunctionCarriers unit] [Skolems unit] (τ : NTy) (value : τ.plain) :
    (Interpretation.concrete unit).certifying τ value =
      (Spec.certified (Invariant := NTy.deepHolds τ value)
        (NTy.ofDeepHolds τ value) : VerexIR.Proofs.Denote.Comp unit τ.carrier) := rfl

/-- A given parameter is one uninterpreted sort. The plain and certified
API roles use this same representation because a generic body neither opens
nor constructs a parameter value. Concrete parameter views and nominal ADT
certificates are separate and are related to this frame explicitly. -/
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
    exact { compare := (Carriers.orders index).order rank,
            lawful := inferInstance, trans := inferInstance }

/-- Choose the canonical representation of each given parameter. The source
operations' potentially invariant-bearing lent layer is not inherited. -/
@[reducible] def GivenFrame.toCarriers (frame : GivenFrame) : Carriers :=
  { frame.operations with
    reads := frame.parameterReads
    lent := fun index => {
      carrier := frame.operations.carrier index
      codec := frame.operations.codec index
      decEq := frame.operations.decEq index
      inhabited := frame.operations.inhabited index
      tight := frame.operations.tight index
      plain := frame.operations.plain index
      orders := frame.operations.orders index
      erasures := frame.operations.erasures index
      of := id
      encode_of := fun _ => rfl
      holds := fun _ => True
      holds_of := fun _ => trivial
      certify := fun value _ => value
      of_certify := fun _ _ => rfl
      certify_of := fun _ => rfl }
    plainRead := fun index => ⟨id, fun value => by
      change value = (frame.operations.reads index).unread
        ((frame.operations.reads index).back ((frame.operations.codec index).encode value))
      rw [← (frame.operations.reads index).write_read value,
        (frame.operations.reads index).back_write,
        (frame.operations.reads index).unread_read]⟩ }

/-- A given interpretation can choose its state independently of runtime
memory. Relating that state to concrete memory is separate work. -/
@[reducible] def GivenFrame.interpretation (frame : GivenFrame) (state : Type) : Interpretation where
  carriers := frame.toCarriers
  State := state

/-- Parameter well-formedness has no obligation in a given frame. -/
theorem GivenFrame.deepHolds_param (frame : GivenFrame) (index : Nat)
    (value : @NTy.plain frame.toCarriers (.param index)) :
    @NTy.deepHolds frame.toCarriers (.param index) value = True := rfl

/-- Choosing given parameter sorts does not erase an ADT's declared invariant. -/
theorem GivenFrame.own_eq (frame : GivenFrame) (source : StructHandle)
    (arguments : NRow) (value : RuntimeValue) :
    @Carriers.own frame.toCarriers source arguments value =
      frame.operations.own source arguments value := rfl

/-- A parameter cut preserves the canonical abstract value and asks for no
parameter invariant. -/
theorem GivenFrame.certifying_param (frame : GivenFrame) (state : Type) (index : Nat)
    (value : @NTy.plain frame.toCarriers (.param index)) :
    (frame.interpretation state).certifying (.param index) value =
      Spec.pure value := by
  simp only [Interpretation.certifying, Spec.certified, Spec.pure, Spec.mk.injEq]
  refine ⟨?_, trivial, ?_⟩
  · funext initial result final
    exact propext ⟨fun ⟨_, result_eq, final_eq⟩ => ⟨result_eq, final_eq⟩,
      fun ⟨result_eq, final_eq⟩ => ⟨trivial, result_eq, final_eq⟩⟩
  · funext _
    exact propext ⟨fun fails => fails trivial, False.elim⟩

end VerexIR.Proofs.Denote
