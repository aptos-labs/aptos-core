-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.Types
import LeanerIR.Proofs.Maps.Table

/-!
# Typed storage of Table contents

A Table's contents occupy a logical memory slot selected by its owner type
(including phantom arguments), key/value types, and handle. The stored entries
are typed independently of the Table's physical fields. Reading a snapshot
uses an explicit memory argument, so old states and state labels need no
program-point metadata here.

These laws provide the Table component of `StorageEncodes` at call boundaries.
Native operation contracts and entry-loan reconciliation must still be related
to execution before the frontend can use this view.
-/

namespace LeanerIR.Proofs.Denote.TableMemory

open LeanerIR.Validation

/-- An unbounded typed entry array, separate from its owner's physical declaration.
The owner in the resource arguments separates owners and phantom instances. -/
def resource (owner key value : NTy) : ResourceType :=
  ⟨.tuple (.cons key (.cons value .nil)), .cons owner .nil, .collection⟩

/-- The native field rows of supported handle-backed Tables. The production
Table carries only its handle; the extension fixture also caches its length.
Entries-layout and empty abstract map owners do not have this representation. -/
def handleFields : NRow → Bool
  | .cons .address .nil => true
  | .cons .address (.cons (.int 64 false) .nil) => true
  | _ => false

/-- The contents resource named by a Table type in a unit. Unlike a global
resource, its owner need not have the `key` ability. Generic types remain
templates here, to be instantiated by a frame before naming runtime storage. -/
def resourceOf (unit : ValidatedUnit) (namespaceId : NamespaceId) (typeId : TypeId) :
    Option ResourceType := do
  -- Shared references erase to their referent in `ntyOf`. Only the owning
  -- nominal type names storage; a reference must not create a second slot.
  let .nominal .. ← unitTypes unit namespaceId typeId | none
  let owner ← ntyOf unit namespaceId typeId
  let .struct handle arguments fields := owner | none
  guard (handleFields fields)
  let .cons key (.cons value .nil) := arguments | none
  let ns ← unit.namespaces[handle.namespaceId.index]?
  let declaration ← ns.structs[handle.structId]?
  guard (ns.intrinsics.any fun intrinsic =>
    intrinsic.model == "map" && intrinsic.owner == declaration.name)
  some (resource owner key value)

/-- The declaration's namespace owns the runtime slot. Other namespaces may
name the same logical resource through `resourceOf`, without duplicating it. -/
def ownerNamespace? (contents : ResourceType) : Option NamespaceId :=
  match contents.arguments with
  | .cons (.struct owner _ _) .nil => some owner.namespaceId
  | _ => none

/-- One physical slot per logical contents resource, even when an input unit
contains duplicate type entries. Type-table interning is not a proof premise. -/
def canonicalAt (unit : ValidatedUnit) (namespaceId : NamespaceId) (typeId : TypeId)
    (contents : ResourceType) : Bool :=
  ownerNamespace? contents == some namespaceId &&
    !(List.range typeId.index).any fun earlier =>
      resourceOf unit namespaceId ⟨earlier⟩ == some contents

/-- Only closed instances at their canonical owner/type key name concrete
native storage. Generic caller/callee templates still use `resourceOf`. -/
def runtimeResourceOf (unit : ValidatedUnit) (namespaceId : NamespaceId) (typeId : TypeId) :
    Option ResourceType :=
  (resourceOf unit namespaceId typeId).filter fun contents =>
    contents.type.paramFree && contents.arguments.paramFree &&
      canonicalAt unit namespaceId typeId contents

theorem runtimeResourceOf_sound {namespaceId : NamespaceId} {typeId : TypeId}
    {contents : ResourceType}
    (named : runtimeResourceOf unit namespaceId typeId = some contents) :
    resourceOf unit namespaceId typeId = some contents ∧
      canonicalAt unit namespaceId typeId contents = true := by
  obtain ⟨source, accepted⟩ := Option.filter_eq_some_iff.mp named
  exact ⟨source, (Bool.and_eq_true_iff.mp accepted).2⟩

