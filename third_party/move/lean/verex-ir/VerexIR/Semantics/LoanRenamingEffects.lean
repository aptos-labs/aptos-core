-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Semantics.LoanRenamingOps

/-!
# Effects under loan renaming

Borrows, global storage, and place operations, run from a state mirroring
another, mirror its result (`evaluateGlobalOperation?_mirror`,
`evaluatePlaceOperation?_mirror`). A mutable borrow mints at each run's next
loan, so the minted identities differ by the offset of the frontiers.
-/

@[simp] theorem Array.isEmpty_map {α β : Type} (f : α → β) (xs : Array α) :
    (xs.map f).isEmpty = xs.isEmpty := by
  simp [Array.isEmpty]

namespace VerexIR

open SemanticOperations

section Effects

variable {ρ : Nat → Nat} {offset frontier inert inert' : Nat} {inside : Nat → Prop} {state state₂ : RuntimeState}

theorem readRuntimePlace?_within (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    {frame : RuntimeFrame} (frame_within : frame.Within inside) {place : RuntimePlace}
    {value : RuntimeValue} (read : readRuntimePlace? frame state place = some value) :
    value.Within inside := by
  obtain ⟨β, γ, base⟩ := shifted.renames.base
  have base_eq : state.globals =
      ({ state with globals := state.globals.rename γ } : RuntimeState).globals.rename β :=
    (base.rename_inverse_globals shifted.globalsWithin).symm
  have based := readRuntimePlace?_rename (ρ := β) (frame := frame.rename γ) base_eq place
  rw [base.rename_inverse_frame frame_within, read] at based
  obtain ⟨original, -, rfl⟩ := Option.map_eq_some_iff.mp based.symm
  exact base.within original

theorem borrowRuntimePlaceAt?_mirror (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    {lexical : Nat} {referenceType : ReferenceType} {kind : BorrowKind} {frame frame' : RuntimeFrame}
    (frame_within : frame.Within inside) {state' : RuntimeState} {place : RuntimePlace}
    {value : RuntimeValue}
    (borrow : borrowRuntimePlaceAt? lexical referenceType kind frame state place =
      some (frame', state', value)) :
    ∃ state₂', borrowRuntimePlaceAt? lexical referenceType kind (frame.rename ρ) state₂ place =
        some (frame'.rename ρ, state₂', value.rename ρ) ∧
      StateRenamed ρ offset frontier inside inert inert' state' state₂' ∧ frame'.Within inside ∧
      value.Within inside := by
  have read_rename := readRuntimePlace?_rename shifted.globals (frame := frame) place
  unfold borrowRuntimePlaceAt? at borrow ⊢
  cases kind with
  | profile _ => simp at borrow
  | immutable =>
      simp only [Option.bind_eq_bind, Option.bind_some] at borrow ⊢
      split at borrow
      · cases borrow
      rw [if_neg ‹_›, read_rename]
      obtain ⟨read, read_eq, borrow⟩ := Option.bind_eq_some_iff.mp borrow
      simp only [Option.some.injEq, Prod.mk.injEq] at borrow
      obtain ⟨rfl, rfl, rfl⟩ := borrow
      exact ⟨state₂, by rw [read_eq]; rfl, shifted, frame_within,
        readRuntimePlace?_within shifted frame_within read_eq⟩
  | mutable =>
      simp only [Option.bind_eq_bind, Option.bind_some] at borrow ⊢
      split at borrow
      · cases borrow
      rw [if_neg ‹_›, read_rename]
      obtain ⟨read, read_eq, borrow⟩ := Option.bind_eq_some_iff.mp borrow
      rw [read_eq, Option.map_some, Option.bind_some]
      obtain ⟨⟨writtenFrame, writtenState⟩, write_eq, borrow⟩ := Option.bind_eq_some_iff.mp borrow
      have minted : StateRenamed ρ offset frontier inside inert inert'
          { state with nextLoan := state.nextLoan + 1 }
          { state₂ with nextLoan := state₂.nextLoan + 1 } :=
        { shifted with
          nextLoan := by simp only [shifted.nextLoan]; omega
          frontier_le := Nat.le_succ_of_le shifted.frontier_le }
      obtain ⟨writtenState₂, write₂, writtenShifted, writtenAbove⟩ :=
        writeRuntimePlace?_mirror minted frame_within
          (value := .loanHole state.nextLoan) (by simpa using shifted.next_inside) write_eq
      rw [RuntimeValue.rename_loanHole, shifted.next] at write₂
      rw [write₂, Option.bind_some]
      simp only [Option.some.injEq, Prod.mk.injEq] at borrow
      obtain ⟨rfl, rfl, rfl⟩ := borrow
      have read_within := readRuntimePlace?_within shifted frame_within read_eq
      have frame_eq : ({ writtenFrame with
          activeLoans := (writtenFrame.activeLoans.filter (·.1 != ⟨lexical⟩)).push
            (⟨lexical⟩, state.nextLoan)
          loanLocations := writtenFrame.loanLocations.push (state.nextLoan, place) } :
            RuntimeFrame).rename ρ =
          { writtenFrame.rename ρ with
            activeLoans := ((writtenFrame.rename ρ).activeLoans.filter
              (·.1 != ⟨lexical⟩)).push (⟨lexical⟩, state₂.nextLoan)
            loanLocations := (writtenFrame.rename ρ).loanLocations.push
              (state₂.nextLoan, place) } := by
        rw [RuntimeFrame.rename_withLoans, ← shifted.next, RuntimeFrame.rename_activeLoans,
          RuntimeFrame.rename_loanLocations, Array.filter_map]
        simp [Function.comp_def]
      have frame_within' : ({ writtenFrame with
          activeLoans := (writtenFrame.activeLoans.filter (·.1 != ⟨lexical⟩)).push
            (⟨lexical⟩, state.nextLoan)
          loanLocations := writtenFrame.loanLocations.push (state.nextLoan, place) } :
            RuntimeFrame).Within inside :=
        { locals := writtenAbove.locals
          activeLoans := fun entry member => by
            rcases Array.mem_push.mp member with member | rfl
            · exact writtenAbove.activeLoans entry (Array.mem_filter.mp member).1
            · exact shifted.next_inside
          loanLocations := fun entry member => by
            rcases Array.mem_push.mp member with member | rfl
            · exact writtenAbove.loanLocations entry member
            · exact shifted.next_inside }
      have value_eq : (RuntimeValue.borrow state.nextLoan read).rename ρ =
          .borrow state₂.nextLoan (read.rename ρ) := by
        rw [RuntimeValue.rename_borrow, shifted.next]
      have value_within : (RuntimeValue.borrow state.nextLoan read).Within inside := by
        simpa using ⟨shifted.next_inside, read_within⟩
      rw [frame_eq, value_eq]
      cases root : place.root with
      | global key =>
          refine ⟨{ writtenState₂ with
              storageLoans := (state₂.nextLoan, .global key) :: writtenState₂.storageLoans }, rfl, ?_,
            frame_within', value_within⟩
          rw [← shifted.next]
          exact writtenShifted.withRegistry (writtenShifted.registry.cons shifted.next_inside (.global key))
      | «local» _ => exact ⟨writtenState₂, rfl, writtenShifted, frame_within', value_within⟩

theorem borrowRuntimePlace?_mirror (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    {unit : Validation.ValidatedUnit} {ns : Validation.ValidatedNamespace} {site : ExprId}
    {referenceType : ReferenceType} {kind : BorrowKind} {frame frame' : RuntimeFrame}
    (frame_within : frame.Within inside) {state' : RuntimeState} {place : RuntimePlace}
    {value : RuntimeValue}
    (borrow : borrowRuntimePlace? unit ns site referenceType kind frame state place =
      some (frame', state', value)) :
    ∃ state₂', borrowRuntimePlace? unit ns site referenceType kind (frame.rename ρ) state₂
        place = some (frame'.rename ρ, state₂', value.rename ρ) ∧
      StateRenamed ρ offset frontier inside inert inert' state' state₂' ∧ frame'.Within inside ∧
      value.Within inside := by
  cases kind with
  | immutable =>
      rw [borrowRuntimePlace?_immutable_at (lexical := 0)] at borrow ⊢
      exact borrowRuntimePlaceAt?_mirror shifted frame_within borrow
  | mutable =>
      cases certificate : certificateLoanId? unit ns.identity site with
      | none =>
          simp [borrowRuntimePlace?, certificate] at borrow
      | some lexical =>
          rw [borrowRuntimePlace?_mutable_at (lexical := lexical) (loan_eq := certificate)] at borrow ⊢
          exact borrowRuntimePlaceAt?_mirror shifted frame_within borrow
  | profile _ => simp [borrowRuntimePlace?] at borrow

theorem RuntimeValue.storageKey?_rename (ρ : Nat → Nat) (value : RuntimeValue) :
    (value.rename ρ).storageKey? = value.storageKey? := by
  cases value <;> simp [RuntimeValue.storageKey?]

theorem RuntimeValue.storageKey_rename (ρ : Nat → Nat) (value : RuntimeValue) :
    (value.rename ρ).storageKey = value.storageKey := by
  simp only [RuntimeValue.storageKey, RuntimeValue.storageKey?_rename]

theorem globalKey_rename (namespaceId : NamespaceId) (typeId : TypeId) (key : RuntimeValue) :
    globalKey namespaceId typeId (key.rename ρ) = globalKey namespaceId typeId key := by
  simp only [globalKey, RuntimeValue.storageKey_rename]

theorem globalValue?_rename (globals_eq : state₂.globals = state.globals.rename ρ)
    (namespaceId : NamespaceId) (typeId : TypeId) (key : RuntimeValue) :
    globalValue? state₂ namespaceId typeId (key.rename ρ) =
      (globalValue? state namespaceId typeId key).map (·.rename ρ) := by
  simp only [globalValue?, globalKey_rename, globals_eq, GlobalMap.lookup_rename]

theorem globalExists_rename (globals_eq : state₂.globals = state.globals.rename ρ)
    (namespaceId : NamespaceId) (typeId : TypeId) (key : RuntimeValue) :
    globalExists state₂ namespaceId typeId (key.rename ρ) =
      globalExists state namespaceId typeId key := by
  simp only [globalExists, globalValue?_rename globals_eq, Option.isSome_map]

theorem GlobalMap.erase_within {globals : GlobalMap} {key : GlobalKey}
    (above : ∀ slot ∈ globals.entries, slot.value.Within inside) :
    ∀ slot ∈ (globals.erase key).entries, slot.value.Within inside := by
  intro slot member
  unfold GlobalMap.erase at member
  exact above slot (Array.mem_filter.mp member).1

theorem updateLocalBorrowValue?_rename (injective : Function.Injective ρ)
    (globals_eq : state₂.globals = state.globals.rename ρ)
    (frame : RuntimeFrame) (loan : Nat) (replacement : RuntimeValue) :
    updateLocalBorrowValue? (frame.rename ρ) state₂ (ρ loan)
        (replacement.rename ρ) =
      (updateLocalBorrowValue? frame state loan replacement).map fun written =>
        (written.1.rename ρ, { state₂ with globals := written.2.globals.rename ρ }) := by
  have node := fun value => borrowRewrite?_rename injective loan replacement value
  unfold updateLocalBorrowValue?
  simp only [Option.bind_eq_bind, localLoanPlace?_rename injective]
  cases localLoanPlace? frame loan with
  | none => rfl
  | some place =>
      simp only [Option.bind_some, readRuntimePlace?_rename globals_eq]
      cases readRuntimePlace? frame state place with
      | none => rfl
      | some value =>
          simp only [Option.map_some, Option.bind_some, rewriteFirst_rename node]
          cases rewriteFirst (borrowRewrite? loan replacement) value with
          | none => rfl
          | some updated =>
              simp only [Option.map_some, Option.bind_some]
              exact writeRuntimePlace?_rename globals_eq place updated

theorem updateBorrowValue?_rename (injective : Function.Injective ρ)
    (globals_eq : state₂.globals = state.globals.rename ρ)
    (frame : RuntimeFrame) (loan : Nat) (replacement : RuntimeValue) :
    updateBorrowValue? (frame.rename ρ) state₂ (ρ loan) (replacement.rename ρ) =
      (updateBorrowValue? frame state loan replacement).map fun written =>
        (written.1.rename ρ, { state₂ with globals := written.2.globals.rename ρ }) := by
  have node := fun value => borrowRewrite?_rename injective loan replacement value
  unfold updateBorrowValue?
  rw [updateLocalBorrowValue?_rename injective globals_eq]
  cases updateLocalBorrowValue? frame state loan replacement with
  | some written => rfl
  | none =>
      simp only [Option.map_none]
      have locals : ((fun slot : Option RuntimeValue =>
            (slot.bind fun value =>
              rewriteFirst (borrowRewrite? (ρ loan) (replacement.rename ρ)) value).isSome) ∘
              Option.map fun value => value.rename ρ) =
          fun slot => (slot.bind fun value =>
            rewriteFirst (borrowRewrite? loan replacement) value).isSome := by
        funext slot; cases slot <;> simp [rewriteFirst_rename node]
      rw [RuntimeFrame.rename_locals, Array.toList_map, indexOfFrom_map, locals]
      cases indexOfFrom (fun slot => (slot.bind fun value =>
          rewriteFirst (borrowRewrite? loan replacement) value).isSome) frame.locals.toList 0 with
      | some index =>
          simp only [Array.getElem?_map]
          cases frame.locals[index]? with
          | none => rfl
          | some slot =>
              cases slot with
              | none => rfl
              | some value =>
                  simp only [Option.map_some, Option.bind_some, rewriteFirst_rename node]
                  cases rewriteFirst (borrowRewrite? loan replacement) value with
                  | none => rfl
                  | some updated =>
                      simp only [Option.map_some, ← globals_eq]
                      simp [RuntimeFrame.rename, Array.map_setIfInBounds]
      | none =>
          have globals : ((fun slot : GlobalSlot =>
                (rewriteFirst (borrowRewrite? (ρ loan) (replacement.rename ρ))
                  slot.value).isSome) ∘
                  fun slot : GlobalSlot => { slot with value := slot.value.rename ρ }) =
              fun slot => (rewriteFirst (borrowRewrite? loan replacement) slot.value).isSome := by
            funext slot; simp [rewriteFirst_rename node]
          rw [globals_eq, GlobalMap.rename_entries, Array.toList_map, indexOfFrom_map, globals]
          cases indexOfFrom (fun slot : GlobalSlot =>
              (rewriteFirst (borrowRewrite? loan replacement) slot.value).isSome)
              state.globals.entries.toList 0 with
          | none => rfl
          | some index =>
              simp only [Array.getElem?_map]
              cases state.globals.entries[index]? with
              | none => rfl
              | some slot =>
                  simp only [Option.map_some, Option.bind_some, rewriteFirst_rename node]
                  cases rewriteFirst (borrowRewrite? loan replacement) slot.value with
                  | none => rfl
                  | some updated =>
                      simp [GlobalMap.rename, Array.set!_eq_setIfInBounds, Array.map_setIfInBounds]

theorem updateBorrowValue?_globals {frame frame' : RuntimeFrame} {state state' : RuntimeState}
    {loan : Nat} {replacement : RuntimeValue}
    (update : updateBorrowValue? frame state loan replacement = some (frame', state')) :
    state' = { state with globals := state'.globals } := by
  unfold updateBorrowValue? at update
  split at update
  · next written local_eq =>
      simp only [Option.some.injEq] at update
      subst update
      unfold updateLocalBorrowValue? at local_eq
      simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at local_eq
      obtain ⟨_, -, _, -, _, -, write⟩ := local_eq
      exact writeRuntimePlace?_globals write
  · simp only at update
    split at update
    · obtain ⟨_, -, update⟩ := Option.map_eq_some_iff.mp update
      simp only [Prod.mk.injEq] at update
      obtain ⟨-, rfl⟩ := update
      rfl
    · split at update
      · obtain ⟨_, -, update⟩ := Option.bind_eq_some_iff.mp update
        obtain ⟨_, -, update⟩ := Option.map_eq_some_iff.mp update
        simp only [Prod.mk.injEq] at update
        obtain ⟨-, rfl⟩ := update
        rfl
      · cases update

theorem updateBorrowValue?_mirror (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    {frame frame' : RuntimeFrame} (frame_within : frame.Within inside) {state' : RuntimeState}
    {loan : Nat} (within : inside loan) {replacement : RuntimeValue}
    (replacement_within : replacement.Within inside)
    (update : updateBorrowValue? frame state loan replacement = some (frame', state')) :
    ∃ state₂', updateBorrowValue? (frame.rename ρ) state₂ (ρ loan)
        (replacement.rename ρ) = some (frame'.rename ρ, state₂') ∧
      StateRenamed ρ offset frontier inside inert inert' state' state₂' ∧ frame'.Within inside := by
  obtain ⟨β, γ, base⟩ := shifted.renames.base
  have base_eq : state.globals =
      ({ state with globals := state.globals.rename γ } : RuntimeState).globals.rename β :=
    (base.rename_inverse_globals shifted.globalsWithin).symm
  have based := updateBorrowValue?_rename base.injective base_eq (frame.rename γ)
    (γ loan) (replacement.rename γ)
  rw [base.rename_inverse_frame frame_within, base.inverse loan within,
    base.rename_inverse replacement replacement_within, update] at based
  obtain ⟨written, -, written_eq⟩ := Option.map_eq_some_iff.mp based.symm
  simp only [Prod.mk.injEq] at written_eq
  obtain ⟨frame_eq, state_eq⟩ := written_eq
  have globals_within : ∀ slot ∈ state'.globals.entries, slot.value.Within inside := by
    rw [← state_eq]
    exact base.within_globals _
  refine ⟨{ state₂ with globals := state'.globals.rename ρ }, ?_, ?_, ?_⟩
  · rw [updateBorrowValue?_rename shifted.renames.injective shifted.globals, update]
    rfl
  · rw [updateBorrowValue?_globals update]
    exact shifted.withGlobals _ globals_within
  · rw [← frame_eq]
    exact base.within_frame _

theorem selectNominalVariantFieldAt?_rename (ρ : Nat → Nat) (owner : StructHandle)
    (choices : Array (String × Nat)) (arguments : Array RuntimeValue) :
    selectNominalVariantFieldAt? owner choices (arguments.map (·.rename ρ)) =
      (selectNominalVariantFieldAt? owner choices arguments).map (·.rename ρ) := by
  unfold selectNominalVariantFieldAt?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, rest⟩⟩
  all_goals try cases a
  all_goals try simp
  rename_i source variant fields
  cases variant <;> simp
  split <;> simp [Function.comp_def]

theorem evaluateDataOperation?_rename (ρ : Nat → Nat) (unit : Validation.ValidatedUnit)
    (sourceNamespace : NamespaceId) (operation : DataOperation) (arguments : Array RuntimeValue) :
    evaluateDataOperation? unit sourceNamespace operation (arguments.map (·.rename ρ)) =
      (evaluateDataOperation? unit sourceNamespace operation arguments).map (·.rename ρ) := by
  unfold evaluateDataOperation?
  rw [Array.toList_map]
  cases operation with
  | selectVariants reference fields =>
      simp [Function.comp_def, selectNominalVariantFieldAt?_rename]
  | _ =>
      rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
      all_goals try (cases a)
      all_goals try (simp; done)
      all_goals try (rename_i variant _; cases variant)
      all_goals simp [Function.comp_def]
      all_goals refine Option.bind_congr fun _ _ => ?_
      all_goals split <;> simp [Function.comp_def]
      all_goals refine Option.bind_congr fun _ _ => ?_
      all_goals split <;> simp

theorem evaluateDataOperation?_within {β γ : Nat → Nat} (base : LoanBase β γ inside)
    {unit : Validation.ValidatedUnit}
    {sourceNamespace : NamespaceId} {operation : DataOperation} {arguments : Array RuntimeValue}
    {value : RuntimeValue} (above : ∀ argument ∈ arguments, argument.Within inside)
    (evaluate : evaluateDataOperation? unit sourceNamespace operation arguments = some value) :
    value.Within inside := by
  rw [← Array.rename_inverse_all base above, evaluateDataOperation?_rename] at evaluate
  obtain ⟨original, -, rfl⟩ := Option.map_eq_some_iff.mp evaluate
  exact base.within original

theorem RuntimeFrame.Within.clearLocal {frame : RuntimeFrame} (above : frame.Within inside)
    (index : Nat) :
    ({ frame with locals := frame.locals.set! index none } : RuntimeFrame).Within inside where
  locals slot member stored stored_eq := by
    simp only [Array.set!_eq_setIfInBounds] at member
    rcases Array.mem_or_eq_of_mem_setIfInBounds member with member | rfl
    · exact above.locals slot member stored stored_eq
    · cases stored_eq
  activeLoans := above.activeLoans
  loanLocations := above.loanLocations

theorem evaluatePlaceOperation?_mirror
    (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    {unit : Validation.ValidatedUnit} {ns : Validation.ValidatedNamespace} {resultType : TypeId}
    {site : ExprId} {operation : Operation} {arguments : Array RuntimeValue}
    {frame frame' : RuntimeFrame} {state' : RuntimeState} {value : RuntimeValue}
    (frame_within : frame.Within inside)
    (arguments_within : ∀ argument ∈ arguments, argument.Within inside)
    (evaluate : evaluatePlaceOperation? unit ns resultType site operation arguments frame state =
      some (frame', state', value)) :
    ∃ state₂', evaluatePlaceOperation? unit ns resultType site operation
        (arguments.map (·.rename ρ)) (frame.rename ρ) state₂ =
        some (frame'.rename ρ, state₂', value.rename ρ) ∧
      StateRenamed ρ offset frontier inside inert inert' state' state₂' ∧ frame'.Within inside ∧
      value.Within inside := by
  have resolve := fun place => resolvePlace?_rename unit ns (frame := frame) shifted.globals place
  have read := readRuntimePlace?_rename shifted.globals (frame := frame)
  cases operation with
  | copy place =>
      simp only [evaluatePlaceOperation?, Array.isEmpty_map, Option.bind_eq_bind] at evaluate ⊢
      split at evaluate
      · cases evaluate
      rw [if_neg ‹_›, resolve]
      obtain ⟨resolved, resolved_eq, evaluate⟩ := Option.bind_eq_some_iff.mp evaluate
      obtain ⟨current, current_eq, evaluate⟩ := Option.bind_eq_some_iff.mp evaluate
      simp only [Option.some.injEq, Prod.mk.injEq] at evaluate
      obtain ⟨rfl, rfl, rfl⟩ := evaluate
      exact ⟨state₂, by simp [resolved_eq, read, current_eq], shifted, frame_within,
        readRuntimePlace?_within shifted frame_within current_eq⟩
  | read place =>
      simp only [evaluatePlaceOperation?, Array.isEmpty_map, Option.bind_eq_bind] at evaluate ⊢
      split at evaluate
      · cases evaluate
      rw [if_neg ‹_›, resolve]
      obtain ⟨resolved, resolved_eq, evaluate⟩ := Option.bind_eq_some_iff.mp evaluate
      obtain ⟨current, current_eq, evaluate⟩ := Option.bind_eq_some_iff.mp evaluate
      simp only [Option.some.injEq, Prod.mk.injEq] at evaluate
      obtain ⟨rfl, rfl, rfl⟩ := evaluate
      exact ⟨state₂, by simp [resolved_eq, read, current_eq], shifted, frame_within,
        readRuntimePlace?_within shifted frame_within current_eq⟩
  | move place =>
      simp only [evaluatePlaceOperation?, Array.isEmpty_map, Option.bind_eq_bind] at evaluate ⊢
      split at evaluate
      · cases evaluate
      rw [if_neg ‹_›, resolve]
      obtain ⟨resolved, resolved_eq, evaluate⟩ := Option.bind_eq_some_iff.mp evaluate
      obtain ⟨current, current_eq, evaluate⟩ := Option.bind_eq_some_iff.mp evaluate
      simp only [resolved_eq, Option.bind_some, read, current_eq, Option.map_some]
      have current_within := readRuntimePlace?_within shifted frame_within current_eq
      split at evaluate
      · next localId root_eq =>
          split at evaluate
          · simp only [Option.some.injEq, Prod.mk.injEq] at evaluate
            obtain ⟨rfl, rfl, rfl⟩ := evaluate
            rw [if_pos ‹_›]
            exact ⟨state₂, by simp [RuntimeFrame.rename, Array.map_setIfInBounds], shifted,
              frame_within.clearLocal _, current_within⟩
          · simp only [Option.some.injEq, Prod.mk.injEq] at evaluate
            obtain ⟨rfl, rfl, rfl⟩ := evaluate
            rw [if_neg ‹_›]
            exact ⟨state₂, rfl, shifted, frame_within, current_within⟩
      · cases evaluate
  | drop place =>
      simp only [evaluatePlaceOperation?, Array.isEmpty_map, Option.bind_eq_bind] at evaluate ⊢
      split at evaluate
      · cases evaluate
      rw [if_neg ‹_›, resolve]
      obtain ⟨resolved, resolved_eq, evaluate⟩ := Option.bind_eq_some_iff.mp evaluate
      simp only [resolved_eq, Option.bind_some]
      split at evaluate
      · next localId root_eq =>
          split at evaluate
          · simp only [Option.some.injEq, Prod.mk.injEq] at evaluate
            obtain ⟨rfl, rfl, rfl⟩ := evaluate
            rw [if_pos ‹_›]
            exact ⟨state₂, by simp [RuntimeFrame.rename, Array.map_setIfInBounds], shifted,
              frame_within.clearLocal _, by simp⟩
          · simp only [Option.some.injEq, Prod.mk.injEq] at evaluate
            obtain ⟨rfl, rfl, rfl⟩ := evaluate
            rw [if_neg ‹_›]
            exact ⟨state₂, by simp, shifted, frame_within, by simp⟩
      · cases evaluate
  | borrow kind place =>
      simp only [evaluatePlaceOperation?, Array.isEmpty_map, Option.bind_eq_bind] at evaluate ⊢
      split at evaluate
      · cases evaluate
      rw [if_neg ‹_›]
      obtain ⟨type, type_eq, evaluate⟩ := Option.bind_eq_some_iff.mp evaluate
      rw [type_eq, Option.bind_some]
      cases type with
      | reference referenceType =>
          simp only [resolve]
          obtain ⟨resolved, resolved_eq, evaluate⟩ := Option.bind_eq_some_iff.mp evaluate
          rw [resolved_eq, Option.bind_some]
          exact borrowRuntimePlace?_mirror shifted frame_within evaluate
      | _ => cases evaluate
  | write place =>
      obtain ⟨arguments⟩ := arguments
      rcases arguments with _ | ⟨written, _ | ⟨second, rest⟩⟩
      · cases evaluate
      · simp only [List.map_toArray, List.map_cons, List.map_nil]
        rw [evaluatePlaceOperation?_write] at evaluate ⊢
        rw [resolve]
        obtain ⟨resolved, resolved_eq, evaluate⟩ := Option.bind_eq_some_iff.mp evaluate
        obtain ⟨⟨writtenFrame, writtenState⟩, write_eq, evaluate⟩ :=
          Option.bind_eq_some_iff.mp evaluate
        simp only [Option.some.injEq, Prod.mk.injEq] at evaluate
        obtain ⟨rfl, rfl, rfl⟩ := evaluate
        obtain ⟨state₂', write₂, writtenShifted, writtenAbove⟩ :=
          writeRuntimePlace?_mirror shifted frame_within (arguments_within written (by simp))
            write_eq
        exact ⟨state₂', by simp [resolved_eq, write₂], writtenShifted, writtenAbove,
          by simp⟩
      · cases evaluate
  | reference kind =>
      cases kind with
      | borrow _ => cases evaluate
      | dereference =>
          obtain ⟨arguments⟩ := arguments
          rcases arguments with _ | ⟨operand, _ | ⟨second, rest⟩⟩
          · unfold evaluatePlaceOperation? at evaluate
            dsimp only at evaluate
            split at evaluate
            · cases evaluate
            · simp [dereferenceBorrow?] at evaluate
          · have operand_within := arguments_within operand (by simp)
            simp only [List.map_toArray, List.map_cons, List.map_nil]
            rw [evaluatePlaceOperation?_dereference] at evaluate ⊢
            split at evaluate
            · simp only [Option.some.injEq, Prod.mk.injEq] at evaluate
              obtain ⟨rfl, rfl, rfl⟩ := evaluate
              rw [if_pos ‹_›]
              exact ⟨state₂, rfl, shifted, frame_within, operand_within⟩
            · rw [if_neg ‹_›]
              cases operand with
              | borrow loan current =>
                  simp only [dereferenceBorrow?, Option.some.injEq,
                    Prod.mk.injEq] at evaluate
                  obtain ⟨rfl, rfl, rfl⟩ := evaluate
                  simp only [RuntimeValue.within_borrow] at operand_within
                  exact ⟨state₂, by simp [dereferenceBorrow?], shifted, frame_within,
                    operand_within.2⟩
              | _ => simp [dereferenceBorrow?] at evaluate
          · unfold evaluatePlaceOperation? at evaluate
            dsimp only at evaluate
            split at evaluate
            · cases evaluate
            · simp [dereferenceBorrow?] at evaluate
      | freeze explicit =>
          obtain ⟨arguments⟩ := arguments
          rcases arguments with _ | ⟨operand, _ | ⟨second, rest⟩⟩
          · unfold evaluatePlaceOperation? at evaluate
            dsimp only at evaluate
            split at evaluate
            · cases evaluate
            · simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at evaluate
              obtain ⟨type, -, evaluate⟩ := evaluate
              cases type <;> cases evaluate
          · have operand_within := arguments_within operand (by simp)
            simp only [List.map_toArray, List.map_cons, List.map_nil]
            rw [evaluatePlaceOperation?_freeze] at evaluate ⊢
            split at evaluate
            · simp only [Option.some.injEq, Prod.mk.injEq] at evaluate
              obtain ⟨rfl, rfl, rfl⟩ := evaluate
              rw [if_pos ‹_›]
              exact ⟨state₂, rfl, shifted, frame_within, operand_within⟩
            · rw [if_neg ‹_›]
              obtain ⟨type, type_eq, evaluate⟩ := Option.bind_eq_some_iff.mp evaluate
              rw [type_eq, Option.bind_some]
              cases type with
              | reference resultReference =>
                  simp only at evaluate ⊢
                  rw [freezeBorrow?_single] at evaluate ⊢
                  cases operand with
                  | borrow loan current =>
                      simp only [RuntimeValue.rename_borrow] at evaluate ⊢
                      split at evaluate
                      · cases evaluate
                      · simp only [Option.some.injEq, Prod.mk.injEq] at evaluate
                        obtain ⟨rfl, rfl, rfl⟩ := evaluate
                        rw [if_neg ‹_›]
                        simp only [RuntimeValue.within_borrow] at operand_within
                        exact ⟨state₂, rfl, shifted, frame_within, operand_within.2⟩
                  | _ => cases evaluate
              | _ => cases evaluate
          · unfold evaluatePlaceOperation? at evaluate
            dsimp only at evaluate
            split at evaluate
            · cases evaluate
            · simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at evaluate
              obtain ⟨type, -, evaluate⟩ := evaluate
              cases type <;> cases evaluate
      | mutate =>
          rw [evaluatePlaceOperation?_mutate] at evaluate ⊢
          unfold mutateBorrow? at evaluate ⊢
          rw [Array.toList_map]
          have member : ∀ value ∈ arguments.toList, value.Within inside :=
            fun value member => arguments_within value (Array.mem_toList_iff.mp member)
          generalize arguments.toList = list at evaluate member ⊢
          rcases list with _ | ⟨target, _ | ⟨replacement, _ | ⟨third, rest⟩⟩⟩
          · cases evaluate
          · cases target <;> cases evaluate
          · cases target with
            | borrow loan current =>
                have target_within := member _ List.mem_cons_self
                simp only [RuntimeValue.within_borrow] at target_within
                have replacement_within := member replacement (by simp)
                simp only [List.map_cons, List.map_nil, RuntimeValue.rename_borrow]
                cases update : updateBorrowValue? frame state loan replacement with
                | some written =>
                    obtain ⟨writtenFrame, writtenState⟩ := written
                    simp only [update, Option.some.injEq, Prod.mk.injEq] at evaluate
                    obtain ⟨rfl, rfl, rfl⟩ := evaluate
                    obtain ⟨state₂', update₂, writtenShifted, writtenAbove⟩ :=
                      updateBorrowValue?_mirror shifted frame_within target_within.1
                        replacement_within update
                    simp only [update₂]
                    exact ⟨state₂', by simp, writtenShifted, writtenAbove,
                      by simp⟩
                | none =>
                    have update₂ := updateBorrowValue?_rename shifted.renames.injective
                      shifted.globals frame loan replacement
                    rw [update, Option.map_none] at update₂
                    simp only [update, Option.some.injEq, Prod.mk.injEq] at evaluate
                    obtain ⟨rfl, rfl, rfl⟩ := evaluate
                    obtain ⟨state₂', back₂, backShifted, backAbove⟩ :=
                      applyWriteBack_rename shifted frame_within target_within.1 replacement_within
                    simp only [update₂, back₂]
                    exact ⟨state₂', by simp, backShifted, backAbove, by simp⟩
            | _ => cases evaluate
          · cases target <;> cases evaluate
  | data operation =>
      simp only [evaluatePlaceOperation?, Option.bind_eq_bind] at evaluate ⊢
      obtain ⟨result, result_eq, evaluate⟩ := Option.bind_eq_some_iff.mp evaluate
      simp only [Option.some.injEq, Prod.mk.injEq] at evaluate
      obtain ⟨rfl, rfl, rfl⟩ := evaluate
      obtain ⟨β, γ, base⟩ := shifted.renames.base
      exact ⟨state₂, by simp [evaluateDataOperation?_rename, result_eq], shifted, frame_within,
        evaluateDataOperation?_within base arguments_within result_eq⟩
  | call _ | global _ | primitive _ | specification _ | assert | profile _ _ => cases evaluate

/-- Two results of a global operation mirror each other. -/
def SemanticOperations.GlobalOperationResult.Mirrors (ρ : Nat → Nat) (offset frontier : Nat)
    (inside : Nat → Prop) (inert inert' : Nat) :
    GlobalOperationResult → GlobalOperationResult → Prop
  | .value frame state result, .value frame₂ state₂ result₂ =>
      VerexIR.Mirrors ρ offset frontier inside inert inert' (frame, state) (frame₂, state₂) ∧
        result₂ = result.rename ρ ∧ result.Within inside
  | .throw_ frame state kind values, .throw_ frame₂ state₂ kind₂ values₂ =>
      VerexIR.Mirrors ρ offset frontier inside inert inert' (frame, state) (frame₂, state₂) ∧
        kind₂ = kind ∧ values₂ = values.map (·.rename ρ) ∧
        ∀ thrown ∈ values, thrown.Within inside
  | _, _ => False

theorem evaluateGlobalOperation?_mirror
    (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    {unit : Validation.ValidatedUnit} {ns : Validation.ValidatedNamespace} {resultType : TypeId}
    {site : ExprId} {kind : GlobalKind} {instantiations : Array GenericArgument}
    {arguments : Array RuntimeValue} {frame : RuntimeFrame} {result : GlobalOperationResult}
    (frame_within : frame.Within inside)
    (arguments_within : ∀ argument ∈ arguments, argument.Within inside)
    (evaluate : evaluateGlobalOperation? unit ns resultType site kind instantiations arguments
      frame state = some result) :
    ∃ result₂, evaluateGlobalOperation? unit ns resultType site kind instantiations
        (arguments.map (·.rename ρ)) (frame.rename ρ) state₂ = some result₂ ∧
      result.Mirrors ρ offset frontier inside inert inert' result₂ := by
  obtain ⟨generics⟩ := instantiations
  rcases generics with _ | ⟨generic, _ | ⟨second, rest⟩⟩
  · cases evaluate
  · cases generic with
    | typeArg resource =>
        rw [evaluateGlobalOperation?_typeArg] at evaluate ⊢
        simp only [RuntimeFrame.rename_typeInstantiation, Array.toList_map] at evaluate ⊢
        generalize instantiatedTypeId frame.typeInstantiation resource.typeId = resourceType
          at evaluate ⊢
        have member : ∀ value ∈ arguments.toList, value.Within inside :=
          fun value member => arguments_within value (Array.mem_toList_iff.mp member)
        generalize arguments.toList = list at evaluate member ⊢
        cases kind with
        | contains =>
            rcases list with _ | ⟨key, _ | ⟨second, rest⟩⟩
            · cases evaluate
            · simp only [List.map_cons, List.map_nil, Option.bind_eq_bind,
                Option.bind_eq_some_iff, Option.some.injEq] at evaluate ⊢
              obtain ⟨storage, storage_eq, rfl⟩ := evaluate
              refine ⟨_, ⟨storage, by rw [RuntimeValue.storageKey?_rename, storage_eq], rfl⟩, ?_⟩
              refine ⟨⟨rfl, shifted, frame_within⟩, ?_, by simp⟩
              rw [globalExists_rename shifted.globals, RuntimeValue.rename_bool]
            · cases evaluate
        | borrow borrowKind =>
            rcases list with _ | ⟨key, _ | ⟨second, rest⟩⟩
            · cases evaluate
            · simp only [List.map_cons, List.map_nil, Option.bind_eq_bind,
                Option.bind_eq_some_iff] at evaluate ⊢
              obtain ⟨storage, storage_eq, evaluate⟩ := evaluate
              simp only [RuntimeValue.storageKey?_rename, storage_eq, Option.some.injEq,
                exists_eq_left', globalValue?_rename shifted.globals, globalKey_rename]
              cases lookup : globalValue? state ns.identity resourceType key with
              | none =>
                  rw [lookup, Option.some.injEq] at evaluate
                  subst evaluate
                  exact ⟨_, rfl, ⟨rfl, shifted, frame_within⟩, rfl, by simp, by simp⟩
              | some current =>
                  simp only [lookup, Option.bind_eq_some_iff] at evaluate
                  simp only [Option.map_some]
                  obtain ⟨type, type_eq, evaluate⟩ := evaluate
                  rw [type_eq, Option.bind_some]
                  cases type with
                  | reference referenceType =>
                      simp only [Option.bind_eq_some_iff, Option.some.injEq] at evaluate
                      obtain ⟨⟨frame', state', value⟩, borrow_eq, rfl⟩ := evaluate
                      obtain ⟨state₂', borrow₂, shifted', frame_within', value_within⟩ :=
                        borrowRuntimePlace?_mirror shifted frame_within borrow_eq
                      refine ⟨.value (frame'.rename ρ) state₂' (value.rename ρ), ?_,
                        ⟨rfl, shifted', frame_within'⟩, rfl, value_within⟩
                      simp [borrow₂]
                  | _ => cases evaluate
            · cases evaluate
        | take =>
            rcases list with _ | ⟨key, _ | ⟨second, rest⟩⟩
            · cases evaluate
            · simp only [List.map_cons, List.map_nil, Option.bind_eq_bind,
                Option.bind_eq_some_iff] at evaluate ⊢
              obtain ⟨storage, storage_eq, evaluate⟩ := evaluate
              simp only [RuntimeValue.storageKey?_rename, storage_eq, Option.some.injEq,
                exists_eq_left', globalValue?_rename shifted.globals, globalKey_rename]
              cases lookup : globalValue? state ns.identity resourceType key with
              | none =>
                  rw [lookup, Option.some.injEq] at evaluate
                  subst evaluate
                  exact ⟨_, rfl, ⟨rfl, shifted, frame_within⟩, rfl, by simp, by simp⟩
              | some current =>
                  simp only [lookup, Option.some.injEq] at evaluate
                  subst evaluate
                  refine ⟨_, rfl, ⟨rfl, ?_, frame_within⟩, rfl, shifted.slot_within lookup⟩
                  have erased := shifted.withGlobals
                    (state.globals.erase (globalKey ns.identity resourceType key))
                    (GlobalMap.erase_within shifted.globalsWithin)
                  rwa [GlobalMap.erase_rename, ← shifted.globals] at erased
            · cases evaluate
        | publish =>
            rcases list with _ | ⟨key, _ | ⟨value, _ | ⟨third, rest⟩⟩⟩
            · cases evaluate
            · cases evaluate
            · simp only [List.map_cons, List.map_nil, Option.bind_eq_bind,
                Option.bind_eq_some_iff] at evaluate ⊢
              obtain ⟨storage, storage_eq, evaluate⟩ := evaluate
              simp only [RuntimeValue.storageKey?_rename, storage_eq, Option.some.injEq,
                exists_eq_left', globalValue?_rename shifted.globals, globalKey_rename]
              cases lookup : globalValue? state ns.identity resourceType key with
              | some current =>
                  rw [lookup, Option.some.injEq] at evaluate
                  subst evaluate
                  exact ⟨_, rfl, ⟨rfl, shifted, frame_within⟩, rfl, by simp, by simp⟩
              | none =>
                  simp only [lookup, Option.some.injEq] at evaluate
                  subst evaluate
                  refine ⟨_, rfl, ⟨rfl, ?_, frame_within⟩, by simp, by simp⟩
                  have inserted := shifted.withGlobals
                    (state.globals.insert (globalKey ns.identity resourceType key) value)
                    (GlobalMap.insert_within shifted.globalsWithin
                      (member value (by simp)))
                  rwa [GlobalMap.insert_rename, ← shifted.globals] at inserted
            · cases evaluate
    | const _ => cases evaluate
    | lifetime _ => cases evaluate
    | evidence _ => cases evaluate
  · cases evaluate

end Effects

end VerexIR
