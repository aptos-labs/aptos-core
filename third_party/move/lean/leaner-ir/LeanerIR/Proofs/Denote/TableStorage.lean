-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.ReturnedStorage
import LeanerIR.Proofs.Denote.TableOperations

/-!
# Table storage operation agreement

Canonical owner/type keys let a single runtime slot update represent a logical
contents update across all caller and callee names of that resource. These laws
preserve both stores and allocation history; they do not assume type-table
interning is injective. Returned-storage laws also preserve observations of
other active loans. Native adapters still have to resolve the owning slot and
establish ownership, allocation freshness, and the native's argument layout.
-/

namespace LeanerIR.Proofs.Denote.TableMemory

variable {unit : Validation.ValidatedUnit}

theorem resourceOf_kind {namespaceId : NamespaceId} {typeId : TypeId}
    {contents : ResourceType} (named : resourceOf unit namespaceId typeId = some contents) :
    contents.kind = .collection := by
  unfold resourceOf at named
  simp only [bind, Option.bind] at named
  repeat (first | simp only at named | split at named | cases named) <;> try rfl

theorem runtimeResourceOf_kind {namespaceId : NamespaceId} {typeId : TypeId}
    {contents : ResourceType}
    (named : runtimeResourceOf unit namespaceId typeId = some contents) :
    contents.kind = .collection := resourceOf_kind (runtimeResourceOf_sound named).1

theorem runtimeResourceOf_ne_allocation (namespaceId : NamespaceId) (typeId : TypeId) :
    runtimeResourceOf unit namespaceId typeId ≠ some allocationResource := by
  intro named
  have canonical := (runtimeResourceOf_sound named).2
  simp [canonicalAt, ownerNamespace?, allocationResource] at canonical

/-- Replacing one typed contents slot preserves the entire heap encoding. -/
theorem Encodes.insert {memory : Memory unit} {heap : GlobalMap}
    (encoded : Encodes unit memory heap) {namespaceId : NamespaceId} {typeId : TypeId}
    {contents : ResourceType} (named : runtimeResourceOf unit namespaceId typeId = some contents)
    (key : StorageKey) (value : contents.carrier unit) :
    Encodes unit (memory.set contents key (some value))
      (heap.insert ⟨namespaceId, typeId, key⟩ (contents.encode value)) := by
  constructor
  · exact encoded.1.insert _ _
  · intro otherNamespace otherType otherKey
    by_cases sameName : otherNamespace = namespaceId ∧ otherType = typeId
    · obtain ⟨rfl, rfl⟩ := sameName
      rw [named]
      by_cases sameKey : otherKey = key
      · subst otherKey
        simp
      · rw [GlobalMap.lookup_insert_other _ _ _ _ (by simp [sameKey]), encoded.2, named]
        simp [sameKey]
    · have distinctSlot : (⟨otherNamespace, otherType, otherKey⟩ : GlobalKey) ≠
          ⟨namespaceId, typeId, key⟩ := by
        intro equal
        cases equal
        exact sameName ⟨rfl, rfl⟩
      rw [GlobalMap.lookup_insert_other _ _ _ _ distinctSlot, encoded.2]
      cases other : runtimeResourceOf unit otherNamespace otherType with
      | none => rfl
      | some resource =>
          have distinctResource : resource ≠ contents := by
            intro equal
            subst resource
            exact sameName (runtimeResourceOf_unique other named)
          simp [Memory.set_other_resource memory _ _ distinctResource]

