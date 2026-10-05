-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.LoanRenamingOps

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

namespace LeanerIR

open SemanticOperations

section Effects

variable {offset frontier inert inert' : Nat} {state state₂ : RuntimeState}

theorem readRuntimePlace?_above (shifted : StateShifted offset frontier inert inert' state state₂)
    {frame : RuntimeFrame} (frame_above : frame.Above frontier) {place : RuntimePlace}
    {value : RuntimeValue} (read : readRuntimePlace? frame state place = some value) :
    value.Above frontier := by
  have base_eq : state.globals =
      ({ state with globals := state.globals.unshift frontier } : RuntimeState).globals.shift
        frontier := (GlobalMap.shift_unshift shifted.globalsAbove).symm
  have base := readRuntimePlace?_shift (offset := frontier) (frame := frame.unshift frontier)
    base_eq place
  rw [RuntimeFrame.shift_unshift frame_above, read] at base
  obtain ⟨original, -, rfl⟩ := Option.map_eq_some_iff.mp base.symm
  exact RuntimeValue.above_shift_self frontier original

theorem borrowRuntimePlaceAt?_mirror (shifted : StateShifted offset frontier inert inert' state state₂)
    {lexical : Nat} {referenceType : ReferenceType} {kind : BorrowKind} {frame frame' : RuntimeFrame}
    (frame_above : frame.Above frontier) {state' : RuntimeState} {place : RuntimePlace}
    {value : RuntimeValue}
    (borrow : borrowRuntimePlaceAt? lexical referenceType kind frame state place =
      some (frame', state', value)) :
    ∃ state₂', borrowRuntimePlaceAt? lexical referenceType kind (frame.shift offset) state₂ place =
        some (frame'.shift offset, state₂', value.shift offset) ∧
      StateShifted offset frontier inert inert' state' state₂' ∧ frame'.Above frontier ∧
      value.Above frontier := by
  have read_shift := readRuntimePlace?_shift shifted.globals (frame := frame) place
  unfold borrowRuntimePlaceAt? at borrow ⊢
  cases kind with
  | profile _ => simp at borrow
  | immutable =>
      simp only [Option.bind_eq_bind, Option.bind_some] at borrow ⊢
      split at borrow
      · cases borrow
      rw [if_neg ‹_›, read_shift]
      obtain ⟨read, read_eq, borrow⟩ := Option.bind_eq_some_iff.mp borrow
      simp only [Option.some.injEq, Prod.mk.injEq] at borrow
      obtain ⟨rfl, rfl, rfl⟩ := borrow
      exact ⟨state₂, by rw [read_eq]; rfl, shifted, frame_above,
        readRuntimePlace?_above shifted frame_above read_eq⟩
  | mutable =>
      simp only [Option.bind_eq_bind, Option.bind_some] at borrow ⊢
      split at borrow
      · cases borrow
      rw [if_neg ‹_›, read_shift]
      obtain ⟨read, read_eq, borrow⟩ := Option.bind_eq_some_iff.mp borrow
      rw [read_eq, Option.map_some, Option.bind_some]
      obtain ⟨⟨writtenFrame, writtenState⟩, write_eq, borrow⟩ := Option.bind_eq_some_iff.mp borrow
      have minted : StateShifted offset frontier inert inert'
          { state with nextLoan := state.nextLoan + 1 }
          { state₂ with nextLoan := state₂.nextLoan + 1 } :=
        { shifted with
          nextLoan := by simp only [shifted.nextLoan]; omega
          frontier_le := Nat.le_succ_of_le shifted.frontier_le }
      obtain ⟨writtenState₂, write₂, writtenShifted, writtenAbove⟩ :=
        writeRuntimePlace?_mirror minted frame_above
          (value := .loanHole state.nextLoan) (by simpa using shifted.frontier_le) write_eq
      rw [RuntimeValue.shift_loanHole, ← shifted.nextLoan] at write₂
      rw [write₂, Option.bind_some]
      simp only [Option.some.injEq, Prod.mk.injEq] at borrow
      obtain ⟨rfl, rfl, rfl⟩ := borrow
      have read_above := readRuntimePlace?_above shifted frame_above read_eq
      have frame_eq : ({ writtenFrame with
          activeLoans := (writtenFrame.activeLoans.filter (·.1 != ⟨lexical⟩)).push
            (⟨lexical⟩, state.nextLoan)
          loanLocations := writtenFrame.loanLocations.push (state.nextLoan, place) } :
            RuntimeFrame).shift offset =
          { writtenFrame.shift offset with
            activeLoans := ((writtenFrame.shift offset).activeLoans.filter
              (·.1 != ⟨lexical⟩)).push (⟨lexical⟩, state₂.nextLoan)
            loanLocations := (writtenFrame.shift offset).loanLocations.push
              (state₂.nextLoan, place) } := by
        rw [RuntimeFrame.shift_withLoans, shifted.nextLoan, RuntimeFrame.shift_activeLoans,
          RuntimeFrame.shift_loanLocations, Array.filter_map]
        simp [Function.comp_def]
      have frame_above' : ({ writtenFrame with
          activeLoans := (writtenFrame.activeLoans.filter (·.1 != ⟨lexical⟩)).push
            (⟨lexical⟩, state.nextLoan)
          loanLocations := writtenFrame.loanLocations.push (state.nextLoan, place) } :
            RuntimeFrame).Above frontier :=
        { locals := writtenAbove.locals
          activeLoans := fun entry member => by
            rcases Array.mem_push.mp member with member | rfl
            · exact writtenAbove.activeLoans entry (Array.mem_filter.mp member).1
            · exact shifted.frontier_le
          loanLocations := fun entry member => by
            rcases Array.mem_push.mp member with member | rfl
            · exact writtenAbove.loanLocations entry member
            · exact shifted.frontier_le }
      have value_eq : (RuntimeValue.borrow state.nextLoan read).shift offset =
          .borrow state₂.nextLoan (read.shift offset) := by
        rw [RuntimeValue.shift_borrow, shifted.nextLoan]
      have value_above : (RuntimeValue.borrow state.nextLoan read).Above frontier := by
        simpa using ⟨shifted.frontier_le, read_above⟩
      rw [frame_eq, value_eq]
      cases root : place.root with
      | global key =>
          refine ⟨{ writtenState₂ with
              globalLoans := (state₂.nextLoan, key) :: writtenState₂.globalLoans }, rfl, ?_,
            frame_above', value_above⟩
          rw [shifted.nextLoan]
          exact writtenShifted.withRegistry (writtenShifted.registry.cons shifted.frontier_le key)
      | «local» _ => exact ⟨writtenState₂, rfl, writtenShifted, frame_above', value_above⟩

theorem borrowRuntimePlace?_mirror (shifted : StateShifted offset frontier inert inert' state state₂)
    {unit : Validation.ValidatedUnit} {ns : Validation.ValidatedNamespace} {site : ExprId}
    {referenceType : ReferenceType} {kind : BorrowKind} {frame frame' : RuntimeFrame}
    (frame_above : frame.Above frontier) {state' : RuntimeState} {place : RuntimePlace}
    {value : RuntimeValue}
    (borrow : borrowRuntimePlace? unit ns site referenceType kind frame state place =
      some (frame', state', value)) :
    ∃ state₂', borrowRuntimePlace? unit ns site referenceType kind (frame.shift offset) state₂
        place = some (frame'.shift offset, state₂', value.shift offset) ∧
      StateShifted offset frontier inert inert' state' state₂' ∧ frame'.Above frontier ∧
      value.Above frontier := by
  cases kind with
  | immutable =>
      rw [borrowRuntimePlace?_immutable_at (lexical := 0)] at borrow ⊢
      exact borrowRuntimePlaceAt?_mirror shifted frame_above borrow
  | mutable =>
      cases certificate : certificateLoanId? unit ns.identity site with
      | none =>
          simp [borrowRuntimePlace?, certificate] at borrow
      | some lexical =>
          rw [borrowRuntimePlace?_mutable_at (lexical := lexical) (loan_eq := certificate)] at borrow ⊢
          exact borrowRuntimePlaceAt?_mirror shifted frame_above borrow
  | profile _ => simp [borrowRuntimePlace?] at borrow

theorem RuntimeValue.storageKey?_shift (offset : Nat) (value : RuntimeValue) :
    (value.shift offset).storageKey? = value.storageKey? := by
  cases value <;> simp [RuntimeValue.storageKey?]

theorem RuntimeValue.storageKey_shift (offset : Nat) (value : RuntimeValue) :
    (value.shift offset).storageKey = value.storageKey := by
  simp only [RuntimeValue.storageKey, RuntimeValue.storageKey?_shift]

theorem globalKey_shift (namespaceId : NamespaceId) (typeId : TypeId) (key : RuntimeValue) :
    globalKey namespaceId typeId (key.shift offset) = globalKey namespaceId typeId key := by
  simp only [globalKey, RuntimeValue.storageKey_shift]

theorem globalValue?_shift (globals_eq : state₂.globals = state.globals.shift offset)
    (namespaceId : NamespaceId) (typeId : TypeId) (key : RuntimeValue) :
    globalValue? state₂ namespaceId typeId (key.shift offset) =
      (globalValue? state namespaceId typeId key).map (·.shift offset) := by
  simp only [globalValue?, globalKey_shift, globals_eq, GlobalMap.lookup_shift]

theorem globalExists_shift (globals_eq : state₂.globals = state.globals.shift offset)
    (namespaceId : NamespaceId) (typeId : TypeId) (key : RuntimeValue) :
    globalExists state₂ namespaceId typeId (key.shift offset) =
      globalExists state namespaceId typeId key := by
  simp only [globalExists, globalValue?_shift globals_eq, Option.isSome_map]

theorem GlobalMap.erase_above {globals : GlobalMap} {key : GlobalKey}
    (above : ∀ slot ∈ globals.entries, slot.value.Above frontier) :
    ∀ slot ∈ (globals.erase key).entries, slot.value.Above frontier := by
  intro slot member
  unfold GlobalMap.erase at member
  exact above slot (Array.mem_filter.mp member).1

theorem updateLocalBorrowValue?_shift (globals_eq : state₂.globals = state.globals.shift offset)
    (frame : RuntimeFrame) (loan : Nat) (replacement : RuntimeValue) :
    updateLocalBorrowValue? (frame.shift offset) state₂ (loan + offset)
        (replacement.shift offset) =
      (updateLocalBorrowValue? frame state loan replacement).map fun written =>
        (written.1.shift offset, { state₂ with globals := written.2.globals.shift offset }) := by
  have node := fun value => borrowRewrite?_shift (offset := offset) loan replacement value
  unfold updateLocalBorrowValue?
  simp only [Option.bind_eq_bind, localLoanPlace?_shift]
  cases localLoanPlace? frame loan with
  | none => rfl
  | some place =>
      simp only [Option.bind_some, readRuntimePlace?_shift globals_eq]
      cases readRuntimePlace? frame state place with
      | none => rfl
      | some value =>
          simp only [Option.map_some, Option.bind_some, rewriteFirst_shift node]
          cases rewriteFirst (borrowRewrite? loan replacement) value with
          | none => rfl
          | some updated =>
              simp only [Option.map_some, Option.bind_some]
              exact writeRuntimePlace?_shift globals_eq place updated

theorem updateBorrowValue?_shift (globals_eq : state₂.globals = state.globals.shift offset)
    (frame : RuntimeFrame) (loan : Nat) (replacement : RuntimeValue) :
    updateBorrowValue? (frame.shift offset) state₂ (loan + offset) (replacement.shift offset) =
      (updateBorrowValue? frame state loan replacement).map fun written =>
        (written.1.shift offset, { state₂ with globals := written.2.globals.shift offset }) := by
  have node := fun value => borrowRewrite?_shift (offset := offset) loan replacement value
  unfold updateBorrowValue?
  rw [updateLocalBorrowValue?_shift globals_eq]
  cases updateLocalBorrowValue? frame state loan replacement with
  | some written => rfl
  | none =>
      simp only [Option.map_none]
      have locals : ((fun slot : Option RuntimeValue =>
            (slot.bind fun value =>
              rewriteFirst (borrowRewrite? (loan + offset) (replacement.shift offset)) value).isSome) ∘
              Option.map fun value => value.shift offset) =
          fun slot => (slot.bind fun value =>
            rewriteFirst (borrowRewrite? loan replacement) value).isSome := by
        funext slot; cases slot <;> simp [rewriteFirst_shift node]
      rw [RuntimeFrame.shift_locals, Array.toList_map, indexOfFrom_map, locals]
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
                  simp only [Option.map_some, Option.bind_some, rewriteFirst_shift node]
                  cases rewriteFirst (borrowRewrite? loan replacement) value with
                  | none => rfl
                  | some updated =>
                      simp only [Option.map_some, ← globals_eq]
                      simp [RuntimeFrame.shift, Array.map_setIfInBounds]
      | none =>
          have globals : ((fun slot : GlobalSlot =>
                (rewriteFirst (borrowRewrite? (loan + offset) (replacement.shift offset))
                  slot.value).isSome) ∘
                  fun slot : GlobalSlot => { slot with value := slot.value.shift offset }) =
              fun slot => (rewriteFirst (borrowRewrite? loan replacement) slot.value).isSome := by
            funext slot; simp [rewriteFirst_shift node]
          rw [globals_eq, GlobalMap.shift_entries, Array.toList_map, indexOfFrom_map, globals]
          cases indexOfFrom (fun slot : GlobalSlot =>
              (rewriteFirst (borrowRewrite? loan replacement) slot.value).isSome)
              state.globals.entries.toList 0 with
          | none => rfl
          | some index =>
              simp only [Array.getElem?_map]
              cases state.globals.entries[index]? with
              | none => rfl
              | some slot =>
                  simp only [Option.map_some, Option.bind_some, rewriteFirst_shift node]
                  cases rewriteFirst (borrowRewrite? loan replacement) slot.value with
                  | none => rfl
                  | some updated =>
                      simp [GlobalMap.shift, Array.set!_eq_setIfInBounds, Array.map_setIfInBounds]

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

theorem updateBorrowValue?_mirror (shifted : StateShifted offset frontier inert inert' state state₂)
    {frame frame' : RuntimeFrame} (frame_above : frame.Above frontier) {state' : RuntimeState}
    {loan : Nat} (above : frontier ≤ loan) {replacement : RuntimeValue}
    (replacement_above : replacement.Above frontier)
    (update : updateBorrowValue? frame state loan replacement = some (frame', state')) :
    ∃ state₂', updateBorrowValue? (frame.shift offset) state₂ (loan + offset)
        (replacement.shift offset) = some (frame'.shift offset, state₂') ∧
      StateShifted offset frontier inert inert' state' state₂' ∧ frame'.Above frontier := by
  have base_eq : state.globals =
      ({ state with globals := state.globals.unshift frontier } : RuntimeState).globals.shift
        frontier := (GlobalMap.shift_unshift shifted.globalsAbove).symm
  have base := updateBorrowValue?_shift (offset := frontier) base_eq (frame.unshift frontier)
    (loan - frontier) (replacement.unshift frontier)
  rw [RuntimeFrame.shift_unshift frame_above, Nat.sub_add_cancel above,
    RuntimeValue.shift_unshift frontier replacement replacement_above, update] at base
  obtain ⟨written, -, written_eq⟩ := Option.map_eq_some_iff.mp base.symm
  simp only [Prod.mk.injEq] at written_eq
  obtain ⟨frame_eq, state_eq⟩ := written_eq
  have globals_above : ∀ slot ∈ state'.globals.entries, slot.value.Above frontier := by
    rw [← state_eq]
    exact GlobalMap.above_shift_self frontier _
  refine ⟨{ state₂ with globals := state'.globals.shift offset }, ?_, ?_, ?_⟩
  · rw [updateBorrowValue?_shift shifted.globals, update]
    rfl
  · rw [updateBorrowValue?_globals update]
    exact shifted.withGlobals _ globals_above
  · rw [← frame_eq]
    exact RuntimeFrame.above_shift_self frontier _

theorem selectNominalVariantFieldAt?_shift (offset : Nat) (owner : StructHandle)
    (choices : Array (String × Nat)) (arguments : Array RuntimeValue) :
    selectNominalVariantFieldAt? owner choices (arguments.map (·.shift offset)) =
      (selectNominalVariantFieldAt? owner choices arguments).map (·.shift offset) := by
  unfold selectNominalVariantFieldAt?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, rest⟩⟩
  all_goals try cases a
  all_goals try simp
  rename_i source variant fields
  cases variant <;> simp
  split <;> simp [Function.comp_def]

theorem evaluateDataOperation?_shift (offset : Nat) (unit : Validation.ValidatedUnit)
    (sourceNamespace : NamespaceId) (operation : DataOperation) (arguments : Array RuntimeValue) :
    evaluateDataOperation? unit sourceNamespace operation (arguments.map (·.shift offset)) =
      (evaluateDataOperation? unit sourceNamespace operation arguments).map (·.shift offset) := by
  unfold evaluateDataOperation?
  rw [Array.toList_map]
  cases operation with
  | selectVariants reference fields =>
      simp [Function.comp_def, selectNominalVariantFieldAt?_shift]
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

theorem evaluateDataOperation?_above {unit : Validation.ValidatedUnit}
    {sourceNamespace : NamespaceId} {operation : DataOperation} {arguments : Array RuntimeValue}
    {value : RuntimeValue} (above : ∀ argument ∈ arguments, argument.Above frontier)
    (evaluate : evaluateDataOperation? unit sourceNamespace operation arguments = some value) :
    value.Above frontier := by
  rw [← Array.shift_unshift_all above, evaluateDataOperation?_shift] at evaluate
  obtain ⟨base, -, rfl⟩ := Option.map_eq_some_iff.mp evaluate
  exact RuntimeValue.above_shift_self frontier base

theorem RuntimeFrame.Above.clearLocal {frame : RuntimeFrame} (above : frame.Above frontier)
    (index : Nat) :
    ({ frame with locals := frame.locals.set! index none } : RuntimeFrame).Above frontier where
  locals slot member stored stored_eq := by
    simp only [Array.set!_eq_setIfInBounds] at member
    rcases Array.mem_or_eq_of_mem_setIfInBounds member with member | rfl
    · exact above.locals slot member stored stored_eq
    · cases stored_eq
  activeLoans := above.activeLoans
  loanLocations := above.loanLocations

theorem evaluatePlaceOperation?_mirror
    (shifted : StateShifted offset frontier inert inert' state state₂)
    {unit : Validation.ValidatedUnit} {ns : Validation.ValidatedNamespace} {resultType : TypeId}
    {site : ExprId} {operation : Operation} {arguments : Array RuntimeValue}
    {frame frame' : RuntimeFrame} {state' : RuntimeState} {value : RuntimeValue}
    (frame_above : frame.Above frontier)
    (arguments_above : ∀ argument ∈ arguments, argument.Above frontier)
    (evaluate : evaluatePlaceOperation? unit ns resultType site operation arguments frame state =
      some (frame', state', value)) :
    ∃ state₂', evaluatePlaceOperation? unit ns resultType site operation
        (arguments.map (·.shift offset)) (frame.shift offset) state₂ =
        some (frame'.shift offset, state₂', value.shift offset) ∧
      StateShifted offset frontier inert inert' state' state₂' ∧ frame'.Above frontier ∧
      value.Above frontier := by
  have resolve := fun place => resolvePlace?_shift unit ns (frame := frame) shifted.globals place
  have read := readRuntimePlace?_shift shifted.globals (frame := frame)
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
      exact ⟨state₂, by simp [resolved_eq, read, current_eq], shifted, frame_above,
        readRuntimePlace?_above shifted frame_above current_eq⟩
  | read place =>
      simp only [evaluatePlaceOperation?, Array.isEmpty_map, Option.bind_eq_bind] at evaluate ⊢
      split at evaluate
      · cases evaluate
      rw [if_neg ‹_›, resolve]
      obtain ⟨resolved, resolved_eq, evaluate⟩ := Option.bind_eq_some_iff.mp evaluate
      obtain ⟨current, current_eq, evaluate⟩ := Option.bind_eq_some_iff.mp evaluate
      simp only [Option.some.injEq, Prod.mk.injEq] at evaluate
      obtain ⟨rfl, rfl, rfl⟩ := evaluate
      exact ⟨state₂, by simp [resolved_eq, read, current_eq], shifted, frame_above,
        readRuntimePlace?_above shifted frame_above current_eq⟩
  | move place =>
      simp only [evaluatePlaceOperation?, Array.isEmpty_map, Option.bind_eq_bind] at evaluate ⊢
      split at evaluate
      · cases evaluate
      rw [if_neg ‹_›, resolve]
      obtain ⟨resolved, resolved_eq, evaluate⟩ := Option.bind_eq_some_iff.mp evaluate
      obtain ⟨current, current_eq, evaluate⟩ := Option.bind_eq_some_iff.mp evaluate
      simp only [resolved_eq, Option.bind_some, read, current_eq, Option.map_some]
      have current_above := readRuntimePlace?_above shifted frame_above current_eq
      split at evaluate
      · next localId root_eq =>
          split at evaluate
          · simp only [Option.some.injEq, Prod.mk.injEq] at evaluate
            obtain ⟨rfl, rfl, rfl⟩ := evaluate
            rw [if_pos ‹_›]
            exact ⟨state₂, by simp [RuntimeFrame.shift, Array.map_setIfInBounds], shifted,
              frame_above.clearLocal _, current_above⟩
          · simp only [Option.some.injEq, Prod.mk.injEq] at evaluate
            obtain ⟨rfl, rfl, rfl⟩ := evaluate
            rw [if_neg ‹_›]
            exact ⟨state₂, rfl, shifted, frame_above, current_above⟩
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
            exact ⟨state₂, by simp [RuntimeFrame.shift, Array.map_setIfInBounds], shifted,
              frame_above.clearLocal _, by simp [RuntimeValue.Above]⟩
          · simp only [Option.some.injEq, Prod.mk.injEq] at evaluate
            obtain ⟨rfl, rfl, rfl⟩ := evaluate
            rw [if_neg ‹_›]
            exact ⟨state₂, by simp, shifted, frame_above, by simp [RuntimeValue.Above]⟩
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
          exact borrowRuntimePlace?_mirror shifted frame_above evaluate
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
          writeRuntimePlace?_mirror shifted frame_above (arguments_above written (by simp))
            write_eq
        exact ⟨state₂', by simp [resolved_eq, write₂], writtenShifted, writtenAbove,
          by simp [RuntimeValue.Above]⟩
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
          · have operand_above := arguments_above operand (by simp)
            simp only [List.map_toArray, List.map_cons, List.map_nil]
            rw [evaluatePlaceOperation?_dereference] at evaluate ⊢
            split at evaluate
            · simp only [Option.some.injEq, Prod.mk.injEq] at evaluate
              obtain ⟨rfl, rfl, rfl⟩ := evaluate
              rw [if_pos ‹_›]
              exact ⟨state₂, rfl, shifted, frame_above, operand_above⟩
            · rw [if_neg ‹_›]
              cases operand with
              | borrow loan current =>
                  simp only [dereferenceBorrow?, Option.some.injEq,
                    Prod.mk.injEq] at evaluate
                  obtain ⟨rfl, rfl, rfl⟩ := evaluate
                  simp only [RuntimeValue.above_borrow] at operand_above
                  exact ⟨state₂, by simp [dereferenceBorrow?], shifted, frame_above,
                    operand_above.2⟩
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
          · have operand_above := arguments_above operand (by simp)
            simp only [List.map_toArray, List.map_cons, List.map_nil]
            rw [evaluatePlaceOperation?_freeze] at evaluate ⊢
            split at evaluate
            · simp only [Option.some.injEq, Prod.mk.injEq] at evaluate
              obtain ⟨rfl, rfl, rfl⟩ := evaluate
              rw [if_pos ‹_›]
              exact ⟨state₂, rfl, shifted, frame_above, operand_above⟩
            · rw [if_neg ‹_›]
              obtain ⟨type, type_eq, evaluate⟩ := Option.bind_eq_some_iff.mp evaluate
              rw [type_eq, Option.bind_some]
              cases type with
              | reference resultReference =>
                  simp only at evaluate ⊢
                  rw [freezeBorrow?_single] at evaluate ⊢
                  cases operand with
                  | borrow loan current =>
                      simp only [RuntimeValue.shift_borrow] at evaluate ⊢
                      split at evaluate
                      · cases evaluate
                      · simp only [Option.some.injEq, Prod.mk.injEq] at evaluate
                        obtain ⟨rfl, rfl, rfl⟩ := evaluate
                        rw [if_neg ‹_›]
                        simp only [RuntimeValue.above_borrow] at operand_above
                        exact ⟨state₂, rfl, shifted, frame_above, operand_above.2⟩
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
          have member : ∀ value ∈ arguments.toList, value.Above frontier :=
            fun value member => arguments_above value (Array.mem_toList_iff.mp member)
          generalize arguments.toList = list at evaluate member ⊢
          rcases list with _ | ⟨target, _ | ⟨replacement, _ | ⟨third, rest⟩⟩⟩
          · cases evaluate
          · cases target <;> cases evaluate
          · cases target with
            | borrow loan current =>
                have target_above := member _ List.mem_cons_self
                simp only [RuntimeValue.above_borrow] at target_above
                have replacement_above := member replacement (by simp)
                simp only [List.map_cons, List.map_nil, RuntimeValue.shift_borrow]
                cases update : updateBorrowValue? frame state loan replacement with
                | some written =>
                    obtain ⟨writtenFrame, writtenState⟩ := written
                    simp only [update, Option.some.injEq, Prod.mk.injEq] at evaluate
                    obtain ⟨rfl, rfl, rfl⟩ := evaluate
                    obtain ⟨state₂', update₂, writtenShifted, writtenAbove⟩ :=
                      updateBorrowValue?_mirror shifted frame_above target_above.1
                        replacement_above update
                    simp only [update₂]
                    exact ⟨state₂', by simp, writtenShifted, writtenAbove,
                      by simp [RuntimeValue.Above]⟩
                | none =>
                    have update₂ := updateBorrowValue?_shift shifted.globals frame loan
                      replacement
                    rw [update, Option.map_none] at update₂
                    simp only [update, Option.some.injEq, Prod.mk.injEq] at evaluate
                    obtain ⟨rfl, rfl, rfl⟩ := evaluate
                    obtain ⟨state₂', back₂, backShifted, backAbove⟩ :=
                      applyWriteBack_shift shifted frame_above target_above.1 replacement_above
                    simp only [update₂, back₂]
                    exact ⟨state₂', by simp, backShifted, backAbove, by simp [RuntimeValue.Above]⟩
            | _ => cases evaluate
          · cases target <;> cases evaluate
  | data operation =>
      simp only [evaluatePlaceOperation?, Option.bind_eq_bind] at evaluate ⊢
      obtain ⟨result, result_eq, evaluate⟩ := Option.bind_eq_some_iff.mp evaluate
      simp only [Option.some.injEq, Prod.mk.injEq] at evaluate
      obtain ⟨rfl, rfl, rfl⟩ := evaluate
      exact ⟨state₂, by simp [evaluateDataOperation?_shift, result_eq], shifted, frame_above,
        evaluateDataOperation?_above arguments_above result_eq⟩
  | call _ | global _ | primitive _ | specification _ | assert | profile _ _ => cases evaluate

/-- Two results of a global operation mirror each other. -/
def SemanticOperations.GlobalOperationResult.Mirrors (offset frontier inert inert' : Nat) :
    GlobalOperationResult → GlobalOperationResult → Prop
  | .value frame state result, .value frame₂ state₂ result₂ =>
      LeanerIR.Mirrors offset frontier inert inert' (frame, state) (frame₂, state₂) ∧
        result₂ = result.shift offset ∧ result.Above frontier
  | .throw_ frame state kind values, .throw_ frame₂ state₂ kind₂ values₂ =>
      LeanerIR.Mirrors offset frontier inert inert' (frame, state) (frame₂, state₂) ∧
        kind₂ = kind ∧ values₂ = values.map (·.shift offset) ∧
        ∀ thrown ∈ values, thrown.Above frontier
  | _, _ => False

theorem evaluateGlobalOperation?_mirror
    (shifted : StateShifted offset frontier inert inert' state state₂)
    {unit : Validation.ValidatedUnit} {ns : Validation.ValidatedNamespace} {resultType : TypeId}
    {site : ExprId} {kind : GlobalKind} {instantiations : Array GenericArgument}
    {arguments : Array RuntimeValue} {frame : RuntimeFrame} {result : GlobalOperationResult}
    (frame_above : frame.Above frontier)
    (arguments_above : ∀ argument ∈ arguments, argument.Above frontier)
    (evaluate : evaluateGlobalOperation? unit ns resultType site kind instantiations arguments
      frame state = some result) :
    ∃ result₂, evaluateGlobalOperation? unit ns resultType site kind instantiations
        (arguments.map (·.shift offset)) (frame.shift offset) state₂ = some result₂ ∧
      result.Mirrors offset frontier inert inert' result₂ := by
  obtain ⟨generics⟩ := instantiations
  rcases generics with _ | ⟨generic, _ | ⟨second, rest⟩⟩
  · cases evaluate
  · cases generic with
    | typeArg resource =>
        rw [evaluateGlobalOperation?_typeArg] at evaluate ⊢
        simp only [RuntimeFrame.shift_typeInstantiation, Array.toList_map] at evaluate ⊢
        generalize instantiatedTypeId frame.typeInstantiation resource.typeId = resourceType
          at evaluate ⊢
        have member : ∀ value ∈ arguments.toList, value.Above frontier :=
          fun value member => arguments_above value (Array.mem_toList_iff.mp member)
        generalize arguments.toList = list at evaluate member ⊢
        cases kind with
        | contains =>
            rcases list with _ | ⟨key, _ | ⟨second, rest⟩⟩
            · cases evaluate
            · simp only [List.map_cons, List.map_nil, Option.bind_eq_bind,
                Option.bind_eq_some_iff, Option.some.injEq] at evaluate ⊢
              obtain ⟨storage, storage_eq, rfl⟩ := evaluate
              refine ⟨_, ⟨storage, by rw [RuntimeValue.storageKey?_shift, storage_eq], rfl⟩, ?_⟩
              refine ⟨⟨rfl, shifted, frame_above⟩, ?_, by simp [RuntimeValue.Above]⟩
              rw [globalExists_shift shifted.globals, RuntimeValue.shift_bool]
            · cases evaluate
        | borrow borrowKind =>
            rcases list with _ | ⟨key, _ | ⟨second, rest⟩⟩
            · cases evaluate
            · simp only [List.map_cons, List.map_nil, Option.bind_eq_bind,
                Option.bind_eq_some_iff] at evaluate ⊢
              obtain ⟨storage, storage_eq, evaluate⟩ := evaluate
              simp only [RuntimeValue.storageKey?_shift, storage_eq, Option.some.injEq,
                exists_eq_left', globalValue?_shift shifted.globals, globalKey_shift]
              cases lookup : globalValue? state ns.identity resourceType key with
              | none =>
                  rw [lookup, Option.some.injEq] at evaluate
                  subst evaluate
                  exact ⟨_, rfl, ⟨rfl, shifted, frame_above⟩, rfl, by simp, by simp⟩
              | some current =>
                  simp only [lookup, Option.bind_eq_some_iff] at evaluate
                  simp only [Option.map_some]
                  obtain ⟨type, type_eq, evaluate⟩ := evaluate
                  rw [type_eq, Option.bind_some]
                  cases type with
                  | reference referenceType =>
                      simp only [Option.bind_eq_some_iff, Option.some.injEq] at evaluate
                      obtain ⟨⟨frame', state', value⟩, borrow_eq, rfl⟩ := evaluate
                      obtain ⟨state₂', borrow₂, shifted', frame_above', value_above⟩ :=
                        borrowRuntimePlace?_mirror shifted frame_above borrow_eq
                      refine ⟨.value (frame'.shift offset) state₂' (value.shift offset), ?_,
                        ⟨rfl, shifted', frame_above'⟩, rfl, value_above⟩
                      simp [borrow₂]
                  | _ => cases evaluate
            · cases evaluate
        | take =>
            rcases list with _ | ⟨key, _ | ⟨second, rest⟩⟩
            · cases evaluate
            · simp only [List.map_cons, List.map_nil, Option.bind_eq_bind,
                Option.bind_eq_some_iff] at evaluate ⊢
              obtain ⟨storage, storage_eq, evaluate⟩ := evaluate
              simp only [RuntimeValue.storageKey?_shift, storage_eq, Option.some.injEq,
                exists_eq_left', globalValue?_shift shifted.globals, globalKey_shift]
              cases lookup : globalValue? state ns.identity resourceType key with
              | none =>
                  rw [lookup, Option.some.injEq] at evaluate
                  subst evaluate
                  exact ⟨_, rfl, ⟨rfl, shifted, frame_above⟩, rfl, by simp, by simp⟩
              | some current =>
                  simp only [lookup, Option.some.injEq] at evaluate
                  subst evaluate
                  refine ⟨_, rfl, ⟨rfl, ?_, frame_above⟩, rfl, shifted.slot_above lookup⟩
                  have erased := shifted.withGlobals
                    (state.globals.erase (globalKey ns.identity resourceType key))
                    (GlobalMap.erase_above shifted.globalsAbove)
                  rwa [GlobalMap.erase_shift, ← shifted.globals] at erased
            · cases evaluate
        | publish =>
            rcases list with _ | ⟨key, _ | ⟨value, _ | ⟨third, rest⟩⟩⟩
            · cases evaluate
            · cases evaluate
            · simp only [List.map_cons, List.map_nil, Option.bind_eq_bind,
                Option.bind_eq_some_iff] at evaluate ⊢
              obtain ⟨storage, storage_eq, evaluate⟩ := evaluate
              simp only [RuntimeValue.storageKey?_shift, storage_eq, Option.some.injEq,
                exists_eq_left', globalValue?_shift shifted.globals, globalKey_shift]
              cases lookup : globalValue? state ns.identity resourceType key with
              | some current =>
                  rw [lookup, Option.some.injEq] at evaluate
                  subst evaluate
                  exact ⟨_, rfl, ⟨rfl, shifted, frame_above⟩, rfl, by simp, by simp⟩
              | none =>
                  simp only [lookup, Option.some.injEq] at evaluate
                  subst evaluate
                  refine ⟨_, rfl, ⟨rfl, ?_, frame_above⟩, by simp, by simp [RuntimeValue.Above]⟩
                  have inserted := shifted.withGlobals
                    (state.globals.insert (globalKey ns.identity resourceType key) value)
                    (GlobalMap.insert_above shifted.globalsAbove
                      (member value (by simp)))
                  rwa [GlobalMap.insert_shift, ← shifted.globals] at inserted
            · cases evaluate
    | const _ => cases evaluate
    | lifetime _ => cases evaluate
    | evidence _ => cases evaluate
  · cases evaluate

end Effects

end LeanerIR
