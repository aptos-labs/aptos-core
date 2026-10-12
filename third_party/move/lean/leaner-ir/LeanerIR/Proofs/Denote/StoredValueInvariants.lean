-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.Resources
import LeanerIR.Proofs.Denote.TableReads

/-!
# Data invariants of external collection values

The declaration predicate states the authored invariant of a nominal value at
its resolved type arguments. The arguments matter even for phantom parameters:
two values with identical physical encodings can have different invariants.
Traversal follows the resolved native type, so invariants of a generic stored
value are not lost when its physical owner only contains a Table handle.
The predicate reads no memory. Nested Table contents are constrained in their
own collection slots; the handle itself does not quantify over other memories.
-/

namespace LeanerIR.Proofs.Denote.DataInvariant

def elements : RuntimeValue → List RuntimeValue
  | .vector values => values.toList
  | _ => []

mutual
def value (declared : StructHandle → NRow → RuntimeValue → Prop) : NTy → RuntimeValue → Prop
  | .struct source arguments fields, raw =>
      declared source arguments raw ∧ row declared fields raw 0
  | .enum source arguments names rows _, raw =>
      declared source arguments raw ∧ variant declared names rows raw
  | .tuple fields, raw => row declared fields raw 0
  | .vector element, raw => ∀ held ∈ elements raw, value declared element held
  | .ref element, raw => value declared element (raw.field 0) ∧ value declared element (raw.field 1)
  | _, _ => True

def row (declared : StructHandle → NRow → RuntimeValue → Prop) : NRow → RuntimeValue → Nat → Prop
  | .nil, _, _ => True
  | .cons type rest, raw, index =>
      value declared type (raw.field index) ∧ row declared rest raw (index + 1)

def variant (declared : StructHandle → NRow → RuntimeValue → Prop) :
    List String → NRows → RuntimeValue → Prop
  | name :: names, .cons fields rest, raw =>
      (raw.variant? = some name → row declared fields raw 0) ∧
        variant declared names rest raw
  | _, _, _ => True
end

/-- Primitive integer elements carry no declared nominal invariant. Resolve
this before traversing a potentially large stored vector expression. -/
@[lir_denote_norm↓] theorem integer_vector
    (declared : StructHandle → NRow → RuntimeValue → Prop) (width : Nat) (signed : Bool)
    (raw : RuntimeValue) : value declared (.vector (.int width signed)) raw := by
  intro held member
  trivial

/-- Nested primitive vectors likewise need no nominal-invariant traversal. -/
@[lir_denote_norm↓] theorem integer_vector_vector
    (declared : StructHandle → NRow → RuntimeValue → Prop) (width : Nat) (signed : Bool)
    (raw : RuntimeValue) : value declared (.vector (.vector (.int width signed))) raw := by
  intro held member
  exact integer_vector declared width signed held

/-- An external collection is unbounded. Its elements, rather than a Move
vector wrapper, carry the invariants of the collection's native element type. -/
def collection (declared : StructHandle → NRow → RuntimeValue → Prop)
    (type : NTy) (raw : RuntimeValue) : Prop :=
  ∀ held ∈ elements raw, value declared type held

/-- A declaration predicate lifted to both storage domains. Callers may keep
their existing global-resource predicate and use just the collection branch. -/
def withCollections (globals : ResourceType → RuntimeValue → Prop)
    (declared : StructHandle → NRow → RuntimeValue → Prop)
    (type : ResourceType) (raw : RuntimeValue) : Prop :=
  match type.kind with
  | .value => globals type raw
  | .collection => collection declared type.type raw

variable {unit : Validation.ValidatedUnit}

