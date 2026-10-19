-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Maps.Model
import VerexIR.Proofs.Maps

/-!
# Map encoding boundary

These lemmas transport the runtime contract model to `SpecMap K V`. Only this
module needs to know how keys, values, and the absent-read sentinel encode.
-/

namespace VerexIR.Maps.Encoding

/-- A typed observation of values, including the contract's absent read. -/
structure View (K : Type) [Inhabited K] where
  write : K → RuntimeValue
  read : RuntimeValue → K
  read_write : ∀ key, read (write key) = key
  read_unit : read .unit = default

variable {K V : Type} [Inhabited K] [Inhabited V]

theorem View.injective (view : View K) : Function.Injective view.write := by
  intro a b equal
  simpa only [view.read_write] using congrArg view.read equal

/-- The typed key order is the language's order transported through the codec. -/
def View.order (view : View K) (ranks : ValueRanks) : SpecMap.KeyOrder K where
  compare a b := RuntimeValue.order ranks (view.write a) (view.write b)
  lawful := {
    compare_self := RuntimeValue.order_self _ _
    eq_of_compare := fun h => view.injective (RuntimeValue.eq_of_order h) }
  trans := {
    eq_swap := RuntimeValue.order_swap _ _ _
    isLE_trans := fun h₁ h₂ => Std.TransCmp.isLE_trans h₁ h₂ }

/-- Integer specifications use mathematical integers, also for fixed-width
program keys. Thus a specification update remains meaningful out of range. -/
def integers : View Int where
  write := .integer
  read := RuntimeValue.asInt
  read_write _ := rfl
  read_unit := rfl

/-- Encode only map contents; allocation identity and ghost slots stay outside
this projection and are still checked by the original contract. -/
def entries (keys : View K) (values : View V) (map : SpecMap K V) : Maps.Entries :=
  map.map fun entry => (keys.write entry.1, values.write entry.2)

attribute [local instance] Maps.decEq

variable [DecidableEq K]

theorem contains (keys : View K) (values : View V) (map : SpecMap K V) (key : K) :
    Maps.containsB (entries keys values map) (keys.write key) = SpecMap.hasKey map key := by
  induction map with
  | nil => rfl
  | cons entry rest ih =>
    simp only [entries, List.map_cons, Maps.containsB, SpecMap.hasKey, SpecMap.containsB] at *
    simp only [ih, (show keys.write entry.1 = keys.write key ↔ entry.1 = key from keys.injective.eq_iff)]

theorem position (keys : View K) (values : View V) (map : SpecMap K V) (key : K) :
    Maps.indexOf (entries keys values map) (keys.write key) = SpecMap.indexOf map key := by
  induction map with
  | nil => rfl
  | cons entry rest ih =>
    simp only [entries, List.map_cons, Maps.indexOf, SpecMap.indexOf] at *
    simp only [ih, (show keys.write entry.1 = keys.write key ↔ entry.1 = key from keys.injective.eq_iff)]

theorem lookup (keys : View K) (values : View V) (map : SpecMap K V) (key : K) :
    values.read (Maps.get (entries keys values map) (keys.write key)) = SpecMap.valueAt map key := by
  induction map with
  | nil => exact values.read_unit
  | cons entry rest ih =>
    simp only [entries, List.map_cons, Maps.get, SpecMap.valueAt, SpecMap.get] at *
    simp only [(show keys.write entry.1 = keys.write key ↔ entry.1 = key from keys.injective.eq_iff)]
    split <;> simp only [*, values.read_write]

omit [DecidableEq K] in
theorem keyAt (keys : View K) (values : View V) (map : SpecMap K V) (raw : RuntimeValue)
    (equal : Maps.entriesOf raw = entries keys values map) (index : Int) :
    keys.read (Maps.keyAt raw index) = SpecMap.keyAt map index := by
  simp only [Maps.keyAt, SpecMap.keyAt, equal, entries, List.getElem?_map, Option.map_map]
  split
  · exact keys.read_unit
  · cases map[index.toNat]? <;> simp [keys.read_unit, keys.read_write]

