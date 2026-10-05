-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.LoanRenaming

/-!
# Operations commute with renaming loans

Each runtime operation applied to a frame and state with their loans raised
by an offset is the operation applied to the originals, raised
(`designs/static-typing.md`, "Loan independence").
-/

namespace LeanerIR

open SemanticOperations

variable {offset : Nat}

/-! ## Global memory -/

@[simp] theorem GlobalMap.shift_entries (offset : Nat) (globals : GlobalMap) :
    (globals.shift offset).entries =
      globals.entries.map fun slot => { slot with value := slot.value.shift offset } := rfl

theorem GlobalMap.lookup_shift (globals : GlobalMap) (key : GlobalKey) :
    (globals.shift offset).lookup key = (globals.lookup key).map (·.shift offset) := by
  unfold GlobalMap.lookup
  simp only [GlobalMap.shift_entries, Array.find?_map, Option.map_map]
  rfl

theorem GlobalMap.erase_shift (globals : GlobalMap) (key : GlobalKey) :
    (globals.erase key).shift offset = (globals.shift offset).erase key := by
  unfold GlobalMap.erase GlobalMap.shift
  simp only [GlobalMap.mk.injEq]
  rw [Array.filter_map]
  rfl

private theorem insertSlot_map (offset : Nat) (slot : GlobalSlot) :
    (entries : List GlobalSlot) →
    (GlobalMap.insertSlot slot entries).map
        (fun slot => ({ slot with value := slot.value.shift offset } : GlobalSlot)) =
      GlobalMap.insertSlot { slot with value := slot.value.shift offset }
        (entries.map fun slot => { slot with value := slot.value.shift offset })
  | [] => rfl
  | head :: rest => by
      simp only [List.map_cons, GlobalMap.insertSlot]
      by_cases less : slot.key.rank < head.key.rank
      · simp only [less, ↓reduceIte, List.map_cons]
      · simp only [less, ↓reduceIte, List.map_cons, insertSlot_map offset slot rest]

theorem GlobalMap.insert_shift (globals : GlobalMap) (key : GlobalKey) (value : RuntimeValue) :
    (globals.insert key value).shift offset = (globals.shift offset).insert key (value.shift offset) := by
  unfold GlobalMap.insert
  rw [← GlobalMap.erase_shift]
  simp only [GlobalMap.shift, List.map_toArray, GlobalMap.mk.injEq, Array.toList_map]
  rw [insertSlot_map]

/-! ## Reads and writes -/

theorem readLocal?_shift (frame : RuntimeFrame) (localId : LocalId) :
    readLocal? (frame.shift offset) localId = (readLocal? frame localId).map (·.shift offset) := by
  unfold readLocal?
  simp only [RuntimeFrame.shift, Array.getElem?_map]
  cases frame.locals[localId.index]? with
  | none => rfl
  | some slot => cases slot <;> rfl

theorem readProjections?_shift :
    ∀ (projections : List RuntimeProjection) (value : RuntimeValue),
      readProjections? (value.shift offset) projections =
        (readProjections? value projections).map (·.shift offset)
  | [], value => rfl
  | .field index :: rest, value => by
      cases value <;> simp [readProjections?, Array.getElem?_map]
      rename_i fields
      cases fields[index]? <;> simp [readProjections?_shift rest]
  | .index index :: rest, value => by
      cases value <;> simp [readProjections?, Array.getElem?_map]
      all_goals (rename_i elements; cases elements[index]? <;> simp [readProjections?_shift rest])
  | .subslice start stop fromEnd :: rest, value => by
      cases value <;> simp [readProjections?]
      rename_i elements
      cases subsliceBounds? elements.size start stop fromEnd with
      | none => rfl
      | some bounds =>
          simp only [Option.bind_some]
          rw [← Array.map_extract, ← RuntimeValue.shift_vector, readProjections?_shift rest]
          rfl
  | .downcast variant :: rest, value => by
      cases value <;> simp [readProjections?]
      rename_i source actual fields
      cases actual with
      | none => rfl
      | some actual =>
          simp only
          split
          · rw [← RuntimeValue.shift_nominal, readProjections?_shift rest]
          · rfl
  | .deref :: rest, value => by
      cases value <;> simp [readProjections?, readProjections?_shift rest]

theorem writeProjections?_shift :
    ∀ (projections : List RuntimeProjection) (value replacement : RuntimeValue),
      writeProjections? (value.shift offset) projections (replacement.shift offset) =
        (writeProjections? value projections replacement).map (·.shift offset)
  | [], value, replacement => rfl
  | .field index :: rest, value, replacement => by
      cases value <;> simp [writeProjections?, Array.getElem?_map]
      rename_i source variant fields
      cases fields[index]? with
      | none => rfl
      | some field =>
          simp only [Option.map_some, Option.bind_some, Function.comp_apply]
          rw [writeProjections?_shift rest]
          cases writeProjections? field rest replacement <;> simp [Array.map_setIfInBounds]
  | .index index :: rest, value, replacement => by
      cases value <;> simp [writeProjections?, Array.getElem?_map]
      all_goals
        rename_i elements
        cases elements[index]? with
        | none => rfl
        | some element =>
            simp only [Option.map_some, Option.bind_some, Function.comp_apply]
            rw [writeProjections?_shift rest]
            cases writeProjections? element rest replacement <;> simp [Array.map_setIfInBounds]
  | .subslice start stop fromEnd :: rest, value, replacement => by
      cases value <;> simp [writeProjections?]
      rename_i elements
      cases subsliceBounds? elements.size start stop fromEnd with
      | none => rfl
      | some bounds =>
          obtain ⟨first, last⟩ := bounds
          simp only [Option.bind_some]
          rw [← Array.map_extract, ← RuntimeValue.shift_vector, writeProjections?_shift rest]
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
          · rw [← RuntimeValue.shift_nominal, writeProjections?_shift rest]
          · rfl
  | .deref :: rest, value, replacement => by
      cases value <;> simp [writeProjections?]
      rename_i loan current
      rw [writeProjections?_shift rest]
      cases writeProjections? current rest replacement <;> simp

@[simp] theorem RuntimeFrame.shift_locals (frame : RuntimeFrame) :
    (frame.shift offset).locals = frame.locals.map (Option.map (·.shift offset)) := rfl

@[simp] theorem RuntimeFrame.shift_typeInstantiation (frame : RuntimeFrame) :
    (frame.shift offset).typeInstantiation = frame.typeInstantiation := rfl

theorem RuntimeFrame.shift_setLocal (frame : RuntimeFrame) (index : Nat)
    (value : Option RuntimeValue) :
    ({ frame with locals := frame.locals.set! index value } : RuntimeFrame).shift offset =
      { frame.shift offset with
        locals := (frame.shift offset).locals.set! index (value.map (·.shift offset)) } := by
  simp [RuntimeFrame.shift, Array.set!_eq_setIfInBounds, Array.map_setIfInBounds]

section Roots

variable {frame : RuntimeFrame} {state state₂ : RuntimeState}
  (globals_eq : state₂.globals = state.globals.shift offset)
include globals_eq

theorem readRoot?_shift (root : RuntimePlaceRoot) :
    readRoot? (frame.shift offset) state₂ root = (readRoot? frame state root).map (·.shift offset) := by
  cases root with
  | «local» localId => exact readLocal?_shift frame localId
  | global key => simp only [readRoot?, globals_eq, GlobalMap.lookup_shift]

theorem readRuntimePlace?_shift (place : RuntimePlace) :
    readRuntimePlace? (frame.shift offset) state₂ place =
      (readRuntimePlace? frame state place).map (·.shift offset) := by
  unfold readRuntimePlace?
  rw [readRoot?_shift globals_eq]
  cases readRoot? frame state place.root with
  | none => rfl
  | some root => simp [readProjections?_shift]

theorem writeRoot?_shift (root : RuntimePlaceRoot) (value : RuntimeValue) :
    writeRoot? (frame.shift offset) state₂ root (value.shift offset) =
      (writeRoot? frame state root value).map fun written =>
        (written.1.shift offset, { state₂ with globals := written.2.globals.shift offset }) := by
  cases root with
  | «local» localId =>
      simp only [writeRoot?, RuntimeFrame.shift_locals, Array.size_map]
      split
      · rfl
      · simp only [Option.map_some, RuntimeFrame.shift_setLocal, ← globals_eq]
        rfl
  | global key =>
      simp only [writeRoot?, globals_eq, GlobalMap.lookup_shift, Option.bind_eq_bind]
      cases state.globals.lookup key with
      | none => rfl
      | some _ => simp [GlobalMap.insert_shift]

theorem writeRuntimePlace?_shift (place : RuntimePlace) (value : RuntimeValue) :
    writeRuntimePlace? (frame.shift offset) state₂ place (value.shift offset) =
      (writeRuntimePlace? frame state place value).map fun written =>
        (written.1.shift offset, { state₂ with globals := written.2.globals.shift offset }) := by
  unfold writeRuntimePlace?
  split
  · rfl
  split
  · exact writeRoot?_shift globals_eq place.root value
  · simp only [Option.bind_eq_bind]
    rw [readRoot?_shift globals_eq]
    cases readRoot? frame state place.root with
    | none => rfl
    | some root =>
        simp only [Option.map_some, Option.bind_some]
        rw [writeProjections?_shift]
        cases writeProjections? root place.projections.toList value with
        | none => rfl
        | some updated => exact writeRoot?_shift globals_eq place.root updated

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

