-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.GivenOwn

namespace VerexIR.Tests.GivenOwn

open VerexIR.Proofs.Denote

private def wrapper (source : StructHandle) : NTy :=
  .struct source (.cons (.param 0) .nil) (.cons (.param 0) .nil)

/-- An ADT with no own clauses packs an arbitrary given parameter value. -/
example (frame : GivenFrame) (source : StructHandle)
    (value : @NTy.plain frame.toCarriers (.param 0)) :
    @NTy.deepHolds (frame.withDeclarations (fun _ _ => none)).toCarriers
      (wrapper source) (value, ()) := by
  exact ⟨trivial, trivial, trivial⟩

/-- Installing declared invariants never constrains the opaque parameter sort. -/
example (frame : GivenFrame) (declarations : GivenOwnDeclarations frame)
    (index : Nat) (value : @NTy.plain frame.toCarriers (.param index)) :
    @NTy.deepHolds (frame.withDeclarations declarations).toCarriers (.param index) value :=
  trivial

private def falseClauses (frame : GivenFrame) (source : StructHandle) :
    GivenOwnDeclarations frame :=
  fun owner _ => if owner = source then some ⟨wrapper source, fun _ => False⟩ else none

/-- A false declared invariant prevents certification, even though the field's
given parameter invariant is True. -/
theorem false_declared_rejected (frame : GivenFrame) (source : StructHandle)
    (value : @NTy.plain frame.toCarriers (.param 0)) :
    ¬ @NTy.deepHolds (frame.withDeclarations (falseClauses frame source)).toCarriers
      (wrapper source) (value, ()) := by
  intro held
  have own := held.1
  exact (frame.withDeclarations_own_encode (falseClauses frame source) source
    (.cons (.param 0) .nil) ⟨wrapper source, fun _ => False⟩
    (by simp [falseClauses]) (value, ())).mp own

/-- Nesting the false-invariant ADT inside another ADT does not hide its own
invariant: deepHolds still descends into the field. -/
theorem nested_declared_rejected (frame : GivenFrame) (inner outer : StructHandle)
    (distinct : outer ≠ inner)
    (value : @NTy.plain frame.toCarriers (.param 0)) :
    let family := (frame.withDeclarations (falseClauses frame inner)).toCarriers
    let type := NTy.struct outer .nil (.cons (wrapper inner) .nil)
    @Carriers.own family outer .nil (@NTy.plainEncode family type ((value, ()), ())) ∧
      ¬ @NTy.deepHolds family type ((value, ()), ()) := by
  constructor
  · apply frame.withDeclarations_own_none (falseClauses frame inner)
    simp [falseClauses, distinct]
  · intro held
    exact false_declared_rejected frame inner value held.2.1

/-- An opaque declaration predicate is preserved exactly, with no additional
condition on parameter values and no runtime surjectivity premise. -/
example (frame : GivenFrame) (source : StructHandle)
    (predicate : @NTy.plain frame.toCarriers (.param 0) → Prop)
    (value : @NTy.plain frame.toCarriers (.param 0)) :
    let declarations : GivenOwnDeclarations frame :=
      fun _ _ => some ⟨wrapper source, fun row => predicate row.1⟩
    @NTy.deepHolds (frame.withDeclarations declarations).toCarriers
      (wrapper source) (value, ()) ↔ predicate value := by
  dsimp only
  have own := frame.withDeclarations_own_encode
    (fun _ _ => some ⟨wrapper source, fun row => predicate row.1⟩) source
    (.cons (.param 0) .nil) ⟨wrapper source, fun row => predicate row.1⟩ rfl (value, ())
  exact ⟨fun held => own.mp held.1, fun held => ⟨own.mpr held, trivial, trivial⟩⟩

#print axioms false_declared_rejected
#print axioms nested_declared_rejected

end VerexIR.Tests.GivenOwn
