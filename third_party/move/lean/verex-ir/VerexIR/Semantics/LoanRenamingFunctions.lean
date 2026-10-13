-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Semantics.LoanRenamingEffects

/-!
# Function boundaries under loan renaming

A frame built from renamed arguments is the renamed frame, a finished
control the renamed outcome, and a dying frame exports its loans into
mirroring states (`finalizeFunctionState_mirror`).
-/

namespace VerexIR

variable {inside : Nat → Prop}

open Validation
open SemanticOperations

theorem RuntimeFrame.rename_empty (ρ : Nat → Nat) :
    ({ locals := #[] } : RuntimeFrame).rename ρ = { locals := #[] } := by
  simp [RuntimeFrame.rename]

theorem RuntimeFrame.empty_within (inside : Nat → Prop) :
    ({ locals := #[] } : RuntimeFrame).Within inside :=
  { locals := fun _ member => by simp at member
    activeLoans := fun _ member => by simp at member
    loanLocations := fun _ member => by simp at member }

section Frames

variable {ρ : Nat → Nat} {offset frontier inert inert' : Nat} {inside : Nat → Prop} {state state₂ : RuntimeState}

theorem frameBorrows_rename (frame : RuntimeFrame) :
    frameBorrows (frame.rename ρ) =
      (frameBorrows frame).map fun entry => (ρ entry.1, entry.2.rename ρ) := by
  unfold frameBorrows
  rw [RuntimeFrame.rename_locals, Array.foldl_map, ← Array.foldl_toList, ← Array.foldl_toList]
  conv => lhs; rw [show (#[] : Array (Nat × RuntimeValue)) =
    (#[] : Array (Nat × RuntimeValue)).map fun entry => (ρ entry.1, entry.2.rename ρ)
    from Array.map_empty.symm]
  generalize (#[] : Array (Nat × RuntimeValue)) = start
  induction frame.locals.toList generalizing start with
  | nil => rfl
  | cons slot slots ih =>
      cases slot with
      | none => exact ih start
      | some value =>
          simp only [List.foldl_cons, Option.map_some]
          rw [outermostBorrows_rename value, ← Array.map_append, ih]

theorem frameBorrows_within {β γ : Nat → Nat} (base : LoanBase β γ inside) {frame : RuntimeFrame}
    (frame_within : frame.Within inside) :
    ∀ entry ∈ frameBorrows frame, inside entry.1 ∧ entry.2.Within inside := by
  intro entry member
  rw [← base.rename_inverse_frame frame_within, frameBorrows_rename] at member
  obtain ⟨original, -, rfl⟩ := Array.mem_map.mp member
  exact ⟨base.image _, base.within original.2⟩

theorem settleFrameLoans_mirror : ∀ (fuel : Nat) {frame : RuntimeFrame} {state state₂ : RuntimeState},
    StateRenamed ρ offset frontier inside inert inert' state state₂ → frame.Within inside →
    Mirrors ρ offset frontier inside inert inert' (settleFrameLoans frame state fuel)
      (settleFrameLoans (frame.rename ρ) state₂ fuel)
  | 0, _, _, _, shifted, frame_within => ⟨rfl, shifted, frame_within⟩
  | fuel + 1, frame, state, state₂, shifted, frame_within => by
      obtain ⟨β, γ, base⟩ := shifted.renames.base
      unfold settleFrameLoans
      have predicate : ((fun entry : Nat × RuntimeValue =>
            holeInFrame (frame.rename ρ) entry.1) ∘
            fun entry : Nat × RuntimeValue => (ρ entry.1, entry.2.rename ρ)) =
          fun entry => holeInFrame frame entry.1 := by
        funext entry; simp [holeInFrame_rename shifted.renames.injective]
      rw [frameBorrows_rename, Array.find?_map, predicate]
      cases found : (frameBorrows frame).find? (fun entry => holeInFrame frame entry.1) with
      | none => exact ⟨rfl, shifted, frame_within⟩
      | some entry =>
          obtain ⟨loan, current⟩ := entry
          obtain ⟨loan_within, current_within⟩ :=
            frameBorrows_within base frame_within _ (Array.mem_of_find?_eq_some found)
          simp only [Option.map_some]
          obtain ⟨cleared₂, clear_eq, clearShifted, clearAbove⟩ :=
            clearBorrowValue_rename shifted frame_within loan_within
          rw [clear_eq]
          obtain ⟨filled₂, fill_eq, fillShifted, fillAbove⟩ :=
            fillVisibleHole_rename clearShifted clearAbove loan_within current_within
          rw [fill_eq]
          exact settleFrameLoans_mirror fuel fillShifted fillAbove

theorem exportSettledLoans_mirror (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    {frame : RuntimeFrame} (frame_within : frame.Within inside) :
    StateRenamed ρ offset frontier inside inert inert' (exportSettledLoans frame state)
      (exportSettledLoans (frame.rename ρ) state₂) := by
  obtain ⟨β, γ, base⟩ := shifted.renames.base
  have entries_within := frameBorrows_within base frame_within
  unfold exportSettledLoans
  rw [frameBorrows_rename, Array.foldl_map, ← Array.foldl_toList, ← Array.foldl_toList]
  simp only [Array.mem_toList_iff.symm] at entries_within
  simp only [holeInFrame_rename shifted.renames.injective]
  revert entries_within
  generalize (frameBorrows frame).toList = borrows
  intro entries_within
  induction borrows generalizing state state₂ with
  | nil => exact shifted
  | cons entry borrows ih =>
      obtain ⟨loan, current⟩ := entry
      obtain ⟨loan_within, current_within⟩ := entries_within _ List.mem_cons_self
      simp only [List.foldl_cons]
      split
      · exact ih shifted fun entry member => entries_within entry (List.mem_cons_of_mem _ member)
      · obtain ⟨written₂, write_eq, writtenShifted, -⟩ :=
          applyWriteBack_rename shifted (RuntimeFrame.empty_within inside) loan_within
            current_within
        rw [RuntimeFrame.rename_empty] at write_eq
        rw [write_eq]
        exact ih writtenShifted fun entry member =>
          entries_within entry (List.mem_cons_of_mem _ member)

theorem exportFrameLoans_mirror (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    {frame : RuntimeFrame} (frame_within : frame.Within inside) :
    StateRenamed ρ offset frontier inside inert inert' (exportFrameLoans frame state)
      (exportFrameLoans (frame.rename ρ) state₂) := by
  unfold exportFrameLoans
  have predicate : ((fun entry : Nat × RuntimeValue =>
        holeInFrame (frame.rename ρ) entry.1) ∘
        fun entry : Nat × RuntimeValue => (ρ entry.1, entry.2.rename ρ)) =
      fun entry => holeInFrame frame entry.1 := by
    funext entry; simp [holeInFrame_rename shifted.renames.injective]
  rw [frameBorrows_rename, Array.find?_map, predicate, Array.size_map]
  cases (frameBorrows frame).find? (fun entry => holeInFrame frame entry.1) with
  | none => exact exportSettledLoans_mirror shifted frame_within
  | some entry =>
      obtain ⟨settled, settledShifted, settledAbove⟩ :=
        settleFrameLoans_mirror (frameBorrows frame).size shifted frame_within
      simp only [Option.map_some]
      rw [settled]
      exact exportSettledLoans_mirror settledShifted settledAbove

theorem Array.contains_map_rename (injective : Function.Injective ρ) (loans : Array Nat)
    (loan : Nat) : (loans.map ρ).contains (ρ loan) = loans.contains loan := by
  rw [Bool.eq_iff_iff]
  simp [injective.eq_iff]

mutual
theorem maskReturnedBorrows_rename (injective : Function.Injective ρ) (loans : Array Nat)
    (value : RuntimeValue) :
    maskReturnedBorrows (loans.map ρ) (value.rename ρ) =
      (maskReturnedBorrows loans value).rename ρ := by
  match value with
  | .borrow loan current =>
      rw [RuntimeValue.rename_borrow, maskReturnedBorrows, maskReturnedBorrows,
        Array.contains_map_rename injective]
      split <;> simp
  | .vector elements =>
      rw [RuntimeValue.rename_vector, maskReturnedBorrows, maskReturnedBorrows, Array.toList_map,
        maskReturnedBorrowList_rename injective loans elements.toList]
      simp
  | .tuple elements =>
      rw [RuntimeValue.rename_tuple, maskReturnedBorrows, maskReturnedBorrows, Array.toList_map,
        maskReturnedBorrowList_rename injective loans elements.toList]
      simp
  | .nominal source variant fields =>
      rw [RuntimeValue.rename_nominal, maskReturnedBorrows, maskReturnedBorrows, Array.toList_map,
        maskReturnedBorrowList_rename injective loans fields.toList]
      simp
  | .closure function mask instantiation captures =>
      rw [RuntimeValue.rename_closure, maskReturnedBorrows, maskReturnedBorrows, Array.toList_map,
        maskReturnedBorrowList_rename injective loans captures.toList]
      simp
  | .unit | .bool _ | .character _ | .integer _ | .address _ | .signer _ | .string _
  | .bytes _ | .loanHole _ => simp [maskReturnedBorrows]
termination_by sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); omega)

theorem maskReturnedBorrowList_rename (injective : Function.Injective ρ) (loans : Array Nat)
    (values : List RuntimeValue) :
    maskReturnedBorrowList (loans.map ρ) (values.map (·.rename ρ)) =
      (maskReturnedBorrowList loans values).map (·.rename ρ) := by
  match values with
  | [] => simp [maskReturnedBorrowList]
  | value :: rest =>
      rw [List.map_cons, maskReturnedBorrowList, maskReturnedBorrowList, List.map_cons,
        maskReturnedBorrows_rename injective loans value, maskReturnedBorrowList_rename injective loans rest]
termination_by sizeOf values
decreasing_by all_goals (simp_wf; omega)
end

mutual
theorem maskReturnedBorrows_within (loans : Array Nat) (value : RuntimeValue)
    (above : value.Within inside) : (maskReturnedBorrows loans value).Within inside := by
  match value with
  | .borrow loan current =>
      rw [maskReturnedBorrows]
      split
      · simp
      · exact above
  | .vector elements =>
      rw [maskReturnedBorrows]
      simp only [RuntimeValue.within_vector, List.mem_toArray] at above ⊢
      exact maskReturnedBorrowList_within loans elements.toList (fun value member =>
        above value (Array.mem_toList_iff.mp member))
  | .tuple elements =>
      rw [maskReturnedBorrows]
      simp only [RuntimeValue.within_tuple, List.mem_toArray] at above ⊢
      exact maskReturnedBorrowList_within loans elements.toList (fun value member =>
        above value (Array.mem_toList_iff.mp member))
  | .nominal source variant fields =>
      rw [maskReturnedBorrows]
      simp only [RuntimeValue.within_nominal, List.mem_toArray] at above ⊢
      exact maskReturnedBorrowList_within loans fields.toList (fun value member =>
        above value (Array.mem_toList_iff.mp member))
  | .closure function mask instantiation captures =>
      rw [maskReturnedBorrows]
      simp only [RuntimeValue.within_closure, List.mem_toArray] at above ⊢
      exact maskReturnedBorrowList_within loans captures.toList (fun value member =>
        above value (Array.mem_toList_iff.mp member))
  | .unit | .bool _ | .character _ | .integer _ | .address _ | .signer _ | .string _
  | .bytes _ | .loanHole _ => rw [maskReturnedBorrows]; exact above
termination_by sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); omega)

theorem maskReturnedBorrowList_within (loans : Array Nat) (values : List RuntimeValue)
    (above : ∀ value ∈ values, value.Within inside) :
    ∀ value ∈ maskReturnedBorrowList loans values, value.Within inside := by
  match values with
  | [] => intro value member; simp [maskReturnedBorrowList] at member
  | head :: rest =>
      rw [maskReturnedBorrowList]
      intro value member
      rcases List.mem_cons.mp member with rfl | member
      · exact maskReturnedBorrows_within loans head (above head List.mem_cons_self)
      · exact maskReturnedBorrowList_within loans rest
          (fun value member => above value (List.mem_cons_of_mem _ member)) value member
termination_by sizeOf values
decreasing_by all_goals (simp_wf; omega)
end

theorem returnedBorrowIds_rename (results : Array RuntimeValue) :
    returnedBorrowIds (results.map (·.rename ρ)) =
      (returnedBorrowIds results).map ρ := by
  unfold returnedBorrowIds
  rw [Array.foldl_map, ← Array.foldl_toList, ← Array.foldl_toList]
  conv => lhs; rw [show (#[] : Array Nat) = (#[] : Array Nat).map ρ
    from Array.map_empty.symm]
  generalize (#[] : Array Nat) = start
  induction results.toList generalizing start with
  | nil => rfl
  | cons result results ih =>
      simp only [List.foldl_cons]
      rw [outermostBorrows_rename result, Array.map_map, ← ih]
      simp [Function.comp_def, Array.map_append, Array.map_map]

theorem scalarFunctionResult_rename (results : Array RuntimeValue) :
    scalarFunctionResult (results.map (·.rename ρ)) = scalarFunctionResult results := by
  unfold scalarFunctionResult
  rw [Array.toList_map]
  rcases results.toList with _ | ⟨result, _ | ⟨second, rest⟩⟩
  · rfl
  · cases result <;> simp
  · simp

theorem exportReturnedFrameLoans_mirror
    (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂) {frame : RuntimeFrame}
    (frame_within : frame.Within inside) (results : Array RuntimeValue) :
    StateRenamed ρ offset frontier inside inert inert' (exportReturnedFrameLoans results frame state)
      (exportReturnedFrameLoans (results.map (·.rename ρ)) (frame.rename ρ) state₂) := by
  unfold exportReturnedFrameLoans
  rw [Array.isEmpty_map, scalarFunctionResult_rename, frameBorrows_rename, Array.isEmpty_map,
    returnedBorrowIds_rename]
  simp only [Array.isEmpty_map]
  split
  · exact exportFrameLoans_mirror shifted frame_within
  split
  · exact exportFrameLoans_mirror shifted frame_within
  split
  · exact shifted
  split
  · exact exportFrameLoans_mirror shifted frame_within
  have frame_eq : ({ frame.rename ρ with
      locals := (frame.rename ρ).locals.map
        (Option.map (maskReturnedBorrows ((returnedBorrowIds results).map ρ))) } :
        RuntimeFrame) =
      ({ frame with
        locals := frame.locals.map (Option.map (maskReturnedBorrows (returnedBorrowIds results))) } :
        RuntimeFrame).rename ρ := by
    simp [RuntimeFrame.rename, Array.map_map, Function.comp_def, Option.map_map,
      maskReturnedBorrows_rename shifted.renames.injective]
  rw [frame_eq]
  refine exportFrameLoans_mirror shifted
    { locals := fun slot member value value_eq => ?_
      activeLoans := frame_within.activeLoans
      loanLocations := frame_within.loanLocations }
  obtain ⟨original, original_member, rfl⟩ := Array.mem_map.mp member
  cases original with
  | none => cases value_eq
  | some original =>
      simp only [Option.map_some, Option.some.injEq] at value_eq
      subst value_eq
      exact maskReturnedBorrows_within _ original
        (frame_within.locals _ original_member original rfl)

@[simp] theorem Outcome.rename_returned (ρ : Nat → Nat) (values : Array RuntimeValue) :
    (Outcome.returned values).rename ρ = .returned (values.map (·.rename ρ)) := rfl
@[simp] theorem Outcome.rename_threw (ρ : Nat → Nat) (kind : ThrowKind) (values : Array RuntimeValue) :
    (Outcome.threw kind values).rename ρ = .threw kind (values.map (·.rename ρ)) := rfl

theorem finalizeFunctionState_mirror {initial initial₂ : RuntimeState}
    (initialShifted : StateRenamed ρ offset frontier inside inert inert' initial initial₂)
    (shifted : StateRenamed ρ offset frontier inside inert inert' state state₂)
    {unit : ValidatedUnit} (executable : ExecutableUnit unit) (profile : Profile)
    {frame : RuntimeFrame}
    (frame_within : frame.Within inside) (outcome : Outcome) :
    StateRenamed ρ offset frontier inside inert inert'
      (finalizeFunctionState executable profile initial state frame outcome)
      (finalizeFunctionState executable profile initial₂ state₂ (frame.rename ρ)
        (outcome.rename ρ)) := by
  cases outcome with
  | returned results =>
      exact exportReturnedFrameLoans_mirror shifted frame_within results
  | threw kind values =>
      simp only [Outcome.rename_threw, finalizeFunctionState]
      split
      · split
        · exact initialShifted
        · exact exportFrameLoans_mirror shifted frame_within
      · exact exportFrameLoans_mirror shifted frame_within

theorem initialLocals_map (f : RuntimeValue → RuntimeValue) (count : Nat)
    (arguments : Array RuntimeValue) :
    initialLocals count (arguments.map f) = (initialLocals count arguments).map (Option.map f) := by
  unfold initialLocals
  apply Array.ext
  · simp
  · intro index _ _
    simp only [Array.getElem_ofFn, Array.getElem_map, Array.size_map]
    split <;> simp

theorem parameterLoanLocations_rename (arguments : Array RuntimeValue) :
    parameterLoanLocations (arguments.map (·.rename ρ)) =
      (parameterLoanLocations arguments).map (renameEntry ρ) := by
  unfold parameterLoanLocations
  rw [Array.zipIdx_map, Array.filterMap_map, Array.map_filterMap]
  congr 1
  funext entry
  obtain ⟨value, index⟩ := entry
  cases value <;> simp

theorem initialFrame?_rename (declaration : FunctionDecl FunctionBody)
    (arguments : Array RuntimeValue) (typeInstantiation : Array (TypeId × TypeId)) :
    initialFrame? declaration (arguments.map (·.rename ρ)) typeInstantiation =
      (initialFrame? declaration arguments typeInstantiation).map (·.rename ρ) := by
  unfold initialFrame?
  simp only [Array.size_map]
  split
  · rfl
  split
  · rfl
  simp [RuntimeFrame.rename, initialLocals_map, parameterLoanLocations_rename]

theorem initialFrame?_within {β γ : Nat → Nat} (base : LoanBase β γ inside)
    {declaration : FunctionDecl FunctionBody}
    {arguments : Array RuntimeValue} {typeInstantiation : Array (TypeId × TypeId)}
    {frame : RuntimeFrame} (above : ∀ argument ∈ arguments, argument.Within inside)
    (initial : initialFrame? declaration arguments typeInstantiation = some frame) :
    frame.Within inside := by
  rw [← Array.rename_inverse_all base above, initialFrame?_rename] at initial
  obtain ⟨original, -, rfl⟩ := Option.map_eq_some_iff.mp initial
  exact base.within_frame original

theorem unpackFallthrough_rename (expected : Nat) (value : RuntimeValue) :
    unpackFallthrough expected (value.rename ρ) =
      (unpackFallthrough expected value).map (·.map (·.rename ρ)) := by
  unfold unpackFallthrough
  match expected with
  | 0 => cases value <;> simp
  | 1 => simp
  | count + 2 => cases value <;> simp

theorem finishControl?_rename (resultCount : Nat) (control : Control) :
    finishControl? resultCount (control.rename ρ) =
      (finishControl? resultCount control).map (·.rename ρ) := by
  cases control with
  | value value =>
      simp only [Control.rename, finishControl?, unpackFallthrough_rename]
      cases unpackFallthrough resultCount value <;> simp
  | return_ values =>
      simp only [Control.rename, finishControl?, Array.size_map]
      split <;> simp
  | throw_ kind values => simp [Control.rename, finishControl?]
  | break_ => simp [Control.rename, finishControl?]
  | continue_ => simp [Control.rename, finishControl?]

theorem finishControl?_within {β γ : Nat → Nat} (base : LoanBase β γ inside)
    {resultCount : Nat} {control : Control} {outcome : Outcome}
    (above : control.Within inside) (finish : finishControl? resultCount control = some outcome) :
    outcome.Within inside := by
  cases control with
  | value value =>
      simp only [finishControl?, Functor.map, Option.map_eq_some_iff] at finish
      obtain ⟨values, values_eq, rfl⟩ := finish
      rw [← base.rename_inverse value above, unpackFallthrough_rename] at values_eq
      obtain ⟨renamed, -, rfl⟩ := Option.map_eq_some_iff.mp values_eq
      intro value member
      obtain ⟨original, -, rfl⟩ := Array.mem_map.mp member
      exact base.within original
  | return_ values =>
      simp only [finishControl?] at finish
      split at finish
      · cases finish; exact above
      · cases finish
  | throw_ kind values => cases finish; exact above
  | break_ => cases finish
  | continue_ => cases finish

mutual
theorem RuntimeValue.holeFree?_rename (value : RuntimeValue) :
    (value.rename ρ).holeFree? = value.holeFree? := by
  match value with
  | .vector elements | .tuple elements =>
      simp only [RuntimeValue.rename_vector, RuntimeValue.rename_tuple]
      rw [RuntimeValue.holeFree?, RuntimeValue.holeFree?, Array.toList_map,
        RuntimeValue.holeFreeList?_rename elements.toList]
  | .nominal source variant fields =>
      rw [RuntimeValue.rename_nominal, RuntimeValue.holeFree?, RuntimeValue.holeFree?,
        Array.toList_map, RuntimeValue.holeFreeList?_rename fields.toList]
  | .closure function mask instantiation captures =>
      rw [RuntimeValue.rename_closure, RuntimeValue.holeFree?, RuntimeValue.holeFree?,
        Array.toList_map, RuntimeValue.holeFreeList?_rename captures.toList]
  | .borrow loan current => simp [RuntimeValue.holeFree?]
  | .loanHole loan => simp [RuntimeValue.holeFree?]
  | .unit | .bool _ | .character _ | .integer _ | .address _ | .signer _ | .string _
  | .bytes _ => simp [RuntimeValue.holeFree?]
termination_by sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); omega)

theorem RuntimeValue.holeFreeList?_rename (values : List RuntimeValue) :
    RuntimeValue.holeFreeList? (values.map (·.rename ρ)) =
      RuntimeValue.holeFreeList? values := by
  match values with
  | [] => simp [RuntimeValue.holeFreeList?]
  | value :: rest =>
      rw [List.map_cons, RuntimeValue.holeFreeList?, RuntimeValue.holeFreeList?,
        RuntimeValue.holeFree?_rename value, RuntimeValue.holeFreeList?_rename rest]
termination_by sizeOf values
decreasing_by all_goals (simp_wf; omega)
end

theorem Outcome.holeFree_rename (outcome : Outcome) :
    (outcome.rename ρ).holeFree = outcome.holeFree := by
  cases outcome with
  | returned values =>
      simp only [Outcome.rename_returned, Outcome.holeFree, Array.all_map]
      congr 1
      funext value
      simp [RuntimeValue.holeFree?_rename]
  | threw => rfl

end Frames

end VerexIR