/-- Replacing a Table's external slot preserves memory invariants when every
key and value written satisfies its resolved data invariant. Other slots keep
their existing facts; no invariant is assumed of an unrelated observation. -/
theorem table_write
    {globals : ResourceType → RuntimeValue → Prop}
    {declared : StructHandle → NRow → RuntimeValue → Prop} {memory : Memory unit}
    (holds : MemoryInvariants (withCollections globals declared) memory)
    (owner key type : NTy) (handle : String)
    (contents : TableMemory.Contents unit key type)
    (keys : ∀ entry ∈ contents,
      value declared key (@NTy.encode (Carriers.runtime unit) key entry.1))
    (values : ∀ entry ∈ contents,
      value declared type (@NTy.encode (Carriers.runtime unit) type entry.2.1)) :
    MemoryInvariants (withCollections globals declared)
      (TableMemory.write memory owner key type handle contents) := by
  apply holds.set_some
  intro raw member
  change raw ∈ (contents.map
    (@NTy.encode (Carriers.runtime unit) (.tuple (.cons key (.cons type .nil))))).toList at member
  simp only [Array.toList_map, List.mem_map, Array.mem_toList_iff] at member
  obtain ⟨entry, member, rfl⟩ := member
  exact ⟨keys entry member, values entry member, trivial⟩

/-- A stored collection member satisfies its resolved element invariant. -/
theorem collection_member
    {globals : ResourceType → RuntimeValue → Prop}
    {declared : StructHandle → NRow → RuntimeValue → Prop} {memory : Memory unit}
    (holds : MemoryInvariants (withCollections globals declared) memory)
    (type : NTy) (arguments : NRow) (key : StorageKey)
    (contents : Array (@NTy.carrier (Carriers.runtime unit) type))
    (read : memory ⟨type, arguments, .collection⟩ key = some contents)
    (held : @NTy.carrier (Carriers.runtime unit) type) (member : held ∈ contents) :
    value declared type (@NTy.encode (Carriers.runtime unit) type held) := by
  have all := holds.read read
  exact all _ (by
    simp only [elements, Array.toList_map, List.mem_map,
      Array.mem_toList_iff]
    exact ⟨held, member, rfl⟩)

/-- Shared lookup carries the stored value's invariant, independently of key
equality and without assuming an invariant of its physical Table handle. -/
theorem table_lookup
    {globals : ResourceType → RuntimeValue → Prop}
    {declared : StructHandle → NRow → RuntimeValue → Prop} {memory : Memory unit}
    (holds : MemoryInvariants (withCollections globals declared) memory)
    (owner key type : NTy) (handle : String)
    (contents : TableMemory.Contents unit key type)
    (read : TableMemory.read memory owner key type handle = some contents)
    (query : @NTy.carrier (Carriers.runtime unit) key)
    (held : @NTy.carrier (Carriers.runtime unit) type)
    (found : TableMemory.lookup? key type contents query = some held) :
    value declared type (@NTy.encode (Carriers.runtime unit) type held) := by
  unfold TableMemory.lookup? at found
  obtain ⟨index, _, found⟩ := Option.bind_eq_some_iff.mp found
  obtain ⟨entry, selected, found⟩ := Option.bind_eq_some_iff.mp found
  cases Option.some.inj found
  have invariant := collection_member holds _ _ _ contents read entry
    (Array.mem_of_getElem? selected)
  simpa only [value, row, NTy.encode_tuple, HList.encode_cons, HList.encode_nil,
    RuntimeValue.field, List.getElem?_toArray, List.getElem?_cons_zero,
    List.getElem?_cons_succ, Option.getD_some] using invariant.2.1

