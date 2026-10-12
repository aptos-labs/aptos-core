-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.TableValueBoundary

namespace LeanerIR.Tests.StateView

open Proofs

-- A minimal storage-backed value: its physical representation retains the
-- identity, while the logical value also retains the observed contents.
private def view : StateView Bool (Nat × Bool) Nat where
  erase := Prod.fst
  recover? := fun state raw => some (raw, state)
  erase_recover := by intros; cases Option.some.inj ‹_›; rfl

example : view.Realizes false (7, false) ∧ view.Realizes true (7, true) ∧
    view.erase (7, false) = view.erase (7, true) := ⟨rfl, rfl, rfl⟩

-- A mutable reference's current and prophetic snapshots use different states.
example : view.prophecy.Realizes (false, true) ((7, false), (7, true)) := by
  rw [StateView.realizes_prophecy]
  exact ⟨rfl, rfl⟩

-- Recovering both sides at entry would silently discard the mutation.
example : ¬view.prophecy.Realizes (false, false) ((7, false), (7, true)) := by
  rw [StateView.realizes_prophecy]
  intro represented
  have wrong := congrArg Prod.snd (Option.some.inj represented.2)
  cases wrong

-- There cannot be a context-free roundtrip which erases the contents.
example : ¬∃ codec : Codec (Nat × Bool) Nat, codec.encode = Prod.fst := by
  rintro ⟨codec, erases⟩
  have equal : (7, false) = (7, true) := codec.encode_injective (by rw [erases])
  cases congrArg Prod.snd equal

private def update : Spec Bool Unit Nat where
  ok := fun _ result final => result = 7 ∧ final = true
  aborts := fun _ _ => False

-- Successful results are recovered in the final state, never the entry state.
example : wp (view.lift update) (fun result _ => result = (7, true)) (fun _ => False) false := by
  rw [StateView.wp_lift]
  refine ⟨?_, fun _ h => h, fun h => h⟩
  rintro raw final ⟨rfl, rfl⟩
  exact ⟨(7, true), rfl, rfl⟩

example : ¬(view.lift update).ok false (7, false) true := by
  rintro ⟨raw, _, recovered⟩
  have wrong := congrArg Prod.snd (Option.some.inj recovered)
  cases wrong

-- Invalid arguments must not make a function with no outcomes verify vacuously.
example : ¬wp (view.function view (fun _ => (Spec.bottom : Spec Bool Unit Nat)) (7, false))
    (fun _ _ => True) (fun _ => True) true := by
  rw [StateView.wp_function]
  intro established
  have wrong := congrArg Prod.snd (Option.some.inj established.1)
  cases wrong

private def rejecting : StateView Bool Nat Nat where
  erase := id
  recover? := fun _ _ => none
  erase_recover := by intros; contradiction

-- Missing representation of a successful result is a proof obligation.
example : ¬wp (rejecting.lift update) (fun _ _ => True) (fun _ => True) false := by
  intro established
  exact established.2.2 (.inr ⟨7, true, ⟨rfl, rfl⟩, rfl⟩)

example : (view.lift (Spec.abort () : Spec Bool Unit Nat)).aborts false () := rfl

example : ¬wp (view.lift { (Spec.bottom : Spec Bool Unit Nat) with undefined := fun _ => True })
    (fun _ _ => True) (fun _ => True) false := by
  intro established
  exact established.2.2 (.inl trivial)

open Denote SnapshotValue

private def owner : StructHandle := ⟨⟨0⟩, 0⟩
private def tableType : NTy :=
  .struct owner (.cons .bool (.cons .bool .nil)) (.cons .address .nil)
private def physical : RuntimeValue := .nominal owner none #[.address "table"]
private def snapshot (contents : Bool) : Value :=
  .table physical (some [(.bool true, .scalar (.bool contents))])

-- The real Table view recovers the current contents behind the same handle.
private theorem recover_table (unit : Validation.ValidatedUnit) (memory : Memory unit)
    (registered : tableOwner unit owner = true) (contents : Bool) :
    let state := TableMemory.write memory tableType .bool .bool "table" #[(true, contents, ())]
    (stateView (Skolems.runtime unit) tableType).recover? state physical = some (snapshot contents) := by
  change observeRuntime? (Skolems.runtime unit) _ tableType
    (@NTy.encode (Carriers.runtime unit) tableType ("table", ())) = _
  rw [observeRuntime?_encode]
  change some (observe _ tableType ("table", ())) = _
  simp [tableType, observe, registered, TableMemory.handleFields, TableMemory.read,
    TableMemory.write, TableMemory.resource, Memory.set, RuntimeValue.field,
    RuntimeValue.asString, snapshot, physical]