/-- Retiring one contents slot preserves every other logical Table resource. -/
theorem Encodes.erase {memory : Memory unit} {heap : GlobalMap}
    (encoded : Encodes unit memory heap) {namespaceId : NamespaceId} {typeId : TypeId}
    {contents : ResourceType} (named : runtimeResourceOf unit namespaceId typeId = some contents)
    (key : StorageKey) :
    Encodes unit (memory.set contents key none)
      (heap.erase ⟨namespaceId, typeId, key⟩) := by
  constructor
  · exact encoded.1.erase _
  · intro otherNamespace otherType otherKey
    by_cases sameName : otherNamespace = namespaceId ∧ otherType = typeId
    · obtain ⟨rfl, rfl⟩ := sameName
      rw [named]
      by_cases sameKey : otherKey = key
      · subst otherKey
        simp
      · rw [GlobalMap.lookup_erase_other _ _ _ (by simp [sameKey]), encoded.2, named]
        simp [sameKey]
    · have distinctSlot : (⟨otherNamespace, otherType, otherKey⟩ : GlobalKey) ≠
          ⟨namespaceId, typeId, key⟩ := by
        intro equal
        cases equal
        exact sameName ⟨rfl, rfl⟩
      rw [GlobalMap.lookup_erase_other _ _ _ distinctSlot, encoded.2]
      cases other : runtimeResourceOf unit otherNamespace otherType with
      | none => rfl
      | some resource =>
          have distinctResource : resource ≠ contents := by
            intro equal
            subst resource
            exact sameName (runtimeResourceOf_unique other named)
          simp [Memory.set_other_resource memory _ _ distinctResource]

/-- Changing a resource no Table slot names leaves the Table heap encoding
unchanged. Allocation history is such a resource. -/
theorem Encodes.set_other {memory : Memory unit} {heap : GlobalMap}
    (encoded : Encodes unit memory heap) (other : ResourceType)
    (unnamed : ∀ namespaceId typeId, runtimeResourceOf unit namespaceId typeId ≠ some other)
    (key : StorageKey) (value : Option (other.carrier unit)) :
    Encodes unit (memory.set other key value) heap := by
  refine ⟨encoded.1, ?_⟩
  intro namespaceId typeId query
  rw [encoded.2]
  cases named : runtimeResourceOf unit namespaceId typeId with
  | none => rfl
  | some contents =>
      have distinct : contents ≠ other := by
        intro equal
        subst contents
        exact unnamed namespaceId typeId named
      simp [Memory.set_other_resource memory key value distinct]

theorem EncodesStorage.insert {memory : Memory unit} {storage : NativeTableStorage}
    (encoded : EncodesStorage unit memory storage) {namespaceId : NamespaceId} {typeId : TypeId}
    {contents : ResourceType} (named : runtimeResourceOf unit namespaceId typeId = some contents)
    (key : StorageKey) (value : contents.carrier unit) :
    EncodesStorage unit (memory.set contents key (some value))
      { storage with contents := storage.contents.insert ⟨namespaceId, typeId, key⟩ (contents.encode value) } := by
  have distinct : allocationResource ≠ contents := by
    intro equal
    subst contents
    exact runtimeResourceOf_ne_allocation namespaceId typeId named
  refine ⟨encoded.1.insert named key value, ?_⟩
  intro query
  simpa only [Memory.set_other_resource memory key (some value) distinct] using encoded.2 query

theorem EncodesStorage.erase {memory : Memory unit} {storage : NativeTableStorage}
    (encoded : EncodesStorage unit memory storage) {namespaceId : NamespaceId} {typeId : TypeId}
    {contents : ResourceType} (named : runtimeResourceOf unit namespaceId typeId = some contents)
    (key : StorageKey) :
    EncodesStorage unit (memory.set contents key none)
      { storage with contents := storage.contents.erase ⟨namespaceId, typeId, key⟩ } := by
  have distinct : allocationResource ≠ contents := by
    intro equal
    subst contents
    exact runtimeResourceOf_ne_allocation namespaceId typeId named
  refine ⟨encoded.1.erase named key, ?_⟩
  intro query
  simpa only [Memory.set_other_resource memory key none distinct] using encoded.2 query

theorem EncodesStorage.set_allocated {memory : Memory unit} {storage : NativeTableStorage}
    (encoded : EncodesStorage unit memory storage) (allocated : Array String) :
    EncodesStorage unit
      (memory.set allocationResource .unit (some (encodeAllocated allocated)))
      { storage with allocated } := by
  refine ⟨encoded.1.set_other allocationResource runtimeResourceOf_ne_allocation _ _, ?_⟩
  intro query
  by_cases atHistory : query = .unit
  · subst query
    simp
  · simpa [atHistory] using encoded.2 query

end LeanerIR.Proofs.Denote.TableMemory

namespace LeanerIR.Proofs.Denote

variable {unit : Validation.ValidatedUnit}