/-- Two runtime names of the same logical Table contents are the same slot.
The allocator and mutable-entry adapter need this to update the whole heap
encoding by updating one selected slot. -/
theorem runtimeResourceOf_unique {leftNamespace rightNamespace : NamespaceId}
    {leftType rightType : TypeId} {contents : ResourceType}
    (left : runtimeResourceOf unit leftNamespace leftType = some contents)
    (right : runtimeResourceOf unit rightNamespace rightType = some contents) :
    leftNamespace = rightNamespace ∧ leftType = rightType := by
  obtain ⟨leftSource, leftCanonical⟩ := runtimeResourceOf_sound left
  obtain ⟨rightSource, rightCanonical⟩ := runtimeResourceOf_sound right
  simp only [canonicalAt, Bool.and_eq_true, beq_iff_eq] at leftCanonical rightCanonical
  have namespaces : leftNamespace = rightNamespace :=
    Option.some.inj (leftCanonical.1.symm.trans rightCanonical.1)
  subst rightNamespace
  have noEarlier (type : TypeId) (canonical :
      (!(List.range type.index).any fun earlier =>
        resourceOf unit leftNamespace ⟨earlier⟩ == some contents) = true) :
      ∀ earlier < type.index, resourceOf unit leftNamespace ⟨earlier⟩ ≠ some contents := by
    simpa only [Bool.not_eq_true', List.any_eq_false, List.mem_range,
      beq_iff_eq] using canonical
  have indexes : leftType.index = rightType.index := by
    rcases Nat.lt_trichotomy leftType.index rightType.index with less | equal | greater
    · exact False.elim (noEarlier rightType rightCanonical.2 leftType.index less leftSource)
    · exact equal
    · exact False.elim (noEarlier leftType leftCanonical.2 rightType.index greater rightSource)
  exact ⟨rfl, by cases leftType; cases rightType; cases indexes; rfl⟩

/-- Resolve a caller's closed Table type to the one physical owner/type key.
Scanning raw resources first keeps this linear in the type-table size. The
final check also rejects open type templates and malformed owner metadata. -/
def slotOf? (unit : ValidatedUnit) (namespaceId : NamespaceId) (typeId : TypeId)
    (handle : String) : Option GlobalKey := do
  let contents ← resourceOf unit namespaceId typeId
  let owner ← ownerNamespace? contents
  let ns ← unit.namespaces[owner.index]?
  let index ← (List.range ns.tables.types.size).find? fun index =>
    resourceOf unit owner ⟨index⟩ == some contents
  guard (runtimeResourceOf unit owner ⟨index⟩ == some contents)
  some ⟨owner, ⟨index⟩, .address handle⟩

theorem slotOf?_sound {namespaceId : NamespaceId} {typeId : TypeId}
    {handle : String} {slot : GlobalKey}
    (resolved : slotOf? unit namespaceId typeId handle = some slot) :
    ∃ contents, resourceOf unit namespaceId typeId = some contents ∧
      runtimeResourceOf unit slot.namespaceId slot.typeId = some contents ∧
      slot.key = .address handle := by
  unfold slotOf? at resolved
  obtain ⟨contents, source, resolved⟩ := Option.bind_eq_some_iff.mp resolved
  obtain ⟨owner, _, resolved⟩ := Option.bind_eq_some_iff.mp resolved
  obtain ⟨ns, _, resolved⟩ := Option.bind_eq_some_iff.mp resolved
  obtain ⟨index, _, resolved⟩ := Option.bind_eq_some_iff.mp resolved
  by_cases named : runtimeResourceOf unit owner ⟨index⟩ = some contents
  · simp [named, guard] at resolved
    subst slot
    exact ⟨contents, source, named, rfl⟩
  · simp [named, guard, failure] at resolved

/-- Caller and callee spelling, including duplicate type-table entries, cannot
change the selected slot when they denote the same logical contents resource. -/
theorem slotOf?_same_resource {leftNamespace rightNamespace : NamespaceId}
    {leftType rightType : TypeId} (handle : String)
    (same : resourceOf unit leftNamespace leftType = resourceOf unit rightNamespace rightType) :
    slotOf? unit leftNamespace leftType handle = slotOf? unit rightNamespace rightType handle := by
  simp only [slotOf?, same]

