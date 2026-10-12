-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.LoanRenaming

/-!
# Operations commute with renaming loans

Each runtime operation applied to a frame and state with their loans
renamed injectively is the operation applied to the originals, renamed
(`designs/static-typing.md`, "Loan independence").
-/

namespace LeanerIR

open SemanticOperations

variable {ρ : Nat → Nat}

/-! ## Global memory -/

@[simp] theorem GlobalMap.rename_entries (ρ : Nat → Nat) (globals : GlobalMap) :
    (globals.rename ρ).entries =
      globals.entries.map fun slot => { slot with value := slot.value.rename ρ } := rfl

theorem GlobalMap.lookup_rename (globals : GlobalMap) (key : GlobalKey) :
    (globals.rename ρ).lookup key = (globals.lookup key).map (·.rename ρ) := by
  unfold GlobalMap.lookup
  simp only [GlobalMap.rename_entries, Array.find?_map, Option.map_map]
  rfl

theorem GlobalMap.erase_rename (globals : GlobalMap) (key : GlobalKey) :
    (globals.erase key).rename ρ = (globals.rename ρ).erase key := by
  unfold GlobalMap.erase GlobalMap.rename
  simp only [GlobalMap.mk.injEq]
  rw [Array.filter_map]
  rfl

private theorem insertSlot_map (ρ : Nat → Nat) (slot : GlobalSlot) :
    (entries : List GlobalSlot) →
    (GlobalMap.insertSlot slot entries).map
        (fun slot => ({ slot with value := slot.value.rename ρ } : GlobalSlot)) =
      GlobalMap.insertSlot { slot with value := slot.value.rename ρ }
        (entries.map fun slot => { slot with value := slot.value.rename ρ })
  | [] => rfl
  | head :: rest => by
      simp only [List.map_cons, GlobalMap.insertSlot]
      by_cases less : slot.key.rank < head.key.rank
      · simp only [less, ↓reduceIte, List.map_cons]
      · simp only [less, ↓reduceIte, List.map_cons, insertSlot_map ρ slot rest]

theorem GlobalMap.insert_rename (globals : GlobalMap) (key : GlobalKey) (value : RuntimeValue) :
    (globals.insert key value).rename ρ = (globals.rename ρ).insert key (value.rename ρ) := by
  unfold GlobalMap.insert
  rw [← GlobalMap.erase_rename]
  simp only [GlobalMap.rename, List.map_toArray, GlobalMap.mk.injEq, Array.toList_map]
  rw [insertSlot_map]

/-! ## Reads and writes -/

theorem readLocal?_rename (frame : RuntimeFrame) (localId : LocalId) :
    readLocal? (frame.rename ρ) localId = (readLocal? frame localId).map (·.rename ρ) := by
  unfold readLocal?
  simp only [RuntimeFrame.rename, Array.getElem?_map]
  cases frame.locals[localId.index]? with
  | none => rfl
  | some slot => cases slot <;> rfl

theorem readProjections?_rename :
    ∀ (projections : List RuntimeProjection) (value : RuntimeValue),
      readProjections? (value.rename ρ) projections =
        (readProjections? value projections).map (·.rename ρ)
  | [], value => rfl
  | .field index :: rest, value => by
      cases value <;> simp [readProjections?, Array.getElem?_map]
      rename_i fields
      cases fields[index]? <;> simp [readProjections?_rename rest]
  | .index index :: rest, value => by
      cases value <;> simp [readProjections?, Array.getElem?_map]
      all_goals (rename_i elements; cases elements[index]? <;> simp [readProjections?_rename rest])
  | .subslice start stop fromEnd :: rest, value => by
      cases value <;> simp [readProjections?]
      rename_i elements
      cases subsliceBounds? elements.size start stop fromEnd with
      | none => rfl
      | some bounds =>
          simp only [Option.bind_some]
          rw [← Array.map_extract, ← RuntimeValue.rename_vector, readProjections?_rename rest]
          rfl
  | .downcast variant :: rest, value => by
      cases value <;> simp [readProjections?]
      rename_i source actual fields
      cases actual with
      | none => rfl
      | some actual =>
          simp only
          split
          · rw [← RuntimeValue.rename_nominal, readProjections?_rename rest]
          · rfl
  | .deref :: rest, value => by
      cases value <;> simp [readProjections?, readProjections?_rename rest]

theorem writeProjections?_rename :
    ∀ (projections : List RuntimeProjection) (value replacement : RuntimeValue),
      writeProjections? (value.rename ρ) projections (replacement.rename ρ) =
        (writeProjections? value projections replacement).map (·.rename ρ)
  | [], value, replacement => rfl
  | .field index :: rest, value, replacement => by
      cases value <;> simp [writeProjections?, Array.getElem?_map]
      rename_i source variant fields
      cases fields[index]? with
      | none => rfl
      | some field =>
          simp only [Option.map_some, Option.bind_some, Function.comp_apply]
          rw [writeProjections?_rename rest]
          cases writeProjections? field rest replacement <;> simp [Array.map_setIfInBounds]
  | .index index :: rest, value, replacement => by
      cases value <;> simp [writeProjections?, Array.getElem?_map]
      all_goals
        rename_i elements
        cases elements[index]? with
        | none => rfl
        | some element =>
            simp only [Option.map_some, Option.bind_some, Function.comp_apply]
            rw [writeProjections?_rename rest]
            cases writeProjections? element rest replacement <;> simp [Array.map_setIfInBounds]
  | .subslice start stop fromEnd :: rest, value, replacement => by
      cases value <;> simp [writeProjections?]
      rename_i elements
      cases subsliceBounds? elements.size start stop fromEnd with
      | none => rfl
      | some bounds =>
          obtain ⟨first, last⟩ := bounds
          simp only [Option.bind_some]
          rw [← Array.map_extract, ← RuntimeValue.rename_vector, writeProjections?_rename rest]
          simp only [Function.comp_apply]
          cases writeProjections? (.vector (elements.extract first last)) rest replacement with
          | none => rfl
          | some updated =>
              cases updated <;> simp
  | .downcast variant :: rest, value, replacement => by
      cases value <;> simp [writeProjections?]
      rename_i source actual fields
      cases actual with
      | none => rfl
      | some actual =>
          simp only
          split
          · rw [← RuntimeValue.rename_nominal, writeProjections?_rename rest]
          · rfl
  | .deref :: rest, value, replacement => by
      cases value <;> simp [writeProjections?]
      rename_i loan current
      rw [writeProjections?_rename rest]
      cases writeProjections? current rest replacement <;> simp

@[simp] theorem RuntimeFrame.rename_locals (frame : RuntimeFrame) :
    (frame.rename ρ).locals = frame.locals.map (Option.map (·.rename ρ)) := rfl

@[simp] theorem RuntimeFrame.rename_typeInstantiation (frame : RuntimeFrame) :
    (frame.rename ρ).typeInstantiation = frame.typeInstantiation := rfl

theorem RuntimeFrame.rename_setLocal (frame : RuntimeFrame) (index : Nat)
    (value : Option RuntimeValue) :
    ({ frame with locals := frame.locals.set! index value } : RuntimeFrame).rename ρ =
      { frame.rename ρ with
        locals := (frame.rename ρ).locals.set! index (value.map (·.rename ρ)) } := by
  simp [RuntimeFrame.rename, Array.set!_eq_setIfInBounds, Array.map_setIfInBounds]

section Roots

variable {frame : RuntimeFrame} {state state₂ : RuntimeState}
  (globals_eq : state₂.globals = state.globals.rename ρ)
include globals_eq

theorem readRoot?_rename (root : RuntimePlaceRoot) :
    readRoot? (frame.rename ρ) state₂ root = (readRoot? frame state root).map (·.rename ρ) := by
  cases root with
  | «local» localId => exact readLocal?_rename frame localId
  | global key => simp only [readRoot?, globals_eq, GlobalMap.lookup_rename]

theorem readRuntimePlace?_rename (place : RuntimePlace) :
    readRuntimePlace? (frame.rename ρ) state₂ place =
      (readRuntimePlace? frame state place).map (·.rename ρ) := by
  unfold readRuntimePlace?
  rw [readRoot?_rename globals_eq]
  cases readRoot? frame state place.root with
  | none => rfl
  | some root => simp [readProjections?_rename]

theorem writeRoot?_rename (root : RuntimePlaceRoot) (value : RuntimeValue) :
    writeRoot? (frame.rename ρ) state₂ root (value.rename ρ) =
      (writeRoot? frame state root value).map fun written =>
        (written.1.rename ρ, { state₂ with globals := written.2.globals.rename ρ }) := by
  cases root with
  | «local» localId =>
      simp only [writeRoot?, RuntimeFrame.rename_locals, Array.size_map]
      split
      · rfl
      · simp only [Option.map_some, RuntimeFrame.rename_setLocal, ← globals_eq]
        rfl
  | global key =>
      simp only [writeRoot?, globals_eq, GlobalMap.lookup_rename, Option.bind_eq_bind]
      cases state.globals.lookup key with
      | none => rfl
      | some _ => simp [GlobalMap.insert_rename]

theorem writeRuntimePlace?_rename (place : RuntimePlace) (value : RuntimeValue) :
    writeRuntimePlace? (frame.rename ρ) state₂ place (value.rename ρ) =
      (writeRuntimePlace? frame state place value).map fun written =>
        (written.1.rename ρ, { state₂ with globals := written.2.globals.rename ρ }) := by
  unfold writeRuntimePlace?
  split
  · rfl
  split
  · exact writeRoot?_rename globals_eq place.root value
  · simp only [Option.bind_eq_bind]
    rw [readRoot?_rename globals_eq]
    cases readRoot? frame state place.root with
    | none => rfl
    | some root =>
        simp only [Option.map_some, Option.bind_some]
        rw [writeProjections?_rename]
        cases writeProjections? root place.projections.toList value with
        | none => rfl
        | some updated => exact writeRoot?_rename globals_eq place.root updated

end Roots

theorem writeRoot?_globals {frame frame' : RuntimeFrame} {state state' : RuntimeState}
    {root : RuntimePlaceRoot} {value : RuntimeValue}
    (write : writeRoot? frame state root value = some (frame', state')) :
    state' = { state with globals := state'.globals } := by
  cases root with
  | «local» localId =>
      simp only [writeRoot?] at write
      split at write
      · cases write
      · simp only [Option.some.injEq, Prod.mk.injEq] at write
        obtain ⟨-, rfl⟩ := write
        rfl
  | global key =>
      simp only [writeRoot?, Option.bind_eq_bind, Option.bind_eq_some_iff] at write
      obtain ⟨_, -, write⟩ := write
      simp only [Option.some.injEq, Prod.mk.injEq] at write
      obtain ⟨-, rfl⟩ := write
      rfl

theorem writeRuntimePlace?_globals {frame frame' : RuntimeFrame} {state state' : RuntimeState}
    {place : RuntimePlace} {value : RuntimeValue}
    (write : writeRuntimePlace? frame state place value = some (frame', state')) :
    state' = { state with globals := state'.globals } := by
  unfold writeRuntimePlace? at write
  split at write
  · cases write
  split at write
  · exact writeRoot?_globals write
  · simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at write
    obtain ⟨_, -, _, -, write⟩ := write
    exact writeRoot?_globals write

/-! ## Place resolution

A resolved place holds no loan, and resolution reads values only for their
shapes and integer indices, which shifts keep. -/