/-- Ordinary globals use value resources, so native collection updates cannot
change their encoding, even if the physical keys have the same spelling. -/
theorem Encodes.set_collection {memory : Memory unit} {globals : GlobalMap}
    (encoded : Encodes unit memory globals) (resource : ResourceType)
    (collection : resource.kind = .collection) (key : StorageKey)
    (value : Option (resource.carrier unit)) :
    Encodes unit (memory.set resource key value) globals := by
  refine ⟨encoded.1, ?_⟩
  intro namespaceId typeId query
  rw [encoded.2]
  cases named : runtimeResourceOf unit namespaceId typeId with
  | none => rfl
  | some other =>
      have distinct : other ≠ resource := by
        intro equal
        have ordinary := runtimeResourceOf_kind named
        rw [equal, collection] at ordinary
        cases ordinary
      simp [Memory.set_other_resource memory key value distinct]

theorem StorageEncodes.insert_table {memory : Memory unit} {state : RuntimeState}
    (encoded : StorageEncodes unit memory state) {namespaceId : NamespaceId} {typeId : TypeId}
    {contents : ResourceType}
    (named : TableMemory.runtimeResourceOf unit namespaceId typeId = some contents)
    (key : StorageKey) (value : contents.carrier unit) :
    StorageEncodes unit (memory.set contents key (some value))
      { state with tables.contents :=
        state.tables.contents.insert ⟨namespaceId, typeId, key⟩ (contents.encode value) } := by
  exact ⟨encoded.globals.set_collection contents (TableMemory.runtimeResourceOf_kind named) key _,
    encoded.tables.insert named key value⟩

theorem StorageEncodes.erase_table {memory : Memory unit} {state : RuntimeState}
    (encoded : StorageEncodes unit memory state) {namespaceId : NamespaceId} {typeId : TypeId}
    {contents : ResourceType}
    (named : TableMemory.runtimeResourceOf unit namespaceId typeId = some contents)
    (key : StorageKey) :
    StorageEncodes unit (memory.set contents key none)
      { state with tables.contents := state.tables.contents.erase ⟨namespaceId, typeId, key⟩ } := by
  exact ⟨encoded.globals.set_collection contents (TableMemory.runtimeResourceOf_kind named) key _,
    encoded.tables.erase named key⟩

theorem StorageEncodes.set_allocated {memory : Memory unit} {state : RuntimeState}
    (encoded : StorageEncodes unit memory state) (allocated : Array String) :
    StorageEncodes unit
      (memory.set TableMemory.allocationResource .unit (some (TableMemory.encodeAllocated allocated)))
      { state with tables.allocated := allocated } := by
  exact ⟨encoded.globals.set_collection TableMemory.allocationResource rfl _ _,
    encoded.tables.set_allocated allocated⟩

theorem StorageEncodes.table_lookup {memory : Memory unit} {state : RuntimeState}
    (encoded : StorageEncodes unit memory state) {namespaceId : NamespaceId} {typeId : TypeId}
    {contents : ResourceType}
    (named : TableMemory.runtimeResourceOf unit namespaceId typeId = some contents)
    (key : StorageKey) :
    state.tables.contents.lookup ⟨namespaceId, typeId, key⟩ =
      (memory contents key).map (contents.encode (unit := unit)) := by
  simpa only [named] using encoded.tables.1.2 namespaceId typeId key

theorem StorageEncodes.table_entries {memory : Memory unit} {state : RuntimeState}
    (encoded : StorageEncodes unit memory state) {namespaceId : NamespaceId} {typeId : TypeId}
    (owner key value : NTy)
    (named : TableMemory.runtimeResourceOf unit namespaceId typeId =
      some (TableMemory.resource owner key value)) (handle : String)
    (contents : TableMemory.Contents unit key value)
    (stored : TableMemory.read memory owner key value handle = some contents) :
    state.tables.contents.lookup (SemanticOperations.tableSlot namespaceId typeId handle) =
      some ((TableMemory.resource owner key value).encode (unit := unit) contents) := by
  change memory (TableMemory.resource owner key value) (.address handle) = some contents at stored
  have found := encoded.table_lookup named (.address handle)
  rw [stored] at found
  exact found