/-- The native table heap encodes the named contents slots of a typed memory.
These are entry vectors, rather than values at the owner's physical type. -/
def Encodes (unit : ValidatedUnit) (memory : Memory unit) (heap : GlobalMap) : Prop :=
  heap.Sorted ∧ ∀ namespaceId typeId key, heap.lookup ⟨namespaceId, typeId, key⟩ =
    match runtimeResourceOf unit namespaceId typeId with
    | some contents =>
        (memory contents key).map (contents.encode (unit := unit))
    | none => none

/-- The fixed logical slot of allocation history. Its address elements and
empty scope are independent of a callee's type parameters, and the collection
carrier imposes no Move vector length bound. -/
def allocationResource : ResourceType := ⟨.address, .nil, .collection⟩

def encodeAllocated (handles : Array String) : allocationResource.carrier unit := handles

/-- Both observable parts of native Table storage, related to one typed
memory. All other keys of the allocation-history resource are absent, so the
encoding determines the whole resource. Loan bookkeeping is handled separately. -/
def EncodesStorage (unit : ValidatedUnit) (memory : Memory unit)
    (storage : NativeTableStorage) : Prop :=
  Encodes unit memory storage.contents ∧
    ∀ key, memory allocationResource key =
      if key = .unit then some (encodeAllocated storage.allocated) else none

theorem Encodes.unique {memory : Memory unit} {left right : GlobalMap}
    (leftEncodes : Encodes unit memory left) (rightEncodes : Encodes unit memory right) :
    left = right :=
  GlobalMap.ext leftEncodes.1 rightEncodes.1 fun ⟨namespaceId, typeId, key⟩ => by
    rw [leftEncodes.2, rightEncodes.2]

/-- Encoded Table contents contain no live loans at an observation boundary. -/
theorem Encodes.plain {memory : Memory unit} {heap : GlobalMap}
    (encodes : Encodes unit memory heap) :
    ∀ slot ∈ heap.entries, SemanticOperations.Plain slot.value := by
  intro slot member
  have found := encodes.2 slot.key.namespaceId slot.key.typeId slot.key.key
  rw [show (⟨slot.key.namespaceId, slot.key.typeId, slot.key.key⟩ : GlobalKey) = slot.key from rfl,
    encodes.1.lookup_of_mem member] at found
  split at found
  · obtain ⟨value, -, encoded⟩ := Option.map_eq_some_iff.mp found.symm
    rw [← encoded]
    exact ResourceType.encode_plain _ value
  · cases found

theorem encodeAllocated_injective (left right : Array String)
    (equal : encodeAllocated (unit := unit) left = encodeAllocated right) : left = right := by
  exact equal

theorem EncodesStorage.unique {memory : Memory unit} {left right : NativeTableStorage}
    (leftEncodes : EncodesStorage unit memory left)
    (rightEncodes : EncodesStorage unit memory right) : left = right := by
  have contents := leftEncodes.1.unique rightEncodes.1
  have allocated := encodeAllocated_injective left.allocated right.allocated
    (Option.some.inj (by simpa using (leftEncodes.2 .unit).symm.trans (rightEncodes.2 .unit)))
  cases left
  cases right
  cases contents
  cases allocated
  rfl

/-- The contents stored at a Table slot. Both entry components are typed,
without a total size bound; distinctness/order remain map invariants. -/
abbrev Contents (unit : ValidatedUnit) (key value : NTy) : Type :=
  Array (@NTy.carrier (Carriers.runtime unit) (.tuple (.cons key (.cons value .nil))))

variable {unit : ValidatedUnit}

noncomputable def entries (key value : NTy) (contents : Contents unit key value) : Maps.Entries :=
  contents.toList.map fun entry =>
    (@NTy.encode (Carriers.runtime unit) key entry.1,
     @NTy.encode (Carriers.runtime unit) value entry.2.1)

theorem entries_length (key value : NTy) (contents : Contents unit key value) :
    (entries key value contents).length = contents.size := by
  simp [entries]

def read (memory : Memory unit) (owner key value : NTy) (handle : String) :
    Option (Contents unit key value) := memory (resource owner key value) (.address handle)

