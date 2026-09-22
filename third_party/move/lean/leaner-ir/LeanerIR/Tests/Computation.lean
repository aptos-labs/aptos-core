-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.ComputationAgreement
import LeanerIR.Proofs.NativeArithmetic

namespace LeanerIR.Tests.Computation

open LeanerIR.Proofs LeanerIR.SemanticOperations

-- No runtime state, runtime frame, or universal value is needed by a native
-- computation. The same generic proof can be instantiated at any call site.
def carry (value : α) : Spec State Error α := Spec.pure value

def identityContract : Contract State Error α α where
  requires := fun _ _ => True
  ensures := fun value _ result _ => result = value
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False

theorem carry_verified : Satisfies (carry (State := State) (Error := Error))
    (identityContract (α := α)) := by
  intro value initial _
  exact ⟨fun _ _ execution => ⟨fun _ => execution.1, execution.2, id⟩,
    fun _ => False.elim, False.elim⟩

-- Opaque callee and theorem parameters prevent this from accidentally
-- passing by reducing a particular callee body at its caller.
example (callee : Bool → Spec Nat Unit Bool)
    (verified : Satisfies callee identityContract) (value : Bool) (initial : Nat) :
    wp (Spec.bind (callee value) Spec.pure)
      (fun result final => result = value ∧ final = initial) (fun _ => False) initial := by
  rw [wp_bind]
  apply wp_mono (wp_of_satisfies verified trivial (by simp [identityContract]))
  · intro result final established
    exact (wp_pure _ _ _ _).mpr established
  · exact fun _ => id

-- A legal left-inverse codec may accept a noncanonical runtime alias.
-- Decoding such an alias is NOT an exact representation certificate.
def aliasCodec : Codec Bool Nat where
  encode := fun value => if value then 1 else 0
  decode? := fun value => some (value != 0)
  decode_encode := by intro value; cases value <;> rfl

example : aliasCodec.decode? 2 = some true := rfl

-- Tuple locals use a heterogeneous native product. Flattening into the VM's
-- single packed tuple happens only at the codec boundary.
example (value : SpecInt (.bits 64) false) :
    (Codec.tuple
      (Codec.tupleCons (Codec.specInt (.bits 64) false)
        (Codec.tupleCons Codec.bool Codec.tupleNil))).encode
      (value, (true, ())) = .tuple #[.integer value.val, .bool true] := rfl