omit [DecidableEq K] in
theorem keyAt_write (keys : View K) (values : View V) (map : SpecMap K V) (raw : RuntimeValue)
    (equal : Maps.entriesOf raw = entries keys values map) (index : Int)
    (low : 0 ≤ index) (high : index < SpecMap.size map) :
    Maps.keyAt raw index = keys.write (SpecMap.keyAt map index) := by
  have bound : index.toNat < map.length := by simp only [SpecMap.size] at high; omega
  simp only [Maps.keyAt, SpecMap.keyAt, equal, entries, List.getElem?_map,
    Int.not_lt.mpr low, if_false, List.getElem?_eq_getElem bound, Option.map_some, Option.getD_some]

omit [DecidableEq K] in
theorem size (keys : View K) (values : View V) (map : SpecMap K V) (raw : RuntimeValue)
    (equal : Maps.entriesOf raw = entries keys values map) :
    Maps.size raw = SpecMap.size map := by simp [Maps.size, equal, entries, SpecMap.size]

theorem hasKey (keys : View K) (values : View V) (map : SpecMap K V) (raw : RuntimeValue)
    (equal : Maps.entriesOf raw = entries keys values map) (key : K) :
    Maps.hasKey raw (keys.write key) = SpecMap.hasKey map key := by
  rw [Maps.hasKey, equal, contains]

theorem rank (keys : View K) (values : View V) (map : SpecMap K V) (raw : RuntimeValue)
    (equal : Maps.entriesOf raw = entries keys values map) (key : K) :
    Maps.rank raw (keys.write key) = SpecMap.rank map key := by
  rw [Maps.rank, equal, position]; rfl

theorem valueAt (keys : View K) (values : View V) (map : SpecMap K V) (raw : RuntimeValue)
    (equal : Maps.entriesOf raw = entries keys values map) (key : K) :
    values.read (Maps.valueAt raw (keys.write key)) = SpecMap.valueAt map key := by
  rw [Maps.valueAt, equal, lookup]

/-- Present reads retain the complete typed value, including aggregate values. -/
theorem valueAt_write (keys : View K) (values : View V) (map : SpecMap K V) (raw : RuntimeValue)
    (equal : Maps.entriesOf raw = entries keys values map) (key : K)
    (present : SpecMap.hasKey map key = true) :
    Maps.valueAt raw (keys.write key) = values.write (SpecMap.valueAt map key) := by
  rw [Maps.valueAt, equal]
  clear equal raw
  induction map with
  | nil => simp [SpecMap.hasKey, SpecMap.containsB] at present
  | cons entry rest ih =>
    simp only [entries, List.map_cons, Maps.get, SpecMap.valueAt, SpecMap.get,
      keys.injective.eq_iff]
    by_cases same : entry.1 = key
    · simp only [same, if_true]
    · simp only [same, if_false]
      apply ih
      simpa only [SpecMap.hasKey, SpecMap.containsB, same, decide_false, Bool.false_or] using present

omit [DecidableEq K] in
theorem ascending (keys : View K) (values : View V) (map : SpecMap K V) (ranks : ValueRanks) :
    Maps.Ascending ranks (entries keys values map) ↔ SpecMap.Ascending (keys.order ranks) map := by
  simp only [Maps.Ascending, entries, List.pairwise_map, SpecMap.Ascending, View.order]

omit [DecidableEq K] in
theorem valid (keys : View K) (values : View V) (map : SpecMap K V) (raw : RuntimeValue)
    (equal : Maps.entriesOf raw = entries keys values map) (ranks : ValueRanks) :
    Maps.Valid (.ordered ranks) raw ↔ SpecMap.Valid (.ordered (keys.order ranks)) map := by
  simp only [Maps.Valid, equal, Maps.Discipline.Valid, SpecMap.Valid, ascending]

