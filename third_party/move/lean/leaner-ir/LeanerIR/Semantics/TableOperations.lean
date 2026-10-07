-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.TableLoans

/-!
# Native Table storage operations

These operations work on resolved Table slots, independently of the owner's
physical handle/length fields. The native adapter must resolve those slots and
establish typed, distinct entries and ownership before calling them. A failed
selection is `none`; mapping it to a native abort or an invalid representation
is the adapter's responsibility.

Allocation takes a proposed handle from the allocator. It checks freshness
against persistent history and every currently stored Table, including other
owner/type instances. This does not implement or assume injectivity of the VM's
hash-based handle generator. Retiring a Table never forgets its allocation.
-/

namespace LeanerIR.SemanticOperations

/-- A resolved native Table's storage key. -/
def tableSlot (namespaceId : NamespaceId) (typeId : TypeId) (handle : String) : GlobalKey :=
  ⟨namespaceId, typeId, .address handle⟩

/-- A handle absent from both the history and all live native contents.
The second check also protects callers importing a pre-existing Table heap. -/
def tableHandleFresh (state : RuntimeState) (handle : String) : Bool :=
  !state.tables.allocated.contains handle &&
    !state.tables.contents.entries.any (fun slot => slot.key.key = .address handle)

/-- Install an empty Table at a fresh proposed identity. -/
def allocateTableAt? (state : RuntimeState) (namespaceId : NamespaceId)
    (typeId : TypeId) (handle : String) : Option RuntimeState := do
  guard (tableHandleFresh state handle)
  some { state with tables := {
    contents := state.tables.contents.insert (tableSlot namespaceId typeId handle) (.vector #[])
    allocated := state.tables.allocated.push handle } }

/-- Append a new binding. The native Table has no total-entry-count limit;
a wrapper caching a `u64` length must check its own arithmetic separately. -/
def addTableEntry? (state : RuntimeState) (slot : GlobalKey) (key value : RuntimeValue) :
    Option RuntimeState := do
  let .vector entries ← state.tables.contents.lookup slot | none
  guard ((tableEntryIndex? (.vector entries) key).isNone)
  some (state.writeLoanValue (.table slot) (.vector (entries.push (.tuple #[key, value]))))

/-- Remove the selected binding and return its value. Table entries have no
public enumeration order; this preserves the remaining entries' internal order. -/
def removeTableEntry? (state : RuntimeState) (slot : GlobalKey) (key : RuntimeValue) :
    Option (RuntimeState × RuntimeValue) := do
  let .vector entries ← state.tables.contents.lookup slot | none
  let index ← tableEntryIndex? (.vector entries) key
  let entry ← entries[index]?
  let (_, value) ← tableEntry? entry
  some (state.writeLoanValue (.table slot) (.vector (entries.eraseIdxIfInBounds index)), value)

/-- Retire an empty Table's contents, retaining its identity in history.
The public wrapper supplies/checks emptiness; the production destroy native
itself does not perform an entry-count check. -/
def retireEmptyTable? (state : RuntimeState) (slot : GlobalKey) : Option RuntimeState := do
  let .vector entries ← state.tables.contents.lookup slot | none
  guard entries.isEmpty
  some { state with tables.contents := state.tables.contents.erase slot }

theorem tableHandleFresh_iff (state : RuntimeState) (handle : String) :
    tableHandleFresh state handle = true ↔
      handle ∉ state.tables.allocated ∧
      ∀ slot ∈ state.tables.contents.entries, slot.key.key ≠ .address handle := by
  simp [tableHandleFresh, Bool.and_eq_true, ← Array.any_toList, List.any_eq_false]

/-- Allocation succeeds exactly under the freshness check and records the
new identity while installing an empty contents slot. -/
theorem allocateTableAt?_eq_some {state final : RuntimeState}
    {namespaceId : NamespaceId} {typeId : TypeId} {handle : String} :
    allocateTableAt? state namespaceId typeId handle = some final ↔
      tableHandleFresh state handle = true ∧ final = { state with tables := {
        contents := state.tables.contents.insert (tableSlot namespaceId typeId handle) (.vector #[])
        allocated := state.tables.allocated.push handle } } := by
  by_cases fresh : tableHandleFresh state handle = true
  · simp [allocateTableAt?, fresh, guard, eq_comm]
  · simp [allocateTableAt?, fresh, guard, failure]

theorem allocateTableAt?_fresh {state final : RuntimeState}
    {namespaceId : NamespaceId} {typeId : TypeId} {handle : String}
    (allocated : allocateTableAt? state namespaceId typeId handle = some final) :
    handle ∉ state.tables.allocated ∧
      ∀ slot ∈ state.tables.contents.entries, slot.key.key ≠ .address handle :=
  (tableHandleFresh_iff state handle).mp (allocateTableAt?_eq_some.mp allocated).1

theorem allocateTableAt?_lookup {state final : RuntimeState}
    {namespaceId : NamespaceId} {typeId : TypeId} {handle : String}
    (allocated : allocateTableAt? state namespaceId typeId handle = some final) :
    final.tables.contents.lookup (tableSlot namespaceId typeId handle) = some (.vector #[]) := by
  rcases allocateTableAt?_eq_some.mp allocated with ⟨_, rfl⟩
  exact GlobalMap.lookup_insert_self _ _ _

theorem allocateTableAt?_history {state final : RuntimeState}
    {namespaceId : NamespaceId} {typeId : TypeId} {handle : String}
    (allocated : allocateTableAt? state namespaceId typeId handle = some final) :
    final.tables.allocated = state.tables.allocated.push handle :=
  congrArg (·.tables.allocated) (allocateTableAt?_eq_some.mp allocated).2

theorem allocateTableAt?_other {state final : RuntimeState}
    {namespaceId : NamespaceId} {typeId : TypeId} {handle : String}
    (allocated : allocateTableAt? state namespaceId typeId handle = some final)
    (other : GlobalKey) (distinct : other ≠ tableSlot namespaceId typeId handle) :
    final.tables.contents.lookup other = state.tables.contents.lookup other := by
  rcases allocateTableAt?_eq_some.mp allocated with ⟨_, rfl⟩
  exact GlobalMap.lookup_insert_other _ _ _ _ distinct

/-- Registering one identity prevents its allocation again, even under a
different owner/type instance. -/
theorem allocateTableAt?_no_reuse {state final : RuntimeState}
    {namespaceId : NamespaceId} {typeId : TypeId} {handle : String}
    (allocated : allocateTableAt? state namespaceId typeId handle = some final)
    (otherNamespace : NamespaceId) (otherType : TypeId) :
    allocateTableAt? final otherNamespace otherType handle = none := by
  simp [allocateTableAt?, tableHandleFresh, allocateTableAt?_history allocated, guard, failure]

/-- Exact insertion behavior, including the absent-key precondition. -/
theorem addTableEntry?_eq_some {state final : RuntimeState} {slot : GlobalKey}
    {key value : RuntimeValue} :
    addTableEntry? state slot key value = some final ↔
      ∃ entries, state.tables.contents.lookup slot = some (.vector entries) ∧
        tableEntryIndex? (.vector entries) key = none ∧
        final = state.writeLoanValue (.table slot)
          (.vector (entries.push (.tuple #[key, value]))) := by
  cases stored : state.tables.contents.lookup slot with
  | none => simp [addTableEntry?, stored]
  | some contents =>
      cases contents <;> simp only [addTableEntry?, stored]
      all_goals try simp
      case vector entries =>
        cases found : tableEntryIndex? (.vector entries) key <;>
          simp [guard, failure, eq_comm]

/-- Exact removal behavior: the value returned belongs to the queried key,
and precisely that array entry is removed. -/
theorem removeTableEntry?_eq_some {state final : RuntimeState} {slot : GlobalKey}
    {key result : RuntimeValue} :
    removeTableEntry? state slot key = some (final, result) ↔
      ∃ entries index, state.tables.contents.lookup slot = some (.vector entries) ∧
        tableEntryIndex? (.vector entries) key = some index ∧
        entries[index]? = some (.tuple #[key, result]) ∧
        final = state.writeLoanValue (.table slot) (.vector (entries.eraseIdxIfInBounds index)) := by
  constructor
  · intro removed
    unfold removeTableEntry? at removed
    obtain ⟨contents, stored, removed⟩ := Option.bind_eq_some_iff.mp removed
    cases contents <;> simp only at removed <;> try contradiction
    case vector entries =>
      simp only [Option.bind_eq_bind, Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at removed
      obtain ⟨index, found, entry, selected, ⟨storedKey, value⟩, decoded, equal, returned⟩ := removed
      have matched := Array.of_findIdx?_eq_some found
      simp [selected, decoded] at matched
      subst storedKey
      dsimp only at returned
      subst value
      exact ⟨entries, index, stored, found,
        selected.trans (congrArg some (tableEntry?_sound decoded)), equal.symm⟩
  · rintro ⟨entries, index, stored, found, selected, rfl⟩
    simp [removeTableEntry?, stored, found, selected, tableEntry?]

/-- Retiring a Table requires a present, empty contents slot. -/
theorem retireEmptyTable?_eq_some {state final : RuntimeState} {slot : GlobalKey} :
    retireEmptyTable? state slot = some final ↔
      state.tables.contents.lookup slot = some (.vector #[]) ∧
        final = { state with tables.contents := state.tables.contents.erase slot } := by
  cases stored : state.tables.contents.lookup slot with
  | none => simp [retireEmptyTable?, stored]
  | some contents =>
      cases contents <;> simp only [retireEmptyTable?, stored]
      all_goals try simp
      case vector entries =>
        by_cases empty : entries = #[]
        · subst entries
          simp [guard, eq_comm]
        · simp [empty, guard, failure]

/-- A native contents operation leaves globals and all loan bookkeeping alone.
The operation-specific laws additionally constrain the affected Table slots. -/
def SameOutsideTables (initial final : RuntimeState) : Prop :=
  final.globals = initial.globals ∧ final.storageLoans = initial.storageLoans ∧
    final.nextLoan = initial.nextLoan ∧ final.pending = initial.pending

theorem SameOutsideTables.discipline
    (same : SameOutsideTables initial final) : LoanDiscipline initial final :=
  LoanDiscipline.of_eq same.2.1 (Nat.le_of_eq same.2.2.1.symm)

theorem allocateTableAt?_frame {state final : RuntimeState}
    {namespaceId : NamespaceId} {typeId : TypeId} {handle : String}
    (allocated : allocateTableAt? state namespaceId typeId handle = some final) :
    SameOutsideTables state final := by
  rcases allocateTableAt?_eq_some.mp allocated with ⟨_, rfl⟩
  exact ⟨rfl, rfl, rfl, rfl⟩

theorem addTableEntry?_frame {state final : RuntimeState} {slot : GlobalKey}
    {key value : RuntimeValue} (added : addTableEntry? state slot key value = some final) :
    SameOutsideTables state final ∧ final.tables.allocated = state.tables.allocated ∧
      ∀ other, other ≠ slot → final.tables.contents.lookup other = state.tables.contents.lookup other := by
  rcases addTableEntry?_eq_some.mp added with ⟨entries, _, _, rfl⟩
  exact ⟨⟨rfl, rfl, rfl, rfl⟩, rfl, fun other distinct =>
    GlobalMap.lookup_insert_other _ _ other _ distinct⟩

theorem removeTableEntry?_frame {state final : RuntimeState} {slot : GlobalKey}
    {key result : RuntimeValue}
    (removed : removeTableEntry? state slot key = some (final, result)) :
    SameOutsideTables state final ∧ final.tables.allocated = state.tables.allocated ∧
      ∀ other, other ≠ slot → final.tables.contents.lookup other = state.tables.contents.lookup other := by
  rcases removeTableEntry?_eq_some.mp removed with ⟨entries, index, _, _, _, rfl⟩
  exact ⟨⟨rfl, rfl, rfl, rfl⟩, rfl, fun other distinct =>
    GlobalMap.lookup_insert_other _ _ other _ distinct⟩

theorem retireEmptyTable?_frame {state final : RuntimeState} {slot : GlobalKey}
    (retired : retireEmptyTable? state slot = some final) :
    SameOutsideTables state final ∧ final.tables.allocated = state.tables.allocated ∧
      ∀ other, other ≠ slot → final.tables.contents.lookup other = state.tables.contents.lookup other := by
  rcases retireEmptyTable?_eq_some.mp retired with ⟨_, rfl⟩
  exact ⟨⟨rfl, rfl, rfl, rfl⟩, rfl, fun other distinct =>
    GlobalMap.lookup_erase_other _ _ other distinct⟩

theorem retireEmptyTable?_lookup {state final : RuntimeState} {slot : GlobalKey}
    (retired : retireEmptyTable? state slot = some final) :
    final.tables.contents.lookup slot = none := by
  rcases retireEmptyTable?_eq_some.mp retired with ⟨_, rfl⟩
  exact GlobalMap.lookup_erase_self _ _

/-- Destroying an allocated Table does not make its identity fresh again. -/
theorem retireEmptyTable?_no_reuse {state final : RuntimeState} {slot : GlobalKey}
    (retired : retireEmptyTable? state slot = some final)
    {handle : String} (allocated : handle ∈ state.tables.allocated)
    (namespaceId : NamespaceId) (typeId : TypeId) :
    allocateTableAt? final namespaceId typeId handle = none := by
  simp [allocateTableAt?, tableHandleFresh, (retireEmptyTable?_frame retired).2.1,
    allocated, guard, failure]

theorem allocateTableAt?_sorted {state final : RuntimeState}
    {namespaceId : NamespaceId} {typeId : TypeId} {handle : String}
    (allocated : allocateTableAt? state namespaceId typeId handle = some final)
    (sorted : state.tables.contents.Sorted) : final.tables.contents.Sorted := by
  rcases allocateTableAt?_eq_some.mp allocated with ⟨_, rfl⟩
  exact sorted.insert _ _

theorem addTableEntry?_sorted {state final : RuntimeState} {slot : GlobalKey}
    {key value : RuntimeValue} (added : addTableEntry? state slot key value = some final)
    (sorted : state.tables.contents.Sorted) : final.tables.contents.Sorted := by
  rcases addTableEntry?_eq_some.mp added with ⟨entries, _, _, rfl⟩
  exact sorted.insert _ _

theorem removeTableEntry?_sorted {state final : RuntimeState} {slot : GlobalKey}
    {key result : RuntimeValue}
    (removed : removeTableEntry? state slot key = some (final, result))
    (sorted : state.tables.contents.Sorted) : final.tables.contents.Sorted := by
  rcases removeTableEntry?_eq_some.mp removed with ⟨entries, index, _, _, _, rfl⟩
  exact sorted.insert _ _

theorem retireEmptyTable?_sorted {state final : RuntimeState} {slot : GlobalKey}
    (retired : retireEmptyTable? state slot = some final)
    (sorted : state.tables.contents.Sorted) : final.tables.contents.Sorted := by
  rcases retireEmptyTable?_eq_some.mp retired with ⟨_, rfl⟩
  exact sorted.erase _

private theorem tableLookup_mirror_plain
    (shifted : StateShifted offset frontier inert inert' state state₂)
    (stored : state.tables.contents.lookup slot = some contents) (plain : Plain contents) :
    state₂.tables.contents.lookup slot = some contents := by
  have lookup := shifted.loanValue (.table slot)
  change state₂.tables.contents.lookup slot =
    (state.tables.contents.lookup slot).map (·.shift offset) at lookup
  simpa only [stored, Option.map_some, RuntimeValue.shift_of_plain offset plain] using lookup

private theorem tableEntries_plain {entries : Array RuntimeValue}
    (plain : Plain (.vector entries)) : ∀ entry ∈ entries, Plain entry := by
  cases plain
  assumption

private theorem tableEntries_erase_plain {entries : Array RuntimeValue}
    (plain : Plain (.vector entries)) (index : Nat) :
    Plain (.vector (entries.eraseIdxIfInBounds index)) := by
  refine .vector _ fun entry member => tableEntries_plain plain entry ?_
  have listed := Array.mem_toList_iff.mpr member
  rw [Array.toList_eraseIdxIfInBounds] at listed
  exact Array.mem_toList_iff.mp (List.mem_of_mem_eraseIdx listed)

theorem tableHandleFresh_mirror
    (shifted : StateShifted offset frontier inert inert' state state₂) (handle : String) :
    tableHandleFresh state₂ handle = tableHandleFresh state handle := by
  simp [tableHandleFresh, shifted.tables, NativeTableStorage.shift, GlobalMap.shift]

theorem allocateTableAt?_mirror
    (shifted : StateShifted offset frontier inert inert' state state₂)
    (allocated : allocateTableAt? state namespaceId typeId handle = some final) :
    ∃ final₂, allocateTableAt? state₂ namespaceId typeId handle = some final₂ ∧
      StateShifted offset frontier inert inert' final final₂ := by
  rcases allocateTableAt?_eq_some.mp allocated with ⟨fresh, rfl⟩
  let final₂ : RuntimeState := { state₂ with tables := {
    contents := state₂.tables.contents.insert (tableSlot namespaceId typeId handle) (.vector #[])
    allocated := state₂.tables.allocated.push handle } }
  refine ⟨final₂, allocateTableAt?_eq_some.mpr ⟨?_, rfl⟩, ?_⟩
  · rw [tableHandleFresh_mirror shifted]
    exact fresh
  · exact { shifted with
      tables := by simp [final₂, shifted.tables, NativeTableStorage.shift,
        GlobalMap.insert_shift]
      tablesAbove := GlobalMap.insert_above shifted.tablesAbove
        (RuntimeValue.above_of_plain frontier (.vector _ fun _ member => by simp at member)) }

theorem addTableEntry?_mirror
    (shifted : StateShifted offset frontier inert inert' state state₂)
    (added : addTableEntry? state slot key value = some final)
    (contentsPlain : ∀ contents, state.tables.contents.lookup slot = some contents → Plain contents)
    (keyPlain : Plain key) (valuePlain : Plain value) :
    ∃ final₂, addTableEntry? state₂ slot (key.shift offset) (value.shift offset) = some final₂ ∧
      StateShifted offset frontier inert inert' final final₂ := by
  rcases addTableEntry?_eq_some.mp added with ⟨entries, stored, missing, rfl⟩
  have plain := contentsPlain _ stored
  have stored₂ := tableLookup_mirror_plain shifted stored plain
  let updated := RuntimeValue.vector (entries.push (.tuple #[key, value]))
  have updatedPlain : Plain updated := by
    refine .vector _ fun entry member => ?_
    rcases Array.mem_push.mp member with member | rfl
    · exact tableEntries_plain plain entry member
    · refine .tuple _ fun part member => ?_
      simp only [List.mem_toArray, List.mem_cons, List.not_mem_nil, or_false] at member
      rcases member with rfl | rfl
      · exact keyPlain
      · exact valuePlain
  refine ⟨state₂.writeLoanValue (.table slot) updated, ?_, ?_⟩
  · rw [RuntimeValue.shift_of_plain offset keyPlain, RuntimeValue.shift_of_plain offset valuePlain]
    exact addTableEntry?_eq_some.mpr ⟨entries, stored₂, missing, rfl⟩
  · have mirror := shifted.writeLoanValue (.table slot)
      (RuntimeValue.above_of_plain frontier updatedPlain)
    simpa only [RuntimeValue.shift_of_plain offset updatedPlain] using mirror

theorem removeTableEntry?_mirror
    (shifted : StateShifted offset frontier inert inert' state state₂)
    (removed : removeTableEntry? state slot key = some (final, result))
    (contentsPlain : ∀ contents, state.tables.contents.lookup slot = some contents → Plain contents)
    (keyPlain : Plain key) :
    ∃ final₂, removeTableEntry? state₂ slot (key.shift offset) = some (final₂, result.shift offset) ∧
      StateShifted offset frontier inert inert' final final₂ := by
  rcases removeTableEntry?_eq_some.mp removed with ⟨entries, index, stored, found, selected, rfl⟩
  have plain := contentsPlain _ stored
  have stored₂ := tableLookup_mirror_plain shifted stored plain
  have selectedPlain := tableEntries_plain plain _ (Array.mem_of_getElem? selected)
  have resultPlain : Plain result := by
    cases selectedPlain with
    | tuple _ fieldsPlain => exact fieldsPlain result (by simp)
  have updatedPlain := tableEntries_erase_plain plain index
  refine ⟨state₂.writeLoanValue (.table slot) (.vector (entries.eraseIdxIfInBounds index)), ?_, ?_⟩
  · rw [RuntimeValue.shift_of_plain offset keyPlain, RuntimeValue.shift_of_plain offset resultPlain]
    exact removeTableEntry?_eq_some.mpr ⟨entries, index, stored₂, found, selected, rfl⟩
  · have mirror := shifted.writeLoanValue (.table slot)
      (RuntimeValue.above_of_plain frontier updatedPlain)
    simpa only [RuntimeValue.shift_of_plain offset updatedPlain] using mirror

theorem retireEmptyTable?_mirror
    (shifted : StateShifted offset frontier inert inert' state state₂)
    (retired : retireEmptyTable? state slot = some final) :
    ∃ final₂, retireEmptyTable? state₂ slot = some final₂ ∧
      StateShifted offset frontier inert inert' final final₂ := by
  rcases retireEmptyTable?_eq_some.mp retired with ⟨stored, rfl⟩
  have stored₂ := tableLookup_mirror_plain shifted stored (.vector _ fun _ member => by simp at member)
  refine ⟨{ state₂ with tables.contents := state₂.tables.contents.erase slot },
    retireEmptyTable?_eq_some.mpr ⟨stored₂, rfl⟩, ?_⟩
  exact { shifted with
    tables := by simp [shifted.tables, NativeTableStorage.shift, GlobalMap.erase_shift]
    tablesAbove := fun entry member => by
      simp only [GlobalMap.erase, Array.mem_filter] at member
      exact shifted.tablesAbove entry member.1 }

end LeanerIR.SemanticOperations
