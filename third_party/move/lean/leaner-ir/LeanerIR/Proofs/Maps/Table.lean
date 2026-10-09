-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Maps

/-!
# Table identity and owned contents

The logical snapshot of a Table separates its allocation identity from its
contents (`designs/intrinsic-maps.md`). Identity equality is deliberately not
Lean equality of snapshots: two states can contain different entries for the
same Table. Contents use the existing intrinsic-map operations and their
proved laws, independently of the physical handle's representation.

This library does not change a physical Table declaration or assert agreement
with native storage. That agreement, the carrier used by the denotation, and
the native contracts are separate integration obligations.
-/

namespace LeanerIR.Maps.Table

attribute [local instance] LeanerIR.Maps.decEq

/-- An observation of one Table at one state. The identity type is independent
of the contents, so a handle or a logical allocation number can identify it. -/
structure Snapshot (Identity : Type) where
  identity : Identity
  entries : Entries

namespace Snapshot

variable {Identity : Type}

/-- Table equality in specifications. It does not identify content snapshots. -/
def SameIdentity (left right : Snapshot Identity) : Prop := left.identity = right.identity

instance [DecidableEq Identity] : DecidableRel (@SameIdentity Identity) :=
  fun _ _ => inferInstanceAs (Decidable (_ = _))

theorem sameIdentity_refl (table : Snapshot Identity) : SameIdentity table table := rfl

theorem sameIdentity_symm {left right : Snapshot Identity} :
    SameIdentity left right → SameIdentity right left := Eq.symm

theorem sameIdentity_trans {left middle right : Snapshot Identity} :
    SameIdentity left middle → SameIdentity middle right → SameIdentity left right := Eq.trans

/-- An empty content snapshot of a specified identity; this does not allocate. -/
def empty (identity : Identity) : Snapshot Identity := ⟨identity, []⟩

def size (table : Snapshot Identity) : Int := table.entries.length

def hasKey (table : Snapshot Identity) (key : RuntimeValue) : Bool :=
  containsB table.entries key

def valueAt (table : Snapshot Identity) (key : RuntimeValue) : RuntimeValue :=
  get table.entries key

/-- The key discipline of the owned contents. Native value typing and any
variant-specific cached length are separate storage/carrier obligations. -/
def Valid (discipline : Discipline) (table : Snapshot Identity) : Prop :=
  discipline.Valid table.entries

def update (discipline : Discipline) (table : Snapshot Identity) (key value : RuntimeValue) :
    Snapshot Identity := ⟨table.identity, discipline.set table.entries key value⟩

def remove (discipline : Discipline) (table : Snapshot Identity) (key : RuntimeValue) :
    Snapshot Identity := ⟨table.identity, discipline.del table.entries key⟩

@[simp] theorem identity_empty (identity : Identity) : (empty identity).identity = identity := rfl

@[simp] theorem identity_update (discipline : Discipline) (table : Snapshot Identity)
    (key value : RuntimeValue) : (update discipline table key value).identity = table.identity := rfl

@[simp] theorem identity_remove (discipline : Discipline) (table : Snapshot Identity)
    (key : RuntimeValue) : (remove discipline table key).identity = table.identity := rfl

@[simp] theorem size_empty (identity : Identity) : size (empty identity) = 0 := rfl

@[simp] theorem hasKey_empty (identity : Identity) (key : RuntimeValue) :
    hasKey (empty identity) key = false := rfl

theorem valid_empty (identity : Identity) (discipline : Discipline) :
    Valid discipline (empty identity) := by
  cases discipline <;> simp [Valid, empty, Discipline.Valid, Distinct, Ascending]

theorem hasKey_update (discipline : Discipline) (table : Snapshot Identity)
    (key value probe : RuntimeValue) :
    hasKey (update discipline table key value) probe =
      (decide (key = probe) || hasKey table probe) := by
  cases discipline <;> simp [hasKey, update, Discipline.set, containsB_ordSet]

theorem valueAt_update (discipline : Discipline) (table : Snapshot Identity)
    (key value probe : RuntimeValue) :
    valueAt (update discipline table key value) probe =
      if probe = key then value else valueAt table probe := by
  cases discipline <;> simp [valueAt, update, Discipline.set, get_ordSet]

theorem valid_update {discipline : Discipline} {table : Snapshot Identity}
    (valid : Valid discipline table) (key value : RuntimeValue) :
    Valid discipline (update discipline table key value) := by
  cases discipline with
  | sequence => exact distinct_seqSet valid key value
  | ordered rank => exact ascending_ordSet rank valid key value

theorem size_update {discipline : Discipline} {table : Snapshot Identity}
    (valid : Valid discipline table) (key value : RuntimeValue) :
    size (update discipline table key value) =
      if hasKey table key then size table else size table + 1 := by
  cases discipline with
  | sequence =>
      simp only [size, update, Discipline.set_sequence, length_seqSet, hasKey]
      split <;> simp
  | ordered rank =>
      simp only [size, update, Discipline.set_ordered, hasKey]
      rw [length_ordSet rank valid]
      split <;> simp

theorem hasKey_remove {discipline : Discipline} {table : Snapshot Identity}
    (valid : Valid discipline table) (key probe : RuntimeValue) :
    hasKey (remove discipline table key) probe =
      (!decide (key = probe) && hasKey table probe) := by
  have distinct := distinct_of_valid valid
  cases discipline with
  | sequence => simpa [hasKey, remove] using containsB_seqDel distinct key probe
  | ordered _ => simpa [hasKey, remove] using containsB_ordDel distinct key probe

