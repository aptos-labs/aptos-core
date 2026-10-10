-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.LoanRenamingEffects

/-!
# Function boundaries under loan renaming

A frame built from shifted arguments is the shifted frame, a finished
control the shifted outcome, and a dying frame exports its loans into
mirroring states (`finalizeFunctionState_mirror`).
-/

namespace LeanerIR

open Validation
open SemanticOperations

theorem RuntimeFrame.shift_empty (offset : Nat) :
    ({ locals := #[] } : RuntimeFrame).shift offset = { locals := #[] } := by
  simp [RuntimeFrame.shift]

theorem RuntimeFrame.empty_above (frontier : Nat) :
    ({ locals := #[] } : RuntimeFrame).Above frontier :=
  { locals := fun _ member => by simp at member
    activeLoans := fun _ member => by simp at member
    loanLocations := fun _ member => by simp at member }

section Frames

variable {offset frontier inert inert' : Nat} {state state₂ : RuntimeState}

theorem frameBorrows_shift (frame : RuntimeFrame) :
    frameBorrows (frame.shift offset) =
      (frameBorrows frame).map fun entry => (entry.1 + offset, entry.2.shift offset) := by
  unfold frameBorrows
  rw [RuntimeFrame.shift_locals, Array.foldl_map, ← Array.foldl_toList, ← Array.foldl_toList]
  conv => lhs; rw [show (#[] : Array (Nat × RuntimeValue)) =
    (#[] : Array (Nat × RuntimeValue)).map fun entry => (entry.1 + offset, entry.2.shift offset)
    from Array.map_empty.symm]
  generalize (#[] : Array (Nat × RuntimeValue)) = start
  induction frame.locals.toList generalizing start with
  | nil => rfl
  | cons slot slots ih =>
      cases slot with
      | none => exact ih start
      | some value =>
          simp only [List.foldl_cons, Option.map_some]
          rw [outermostBorrows_shift value, ← Array.map_append, ih]

theorem frameBorrows_above {frame : RuntimeFrame} (frame_above : frame.Above frontier) :
    ∀ entry ∈ frameBorrows frame, frontier ≤ entry.1 ∧ entry.2.Above frontier := by
  intro entry member
  rw [← RuntimeFrame.shift_unshift frame_above, frameBorrows_shift] at member
  obtain ⟨base, -, rfl⟩ := Array.mem_map.mp member
  exact ⟨Nat.le_add_left _ _, RuntimeValue.above_shift_self frontier base.2⟩

theorem settleFrameLoans_mirror : ∀ (fuel : Nat) {frame : RuntimeFrame} {state state₂ : RuntimeState},
    StateShifted offset frontier inert inert' state state₂ → frame.Above frontier →
    Mirrors offset frontier inert inert' (settleFrameLoans frame state fuel)
      (settleFrameLoans (frame.shift offset) state₂ fuel)
  | 0, _, _, _, shifted, frame_above => ⟨rfl, shifted, frame_above⟩
  | fuel + 1, frame, state, state₂, shifted, frame_above => by
      unfold settleFrameLoans
      have predicate : ((fun entry : Nat × RuntimeValue =>
            holeInFrame (frame.shift offset) entry.1) ∘
            fun entry : Nat × RuntimeValue => (entry.1 + offset, entry.2.shift offset)) =
          fun entry => holeInFrame frame entry.1 := by
        funext entry; simp [holeInFrame_shift]
      rw [frameBorrows_shift, Array.find?_map, predicate]
      cases found : (frameBorrows frame).find? (fun entry => holeInFrame frame entry.1) with
      | none => exact ⟨rfl, shifted, frame_above⟩
      | some entry =>
          obtain ⟨loan, current⟩ := entry
          obtain ⟨loan_above, current_above⟩ :=
            frameBorrows_above frame_above _ (Array.mem_of_find?_eq_some found)
          simp only [Option.map_some]
          obtain ⟨cleared₂, clear_eq, clearShifted, clearAbove⟩ :=
            clearBorrowValue_shift shifted frame_above loan_above
          rw [clear_eq]
          obtain ⟨filled₂, fill_eq, fillShifted, fillAbove⟩ :=
            fillVisibleHole_shift clearShifted clearAbove loan_above current_above
          rw [fill_eq]
          exact settleFrameLoans_mirror fuel fillShifted fillAbove

theorem exportSettledLoans_mirror (shifted : StateShifted offset frontier inert inert' state state₂)
    {frame : RuntimeFrame} (frame_above : frame.Above frontier) :
    StateShifted offset frontier inert inert' (exportSettledLoans frame state)
      (exportSettledLoans (frame.shift offset) state₂) := by
  have entries_above := frameBorrows_above frame_above
  unfold exportSettledLoans
  rw [frameBorrows_shift, Array.foldl_map, ← Array.foldl_toList, ← Array.foldl_toList]
  simp only [Array.mem_toList_iff.symm] at entries_above
  simp only [holeInFrame_shift]
  revert entries_above
  generalize (frameBorrows frame).toList = borrows
  intro entries_above
  induction borrows generalizing state state₂ with
  | nil => exact shifted
  | cons entry borrows ih =>
      obtain ⟨loan, current⟩ := entry
      obtain ⟨loan_above, current_above⟩ := entries_above _ List.mem_cons_self
      simp only [List.foldl_cons]
      split
      · exact ih shifted fun entry member => entries_above entry (List.mem_cons_of_mem _ member)
      · obtain ⟨written₂, write_eq, writtenShifted, -⟩ :=
          applyWriteBack_shift shifted (RuntimeFrame.empty_above frontier) loan_above
            current_above
        rw [RuntimeFrame.shift_empty] at write_eq
        rw [write_eq]
        exact ih writtenShifted fun entry member =>
          entries_above entry (List.mem_cons_of_mem _ member)

theorem exportFrameLoans_mirror (shifted : StateShifted offset frontier inert inert' state state₂)
    {frame : RuntimeFrame} (frame_above : frame.Above frontier) :
    StateShifted offset frontier inert inert' (exportFrameLoans frame state)
      (exportFrameLoans (frame.shift offset) state₂) := by
  unfold exportFrameLoans
  have predicate : ((fun entry : Nat × RuntimeValue =>
        holeInFrame (frame.shift offset) entry.1) ∘
        fun entry : Nat × RuntimeValue => (entry.1 + offset, entry.2.shift offset)) =
      fun entry => holeInFrame frame entry.1 := by
    funext entry; simp [holeInFrame_shift]
  rw [frameBorrows_shift, Array.find?_map, predicate, Array.size_map]
  cases (frameBorrows frame).find? (fun entry => holeInFrame frame entry.1) with
  | none => exact exportSettledLoans_mirror shifted frame_above
  | some entry =>
      obtain ⟨settled, settledShifted, settledAbove⟩ :=
        settleFrameLoans_mirror (frameBorrows frame).size shifted frame_above
      simp only [Option.map_some]
      rw [settled]
      exact exportSettledLoans_mirror settledShifted settledAbove

theorem Array.contains_map_add (loans : Array Nat) (loan offset : Nat) :
    (loans.map (· + offset)).contains (loan + offset) = loans.contains loan := by
  rw [Bool.eq_iff_iff]
  simp

mutual
theorem maskReturnedBorrows_shift (loans : Array Nat) (value : RuntimeValue) :
    maskReturnedBorrows (loans.map (· + offset)) (value.shift offset) =
      (maskReturnedBorrows loans value).shift offset := by
  match value with
  | .borrow loan current =>
      rw [RuntimeValue.shift_borrow, maskReturnedBorrows, maskReturnedBorrows,
        Array.contains_map_add]
      split <;> simp
  | .vector elements =>
      rw [RuntimeValue.shift_vector, maskReturnedBorrows, maskReturnedBorrows, Array.toList_map,
        maskReturnedBorrowList_shift loans elements.toList]
      simp
  | .tuple elements =>
      rw [RuntimeValue.shift_tuple, maskReturnedBorrows, maskReturnedBorrows, Array.toList_map,
        maskReturnedBorrowList_shift loans elements.toList]
      simp
  | .nominal source variant fields =>
      rw [RuntimeValue.shift_nominal, maskReturnedBorrows, maskReturnedBorrows, Array.toList_map,
        maskReturnedBorrowList_shift loans fields.toList]
      simp
  | .closure function mask instantiation captures =>
      rw [RuntimeValue.shift_closure, maskReturnedBorrows, maskReturnedBorrows, Array.toList_map,
        maskReturnedBorrowList_shift loans captures.toList]
      simp
  | .unit | .bool _ | .character _ | .integer _ | .address _ | .signer _ | .string _
  | .bytes _ | .loanHole _ => simp [maskReturnedBorrows]
termination_by sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); omega)

theorem maskReturnedBorrowList_shift (loans : Array Nat) (values : List RuntimeValue) :
    maskReturnedBorrowList (loans.map (· + offset)) (values.map (·.shift offset)) =
      (maskReturnedBorrowList loans values).map (·.shift offset) := by
  match values with
  | [] => simp [maskReturnedBorrowList]
  | value :: rest =>
      rw [List.map_cons, maskReturnedBorrowList, maskReturnedBorrowList, List.map_cons,
        maskReturnedBorrows_shift loans value, maskReturnedBorrowList_shift loans rest]
termination_by sizeOf values
decreasing_by all_goals (simp_wf; omega)
end

mutual
theorem maskReturnedBorrows_above (loans : Array Nat) (value : RuntimeValue)
    (above : value.Above frontier) : (maskReturnedBorrows loans value).Above frontier := by
  match value with
  | .borrow loan current =>
      rw [maskReturnedBorrows]
      split
      · simp [RuntimeValue.Above]
      · exact above
  | .vector elements =>
      rw [maskReturnedBorrows]
      simp only [RuntimeValue.above_vector, List.mem_toArray] at above ⊢
      exact maskReturnedBorrowList_above loans elements.toList (fun value member =>
        above value (Array.mem_toList_iff.mp member))
  | .tuple elements =>
      rw [maskReturnedBorrows]
      simp only [RuntimeValue.above_tuple, List.mem_toArray] at above ⊢
      exact maskReturnedBorrowList_above loans elements.toList (fun value member =>
        above value (Array.mem_toList_iff.mp member))
  | .nominal source variant fields =>
      rw [maskReturnedBorrows]
      simp only [RuntimeValue.above_nominal, List.mem_toArray] at above ⊢
      exact maskReturnedBorrowList_above loans fields.toList (fun value member =>
        above value (Array.mem_toList_iff.mp member))
  | .closure function mask instantiation captures =>
      rw [maskReturnedBorrows]
      simp only [RuntimeValue.above_closure, List.mem_toArray] at above ⊢
      exact maskReturnedBorrowList_above loans captures.toList (fun value member =>
        above value (Array.mem_toList_iff.mp member))
  | .unit | .bool _ | .character _ | .integer _ | .address _ | .signer _ | .string _
  | .bytes _ | .loanHole _ => rw [maskReturnedBorrows]; exact above
termination_by sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); omega)

theorem maskReturnedBorrowList_above (loans : Array Nat) (values : List RuntimeValue)
    (above : ∀ value ∈ values, value.Above frontier) :
    ∀ value ∈ maskReturnedBorrowList loans values, value.Above frontier := by
  match values with
  | [] => intro value member; simp [maskReturnedBorrowList] at member
  | head :: rest =>
      rw [maskReturnedBorrowList]
      intro value member
      rcases List.mem_cons.mp member with rfl | member
      · exact maskReturnedBorrows_above loans head (above head List.mem_cons_self)
      · exact maskReturnedBorrowList_above loans rest
          (fun value member => above value (List.mem_cons_of_mem _ member)) value member
termination_by sizeOf values
decreasing_by all_goals (simp_wf; omega)
end

theorem returnedBorrowIds_shift (results : Array RuntimeValue) :
    returnedBorrowIds (results.map (·.shift offset)) =
      (returnedBorrowIds results).map (· + offset) := by
  unfold returnedBorrowIds
  rw [Array.foldl_map, ← Array.foldl_toList, ← Array.foldl_toList]
  conv => lhs; rw [show (#[] : Array Nat) = (#[] : Array Nat).map (· + offset)
    from Array.map_empty.symm]
  generalize (#[] : Array Nat) = start
  induction results.toList generalizing start with
  | nil => rfl
  | cons result results ih =>
      simp only [List.foldl_cons]
      rw [outermostBorrows_shift result, Array.map_map, ← ih]
      simp [Function.comp_def, Array.map_append, Array.map_map]

theorem scalarFunctionResult_shift (results : Array RuntimeValue) :
    scalarFunctionResult (results.map (·.shift offset)) = scalarFunctionResult results := by
  unfold scalarFunctionResult
  rw [Array.toList_map]
  rcases results.toList with _ | ⟨result, _ | ⟨second, rest⟩⟩
  · rfl
  · cases result <;> simp
  · simp

theorem exportReturnedFrameLoans_mirror
    (shifted : StateShifted offset frontier inert inert' state state₂) {frame : RuntimeFrame}
    (frame_above : frame.Above frontier) (results : Array RuntimeValue) :
    StateShifted offset frontier inert inert' (exportReturnedFrameLoans results frame state)
      (exportReturnedFrameLoans (results.map (·.shift offset)) (frame.shift offset) state₂) := by
  unfold exportReturnedFrameLoans
  rw [Array.isEmpty_map, scalarFunctionResult_shift, frameBorrows_shift, Array.isEmpty_map,
    returnedBorrowIds_shift]
  simp only [Array.isEmpty_map]
  split
  · exact exportFrameLoans_mirror shifted frame_above
  split
  · exact exportFrameLoans_mirror shifted frame_above
  split
  · exact shifted
  split
  · exact exportFrameLoans_mirror shifted frame_above
  have frame_eq : ({ frame.shift offset with
      locals := (frame.shift offset).locals.map
        (Option.map (maskReturnedBorrows ((returnedBorrowIds results).map (· + offset)))) } :
        RuntimeFrame) =
      ({ frame with
        locals := frame.locals.map (Option.map (maskReturnedBorrows (returnedBorrowIds results))) } :
        RuntimeFrame).shift offset := by
    simp [RuntimeFrame.shift, Array.map_map, Function.comp_def, Option.map_map,
      maskReturnedBorrows_shift]
  rw [frame_eq]
  refine exportFrameLoans_mirror shifted
    { locals := fun slot member value value_eq => ?_
      activeLoans := frame_above.activeLoans
      loanLocations := frame_above.loanLocations }
  obtain ⟨original, original_member, rfl⟩ := Array.mem_map.mp member
  cases original with
  | none => cases value_eq
  | some original =>
      simp only [Option.map_some, Option.some.injEq] at value_eq
      subst value_eq
      exact maskReturnedBorrows_above _ original
        (frame_above.locals _ original_member original rfl)

@[simp] theorem Outcome.shift_returned (offset : Nat) (values : Array RuntimeValue) :
    (Outcome.returned values).shift offset = .returned (values.map (·.shift offset)) := rfl
@[simp] theorem Outcome.shift_threw (offset : Nat) (kind : ThrowKind) (values : Array RuntimeValue) :
    (Outcome.threw kind values).shift offset = .threw kind (values.map (·.shift offset)) := rfl

theorem finalizeFunctionState_mirror {initial initial₂ : RuntimeState}
    (initialShifted : StateShifted offset frontier inert inert' initial initial₂)
    (shifted : StateShifted offset frontier inert inert' state state₂)
    {unit : ValidatedUnit} (executable : ExecutableUnit unit) (profile : Profile)
    {frame : RuntimeFrame}
    (frame_above : frame.Above frontier) (outcome : Outcome) :
    StateShifted offset frontier inert inert'
      (finalizeFunctionState executable profile initial state frame outcome)
      (finalizeFunctionState executable profile initial₂ state₂ (frame.shift offset)
        (outcome.shift offset)) := by
  cases outcome with
  | returned results =>
      exact exportReturnedFrameLoans_mirror shifted frame_above results
  | threw kind values =>
      simp only [Outcome.shift_threw, finalizeFunctionState]
      split
      · split
        · exact initialShifted
        · exact exportFrameLoans_mirror shifted frame_above
      · exact exportFrameLoans_mirror shifted frame_above

theorem initialLocals_map (f : RuntimeValue → RuntimeValue) (count : Nat)
    (arguments : Array RuntimeValue) :
    initialLocals count (arguments.map f) = (initialLocals count arguments).map (Option.map f) := by
  unfold initialLocals
  apply Array.ext
  · simp
  · intro index _ _
    simp only [Array.getElem_ofFn, Array.getElem_map, Array.size_map]
    split <;> simp

theorem parameterLoanLocations_shift (arguments : Array RuntimeValue) :
    parameterLoanLocations (arguments.map (·.shift offset)) =
      (parameterLoanLocations arguments).map (shiftEntry offset) := by
  unfold parameterLoanLocations
  rw [Array.zipIdx_map, Array.filterMap_map, Array.map_filterMap]
  congr 1
  funext entry
  obtain ⟨value, index⟩ := entry
  cases value <;> simp

theorem initialFrame?_shift (declaration : FunctionDecl FunctionBody)
    (arguments : Array RuntimeValue) (typeInstantiation : Array (TypeId × TypeId)) :
    initialFrame? declaration (arguments.map (·.shift offset)) typeInstantiation =
      (initialFrame? declaration arguments typeInstantiation).map (·.shift offset) := by
  unfold initialFrame?
  simp only [Array.size_map]
  split
  · rfl
  split
  · rfl
  simp [RuntimeFrame.shift, initialLocals_map, parameterLoanLocations_shift]

theorem initialFrame?_above {declaration : FunctionDecl FunctionBody}
    {arguments : Array RuntimeValue} {typeInstantiation : Array (TypeId × TypeId)}
    {frame : RuntimeFrame} (above : ∀ argument ∈ arguments, argument.Above frontier)
    (initial : initialFrame? declaration arguments typeInstantiation = some frame) :
    frame.Above frontier := by
  rw [← Array.shift_unshift_all above, initialFrame?_shift] at initial
  obtain ⟨base, -, rfl⟩ := Option.map_eq_some_iff.mp initial
  exact RuntimeFrame.above_shift_self frontier base

theorem unpackFallthrough_shift (expected : Nat) (value : RuntimeValue) :
    unpackFallthrough expected (value.shift offset) =
      (unpackFallthrough expected value).map (·.map (·.shift offset)) := by
  unfold unpackFallthrough
  match expected with
  | 0 => cases value <;> simp
  | 1 => simp
  | count + 2 => cases value <;> simp

theorem finishControl?_shift (resultCount : Nat) (control : Control) :
    finishControl? resultCount (control.shift offset) =
      (finishControl? resultCount control).map (·.shift offset) := by
  cases control with
  | value value =>
      simp only [Control.shift, finishControl?, unpackFallthrough_shift]
      cases unpackFallthrough resultCount value <;> simp
  | return_ values =>
      simp only [Control.shift, finishControl?, Array.size_map]
      split <;> simp
  | throw_ kind values => simp [Control.shift, finishControl?]
  | break_ => simp [Control.shift, finishControl?]
  | continue_ => simp [Control.shift, finishControl?]

theorem finishControl?_above {resultCount : Nat} {control : Control} {outcome : Outcome}
    (above : control.Above frontier) (finish : finishControl? resultCount control = some outcome) :
    outcome.Above frontier := by
  cases control with
  | value value =>
      simp only [finishControl?, Functor.map, Option.map_eq_some_iff] at finish
      obtain ⟨values, values_eq, rfl⟩ := finish
      rw [← RuntimeValue.shift_unshift frontier value above, unpackFallthrough_shift] at values_eq
      obtain ⟨base, -, rfl⟩ := Option.map_eq_some_iff.mp values_eq
      intro value member
      obtain ⟨original, -, rfl⟩ := Array.mem_map.mp member
      exact RuntimeValue.above_shift_self frontier original
  | return_ values =>
      simp only [finishControl?] at finish
      split at finish
      · cases finish; exact above
      · cases finish
  | throw_ kind values => cases finish; exact above
  | break_ => cases finish
  | continue_ => cases finish

mutual
theorem RuntimeValue.holeFree?_shift (value : RuntimeValue) :
    (value.shift offset).holeFree? = value.holeFree? := by
  match value with
  | .vector elements | .tuple elements =>
      simp only [RuntimeValue.shift_vector, RuntimeValue.shift_tuple]
      rw [RuntimeValue.holeFree?, RuntimeValue.holeFree?, Array.toList_map,
        RuntimeValue.holeFreeList?_shift elements.toList]
  | .nominal source variant fields =>
      rw [RuntimeValue.shift_nominal, RuntimeValue.holeFree?, RuntimeValue.holeFree?,
        Array.toList_map, RuntimeValue.holeFreeList?_shift fields.toList]
  | .closure function mask instantiation captures =>
      rw [RuntimeValue.shift_closure, RuntimeValue.holeFree?, RuntimeValue.holeFree?,
        Array.toList_map, RuntimeValue.holeFreeList?_shift captures.toList]
  | .borrow loan current => simp [RuntimeValue.holeFree?]
  | .loanHole loan => simp [RuntimeValue.holeFree?]
  | .unit | .bool _ | .character _ | .integer _ | .address _ | .signer _ | .string _
  | .bytes _ => simp [RuntimeValue.holeFree?]
termination_by sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); omega)

theorem RuntimeValue.holeFreeList?_shift (values : List RuntimeValue) :
    RuntimeValue.holeFreeList? (values.map (·.shift offset)) =
      RuntimeValue.holeFreeList? values := by
  match values with
  | [] => simp [RuntimeValue.holeFreeList?]
  | value :: rest =>
      rw [List.map_cons, RuntimeValue.holeFreeList?, RuntimeValue.holeFreeList?,
        RuntimeValue.holeFree?_shift value, RuntimeValue.holeFreeList?_shift rest]
termination_by sizeOf values
decreasing_by all_goals (simp_wf; omega)
end

theorem Outcome.holeFree_shift (outcome : Outcome) :
    (outcome.shift offset).holeFree = outcome.holeFree := by
  cases outcome with
  | returned values =>
      simp only [Outcome.shift_returned, Outcome.holeFree, Array.all_map]
      congr 1
      funext value
      simp [RuntimeValue.holeFree?_shift]
  | threw => rfl

end Frames

end LeanerIR