-- A typed-storage update returns the unchanged handle. Its lifted result carries
-- the new contents, which a client can use as an ordinary logical value.
example (unit : Validation.ValidatedUnit) (memory : Memory unit)
    (registered : tableOwner unit owner = true) :
    let action : Spec (Memory unit) Unit RuntimeValue := {
      ok := fun initial raw final => raw = physical ∧
        final = TableMemory.write initial tableType .bool .bool "table" #[(true, true, ())]
      aborts := fun _ _ => False }
    wp ((stateView (Skolems.runtime unit) tableType).lift action)
      (fun result _ => result.getValue (.bool true) = .scalar (.bool true))
      (fun _ => False) memory := by
  dsimp only
  rw [StateView.wp_lift]
  refine ⟨?_, fun _ h => h, fun h => h⟩
  rintro raw final ⟨rfl, rfl⟩
  refine ⟨snapshot true, recover_table unit memory registered true, ?_⟩
  simp [snapshot, Value.getValue, Value.lookup?]

-- A logical snapshot cannot be substituted for another just because their
-- physical handles match. In one state the recovery relation is functional.
example (unit : Validation.ValidatedUnit) (memory : Memory unit) :
    ¬((stateView (Skolems.runtime unit) tableType).Realizes memory (snapshot false) ∧
      (stateView (Skolems.runtime unit) tableType).Realizes memory (snapshot true)) := by
  rintro ⟨before, after⟩
  have same := StateView.erase_injective_at _ memory before after
    (by simp [stateView, snapshot])
  simp [snapshot] at same

-- Generic caller instantiation does not change the representation relation.
example (frame : Skolems unit) (arguments : TypeArgs) (memory : Memory unit)
    (type : NTy) (value : Value) :
    (stateView (Skolems.instantiate arguments frame) type).Realizes memory value ↔
      (stateView frame (type.subst arguments.1)).Realizes memory value := by
  simp only [StateView.Realizes, stateView, observeRuntime?_instantiate]

-- The enriched carrier retains values after their capture state is out of scope.
example (unit : Validation.ValidatedUnit) (memory : Memory unit)
    (registered : tableOwner unit owner = true) :
    let before := TableMemory.write memory tableType .bool .bool "table" #[(true, false, ())]
    let after := TableMemory.write before tableType .bool .bool "table" #[(true, true, ())]
    let oldValue := Observed.capture (Skolems.runtime unit) before tableType ("table", ())
    let newValue := Observed.capture (Skolems.runtime unit) after tableType ("table", ())
    oldValue.physical = newValue.physical ∧
      oldValue.value.getValue (.bool true) = .scalar (.bool false) ∧
      newValue.value.getValue (.bool true) = .scalar (.bool true) := by
  dsimp only
  refine ⟨rfl, ?_, ?_⟩
  all_goals
    change (observe _ tableType ("table", ())).getValue (.bool true) = _
    simp [tableType, observe, registered, TableMemory.handleFields, TableMemory.read,
      TableMemory.write, TableMemory.resource, Memory.set, RuntimeValue.field,
      RuntimeValue.asString, Value.getValue, Value.lookup?]

-- A mixed function row carries a mutable current/final pair and a shared/plain
-- entry observation. Its physical side is the existing denotation's HList.
example (frame : Skolems unit) (before after : Memory unit)
    (current final shared : @NTy.carrier frame.toCarriers tableType) :
    (argumentRowView frame (.cons (.ref tableType) (.cons tableType .nil))).recover?
      (before, after) ((current, final), (shared, ())) =
      some ((Observed.capture frame before tableType current,
        Observed.capture frame after tableType final),
        (Observed.capture frame before tableType shared, ())) := by
  rfl

-- Re-instantiation keeps the logical observation, including nested Table data.
example (frame : Skolems unit) (arguments : TypeArgs) (type : NTy)
    (value : Observed frame (type.subst arguments.1)) :
    (Observed.toSkolem frame arguments type value).value = value.value ∧
      Observed.ofSkolem frame arguments type (Observed.toSkolem frame arguments type value) = value :=
  ⟨rfl, Observed.ofSkolem_toSkolem frame arguments type value⟩

-- A returned tuple has independently observed reference components. An escaping
-- reference's prophecy uses the supplied resolved memory, not its current view.
example (frame : Skolems unit) (currentMemory prophecyMemory : Memory unit)
    (current prophecy plain : @NTy.carrier frame.toCarriers tableType) :
    (resultView frame (.one (.tuple (.cons (.ref tableType) (.cons tableType .nil))))).recover?
      (currentMemory, prophecyMemory) ((current, prophecy), (plain, ())) =
      some ((Observed.capture frame currentMemory tableType current,
        Observed.capture frame prophecyMemory tableType prophecy),
        (Observed.capture frame currentMemory tableType plain, ())) := by
  rfl

example (frame : Skolems unit) (currentMemory prophecyMemory : Memory unit) :
    (resultView frame .none).recover? (currentMemory, prophecyMemory) () = some () := rfl

end LeanerIR.Tests.StateView