theorem valueAt_remove {discipline : Discipline} {table : Snapshot Identity}
    (valid : Valid discipline table) (key probe : RuntimeValue) (differs : probe ≠ key) :
    valueAt (remove discipline table key) probe = valueAt table probe := by
  cases discipline with
  | sequence => simpa [valueAt, remove] using get_seqDel valid key probe differs
  | ordered _ => simpa [valueAt, remove] using get_ordDel table.entries key probe differs

theorem valid_remove {discipline : Discipline} {table : Snapshot Identity}
    (valid : Valid discipline table) (key : RuntimeValue) :
    Valid discipline (remove discipline table key) := by
  cases discipline with
  | sequence => exact distinct_seqDel valid key
  | ordered rank => exact ascending_ordDel rank valid key

/-- The key positions remain fixed when a present entry is written through a
mutable reference. Structural operations need not have this property. -/
theorem keys_update_present {discipline : Discipline} {table : Snapshot Identity}
    (valid : Valid discipline table) (key value : RuntimeValue)
    (present : hasKey table key = true) :
    (update discipline table key value).entries.map Prod.fst = table.entries.map Prod.fst := by
  cases discipline with
  | sequence => exact map_fst_seqSet_present value present
  | ordered rank => exact map_fst_ordSet_present rank value present valid

/-- The content part of a mutable entry borrow: its current observation and
the owner's snapshot at the returned reference's final value. The borrow
certificate, rather than this pure data, licenses exclusive ownership. -/
structure EntryLoan (Identity : Type) where
  current : RuntimeValue
  reconcile : RuntimeValue → Snapshot Identity

def borrowEntry (discipline : Discipline) (table : Snapshot Identity) (key : RuntimeValue) :
    EntryLoan Identity := ⟨valueAt table key, update discipline table key⟩

theorem borrowEntry_current (discipline : Discipline) (table : Snapshot Identity)
    (key : RuntimeValue) : (borrowEntry discipline table key).current = valueAt table key := rfl

theorem borrowEntry_identity (discipline : Discipline) (table : Snapshot Identity)
    (key final : RuntimeValue) :
    SameIdentity ((borrowEntry discipline table key).reconcile final) table := rfl

theorem borrowEntry_value (discipline : Discipline) (table : Snapshot Identity)
    (key final : RuntimeValue) :
    valueAt ((borrowEntry discipline table key).reconcile final) key = final := by
  simp [borrowEntry, valueAt_update]

theorem borrowEntry_frame (discipline : Discipline) (table : Snapshot Identity)
    (key final probe : RuntimeValue) (differs : probe ≠ key) :
    valueAt ((borrowEntry discipline table key).reconcile final) probe = valueAt table probe := by
  simp [borrowEntry, valueAt_update, differs]

theorem borrowEntry_size {discipline : Discipline} {table : Snapshot Identity}
    (valid : Valid discipline table) (key final : RuntimeValue)
    (present : hasKey table key = true) :
    size ((borrowEntry discipline table key).reconcile final) = size table := by
  simp [borrowEntry, size_update valid, present]

theorem borrowEntry_keys {discipline : Discipline} {table : Snapshot Identity}
    (valid : Valid discipline table) (key final : RuntimeValue)
    (present : hasKey table key = true) :
    ((borrowEntry discipline table key).reconcile final).entries.map Prod.fst =
      table.entries.map Prod.fst :=
  keys_update_present valid key final present

/-- Same identity supplies no equality of observations across snapshots. -/
theorem sameIdentity_different_values (identity : Identity) :
    ∃ left right : Snapshot Identity, SameIdentity left right ∧
      valueAt left (.integer 0) ≠ valueAt right (.integer 0) := by
  refine ⟨update .sequence (empty identity) (.integer 0) (.integer 1),
    update .sequence (empty identity) (.integer 0) (.integer 2), rfl, ?_⟩
  simp only [valueAt_update, ↓reduceIte]
  simp

end Snapshot

/-- A finite allocation history represented by a monotone logical counter.
The native-storage agreement must relate these identities to fresh handles;
the counter is not the concrete address-generation algorithm. -/
structure Allocator where
  next : Nat
  deriving Inhabited

def Allocator.Allocated (allocator : Allocator) (identity : Nat) : Prop := identity < allocator.next

/-- Allocate a new identity and an empty snapshot, advancing the history even
if the caller later destroys the Table. Identities are never reused. -/
def Allocator.allocate (allocator : Allocator) : Allocator × Snapshot Nat :=
  (⟨allocator.next + 1⟩, Snapshot.empty allocator.next)

theorem Allocator.fresh (allocator : Allocator) :
    ¬ allocator.Allocated allocator.allocate.2.identity := by
  simp [Allocated, allocate, Snapshot.empty]

theorem Allocator.allocated (allocator : Allocator) :
    allocator.allocate.1.Allocated allocator.allocate.2.identity := by
  simp [Allocated, allocate, Snapshot.empty]

theorem Allocator.preserves {allocator : Allocator} {identity : Nat}
    (allocated : allocator.Allocated identity) : allocator.allocate.1.Allocated identity := by
  simp only [Allocated, allocate] at *
  omega

theorem Allocator.distinct_from_allocated {allocator : Allocator}
    {table : Snapshot Nat} (allocated : allocator.Allocated table.identity) :
    ¬ Snapshot.SameIdentity allocator.allocate.2 table := by
  simp only [Snapshot.SameIdentity, allocate, Snapshot.empty]
  exact Nat.ne_of_gt allocated

theorem Allocator.two_allocations_distinct (allocator : Allocator) :
    ¬ Snapshot.SameIdentity allocator.allocate.2 allocator.allocate.1.allocate.2 := by
  simp [Snapshot.SameIdentity, allocate, Snapshot.empty]

end LeanerIR.Maps.Table