theorem resolvePlaceFuel?_rename (unit : Validation.ValidatedUnit) (ns : Validation.ValidatedNamespace)
    {frame : RuntimeFrame} {state state₂ : RuntimeState}
    (globals_eq : state₂.globals = state.globals.rename ρ) :
    ∀ fuel : Nat,
      (∀ placeId, resolvePlaceFuel? unit ns (frame.rename ρ) state₂ fuel placeId =
        resolvePlaceFuel? unit ns frame state fuel placeId) ∧
      (∀ expressionId, simpleIndexFuel? unit ns (frame.rename ρ) state₂ fuel expressionId =
        simpleIndexFuel? unit ns frame state fuel expressionId)
  | 0 => ⟨fun _ => by simp [resolvePlaceFuel?], fun _ => by simp [simpleIndexFuel?]⟩
  | fuel + 1 => by
      obtain ⟨places, indices⟩ := resolvePlaceFuel?_rename unit ns (frame := frame) globals_eq fuel
      have read := fun place => readRuntimePlace?_rename (frame := frame) globals_eq place
      constructor
      · intro placeId
        simp only [resolvePlaceFuel?, Option.bind_eq_bind]
        cases ns.places[placeId.index]? with
        | none => rfl
        | some place =>
            simp only [Option.bind_some]
            cases place with
            | localVar localId => simp [RuntimeFrame.rename_locals]
            | deref base =>
                simp only [places]
                split
                · rfl
                · cases resolvePlaceFuel? unit ns frame state fuel base with
                  | none => rfl
                  | some basePlace =>
                      simp only [Option.bind_some, read]
                      cases readRuntimePlace? frame state basePlace with
                      | none => rfl
                      | some value => cases value <;> simp
            | field base owner field =>
                simp only [places]
                cases resolvePlaceFuel? unit ns frame state fuel base with
                | none => rfl
                | some basePlace =>
                    simp only [Option.bind_some, read]
                    cases readRuntimePlace? frame state basePlace with
                    | none => rfl
                    | some value => cases value <;> simp
            | index base indexExpression =>
                simp only [places, indices]
                cases resolvePlaceFuel? unit ns frame state fuel base with
                | none => rfl
                | some basePlace =>
                    simp only [Option.bind_some]
                    cases simpleIndexFuel? unit ns frame state fuel indexExpression with
                    | none => rfl
                    | some index =>
                        simp only [Option.bind_some, read]
                        cases readRuntimePlace? frame state basePlace with
                        | none => rfl
                        | some value => cases value <;> simp
            | subslice base start stop fromEnd => simp only [places]
            | downcast base variant =>
                simp only [places]
                cases resolvePlaceFuel? unit ns frame state fuel base with
                | none => rfl
                | some basePlace =>
                    simp only [Option.bind_some, read]
                    cases readRuntimePlace? frame state basePlace with
                    | none => rfl
                    | some value =>
                        cases value with
                        | nominal source actualVariant fields =>
                            cases actualVariant <;> simp
                        | _ => simp
      · intro expressionId
        simp only [simpleIndexFuel?, Option.bind_eq_bind]
        cases Validation.placeIndexForm? ns expressionId with
        | none => rfl
        | some form =>
            cases form with
            | literal value => rfl
            | «local» localId | copyLocal localId =>
                simp only [readLocal?_rename]
                cases readLocal? frame localId with
                | none => rfl
                | some value => cases value <;> simp
            | fromEnd sourcePlace sourceOffset =>
                simp only [places]
                cases resolvePlaceFuel? unit ns frame state fuel sourcePlace with
                | none => rfl
                | some resolved =>
                    simp only [Option.bind_some, read]
                    cases readRuntimePlace? frame state resolved with
                    | none => rfl
                    | some value => cases value <;> simp

theorem resolvePlace?_rename (unit : Validation.ValidatedUnit) (ns : Validation.ValidatedNamespace)
    {frame : RuntimeFrame} {state state₂ : RuntimeState}
    (globals_eq : state₂.globals = state.globals.rename ρ) (placeId : PlaceId) :
    resolvePlace? unit ns (frame.rename ρ) state₂ placeId =
      resolvePlace? unit ns frame state placeId :=
  (resolvePlaceFuel?_rename unit ns globals_eq _).1 placeId

/-- Whether a place mismatches a variant reads shapes only, which shifts keep. -/
theorem placeVariantMismatchFuel?_rename (unit : Validation.ValidatedUnit)
    (ns : Validation.ValidatedNamespace) {frame : RuntimeFrame} {state state₂ : RuntimeState}
    (globals_eq : state₂.globals = state.globals.rename ρ) :
    ∀ fuel placeId, placeVariantMismatchFuel? unit ns (frame.rename ρ) state₂ fuel placeId =
      placeVariantMismatchFuel? unit ns frame state fuel placeId
  | 0, _ => rfl
  | fuel + 1, placeId => by
      have ih := placeVariantMismatchFuel?_rename unit ns (frame := frame) globals_eq fuel
      have resolve := (resolvePlaceFuel?_rename unit ns (frame := frame) globals_eq fuel).1
      have read := fun place => readRuntimePlace?_rename (frame := frame) globals_eq place
      simp only [placeVariantMismatchFuel?]
      cases ns.places[placeId.index]? with
      | none => rfl
      | some place =>
          cases place with
          | localVar => rfl
          | field base owner field =>
              simp only [resolve, ih]
              cases resolvePlaceFuel? unit ns frame state fuel base with
              | none => rfl
              | some basePlace =>
                  simp only [read]
                  cases readRuntimePlace? frame state basePlace with
                  | none => rfl
                  | some value =>
                      cases value <;> try simp
                      rename_i source variant values
                      cases variant <;> cases resolveStruct? unit ns.identity owner <;>
                        cases sourceFieldName? ns field <;> rfl
          | downcast base variant =>
              simp only [resolve, ih]
              cases resolvePlaceFuel? unit ns frame state fuel base with
              | none => rfl
              | some basePlace =>
                  simp only [read]
                  cases readRuntimePlace? frame state basePlace with
                  | none => rfl
                  | some value =>
                      cases value <;> try simp
                      rename_i source actual values
                      cases actual <;> cases sourceFieldName? ns variant <;> rfl
          | deref base => simp only [resolve, ih]
          | index base _ => simp only [resolve, ih]
          | subslice base _ _ _ => simp only [resolve, ih]

theorem variantMismatch?_rename (unit : Validation.ValidatedUnit)
    (ns : Validation.ValidatedNamespace) (operation : Operation) (arguments : Array RuntimeValue)
    {frame : RuntimeFrame} {state state₂ : RuntimeState}
    (globals_eq : state₂.globals = state.globals.rename ρ) :
    variantMismatch? unit ns operation (arguments.map (·.rename ρ)) (frame.rename ρ)
        state₂ = variantMismatch? unit ns operation arguments frame state := by
  cases operation with
  | data kind =>
      cases kind with
      | selectVariants reference fields =>
          simp only [variantMismatch?, Array.toList_map]
          rcases arguments.toList with _ | ⟨a, _ | ⟨b, rest⟩⟩
          · rfl
          · cases a <;> try simp
            rename_i source variant values
            cases variant <;> cases resolveStruct? unit ns.identity reference <;> rfl
          · simp only [List.map_cons]
            cases a <;> simp only [RuntimeValue.rename]
      | _ => rfl
  | copy place | read place | move place | borrow _ place | write place | drop place =>
      simp only [variantMismatch?, placeVariantMismatchFuel?_rename unit ns globals_eq]
  | _ => rfl

/-! ## Constants -/

theorem List.mapM_some_forall {α β : Type} {f : α → Option β} {P : β → Prop} :
    ∀ {xs : List α} {ys : List β}, xs.mapM f = some ys →
      (∀ x ∈ xs, ∀ y, f x = some y → P y) → ∀ y ∈ ys, P y
  | [], ys, mapped, _ => by
      simp only [List.mapM_nil, Option.pure_def, Option.some.injEq] at mapped
      subst mapped; simp
  | x :: xs, ys, mapped, each => by
      simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
        Option.some.injEq] at mapped
      obtain ⟨y, y_eq, rest, rest_eq, rfl⟩ := mapped
      intro z member
      rcases List.mem_cons.mp member with rfl | member
      · exact each x (by simp) z y_eq
      · exact List.mapM_some_forall rest_eq (fun x member => each x (by simp [member])) z member

theorem constValue?_plain : ∀ (fuel : Nat) (literal : ConstValue), sizeOf literal ≤ fuel →
    ∀ {value : RuntimeValue}, constValue? literal = some value → Plain value
  | 0, literal, small, _, _ => by cases literal <;> simp at small
  | fuel + 1, literal, small, value, value_eq => by
      have elements : ∀ (literals : Array ConstValue), sizeOf literals ≤ fuel + 1 →
          ∀ {values : Array RuntimeValue},
            literals.attach.mapM (fun ⟨element, _⟩ => constValue? element) = some values →
            ∀ value ∈ values, Plain value := by
        intro literals size values mapped value member
        rw [Array.mapM_eq_mapM_toList, Array.toList_attach] at mapped
        simp only [Functor.map, Option.map_eq_some_iff] at mapped
        obtain ⟨list, list_eq, rfl⟩ := mapped
        refine List.mapM_some_forall list_eq (fun x member y y_eq => ?_) value (by simpa using member)
        obtain ⟨element, element_in⟩ := x
        have := List.sizeOf_lt_of_mem (Array.mem_def.mp element_in)
        cases literals
        exact constValue?_plain fuel element (by simp at size this; omega) y_eq
      cases literal with
      | vector literals =>
          simp only [constValue?, Functor.map, Option.map_eq_some_iff] at value_eq
          obtain ⟨values, values_eq, rfl⟩ := value_eq
          exact .vector _ (elements literals (by simp at small; omega) values_eq)
      | tuple literals =>
          simp only [constValue?, Functor.map, Option.map_eq_some_iff] at value_eq
          obtain ⟨values, values_eq, rfl⟩ := value_eq
          exact .tuple _ (elements literals (by simp at small; omega) values_eq)
      | character value =>
          simp only [constValue?] at value_eq
          split at value_eq
          · cases value_eq; exact .character _
          · cases value_eq
      | profile _ => simp [constValue?] at value_eq
      | unit | bool _ | integer _ | address _ | string _ | bytes _ =>
          simp only [constValue?, Option.some.injEq] at value_eq
          subst value_eq
          constructor

/-! ## Patterns -/

