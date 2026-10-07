-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.SnapshotValue

/-!
# Storage read by a Table-bearing value

A frame for an owned Table must account for its nested Tables, not just the
outer handle. These footprints name the typed storage slots an observation
reads. They confer no ownership by themselves: the contract adapter must
select values the function owns or may mutate, and account for allocation
separately. In particular, a shared reference is not a write permission.
-/

namespace LeanerIR.Proofs.Denote.SnapshotValue

abbrev Footprint := List (ResourceType × StorageKey)

variable {unit : Validation.ValidatedUnit}

local notation "runtimeEncode" => @NTy.encode (Carriers.runtime unit)
local notation "runtimeCarrier" => @NTy.carrier (Carriers.runtime unit)
local notation "runtimeRow" => @HList (Carriers.runtime unit)
local notation "runtimeVariant" => @variantCarrier (Carriers.runtime unit)

mutual
/-- Every slot read by `observe`, including Tables nested in stored values.
An absent Table still names its slot: making it present changes observation.
Function values are observed as physical values, not by executing their captures. -/
noncomputable def footprint (memory : Memory unit) :
    (type : NTy) → runtimeCarrier type → Footprint
  | .tuple elements, value => footprintRow memory elements value
  | .struct source arguments fields, value =>
      match arguments with
      | .cons key (.cons element .nil) =>
          if tableOwner unit source && TableMemory.handleFields fields then
            let owner := NTy.struct source arguments fields
            let handle := ((runtimeEncode owner value).field 0).asString
            (TableMemory.resource owner key element, .address handle) ::
              ((TableMemory.read memory owner key element handle).toList.flatMap fun entries =>
                entries.toList.flatMap fun entry => footprint memory element entry.2.1)
          else footprintRow memory fields value
      | _ => footprintRow memory fields value
  | .enum _ _ names rows _, value => footprintVariant memory names rows value
  | .vector element, value => value.values.toList.flatMap (footprint memory element)
  | .ref referent, value => footprint memory referent value.1 ++ footprint memory referent value.2
  | .unit, _ | .bool, _ | .int .., _ | .address, _ | .signer, _
  | .string, _ | .bytes, _ | .param _, _ | .function .., _ => []

noncomputable def footprintRow (memory : Memory unit) :
    (row : NRow) → runtimeRow row → Footprint
  | .nil, _ => []
  | .cons type rest, (value, values) =>
      footprint memory type value ++ footprintRow memory rest values

noncomputable def footprintVariant (memory : Memory unit) :
    (names : List String) → (rows : NRows) → runtimeVariant names rows → Footprint
  | _, .nil, value => nomatch value
  | [], .cons _ _, value => nomatch value
  | _ :: _, .cons fields _, .inl value => footprintRow memory fields value
  | _ :: names, .cons _ rest, .inr value => footprintVariant memory names rest value
end

/-- Agreement on an observation's dependencies, not permission to modify them. -/
def AgreesOn (before after : Memory unit) (slots : Footprint) : Prop :=
  ∀ resource key, (resource, key) ∈ slots → before resource key = after resource key

theorem AgreesOn.mono {before after : Memory unit} {small large : Footprint}
    (agreement : AgreesOn before after large) (included : ∀ slot ∈ small, slot ∈ large) :
    AgreesOn before after small := fun resource key member =>
  agreement resource key (included _ member)

theorem AgreesOn.append_left {before after : Memory unit} {left right : Footprint}
    (agreement : AgreesOn before after (left ++ right)) : AgreesOn before after left :=
  agreement.mono fun _ member => List.mem_append_left _ member

theorem AgreesOn.append_right {before after : Memory unit} {left right : Footprint}
    (agreement : AgreesOn before after (left ++ right)) : AgreesOn before after right :=
  agreement.mono fun _ member => List.mem_append_right _ member