theorem resolvePlaceFuel?_shift (unit : Validation.ValidatedUnit) (ns : Validation.ValidatedNamespace)
    {frame : RuntimeFrame} {state state₂ : RuntimeState}
    (globals_eq : state₂.globals = state.globals.shift offset) :
    ∀ fuel : Nat,
      (∀ placeId, resolvePlaceFuel? unit ns (frame.shift offset) state₂ fuel placeId =
        resolvePlaceFuel? unit ns frame state fuel placeId) ∧
      (∀ expressionId, simpleIndexFuel? unit ns (frame.shift offset) state₂ fuel expressionId =
        simpleIndexFuel? unit ns frame state fuel expressionId)
  | 0 => ⟨fun _ => by simp [resolvePlaceFuel?], fun _ => by simp [simpleIndexFuel?]⟩
  | fuel + 1 => by
      obtain ⟨places, indices⟩ := resolvePlaceFuel?_shift unit ns (frame := frame) globals_eq fuel
      have read := fun place => readRuntimePlace?_shift (frame := frame) globals_eq place
      constructor
      · intro placeId
        simp only [resolvePlaceFuel?, Option.bind_eq_bind]
        cases ns.places[placeId.index]? with
        | none => rfl
        | some place =>
            simp only [Option.bind_some]
            cases place with
            | localVar localId => simp [RuntimeFrame.shift_locals]
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
                simp only [readLocal?_shift]
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

theorem resolvePlace?_shift (unit : Validation.ValidatedUnit) (ns : Validation.ValidatedNamespace)
    {frame : RuntimeFrame} {state state₂ : RuntimeState}
    (globals_eq : state₂.globals = state.globals.shift offset) (placeId : PlaceId) :
    resolvePlace? unit ns (frame.shift offset) state₂ placeId =
      resolvePlace? unit ns frame state placeId :=
  (resolvePlaceFuel?_shift unit ns globals_eq _).1 placeId

/-- Whether a place mismatches a variant reads shapes only, which shifts keep. -/
theorem placeVariantMismatchFuel?_shift (unit : Validation.ValidatedUnit)
    (ns : Validation.ValidatedNamespace) {frame : RuntimeFrame} {state state₂ : RuntimeState}
    (globals_eq : state₂.globals = state.globals.shift offset) :
    ∀ fuel placeId, placeVariantMismatchFuel? unit ns (frame.shift offset) state₂ fuel placeId =
      placeVariantMismatchFuel? unit ns frame state fuel placeId
  | 0, _ => rfl
  | fuel + 1, placeId => by
      have ih := placeVariantMismatchFuel?_shift unit ns (frame := frame) globals_eq fuel
      have resolve := (resolvePlaceFuel?_shift unit ns (frame := frame) globals_eq fuel).1
      have read := fun place => readRuntimePlace?_shift (frame := frame) globals_eq place
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

theorem variantMismatch?_shift (unit : Validation.ValidatedUnit)
    (ns : Validation.ValidatedNamespace) (operation : Operation) (arguments : Array RuntimeValue)
    {frame : RuntimeFrame} {state state₂ : RuntimeState}
    (globals_eq : state₂.globals = state.globals.shift offset) :
    variantMismatch? unit ns operation (arguments.map (·.shift offset)) (frame.shift offset)
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
            cases a <;> simp only [RuntimeValue.shift]
      | _ => rfl
  | copy place | read place | move place | borrow _ place | write place | drop place =>
      simp only [variantMismatch?, placeVariantMismatchFuel?_shift unit ns globals_eq]
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

