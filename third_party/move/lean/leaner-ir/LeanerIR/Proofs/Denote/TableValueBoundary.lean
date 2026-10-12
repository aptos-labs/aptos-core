-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.StateView
import LeanerIR.Proofs.Denote.SnapshotValue

/-! A representation boundary for owned logical observations. Storage is consulted
at this boundary, not embedded in the logical value. This supports nested and
generic Table snapshots and is independent of program points. It does not yet
replace denotation carriers or supply allocation/ownership validity. -/

namespace LeanerIR.Proofs.Denote.SnapshotValue

noncomputable def stateView (frame : Skolems unit) (type : NTy) :
    StateView (Memory unit) Value RuntimeValue where
  erase := Value.physical
  recover? := fun memory => observeRuntime? frame memory type
  erase_recover := fun memory => observeRuntime?_physical frame memory type

theorem stateView_realizes_observe (frame : Skolems unit) (memory : Memory unit)
    (type : NTy) (value : @NTy.carrier frame.toCarriers type) :
    (stateView frame type).Realizes memory (observeInFrame frame memory type value) := by
  change observeRuntime? frame memory type (observeInFrame frame memory type value).physical = _
  rw [physical_observeInFrame, observeRuntime?_encode]

/-- A typed logical value retains its observation, rather than rereading its
handle when it is used. The witness says that the snapshot has a representation
at some state, not necessarily the state in which a later operation runs. It
does not grant ownership or assert that a Table's contents slot is allocated. -/
structure Observed (frame : Skolems unit) (type : NTy) where
  physical : @NTy.carrier frame.toCarriers type
  value : Value
  represented : ∃ memory, observeInFrame frame memory type physical = value

namespace Observed

/-- Capture once at a boundary. Ordinary logical operations consume `value`. -/
noncomputable def capture (frame : Skolems unit) (memory : Memory unit) (type : NTy)
    (physical : @NTy.carrier frame.toCarriers type) : Observed frame type :=
  ⟨physical, observeInFrame frame memory type physical, ⟨memory, rfl⟩⟩

@[ext] theorem ext {left right : Observed frame type}
    (physical : left.physical = right.physical) (value : left.value = right.value) :
    left = right := by
  cases left
  cases right
  cases physical
  cases value
  rfl

theorem physical_value (observed : Observed frame type) :
    observed.value.physical = @NTy.encode frame.toCarriers type observed.physical := by
  obtain ⟨memory, represented⟩ := observed.represented
  rw [← represented, physical_observeInFrame]

/-- Typed physical carriers still supply the runtime side of the boundary.
Logical clients retain the enriched value even when the physical value is just
a handle. A live input must additionally satisfy this view's `Realizes`. -/
noncomputable def view (frame : Skolems unit) (type : NTy) :
    StateView (Memory unit) (Observed frame type) (@NTy.carrier frame.toCarriers type) where
  erase := Observed.physical
  recover? := fun memory physical => some (capture frame memory type physical)
  erase_recover := by
    intro memory physical value recovered
    cases Option.some.inj recovered
    rfl

theorem realizes_iff (frame : Skolems unit) (memory : Memory unit) (type : NTy)
    (value : Observed frame type) :
    (view frame type).Realizes memory value ↔
      observeInFrame frame memory type value.physical = value.value := by
  change some (capture frame memory type value.physical) = some value ↔ _
  constructor
  · intro same
    exact congrArg Observed.value (Option.some.inj same)
  · intro same
    exact congrArg some (Observed.ext rfl same)

/-- Generic transport changes the physical carrier while retaining the exact
logical snapshot. No observation is taken at a new or arbitrary state. -/
noncomputable def toSkolem (frame : Skolems unit) (arguments : TypeArgs) (type : NTy)
    (value : Observed frame (type.subst arguments.1)) :
    Observed (Skolems.instantiate arguments frame) type where
  physical := @NTy.toSkolem frame.toCarriers arguments type value.physical
  value := value.value
  represented := by
    obtain ⟨memory, observed⟩ := value.represented
    exact ⟨memory, (observeInFrame_toSkolem frame arguments memory type value.physical).trans observed⟩

noncomputable def ofSkolem (frame : Skolems unit) (arguments : TypeArgs) (type : NTy)
    (value : Observed (Skolems.instantiate arguments frame) type) :
    Observed frame (type.subst arguments.1) where
  physical := @NTy.ofSkolem frame.toCarriers arguments type value.physical
  value := value.value
  represented := by
    obtain ⟨memory, observed⟩ := value.represented
    refine ⟨memory, ?_⟩
    rw [← observeInFrame_toSkolem frame arguments memory type, NTy.toSkolem_ofSkolem]
    exact observed

theorem ofSkolem_toSkolem (frame : Skolems unit) (arguments : TypeArgs) (type : NTy)
    (value : Observed frame (type.subst arguments.1)) :
    ofSkolem frame arguments type (toSkolem frame arguments type value) = value := by
  apply Observed.ext
  · exact NTy.ofSkolem_toSkolem arguments type value.physical
  · rfl

theorem toSkolem_ofSkolem (frame : Skolems unit) (arguments : TypeArgs) (type : NTy)
    (value : Observed (Skolems.instantiate arguments frame) type) :
    toSkolem frame arguments type (ofSkolem frame arguments type value) = value := by
  apply Observed.ext
  · exact NTy.toSkolem_ofSkolem arguments type value.physical
  · rfl

end Observed

mutual
/-- Mutable boundary values retain separate current and prophetic logical values.
Plain/shared values use the first observation state. Tuples carry their components
separately, including returned references. The states must be supplied by loan
resolution, not inferred from a physical handle. -/
def Argument (frame : Skolems unit) : NTy → Type
  | .ref referent => Observed frame referent × Observed frame referent
  | .tuple fields => ArgumentRow frame fields
  | type => Observed frame type

/-- Logical argument rows use the compiler's existing type indexes and physical
row shape, but carry observations through the function boundary as values. -/
def ArgumentRow (frame : Skolems unit) : NRow → Type
  | .nil => Unit
  | .cons type rest => Argument frame type × ArgumentRow frame rest
end

mutual
noncomputable def argumentView (frame : Skolems unit) (type : NTy) :
    StateView (Memory unit × Memory unit) (Argument frame type)
      (@NTy.carrier frame.toCarriers type) := by
  cases type <;> first
    | exact (Observed.view frame _).prophecy
    | exact argumentRowView frame _
    | exact (Observed.view frame _).atState Prod.fst

noncomputable def argumentRowView (frame : Skolems unit) : (row : NRow) →
    StateView (Memory unit × Memory unit) (ArgumentRow frame row)
      (@HList frame.toCarriers row)
  | .nil => {
      erase := id
      recover? := fun _ value => some value
      erase_recover := fun _ _ _ recovered => (Option.some.inj recovered).symm }
  | .cons type rest => (argumentView frame type).prod (argumentRowView frame rest)
end

/-- The compiled result shape also retains logical values. For a returned
reference, the second memory is its loan-resolved prophetic observation. -/
def Result (frame : Skolems unit) : ResultShape → Type
  | .none => Unit
  | .one type => Argument frame type

noncomputable def resultView (frame : Skolems unit) : (shape : ResultShape) →
    StateView (Memory unit × Memory unit) (Result frame shape)
      (@ResultShape.carrier frame.toCarriers shape)
  | .none => argumentRowView frame .nil
  | .one type => argumentView frame type

end LeanerIR.Proofs.Denote.SnapshotValue
