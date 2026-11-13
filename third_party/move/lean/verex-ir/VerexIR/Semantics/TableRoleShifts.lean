-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Semantics.TableRoles
import VerexIR.Semantics.LoanRenamingOps

/-!
# Table roles under loan renaming

A table role reads no loan (`tableRoleCall`): its step works on loan-free
values and yields loan-free values (`roleStep_plain`). A run of a role is
therefore mirrored by the run on its arguments with their loans renamed,
the loan of an entry it lends taken at the second run's frontier
(`tableRoleCall_shifts`), as the loan renaming of the runs' other steps is
(`Semantics/LoanIndependence.lean`).
-/

namespace VerexIR.SemanticOperations

open Validation

attribute [local instance] Maps.decEq

/-! ## Loan-free values of the map model -/

/-- Entries whose keys and values hold no loan. -/
def EntriesPlain (entries : Maps.Entries) : Prop :=
  ∀ entry ∈ entries, Plain entry.1 ∧ Plain entry.2

theorem EntriesPlain.mono {entries entries' : Maps.Entries}
    (plain : EntriesPlain entries) (subset : ∀ entry ∈ entries', entry ∈ entries) :
    EntriesPlain entries' :=
  fun entry member => plain entry (subset entry member)

theorem EntriesPlain.nil : EntriesPlain [] := fun _ member => nomatch member

theorem Plain.field {value : RuntimeValue} (plain : Plain value) (index : Nat) :
    Plain (value.field index) := by
  cases plain with
  | vector elements all | tuple elements all | nominal _ _ elements all =>
      simp only [RuntimeValue.field]
      cases found : elements[index]? with
      | none => exact .unit
      | some element => exact all element (Array.mem_of_getElem? found)
  | _ => exact .unit

theorem _root_.VerexIR.Maps.entriesOf_plain {map : RuntimeValue} (plain : Plain map) :
    EntriesPlain (Maps.entriesOf map) := by
  intro entry member
  unfold Maps.entriesOf at member
  cases plain with
  | nominal _ _ fields all =>
      simp only at member
      split at member
      · rename_i elements found
        cases all _ (Array.mem_of_back? found) with
        | vector _ elementsPlain =>
            simp only [List.mem_map] at member
            obtain ⟨element, inside, rfl⟩ := member
            have elementPlain := elementsPlain element (by simpa using inside)
            exact ⟨elementPlain.field 0, elementPlain.field 1⟩
      · simp at member
  | _ => simp at member

theorem _root_.VerexIR.Maps.Layout.entryValue_plain (layout : Maps.Layout) {entry : RuntimeValue × RuntimeValue}
    (plain : Plain entry.1 ∧ Plain entry.2) : Plain (layout.entryValue entry) := by
  unfold Maps.Layout.entryValue
  split
  · refine .nominal _ _ _ fun field member => ?_
    simp only [List.mem_toArray, List.mem_cons, List.not_mem_nil, or_false] at member
    rcases member with rfl | rfl
    · exact plain.1
    · exact plain.2
  · refine .tuple _ fun field member => ?_
    simp only [List.mem_toArray, List.mem_cons, List.not_mem_nil, or_false] at member
    rcases member with rfl | rfl
    · exact plain.1
    · exact plain.2

theorem _root_.VerexIR.Maps.Layout.build_plain (layout : Maps.Layout) {handle : RuntimeValue}
    (handlePlain : Plain handle) {slots : List RuntimeValue}
    (slotsPlain : ∀ slot ∈ slots, Plain slot) {entries : Maps.Entries}
    (plain : EntriesPlain entries) : Plain (layout.build handle entries slots) := by
  unfold Maps.Layout.build
  refine .nominal _ _ _ fun field member => ?_
  rcases Array.mem_push.mp member with member | rfl
  · rcases Array.mem_append.mp member with member | member
    · rcases Array.mem_append.mp member with member | member
      · split at member
        · simp only [List.mem_toArray, List.mem_cons, List.not_mem_nil, or_false] at member
          exact member ▸ handlePlain
        · simp at member
      · split at member
        · simp only [List.mem_toArray, List.mem_cons, List.not_mem_nil, or_false] at member
          exact member ▸ .integer _
        · simp at member
    · exact slotsPlain field (List.mem_toArray.mp member)
  · refine .vector _ fun element member => ?_
    simp only [List.mem_toArray, List.mem_map] at member
    obtain ⟨entry, inside, rfl⟩ := member
    exact layout.entryValue_plain (plain entry inside)

theorem _root_.VerexIR.Maps.Layout.handleOf_plain (layout : Maps.Layout) {map : RuntimeValue}
    (plain : Plain map) : Plain (layout.handleOf map) := by
  unfold Maps.Layout.handleOf
  split
  · exact plain.field 0
  · exact .unit

/-- A plain map's ghost slots are plain. -/
theorem _root_.VerexIR.Maps.Layout.slotsOf_plain (layout : Maps.Layout) {map : RuntimeValue}
    (plain : Plain map) : ∀ slot ∈ layout.slotsOf map, Plain slot := by
  intro slot member
  simp only [Maps.Layout.slotsOf, List.mem_map, List.mem_range] at member
  obtain ⟨_, _, rfl⟩ := member
  exact plain.field _

/-- Fresh ghost slots are plain. -/
theorem _root_.VerexIR.Maps.Layout.freshSlots_plain (layout : Maps.Layout) :
    ∀ slot ∈ layout.freshSlots, Plain slot := by
  intro slot member
  simp only [Maps.Layout.freshSlots, List.mem_replicate] at member
  exact member.2 ▸ .ghost _ (.integer _)

/-- A value built with fresh slots from a plain handle and plain entries is
plain: `build_plain` at the slots a proof search meets. -/
theorem _root_.VerexIR.Maps.Layout.build_plain_fresh (layout : Maps.Layout) {handle : RuntimeValue}
    (handlePlain : Plain handle) {entries : Maps.Entries} (plain : EntriesPlain entries) :
    Plain (layout.build handle entries) :=
  layout.build_plain handlePlain layout.freshSlots_plain plain

/-- A value built with a plain map's slots from a plain handle and plain
entries is plain. -/
theorem _root_.VerexIR.Maps.Layout.build_plain_slotsOf (layout : Maps.Layout)
    {handle : RuntimeValue} (handlePlain : Plain handle) {map : RuntimeValue} (mapPlain : Plain map)
    {entries : Maps.Entries} (plain : EntriesPlain entries) :
    Plain (layout.build handle entries (layout.slotsOf map)) :=
  layout.build_plain handlePlain (layout.slotsOf_plain mapPlain) plain

theorem _root_.VerexIR.Maps.mem_replaceFirst {key value : RuntimeValue} :
    {entries : Maps.Entries} → {entry : RuntimeValue × RuntimeValue} →
    entry ∈ Maps.replaceFirst key value entries → entry = (key, value) ∨ entry ∈ entries
  | [], _, member => by simp [Maps.replaceFirst] at member
  | head :: rest, entry, member => by
      simp only [Maps.replaceFirst] at member
      split at member
      · rcases List.mem_cons.mp member with same | inside
        · exact .inl same
        · exact .inr (List.mem_cons_of_mem _ inside)
      · rcases List.mem_cons.mp member with same | inside
        · exact .inr (same ▸ List.mem_cons_self)
        · rcases Maps.mem_replaceFirst inside with same | inside
          · exact .inl same
          · exact .inr (List.mem_cons_of_mem _ inside)

theorem _root_.VerexIR.Maps.Discipline.set_plain (discipline : Maps.Discipline) {entries : Maps.Entries}
    (plain : EntriesPlain entries) {key value : RuntimeValue} (keyPlain : Plain key)
    (valuePlain : Plain value) : EntriesPlain (discipline.set entries key value) := by
  intro entry member
  have member : entry = (key, value) ∨ entry ∈ entries := by
    cases discipline with
    | sequence =>
        simp only [Maps.Discipline.set, Maps.seqSet] at member
        split at member
        · exact Maps.mem_replaceFirst member
        · rcases List.mem_append.mp member with inside | same
          · exact .inr inside
          · exact .inl (List.mem_singleton.mp same)
    | ordered rank =>
        simp only [Maps.Discipline.set] at member
        exact Maps.mem_ordSet rank member
  rcases member with rfl | inside
  · exact ⟨keyPlain, valuePlain⟩
  · exact plain entry inside

theorem _root_.VerexIR.Maps.Discipline.del_plain (discipline : Maps.Discipline) {entries : Maps.Entries}
    (plain : EntriesPlain entries) (key : RuntimeValue) :
    EntriesPlain (discipline.del entries key) := by
  cases discipline with
  | sequence =>
      simp only [Maps.Discipline.del, Maps.seqDel]
      split
      · exact plain.mono fun entry member =>
          List.mem_of_mem_eraseIdx ((Maps.swapRemove_perm _ _).mem_iff.mp member)
      · exact plain
  | ordered rank =>
      simp only [Maps.Discipline.del]
      exact plain.mono fun entry member => (Maps.sublist_ordDel key entries).subset member

theorem _root_.VerexIR.Maps.elementsOf_plain {vector : RuntimeValue} (plain : Plain vector) :
    ∀ element ∈ Maps.elementsOf vector, Plain element := by
  intro element member
  cases plain with
  | vector elements all => exact all element (by simpa [Maps.elementsOf] using member)
  | _ => simp [Maps.elementsOf] at member

theorem _root_.VerexIR.Maps.foldl_set_plain (discipline : Maps.Discipline) :
    (pairs : List (RuntimeValue × RuntimeValue)) → (∀ pair ∈ pairs, Plain pair.1 ∧ Plain pair.2) →
    {entries : Maps.Entries} → EntriesPlain entries →
    EntriesPlain (pairs.foldl (fun entries entry => discipline.set entries entry.1 entry.2) entries)
  | [], _, _, plain => plain
  | pair :: rest, pairsPlain, _, plain => by
      simp only [List.foldl_cons]
      exact Maps.foldl_set_plain discipline rest
        (fun pair member => pairsPlain pair (List.mem_cons_of_mem _ member))
        (discipline.set_plain plain (pairsPlain pair List.mem_cons_self).1
          (pairsPlain pair List.mem_cons_self).2)

theorem _root_.VerexIR.Maps.zip_plain {keys values : List RuntimeValue} (keysPlain : ∀ key ∈ keys, Plain key)
    (valuesPlain : ∀ value ∈ values, Plain value) :
    ∀ pair ∈ keys.zip values, Plain pair.1 ∧ Plain pair.2 := by
  intro pair member
  exact ⟨keysPlain _ (List.of_mem_zip member).1, valuesPlain _ (List.of_mem_zip member).2⟩

theorem _root_.VerexIR.Maps.get_plain {entries : Maps.Entries} (plain : EntriesPlain entries) (key : RuntimeValue) :
    Plain (Maps.get entries key) := by
  induction entries with
  | nil => exact .unit
  | cons head rest ih =>
      simp only [Maps.get]
      split
      · exact (plain head List.mem_cons_self).2
      · exact ih fun entry member => plain entry (List.mem_cons_of_mem _ member)

theorem _root_.VerexIR.Maps.keyAt_plain {map : RuntimeValue} (plain : Plain map) (index : Int) :
    Plain (Maps.keyAt map index) := by
  unfold Maps.keyAt
  split
  · exact .unit
  · cases found : (Maps.entriesOf map)[index.toNat]? with
    | none => exact .unit
    | some entry =>
        exact (Maps.entriesOf_plain plain entry (List.mem_of_getElem? found)).1

theorem _root_.VerexIR.Maps.update_plain (layout : Maps.Layout) (discipline : Maps.Discipline)
    {map key value : RuntimeValue} (plain : Plain map) (keyPlain : Plain key)
    (valuePlain : Plain value) : Plain (Maps.update layout discipline map key value) :=
  layout.build_plain (layout.handleOf_plain plain) (layout.slotsOf_plain plain)
    (discipline.set_plain (Maps.entriesOf_plain plain) keyPlain valuePlain)

theorem _root_.VerexIR.Maps.remove_plain (layout : Maps.Layout) (discipline : Maps.Discipline)
    {map : RuntimeValue} (plain : Plain map) (key : RuntimeValue) :
    Plain (Maps.remove layout discipline map key) :=
  layout.build_plain (layout.handleOf_plain plain) (layout.slotsOf_plain plain)
    (discipline.del_plain (Maps.entriesOf_plain plain) key)

theorem _root_.VerexIR.Maps.updateAll_plain (layout : Maps.Layout) (discipline : Maps.Discipline)
    {map keys values : RuntimeValue} (plain : Plain map) (keysPlain : Plain keys)
    (valuesPlain : Plain values) : Plain (Maps.updateAll layout discipline map keys values) :=
  layout.build_plain (layout.handleOf_plain plain) (layout.slotsOf_plain plain)
    (Maps.foldl_set_plain discipline _
      (Maps.zip_plain (Maps.elementsOf_plain keysPlain) (Maps.elementsOf_plain valuesPlain))
      (Maps.entriesOf_plain plain))

theorem _root_.VerexIR.Maps.empty_plain (layout : Maps.Layout) : Plain (Maps.empty layout) := by
  unfold Maps.empty
  refine layout.build_plain ?_ layout.freshSlots_plain fun _ member => nomatch member
  split
  · exact .bool _
  · exact .unit

theorem _root_.VerexIR.Maps.takeEntries_plain (layout : Maps.Layout) {map : RuntimeValue}
    (plain : Plain map) (count : Int) : Plain (Maps.takeEntries layout map count) :=
  layout.build_plain (layout.handleOf_plain plain) (layout.slotsOf_plain plain)
    ((Maps.entriesOf_plain plain).mono fun _ member => List.mem_of_mem_take member)

theorem _root_.VerexIR.Maps.dropEntries_plain (layout : Maps.Layout) {map : RuntimeValue}
    (plain : Plain map) (count : Int) : Plain (Maps.dropEntries layout map count) :=
  layout.build_plain .unit layout.freshSlots_plain
    ((Maps.entriesOf_plain plain).mono fun _ member => List.mem_of_mem_drop member)

theorem _root_.VerexIR.Maps.replaceKey_plain (layout : Maps.Layout) {map old new : RuntimeValue}
    (plain : Plain map) (newPlain : Plain new) : Plain (Maps.replaceKey layout map old new) := by
  refine layout.build_plain (layout.handleOf_plain plain) (layout.slotsOf_plain plain)
    fun entry member => ?_
  simp only [List.mem_map] at member
  obtain ⟨original, inside, rfl⟩ := member
  have originalPlain := Maps.entriesOf_plain plain original inside
  split
  · exact ⟨newPlain, originalPlain.2⟩
  · exact originalPlain

theorem _root_.VerexIR.Maps.keysOf_plain {map : RuntimeValue} (plain : Plain map) :
    Plain (Maps.keysOf map) := by
  refine .vector _ fun key member => ?_
  simp only [List.mem_toArray, List.mem_map] at member
  obtain ⟨entry, inside, rfl⟩ := member
  exact (Maps.entriesOf_plain plain entry inside).1

theorem _root_.VerexIR.Maps.valuesOf_plain {map : RuntimeValue} (plain : Plain map) :
    Plain (Maps.valuesOf map) := by
  refine .vector _ fun value member => ?_
  simp only [List.mem_toArray, List.mem_map] at member
  obtain ⟨entry, inside, rfl⟩ := member
  exact (Maps.entriesOf_plain plain entry inside).2

theorem _root_.VerexIR.Maps.valueAt_plain {map : RuntimeValue} (plain : Plain map)
    (key : RuntimeValue) : Plain (Maps.valueAt map key) :=
  Maps.get_plain (Maps.entriesOf_plain plain) key

theorem _root_.VerexIR.Maps.prevKey?_plain (rank : ValueRanks) {map key found : RuntimeValue}
    (plain : Plain map) (prev : Maps.prevKey? rank map key = some found) : Plain found := by
  unfold Maps.prevKey? at prev
  obtain ⟨entry, last, rfl⟩ := Option.map_eq_some_iff.mp prev
  exact (Maps.entriesOf_plain plain entry (List.mem_filter.mp (List.mem_of_getLast? last)).1).1

theorem _root_.VerexIR.Maps.nextKey?_plain (rank : ValueRanks) {map key found : RuntimeValue}
    (plain : Plain map) (next : Maps.nextKey? rank map key = some found) : Plain found := by
  unfold Maps.nextKey? at next
  obtain ⟨entry, first, rfl⟩ := Option.map_eq_some_iff.mp next
  exact (Maps.entriesOf_plain plain entry (List.mem_of_find?_eq_some first)).1

/-- A step whose values hold no loan. -/
def RoleStep.Plain : RoleStep → Prop
  | .abort _ => True
  | .returned result updated _ =>
      SemanticOperations.Plain result ∧ ∀ value, updated = some value → SemanticOperations.Plain value
  | .lends map key value =>
      SemanticOperations.Plain map ∧ SemanticOperations.Plain key ∧ SemanticOperations.Plain value

theorem Plain.pair {key value : RuntimeValue} (keyPlain : Plain key) (valuePlain : Plain value) :
    Plain (.tuple #[key, value]) := by
  refine .tuple _ fun element member => ?_
  simp only [List.mem_toArray, List.mem_cons, List.not_mem_nil, or_false] at member
  rcases member with rfl | rfl
  · exact keyPlain
  · exact valuePlain

theorem Plain.optional (owner : StructHandle) {value : Option RuntimeValue}
    (plain : ∀ found, value = some found → Plain found) :
    Plain (.nominal owner none #[.vector value.toArray]) := by
  refine .nominal _ _ _ fun field member => ?_
  simp only [List.mem_toArray, List.mem_cons, List.not_mem_nil, or_false] at member
  subst member
  refine .vector _ fun element member => ?_
  cases value with
  | none => simp at member
  | some found =>
      simp only [Option.toArray_some, List.mem_toArray, List.mem_singleton] at member
      exact member ▸ plain found rfl

theorem roleOptional_plain {unit : ValidatedUnit} {ns : ValidatedNamespace}
    {declaration : FunctionDecl FunctionBody} {value : Option RuntimeValue} {result : RuntimeValue}
    (found : roleOptional unit ns declaration value = some result)
    (plain : ∀ held, value = some held → Plain held) : Plain result := by
  unfold roleOptional at found
  simp only [bind, Option.bind_eq_some_iff] at found
  obtain ⟨_, -, _, -, found⟩ := found
  split at found
  · simp only [bind, Option.bind_eq_some_iff] at found
    obtain ⟨owner, -, _, -, _, -, layout, -, found⟩ := found
    cases found
    cases layout with
    | vector => exact Plain.optional _ plain
    | variants empty present =>
        refine .nominal _ _ _ fun field member => ?_
        cases value with
        | none => simp [OptionalLayout.encode] at member
        | some held =>
            simp only [Option.toArray_some, List.mem_toArray, List.mem_singleton] at member
            exact member ▸ plain held rfl
  · cases found

/-- A field of a plain nominal value is plain. -/
theorem Plain.of_nominal_field {source : StructHandle} {variant : Option String}
    {fields : Array RuntimeValue} (plain : Plain (.nominal source variant fields)) {index : Nat}
    {field : RuntimeValue} (found : fields[index]? = some field) : Plain field := by
  cases plain with
  | nominal _ _ _ all => exact all field (Array.mem_of_getElem? found)

/-- The key an iterator names in a plain map, from a plain payload, is plain. -/
theorem iteratorKey?_plain {shape : IteratorShape} {map payload key : RuntimeValue}
    (mapPlain : Plain map) (payloadPlain : Plain payload)
    (named : iteratorKey? shape map payload = some key) : Plain key := by
  unfold iteratorKey? at named
  split at named
  · cases named; exact payloadPlain
  · split at named
    · split at named
      · cases named; exact Maps.keyAt_plain mapPlain _
      · cases named
    · cases named

/-- The entry an iterator names in a plain map, by a plain iterator, is plain. -/
theorem iteratorEntryStep_plain {shape : IteratorShape} {map iterator : RuntimeValue}
    {step : RoleStep} (mapPlain : Plain map) (iteratorPlain : Plain iterator)
    (stepped : iteratorEntryStep shape map iterator = some step) : step.Plain := by
  unfold iteratorEntryStep at stepped
  split at stepped
  · split at stepped
    · cases stepped; trivial
    · simp only [bind, Option.bind_eq_some_iff] at stepped
      obtain ⟨payload, found, stepped⟩ := stepped
      split at stepped
      · split at stepped
        · cases stepped
          exact ⟨mapPlain, iteratorKey?_plain mapPlain (Plain.of_nominal_field iteratorPlain found)
            ‹_›, Maps.valueAt_plain mapPlain _⟩
        · cases stepped; trivial
      · cases stepped; trivial
  · cases stepped

/-- A plain map converts to a plain table. -/
theorem conversionStep_plain (unit : ValidatedUnit) {result : TableLayout} {map : RuntimeValue}
    (handles : Array String) (mapPlain : Plain map) : (conversionStep unit result map handles).Plain := by
  unfold conversionStep
  split
  · exact ⟨Maps.updateAll_plain _ _ (Maps.Layout.build_plain_fresh _ (.address _) EntriesPlain.nil)
      (Maps.keysOf_plain mapPlain) (Maps.valuesOf_plain mapPlain), fun _ found => nomatch found⟩
  · exact ⟨Maps.updateAll_plain _ _ (Maps.empty_plain _)
      (Maps.keysOf_plain mapPlain) (Maps.valuesOf_plain mapPlain), fun _ found => nomatch found⟩

set_option maxHeartbeats 2000000 in

/-- A role's step on loan-free values yields loan-free values. -/
theorem roleStep_plain {unit : ValidatedUnit} {ns : ValidatedNamespace} {declaration : FunctionDecl FunctionBody}
    {role : String} {layout : TableLayout} {handles : Array String} {values : Array RuntimeValue}
    {step : RoleStep}
    (stepped : roleStep unit ns declaration role layout handles values = some step)
    (plain : ∀ value ∈ values, Plain value) : step.Plain := by
  unfold roleStep at stepped
  simp only [bind, Option.bind_eq_some_iff] at stepped
  obtain ⟨operation, _, stepped⟩ := stepped
  unfold tableOperationStep at stepped
  split at stepped
  all_goals
    try simp only [bind, pure, Option.bind_eq_some_iff, Option.some.injEq] at stepped
    repeat'
      first
      | (rw [Option.some.injEq] at stepped; subst stepped)
      | subst stepped
      | obtain ⟨_, found, stepped⟩ := stepped
        try have := plain _ (Array.mem_of_getElem? found)
      | (simp only [Option.bind_some, Option.bind_eq_some_iff] at stepped)
      | split at stepped
  -- The entry an iterator names, and a conversion's table, by their steps.
  all_goals try exact iteratorEntryStep_plain ‹_› ‹_› stepped
  all_goals try exact conversionStep_plain _ _ ‹_›
  all_goals
    simp only [RoleStep.Plain, Option.some.injEq, forall_eq', reduceCtorEq, false_implies,
      implies_true, and_true, true_and, endKey]
    repeat' (apply And.intro)
    repeat' split
    all_goals
      first
      | trivial
      -- A fresh table at a fresh handle, with fresh slots. The search unifies
      -- at reducible transparency, which keeps it from computing built maps.
      | with_reducible exact Maps.Layout.build_plain_fresh _ (.address _) EntriesPlain.nil
      | solve_by_elim (maxDepth := 12) (transparency := .reducible) [Maps.update_plain,
          Maps.remove_plain, Maps.updateAll_plain,
          Maps.empty_plain, Maps.takeEntries_plain, Maps.dropEntries_plain, Maps.replaceKey_plain,
          Maps.keysOf_plain, Maps.valuesOf_plain, Maps.valueAt_plain, Maps.keyAt_plain,
          Plain.pair, Plain.unit, Plain.integer, Plain.bool, Plain.address, EntriesPlain.nil]
      | (apply roleOptional_plain ‹roleOptional _ _ _ _ = some _›
         intro held found
         repeat' split at found
         all_goals
           first
           | (simp at found; done)
           | (injection found with found
              subst found
              exact Maps.valueAt_plain (by assumption) _)
           | exact Maps.prevKey?_plain _ ‹_› found
           | exact Maps.nextKey?_plain _ ‹_› found)


/-! ## The run under loan renaming -/

section Shifts

variable {ρ : Nat → Nat} {offset frontier : Nat} {inside : Nat → Prop} {inert inert' : Nat}

/-- An argument whose value holds no loan reads the same value renamed. -/
theorem mapArgument_rename (ρ : Nat → Nat) {argument : RuntimeValue}
    (plain : Plain (mapArgument argument).2) :
    (mapArgument (argument.rename ρ)).2 = (mapArgument argument).2 := by
  cases argument <;> simpa [mapArgument] using RuntimeValue.rename_of_plain ρ plain

theorem roleExportsFrom_rename (ρ : Nat → Nat) (updated : Option RuntimeValue) (mapIndex : Nat) :
    (arguments : List RuntimeValue) → (index : Nat) →
    roleExportsFrom (updated.map (·.rename ρ)) mapIndex (arguments.map (·.rename ρ)) index =
      (roleExportsFrom updated mapIndex arguments index).map fun entry =>
        (ρ entry.1, entry.2.rename ρ)
  | [], _ => rfl
  | argument :: rest, index => by
      have later := roleExportsFrom_rename ρ updated mapIndex rest (index + 1)
      cases argument <;> simp only [List.map_cons, RuntimeValue.rename_borrow] <;>
        simp only [roleExportsFrom, later, List.map_cons] <;>
        first
        | rfl
        | (cases updated <;> split <;> simp_all)
        | simp_all [roleExportsFrom]

theorem roleExportsFrom_within (updated : Option RuntimeValue)
    (updatedWithin : ∀ value, updated = some value → value.Within inside) (mapIndex : Nat) :
    (arguments : List RuntimeValue) → (index : Nat) →
    (∀ argument ∈ arguments, argument.Within inside) →
    ∀ entry ∈ roleExportsFrom updated mapIndex arguments index,
      inside entry.1 ∧ entry.2.Within inside
  | [], _, _, entry, member => by simp [roleExportsFrom] at member
  | argument :: rest, index, within, entry, member => by
      have later := roleExportsFrom_within updated updatedWithin mapIndex rest (index + 1)
        fun other inside => within other (List.mem_cons_of_mem _ inside)
      cases argument
      case borrow loan current =>
        simp only [roleExportsFrom, List.mem_cons] at member
        rcases member with rfl | member
        · have headWithin := within _ List.mem_cons_self
          simp only [RuntimeValue.Within] at headWithin
          refine ⟨headWithin.1, ?_⟩
          split
          · cases found : updated with
            | none => simpa using headWithin.2
            | some value => simpa using updatedWithin value found
          · exact headWithin.2
        · exact later entry member
      all_goals exact later entry (by simpa [roleExportsFrom] using member)

theorem _root_.VerexIR.StateRenamed.appendPending {state state₂ : RuntimeState}
    (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂) :
    (exports : List (Nat × RuntimeValue)) → (∀ entry ∈ exports, inside entry.1 ∧ entry.2.Within inside) →
    StateRenamed ρ offset frontier inside inert inert'
      { state with pending := state.pending ++ exports.toArray }
      { state₂ with pending := state₂.pending ++
          (exports.map fun entry => (ρ entry.1, entry.2.rename ρ)).toArray }
  | [], _ => by simpa using shifted
  | entry :: rest, within => by
      have pushed := shifted.push (within entry List.mem_cons_self).1 (within entry List.mem_cons_self).2
      have appended := StateRenamed.appendPending pushed rest
        fun other member => within other (List.mem_cons_of_mem _ member)
      have cons {α : Type} (array : Array α) (head : α) (tail : List α) :
          array ++ (head :: tail).toArray = array.push head ++ tail.toArray := by
        apply Array.ext'; simp
      rw [List.map_cons, cons, cons]
      exact appended

theorem unpackFallthrough_rename (ρ : Nat → Nat) (count : Nat) (value : RuntimeValue) :
    unpackFallthrough count (value.rename ρ) =
      (unpackFallthrough count value).map (·.map (·.rename ρ)) := by
  match count with
  | 0 => cases value <;> simp [unpackFallthrough]
  | 1 => simp [unpackFallthrough]
  | count + 2 =>
      cases value <;> simp [unpackFallthrough] <;> (try split) <;> simp_all

theorem unpackFallthrough_within {count : Nat} {value : RuntimeValue} {results : Array RuntimeValue}
    (unpacked : unpackFallthrough count value = some results) (within : value.Within inside) :
    ∀ result ∈ results, result.Within inside := by
  intro result member
  match count with
  | 0 =>
      cases value <;> simp [unpackFallthrough] at unpacked
      subst unpacked; simp at member
  | 1 =>
      simp only [unpackFallthrough, Option.some.injEq] at unpacked
      subst unpacked
      simp only [List.mem_toArray, List.mem_singleton] at member
      exact member ▸ within
  | count + 2 =>
      cases value <;> simp only [unpackFallthrough, reduceCtorEq] at unpacked
      split at unpacked
      · simp only [Option.some.injEq] at unpacked
        subst unpacked
        exact (RuntimeValue.within_tuple (inside := inside) _).mp within result member
      · cases unpacked

theorem withEntryHole_rename (ρ : Nat → Nat) {map : RuntimeValue} (plain : Plain map)
    (key : RuntimeValue) (loan : Nat) :
    withEntryHole map key (ρ loan) = (withEntryHole map key loan).map (·.rename ρ) := by
  unfold withEntryHole
  cases map with
  | nominal source variant fields =>
      simp only [bind, Option.bind_eq_bind]
      by_cases positive : 0 < fields.size
      · simp only [guard, positive, ↓reduceIte, Option.bind_some]
        have renamed := writeProjections?_rename (ρ := ρ)
          [.field (fields.size - 1), .index (Maps.indexOf (Maps.entriesOf (.nominal source variant
            fields)) key), .index 1] (.nominal source variant fields) (.loanHole loan)
        rw [RuntimeValue.rename_of_plain ρ plain, RuntimeValue.rename_loanHole] at renamed
        exact renamed
      · simp [guard, positive, failure]
  | _ => rfl

theorem withEntryHole_within {map key holed : RuntimeValue} {loan : Nat}
    (holes : withEntryHole map key loan = some holed) (plain : Plain map)
    {β γ : Nat → Nat} (base : LoanBase β γ inside) (loanInside : inside loan) :
    holed.Within inside := by
  have renamed := withEntryHole_rename β plain key (γ loan)
  rw [base.inverse loan loanInside, holes] at renamed
  obtain ⟨original, -, rfl⟩ := Option.map_eq_some_iff.mp renamed.symm
  exact base.within original

theorem roleExports_rename (ρ : Nat → Nat) (arguments : Array RuntimeValue)
    (updated : Option RuntimeValue) (mapIndex : Nat) :
    roleExports (arguments.map (·.rename ρ)) (updated.map (·.rename ρ)) mapIndex =
      (roleExportsFrom updated mapIndex arguments.toList 0).toArray.map
        fun entry => (ρ entry.1, entry.2.rename ρ) := by
  simp only [roleExports, Array.toList_map, roleExportsFrom_rename, List.map_toArray]

/-- A run of a table role is mirrored by its run on the renamed arguments
from a state mirroring its start, the loan of an entry it lends minted at
the second run's frontier. -/
theorem tableRoleCall_shifts {unit : ValidatedUnit} {handle : FunctionHandle} {role : String}
    {layout : TableLayout} {state state' : RuntimeState} {arguments : Array RuntimeValue}
    {outcome : Outcome}
    (call : tableRoleCall unit handle role layout state arguments = some (state', outcome))
    {state₂ : RuntimeState} (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    (within : ∀ argument ∈ arguments, argument.Within inside) :
    ∃ state₂', StateRenamed ρ offset frontier inside inert inert' state' state₂' ∧
      outcome.Within inside ∧
      tableRoleCall unit handle role layout state₂ (arguments.map (·.rename ρ)) =
        some (state₂', outcome.rename ρ) := by
  unfold tableRoleCall at call ⊢
  simp only [bind, Option.bind_eq_some_iff] at call
  obtain ⟨ns, ns_eq, declaration, declaration_eq, _, arity, _, plainGuard, _, fitsGuard, step,
    stepped, _, stepFitsGuard, call⟩ := call
  have arity : arguments.size = declaration.signature.parameters.size := by
    simp only [guard] at arity
    split at arity
    · rename_i holds; simpa using holds
    · cases arity
  have allPlain : (arguments.map fun argument => (mapArgument argument).2).all plain? = true := by
    simp only [guard] at plainGuard
    split at plainGuard
    · rename_i holds; exact holds
    · cases plainGuard
  have allFit : (arguments.map fun argument => (mapArgument argument).2).all layout.fits = true := by
    simp only [guard] at fitsGuard
    split at fitsGuard
    · rename_i holds; exact holds
    · cases fitsGuard
  have stepFits : step.values.all layout.fits = true := by
    simp only [guard] at stepFitsGuard
    split at stepFitsGuard
    · rename_i holds; exact holds
    · cases stepFitsGuard
  have valuesPlain : ∀ value ∈ arguments.map (fun argument => (mapArgument argument).2),
      Plain value := by
    intro value member
    obtain ⟨index, bound, rfl⟩ := Array.mem_iff_getElem.mp member
    exact (plain?_iff _).mp (Array.all_eq_true.mp allPlain index bound)
  have sameValues : (arguments.map (·.rename ρ)).map (fun argument => (mapArgument argument).2) =
      arguments.map (fun argument => (mapArgument argument).2) := by
    rw [Array.map_map]
    apply Array.map_congr_left
    intro argument member
    exact mapArgument_rename ρ (valuesPlain _ (Array.mem_map_of_mem member))
  have stepPlain := roleStep_plain stepped valuesPlain
  have base := shifted.renames.base
  obtain ⟨β, γ, base⟩ := base
  have allPlain' := allPlain
  simp only [Array.size_map, arity] at allPlain'
  have allFit' := allFit
  simp only [Array.size_map, arity] at allFit'
  simp only [bind, ns_eq, declaration_eq, Option.bind_some, Array.size_map, arity, beq_self_eq_true,
    guard, ↓reduceIte, pure, sameValues, allPlain', allFit', shifted.tableHandles, stepped,
    stepFits]
  have argumentsWithin : ∀ argument ∈ arguments.toList, argument.Within inside :=
    fun argument member => within argument (Array.mem_toList_iff.mp member)
  cases step with
  | abort code =>
      dsimp only at call ⊢
      simp only [Option.some.injEq, Prod.mk.injEq] at call
      obtain ⟨rfl, rfl⟩ := call
      exact ⟨state₂, shifted, by simp [Outcome.Within], by simp [Outcome.rename]⟩
  | returned result updated allocated =>
      obtain ⟨resultPlain, updatedPlain⟩ := stepPlain
      dsimp only at call ⊢
      obtain ⟨results, unpacked, call⟩ := Option.bind_eq_some_iff.mp call
      simp only [Option.some.injEq, Prod.mk.injEq] at call
      obtain ⟨rfl, rfl⟩ := call
      have updatedSame : updated.map (·.rename ρ) = updated := by
        cases found : updated with
        | none => rfl
        | some value => simp [RuntimeValue.rename_of_plain ρ (updatedPlain value found)]
      have exported := roleExports_rename ρ arguments updated (roleMapArgument role)
      rw [updatedSame] at exported
      have resultsSame : results.map (·.rename ρ) = results := by
        have renamed := unpackFallthrough_rename ρ declaration.signature.results.size result
        rw [RuntimeValue.rename_of_plain ρ resultPlain, unpacked] at renamed
        simpa using renamed.symm
      have handled : StateRenamed ρ offset frontier inside inert inert'
          { state with tableHandles := allocated.elim state.tableHandles state.tableHandles.push }
          { state₂ with tableHandles := allocated.elim state.tableHandles state.tableHandles.push } :=
        { shifted with tableHandles := rfl }
      have appended := handled.appendPending
        (roleExportsFrom updated (roleMapArgument role) arguments.toList 0)
        (roleExportsFrom_within updated
          (fun value found => RuntimeValue.within_of_plain _ (updatedPlain value found))
          (roleMapArgument role) arguments.toList 0 argumentsWithin)
      refine ⟨_, appended, ?_, ?_⟩
      · exact unpackFallthrough_within unpacked (RuntimeValue.within_of_plain _ resultPlain)
      · have exports := roleExportsFrom_rename ρ updated (roleMapArgument role) arguments.toList 0
        rw [updatedSame, ← Array.toList_map] at exports
        simp only [unpacked, Option.bind_some, Outcome.rename, resultsSame, roleExports, exports]
  | lends map key value =>
      obtain ⟨mapPlain, keyPlain, valuePlain⟩ := stepPlain
      dsimp only at call ⊢
      split at call
      · rename_i loan current lent
        obtain ⟨holed, holes, call⟩ := Option.bind_eq_some_iff.mp call
        obtain ⟨results, unpacked, call⟩ := Option.bind_eq_some_iff.mp call
        simp only [Option.some.injEq, Prod.mk.injEq] at call
        obtain ⟨rfl, rfl⟩ := call
        have lent₂ : (arguments.map (·.rename ρ))[roleMapArgument role]? =
            some (.borrow (ρ loan) (current.rename ρ)) := by
          simp [Array.getElem?_map, lent]
        have holes₂ : withEntryHole map key state₂.nextLoan = some (holed.rename ρ) := by
          rw [← shifted.next, withEntryHole_rename ρ mapPlain key, holes]
          rfl
        have exported := roleExports_rename ρ arguments (some holed) (roleMapArgument role)
        have unpacked₂ := unpackFallthrough_rename ρ declaration.signature.results.size
          (.borrow state.nextLoan value)
        rw [unpacked, RuntimeValue.rename_borrow, RuntimeValue.rename_of_plain ρ valuePlain,
          shifted.next] at unpacked₂
        have holedWithin : holed.Within inside :=
          withEntryHole_within holes mapPlain base shifted.next_inside
        have minted : StateRenamed ρ offset frontier inside inert inert'
            { state with nextLoan := state.nextLoan + 1 }
            { state₂ with nextLoan := state₂.nextLoan + 1 } :=
          { shifted with
            nextLoan := by simp only [shifted.nextLoan]; omega
            frontier_le := Nat.le_succ_of_le shifted.frontier_le }
        have appended := minted.appendPending
          (roleExportsFrom (some holed) (roleMapArgument role) arguments.toList 0)
          (roleExportsFrom_within (some holed)
            (fun found same => by cases same; exact holedWithin) (roleMapArgument role)
            arguments.toList 0 argumentsWithin)
        refine ⟨_, appended, ?_, ?_⟩
        · exact unpackFallthrough_within unpacked
            (by simpa [RuntimeValue.Within] using
              ⟨shifted.next_inside, RuntimeValue.within_of_plain _ valuePlain⟩)
        · have exports := roleExportsFrom_rename ρ (some holed) (roleMapArgument role)
            arguments.toList 0
          rw [Option.map_some, ← Array.toList_map] at exports
          simp only [lent₂, holes₂, Option.bind_some, unpacked₂, Option.map_some, Outcome.rename,
            roleExports, exports, shifted.tableHandles]
      · cases call

end Shifts

end VerexIR.SemanticOperations