theorem bindPatternRow_rename
    {bind bind' : PatternId → RuntimeValue → RuntimeFrame → Option RuntimeFrame}
    (commutes : ∀ pattern value frame, bind' pattern (value.rename ρ) (frame.rename ρ) =
      (bind pattern value frame).map (·.rename ρ)) :
    ∀ (patterns : List PatternId) (values : List RuntimeValue) (frame : RuntimeFrame),
      bindPatternRow bind' patterns (values.map (·.rename ρ)) (frame.rename ρ) =
        (bindPatternRow bind patterns values frame).map (·.rename ρ)
  | [], [], frame => rfl
  | [], _ :: _, _ => rfl
  | _ :: _, [], _ => rfl
  | pattern :: patterns, value :: values, frame => by
      simp only [List.map_cons, bindPatternRow, Option.bind_eq_bind, commutes]
      cases bind pattern value frame with
      | none => rfl
      | some bound => exact bindPatternRow_rename commutes patterns values bound

theorem bindPatternFuel_rename (injective : Function.Injective ρ) (unit : Validation.ValidatedUnit)
    (ns : Validation.ValidatedNamespace) :
    ∀ (fuel : Nat) (frame : RuntimeFrame) (patternId : PatternId) (value : RuntimeValue),
      bindPatternFuel unit ns (frame.rename ρ) fuel patternId (value.rename ρ) =
        (bindPatternFuel unit ns frame fuel patternId value).map (·.rename ρ)
  | 0, _, _, _ => rfl
  | fuel + 1, frame, patternId, value => by
      have row := bindPatternRow_rename (ρ := ρ)
        (bind := fun pattern value frame => bindPatternFuel unit ns frame fuel pattern value)
        (bind' := fun pattern value frame => bindPatternFuel unit ns frame fuel pattern value)
        (fun pattern value frame => bindPatternFuel_rename injective unit ns fuel frame pattern value)
      simp only [bindPatternFuel, Option.bind_eq_bind]
      cases ns.patterns[patternId.index]? with
      | none => rfl
      | some pattern =>
          simp only [Option.bind_some]
          generalize pattern.kind = kind
          cases kind <;> dsimp only
          case wildcard => rfl
          case «variable» localId =>
              simp only [RuntimeFrame.rename_locals, Array.size_map]
              split
              · simp [RuntimeFrame.rename, Array.map_setIfInBounds, pure]
              · rfl
          case tuple elements =>
              cases value <;> simp only [RuntimeValue.rename_tuple, RuntimeValue.rename_unit, RuntimeValue.rename_bool,
                RuntimeValue.rename_character, RuntimeValue.rename_integer, RuntimeValue.rename_address,
                RuntimeValue.rename_signer, RuntimeValue.rename_string, RuntimeValue.rename_bytes,
                RuntimeValue.rename_vector, RuntimeValue.rename_nominal, RuntimeValue.rename_closure,
                RuntimeValue.rename_borrow, RuntimeValue.rename_loanHole] <;> try rfl
              case tuple values =>
                simp only [Array.size_map]
                split
                · rfl
                · simp only [Array.toList_map]; exact row elements.toList values.toList frame
          case literal literal =>
              cases expected_eq : constValue? literal with
              | none => rfl
              | some expected =>
                  have plain := constValue?_plain _ literal (Nat.le_refl _) expected_eq
                  simp only [Option.bind_some]
                  rw [← RuntimeValue.rename_of_plain ρ plain, RuntimeValue.rename_beq injective,
                    RuntimeValue.rename_of_plain ρ plain]
                  split <;> rfl
          case range lower upper inclusive =>
              cases value <;> simp only [RuntimeValue.rename_tuple, RuntimeValue.rename_unit, RuntimeValue.rename_bool,
                RuntimeValue.rename_character, RuntimeValue.rename_integer, RuntimeValue.rename_address,
                RuntimeValue.rename_signer, RuntimeValue.rename_string, RuntimeValue.rename_bytes,
                RuntimeValue.rename_vector, RuntimeValue.rename_nominal, RuntimeValue.rename_closure,
                RuntimeValue.rename_borrow, RuntimeValue.rename_loanHole] <;> try rfl
              all_goals (repeat' split) <;> simp_all
          case constructor name spelled variant fields =>
              cases value <;> simp only [RuntimeValue.rename_tuple, RuntimeValue.rename_unit, RuntimeValue.rename_bool,
                RuntimeValue.rename_character, RuntimeValue.rename_integer, RuntimeValue.rename_address,
                RuntimeValue.rename_signer, RuntimeValue.rename_string, RuntimeValue.rename_bytes,
                RuntimeValue.rename_vector, RuntimeValue.rename_nominal, RuntimeValue.rename_closure,
                RuntimeValue.rename_borrow, RuntimeValue.rename_loanHole] <;> try rfl
              case nominal source actualVariant values =>
                cases resolveNominal? unit ns name with
                | none => rfl
                | some expected =>
                    simp only [Option.bind_some, Array.size_map]
                    by_cases mismatch : (expected != source || variant != actualVariant ||
                        fields.size != values.size) = true
                    · simp only [mismatch, if_true, Option.map_none]
                    · simp only [mismatch, Bool.false_eq_true, if_false, Array.toList_map]
                      exact row fields.toList values.toList frame

theorem bindPattern_rename (injective : Function.Injective ρ) (unit : Validation.ValidatedUnit)
    (ns : Validation.ValidatedNamespace)
    (frame : RuntimeFrame) (patternId : PatternId) (value : RuntimeValue) :
    bindPattern unit ns (frame.rename ρ) patternId (value.rename ρ) =
      (bindPattern unit ns frame patternId value).map (·.rename ρ) :=
  bindPatternFuel_rename injective unit ns _ frame patternId value

theorem bindPattern_within {β γ : Nat → Nat} (base : LoanBase β γ inside)
    {unit : Validation.ValidatedUnit} {ns : Validation.ValidatedNamespace}
    {frame frame' : RuntimeFrame} (frame_within : frame.Within inside)
    {value : RuntimeValue} (value_within : value.Within inside) {patternId : PatternId}
    (bind : bindPattern unit ns frame patternId value = some frame') : frame'.Within inside := by
  rw [← base.rename_inverse_frame frame_within, ← base.rename_inverse value value_within,
    bindPattern_rename base.injective] at bind
  obtain ⟨original, -, rfl⟩ := Option.map_eq_some_iff.mp bind
  exact base.within_frame original

/-! ## Loan bookkeeping -/

theorem Nat.add_beq_add (left right shift : Nat) :
    (left + shift == right + shift) = (left == right) := by
  rw [Bool.eq_iff_iff]; simp

theorem indexOfFrom_map {α β : Type} (accepts : β → Bool) (f : α → β) :
    ∀ (values : List α) (index : Nat),
      indexOfFrom accepts (values.map f) index = indexOfFrom (accepts ∘ f) values index
  | [], _ => rfl
  | value :: values, index => by
      simp only [List.map_cons, indexOfFrom, Function.comp_apply]
      split <;> simp [indexOfFrom_map accepts f values]

theorem Array.findRev?_map' {α β : Type} (p : β → Bool) (f : α → β) (values : Array α) :
    (values.map f).findRev? p = (values.findRev? (p ∘ f)).map f := by
  simp only [Array.findRev?_eq_find?_reverse, ← Array.map_reverse, Array.find?_map]

@[simp] theorem RuntimeFrame.rename_activeLoans (frame : RuntimeFrame) :
    (frame.rename ρ).activeLoans =
      frame.activeLoans.map fun entry => (entry.1, ρ entry.2) := rfl

@[simp] theorem RuntimeFrame.rename_loanLocations (frame : RuntimeFrame) :
    (frame.rename ρ).loanLocations = frame.loanLocations.map (renameEntry ρ) := rfl

theorem localLoanPlace?_rename (injective : Function.Injective ρ) (frame : RuntimeFrame) (loan : Nat) :
    localLoanPlace? (frame.rename ρ) (ρ loan) = localLoanPlace? frame loan := by
  unfold localLoanPlace?
  rw [RuntimeFrame.rename_loanLocations, Array.findRev?_map']
  have same : ((fun entry : Nat × RuntimePlace => entry.1 == ρ loan) ∘ renameEntry ρ) =
      fun entry => entry.1 == loan := by
    funext entry; simp [injective.beq_eq]
  rw [same]
  cases frame.loanLocations.findRev? (·.1 == loan) <;> rfl

theorem holeWithin_rename (injective : Function.Injective ρ) (loan : Nat) (value : RuntimeValue) :
    holeWithin (ρ loan) (value.rename ρ) = holeWithin loan value := by
  unfold holeWithin
  rw [findFirst_rename (g := holeMark? loan) (φ := id) (fun value => holeMark?_rename injective loan value)]
  simp

theorem holeInFrame_rename (injective : Function.Injective ρ) (frame : RuntimeFrame) (loan : Nat) :
    holeInFrame (frame.rename ρ) (ρ loan) = holeInFrame frame loan := by
  unfold holeInFrame
  rw [RuntimeFrame.rename_locals, Array.toList_map, indexOfFrom_map]
  congr 2
  funext slot
  cases slot <;> simp [holeWithin_rename injective]

theorem fillHole?_rename (injective : Function.Injective ρ) (loan : Nat) (replacement value : RuntimeValue) :
    fillHole? (ρ loan) (replacement.rename ρ) (value.rename ρ) =
      (fillHole? loan replacement value).map (·.rename ρ) :=
  rewriteFirst_rename (fun value => holeFill?_rename injective loan replacement value) value

theorem transferredLoan?_rename (replacement : RuntimeValue) :
    transferredLoan? (replacement.rename ρ) = (transferredLoan? replacement).map ρ :=
  findFirst_rename (fun value => anyHole?_rename value) replacement

theorem transferActiveLoan_rename (injective : Function.Injective ρ) (activeLoans : Array (ExprId × Nat)) (loan : Nat)
    (replacement : RuntimeValue) :
    transferActiveLoan (activeLoans.map fun entry => (entry.1, ρ entry.2)) (ρ loan)
        (replacement.rename ρ) =
      (transferActiveLoan activeLoans loan replacement).map fun entry =>
        (entry.1, ρ entry.2) := by
  unfold transferActiveLoan
  rw [transferredLoan?_rename, Array.findRev?_map', Array.filter_map]
  have keep : ((fun entry : ExprId × Nat => entry.2 != ρ loan) ∘
      fun entry : ExprId × Nat => (entry.1, ρ entry.2)) = fun entry => entry.2 != loan := by
    funext entry; simp [bne, injective.beq_eq]
  have found : ((fun entry : ExprId × Nat => entry.2 == ρ loan) ∘
      fun entry : ExprId × Nat => (entry.1, ρ entry.2)) = fun entry => entry.2 == loan := by
    funext entry; simp [injective.beq_eq]
  rw [keep, found]
  cases transferredLoan? replacement <;> cases activeLoans.findRev? (·.2 == loan) <;>
    simp [Array.map_push]

theorem transferLoanLocation_rename (injective : Function.Injective ρ) (loanLocations : Array (Nat × RuntimePlace)) (loan : Nat)
    (replacement : RuntimeValue) :
    transferLoanLocation (loanLocations.map (renameEntry ρ)) (ρ loan)
        (replacement.rename ρ) =
      (transferLoanLocation loanLocations loan replacement).map (renameEntry ρ) := by
  unfold transferLoanLocation
  rw [transferredLoan?_rename, Array.findRev?_map', Array.filter_map]
  have keep : ((fun entry : Nat × RuntimePlace => entry.1 != ρ loan) ∘
      renameEntry ρ) = fun entry => entry.1 != loan := by
    funext entry; simp [bne, injective.beq_eq]
  have found : ((fun entry : Nat × RuntimePlace => entry.1 == ρ loan) ∘
      renameEntry ρ) = fun entry => entry.1 == loan := by
    funext entry; simp [injective.beq_eq]
  rw [keep, found]
  cases transferredLoan? replacement <;> cases loanLocations.findRev? (·.1 == loan) <;>
    simp [Array.map_push, renameEntry]

/-! ## The loan registry -/

section Registry

variable {inside : Nat → Prop}

theorem storageLoanTargetIn?_append (first second : List (Nat × LoanTarget)) (loan : Nat) :
    storageLoanTargetIn? (first ++ second) loan =
      (storageLoanTargetIn? first loan).or (storageLoanTargetIn? second loan) := by
  unfold storageLoanTargetIn?
  rw [List.find?_append]
  cases first.find? (·.1 == loan) <;> rfl

theorem storageLoanTargetIn?_outside {junk : List (Nat × LoanTarget)} {loan : Nat}
    (outside : ∀ entry ∈ junk, entry.1 ≠ loan) :
    storageLoanTargetIn? junk loan = none := by
  unfold storageLoanTargetIn?
  rw [Option.map_eq_none_iff, List.find?_eq_none]
  intro entry member
  simpa only [beq_iff_eq] using outside entry member

theorem storageLoanTargetIn?_map_rename (injective : Function.Injective ρ)
    (held : List (Nat × LoanTarget)) (loan : Nat) :
    storageLoanTargetIn? (held.map (renameEntry ρ)) (ρ loan) =
      storageLoanTargetIn? held loan := by
  unfold storageLoanTargetIn?
  rw [List.find?_map, Option.map_map]
  have same : ((fun entry : Nat × LoanTarget => entry.1 == ρ loan) ∘ renameEntry ρ) =
      fun entry => entry.1 == loan := by
    funext entry; simp [injective.beq_eq]
  rw [same]
  rfl

/-- The junk of a first run's registry holds no loan the run holds. -/
theorem RegistryRenamed.junk_free {junk : List (Nat × LoanTarget)}
    (outside : ∀ entry ∈ junk, ¬inside entry.1) {loan : Nat} (within : inside loan) :
    storageLoanTargetIn? junk loan = none :=
  storageLoanTargetIn?_outside fun entry member same => outside entry member (same ▸ within)

/-- The junk of a second run's registry holds no renaming of a loan the
first holds. -/
theorem RegistryRenamed.junk_free' {junk' : List (Nat × LoanTarget)}
    (outside : ∀ entry ∈ junk', ∀ loan, inside loan → ρ loan ≠ entry.1) {loan : Nat}
    (within : inside loan) : storageLoanTargetIn? junk' (ρ loan) = none :=
  storageLoanTargetIn?_outside fun entry member same => outside entry member loan within same.symm

theorem RegistryRenamed.lookup (injective : Function.Injective ρ)
    {registry registry' : List (Nat × LoanTarget)}
    (shifted : RegistryRenamed ρ inside registry registry') {loan : Nat}
    (within : inside loan) :
    storageLoanTargetIn? registry' (ρ loan) = storageLoanTargetIn? registry loan := by
  obtain ⟨held, junk, junk', rfl, rfl, -, outside, outside'⟩ := shifted
  rw [storageLoanTargetIn?_append, storageLoanTargetIn?_append,
    storageLoanTargetIn?_map_rename injective, RegistryRenamed.junk_free outside within,
    RegistryRenamed.junk_free' outside' within]

theorem removeStorageLoan_append_free (held junk : List (Nat × LoanTarget)) {loan : Nat}
    (free : storageLoanTargetIn? junk loan = none) :
    removeStorageLoan (held ++ junk) loan = removeStorageLoan held loan ++ junk := by
  induction held with
  | nil => simp [removeStorageLoan_of_free junk loan free, removeStorageLoan]
  | cons entry rest ih =>
      simp only [List.cons_append, removeStorageLoan]
      split <;> simp [ih]

theorem removeStorageLoan_map_rename (injective : Function.Injective ρ)
    (held : List (Nat × LoanTarget)) (loan : Nat) :
    removeStorageLoan (held.map (renameEntry ρ)) (ρ loan) =
      (removeStorageLoan held loan).map (renameEntry ρ) := by
  induction held with
  | nil => rfl
  | cons entry rest ih =>
      simp only [List.map_cons, removeStorageLoan, renameEntry, injective.beq_eq]
      split <;> simp [ih, renameEntry]

theorem removeStorageLoan_sublist (registry : List (Nat × LoanTarget)) (loan : Nat) :
    ∀ entry ∈ removeStorageLoan registry loan, entry ∈ registry := by
  induction registry with
  | nil => simp [removeStorageLoan]
  | cons head rest ih =>
      intro entry member
      simp only [removeStorageLoan] at member
      split at member
      · exact List.mem_cons_of_mem _ member
      · rcases List.mem_cons.mp member with rfl | member
        · exact List.mem_cons_self
        · exact List.mem_cons_of_mem _ (ih entry member)

theorem RegistryRenamed.remove (injective : Function.Injective ρ)
    {registry registry' : List (Nat × LoanTarget)}
    (shifted : RegistryRenamed ρ inside registry registry') {loan : Nat}
    (within : inside loan) :
    RegistryRenamed ρ inside (removeStorageLoan registry loan)
      (removeStorageLoan registry' (ρ loan)) := by
  obtain ⟨held, junk, junk', rfl, rfl, held_within, outside, outside'⟩ := shifted
  refine ⟨removeStorageLoan held loan, junk, junk',
    removeStorageLoan_append_free held junk (RegistryRenamed.junk_free outside within),
    ?_, fun entry member => held_within entry (removeStorageLoan_sublist held loan entry member),
    outside, outside'⟩
  rw [removeStorageLoan_append_free _ junk' (RegistryRenamed.junk_free' outside' within),
    removeStorageLoan_map_rename injective]

theorem RegistryRenamed.cons {registry registry' : List (Nat × LoanTarget)}
    (shifted : RegistryRenamed ρ inside registry registry') {loan : Nat}
    (within : inside loan) (key : LoanTarget) :
    RegistryRenamed ρ inside ((loan, key) :: registry) ((ρ loan, key) :: registry') := by
  obtain ⟨held, junk, junk', rfl, rfl, held_within, outside, outside'⟩ := shifted
  refine ⟨(loan, key) :: held, junk, junk', rfl, rfl, ?_, outside, outside'⟩
  intro entry member
  rcases List.mem_cons.mp member with rfl | member
  · exact within
  · exact held_within entry member

theorem transferredLoan?_within {β γ : Nat → Nat} (base : LoanBase β γ inside)
    {value : RuntimeValue} (within : value.Within inside)
    {loan : Nat} (found : transferredLoan? value = some loan) : inside loan := by
  rw [← base.rename_inverse value within, transferredLoan?_rename] at found
  obtain ⟨original, -, rfl⟩ := Option.map_eq_some_iff.mp found
  exact base.image original

theorem fillHole?_within {β γ : Nat → Nat} (base : LoanBase β γ inside)
    {loan : Nat} {replacement value result : RuntimeValue}
    (loan_within : inside loan) (replacement_within : replacement.Within inside)
    (value_within : value.Within inside)
    (filled : fillHole? loan replacement value = some result) : result.Within inside := by
  rw [← base.rename_inverse value value_within,
    ← base.rename_inverse replacement replacement_within,
    ← base.inverse loan loan_within, fillHole?_rename base.injective] at filled
  obtain ⟨original, -, rfl⟩ := Option.map_eq_some_iff.mp filled
  exact base.within original

theorem RegistryRenamed.transfer (injective : Function.Injective ρ) {β γ : Nat → Nat}
    (base : LoanBase β γ inside) {registry registry' : List (Nat × LoanTarget)}
    (shifted : RegistryRenamed ρ inside registry registry') {loan : Nat}
    (within : inside loan) (key : LoanTarget) {replacement : RuntimeValue}
    (replacement_within : replacement.Within inside) :
    RegistryRenamed ρ inside (transferStorageLoan registry loan key replacement)
      (transferStorageLoan registry' (ρ loan) key (replacement.rename ρ)) := by
  unfold transferStorageLoan
  simp only [transferredLoan?_rename]
  cases found : transferredLoan? replacement with
  | none => exact shifted.remove injective within
  | some transferred =>
      exact (shifted.remove injective within).cons
        (transferredLoan?_within base replacement_within found) key

end Registry

theorem RuntimeFrame.rename_withLoans (frame : RuntimeFrame) (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace)) :
    ({ frame with activeLoans, loanLocations } : RuntimeFrame).rename ρ =
      { frame.rename ρ with
        activeLoans := activeLoans.map fun entry => (entry.1, ρ entry.2)
        loanLocations := loanLocations.map (renameEntry ρ) } := rfl

theorem fillLocalLoanHole?_rename (injective : Function.Injective ρ) {frame : RuntimeFrame}
    {state state₂ : RuntimeState} (globals_eq : state₂.globals = state.globals.rename ρ) (loan : Nat)
    (replacement : RuntimeValue) :
    fillLocalLoanHole? (frame.rename ρ) state₂ (ρ loan) (replacement.rename ρ) =
      (fillLocalLoanHole? frame state loan replacement).map fun resolved =>
        (resolved.1.rename ρ, state₂) := by
  unfold fillLocalLoanHole?
  simp only [Option.bind_eq_bind]
  rw [localLoanPlace?_rename injective]
  cases localLoanPlace? frame loan with
  | none => rfl
  | some place =>
      simp only [Option.bind_some]
      rw [readRuntimePlace?_rename globals_eq]
      cases readRuntimePlace? frame state place with
      | none => rfl
      | some value =>
          simp only [Option.map_some, Option.bind_some]
          rw [fillHole?_rename injective]
          cases fillHole? loan replacement value with
          | none => rfl
          | some filled =>
              simp only [Option.map_some, Option.bind_some]
              rw [writeRuntimePlace?_rename globals_eq]
              cases writeRuntimePlace? frame state place filled with
              | none => rfl
              | some written =>
                  simp only [Option.map_some, Option.bind_some, RuntimeFrame.rename_activeLoans,
                    RuntimeFrame.rename_loanLocations, transferActiveLoan_rename injective,
                    transferLoanLocation_rename injective, RuntimeFrame.rename_withLoans]

/-! ## States -/

theorem Array.extract_push_size {α : Type} (values : Array α) (value : α) {start : Nat}
    (le : start ≤ values.size) :
    (values.push value).extract start (values.push value).size =
      (values.extract start values.size).push value := by
  apply Array.ext
  · simp; omega
  · intro index bound bound'
    simp only [Array.getElem_extract, Array.getElem_push, Array.size_extract, Array.size_push]
      at bound bound' ⊢
    split <;> split <;> first | rfl | omega

section States

variable {offset frontier inert inert' : Nat} {inside : Nat → Prop} {state state₂ : RuntimeState}

theorem StateRenamed.withGlobals (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    (globals : GlobalMap) (above : ∀ slot ∈ globals.entries, slot.value.Within inside) :
    StateRenamed ρ offset frontier inside inert inert' { state with globals }
      { state₂ with globals := globals.rename ρ } :=
  { shifted with globals := rfl, globalsWithin := above }

theorem StateRenamed.withRegistry (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    {registry registry' : List (Nat × LoanTarget)}
    (related : RegistryRenamed ρ inside registry registry') :
    StateRenamed ρ offset frontier inside inert inert' { state with storageLoans := registry }
      { state₂ with storageLoans := registry' } :=
  { shifted with registry := related }

theorem StateRenamed.push (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    {loan : Nat} (within : inside loan) {value : RuntimeValue}
    (value_within : value.Within inside) :
    StateRenamed ρ offset frontier inside inert inert'
      { state with pending := state.pending.push (loan, value) }
      { state₂ with pending := state₂.pending.push (ρ loan, value.rename ρ) } where
  renames := shifted.renames
  globals := shifted.globals
  tables := shifted.tables
  nextLoan := shifted.nextLoan
  frontier_le := shifted.frontier_le
  inert_le := by simp only [Array.size_push]; exact Nat.le_succ_of_le shifted.inert_le
  inert_le' := by simp only [Array.size_push]; exact Nat.le_succ_of_le shifted.inert_le'
  pending := by
    simp only
    rw [Array.extract_push_size _ _ shifted.inert_le', Array.extract_push_size _ _ shifted.inert_le,
      shifted.pending, Array.map_push]
  registry := shifted.registry
  globalsWithin := shifted.globalsWithin
  tablesWithin := shifted.tablesWithin
  pendingWithin := by
    simp only
    rw [Array.extract_push_size _ _ shifted.inert_le]
    intro entry member
    rcases Array.mem_push.mp member with member | rfl
    · exact shifted.pendingWithin entry member
    · exact ⟨within, value_within⟩

theorem StateRenamed.lookup (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    {loan : Nat} (within : inside loan) :
    storageLoanTarget? state₂ (ρ loan) = storageLoanTarget? state loan :=
  shifted.registry.lookup shifted.renames.injective within

theorem StateRenamed.slot_within (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    {key : GlobalKey} {value : RuntimeValue} (lookup_eq : state.globals.lookup key = some value) :
    value.Within inside := by
  unfold GlobalMap.lookup at lookup_eq
  obtain ⟨slot, found, rfl⟩ := Option.map_eq_some_iff.mp lookup_eq
  exact shifted.globalsWithin slot (Array.mem_of_find?_eq_some found)

theorem GlobalMap.insert_within {globals : GlobalMap} {key : GlobalKey} {value : RuntimeValue}
    (above : ∀ slot ∈ globals.entries, slot.value.Within inside)
    (value_within : value.Within inside) :
    ∀ slot ∈ (globals.insert key value).entries, slot.value.Within inside := by
  intro slot member
  rcases GlobalMap.mem_insert.mp member with ⟨member, -⟩ | rfl
  · exact above slot member
  · exact value_within

theorem StateRenamed.loanValue (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    (target : LoanTarget) :
    state₂.loanValue? target = (state.loanValue? target).map (·.rename ρ) := by
  cases target with
  | global key => simp [shifted.globals, GlobalMap.lookup_rename]
  | table key => simp [shifted.tables, NativeTableStorage.rename, GlobalMap.lookup_rename]

theorem StateRenamed.loanValue_within
    (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    {target : LoanTarget} {value : RuntimeValue}
    (found : state.loanValue? target = some value) : value.Within inside := by
  cases target with
  | global key => exact shifted.slot_within found
  | table key =>
      change state.tables.contents.lookup key = some value at found
      unfold GlobalMap.lookup at found
      obtain ⟨slot, located, rfl⟩ := Option.map_eq_some_iff.mp found
      exact shifted.tablesWithin slot (Array.mem_of_find?_eq_some located)

theorem StateRenamed.writeLoanValue
    (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    (target : LoanTarget) {value : RuntimeValue} (value_within : value.Within inside) :
    StateRenamed ρ offset frontier inside inert inert' (state.writeLoanValue target value)
      (state₂.writeLoanValue target (value.rename ρ)) := by
  cases target with
  | global key =>
      simp only [RuntimeState.writeLoanValue_global]
      exact { shifted with
        globals := by simp [shifted.globals, GlobalMap.insert_rename]
        globalsWithin := GlobalMap.insert_within shifted.globalsWithin value_within }
  | table key =>
      simp only [RuntimeState.writeLoanValue_table]
      exact { shifted with
        tables := by simp [shifted.tables, NativeTableStorage.rename, GlobalMap.insert_rename]
        tablesWithin := GlobalMap.insert_within shifted.tablesWithin value_within }

theorem RuntimeFrame.Within.setLocal {frame : RuntimeFrame} (above : frame.Within inside)
    (index : Nat) {value : RuntimeValue} (value_within : value.Within inside) :
    ({ frame with locals := frame.locals.set! index (some value) } : RuntimeFrame).Within inside where
  locals slot member stored stored_eq := by
    simp only [Array.set!_eq_setIfInBounds] at member
    rcases Array.mem_or_eq_of_mem_setIfInBounds member with member | rfl
    · exact above.locals slot member stored stored_eq
    · cases stored_eq; exact value_within
  activeLoans := above.activeLoans
  loanLocations := above.loanLocations

theorem RuntimeFrame.Within.slot {frame : RuntimeFrame} (above : frame.Within inside)
    {index : Nat} {value : RuntimeValue} (slot_eq : frame.locals[index]? = some (some value)) :
    value.Within inside :=
  above.locals _ (Array.mem_of_getElem? slot_eq) value rfl

theorem holeSlot_rename (injective : Function.Injective ρ) (loan : Nat) :
    ((fun slot : Option RuntimeValue => (slot.map (holeWithin (ρ loan))).getD false) ∘
        Option.map (·.rename ρ)) =
      fun slot => (slot.map (holeWithin loan)).getD false := by
  funext slot; cases slot <;> simp [holeWithin_rename injective]

theorem fillVisibleHole_rename {frame : RuntimeFrame}
    (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    (frame_within : frame.Within inside) {loan : Nat} (within : inside loan)
    {replacement : RuntimeValue} (replacement_within : replacement.Within inside) :
    ∃ state₂', fillVisibleHole (frame.rename ρ) state₂ (ρ loan)
        (replacement.rename ρ) =
      ((fillVisibleHole frame state loan replacement).1.rename ρ, state₂',
        (fillVisibleHole frame state loan replacement).2.2) ∧
      StateRenamed ρ offset frontier inside inert inert' (fillVisibleHole frame state loan replacement).2.1
        state₂' ∧
      (fillVisibleHole frame state loan replacement).1.Within inside := by
  have injective := shifted.renames.injective
  obtain ⟨β, γ, base⟩ := shifted.renames.base
  unfold fillVisibleHole
  rw [holeInFrame_rename injective]
  split
  · rw [RuntimeFrame.rename_locals, Array.toList_map, indexOfFrom_map, holeSlot_rename injective]
    cases indexOfFrom (fun slot => (slot.map (holeWithin loan)).getD false)
        frame.locals.toList 0 with
    | none => exact ⟨state₂, rfl, shifted, frame_within⟩
    | some index =>
        simp only [Array.getElem?_map]
        cases slot_eq : frame.locals[index]? with
        | none => exact ⟨state₂, rfl, shifted, frame_within⟩
        | some slot =>
            cases slot with
            | none => exact ⟨state₂, rfl, shifted, frame_within⟩
            | some value =>
                simp only [Option.map_some, Option.bind_some, fillHole?_rename injective]
                cases fill_eq : fillHole? loan replacement value with
                | none => exact ⟨state₂, rfl, shifted, frame_within⟩
                | some filled =>
                    refine ⟨state₂, ?_, shifted, frame_within.setLocal index
                      (fillHole?_within base within replacement_within (frame_within.slot slot_eq)
                        fill_eq)⟩
                    simp [RuntimeFrame.rename, Array.map_setIfInBounds]
  · rw [shifted.lookup within]
    cases storageLoanTarget? state loan with
    | none => exact ⟨state₂, rfl, shifted, frame_within⟩
    | some key =>
        simp only [shifted.loanValue key]
        cases lookup_eq : state.loanValue? key with
        | none => exact ⟨state₂, rfl, shifted, frame_within⟩
        | some value =>
            simp only [Option.map_some, Option.bind_some, fillHole?_rename injective]
            cases fill_eq : fillHole? loan replacement value with
            | none => exact ⟨state₂, rfl, shifted, frame_within⟩
            | some filled =>
                refine ⟨{ state₂.writeLoanValue key (filled.rename ρ) with
                    storageLoans := transferStorageLoan state₂.storageLoans (ρ loan) key
                      (replacement.rename ρ) }, rfl, ?_, frame_within⟩
                exact (shifted.writeLoanValue key
                  (fillHole?_within base within replacement_within
                    (shifted.loanValue_within lookup_eq) fill_eq)).withRegistry
                  (shifted.registry.transfer injective base within key replacement_within)

theorem applyWriteBack_rename {frame : RuntimeFrame}
    (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    (frame_within : frame.Within inside) {loan : Nat} (within : inside loan)
    {current : RuntimeValue} (current_within : current.Within inside) :
    ∃ state₂', applyWriteBack (frame.rename ρ) state₂ (ρ loan) (current.rename ρ) =
      ((applyWriteBack frame state loan current).1.rename ρ, state₂') ∧
      StateRenamed ρ offset frontier inside inert inert' (applyWriteBack frame state loan current).2 state₂' ∧
      (applyWriteBack frame state loan current).1.Within inside := by
  obtain ⟨state₂', fill_eq, filled, filled_within⟩ :=
    fillVisibleHole_rename shifted frame_within within current_within
  unfold applyWriteBack
  rw [fill_eq]
  simp only
  split
  · exact ⟨state₂', rfl, filled, filled_within⟩
  · exact ⟨_, rfl, filled.push within current_within, filled_within⟩

/-- A rewrite by a matcher that commutes with injective renamings keeps
values among the loans an enumeration admits, its loan among them. -/
theorem rewriteFirst_within_of_rename {matcher : Nat → RuntimeValue → Option RuntimeValue}
    (commutes : ∀ σ : Nat → Nat, Function.Injective σ → ∀ loan value,
      matcher (σ loan) (value.rename σ) = (matcher loan value).map (·.rename σ))
    {β γ : Nat → Nat} (base : LoanBase β γ inside)
    {loan : Nat} (within : inside loan) {value result : RuntimeValue}
    (value_within : value.Within inside) (rewrite : rewriteFirst (matcher loan) value = some result) :
    result.Within inside := by
  rw [← base.rename_inverse value value_within, ← base.inverse loan within,
    rewriteFirst_rename (commutes β base.injective (γ loan))] at rewrite
  obtain ⟨original, -, rfl⟩ := Option.map_eq_some_iff.mp rewrite
  exact base.within original

theorem findFirst_within_of_rename {matcher : Nat → RuntimeValue → Option RuntimeValue}
    (commutes : ∀ σ : Nat → Nat, Function.Injective σ → ∀ loan value,
      matcher (σ loan) (value.rename σ) = (matcher loan value).map (·.rename σ))
    {β γ : Nat → Nat} (base : LoanBase β γ inside)
    {loan : Nat} (within : inside loan) {value result : RuntimeValue}
    (value_within : value.Within inside) (query : findFirst (matcher loan) value = some result) :
    result.Within inside := by
  rw [← base.rename_inverse value value_within, ← base.inverse loan within,
    findFirst_rename (commutes β base.injective (γ loan))] at query
  obtain ⟨original, -, rfl⟩ := Option.map_eq_some_iff.mp query
  exact base.within original

theorem Array.findSome?_map_result {α β γ : Type} (values : Array α) (find : α → Option β)
    (f : β → γ) :
    values.findSome? (fun value => (find value).map f) = (values.findSome? find).map f := by
  rw [← Array.findSome?_toList, ← Array.findSome?_toList]
  induction values.toList with
  | nil => rfl
  | cons head tail ih =>
      simp only [List.findSome?_cons]
      cases find head <;> simp [ih]

theorem findBorrowValue?_rename {frame : RuntimeFrame}
    (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂) (loan : Nat) :
    findBorrowValue? (frame.rename ρ) state₂ (ρ loan) =
      (findBorrowValue? frame state loan).map (·.rename ρ) := by
  unfold findBorrowValue?
  have node := fun value => borrowCurrent?_rename shifted.renames.injective loan value
  simp only [RuntimeFrame.rename_locals, Array.findSome?_map, shifted.globals,
    GlobalMap.rename_entries]
  have locals : ((fun slot : Option RuntimeValue =>
        slot.bind fun value => findFirst (borrowCurrent? (ρ loan)) value) ∘
          Option.map fun value => value.rename ρ) =
      fun slot => (slot.bind fun value => findFirst (borrowCurrent? loan) value).map
        (·.rename ρ) := by
    funext slot; cases slot <;> simp [findFirst_rename node]
  have globals : ((fun slot : GlobalSlot => findFirst (borrowCurrent? (ρ loan)) slot.value) ∘
        fun slot : GlobalSlot => { slot with value := slot.value.rename ρ }) =
      fun slot => (findFirst (borrowCurrent? loan) slot.value).map (·.rename ρ) := by
    funext slot; simp [findFirst_rename node]
  rw [locals, globals, Array.findSome?_map_result, Array.findSome?_map_result]
  cases frame.locals.findSome? (fun slot => slot.bind fun value =>
    findFirst (borrowCurrent? loan) value) <;> rfl

theorem findBorrowValue?_within {frame : RuntimeFrame}
    (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    (frame_within : frame.Within inside) {loan : Nat} (within : inside loan)
    {current : RuntimeValue} (found : findBorrowValue? frame state loan = some current) :
    current.Within inside := by
  obtain ⟨β, γ, base⟩ := shifted.renames.base
  unfold findBorrowValue? at found
  have commutes := fun (σ : Nat → Nat) (injective : Function.Injective σ) loan value =>
    borrowCurrent?_rename injective loan value
  simp only at found
  split at found
  · rename_i result result_eq
    simp only [Option.some.injEq] at found
    subst found
    obtain ⟨slot, member, found⟩ := Array.exists_of_findSome?_eq_some result_eq
    cases slot with
    | none => cases found
    | some value =>
        exact findFirst_within_of_rename commutes base within
          (frame_within.locals _ member value rfl) found
  · obtain ⟨slot, member, found⟩ := Array.exists_of_findSome?_eq_some found
    exact findFirst_within_of_rename commutes base within (shifted.globalsWithin slot member) found

theorem clearBorrowValue_rename {frame : RuntimeFrame}
    (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    (frame_within : frame.Within inside) {loan : Nat} (within : inside loan) :
    ∃ state₂', clearBorrowValue (frame.rename ρ) state₂ (ρ loan) =
      ((clearBorrowValue frame state loan).1.rename ρ, state₂') ∧
      StateRenamed ρ offset frontier inside inert inert' (clearBorrowValue frame state loan).2 state₂' ∧
      (clearBorrowValue frame state loan).1.Within inside := by
  obtain ⟨β, γ, base⟩ := shifted.renames.base
  have commutes := fun (σ : Nat → Nat) (injective : Function.Injective σ) loan value =>
    borrowClear?_rename injective loan value
  have node := fun value => borrowClear?_rename shifted.renames.injective loan value
  unfold clearBorrowValue
  simp only
  have locals : ((fun slot : Option RuntimeValue =>
        (slot.bind fun value => rewriteFirst (borrowClear? (ρ loan)) value).isSome) ∘
          Option.map fun value => value.rename ρ) =
      fun slot => (slot.bind fun value => rewriteFirst (borrowClear? loan) value).isSome := by
    funext slot; cases slot <;> simp [rewriteFirst_rename node]
  rw [RuntimeFrame.rename_locals, Array.toList_map, indexOfFrom_map, locals]
  cases indexOfFrom (fun slot => (slot.bind fun value =>
      rewriteFirst (borrowClear? loan) value).isSome) frame.locals.toList 0 with
  | some index =>
      simp only [Array.getElem?_map]
      cases slot_eq : frame.locals[index]? with
      | none => exact ⟨state₂, rfl, shifted, frame_within⟩
      | some slot =>
          cases slot with
          | none => exact ⟨state₂, rfl, shifted, frame_within⟩
          | some value =>
              simp only [Option.map_some, Option.bind_some, rewriteFirst_rename node]
              cases clear_eq : rewriteFirst (borrowClear? loan) value with
              | none => exact ⟨state₂, rfl, shifted, frame_within⟩
              | some updated =>
                  refine ⟨state₂, by simp [RuntimeFrame.rename, Array.map_setIfInBounds], shifted,
                    frame_within.setLocal index (rewriteFirst_within_of_rename commutes base within
                      (frame_within.slot slot_eq) clear_eq)⟩
  | none =>
      have globals : ((fun slot : GlobalSlot =>
            (rewriteFirst (borrowClear? (ρ loan)) slot.value).isSome) ∘
              fun slot : GlobalSlot => { slot with value := slot.value.rename ρ }) =
          fun slot => (rewriteFirst (borrowClear? loan) slot.value).isSome := by
        funext slot; simp [rewriteFirst_rename node]
      rw [shifted.globals, GlobalMap.rename_entries, Array.toList_map, indexOfFrom_map, globals]
      cases indexOfFrom (fun slot : GlobalSlot =>
          (rewriteFirst (borrowClear? loan) slot.value).isSome) state.globals.entries.toList 0 with
      | none => exact ⟨state₂, rfl, shifted, frame_within⟩
      | some index =>
          simp only [Array.getElem?_map]
          cases slot_eq : state.globals.entries[index]? with
          | none => exact ⟨state₂, rfl, shifted, frame_within⟩
          | some slot =>
              simp only [Option.map_some, Option.bind_some, rewriteFirst_rename node]
              cases clear_eq : rewriteFirst (borrowClear? loan) slot.value with
              | none => exact ⟨state₂, rfl, shifted, frame_within⟩
              | some updated =>
                  refine ⟨{ state₂ with
                      globals := (⟨state.globals.entries.set! index { slot with value := updated }⟩ :
                        GlobalMap).rename ρ },
                    by simp [GlobalMap.rename, Array.map_setIfInBounds], ?_, frame_within⟩
                  refine shifted.withGlobals _ fun slot' member => ?_
                  simp only [Array.set!_eq_setIfInBounds] at member
                  rcases Array.mem_or_eq_of_mem_setIfInBounds member with member | rfl
                  · exact shifted.globalsWithin slot' member
                  · exact rewriteFirst_within_of_rename commutes base within
                      (shifted.globalsWithin slot (Array.mem_of_getElem? slot_eq)) clear_eq

/-- A frame and state of a second run mirroring those of a first. -/
def Mirrors (ρ : Nat → Nat) (offset frontier : Nat) (inside : Nat → Prop) (inert inert' : Nat)
    (first second : RuntimeFrame × RuntimeState) : Prop :=
  second.1 = first.1.rename ρ ∧
    StateRenamed ρ offset frontier inside inert inert' first.2 second.2 ∧ first.1.Within inside

theorem settleLoans_rename {loans : Array LoanId} {frame : RuntimeFrame}
    (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    (frame_within : frame.Within inside) :
    Mirrors ρ offset frontier inside inert inert' (settleLoans loans frame state)
      (settleLoans loans (frame.rename ρ) state₂) := by
  unfold settleLoans
  rw [← Array.foldr_toList, ← Array.foldr_toList]
  induction loans.toList with
  | nil => exact ⟨rfl, shifted, frame_within⟩
  | cons lexical rest ih =>
      simp only [List.foldr_cons]
      obtain ⟨same, related, above⟩ := ih
      generalize List.foldr _ (frame, state) rest = first at same related above
      generalize List.foldr _ (frame.rename ρ, state₂) rest = second at same related above
      obtain ⟨current₁, state₁⟩ := first
      obtain ⟨current₂, state₂'⟩ := second
      simp only at same related above
      subst same
      simp only [RuntimeFrame.rename_activeLoans, Array.find?_map]
      have site : ((fun entry : ExprId × Nat => entry.1 == ⟨lexical.index⟩) ∘
          fun entry : ExprId × Nat => (entry.1, ρ entry.2)) =
          fun entry => entry.1 == ⟨lexical.index⟩ := by
        funext entry; rfl
      rw [site]
      cases found : current₁.activeLoans.find? (·.1 == ⟨lexical.index⟩) with
      | none => exact ⟨rfl, related, above⟩
      | some entry =>
          obtain ⟨lexical', loanInstance⟩ := entry
          have instance_within : inside loanInstance :=
            above.activeLoans _ (Array.mem_of_find?_eq_some found)
          simp only [Option.map_some]
          have filtered : RuntimeFrame.mk (current₁.rename ρ).locals
              (Array.filter (fun entry => entry.fst != { index := lexical.index })
                (current₁.rename ρ).activeLoans)
              (current₁.rename ρ).loanLocations (current₁.rename ρ).typeInstantiation =
              (RuntimeFrame.mk current₁.locals
                (current₁.activeLoans.filter (fun entry => entry.fst != { index := lexical.index }))
                current₁.loanLocations current₁.typeInstantiation).rename ρ := by
            simp only [RuntimeFrame.rename, Array.filter_map]
            rfl
          have filtered_within : ({ current₁ with
              activeLoans := (current₁.activeLoans.filter (·.1 != ⟨lexical.index⟩)) } :
              RuntimeFrame).Within inside :=
            { locals := above.locals
              activeLoans := fun entry member => above.activeLoans entry (Array.mem_filter.mp member).1
              loanLocations := above.loanLocations }
          rw [← RuntimeFrame.rename_activeLoans, filtered, findBorrowValue?_rename related]
          cases borrow_eq : findBorrowValue? { current₁ with
              activeLoans := (current₁.activeLoans.filter (·.1 != ⟨lexical.index⟩)) }
              state₁ loanInstance with
          | none => exact ⟨rfl, related, filtered_within⟩
          | some currentValue =>
              have current_within := findBorrowValue?_within related filtered_within instance_within
                borrow_eq
              obtain ⟨cleared₂, clear_eq, cleared, cleared_within⟩ :=
                clearBorrowValue_rename related filtered_within instance_within
              simp only [Option.map_some, clear_eq]
              obtain ⟨written₂, write_eq, written, written_within⟩ :=
                applyWriteBack_rename cleared cleared_within instance_within current_within
              rw [write_eq]
              exact ⟨rfl, written, written_within⟩

theorem settleAfter_rename {loans : Array LoanId} {control : Control} {frame : RuntimeFrame}
    (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    (frame_within : frame.Within inside) :
    Mirrors ρ offset frontier inside inert inert' (settleAfter loans control frame state)
      (settleAfter loans (control.rename ρ) (frame.rename ρ) state₂) := by
  cases control <;> simp only [settleAfter, Control.rename]
  · exact settleLoans_rename shifted frame_within
  all_goals exact ⟨rfl, shifted, frame_within⟩

end States

/-- A write from a frame and state above a frontier, of a value above it,
leaves them above it, and a second run's write mirrors it. -/
theorem writeRuntimePlace?_mirror {offset frontier inert inert' : Nat} {inside : Nat → Prop}
    {frame frame' : RuntimeFrame}
    {state state' state₂ : RuntimeState}
    (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    (frame_within : frame.Within inside) {place : RuntimePlace} {value : RuntimeValue}
    (value_within : value.Within inside)
    (write : writeRuntimePlace? frame state place value = some (frame', state')) :
    ∃ state₂', writeRuntimePlace? (frame.rename ρ) state₂ place (value.rename ρ) =
        some (frame'.rename ρ, state₂') ∧
      StateRenamed ρ offset frontier inside inert inert' state' state₂' ∧ frame'.Within inside := by
  obtain ⟨β, γ, base⟩ := shifted.renames.base
  have base_eq : state.globals =
      ({ state with globals := state.globals.rename γ } : RuntimeState).globals.rename β :=
    (base.rename_inverse_globals shifted.globalsWithin).symm
  have based := writeRuntimePlace?_rename (ρ := β) (frame := frame.rename γ)
    base_eq place (value.rename γ)
  rw [base.rename_inverse_frame frame_within, base.rename_inverse value value_within, write] at based
  obtain ⟨written, -, written_eq⟩ := Option.map_eq_some_iff.mp based.symm
  simp only [Prod.mk.injEq] at written_eq
  obtain ⟨frame_eq, state_eq⟩ := written_eq
  have globals_within : ∀ slot ∈ state'.globals.entries, slot.value.Within inside := by
    rw [← state_eq]
    exact base.within_globals _
  refine ⟨{ state₂ with globals := state'.globals.rename ρ }, ?_, ?_, ?_⟩
  · rw [writeRuntimePlace?_rename shifted.globals, write]
    rfl
  · rw [writeRuntimePlace?_globals write]
    exact shifted.withGlobals _ globals_within
  · rw [← frame_eq]
    exact base.within_frame _

/-! ## Write-backs across calls -/

theorem applyPendingWriteBack_rename (injective : Function.Injective ρ) {frame : RuntimeFrame}
    {state state₂ : RuntimeState} (globals_eq : state₂.globals = state.globals.rename ρ) (loan : Nat)
    (current : RuntimeValue) :
    (applyPendingWriteBack (frame.rename ρ) state₂ (ρ loan) (current.rename ρ)).1 =
        (applyPendingWriteBack frame state loan current).1.rename ρ ∧
      (((applyPendingWriteBack frame state loan current).2 = state ∧
          (applyPendingWriteBack (frame.rename ρ) state₂ (ρ loan)
            (current.rename ρ)).2 = state₂) ∨
        ((applyPendingWriteBack frame state loan current).2 =
            { state with pending := state.pending.push (loan, current) } ∧
          (applyPendingWriteBack (frame.rename ρ) state₂ (ρ loan)
            (current.rename ρ)).2 =
            { state₂ with pending := state₂.pending.push (ρ loan, current.rename ρ) })) := by
  unfold applyPendingWriteBack
  rw [fillLocalLoanHole?_rename injective globals_eq]
  cases local_eq : fillLocalLoanHole? frame state loan current with
  | some resolved =>
      obtain ⟨resolvedFrame, resolvedState⟩ := resolved
      have same : resolvedState = state := by
        unfold fillLocalLoanHole? at local_eq
        simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at local_eq
        obtain ⟨_, -, _, -, _, -, ⟨_, _⟩, -, local_eq⟩ := local_eq
        simp only [Option.some.injEq, Prod.mk.injEq] at local_eq
        exact local_eq.2.symm
      subst same
      exact ⟨rfl, .inl ⟨rfl, rfl⟩⟩
  | none =>
      simp only [Option.map_none]
      rw [holeInFrame_rename injective]
      split
      · rw [RuntimeFrame.rename_locals, Array.toList_map, indexOfFrom_map, holeSlot_rename injective]
        cases indexOfFrom (fun slot => (slot.map (holeWithin loan)).getD false)
            frame.locals.toList 0 with
        | none => exact ⟨rfl, .inr ⟨rfl, rfl⟩⟩
        | some index =>
            simp only [Array.getElem?_map]
            cases slot_eq : frame.locals[index]? with
            | none => exact ⟨rfl, .inr ⟨rfl, rfl⟩⟩
            | some slot =>
                cases slot with
                | none => exact ⟨rfl, .inr ⟨rfl, rfl⟩⟩
                | some value =>
                    simp only [Option.map_some, Option.bind_some, fillHole?_rename injective]
                    cases fillHole? loan current value with
                    | none => exact ⟨rfl, .inr ⟨rfl, rfl⟩⟩
                    | some filled =>
                        refine ⟨?_, .inl ⟨rfl, rfl⟩⟩
                        simp only [Option.map_some, RuntimeFrame.rename_activeLoans,
                          RuntimeFrame.rename_loanLocations, transferActiveLoan_rename injective,
                          transferLoanLocation_rename injective]
                        simp [RuntimeFrame.rename, Array.map_setIfInBounds]
      · exact ⟨rfl, .inr ⟨rfl, rfl⟩⟩

theorem Array.extract_from {α : Type} (values : Array α) {start start' : Nat}
    (le : start ≤ start') :
    values.extract start' values.size =
      (values.extract start values.size).extract (start' - start) (values.size - start) := by
  rw [Array.extract_extract]
  congr 1 <;> omega

theorem Array.mem_extract_of_start_le {α : Type} {values : Array α} {start start' stop : Nat}
    {value : α} (member : value ∈ values.extract start' stop) (le : start ≤ start') :
    value ∈ values.extract start stop := by
  rw [Array.mem_iff_getElem] at member ⊢
  obtain ⟨k, bound, rfl⟩ := member
  simp only [Array.size_extract] at bound
  refine ⟨k + (start' - start), by simp only [Array.size_extract]; omega, ?_⟩
  simp only [Array.getElem_extract]
  congr 1
  omega

section Calls

variable {frontier inert inert' : Nat}

theorem StateRenamed.pending_from {state state₂ : RuntimeState}
    (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂) {start start' : Nat}
    (le : inert ≤ start) (le' : inert' ≤ start') (same : start - inert = start' - inert') :
    state₂.pending.extract start' state₂.pending.size =
      (state.pending.extract start state.pending.size).map fun entry =>
        (ρ entry.1, entry.2.rename ρ) := by
  have sizes := congrArg Array.size shifted.pending
  simp only [Array.size_extract, Array.size_map, Nat.min_self] at sizes
  rw [Array.extract_from _ le', shifted.pending, Array.extract_from _ le, ← Array.map_extract]
  congr 1
  rw [same, sizes]

theorem StateRenamed.restart {state state₂ arguments arguments₂ : RuntimeState}
    (final : StateRenamed ρ offset frontier inside inert inert' state state₂)
    (calling : StateRenamed ρ offset frontier inside inert inert' arguments arguments₂) :
    StateRenamed ρ offset frontier inside inert inert' { state with pending := arguments.pending }
      { state₂ with pending := arguments₂.pending } where
  renames := final.renames
  globals := final.globals
  tables := final.tables
  nextLoan := final.nextLoan
  frontier_le := final.frontier_le
  inert_le := calling.inert_le
  inert_le' := calling.inert_le'
  pending := calling.pending
  registry := final.registry
  globalsWithin := final.globalsWithin
  tablesWithin := final.tablesWithin
  pendingWithin := calling.pendingWithin

theorem applyPendingWriteBack_mirror {first second : RuntimeFrame × RuntimeState}
    (mirrors : Mirrors ρ offset frontier inside inert inert' first second) {loan : Nat}
    (within : inside loan) {current : RuntimeValue} (current_within : current.Within inside) :
    Mirrors ρ offset frontier inside inert inert'
      (applyPendingWriteBack first.1 first.2 loan current)
      (applyPendingWriteBack second.1 second.2 (ρ loan) (current.rename ρ)) := by
  obtain ⟨frame, state⟩ := first
  obtain ⟨frame₂, state₂⟩ := second
  obtain ⟨rfl, shifted, frame_within⟩ := mirrors
  obtain ⟨frame_eq, states⟩ :=
    applyPendingWriteBack_rename shifted.renames.injective (frame := frame) shifted.globals loan current
  obtain ⟨β, γ, base⟩ := shifted.renames.base
  have base_eq : state.globals =
      ({ state with globals := state.globals.rename γ } : RuntimeState).globals.rename β :=
    (base.rename_inverse_globals shifted.globalsWithin).symm
  have based := (applyPendingWriteBack_rename base.injective (frame := frame.rename γ)
    base_eq (γ loan) (current.rename γ)).1
  rw [base.rename_inverse_frame frame_within, base.inverse loan within,
    base.rename_inverse current current_within] at based
  refine ⟨frame_eq, ?_, based ▸ base.within_frame _⟩
  rcases states with ⟨first_eq, second_eq⟩ | ⟨first_eq, second_eq⟩
  · simp only at first_eq second_eq ⊢
    rw [first_eq, second_eq]
    exact shifted
  · simp only at first_eq second_eq ⊢
    rw [first_eq, second_eq]
    exact shifted.push within current_within

theorem applyPendingFrom_mirror {frame : RuntimeFrame}
    {arguments arguments₂ state state₂ : RuntimeState} (frame_within : frame.Within inside)
    (calling : StateRenamed ρ offset frontier inside inert inert' arguments arguments₂)
    (final : StateRenamed ρ offset frontier inside inert inert' state state₂) :
    Mirrors ρ offset frontier inside inert inert' (applyPendingFrom arguments.pending frame state)
      (applyPendingFrom arguments₂.pending (frame.rename ρ) state₂) := by
  have sizes := congrArg Array.size calling.pending
  simp only [Array.size_extract, Array.size_map, Nat.min_self] at sizes
  have calling_le := calling.inert_le
  have calling_le' := calling.inert_le'
  unfold applyPendingFrom
  rw [final.pending_from calling_le calling_le' (by omega), Array.foldl_map]
  have entries : ∀ entry ∈ state.pending.extract arguments.pending.size state.pending.size,
      inside entry.1 ∧ entry.2.Within inside := fun entry member =>
    final.pendingWithin entry (Array.mem_extract_of_start_le member calling_le)
  rw [← Array.foldl_toList, ← Array.foldl_toList]
  have start : Mirrors ρ offset frontier inside inert inert' (frame, { state with pending := arguments.pending })
      (frame.rename ρ, { state₂ with pending := arguments₂.pending }) :=
    ⟨rfl, final.restart calling, frame_within⟩
  have entries' : ∀ entry ∈ (state.pending.extract arguments.pending.size
      state.pending.size).toList, inside entry.1 ∧ entry.2.Within inside :=
    fun entry member => entries entry (Array.mem_def.mpr member)
  clear entries
  generalize (state.pending.extract arguments.pending.size state.pending.size).toList = list
    at entries'
  suffices fold : ∀ (list : List (Nat × RuntimeValue)) (first second : RuntimeFrame × RuntimeState),
      (∀ entry ∈ list, inside entry.1 ∧ entry.2.Within inside) →
      Mirrors ρ offset frontier inside inert inert' first second →
      Mirrors ρ offset frontier inside inert inert'
        (list.foldl (fun pair entry => applyPendingWriteBack pair.1 pair.2 entry.1 entry.2) first)
        (list.foldl (fun pair entry =>
          applyPendingWriteBack pair.1 pair.2 (ρ entry.1) (entry.2.rename ρ)) second) by
    exact fold list _ _ entries' start
  intro list
  induction list with
  | nil => intro first second _ mirrors; exact mirrors
  | cons head tail ih =>
      intro first second entries mirrors
      simp only [List.foldl_cons]
      exact ih _ _ (fun entry member => entries entry (by simp [member]))
        (applyPendingWriteBack_mirror mirrors (entries head (by simp)).1 (entries head (by simp)).2)

theorem borrowEntry?_rename (value : RuntimeValue) :
    borrowEntry? (value.rename ρ) =
      (borrowEntry? value).map (Option.map fun entry => (ρ entry.1, entry.2.rename ρ)) := by
  cases value <;> simp [borrowEntry?]

theorem outermostBorrows_rename (value : RuntimeValue) :
    outermostBorrows (value.rename ρ) =
      (outermostBorrows value).map fun entry => (ρ entry.1, entry.2.rename ρ) := by
  unfold outermostBorrows
  rw [collectPruned_rename (fun value => borrowEntry?_rename value)]
  apply Array.ext'
  simp [List.filterMap_map, List.map_filterMap]

theorem returnedBorrows_rename (results : Array RuntimeValue) :
    (results.map (·.rename ρ)).foldl (init := #[]) (fun found result =>
        found ++ outermostBorrows result) =
      (results.foldl (init := #[]) fun found result => found ++ outermostBorrows result).map
        fun entry => (ρ entry.1, entry.2.rename ρ) := by
  rw [Array.foldl_map, ← Array.foldl_toList, ← Array.foldl_toList]
  suffices general : ∀ (list : List RuntimeValue) (start : Array (Nat × RuntimeValue)),
      list.foldl (fun found result => found ++ outermostBorrows (result.rename ρ))
          (start.map fun entry => (ρ entry.1, entry.2.rename ρ)) =
        (list.foldl (fun found result => found ++ outermostBorrows result) start).map
          fun entry => (ρ entry.1, entry.2.rename ρ) by
    simpa using general results.toList #[]
  intro list
  induction list with
  | nil => intro start; rfl
  | cons head tail ih =>
      intro start
      simp only [List.foldl_cons]
      rw [outermostBorrows_rename head, ← Array.map_append, ih]

theorem registerReturnedLoan_rename (lexical : Option Nat) (results : Array RuntimeValue)
    (frame : RuntimeFrame) :
    registerReturnedLoan lexical (results.map (·.rename ρ)) (frame.rename ρ) =
      (registerReturnedLoan lexical results frame).rename ρ := by
  cases lexical with
  | none => rfl
  | some lexical =>
      unfold registerReturnedLoan
      simp only [returnedBorrows_rename, Array.getElem?_map]
      cases (results.foldl (init := #[]) fun found result =>
          found ++ outermostBorrows result)[0]? with
      | none => rfl
      | some entry =>
          simp only [Option.map_some, RuntimeFrame.rename]
          rw [Array.filter_map, Array.map_push]
          rfl

theorem packResults_nil : packResults #[] = .unit := rfl
theorem packResults_single (value : RuntimeValue) : packResults #[value] = value := rfl
theorem packResults_many (first second : RuntimeValue) (rest : List RuntimeValue) :
    packResults (first :: second :: rest).toArray = .tuple (first :: second :: rest).toArray := rfl

theorem packResults_rename (results : Array RuntimeValue) :
    packResults (results.map (·.rename ρ)) = (packResults results).rename ρ := by
  rcases results with ⟨_ | ⟨first, _ | ⟨second, rest⟩⟩⟩
  · rw [List.map_toArray, List.map_nil, packResults_nil, RuntimeValue.rename_unit]
  · rw [List.map_toArray, List.map_cons, List.map_nil, packResults_single, packResults_single]
  · rw [List.map_toArray, List.map_cons, List.map_cons, packResults_many, packResults_many,
      RuntimeValue.rename_tuple, List.map_toArray]
    rfl

theorem Array.rename_inverse_all {β γ : Nat → Nat} (base : LoanBase β γ inside)
    {values : Array RuntimeValue} (within : ∀ value ∈ values, value.Within inside) :
    (values.map (·.rename γ)).map (·.rename β) = values := by
  rw [Array.map_map]
  conv => rhs; rw [← Array.map_id values]
  exact Array.map_congr_left fun value member => base.rename_inverse value (within value member)

theorem packResults_within {β γ : Nat → Nat} (base : LoanBase β γ inside)
    {results : Array RuntimeValue} (within : ∀ value ∈ results, value.Within inside) :
    (packResults results).Within inside := by
  rw [← Array.rename_inverse_all base within, packResults_rename]
  exact base.within _

theorem registerReturnedLoan_within {β γ : Nat → Nat} (base : LoanBase β γ inside)
    {lexical : Option Nat} {results : Array RuntimeValue}
    {frame : RuntimeFrame} (frame_within : frame.Within inside)
    (within : ∀ value ∈ results, value.Within inside) :
    (registerReturnedLoan lexical results frame).Within inside := by
  rw [← Array.rename_inverse_all base within, ← base.rename_inverse_frame frame_within,
    registerReturnedLoan_rename]
  exact base.within_frame _

end Calls

/-! ## Data and closures -/

theorem constructNominal?_rename (unit : Validation.ValidatedUnit) (sourceNamespace : NamespaceId)
    (reference : QualifiedRef) (variant : Option String) (fields : Array RuntimeValue) :
    constructNominal? unit sourceNamespace reference variant (fields.map (·.rename ρ)) =
      (constructNominal? unit sourceNamespace reference variant fields).map (·.rename ρ) := by
  unfold constructNominal?
  simp only [Option.bind_eq_bind, Array.size_map]
  cases resolveStruct? unit sourceNamespace reference with
  | none => rfl
  | some handle =>
      simp only [Option.bind_some]
      cases constructorFields? unit handle variant with
      | none => rfl
      | some expected =>
          simp only [Option.bind_some]
          split <;> simp [pure]

theorem destructNominal?_rename (unit : Validation.ValidatedUnit) (sourceNamespace : NamespaceId)
    (reference : QualifiedRef) (variant : Option String) (value : RuntimeValue) :
    destructNominal? unit sourceNamespace reference variant (value.rename ρ) =
      (destructNominal? unit sourceNamespace reference variant value).map
        (·.map (·.rename ρ)) := by
  unfold destructNominal?
  cases value <;> simp only [RuntimeValue.rename_tuple, RuntimeValue.rename_unit,
    RuntimeValue.rename_bool, RuntimeValue.rename_character, RuntimeValue.rename_integer,
    RuntimeValue.rename_address, RuntimeValue.rename_signer, RuntimeValue.rename_string,
    RuntimeValue.rename_bytes, RuntimeValue.rename_vector, RuntimeValue.rename_nominal,
    RuntimeValue.rename_closure, RuntimeValue.rename_borrow, RuntimeValue.rename_loanHole] <;> try rfl
  case nominal source actualVariant fields =>
    simp only [Option.bind_eq_bind, Array.size_map]
    cases resolveStruct? unit sourceNamespace reference with
    | none => rfl
    | some handle =>
        simp only [Option.bind_some]
        cases constructorFields? unit handle variant with
        | none => rfl
        | some expected =>
            simp only [Option.bind_some]
            split <;> rfl

theorem ClosureMask.compose_go_map {α β : Type} (f : α → β) :
    ∀ (fuel mask : Nat) (captures supplied : List α),
      ClosureMask.compose.go fuel mask (captures.map f) (supplied.map f) =
        (ClosureMask.compose.go fuel mask captures supplied).map (·.map f)
  | _, 0, [], supplied => by simp [ClosureMask.compose.go]
  | _, 0, _ :: _, _ => by simp [ClosureMask.compose.go]
  | 0, _ + 1, _, _ => by simp [ClosureMask.compose.go]
  | fuel + 1, mask + 1, captures, supplied => by
      simp only [ClosureMask.compose.go]
      split
      · cases captures with
        | nil => rfl
        | cons capture captures =>
            simp only [List.map_cons, Functor.map, ClosureMask.compose_go_map f fuel _ captures supplied,
              Option.map_map]
            rfl
      · cases supplied with
        | nil => rfl
        | cons value supplied =>
            simp only [List.map_cons, Functor.map, ClosureMask.compose_go_map f fuel _ captures supplied,
              Option.map_map]
            rfl

theorem ClosureMask.compose_map {α β : Type} (f : α → β) (mask : Nat)
    (captures supplied : List α) :
    ClosureMask.compose mask (captures.map f) (supplied.map f) =
      (ClosureMask.compose mask captures supplied).map (·.map f) := by
  unfold ClosureMask.compose
  simp only [List.length_map]
  exact ClosureMask.compose_go_map f _ mask captures supplied

theorem List.mapM_congr_of_mem {α β : Type} {f g : α → Option β} :
    ∀ {xs : List α}, (∀ x ∈ xs, f x = g x) → xs.mapM f = xs.mapM g
  | [], _ => rfl
  | x :: xs, same => by
      simp only [List.mapM_cons, same x (by simp),
        List.mapM_congr_of_mem (xs := xs) (fun y member => same y (List.mem_cons_of_mem _ member))]

theorem toConst?_rename :
    ∀ (fuel : Nat) (value : RuntimeValue), sizeOf value ≤ fuel →
      RuntimeValue.toConst? (value.rename ρ) = RuntimeValue.toConst? value
  | 0, value, small => by cases value <;> simp at small
  | fuel + 1, value, small => by
      have elements : ∀ (elements : Array RuntimeValue), sizeOf elements ≤ fuel + 1 →
          (elements.map (·.rename ρ)).attach.mapM
              (fun ⟨element, _⟩ => RuntimeValue.toConst? element) =
            elements.attach.mapM (fun ⟨element, _⟩ => RuntimeValue.toConst? element) := by
        intro elements size
        rw [Array.mapM_subtype (g := RuntimeValue.toConst?) (fun _ _ => rfl),
          Array.mapM_subtype (g := RuntimeValue.toConst?) (fun _ _ => rfl), Array.unattach_attach,
          Array.unattach_attach, Array.mapM_eq_mapM_toList, Array.mapM_eq_mapM_toList,
          Array.toList_map, List.mapM_map]
        congr 1
        apply List.mapM_congr_of_mem
        intro element member
        have := List.sizeOf_lt_of_mem member
        cases elements
        exact toConst?_rename fuel element (by simp at size this; omega)
      cases value with
      | vector values =>
          rw [RuntimeValue.rename_vector]
          simp only [RuntimeValue.toConst?]
          rw [elements values (by simp at small; omega)]
      | tuple values =>
          rw [RuntimeValue.rename_tuple]
          simp only [RuntimeValue.toConst?]
          rw [elements values (by simp at small; omega)]
      | nominal | closure | borrow | loanHole => simp [RuntimeValue.toConst?]
      | unit | bool _ | character _ | integer _ | address _ | signer _ | string _ | bytes _ =>
          simp [RuntimeValue.toConst?]

theorem evaluateProfileOperation?_rename {unit : Validation.ValidatedUnit}
    (executable : Validation.ExecutableUnit unit)
    (ns : Validation.ValidatedNamespace) (resultType : TypeId) (operation : ProfileValue)
    (arguments : Array RuntimeValue) :
    evaluateProfileOperation? executable ns resultType operation (arguments.map (·.rename ρ)) =
      evaluateProfileOperation? executable ns resultType operation arguments := by
  unfold evaluateProfileOperation?
  have closed : (arguments.map (·.rename ρ)).mapM RuntimeValue.toConst? =
      arguments.mapM RuntimeValue.toConst? := by
    rw [Array.mapM_eq_mapM_toList, Array.mapM_eq_mapM_toList, Array.toList_map, List.mapM_map]
    congr 1
    exact List.mapM_congr_of_mem fun value _ =>
      toConst?_rename (ρ := ρ) _ value (Nat.le_refl _)
  simp only [closed]

theorem constructNominal?_within {β γ : Nat → Nat} (base : LoanBase β γ inside)
    {unit : Validation.ValidatedUnit}
    {sourceNamespace : NamespaceId} {reference : QualifiedRef} {variant : Option String}
    {fields : Array RuntimeValue} {value : RuntimeValue}
    (within : ∀ field ∈ fields, field.Within inside)
    (construct : constructNominal? unit sourceNamespace reference variant fields = some value) :
    value.Within inside := by
  rw [← Array.rename_inverse_all base within, constructNominal?_rename] at construct
  obtain ⟨original, -, rfl⟩ := Option.map_eq_some_iff.mp construct
  exact base.within original

theorem destructNominal?_within {β γ : Nat → Nat} (base : LoanBase β γ inside)
    {unit : Validation.ValidatedUnit}
    {sourceNamespace : NamespaceId} {reference : QualifiedRef} {variant : Option String}
    {value : RuntimeValue} {fields : Array RuntimeValue} (within : value.Within inside)
    (destruct : destructNominal? unit sourceNamespace reference variant value = some fields) :
    ∀ field ∈ fields, field.Within inside := by
  rw [← base.rename_inverse value within, destructNominal?_rename] at destruct
  obtain ⟨renamed, -, rfl⟩ := Option.map_eq_some_iff.mp destruct
  intro field member
  obtain ⟨original, -, rfl⟩ := Array.mem_map.mp member
  exact base.within original

theorem ClosureMask.compose_go_mem {α : Type} :
    ∀ (fuel mask : Nat) (captures supplied composed : List α),
      ClosureMask.compose.go fuel mask captures supplied = some composed →
      ∀ x ∈ composed, x ∈ captures ∨ x ∈ supplied
  | _, 0, [], supplied, composed, composed_eq => by
      simp only [ClosureMask.compose.go, Option.some.injEq] at composed_eq
      subst composed_eq; exact fun x member => .inr member
  | _, 0, _ :: _, _, _, composed_eq => by simp [ClosureMask.compose.go] at composed_eq
  | 0, _ + 1, _, _, _, composed_eq => by simp [ClosureMask.compose.go] at composed_eq
  | fuel + 1, mask + 1, captures, supplied, composed, composed_eq => by
      simp only [ClosureMask.compose.go] at composed_eq
      split at composed_eq
      · cases captures with
        | nil => cases composed_eq
        | cons capture captures =>
            simp only [Functor.map, Option.map_eq_some_iff] at composed_eq
            obtain ⟨rest, rest_eq, rfl⟩ := composed_eq
            intro x member
            rcases List.mem_cons.mp member with rfl | member
            · exact .inl List.mem_cons_self
            · rcases ClosureMask.compose_go_mem fuel _ captures supplied rest rest_eq x member with
                member | member
              · exact .inl (List.mem_cons_of_mem _ member)
              · exact .inr member
      · cases supplied with
        | nil => cases composed_eq
        | cons value supplied =>
            simp only [Functor.map, Option.map_eq_some_iff] at composed_eq
            obtain ⟨rest, rest_eq, rfl⟩ := composed_eq
            intro x member
            rcases List.mem_cons.mp member with rfl | member
            · exact .inr List.mem_cons_self
            · rcases ClosureMask.compose_go_mem fuel _ captures supplied rest rest_eq x member with
                member | member
              · exact .inl member
              · exact .inr (List.mem_cons_of_mem _ member)

theorem ClosureMask.compose_mem {α : Type} {mask : Nat} {captures supplied composed : List α}
    (composed_eq : ClosureMask.compose mask captures supplied = some composed) :
    ∀ x ∈ composed, x ∈ captures ∨ x ∈ supplied :=
  ClosureMask.compose_go_mem _ mask captures supplied composed composed_eq

theorem evaluateProfileOperation?_plain {unit : Validation.ValidatedUnit}
    {executable : Validation.ExecutableUnit unit}
    {ns : Validation.ValidatedNamespace} {resultType : TypeId} {operation : ProfileValue}
    {arguments : Array RuntimeValue} {result : Except (ThrowKind × Array RuntimeValue) RuntimeValue}
    (evaluate : evaluateProfileOperation? executable ns resultType operation arguments = some result) :
    (∀ value, result = .ok value → Plain value) ∧
      (∀ kind values, result = .error (kind, values) → ∀ value ∈ values, Plain value) := by
  unfold evaluateProfileOperation? at evaluate
  simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at evaluate
  obtain ⟨_, -, _, -, _, -, evaluate⟩ := evaluate
  split at evaluate
  · cases evaluate
  · obtain ⟨value, value_eq, evaluate⟩ := Option.bind_eq_some_iff.mp evaluate
    simp only [Option.some.injEq] at evaluate
    subst evaluate
    refine ⟨fun runtimeValue same => ?_, fun _ _ same => by cases same⟩
    cases same
    exact constValue?_plain _ _ (Nat.le_refl _) value_eq
  · obtain ⟨values, values_eq, evaluate⟩ := Option.bind_eq_some_iff.mp evaluate
    simp only [Option.some.injEq] at evaluate
    subst evaluate
    refine ⟨fun _ same => (by cases same), fun kind' runtimeValues same => ?_⟩
    simp only [Except.error.injEq, Prod.mk.injEq] at same
    obtain ⟨-, rfl⟩ := same
    intro value member
    rw [Array.mapM_eq_mapM_toList] at values_eq
    simp only [Functor.map, Option.map_eq_some_iff] at values_eq
    obtain ⟨list, list_eq, rfl⟩ := values_eq
    exact List.mapM_some_forall list_eq (fun literal _ value value_eq =>
      constValue?_plain _ literal (Nat.le_refl _) value_eq) value (by simpa using member)

end LeanerIR