/-- A successful native insertion yields typed updated contents and preserves
the encoding of both stores and the allocation history. -/
theorem StorageEncodes.add_table {memory : Memory unit} {state final : RuntimeState}
    (encoded : StorageEncodes unit memory state) {namespaceId : NamespaceId} {typeId : TypeId}
    (owner key value : NTy)
    (named : TableMemory.runtimeResourceOf unit namespaceId typeId =
      some (TableMemory.resource owner key value)) (handle : String)
    (contents : TableMemory.Contents unit key value)
    (stored : TableMemory.read memory owner key value handle = some contents)
    (newKey : @NTy.carrier (Carriers.runtime unit) key)
    (newValue : @NTy.carrier (Carriers.runtime unit) value)
    (added : SemanticOperations.addTableEntry? state
      (SemanticOperations.tableSlot namespaceId typeId handle)
      (@NTy.encode (Carriers.runtime unit) key newKey)
      (@NTy.encode (Carriers.runtime unit) value newValue) = some final) :
    ∃ updated, TableMemory.add? key value contents newKey newValue = some updated ∧
      StorageEncodes unit (TableMemory.write memory owner key value handle updated) final := by
  have agrees := TableMemory.add?_agrees owner key value contents newKey newValue state _
    (encoded.table_entries owner key value named handle contents stored)
  rw [added] at agrees
  obtain ⟨updated, changed, equal⟩ := Option.map_eq_some_iff.mp agrees.symm
  subst final
  exact ⟨updated, changed, by
    simpa only [RuntimeState.writeLoanValue_table, SemanticOperations.tableSlot, TableMemory.write]
      using encoded.insert_table named (.address handle) updated⟩

/-- Native removal returns the encoded typed value from the selected entry
and preserves the full storage encoding after deleting that binding. -/
theorem StorageEncodes.remove_table {memory : Memory unit} {state final : RuntimeState}
    (encoded : StorageEncodes unit memory state) {namespaceId : NamespaceId} {typeId : TypeId}
    (owner key value : NTy)
    (named : TableMemory.runtimeResourceOf unit namespaceId typeId =
      some (TableMemory.resource owner key value)) (handle : String)
    (contents : TableMemory.Contents unit key value)
    (stored : TableMemory.read memory owner key value handle = some contents)
    (query : @NTy.carrier (Carriers.runtime unit) key) (removed : RuntimeValue)
    (taken : SemanticOperations.removeTableEntry? state
      (SemanticOperations.tableSlot namespaceId typeId handle)
      (@NTy.encode (Carriers.runtime unit) key query) = some (final, removed)) :
    ∃ updated returned, TableMemory.remove? key value contents query = some (updated, returned) ∧
      removed = @NTy.encode (Carriers.runtime unit) value returned ∧
      StorageEncodes unit (TableMemory.write memory owner key value handle updated) final := by
  have agrees := TableMemory.remove?_agrees owner key value contents query state _
    (encoded.table_entries owner key value named handle contents stored)
  rw [taken] at agrees
  obtain ⟨⟨updated, returned⟩, changed, equal⟩ := Option.map_eq_some_iff.mp agrees.symm
  obtain ⟨rfl, rfl⟩ := Prod.mk.inj equal
  exact ⟨updated, returned, changed, rfl, by
    simpa only [RuntimeState.writeLoanValue_table, SemanticOperations.tableSlot, TableMemory.write]
      using encoded.insert_table named (.address handle) updated⟩

theorem StorageEncodes.retire_table {memory : Memory unit} {state final : RuntimeState}
    (encoded : StorageEncodes unit memory state) {namespaceId : NamespaceId} {typeId : TypeId}
    {contents : ResourceType}
    (named : TableMemory.runtimeResourceOf unit namespaceId typeId = some contents)
    (handle : String) (retired : SemanticOperations.retireEmptyTable? state
      (SemanticOperations.tableSlot namespaceId typeId handle) = some final) :
    StorageEncodes unit (memory.set contents (.address handle) none) final := by
  obtain ⟨_, rfl⟩ := SemanticOperations.retireEmptyTable?_eq_some.mp retired
  exact encoded.erase_table named (.address handle)