example (value : SpecInt (.bits 64) false) :
    (Codec.tuple
      (Codec.tupleCons (Codec.specInt (.bits 64) false)
        (Codec.tupleCons Codec.bool Codec.tupleNil))).decode?
      (.tuple #[.integer value.val, .bool true]) = some (value, (true, ())) := by
  let codec := Codec.tuple
    (Codec.tupleCons (Codec.specInt (.bits 64) false)
      (Codec.tupleCons Codec.bool Codec.tupleNil))
  change codec.decode? (codec.encode (value, (true, ()))) = some (value, (true, ()))
  exact codec.decode_encode _

example : ¬Represents (Codec.identity Unit) aliasCodec
    (fun _ => Spec.pure true : Unit → Spec Unit Unit Bool)
    (fun _ => Spec.pure 2) := by
  intro agreement
  obtain ⟨value, ⟨equal, _⟩, encoded⟩ :=
    (agreement ()).ok () 2 () |>.mp ⟨rfl, rfl⟩
  cases equal
  cases encoded

-- The roundtrip applies to arbitrary relations, including failures and
-- undefined executions, not just successful pure functions.
example (action : Spec Nat String Bool) :
    Spec.Equiv (decodeSpec aliasCodec (encodeSpec aliasCodec action)) action :=
  decodeSpec_encodeSpec _ _

example (state : Nat) (message : String) :
    (encodeSpec aliasCodec (Spec.abort message : Spec Nat String Bool)).aborts state message :=
  rfl

example (state : Nat) :
    (decodeSpec aliasCodec (encodeSpec aliasCodec
      ({ ok := fun _ _ _ => False, aborts := fun _ _ => False,
         undefined := fun _ => True } : Spec Nat String Bool))).undefined state :=
  Or.inl trivial

-- Generic move agreement remains valid for a carrier represented by a
-- borrow. It must move the local out, not assume the encoding is borrow-free.
example (loan : Nat) (value : RuntimeValue) :
    ComputationAgreement.Returns
      (Denotation.nativeLocalOperation (.move ⟨⟨0⟩⟩) Denotation.valuesNil)
      { locals := #[some (.borrow loan value)] } { locals := #[none] }
      (.borrow loan value) :=
  ComputationAgreement.move _ _ _ rfl (by simp)

-- Arithmetic also works without the runtime error/state vocabulary. The
-- failed mathematical value is retained exactly, not clamped or wrapped.
example : (NativeArithmetic.checkedInteger (.bits 8) false id 255 :
    Spec Unit Int (SpecInt (.bits 8) false)).ok () ⟨255, by decide⟩ () := by
  simp [NativeArithmetic.checkedInteger, IntegerValueFits, Ty.integerValueFits?,
    Ty.integerBounds?, Spec.pure]

example : (NativeArithmetic.checkedInteger (.bits 8) false id 256 :
    Spec Unit Int (SpecInt (.bits 8) false)).aborts () 256 := by
  simp [NativeArithmetic.checkedInteger, IntegerValueFits, Ty.integerValueFits?,
    Ty.integerBounds?, Spec.abort]

example : ¬(NativeArithmetic.checkedInteger (.bits 8) false id 256 :
    Spec Unit Int (SpecInt (.bits 8) false)).aborts () 0 := by
  simp [NativeArithmetic.checkedInteger, IntegerValueFits, Ty.integerValueFits?,
    Ty.integerBounds?, Spec.abort]

example : (NativeArithmetic.checkedInteger (.bits 8) false id (-1) :
    Spec Unit Int (SpecInt (.bits 8) false)).aborts () (-1) := by
  simp [NativeArithmetic.checkedInteger, IntegerValueFits, Ty.integerValueFits?,
    Ty.integerBounds?, Spec.abort]

example : (NativeArithmetic.checkedInteger (.bits 8) true id (-128) :
    Spec Unit Int (SpecInt (.bits 8) true)).ok () ⟨-128, by decide⟩ () := by
  simp [NativeArithmetic.checkedInteger, IntegerValueFits, Ty.integerValueFits?,
    Ty.integerBounds?, Spec.pure]

example : (NativeArithmetic.checkedInteger (.bits 8) true id 128 :
    Spec Unit Int (SpecInt (.bits 8) true)).aborts () 128 := by
  simp [NativeArithmetic.checkedInteger, IntegerValueFits, Ty.integerValueFits?,
    Ty.integerBounds?, Spec.abort]

example : (NativeArithmetic.checkedInteger (.bits 8) true id (-129) :
    Spec Unit Int (SpecInt (.bits 8) true)).aborts () (-129) := by
  simp [NativeArithmetic.checkedInteger, IntegerValueFits, Ty.integerValueFits?,
    Ty.integerBounds?, Spec.abort]

-- Sequence opaque, potentially aborting native callees. In particular,
-- input 1 passes the first call and fails in the second: the complete call
-- rule must retain that failure without requiring a no-abort premise.
def stepContract : Contract Unit Nat Nat Nat where
  requires := fun _ _ => True
  ensures := fun input _ result _ => result = input + 1
  aborts := fun input _ error => input ≥ 2 ∧ error = input
  mayAbort := fun input _ => input ≥ 2
  mustAbort := fun input _ => input ≥ 2

example (stage : Nat → Spec Unit Nat Nat) (verified : Satisfies stage stepContract)
    (input : Nat) :
    wp (Spec.bind (stage input) stage)
      (fun result _ => result = input + 2)
      (fun error => (input ≥ 2 ∧ error = input) ∨
        (input < 2 ∧ input + 1 ≥ 2 ∧ error = input + 1)) () := by
  rw [wp_bind]
  apply wp_mono (wp_of_satisfies_complete verified input () trivial)
  · intro middle state established
    obtain ⟨post, _, normal⟩ := established
    have equal := post normal
    change middle = input + 1 at equal
    subst middle
    apply wp_mono (wp_of_satisfies_complete verified (input + 1) state trivial)
    · intro result final established
      have equal := established.1 established.2.2
      simpa [stepContract, Nat.add_assoc] using equal
    · intro error aborted
      exact Or.inr ⟨by change ¬ input ≥ 2 at normal; omega, aborted⟩
  · intro error aborted
    exact Or.inl aborted

end LeanerIR.Tests.Computation

namespace LeanerIR.Tests.NativeDescriptorCoverage

open LeanerIR.Proofs.Denotation LeanerIR.SemanticOperations

example (frame : RuntimeFrame) (state : RuntimeState) :
    PrimitiveLocationOperation.logicalNot.evaluate? #[.bool true] frame state =
      some (.value frame state (.bool false)) := rfl

example (frame : RuntimeFrame) (state : RuntimeState) :
    PrimitiveLocationOperation.logicalNot.evaluate? #[.integer 0] frame state = none := rfl

example (frame : RuntimeFrame) (state : RuntimeState) :
    (PrimitiveLocationOperation.length (.integer (.bits 64) false)).evaluate?
      #[.string "é"] frame state = some (.value frame state (.integer 2)) := rfl

example (frame : RuntimeFrame) (state : RuntimeState) :
    (PrimitiveLocationOperation.length (.integer (.bits 8) false)).evaluate?
      #[.vector (Array.replicate 256 .unit)] frame state =
        some (.value frame state (.integer 0)) := by
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator, modularInteger]

example (frame : RuntimeFrame) (state : RuntimeState) :
    (PrimitiveLocationOperation.length (.integer .unbounded true)).evaluate?
      #[.vector (Array.replicate 256 .unit)] frame state =
        some (.value frame state (.integer 256)) := by
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator]

example (frame : RuntimeFrame) (state : RuntimeState) :
    (PrimitiveLocationOperation.length (.integer (.bits 64) false)).evaluate?
      #[.bool false] frame state = none := rfl

-- Porting v0 Language/Signed exposes a contract discrepancy: its remainder
-- contract excludes only zero, but the actual Move VM's checked_rem also
-- rejects MIN_INT % -1. Keep the existing execution semantics pinned while
-- the source-contract correction is decided; a mathematical zero remainder
-- does not imply that this checked operation returns normally.
example : (-2147483648 : Int).tmod (-1) = 0 := by decide

example (frame : RuntimeFrame) (state : RuntimeState) :
    (PrimitiveLocationOperation.checkedModulo .abort (.integer (.bits 32) true)).evaluate?
      #[.integer (-2147483648), .integer (-1)] frame state =
        some (.throw_ frame state .abort #[.integer 2147483648]) := by
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    checkedModuloIntegers?, truncatingQuotient?, checkedInteger, Ty.integerBounds?]

example (frame : RuntimeFrame) (state : RuntimeState) :
    (PrimitiveLocationOperation.checkedModulo .abort (.integer (.bits 32) true)).evaluate?
      #[.integer (-2147483648), .integer 1] frame state =
        some (.value frame state (.integer 0)) := by
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    checkedModuloIntegers?, truncatingQuotient?, checkedInteger, Ty.integerBounds?]

end LeanerIR.Tests.NativeDescriptorCoverage
