-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.StoredValueInvariants

namespace LeanerIR.Tests.StoredValueInvariants

open Proofs.Denote

private def optionOwner : StructHandle := ⟨⟨0⟩, 0⟩
private def optionType : NTy :=
  .struct optionOwner (.cons .bool .nil) (.cons (.vector .bool) .nil)
private def tableType : NTy :=
  .struct ⟨⟨0⟩, 1⟩ (.cons .bool (.cons optionType .nil)) (.cons .address .nil)

private def declared (source : StructHandle) (_arguments : NRow) (raw : RuntimeValue) : Prop :=
  source = optionOwner → (DataInvariant.elements (raw.field 0)).length ≤ 1

private def twoValues : SpecVector Bool := ⟨#[true, false], by decide⟩

-- The global-resource predicate contributes no premise. The Option invariant
-- is inherited solely from its external Table slot, through a shared lookup.
example {unit : Validation.ValidatedUnit} (memory : Memory unit)
    (holds : MemoryInvariants (DataInvariant.withCollections (fun _ _ => True) declared) memory)
    (contents : TableMemory.Contents unit .bool optionType)
    (read : TableMemory.read memory tableType .bool optionType "table" = some contents)
    (query : Bool) (held : @NTy.carrier (Carriers.runtime unit) optionType)
    (found : TableMemory.lookup? .bool optionType contents query = some held) :
    held.1.values.size ≤ 1 := by
  have valid := DataInvariant.table_lookup holds tableType .bool optionType "table"
    contents read query held found
  have own := valid.1 rfl
  simpa [optionType, DataInvariant.elements, RuntimeValue.field] using own

-- An arbitrary typed memory can contain a vector of length two. Typing alone
-- must not invent the declared Option invariant: such storage is rejected.
example (unit : Validation.ValidatedUnit) :
    let bad : @NTy.carrier (Carriers.runtime unit) optionType :=
      (twoValues, ())
    let memory := TableMemory.write (fun _ _ => none) tableType .bool optionType "table"
      #[(true, bad, ())]
    ¬MemoryInvariants (DataInvariant.withCollections (fun _ _ => True) declared) memory := by
  dsimp only
  intro holds
  have own := DataInvariant.collection_member holds
    (.tuple (.cons .bool (.cons optionType .nil))) (.cons tableType .nil) (.address "table")
    #[(true, (twoValues, ()), ())]
    (by simp [TableMemory.write, TableMemory.resource])
    (true, (twoValues, ()), ()) (by simp)
  have impossible := own.2.1.1 rfl
  simp [optionType, DataInvariant.elements, RuntimeValue.field, twoValues] at impossible

-- Phantom arguments do not appear in the physical encoding, but an authored
-- invariant may distinguish them (for example through a generic spec call).
private def phantomOwner : StructHandle := ⟨⟨0⟩, 2⟩

private def phantomType (argument : NTy) : NTy :=
  .struct phantomOwner (.cons argument .nil) (.cons .bool .nil)

private def phantomDeclared (source : StructHandle) (arguments : NRow)
    (raw : RuntimeValue) : Prop :=
  source = phantomOwner → (arguments = .cons .bool .nil ↔ raw.field 0 = .bool true)

example {unit : Validation.ValidatedUnit} (memory : Memory unit)
    (holds : MemoryInvariants
      (DataInvariant.withCollections (fun _ _ => True) phantomDeclared) memory)
    (argument : NTy) (contents : TableMemory.Contents unit .bool (phantomType argument))
    (read : TableMemory.read memory tableType .bool (phantomType argument) "table" =
      some contents)
    (held : @NTy.carrier (Carriers.runtime unit) (phantomType argument))
    (found : TableMemory.lookup? .bool (phantomType argument) contents true = some held) :
    argument = .bool ↔ held.1 = true := by
  have valid := DataInvariant.table_lookup holds tableType .bool (phantomType argument)
    "table" contents read true held found
  have own := valid.1 rfl
  simpa [phantomType, RuntimeValue.field] using own

-- The same physical fields satisfy the predicate at one instantiation and
-- violate it at another. Dropping the type arguments would conflate them.
example :
    DataInvariant.value phantomDeclared (phantomType .bool)
      (.nominal phantomOwner none #[.bool true]) ∧
    ¬DataInvariant.value phantomDeclared (phantomType .address)
      (.nominal phantomOwner none #[.bool true]) := by
  simp [DataInvariant.value, DataInvariant.row, phantomType, phantomDeclared,
    RuntimeValue.field]

-- Exercise the write rule, including its obligation on the value, rather
-- than assuming the resulting memory already satisfies the predicate.
example (unit : Validation.ValidatedUnit) :
    MemoryInvariants (DataInvariant.withCollections (fun _ _ => True) phantomDeclared)
      (TableMemory.write (unit := unit) (fun _ _ => none) tableType .bool
        (phantomType .bool) "table" #[(true, (true, ()), ())]) := by
  have empty : MemoryInvariants
      (DataInvariant.withCollections (fun _ _ => True) phantomDeclared)
      (unit := unit) (fun _ _ => none) := by
    intro resource key value read
    contradiction
  apply DataInvariant.table_write empty
  · intro entry member
    trivial
  · intro entry member
    simp only [Array.mem_singleton] at member
    subst entry
    simp [DataInvariant.value, DataInvariant.row, phantomType, phantomDeclared,
      RuntimeValue.field]

-- A write can invalidate the predicate. An invariant of the old memory does
-- not justify assuming the same fact of a separately chosen new memory.
example (unit : Validation.ValidatedUnit) :
    ¬MemoryInvariants (DataInvariant.withCollections (fun _ _ => True) phantomDeclared)
      (TableMemory.write (unit := unit) (fun _ _ => none) tableType .bool
        (phantomType .address) "table" #[(true, (true, ()), ())]) := by
  intro holds
  have valid := DataInvariant.collection_member holds
    (.tuple (.cons .bool (.cons (phantomType .address) .nil))) (.cons tableType .nil)
    (.address "table") #[(true, (true, ()), ())]
    (by simp [TableMemory.write, TableMemory.resource])
    (true, (true, ()), ()) (by simp)
  have impossible := valid.2.1.1 rfl
  simp [phantomType, RuntimeValue.field] at impossible

-- Traversal follows the active enum variant, then vectors and nominal fields.
-- It must neither omit the held value nor impose the inactive variant's fields.
private def wrappedType : NTy :=
  .enum ⟨⟨0⟩, 3⟩ .nil ["Held", "Empty"]
    (.cons (.cons (.vector (phantomType .bool)) .nil) (.cons .nil .nil)) (by decide)

example :
    DataInvariant.value phantomDeclared wrappedType
      (.nominal ⟨⟨0⟩, 3⟩ (some "Empty") #[]) ∧
    ¬DataInvariant.value phantomDeclared wrappedType
      (.nominal ⟨⟨0⟩, 3⟩ (some "Held")
        #[.vector #[.nominal phantomOwner none #[.bool false]]]) := by
  simp [DataInvariant.value, DataInvariant.row, DataInvariant.variant,
    DataInvariant.elements, wrappedType, phantomType, phantomDeclared,
    phantomOwner, RuntimeValue.field, RuntimeValue.variant?]

end LeanerIR.Tests.StoredValueInvariants