theorem AgreesOn.flatMap {before after : Memory unit} {values : List α}
    {slots : α → Footprint} (agreement : AgreesOn before after (values.flatMap slots))
    {value : α} (member : value ∈ values) : AgreesOn before after (slots value) :=
  agreement.mono fun _ contained => List.mem_flatMap.mpr ⟨value, member, contained⟩

private theorem observe_table_congr (before after : Memory unit) (source : StructHandle)
    (key element : NTy) (fields : NRow) (value : runtimeRow fields)
    (registered : tableOwner unit source = true) (layout : TableMemory.handleFields fields = true)
    (same : TableMemory.read before (.struct source (.cons key (.cons element .nil)) fields)
        key element ((runtimeEncode (.struct source (.cons key (.cons element .nil)) fields)
          value).field 0).asString =
      TableMemory.read after (.struct source (.cons key (.cons element .nil)) fields)
        key element ((runtimeEncode (.struct source (.cons key (.cons element .nil)) fields)
          value).field 0).asString)
    (children : ∀ entries, TableMemory.read before
        (.struct source (.cons key (.cons element .nil)) fields) key element
        ((runtimeEncode (.struct source (.cons key (.cons element .nil)) fields)
          value).field 0).asString = some entries →
      ∀ entry ∈ entries.toList, observe before element entry.2.1 = observe after element entry.2.1) :
    observe before (.struct source (.cons key (.cons element .nil)) fields) value =
      observe after (.struct source (.cons key (.cons element .nil)) fields) value := by
  rw [observe_table before source key element fields value registered layout,
      observe_table after source key element fields value registered layout]
  dsimp only
  rw [← same]
  apply congrArg (Value.table _)
  apply Option.map_congr
  intro entries stored
  apply List.map_congr_left
  intro entry member
  exact congrArg (Prod.mk (runtimeEncode key entry.1)) (children entries stored entry member)

mutual
/-- The contents of every nested Table are stable if all slots in the
pre-state observation's footprint are preserved. The pre-state footprint is
sufficient because it includes each slot used to discover the next children. -/
theorem observe_eq_of_agrees (before after : Memory unit) :
    (type : NTy) → (value : runtimeCarrier type) →
    AgreesOn before after (footprint before type value) →
    observe before type value = observe after type value
  | .tuple elements, value, agreement =>
      congrArg (Value.aggregate .tuple) (observeRow_eq_of_agrees before after elements value agreement)
  | .struct source arguments fields, value, agreement => by
      cases arguments with
      | nil =>
          exact congrArg (Value.aggregate (.nominal source none))
            (observeRow_eq_of_agrees before after fields value agreement)
      | cons key rest =>
          cases rest with
          | nil =>
              exact congrArg (Value.aggregate (.nominal source none))
                (observeRow_eq_of_agrees before after fields value agreement)
          | cons element rest =>
              cases rest with
              | nil =>
                  by_cases model : (tableOwner unit source && TableMemory.handleFields fields) = true
                  · have parts : tableOwner unit source = true ∧ TableMemory.handleFields fields = true := by
                      simpa only [Bool.and_eq_true] using model
                    obtain ⟨registered, layout⟩ := parts
                    simp only [footprint, model, ↓reduceIte] at agreement
                    apply observe_table_congr before after source key element fields value registered layout
                    · exact agreement _ _ (List.mem_cons_self)
                    · intro entries stored entry member
                      apply observe_eq_of_agrees before after element entry.2.1
                      apply agreement.mono
                      intro slot contained
                      simp only [stored, Option.toList_some, List.flatMap_cons,
                        List.flatMap_nil, List.append_nil]
                      exact List.mem_cons_of_mem _ (List.mem_flatMap.mpr ⟨entry, member, contained⟩)
                  · simp only [observe, footprint, model] at agreement ⊢
                    exact congrArg (Value.aggregate (.nominal source none))
                      (observeRow_eq_of_agrees before after fields value agreement)
              | cons _ _ =>
                  exact congrArg (Value.aggregate (.nominal source none))
                    (observeRow_eq_of_agrees before after fields value agreement)
  | .enum source _ names rows _, value, agreement =>
      observeVariant_eq_of_agrees before after source names rows value agreement
  | .vector element, value, agreement => by
      apply congrArg (Value.aggregate .vector)
      apply List.map_congr_left
      intro entry member
      exact observe_eq_of_agrees before after element entry (agreement.flatMap member)
  | .ref referent, value, agreement => by
      simp only [observe]
      rw [observe_eq_of_agrees before after referent value.1 agreement.append_left,
        observe_eq_of_agrees before after referent value.2 agreement.append_right]
  | .unit, _ , _ | .bool, _, _ | .int .., _, _ | .address, _, _ | .signer, _, _
  | .string, _, _ | .bytes, _, _ | .param _, _, _ | .function .., _, _ => rfl