/-- The invariant refers to the very snapshot returned by a Table read. It
does not assert anything about an independently chosen observation memory. -/
theorem table_get
    {globals : ResourceType → RuntimeValue → Prop}
    {declared : StructHandle → NRow → RuntimeValue → Prop} {memory : Memory unit}
    (holds : MemoryInvariants (withCollections globals declared) memory)
    (source : StructHandle) (key type : NTy) (fields : NRow)
    (table : @HList (Carriers.runtime unit) fields)
    (query : @NTy.carrier (Carriers.runtime unit) key)
    (registered : SnapshotValue.tableOwner unit source = true)
    (layout : TableMemory.handleFields fields = true)
    (present : (SnapshotValue.observe memory
      (.struct source (.cons key (.cons type .nil)) fields) table).hasKey
        (@NTy.encode (Carriers.runtime unit) key query) = true) :
    value declared type ((SnapshotValue.observe memory
      (.struct source (.cons key (.cons type .nil)) fields) table).getValue
        (@NTy.encode (Carriers.runtime unit) key query)).physical := by
  have lookup := SnapshotValue.lookup_observe_table memory source key type fields table
    query registered layout
  let owner := NTy.struct source (.cons key (.cons type .nil)) fields
  let handle := ((@NTy.encode (Carriers.runtime unit) owner table).field 0).asString
  change _ = ((TableMemory.read memory owner key type handle).bind
    (fun contents => TableMemory.lookup? key type contents query)).map
      (SnapshotValue.observe memory type) at lookup
  unfold SnapshotValue.Value.hasKey at present
  rw [lookup] at present
  cases read : TableMemory.read memory owner key type handle with
  | none => simp [read] at present
  | some contents =>
      cases found : TableMemory.lookup? key type contents query with
      | none => simp [read, found] at present
      | some held =>
          unfold SnapshotValue.Value.getValue
          rw [lookup]
          simp only [read, found, Option.bind_some, Option.map_some, Option.getD_some,
            SnapshotValue.physical_observe]
          exact table_lookup holds owner key type handle contents read query held found

/-- Membership of a raw snapshot key suffices: it selects an actual typed
entry, without reconstructing a native key or proving bounds on its encoding. -/
theorem table_get_raw
    {globals : ResourceType → RuntimeValue → Prop}
    {declared : StructHandle → NRow → RuntimeValue → Prop} {memory : Memory unit}
    (holds : MemoryInvariants (withCollections globals declared) memory)
    (source : StructHandle) (key type : NTy) (fields : NRow)
    (table : @HList (Carriers.runtime unit) fields)
    (query : RuntimeValue)
    (registered : SnapshotValue.tableOwner unit source = true)
    (layout : TableMemory.handleFields fields = true)
    (present : (SnapshotValue.observe memory
      (.struct source (.cons key (.cons type .nil)) fields) table).hasKey query = true) :
    value declared type ((SnapshotValue.observe memory
      (.struct source (.cons key (.cons type .nil)) fields) table).getValue query).physical := by
  rw [SnapshotValue.observe_table memory source key type fields table registered layout] at present ⊢
  dsimp only at present ⊢
  cases read : TableMemory.read memory
      (.struct source (.cons key (.cons type .nil)) fields) key type
      ((@NTy.encode (Carriers.runtime unit)
        (.struct source (.cons key (.cons type .nil)) fields) table).field 0).asString with
  | none => simp only [read, Option.map_none, SnapshotValue.Value.hasKey,
      SnapshotValue.Value.lookup?, Option.isSome_none, Bool.false_eq_true] at present
  | some contents =>
      simp only [read, Option.map_some, SnapshotValue.Value.hasKey,
        SnapshotValue.Value.getValue, SnapshotValue.Value.lookup?, List.find?_map,
        Function.comp_def, Option.bind_eq_bind] at present ⊢
      cases found : contents.toList.find? (fun entry =>
        @NTy.encode (Carriers.runtime unit) key entry.1 == query) with
      | none => simp [found] at present
      | some entry =>
          simp only [Option.map_some, Option.bind_some, Option.getD_some,
            SnapshotValue.physical_observe]
          have invariant := collection_member holds _ _ _ contents read entry
            (by simpa only [Array.mem_toList_iff] using List.mem_of_find?_eq_some found)
          simpa only [value, row, NTy.encode_tuple, HList.encode_cons, HList.encode_nil,
            RuntimeValue.field, List.getElem?_toArray, List.getElem?_cons_zero,
            List.getElem?_cons_succ, Option.getD_some] using invariant.2.1

end LeanerIR.Proofs.Denote.DataInvariant