/-- Successful runtime allocation agrees with an empty typed contents slot
and the extended persistent history, while preserving all ordinary globals
and unrelated Tables. Freshness follows from the runtime success premise. -/
theorem StorageEncodes.allocate_table {memory : Memory unit} {state final : RuntimeState}
    (encoded : StorageEncodes unit memory state) {namespaceId : NamespaceId} {typeId : TypeId}
    (owner key value : NTy)
    (named : TableMemory.runtimeResourceOf unit namespaceId typeId =
      some (TableMemory.resource owner key value)) (handle : String)
    (allocated : SemanticOperations.allocateTableAt? state namespaceId typeId handle = some final) :
    StorageEncodes unit
      ((TableMemory.write memory owner key value handle #[]).set TableMemory.allocationResource
        .unit (some (TableMemory.encodeAllocated final.tables.allocated))) final := by
  obtain ⟨_, rfl⟩ := SemanticOperations.allocateTableAt?_eq_some.mp allocated
  have inserted := encoded.insert_table named (.address handle) #[]
  have recorded := inserted.set_allocated (state.tables.allocated.push handle)
  simpa [TableMemory.write, TableMemory.resource, ResourceType.encode,
    SemanticOperations.tableSlot] using recorded

/-- Replacing a Table slot with typed contents preserves the observation of
unrelated active loans. This is a storage law, not permission to overwrite a
borrowed owner: an executing native must also establish exclusive access. -/
theorem StorageEncodesReturned.insert_table {memory : Memory unit} {state : RuntimeState}
    {results : Array RuntimeValue} (encoded : StorageEncodesReturned unit memory state results)
    {namespaceId : NamespaceId} {typeId : TypeId} {contents : ResourceType}
    (named : TableMemory.runtimeResourceOf unit namespaceId typeId = some contents)
    (key : StorageKey) (value : contents.carrier unit) :
    StorageEncodesReturned unit (memory.set contents key (some value))
      { state with tables.contents :=
        state.tables.contents.insert ⟨namespaceId, typeId, key⟩ (contents.encode value) } results := by
  have sorted : state.tables.contents.Sorted :=
    (GlobalMap.sorted_resolveReturned_iff results _).mp encoded.tables.1.1
  have updated := StorageEncodes.insert_table encoded named key value
  simpa only [StorageEncodesReturned, RuntimeState.resolveReturned,
    GlobalMap.resolveReturned_insert results sorted,
    resolveReturnedBorrows_of_plain results (ResourceType.encode_plain contents value)] using updated

theorem StorageEncodesReturned.erase_table {memory : Memory unit} {state : RuntimeState}
    {results : Array RuntimeValue} (encoded : StorageEncodesReturned unit memory state results)
    {namespaceId : NamespaceId} {typeId : TypeId} {contents : ResourceType}
    (named : TableMemory.runtimeResourceOf unit namespaceId typeId = some contents)
    (key : StorageKey) :
    StorageEncodesReturned unit (memory.set contents key none)
      { state with tables.contents := state.tables.contents.erase ⟨namespaceId, typeId, key⟩ }
      results := by
  have sorted : state.tables.contents.Sorted :=
    (GlobalMap.sorted_resolveReturned_iff results _).mp encoded.tables.1.1
  have updated := StorageEncodes.erase_table encoded named key
  simpa only [StorageEncodesReturned, RuntimeState.resolveReturned,
    GlobalMap.resolveReturned_erase results sorted] using updated

theorem StorageEncodesReturned.set_allocated {memory : Memory unit} {state : RuntimeState}
    {results : Array RuntimeValue} (encoded : StorageEncodesReturned unit memory state results)
    (allocated : Array String) :
    StorageEncodesReturned unit
      (memory.set TableMemory.allocationResource .unit (some (TableMemory.encodeAllocated allocated)))
      { state with tables.allocated := allocated } results :=
  StorageEncodes.set_allocated encoded allocated

/-- Execute insertion at a plain Table slot while other owners may have
active loans. The raw-slot premise cannot be replaced by an observed lookup. -/
theorem StorageEncodesReturned.add_table {memory : Memory unit} {state final : RuntimeState}
    {results : Array RuntimeValue} (encoded : StorageEncodesReturned unit memory state results)
    {namespaceId : NamespaceId} {typeId : TypeId} (owner key value : NTy)
    (named : TableMemory.runtimeResourceOf unit namespaceId typeId =
      some (TableMemory.resource owner key value)) (handle : String)
    (contents : TableMemory.Contents unit key value)
    (stored : state.tables.contents.lookup (SemanticOperations.tableSlot namespaceId typeId handle) =
      some ((TableMemory.resource owner key value).encode (unit := unit) contents))
    (newKey : @NTy.carrier (Carriers.runtime unit) key)
    (newValue : @NTy.carrier (Carriers.runtime unit) value)
    (added : SemanticOperations.addTableEntry? state
      (SemanticOperations.tableSlot namespaceId typeId handle)
      (@NTy.encode (Carriers.runtime unit) key newKey)
      (@NTy.encode (Carriers.runtime unit) value newValue) = some final) :
    ∃ updated, TableMemory.add? key value contents newKey newValue = some updated ∧
      StorageEncodesReturned unit (TableMemory.write memory owner key value handle updated) final results := by
  have agrees := TableMemory.add?_agrees owner key value contents newKey newValue state _ stored
  rw [added] at agrees
  obtain ⟨updated, changed, equal⟩ := Option.map_eq_some_iff.mp agrees.symm
  subst final
  exact ⟨updated, changed, by
    simpa only [RuntimeState.writeLoanValue_table, SemanticOperations.tableSlot, TableMemory.write]
      using encoded.insert_table named (.address handle) updated⟩

theorem StorageEncodesReturned.remove_table {memory : Memory unit} {state final : RuntimeState}
    {results : Array RuntimeValue} (encoded : StorageEncodesReturned unit memory state results)
    {namespaceId : NamespaceId} {typeId : TypeId} (owner key value : NTy)
    (named : TableMemory.runtimeResourceOf unit namespaceId typeId =
      some (TableMemory.resource owner key value)) (handle : String)
    (contents : TableMemory.Contents unit key value)
    (stored : state.tables.contents.lookup (SemanticOperations.tableSlot namespaceId typeId handle) =
      some ((TableMemory.resource owner key value).encode (unit := unit) contents))
    (query : @NTy.carrier (Carriers.runtime unit) key) (removed : RuntimeValue)
    (taken : SemanticOperations.removeTableEntry? state
      (SemanticOperations.tableSlot namespaceId typeId handle)
      (@NTy.encode (Carriers.runtime unit) key query) = some (final, removed)) :
    ∃ updated returned, TableMemory.remove? key value contents query = some (updated, returned) ∧
      removed = @NTy.encode (Carriers.runtime unit) value returned ∧
      StorageEncodesReturned unit (TableMemory.write memory owner key value handle updated) final results := by
  have agrees := TableMemory.remove?_agrees owner key value contents query state _ stored
  rw [taken] at agrees
  obtain ⟨⟨updated, returned⟩, changed, equal⟩ := Option.map_eq_some_iff.mp agrees.symm
  obtain ⟨rfl, rfl⟩ := Prod.mk.inj equal
  exact ⟨updated, returned, changed, rfl, by
    simpa only [RuntimeState.writeLoanValue_table, SemanticOperations.tableSlot, TableMemory.write]
      using encoded.insert_table named (.address handle) updated⟩

theorem StorageEncodesReturned.retire_table {memory : Memory unit} {state final : RuntimeState}
    {results : Array RuntimeValue} (encoded : StorageEncodesReturned unit memory state results)
    {namespaceId : NamespaceId} {typeId : TypeId} {contents : ResourceType}
    (named : TableMemory.runtimeResourceOf unit namespaceId typeId = some contents)
    (handle : String) (retired : SemanticOperations.retireEmptyTable? state
      (SemanticOperations.tableSlot namespaceId typeId handle) = some final) :
    StorageEncodesReturned unit (memory.set contents (.address handle) none) final results := by
  obtain ⟨_, rfl⟩ := SemanticOperations.retireEmptyTable?_eq_some.mp retired
  exact encoded.erase_table named (.address handle)

theorem StorageEncodesReturned.allocate_table {memory : Memory unit} {state final : RuntimeState}
    {results : Array RuntimeValue} (encoded : StorageEncodesReturned unit memory state results)
    {namespaceId : NamespaceId} {typeId : TypeId} (owner key value : NTy)
    (named : TableMemory.runtimeResourceOf unit namespaceId typeId =
      some (TableMemory.resource owner key value)) (handle : String)
    (allocated : SemanticOperations.allocateTableAt? state namespaceId typeId handle = some final) :
    StorageEncodesReturned unit
      ((TableMemory.write memory owner key value handle #[]).set TableMemory.allocationResource
        .unit (some (TableMemory.encodeAllocated final.tables.allocated))) final results := by
  obtain ⟨_, rfl⟩ := SemanticOperations.allocateTableAt?_eq_some.mp allocated
  have inserted := encoded.insert_table named (.address handle) #[]
  have recorded := inserted.set_allocated (state.tables.allocated.push handle)
  simpa [TableMemory.write, TableMemory.resource, ResourceType.encode,
    SemanticOperations.tableSlot] using recorded

private theorem insert_overwrite {heap : GlobalMap} (sorted : heap.Sorted)
    (key : GlobalKey) (before after : RuntimeValue) :
    (heap.insert key before).insert key after = heap.insert key after := by
  apply GlobalMap.ext ((sorted.insert _ _).insert _ _) (sorted.insert _ _)
  intro query
  by_cases equal : query = key
  · subst query
    simp
  · simp [equal]

/-- Actual key-based borrowing followed by settling the entry's loan agrees
with the typed contents update. Other active loans are observed through the
same returned row throughout. The selected Table must have plain contents in
the executing state; agreement only after resolving its holes would not grant
permission to borrow it again. -/
theorem StorageEncodesReturned.borrow_table_reconcile
    {memory : Memory unit} {state : RuntimeState} {results : Array RuntimeValue}
    (encoded : StorageEncodesReturned unit memory state results)
    {namespaceId : NamespaceId} {typeId : TypeId} (owner key value : NTy)
    (named : TableMemory.runtimeResourceOf unit namespaceId typeId =
      some (TableMemory.resource owner key value)) (handle : String)
    (contents : TableMemory.Contents unit key value)
    (stored : state.tables.contents.lookup (SemanticOperations.tableSlot namespaceId typeId handle) =
      some ((TableMemory.resource owner key value).encode (unit := unit) contents))
    (query : @NTy.carrier (Carriers.runtime unit) key)
    (readValue writtenValue : @NTy.carrier (Carriers.runtime unit) value)
    (found : TableMemory.lookup? key value contents query = some readValue) :
    ∃ (focus : TableMemory.EntryFocus unit key value) (borrowed : RuntimeState),
      focus.entryKey = query ∧ contents = focus.fill readValue ∧
      SemanticOperations.borrowTableEntry? state
        (SemanticOperations.tableSlot namespaceId typeId handle)
        (@NTy.encode (Carriers.runtime unit) key query) =
          some (borrowed, .borrow state.nextLoan (@NTy.encode (Carriers.runtime unit) value readValue)) ∧
      StorageEncodesReturned unit
        (TableMemory.write memory owner key value handle (focus.fill writtenValue))
        (SemanticOperations.applyWriteBack {} borrowed state.nextLoan
          (@NTy.encode (Carriers.runtime unit) value writtenValue)).2 results := by
  obtain ⟨focus, selected, filled, borrowed⟩ :=
    TableMemory.lookup?_borrow owner key value contents query readValue state _ stored found
  refine ⟨focus, _, selected, filled, borrowed, ?_⟩
  have settled := focus.reconcile owner { state with nextLoan := state.nextLoan + 1 }
    (SemanticOperations.tableSlot namespaceId typeId handle) state.nextLoan writtenValue
  rw [settled]
  have sorted : state.tables.contents.Sorted :=
    (GlobalMap.sorted_resolveReturned_iff results _).mp encoded.tables.1.1
  rw [insert_overwrite sorted]
  have updated := encoded.insert_table named (.address handle) (focus.fill writtenValue)
  exact ⟨updated.globals, updated.tables⟩

end LeanerIR.Proofs.Denote