theorem observeRow_eq_of_agrees (before after : Memory unit) :
    (row : NRow) → (values : runtimeRow row) →
    AgreesOn before after (footprintRow before row values) →
    observeRow before row values = observeRow after row values
  | .nil, _, _ => rfl
  | .cons type rest, (value, values), agreement => by
      simp only [observeRow]
      rw [observe_eq_of_agrees before after type value agreement.append_left,
        observeRow_eq_of_agrees before after rest values agreement.append_right]

theorem observeVariant_eq_of_agrees (before after : Memory unit) (source : StructHandle) :
    (names : List String) → (rows : NRows) → (value : runtimeVariant names rows) →
    AgreesOn before after (footprintVariant before names rows value) →
    observeVariant before source names rows value = observeVariant after source names rows value
  | _, .nil, value, _ => nomatch value
  | [], .cons _ _, value, _ => nomatch value
  | name :: _, .cons fields _, .inl value, agreement =>
      congrArg (Value.aggregate (.nominal source (some name)))
        (observeRow_eq_of_agrees before after fields value agreement)
  | _ :: names, .cons _ rest, .inr value, agreement =>
      observeVariant_eq_of_agrees before after source names rest value agreement
end

/-- The state effect permitted by a set of owned or explicitly declared slots.
Allocation history is not implicitly writable; an allocator must name it too. -/
def PreservesOutside (before after : Memory unit) (writes : Footprint) : Prop :=
  ∀ resource key, (resource, key) ∉ writes → before resource key = after resource key

theorem PreservesOutside.set (before : Memory unit) (resource : ResourceType)
    (key : StorageKey) (value : Option (resource.carrier unit)) :
    PreservesOutside before (before.set resource key value) [(resource, key)] := by
  intro other otherKey outside
  by_cases same : other = resource
  · subst other
    have distinct : otherKey ≠ key := by
      intro equal
      subst otherKey
      exact outside (by simp)
    simp [Memory.set_same, distinct]
  · simp [Memory.set_other, same]

theorem PreservesOutside.trans {before middle after : Memory unit} {left right : Footprint}
    (first : PreservesOutside before middle left)
    (second : PreservesOutside middle after right) :
    PreservesOutside before after (left ++ right) := by
  intro resource key outside
  exact (first resource key fun member => outside (List.mem_append_left _ member)).trans
    (second resource key fun member => outside (List.mem_append_right _ member))

/-- A native update cannot change any observation whose read slots are outside
its frame. This also covers nested Tables and absent Table slots. -/
theorem observe_eq_of_frame (before after : Memory unit) (type : NTy)
    (value : runtimeCarrier type) (writes : Footprint)
    (frame : PreservesOutside before after writes)
    (disjoint : ∀ slot ∈ footprint before type value, slot ∉ writes) :
    observe before type value = observe after type value :=
  observe_eq_of_agrees before after type value fun resource key member =>
    frame resource key (disjoint _ member)