theorem bindPatternRow_shift
    {bind bind' : PatternId → RuntimeValue → RuntimeFrame → Option RuntimeFrame}
    (commutes : ∀ pattern value frame, bind' pattern (value.shift offset) (frame.shift offset) =
      (bind pattern value frame).map (·.shift offset)) :
    ∀ (patterns : List PatternId) (values : List RuntimeValue) (frame : RuntimeFrame),
      bindPatternRow bind' patterns (values.map (·.shift offset)) (frame.shift offset) =
        (bindPatternRow bind patterns values frame).map (·.shift offset)
  | [], [], frame => rfl
  | [], _ :: _, _ => rfl
  | _ :: _, [], _ => rfl
  | pattern :: patterns, value :: values, frame => by
      simp only [List.map_cons, bindPatternRow, Option.bind_eq_bind, commutes]
      cases bind pattern value frame with
      | none => rfl
      | some bound => exact bindPatternRow_shift commutes patterns values bound

theorem bindPatternFuel_shift (unit : Validation.ValidatedUnit)
    (ns : Validation.ValidatedNamespace) :
    ∀ (fuel : Nat) (frame : RuntimeFrame) (patternId : PatternId) (value : RuntimeValue),
      bindPatternFuel unit ns (frame.shift offset) fuel patternId (value.shift offset) =
        (bindPatternFuel unit ns frame fuel patternId value).map (·.shift offset)
  | 0, _, _, _ => rfl
  | fuel + 1, frame, patternId, value => by
      have row := bindPatternRow_shift (offset := offset)
        (bind := fun pattern value frame => bindPatternFuel unit ns frame fuel pattern value)
        (bind' := fun pattern value frame => bindPatternFuel unit ns frame fuel pattern value)
        (fun pattern value frame => bindPatternFuel_shift unit ns fuel frame pattern value)
      simp only [bindPatternFuel, Option.bind_eq_bind]
      cases ns.patterns[patternId.index]? with
      | none => rfl
      | some pattern =>
          simp only [Option.bind_some]
          generalize pattern.kind = kind
          cases kind <;> dsimp only
          case wildcard => rfl
          case «variable» localId =>
              simp only [RuntimeFrame.shift_locals, Array.size_map]
              split
              · simp [RuntimeFrame.shift, Array.map_setIfInBounds, pure]
              · rfl
          case tuple elements =>
              cases value <;> simp only [RuntimeValue.shift_tuple, RuntimeValue.shift_unit, RuntimeValue.shift_bool,
                RuntimeValue.shift_character, RuntimeValue.shift_integer, RuntimeValue.shift_address,
                RuntimeValue.shift_signer, RuntimeValue.shift_string, RuntimeValue.shift_bytes,
                RuntimeValue.shift_vector, RuntimeValue.shift_nominal, RuntimeValue.shift_closure,
                RuntimeValue.shift_borrow, RuntimeValue.shift_loanHole] <;> try rfl
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
                  rw [← RuntimeValue.shift_of_plain offset plain, RuntimeValue.shift_beq,
                    RuntimeValue.shift_of_plain offset plain]
                  split <;> rfl
          case range lower upper inclusive =>
              cases value <;> simp only [RuntimeValue.shift_tuple, RuntimeValue.shift_unit, RuntimeValue.shift_bool,
                RuntimeValue.shift_character, RuntimeValue.shift_integer, RuntimeValue.shift_address,
                RuntimeValue.shift_signer, RuntimeValue.shift_string, RuntimeValue.shift_bytes,
                RuntimeValue.shift_vector, RuntimeValue.shift_nominal, RuntimeValue.shift_closure,
                RuntimeValue.shift_borrow, RuntimeValue.shift_loanHole] <;> try rfl
              all_goals (repeat' split) <;> simp_all
          case constructor name spelled variant fields =>
              cases value <;> simp only [RuntimeValue.shift_tuple, RuntimeValue.shift_unit, RuntimeValue.shift_bool,
                RuntimeValue.shift_character, RuntimeValue.shift_integer, RuntimeValue.shift_address,
                RuntimeValue.shift_signer, RuntimeValue.shift_string, RuntimeValue.shift_bytes,
                RuntimeValue.shift_vector, RuntimeValue.shift_nominal, RuntimeValue.shift_closure,
                RuntimeValue.shift_borrow, RuntimeValue.shift_loanHole] <;> try rfl
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

theorem bindPattern_shift (unit : Validation.ValidatedUnit) (ns : Validation.ValidatedNamespace)
    (frame : RuntimeFrame) (patternId : PatternId) (value : RuntimeValue) :
    bindPattern unit ns (frame.shift offset) patternId (value.shift offset) =
      (bindPattern unit ns frame patternId value).map (·.shift offset) :=
  bindPatternFuel_shift unit ns _ frame patternId value

theorem bindPattern_above {unit : Validation.ValidatedUnit} {ns : Validation.ValidatedNamespace}
    {frontier : Nat} {frame frame' : RuntimeFrame} (frame_above : frame.Above frontier)
    {value : RuntimeValue} (value_above : value.Above frontier) {patternId : PatternId}
    (bind : bindPattern unit ns frame patternId value = some frame') : frame'.Above frontier := by
  rw [← RuntimeFrame.shift_unshift frame_above,
    ← RuntimeValue.shift_unshift frontier value value_above, bindPattern_shift] at bind
  obtain ⟨base, -, rfl⟩ := Option.map_eq_some_iff.mp bind
  exact RuntimeFrame.above_shift_self frontier base

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

@[simp] theorem RuntimeFrame.shift_activeLoans (frame : RuntimeFrame) :
    (frame.shift offset).activeLoans =
      frame.activeLoans.map fun entry => (entry.1, entry.2 + offset) := rfl

@[simp] theorem RuntimeFrame.shift_loanLocations (frame : RuntimeFrame) :
    (frame.shift offset).loanLocations = frame.loanLocations.map (shiftEntry offset) := rfl

theorem localLoanPlace?_shift (frame : RuntimeFrame) (loan : Nat) :
    localLoanPlace? (frame.shift offset) (loan + offset) = localLoanPlace? frame loan := by
  unfold localLoanPlace?
  rw [RuntimeFrame.shift_loanLocations, Array.findRev?_map']
  have same : ((fun entry : Nat × RuntimePlace => entry.1 == loan + offset) ∘ shiftEntry offset) =
      fun entry => entry.1 == loan := by
    funext entry; simp [Nat.add_beq_add]
  rw [same]
  cases frame.loanLocations.findRev? (·.1 == loan) <;> rfl

theorem holeWithin_shift (loan : Nat) (value : RuntimeValue) :
    holeWithin (loan + offset) (value.shift offset) = holeWithin loan value := by
  unfold holeWithin
  rw [findFirst_shift (g := holeMark? loan) (φ := id) (fun value => holeMark?_shift loan value)]
  simp

theorem holeInFrame_shift (frame : RuntimeFrame) (loan : Nat) :
    holeInFrame (frame.shift offset) (loan + offset) = holeInFrame frame loan := by
  unfold holeInFrame
  rw [RuntimeFrame.shift_locals, Array.toList_map, indexOfFrom_map]
  congr 2
  funext slot
  cases slot <;> simp [holeWithin_shift]

theorem fillHole?_shift (loan : Nat) (replacement value : RuntimeValue) :
    fillHole? (loan + offset) (replacement.shift offset) (value.shift offset) =
      (fillHole? loan replacement value).map (·.shift offset) :=
  rewriteFirst_shift (fun value => holeFill?_shift loan replacement value) value

theorem transferredLoan?_shift (replacement : RuntimeValue) :
    transferredLoan? (replacement.shift offset) = (transferredLoan? replacement).map (· + offset) :=
  findFirst_shift (fun value => anyHole?_shift value) replacement

theorem transferActiveLoan_shift (activeLoans : Array (ExprId × Nat)) (loan : Nat)
    (replacement : RuntimeValue) :
    transferActiveLoan (activeLoans.map fun entry => (entry.1, entry.2 + offset)) (loan + offset)
        (replacement.shift offset) =
      (transferActiveLoan activeLoans loan replacement).map fun entry =>
        (entry.1, entry.2 + offset) := by
  unfold transferActiveLoan
  rw [transferredLoan?_shift, Array.findRev?_map', Array.filter_map]
  have keep : ((fun entry : ExprId × Nat => entry.2 != loan + offset) ∘
      fun entry : ExprId × Nat => (entry.1, entry.2 + offset)) = fun entry => entry.2 != loan := by
    funext entry; simp [bne, Nat.add_beq_add]
  have found : ((fun entry : ExprId × Nat => entry.2 == loan + offset) ∘
      fun entry : ExprId × Nat => (entry.1, entry.2 + offset)) = fun entry => entry.2 == loan := by
    funext entry; simp [Nat.add_beq_add]
  rw [keep, found]
  cases transferredLoan? replacement <;> cases activeLoans.findRev? (·.2 == loan) <;>
    simp [Array.map_push]

theorem transferLoanLocation_shift (loanLocations : Array (Nat × RuntimePlace)) (loan : Nat)
    (replacement : RuntimeValue) :
    transferLoanLocation (loanLocations.map (shiftEntry offset)) (loan + offset)
        (replacement.shift offset) =
      (transferLoanLocation loanLocations loan replacement).map (shiftEntry offset) := by
  unfold transferLoanLocation
  rw [transferredLoan?_shift, Array.findRev?_map', Array.filter_map]
  have keep : ((fun entry : Nat × RuntimePlace => entry.1 != loan + offset) ∘
      shiftEntry offset) = fun entry => entry.1 != loan := by
    funext entry; simp [bne, Nat.add_beq_add]
  have found : ((fun entry : Nat × RuntimePlace => entry.1 == loan + offset) ∘
      shiftEntry offset) = fun entry => entry.1 == loan := by
    funext entry; simp [Nat.add_beq_add]
  rw [keep, found]
  cases transferredLoan? replacement <;> cases loanLocations.findRev? (·.1 == loan) <;>
    simp [Array.map_push, shiftEntry]

/-! ## The loan registry -/

section Registry

variable {frontier : Nat}

theorem globalLoanKeyIn?_append (first second : List (Nat × GlobalKey)) (loan : Nat) :
    globalLoanKeyIn? (first ++ second) loan =
      (globalLoanKeyIn? first loan).or (globalLoanKeyIn? second loan) := by
  unfold globalLoanKeyIn?
  rw [List.find?_append]
  cases first.find? (·.1 == loan) <;> rfl

theorem globalLoanKeyIn?_below {junk : List (Nat × GlobalKey)} {bound loan : Nat}
    (below : ∀ entry ∈ junk, entry.1 < bound) (above : bound ≤ loan) :
    globalLoanKeyIn? junk loan = none := by
  unfold globalLoanKeyIn?
  rw [Option.map_eq_none_iff, List.find?_eq_none]
  intro entry member
  have := below entry member
  simp only [beq_iff_eq]
  omega

theorem globalLoanKeyIn?_map_shift (minted : List (Nat × GlobalKey)) (loan : Nat) :
    globalLoanKeyIn? (minted.map (shiftEntry offset)) (loan + offset) =
      globalLoanKeyIn? minted loan := by
  unfold globalLoanKeyIn?
  rw [List.find?_map, Option.map_map]
  have same : ((fun entry : Nat × GlobalKey => entry.1 == loan + offset) ∘ shiftEntry offset) =
      fun entry => entry.1 == loan := by
    funext entry; simp [Nat.add_beq_add]
  rw [same]
  rfl

theorem RegistryShifted.lookup {registry registry' : List (Nat × GlobalKey)}
    (shifted : RegistryShifted offset frontier registry registry') {loan : Nat}
    (above : frontier ≤ loan) :
    globalLoanKeyIn? registry' (loan + offset) = globalLoanKeyIn? registry loan := by
  obtain ⟨minted, junk, junk', rfl, rfl, -, below, below'⟩ := shifted
  rw [globalLoanKeyIn?_append, globalLoanKeyIn?_append, globalLoanKeyIn?_map_shift,
    globalLoanKeyIn?_below below above, globalLoanKeyIn?_below below' (by omega)]

theorem removeGlobalLoan_append_free (minted junk : List (Nat × GlobalKey)) {loan : Nat}
    (free : globalLoanKeyIn? junk loan = none) :
    removeGlobalLoan (minted ++ junk) loan = removeGlobalLoan minted loan ++ junk := by
  induction minted with
  | nil => simp [removeGlobalLoan_of_free junk loan free, removeGlobalLoan]
  | cons entry rest ih =>
      simp only [List.cons_append, removeGlobalLoan]
      split <;> simp [ih]

theorem removeGlobalLoan_map_shift (minted : List (Nat × GlobalKey)) (loan : Nat) :
    removeGlobalLoan (minted.map (shiftEntry offset)) (loan + offset) =
      (removeGlobalLoan minted loan).map (shiftEntry offset) := by
  induction minted with
  | nil => rfl
  | cons entry rest ih =>
      simp only [List.map_cons, removeGlobalLoan, shiftEntry, Nat.add_beq_add]
      split <;> simp [ih, shiftEntry]

theorem removeGlobalLoan_sublist (registry : List (Nat × GlobalKey)) (loan : Nat) :
    ∀ entry ∈ removeGlobalLoan registry loan, entry ∈ registry := by
  induction registry with
  | nil => simp [removeGlobalLoan]
  | cons head rest ih =>
      intro entry member
      simp only [removeGlobalLoan] at member
      split at member
      · exact List.mem_cons_of_mem _ member
      · rcases List.mem_cons.mp member with rfl | member
        · exact List.mem_cons_self
        · exact List.mem_cons_of_mem _ (ih entry member)

theorem RegistryShifted.remove {registry registry' : List (Nat × GlobalKey)}
    (shifted : RegistryShifted offset frontier registry registry') {loan : Nat}
    (above : frontier ≤ loan) :
    RegistryShifted offset frontier (removeGlobalLoan registry loan)
      (removeGlobalLoan registry' (loan + offset)) := by
  obtain ⟨minted, junk, junk', rfl, rfl, minted_above, below, below'⟩ := shifted
  refine ⟨removeGlobalLoan minted loan, junk, junk',
    removeGlobalLoan_append_free minted junk (globalLoanKeyIn?_below below above), ?_,
    fun entry member => minted_above entry (removeGlobalLoan_sublist minted loan entry member),
    below, below'⟩
  rw [removeGlobalLoan_append_free _ junk' (globalLoanKeyIn?_below below' (by omega)),
    removeGlobalLoan_map_shift]

theorem RegistryShifted.cons {registry registry' : List (Nat × GlobalKey)}
    (shifted : RegistryShifted offset frontier registry registry') {loan : Nat}
    (above : frontier ≤ loan) (key : GlobalKey) :
    RegistryShifted offset frontier ((loan, key) :: registry) ((loan + offset, key) :: registry') := by
  obtain ⟨minted, junk, junk', rfl, rfl, minted_above, below, below'⟩ := shifted
  refine ⟨(loan, key) :: minted, junk, junk', rfl, rfl, ?_, below, below'⟩
  intro entry member
  rcases List.mem_cons.mp member with rfl | member
  · exact above
  · exact minted_above entry member

theorem transferredLoan?_above {value : RuntimeValue} (above : value.Above frontier)
    {loan : Nat} (found : transferredLoan? value = some loan) : frontier ≤ loan := by
  rw [← RuntimeValue.shift_unshift frontier value above, transferredLoan?_shift] at found
  obtain ⟨base, -, rfl⟩ := Option.map_eq_some_iff.mp found
  omega

theorem fillHole?_above {loan : Nat} {replacement value result : RuntimeValue}
    (loan_above : frontier ≤ loan) (replacement_above : replacement.Above frontier)
    (value_above : value.Above frontier)
    (filled : fillHole? loan replacement value = some result) : result.Above frontier := by
  rw [← RuntimeValue.shift_unshift frontier value value_above,
    ← RuntimeValue.shift_unshift frontier replacement replacement_above,
    ← Nat.sub_add_cancel loan_above, fillHole?_shift] at filled
  obtain ⟨base, -, rfl⟩ := Option.map_eq_some_iff.mp filled
  exact RuntimeValue.above_shift_self frontier base

theorem RegistryShifted.transfer {registry registry' : List (Nat × GlobalKey)}
    (shifted : RegistryShifted offset frontier registry registry') {loan : Nat}
    (above : frontier ≤ loan) (key : GlobalKey) {replacement : RuntimeValue}
    (replacement_above : replacement.Above frontier) :
    RegistryShifted offset frontier (transferGlobalLoan registry loan key replacement)
      (transferGlobalLoan registry' (loan + offset) key (replacement.shift offset)) := by
  unfold transferGlobalLoan
  simp only [transferredLoan?_shift]
  cases found : transferredLoan? replacement with
  | none => exact shifted.remove above
  | some transferred =>
      exact (shifted.remove above).cons (transferredLoan?_above replacement_above found) key

end Registry

theorem RuntimeFrame.shift_withLoans (frame : RuntimeFrame) (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace)) :
    ({ frame with activeLoans, loanLocations } : RuntimeFrame).shift offset =
      { frame.shift offset with
        activeLoans := activeLoans.map fun entry => (entry.1, entry.2 + offset)
        loanLocations := loanLocations.map (shiftEntry offset) } := rfl

theorem fillLocalLoanHole?_shift {frame : RuntimeFrame} {state state₂ : RuntimeState}
    (globals_eq : state₂.globals = state.globals.shift offset) (loan : Nat)
    (replacement : RuntimeValue) :
    fillLocalLoanHole? (frame.shift offset) state₂ (loan + offset) (replacement.shift offset) =
      (fillLocalLoanHole? frame state loan replacement).map fun resolved =>
        (resolved.1.shift offset, state₂) := by
  unfold fillLocalLoanHole?
  simp only [Option.bind_eq_bind]
  rw [localLoanPlace?_shift]
  cases localLoanPlace? frame loan with
  | none => rfl
  | some place =>
      simp only [Option.bind_some]
      rw [readRuntimePlace?_shift globals_eq]
      cases readRuntimePlace? frame state place with
      | none => rfl
      | some value =>
          simp only [Option.map_some, Option.bind_some]
          rw [fillHole?_shift]
          cases fillHole? loan replacement value with
          | none => rfl
          | some filled =>
              simp only [Option.map_some, Option.bind_some]
              rw [writeRuntimePlace?_shift globals_eq]
              cases writeRuntimePlace? frame state place filled with
              | none => rfl
              | some written =>
                  simp only [Option.map_some, Option.bind_some, RuntimeFrame.shift_activeLoans,
                    RuntimeFrame.shift_loanLocations, transferActiveLoan_shift,
                    transferLoanLocation_shift, RuntimeFrame.shift_withLoans]

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

variable {frontier inert inert' : Nat} {state state₂ : RuntimeState}

theorem StateShifted.withGlobals (shifted : StateShifted offset frontier inert inert' state state₂)
    (globals : GlobalMap) (above : ∀ slot ∈ globals.entries, slot.value.Above frontier) :
    StateShifted offset frontier inert inert' { state with globals }
      { state₂ with globals := globals.shift offset } :=
  { shifted with globals := rfl, globalsAbove := above }

theorem StateShifted.withRegistry (shifted : StateShifted offset frontier inert inert' state state₂)
    {registry registry' : List (Nat × GlobalKey)}
    (related : RegistryShifted offset frontier registry registry') :
    StateShifted offset frontier inert inert' { state with globalLoans := registry }
      { state₂ with globalLoans := registry' } :=
  { shifted with registry := related }

theorem StateShifted.push (shifted : StateShifted offset frontier inert inert' state state₂)
    {loan : Nat} (above : frontier ≤ loan) {value : RuntimeValue}
    (value_above : value.Above frontier) :
    StateShifted offset frontier inert inert'
      { state with pending := state.pending.push (loan, value) }
      { state₂ with pending := state₂.pending.push (loan + offset, value.shift offset) } where
  globals := shifted.globals
  nextLoan := shifted.nextLoan
  frontier_le := shifted.frontier_le
  inert_le := by simp only [Array.size_push]; exact Nat.le_succ_of_le shifted.inert_le
  inert_le' := by simp only [Array.size_push]; exact Nat.le_succ_of_le shifted.inert_le'
  pending := by
    simp only
    rw [Array.extract_push_size _ _ shifted.inert_le', Array.extract_push_size _ _ shifted.inert_le,
      shifted.pending, Array.map_push]
  registry := shifted.registry
  globalsAbove := shifted.globalsAbove
  pendingAbove := by
    simp only
    rw [Array.extract_push_size _ _ shifted.inert_le]
    intro entry member
    rcases Array.mem_push.mp member with member | rfl
    · exact shifted.pendingAbove entry member
    · exact ⟨above, value_above⟩

theorem StateShifted.lookup (shifted : StateShifted offset frontier inert inert' state state₂)
    {loan : Nat} (above : frontier ≤ loan) :
    globalLoanKey? state₂ (loan + offset) = globalLoanKey? state loan :=
  shifted.registry.lookup above

theorem StateShifted.slot_above (shifted : StateShifted offset frontier inert inert' state state₂)
    {key : GlobalKey} {value : RuntimeValue} (lookup_eq : state.globals.lookup key = some value) :
    value.Above frontier := by
  unfold GlobalMap.lookup at lookup_eq
  obtain ⟨slot, found, rfl⟩ := Option.map_eq_some_iff.mp lookup_eq
  exact shifted.globalsAbove slot (Array.mem_of_find?_eq_some found)

theorem GlobalMap.insert_above {globals : GlobalMap} {key : GlobalKey} {value : RuntimeValue}
    (above : ∀ slot ∈ globals.entries, slot.value.Above frontier)
    (value_above : value.Above frontier) :
    ∀ slot ∈ (globals.insert key value).entries, slot.value.Above frontier := by
  intro slot member
  rcases GlobalMap.mem_insert.mp member with ⟨member, -⟩ | rfl
  · exact above slot member
  · exact value_above

theorem RuntimeFrame.Above.setLocal {frame : RuntimeFrame} (above : frame.Above frontier)
    (index : Nat) {value : RuntimeValue} (value_above : value.Above frontier) :
    ({ frame with locals := frame.locals.set! index (some value) } : RuntimeFrame).Above frontier where
  locals slot member stored stored_eq := by
    simp only [Array.set!_eq_setIfInBounds] at member
    rcases Array.mem_or_eq_of_mem_setIfInBounds member with member | rfl
    · exact above.locals slot member stored stored_eq
    · cases stored_eq; exact value_above
  activeLoans := above.activeLoans
  loanLocations := above.loanLocations

theorem RuntimeFrame.Above.slot {frame : RuntimeFrame} (above : frame.Above frontier)
    {index : Nat} {value : RuntimeValue} (slot_eq : frame.locals[index]? = some (some value)) :
    value.Above frontier :=
  above.locals _ (Array.mem_of_getElem? slot_eq) value rfl

theorem holeSlot_shift (loan : Nat) :
    ((fun slot : Option RuntimeValue => (slot.map (holeWithin (loan + offset))).getD false) ∘
        Option.map (·.shift offset)) =
      fun slot => (slot.map (holeWithin loan)).getD false := by
  funext slot; cases slot <;> simp [holeWithin_shift]

theorem fillVisibleHole_shift {frame : RuntimeFrame}
    (shifted : StateShifted offset frontier inert inert' state state₂)
    (frame_above : frame.Above frontier) {loan : Nat} (above : frontier ≤ loan)
    {replacement : RuntimeValue} (replacement_above : replacement.Above frontier) :
    ∃ state₂', fillVisibleHole (frame.shift offset) state₂ (loan + offset)
        (replacement.shift offset) =
      ((fillVisibleHole frame state loan replacement).1.shift offset, state₂',
        (fillVisibleHole frame state loan replacement).2.2) ∧
      StateShifted offset frontier inert inert' (fillVisibleHole frame state loan replacement).2.1
        state₂' ∧
      (fillVisibleHole frame state loan replacement).1.Above frontier := by
  unfold fillVisibleHole
  rw [holeInFrame_shift]
  split
  · rw [RuntimeFrame.shift_locals, Array.toList_map, indexOfFrom_map, holeSlot_shift]
    cases indexOfFrom (fun slot => (slot.map (holeWithin loan)).getD false)
        frame.locals.toList 0 with
    | none => exact ⟨state₂, rfl, shifted, frame_above⟩
    | some index =>
        simp only [Array.getElem?_map]
        cases slot_eq : frame.locals[index]? with
        | none => exact ⟨state₂, rfl, shifted, frame_above⟩
        | some slot =>
            cases slot with
            | none => exact ⟨state₂, rfl, shifted, frame_above⟩
            | some value =>
                simp only [Option.map_some, Option.bind_some, fillHole?_shift]
                cases fill_eq : fillHole? loan replacement value with
                | none => exact ⟨state₂, rfl, shifted, frame_above⟩
                | some filled =>
                    refine ⟨state₂, ?_, shifted, frame_above.setLocal index
                      (fillHole?_above above replacement_above (frame_above.slot slot_eq) fill_eq)⟩
                    simp [RuntimeFrame.shift, Array.map_setIfInBounds]
  · rw [shifted.lookup above]
    cases globalLoanKey? state loan with
    | none => exact ⟨state₂, rfl, shifted, frame_above⟩
    | some key =>
        simp only [shifted.globals, GlobalMap.lookup_shift]
        cases lookup_eq : state.globals.lookup key with
        | none => exact ⟨state₂, rfl, shifted, frame_above⟩
        | some value =>
            simp only [Option.map_some, Option.bind_some, fillHole?_shift]
            cases fill_eq : fillHole? loan replacement value with
            | none => exact ⟨state₂, rfl, shifted, frame_above⟩
            | some filled =>
                refine ⟨{ state₂ with
                    globals := (state.globals.insert key filled).shift offset
                    globalLoans := transferGlobalLoan state₂.globalLoans (loan + offset) key
                      (replacement.shift offset) },
                  by simp [GlobalMap.insert_shift], ?_, frame_above⟩
                exact (shifted.withGlobals _ (GlobalMap.insert_above shifted.globalsAbove
                  (fillHole?_above above replacement_above (shifted.slot_above lookup_eq) fill_eq))
                  ).withRegistry (shifted.registry.transfer above key replacement_above)

theorem applyWriteBack_shift {frame : RuntimeFrame}
    (shifted : StateShifted offset frontier inert inert' state state₂)
    (frame_above : frame.Above frontier) {loan : Nat} (above : frontier ≤ loan)
    {current : RuntimeValue} (current_above : current.Above frontier) :
    ∃ state₂', applyWriteBack (frame.shift offset) state₂ (loan + offset) (current.shift offset) =
      ((applyWriteBack frame state loan current).1.shift offset, state₂') ∧
      StateShifted offset frontier inert inert' (applyWriteBack frame state loan current).2 state₂' ∧
      (applyWriteBack frame state loan current).1.Above frontier := by
  obtain ⟨state₂', fill_eq, filled, filled_above⟩ :=
    fillVisibleHole_shift shifted frame_above above current_above
  unfold applyWriteBack
  rw [fill_eq]
  simp only
  split
  · exact ⟨state₂', rfl, filled, filled_above⟩
  · exact ⟨_, rfl, filled.push above current_above, filled_above⟩

/-- A rewrite by a matcher that commutes with shifts keeps values above a
frontier its loan is at or beyond. -/
theorem rewriteFirst_above_of_shift {matcher : Nat → RuntimeValue → Option RuntimeValue}
    (commutes : ∀ offset loan value, matcher (loan + offset) (value.shift offset) =
      (matcher loan value).map (·.shift offset))
    {loan : Nat} (above : frontier ≤ loan) {value result : RuntimeValue}
    (value_above : value.Above frontier) (rewrite : rewriteFirst (matcher loan) value = some result) :
    result.Above frontier := by
  rw [← RuntimeValue.shift_unshift frontier value value_above, ← Nat.sub_add_cancel above,
    rewriteFirst_shift (commutes frontier (loan - frontier))] at rewrite
  obtain ⟨base, -, rfl⟩ := Option.map_eq_some_iff.mp rewrite
  exact RuntimeValue.above_shift_self frontier base

theorem findFirst_above_of_shift {matcher : Nat → RuntimeValue → Option RuntimeValue}
    (commutes : ∀ offset loan value, matcher (loan + offset) (value.shift offset) =
      (matcher loan value).map (·.shift offset))
    {loan : Nat} (above : frontier ≤ loan) {value result : RuntimeValue}
    (value_above : value.Above frontier) (query : findFirst (matcher loan) value = some result) :
    result.Above frontier := by
  rw [← RuntimeValue.shift_unshift frontier value value_above, ← Nat.sub_add_cancel above,
    findFirst_shift (commutes frontier (loan - frontier))] at query
  obtain ⟨base, -, rfl⟩ := Option.map_eq_some_iff.mp query
  exact RuntimeValue.above_shift_self frontier base

theorem Array.findSome?_map_result {α β γ : Type} (values : Array α) (find : α → Option β)
    (f : β → γ) :
    values.findSome? (fun value => (find value).map f) = (values.findSome? find).map f := by
  rw [← Array.findSome?_toList, ← Array.findSome?_toList]
  induction values.toList with
  | nil => rfl
  | cons head tail ih =>
      simp only [List.findSome?_cons]
      cases find head <;> simp [ih]

theorem findBorrowValue?_shift {frame : RuntimeFrame}
    (shifted : StateShifted offset frontier inert inert' state state₂) (loan : Nat) :
    findBorrowValue? (frame.shift offset) state₂ (loan + offset) =
      (findBorrowValue? frame state loan).map (·.shift offset) := by
  unfold findBorrowValue?
  have node := fun value => borrowCurrent?_shift (offset := offset) loan value
  simp only [RuntimeFrame.shift_locals, Array.findSome?_map, shifted.globals,
    GlobalMap.shift_entries]
  have locals : ((fun slot : Option RuntimeValue =>
        slot.bind fun value => findFirst (borrowCurrent? (loan + offset)) value) ∘
          Option.map fun value => value.shift offset) =
      fun slot => (slot.bind fun value => findFirst (borrowCurrent? loan) value).map
        (·.shift offset) := by
    funext slot; cases slot <;> simp [findFirst_shift node]
  have globals : ((fun slot : GlobalSlot => findFirst (borrowCurrent? (loan + offset)) slot.value) ∘
        fun slot : GlobalSlot => { slot with value := slot.value.shift offset }) =
      fun slot => (findFirst (borrowCurrent? loan) slot.value).map (·.shift offset) := by
    funext slot; simp [findFirst_shift node]
  rw [locals, globals, Array.findSome?_map_result, Array.findSome?_map_result]
  cases frame.locals.findSome? (fun slot => slot.bind fun value =>
    findFirst (borrowCurrent? loan) value) <;> rfl

theorem findBorrowValue?_above {frame : RuntimeFrame}
    (shifted : StateShifted offset frontier inert inert' state state₂)
    (frame_above : frame.Above frontier) {loan : Nat} (above : frontier ≤ loan)
    {current : RuntimeValue} (found : findBorrowValue? frame state loan = some current) :
    current.Above frontier := by
  unfold findBorrowValue? at found
  have commutes := fun offset loan value => borrowCurrent?_shift (offset := offset) loan value
  simp only at found
  split at found
  · rename_i result result_eq
    simp only [Option.some.injEq] at found
    subst found
    obtain ⟨slot, member, found⟩ := Array.exists_of_findSome?_eq_some result_eq
    cases slot with
    | none => cases found
    | some value =>
        exact findFirst_above_of_shift commutes above
          (frame_above.locals _ member value rfl) found
  · obtain ⟨slot, member, found⟩ := Array.exists_of_findSome?_eq_some found
    exact findFirst_above_of_shift commutes above (shifted.globalsAbove slot member) found

theorem clearBorrowValue_shift {frame : RuntimeFrame}
    (shifted : StateShifted offset frontier inert inert' state state₂)
    (frame_above : frame.Above frontier) {loan : Nat} (above : frontier ≤ loan) :
    ∃ state₂', clearBorrowValue (frame.shift offset) state₂ (loan + offset) =
      ((clearBorrowValue frame state loan).1.shift offset, state₂') ∧
      StateShifted offset frontier inert inert' (clearBorrowValue frame state loan).2 state₂' ∧
      (clearBorrowValue frame state loan).1.Above frontier := by
  have commutes := fun offset loan value => borrowClear?_shift (offset := offset) loan value
  have node := fun value => borrowClear?_shift (offset := offset) loan value
  unfold clearBorrowValue
  simp only
  have locals : ((fun slot : Option RuntimeValue =>
        (slot.bind fun value => rewriteFirst (borrowClear? (loan + offset)) value).isSome) ∘
          Option.map fun value => value.shift offset) =
      fun slot => (slot.bind fun value => rewriteFirst (borrowClear? loan) value).isSome := by
    funext slot; cases slot <;> simp [rewriteFirst_shift node]
  rw [RuntimeFrame.shift_locals, Array.toList_map, indexOfFrom_map, locals]
  cases indexOfFrom (fun slot => (slot.bind fun value =>
      rewriteFirst (borrowClear? loan) value).isSome) frame.locals.toList 0 with
  | some index =>
      simp only [Array.getElem?_map]
      cases slot_eq : frame.locals[index]? with
      | none => exact ⟨state₂, rfl, shifted, frame_above⟩
      | some slot =>
          cases slot with
          | none => exact ⟨state₂, rfl, shifted, frame_above⟩
          | some value =>
              simp only [Option.map_some, Option.bind_some, rewriteFirst_shift node]
              cases clear_eq : rewriteFirst (borrowClear? loan) value with
              | none => exact ⟨state₂, rfl, shifted, frame_above⟩
              | some updated =>
                  refine ⟨state₂, by simp [RuntimeFrame.shift, Array.map_setIfInBounds], shifted,
                    frame_above.setLocal index (rewriteFirst_above_of_shift commutes above
                      (frame_above.slot slot_eq) clear_eq)⟩
  | none =>
      have globals : ((fun slot : GlobalSlot =>
            (rewriteFirst (borrowClear? (loan + offset)) slot.value).isSome) ∘
              fun slot : GlobalSlot => { slot with value := slot.value.shift offset }) =
          fun slot => (rewriteFirst (borrowClear? loan) slot.value).isSome := by
        funext slot; simp [rewriteFirst_shift node]
      rw [shifted.globals, GlobalMap.shift_entries, Array.toList_map, indexOfFrom_map, globals]
      cases indexOfFrom (fun slot : GlobalSlot =>
          (rewriteFirst (borrowClear? loan) slot.value).isSome) state.globals.entries.toList 0 with
      | none => exact ⟨state₂, rfl, shifted, frame_above⟩
      | some index =>
          simp only [Array.getElem?_map]
          cases slot_eq : state.globals.entries[index]? with
          | none => exact ⟨state₂, rfl, shifted, frame_above⟩
          | some slot =>
              simp only [Option.map_some, Option.bind_some, rewriteFirst_shift node]
              cases clear_eq : rewriteFirst (borrowClear? loan) slot.value with
              | none => exact ⟨state₂, rfl, shifted, frame_above⟩
              | some updated =>
                  refine ⟨{ state₂ with
                      globals := (⟨state.globals.entries.set! index { slot with value := updated }⟩ :
                        GlobalMap).shift offset },
                    by simp [GlobalMap.shift, Array.map_setIfInBounds], ?_, frame_above⟩
                  refine shifted.withGlobals _ fun slot' member => ?_
                  simp only [Array.set!_eq_setIfInBounds] at member
                  rcases Array.mem_or_eq_of_mem_setIfInBounds member with member | rfl
                  · exact shifted.globalsAbove slot' member
                  · exact rewriteFirst_above_of_shift commutes above
                      (shifted.globalsAbove slot (Array.mem_of_getElem? slot_eq)) clear_eq

/-- A frame and state of a second run mirroring those of a first. -/
def Mirrors (offset frontier inert inert' : Nat) (first second : RuntimeFrame × RuntimeState) :
    Prop :=
  second.1 = first.1.shift offset ∧
    StateShifted offset frontier inert inert' first.2 second.2 ∧ first.1.Above frontier

theorem settleLoans_shift {loans : Array LoanId} {frame : RuntimeFrame}
    (shifted : StateShifted offset frontier inert inert' state state₂)
    (frame_above : frame.Above frontier) :
    Mirrors offset frontier inert inert' (settleLoans loans frame state)
      (settleLoans loans (frame.shift offset) state₂) := by
  unfold settleLoans
  rw [← Array.foldr_toList, ← Array.foldr_toList]
  induction loans.toList with
  | nil => exact ⟨rfl, shifted, frame_above⟩
  | cons lexical rest ih =>
      simp only [List.foldr_cons]
      obtain ⟨same, related, above⟩ := ih
      generalize List.foldr _ (frame, state) rest = first at same related above
      generalize List.foldr _ (frame.shift offset, state₂) rest = second at same related above
      obtain ⟨current₁, state₁⟩ := first
      obtain ⟨current₂, state₂'⟩ := second
      simp only at same related above
      subst same
      simp only [RuntimeFrame.shift_activeLoans, Array.find?_map]
      have site : ((fun entry : ExprId × Nat => entry.1 == ⟨lexical.index⟩) ∘
          fun entry : ExprId × Nat => (entry.1, entry.2 + offset)) =
          fun entry => entry.1 == ⟨lexical.index⟩ := by
        funext entry; rfl
      rw [site]
      cases found : current₁.activeLoans.find? (·.1 == ⟨lexical.index⟩) with
      | none => exact ⟨rfl, related, above⟩
      | some entry =>
          obtain ⟨lexical', loanInstance⟩ := entry
          have instance_above : frontier ≤ loanInstance :=
            above.activeLoans _ (Array.mem_of_find?_eq_some found)
          simp only [Option.map_some]
          have filtered : RuntimeFrame.mk (current₁.shift offset).locals
              (Array.filter (fun entry => entry.fst != { index := lexical.index })
                (current₁.shift offset).activeLoans)
              (current₁.shift offset).loanLocations (current₁.shift offset).typeInstantiation =
              (RuntimeFrame.mk current₁.locals
                (current₁.activeLoans.filter (fun entry => entry.fst != { index := lexical.index }))
                current₁.loanLocations current₁.typeInstantiation).shift offset := by
            simp only [RuntimeFrame.shift, Array.filter_map]
            rfl
          have filtered_above : ({ current₁ with
              activeLoans := (current₁.activeLoans.filter (·.1 != ⟨lexical.index⟩)) } :
              RuntimeFrame).Above frontier :=
            { locals := above.locals
              activeLoans := fun entry member => above.activeLoans entry (Array.mem_filter.mp member).1
              loanLocations := above.loanLocations }
          rw [← RuntimeFrame.shift_activeLoans, filtered, findBorrowValue?_shift related]
          cases borrow_eq : findBorrowValue? { current₁ with
              activeLoans := (current₁.activeLoans.filter (·.1 != ⟨lexical.index⟩)) }
              state₁ loanInstance with
          | none => exact ⟨rfl, related, filtered_above⟩
          | some currentValue =>
              have current_above := findBorrowValue?_above related filtered_above instance_above
                borrow_eq
              obtain ⟨cleared₂, clear_eq, cleared, cleared_above⟩ :=
                clearBorrowValue_shift related filtered_above instance_above
              simp only [Option.map_some, clear_eq]
              obtain ⟨written₂, write_eq, written, written_above⟩ :=
                applyWriteBack_shift cleared cleared_above instance_above current_above
              rw [write_eq]
              exact ⟨rfl, written, written_above⟩

theorem settleAfter_shift {loans : Array LoanId} {control : Control} {frame : RuntimeFrame}
    (shifted : StateShifted offset frontier inert inert' state state₂)
    (frame_above : frame.Above frontier) :
    Mirrors offset frontier inert inert' (settleAfter loans control frame state)
      (settleAfter loans (control.shift offset) (frame.shift offset) state₂) := by
  cases control <;> simp only [settleAfter, Control.shift]
  · exact settleLoans_shift shifted frame_above
  all_goals exact ⟨rfl, shifted, frame_above⟩

end States

/-- A write from a frame and state above a frontier, of a value above it,
leaves them above it, and a second run's write mirrors it. -/
theorem writeRuntimePlace?_mirror {frontier inert inert' : Nat} {frame frame' : RuntimeFrame}
    {state state' state₂ : RuntimeState}
    (shifted : StateShifted offset frontier inert inert' state state₂)
    (frame_above : frame.Above frontier) {place : RuntimePlace} {value : RuntimeValue}
    (value_above : value.Above frontier)
    (write : writeRuntimePlace? frame state place value = some (frame', state')) :
    ∃ state₂', writeRuntimePlace? (frame.shift offset) state₂ place (value.shift offset) =
        some (frame'.shift offset, state₂') ∧
      StateShifted offset frontier inert inert' state' state₂' ∧ frame'.Above frontier := by
  have base_eq : state.globals =
      ({ state with globals := state.globals.unshift frontier } : RuntimeState).globals.shift
        frontier := (GlobalMap.shift_unshift shifted.globalsAbove).symm
  have base := writeRuntimePlace?_shift (offset := frontier) (frame := frame.unshift frontier)
    base_eq place (value.unshift frontier)
  rw [RuntimeFrame.shift_unshift frame_above,
    RuntimeValue.shift_unshift frontier value value_above, write] at base
  obtain ⟨written, -, written_eq⟩ := Option.map_eq_some_iff.mp base.symm
  simp only [Prod.mk.injEq] at written_eq
  obtain ⟨frame_eq, state_eq⟩ := written_eq
  have globals_above : ∀ slot ∈ state'.globals.entries, slot.value.Above frontier := by
    rw [← state_eq]
    exact GlobalMap.above_shift_self frontier _
  refine ⟨{ state₂ with globals := state'.globals.shift offset }, ?_, ?_, ?_⟩
  · rw [writeRuntimePlace?_shift shifted.globals, write]
    rfl
  · rw [writeRuntimePlace?_globals write]
    exact shifted.withGlobals _ globals_above
  · rw [← frame_eq]
    exact RuntimeFrame.above_shift_self frontier _

/-! ## Write-backs across calls -/

theorem applyPendingWriteBack_shift {frame : RuntimeFrame} {state state₂ : RuntimeState}
    (globals_eq : state₂.globals = state.globals.shift offset) (loan : Nat)
    (current : RuntimeValue) :
    (applyPendingWriteBack (frame.shift offset) state₂ (loan + offset) (current.shift offset)).1 =
        (applyPendingWriteBack frame state loan current).1.shift offset ∧
      (((applyPendingWriteBack frame state loan current).2 = state ∧
          (applyPendingWriteBack (frame.shift offset) state₂ (loan + offset)
            (current.shift offset)).2 = state₂) ∨
        ((applyPendingWriteBack frame state loan current).2 =
            { state with pending := state.pending.push (loan, current) } ∧
          (applyPendingWriteBack (frame.shift offset) state₂ (loan + offset)
            (current.shift offset)).2 =
            { state₂ with pending := state₂.pending.push (loan + offset, current.shift offset) })) := by
  unfold applyPendingWriteBack
  rw [fillLocalLoanHole?_shift globals_eq]
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
      rw [holeInFrame_shift]
      split
      · rw [RuntimeFrame.shift_locals, Array.toList_map, indexOfFrom_map, holeSlot_shift]
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
                    simp only [Option.map_some, Option.bind_some, fillHole?_shift]
                    cases fillHole? loan current value with
                    | none => exact ⟨rfl, .inr ⟨rfl, rfl⟩⟩
                    | some filled =>
                        refine ⟨?_, .inl ⟨rfl, rfl⟩⟩
                        simp only [Option.map_some, RuntimeFrame.shift_activeLoans,
                          RuntimeFrame.shift_loanLocations, transferActiveLoan_shift,
                          transferLoanLocation_shift]
                        simp [RuntimeFrame.shift, Array.map_setIfInBounds]
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

theorem StateShifted.pending_from {state state₂ : RuntimeState}
    (shifted : StateShifted offset frontier inert inert' state state₂) {start start' : Nat}
    (le : inert ≤ start) (le' : inert' ≤ start') (same : start - inert = start' - inert') :
    state₂.pending.extract start' state₂.pending.size =
      (state.pending.extract start state.pending.size).map fun entry =>
        (entry.1 + offset, entry.2.shift offset) := by
  have sizes := congrArg Array.size shifted.pending
  simp only [Array.size_extract, Array.size_map, Nat.min_self] at sizes
  rw [Array.extract_from _ le', shifted.pending, Array.extract_from _ le, ← Array.map_extract]
  congr 1
  rw [same, sizes]

theorem StateShifted.restart {state state₂ arguments arguments₂ : RuntimeState}
    (final : StateShifted offset frontier inert inert' state state₂)
    (calling : StateShifted offset frontier inert inert' arguments arguments₂) :
    StateShifted offset frontier inert inert' { state with pending := arguments.pending }
      { state₂ with pending := arguments₂.pending } where
  globals := final.globals
  nextLoan := final.nextLoan
  frontier_le := final.frontier_le
  inert_le := calling.inert_le
  inert_le' := calling.inert_le'
  pending := calling.pending
  registry := final.registry
  globalsAbove := final.globalsAbove
  pendingAbove := calling.pendingAbove

theorem applyPendingWriteBack_mirror {first second : RuntimeFrame × RuntimeState}
    (mirrors : Mirrors offset frontier inert inert' first second) {loan : Nat}
    (above : frontier ≤ loan) {current : RuntimeValue} (current_above : current.Above frontier) :
    Mirrors offset frontier inert inert'
      (applyPendingWriteBack first.1 first.2 loan current)
      (applyPendingWriteBack second.1 second.2 (loan + offset) (current.shift offset)) := by
  obtain ⟨frame, state⟩ := first
  obtain ⟨frame₂, state₂⟩ := second
  obtain ⟨rfl, shifted, frame_above⟩ := mirrors
  obtain ⟨frame_eq, states⟩ := applyPendingWriteBack_shift (frame := frame) shifted.globals loan current
  have base_eq : state.globals =
      ({ state with globals := state.globals.unshift frontier } : RuntimeState).globals.shift
        frontier := (GlobalMap.shift_unshift shifted.globalsAbove).symm
  have base := (applyPendingWriteBack_shift (offset := frontier) (frame := frame.unshift frontier)
    base_eq (loan - frontier) (current.unshift frontier)).1
  rw [RuntimeFrame.shift_unshift frame_above, Nat.sub_add_cancel above,
    RuntimeValue.shift_unshift frontier current current_above] at base
  refine ⟨frame_eq, ?_, base ▸ RuntimeFrame.above_shift_self frontier _⟩
  rcases states with ⟨first_eq, second_eq⟩ | ⟨first_eq, second_eq⟩
  · simp only at first_eq second_eq ⊢
    rw [first_eq, second_eq]
    exact shifted
  · simp only at first_eq second_eq ⊢
    rw [first_eq, second_eq]
    exact shifted.push above current_above

theorem applyPendingFrom_mirror {frame : RuntimeFrame}
    {arguments arguments₂ state state₂ : RuntimeState} (frame_above : frame.Above frontier)
    (calling : StateShifted offset frontier inert inert' arguments arguments₂)
    (final : StateShifted offset frontier inert inert' state state₂) :
    Mirrors offset frontier inert inert' (applyPendingFrom arguments.pending frame state)
      (applyPendingFrom arguments₂.pending (frame.shift offset) state₂) := by
  have sizes := congrArg Array.size calling.pending
  simp only [Array.size_extract, Array.size_map, Nat.min_self] at sizes
  have calling_le := calling.inert_le
  have calling_le' := calling.inert_le'
  unfold applyPendingFrom
  rw [final.pending_from calling_le calling_le' (by omega), Array.foldl_map]
  have entries : ∀ entry ∈ state.pending.extract arguments.pending.size state.pending.size,
      frontier ≤ entry.1 ∧ entry.2.Above frontier := fun entry member =>
    final.pendingAbove entry (Array.mem_extract_of_start_le member calling_le)
  rw [← Array.foldl_toList, ← Array.foldl_toList]
  have start : Mirrors offset frontier inert inert' (frame, { state with pending := arguments.pending })
      (frame.shift offset, { state₂ with pending := arguments₂.pending }) :=
    ⟨rfl, final.restart calling, frame_above⟩
  have entries' : ∀ entry ∈ (state.pending.extract arguments.pending.size
      state.pending.size).toList, frontier ≤ entry.1 ∧ entry.2.Above frontier :=
    fun entry member => entries entry (Array.mem_def.mpr member)
  clear entries
  generalize (state.pending.extract arguments.pending.size state.pending.size).toList = list
    at entries'
  suffices fold : ∀ (list : List (Nat × RuntimeValue)) (first second : RuntimeFrame × RuntimeState),
      (∀ entry ∈ list, frontier ≤ entry.1 ∧ entry.2.Above frontier) →
      Mirrors offset frontier inert inert' first second →
      Mirrors offset frontier inert inert'
        (list.foldl (fun pair entry => applyPendingWriteBack pair.1 pair.2 entry.1 entry.2) first)
        (list.foldl (fun pair entry =>
          applyPendingWriteBack pair.1 pair.2 (entry.1 + offset) (entry.2.shift offset)) second) by
    exact fold list _ _ entries' start
  intro list
  induction list with
  | nil => intro first second _ mirrors; exact mirrors
  | cons head tail ih =>
      intro first second entries mirrors
      simp only [List.foldl_cons]
      exact ih _ _ (fun entry member => entries entry (by simp [member]))
        (applyPendingWriteBack_mirror mirrors (entries head (by simp)).1 (entries head (by simp)).2)

theorem borrowEntry?_shift (value : RuntimeValue) :
    borrowEntry? (value.shift offset) =
      (borrowEntry? value).map (Option.map fun entry => (entry.1 + offset, entry.2.shift offset)) := by
  cases value <;> simp [borrowEntry?]

theorem outermostBorrows_shift (value : RuntimeValue) :
    outermostBorrows (value.shift offset) =
      (outermostBorrows value).map fun entry => (entry.1 + offset, entry.2.shift offset) := by
  unfold outermostBorrows
  rw [collectPruned_shift (fun value => borrowEntry?_shift value)]
  apply Array.ext'
  simp [List.filterMap_map, List.map_filterMap]

theorem returnedBorrows_shift (results : Array RuntimeValue) :
    (results.map (·.shift offset)).foldl (init := #[]) (fun found result =>
        found ++ outermostBorrows result) =
      (results.foldl (init := #[]) fun found result => found ++ outermostBorrows result).map
        fun entry => (entry.1 + offset, entry.2.shift offset) := by
  rw [Array.foldl_map, ← Array.foldl_toList, ← Array.foldl_toList]
  suffices general : ∀ (list : List RuntimeValue) (start : Array (Nat × RuntimeValue)),
      list.foldl (fun found result => found ++ outermostBorrows (result.shift offset))
          (start.map fun entry => (entry.1 + offset, entry.2.shift offset)) =
        (list.foldl (fun found result => found ++ outermostBorrows result) start).map
          fun entry => (entry.1 + offset, entry.2.shift offset) by
    simpa using general results.toList #[]
  intro list
  induction list with
  | nil => intro start; rfl
  | cons head tail ih =>
      intro start
      simp only [List.foldl_cons]
      rw [outermostBorrows_shift head, ← Array.map_append, ih]

theorem registerReturnedLoan_shift (lexical : Option Nat) (results : Array RuntimeValue)
    (frame : RuntimeFrame) :
    registerReturnedLoan lexical (results.map (·.shift offset)) (frame.shift offset) =
      (registerReturnedLoan lexical results frame).shift offset := by
  cases lexical with
  | none => rfl
  | some lexical =>
      unfold registerReturnedLoan
      simp only [returnedBorrows_shift, Array.getElem?_map]
      cases (results.foldl (init := #[]) fun found result =>
          found ++ outermostBorrows result)[0]? with
      | none => rfl
      | some entry =>
          simp only [Option.map_some, RuntimeFrame.shift]
          rw [Array.filter_map, Array.map_push]
          rfl

theorem packResults_nil : packResults #[] = .unit := rfl
theorem packResults_single (value : RuntimeValue) : packResults #[value] = value := rfl
theorem packResults_many (first second : RuntimeValue) (rest : List RuntimeValue) :
    packResults (first :: second :: rest).toArray = .tuple (first :: second :: rest).toArray := rfl

theorem packResults_shift (results : Array RuntimeValue) :
    packResults (results.map (·.shift offset)) = (packResults results).shift offset := by
  rcases results with ⟨_ | ⟨first, _ | ⟨second, rest⟩⟩⟩
  · rw [List.map_toArray, List.map_nil, packResults_nil, RuntimeValue.shift_unit]
  · rw [List.map_toArray, List.map_cons, List.map_nil, packResults_single, packResults_single]
  · rw [List.map_toArray, List.map_cons, List.map_cons, packResults_many, packResults_many,
      RuntimeValue.shift_tuple, List.map_toArray]
    rfl

theorem Array.shift_unshift_all {frontier : Nat} {values : Array RuntimeValue}
    (above : ∀ value ∈ values, value.Above frontier) :
    (values.map (·.unshift frontier)).map (·.shift frontier) = values := by
  rw [Array.map_map]
  conv => rhs; rw [← Array.map_id values]
  exact Array.map_congr_left fun value member =>
    RuntimeValue.shift_unshift frontier value (above value member)

theorem packResults_above {results : Array RuntimeValue}
    (above : ∀ value ∈ results, value.Above frontier) : (packResults results).Above frontier := by
  rw [← Array.shift_unshift_all above, packResults_shift]
  exact RuntimeValue.above_shift_self frontier _

theorem registerReturnedLoan_above {lexical : Option Nat} {results : Array RuntimeValue}
    {frame : RuntimeFrame} (frame_above : frame.Above frontier)
    (above : ∀ value ∈ results, value.Above frontier) :
    (registerReturnedLoan lexical results frame).Above frontier := by
  rw [← Array.shift_unshift_all above, ← RuntimeFrame.shift_unshift frame_above,
    registerReturnedLoan_shift]
  exact RuntimeFrame.above_shift_self frontier _

end Calls

/-! ## Data and closures -/

theorem constructNominal?_shift (unit : Validation.ValidatedUnit) (sourceNamespace : NamespaceId)
    (reference : QualifiedRef) (variant : Option String) (fields : Array RuntimeValue) :
    constructNominal? unit sourceNamespace reference variant (fields.map (·.shift offset)) =
      (constructNominal? unit sourceNamespace reference variant fields).map (·.shift offset) := by
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

theorem destructNominal?_shift (unit : Validation.ValidatedUnit) (sourceNamespace : NamespaceId)
    (reference : QualifiedRef) (variant : Option String) (value : RuntimeValue) :
    destructNominal? unit sourceNamespace reference variant (value.shift offset) =
      (destructNominal? unit sourceNamespace reference variant value).map
        (·.map (·.shift offset)) := by
  unfold destructNominal?
  cases value <;> simp only [RuntimeValue.shift_tuple, RuntimeValue.shift_unit,
    RuntimeValue.shift_bool, RuntimeValue.shift_character, RuntimeValue.shift_integer,
    RuntimeValue.shift_address, RuntimeValue.shift_signer, RuntimeValue.shift_string,
    RuntimeValue.shift_bytes, RuntimeValue.shift_vector, RuntimeValue.shift_nominal,
    RuntimeValue.shift_closure, RuntimeValue.shift_borrow, RuntimeValue.shift_loanHole] <;> try rfl
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

theorem toConst?_shift :
    ∀ (fuel : Nat) (value : RuntimeValue), sizeOf value ≤ fuel →
      RuntimeValue.toConst? (value.shift offset) = RuntimeValue.toConst? value
  | 0, value, small => by cases value <;> simp at small
  | fuel + 1, value, small => by
      have elements : ∀ (elements : Array RuntimeValue), sizeOf elements ≤ fuel + 1 →
          (elements.map (·.shift offset)).attach.mapM
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
        exact toConst?_shift fuel element (by simp at size this; omega)
      cases value with
      | vector values =>
          rw [RuntimeValue.shift_vector]
          simp only [RuntimeValue.toConst?]
          rw [elements values (by simp at small; omega)]
      | tuple values =>
          rw [RuntimeValue.shift_tuple]
          simp only [RuntimeValue.toConst?]
          rw [elements values (by simp at small; omega)]
      | nominal | closure | borrow | loanHole => simp [RuntimeValue.toConst?]
      | unit | bool _ | character _ | integer _ | address _ | signer _ | string _ | bytes _ =>
          simp [RuntimeValue.toConst?]

theorem evaluateProfileOperation?_shift {unit : Validation.ValidatedUnit}
    (executable : Validation.ExecutableUnit unit)
    (ns : Validation.ValidatedNamespace) (resultType : TypeId) (operation : ProfileValue)
    (arguments : Array RuntimeValue) :
    evaluateProfileOperation? executable ns resultType operation (arguments.map (·.shift offset)) =
      evaluateProfileOperation? executable ns resultType operation arguments := by
  unfold evaluateProfileOperation?
  have closed : (arguments.map (·.shift offset)).mapM RuntimeValue.toConst? =
      arguments.mapM RuntimeValue.toConst? := by
    rw [Array.mapM_eq_mapM_toList, Array.mapM_eq_mapM_toList, Array.toList_map, List.mapM_map]
    congr 1
    exact List.mapM_congr_of_mem fun value _ =>
      toConst?_shift (offset := offset) _ value (Nat.le_refl _)
  simp only [closed]

theorem constructNominal?_above {frontier : Nat} {unit : Validation.ValidatedUnit}
    {sourceNamespace : NamespaceId} {reference : QualifiedRef} {variant : Option String}
    {fields : Array RuntimeValue} {value : RuntimeValue}
    (above : ∀ field ∈ fields, field.Above frontier)
    (construct : constructNominal? unit sourceNamespace reference variant fields = some value) :
    value.Above frontier := by
  rw [← Array.shift_unshift_all above, constructNominal?_shift] at construct
  obtain ⟨base, -, rfl⟩ := Option.map_eq_some_iff.mp construct
  exact RuntimeValue.above_shift_self frontier base

theorem destructNominal?_above {frontier : Nat} {unit : Validation.ValidatedUnit}
    {sourceNamespace : NamespaceId} {reference : QualifiedRef} {variant : Option String}
    {value : RuntimeValue} {fields : Array RuntimeValue} (above : value.Above frontier)
    (destruct : destructNominal? unit sourceNamespace reference variant value = some fields) :
    ∀ field ∈ fields, field.Above frontier := by
  rw [← RuntimeValue.shift_unshift frontier value above, destructNominal?_shift] at destruct
  obtain ⟨base, -, rfl⟩ := Option.map_eq_some_iff.mp destruct
  intro field member
  obtain ⟨original, -, rfl⟩ := Array.mem_map.mp member
  exact RuntimeValue.above_shift_self frontier original

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