omit [DecidableEq K] in
theorem orderedSet (keys : View K) (values : View V) (map : SpecMap K V)
    (ranks : ValueRanks) (key : K) (value : V) :
    Maps.ordSet ranks (keys.write key) (values.write value) (entries keys values map) =
      entries keys values (SpecMap.ordSet (keys.order ranks) key value map) := by
  induction map with
  | nil => rfl
  | cons entry rest ih =>
    simp only [entries, List.map_cons, Maps.ordSet, SpecMap.ordSet, View.order] at *
    split <;> simp_all only [List.map_cons]

theorem orderedDel (keys : View K) (values : View V) (map : SpecMap K V) (key : K) :
    Maps.ordDel (keys.write key) (entries keys values map) =
      entries keys values (SpecMap.ordDel key map) := by
  induction map with
  | nil => rfl
  | cons entry rest ih =>
    simp only [entries, List.map_cons, Maps.ordDel, SpecMap.ordDel] at *
    simp only [(show keys.write entry.1 = keys.write key ↔ entry.1 = key from keys.injective.eq_iff)]
    split <;> simp_all only [List.map_cons]

theorem update (keys : View K) (values : View V) (map : SpecMap K V) (raw : RuntimeValue)
    (equal : Maps.entriesOf raw = entries keys values map) (layout : Layout) (ranks : ValueRanks)
    (key : K) (value : V) :
    Maps.entriesOf (Maps.update layout (.ordered ranks) raw (keys.write key) (values.write value)) =
      entries keys values (SpecMap.update (.ordered (keys.order ranks)) map key value) := by
  simp only [Maps.update, Maps.entriesOf_build, Maps.Discipline.set_ordered, equal, orderedSet,
    SpecMap.update, SpecMap.Discipline.set]

theorem remove (keys : View K) (values : View V) (map : SpecMap K V) (raw : RuntimeValue)
    (equal : Maps.entriesOf raw = entries keys values map) (layout : Layout) (ranks : ValueRanks)
    (key : K) :
    Maps.entriesOf (Maps.remove layout (.ordered ranks) raw (keys.write key)) =
      entries keys values (SpecMap.remove (.ordered (keys.order ranks)) map key) := by
  simp only [Maps.remove, Maps.entriesOf_build, Maps.Discipline.del_ordered, equal, orderedDel,
    SpecMap.remove, SpecMap.Discipline.del]

omit [DecidableEq K] in
theorem entries_injective (keys : View K) (values : View V) : Function.Injective (entries keys values) := by
  intro a b equal
  have := congrArg (List.map fun pair => (keys.read pair.1, values.read pair.2)) equal
  simpa [entries, List.map_map, Function.comp_def, keys.read_write, values.read_write] using this

omit [DecidableEq K] in
theorem nextKey (keys : View K) (values : View V) (map : SpecMap K V) (raw : RuntimeValue)
    (model : Maps.entriesOf raw = entries keys values map) (ranks : ValueRanks) (key : K) :
    (Maps.nextKey? ranks raw (keys.write key)).map keys.read =
      SpecMap.nextKey? (keys.order ranks) map key := by
  simp [Maps.nextKey?, model, entries, List.find?_map, Option.map_map,
    SpecMap.nextKey?, View.order, Function.comp_def, keys.read_write]

omit [DecidableEq K] in
theorem prevKey (keys : View K) (values : View V) (map : SpecMap K V) (raw : RuntimeValue)
    (model : Maps.entriesOf raw = entries keys values map) (ranks : ValueRanks) (key : K) :
    (Maps.prevKey? ranks raw (keys.write key)).map keys.read =
      SpecMap.prevKey? (keys.order ranks) map key := by
  simp [Maps.prevKey?, model, entries, List.filter_map, Option.map_map,
    SpecMap.prevKey?, View.order, Function.comp_def, keys.read_write]

end VerexIR.Maps.Encoding