/-- Resolve generic parameters before collecting the slots. No program point
is involved: the memory may be any caller-bound state label. -/
noncomputable def footprintInFrame (frame : Skolems unit) (memory : Memory unit) (type : NTy)
    (value : @NTy.carrier frame.toCarriers type) : Footprint :=
  footprint memory (frame.resolve type) (frame.toRuntime type value)

theorem footprintInFrame_toSkolem (frame : Skolems unit) (arguments : TypeArgs)
    (memory : Memory unit) (type : NTy)
    (value : @NTy.carrier frame.toCarriers (type.subst arguments.1)) :
    footprintInFrame (Skolems.instantiate arguments frame) memory type
        (@NTy.toSkolem frame.toCarriers arguments type value) =
      footprintInFrame frame memory (type.subst arguments.1) value := by
  simp only [footprintInFrame, Skolems.resolve_instantiate, Skolems.toRuntime_instantiate,
    NTy.ofSkolem_toSkolem]

theorem observeInFrame_eq_of_agrees (frame : Skolems unit) (before after : Memory unit)
    (type : NTy) (value : @NTy.carrier frame.toCarriers type)
    (agreement : AgreesOn before after (footprintInFrame frame before type value)) :
    observeInFrame frame before type value = observeInFrame frame after type value :=
  observe_eq_of_agrees before after (frame.resolve type) (frame.toRuntime type value) agreement

/-- Contract binders may already be runtime-encoded. Decode at their actual
frame before traversing Table slots; malformed inputs have no footprint result. -/
noncomputable def footprintRuntime? (frame : Skolems unit) (memory : Memory unit)
    (type : NTy) (raw : RuntimeValue) : Option Footprint :=
  (@NTy.codec frame.toCarriers type).decode? raw |>.map (footprintInFrame frame memory type)

theorem footprintRuntime?_encode (frame : Skolems unit) (memory : Memory unit)
    (type : NTy) (value : @NTy.carrier frame.toCarriers type) :
    footprintRuntime? frame memory type (@NTy.encode frame.toCarriers type value) =
      some (footprintInFrame frame memory type value) := by
  simp only [footprintRuntime?, NTy.decode_encode, Option.map_some]

theorem footprintRuntime?_instantiate (frame : Skolems unit) (arguments : TypeArgs)
    (memory : Memory unit) (type : NTy) (raw : RuntimeValue) :
    footprintRuntime? (Skolems.instantiate arguments frame) memory type raw =
      footprintRuntime? frame memory (type.subst arguments.1) raw := by
  cases decoded : (@NTy.codec frame.toCarriers (type.subst arguments.1)).decode? raw with
  | some value =>
      have physical := @NTy.codec_tight frame.toCarriers (type.subst arguments.1) raw value decoded
      change @NTy.encode frame.toCarriers (type.subst arguments.1) value = raw at physical
      rw [← physical]
      have transported := @NTy.encode_toSkolem frame.toCarriers arguments type value
      rw [← transported, footprintRuntime?_encode, footprintInFrame_toSkolem,
        transported, footprintRuntime?_encode]
  | none =>
      cases inner : (@NTy.codec (Skolems.instantiate arguments frame).toCarriers type).decode? raw with
      | none => simp only [footprintRuntime?, decoded, inner, Option.map_none]
      | some value =>
          have physical := @NTy.codec_tight (Skolems.instantiate arguments frame).toCarriers
            type raw value inner
          change @NTy.encode (Skolems.instantiate arguments frame).toCarriers type value = raw at physical
          have outer := @NTy.encode_ofSkolem frame.toCarriers arguments type value
          rw [physical] at outer
          have decoded' := @NTy.decode?_of_encode frame.toCarriers (type.subst arguments.1)
            (NTy.ofSkolem arguments type value) raw outer
          rw [decoded] at decoded'
          contradiction

end LeanerIR.Proofs.Denote.SnapshotValue