def write (memory : Memory unit) (owner key value : NTy) (handle : String)
    (contents : Contents unit key value) : Memory unit :=
  memory.set (resource owner key value) (.address handle) (some contents)

/-- A total observation. The native contract/data invariant must additionally
require the slot to exist for an allocated Table; an absent slot is junk. -/
noncomputable def snapshot (memory : Memory unit) (owner key value : NTy) (handle : String) :
    Maps.Table.Snapshot String :=
  ⟨handle, ((read memory owner key value handle).map (entries key value)).getD []⟩

@[simp] theorem read_write (memory : Memory unit) (owner key value : NTy) (handle : String)
    (contents : Contents unit key value) :
    read (write memory owner key value handle contents) owner key value handle = some contents := by
  simp only [read, write, Memory.set_same, ite_true]
  rfl

theorem read_write_other_handle (memory : Memory unit) (owner key value : NTy)
    (handle other : String) (contents : Contents unit key value) (distinct : other ≠ handle) :
    read (write memory owner key value handle contents) owner key value other =
      read memory owner key value other := by
  simp [read, write, distinct]

theorem write_other_resource (memory : Memory unit) (owner key value : NTy)
    (handle : String) (contents : Contents unit key value) (other : ResourceType)
    (distinct : other ≠ resource owner key value) :
    write memory owner key value handle contents other = memory other :=
  Memory.set_other_resource memory _ _ distinct

theorem resource_distinct_owners (left right key value : NTy) (distinct : left ≠ right) :
    resource left key value ≠ resource right key value := by
  intro equal
  have := congrArg ResourceType.arguments equal
  simp only [resource, NRow.cons.injEq, and_true] at this
  exact distinct this

theorem read_write_other_owner (memory : Memory unit) (owner other key value : NTy)
    (handle otherHandle : String) (contents : Contents unit key value)
    (distinct : other ≠ owner) :
    read (write memory owner key value handle contents) other key value otherHandle =
      read memory other key value otherHandle := by
  exact congrFun (write_other_resource memory owner key value handle contents
    (resource other key value) (resource_distinct_owners _ _ _ _ distinct)) (.address otherHandle)

@[simp] theorem snapshot_identity (memory : Memory unit) (owner key value : NTy) (handle : String) :
    (snapshot memory owner key value handle).identity = handle := rfl

@[simp] theorem snapshot_write_entries (memory : Memory unit) (owner key value : NTy)
    (handle : String) (contents : Contents unit key value) :
    (snapshot (write memory owner key value handle contents) owner key value handle).entries =
      entries key value contents := by
  simp [snapshot]

theorem snapshot_write_other_handle (memory : Memory unit) (owner key value : NTy)
    (handle other : String) (contents : Contents unit key value) (distinct : other ≠ handle) :
    snapshot (write memory owner key value handle contents) owner key value other =
      snapshot memory owner key value other := by
  simp only [snapshot, read_write_other_handle _ _ _ _ _ _ _ distinct]

/-- Snapshots read at different memories may share identity. No equality of
contents, or substitution between observations, follows from this fact. -/
theorem snapshot_same_identity (before after : Memory unit) (owner key value : NTy)
    (handle : String) :
    Maps.Table.Snapshot.SameIdentity (snapshot before owner key value handle)
      (snapshot after owner key value handle) := rfl

/-- Writes to distinct Table handles commute, including every unrelated
resource's slots in the memory. -/
theorem write_commute (memory : Memory unit) (owner key value : NTy) (left right : String)
    (leftContents rightContents : Contents unit key value) (distinct : left ≠ right) :
    write (write memory owner key value left leftContents) owner key value right rightContents =
      write (write memory owner key value right rightContents) owner key value left leftContents := by
  funext other otherKey
  by_cases same : other = resource owner key value
  · subst other
    simp only [write, Memory.set_same]
    by_cases atLeft : otherKey = .address left
    · subst otherKey
      simp [distinct]
    · by_cases atRight : otherKey = .address right
      · subst otherKey
        simp [Ne.symm distinct]
      · simp [atLeft, atRight]
  · simp [write, Memory.set_other, same]

end LeanerIR.Proofs.Denote.TableMemory
