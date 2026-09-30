-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.LoanRenamingOps

/-!
# Primitive operations under loan renaming

The primitive vocabulary reads loan identities only through structural
equality and the value order, both unchanged by a shift: a primitive applied
to shifted arguments yields the shifted result
(`evaluatePrimitiveOperation?_shift`).
-/

theorem List.map_eraseIdx {α β : Type} (f : α → β) :
    ∀ (l : List α) (i : Nat), (l.eraseIdx i).map f = (l.map f).eraseIdx i
  | [], _ => rfl
  | _ :: _, 0 => rfl
  | a :: l, i + 1 => by simp [List.map_eraseIdx f l i]

theorem List.map_insertIdx {α β : Type} (f : α → β) (a : α) :
    ∀ (l : List α) (i : Nat), (l.insertIdx i a).map f = (l.map f).insertIdx i (f a)
  | _, 0 => rfl
  | [], _ + 1 => rfl
  | b :: l, i + 1 => by simp [List.map_insertIdx f a l i]

@[simp] theorem Array.map_eraseIdxIfInBounds {α β : Type} (f : α → β) (xs : Array α) (i : Nat) :
    (xs.eraseIdxIfInBounds i).map f = (xs.map f).eraseIdxIfInBounds i := by
  apply Array.toList_inj.mp
  simp [List.map_eraseIdx]

@[simp] theorem Array.map_insertIdxIfInBounds {α β : Type} (f : α → β) (xs : Array α) (i : Nat)
    (a : α) : (xs.insertIdxIfInBounds i a).map f = (xs.map f).insertIdxIfInBounds i (f a) := by
  unfold Array.insertIdxIfInBounds
  simp only [Array.size_map]
  split
  · apply Array.toList_inj.mp
    simp [Array.toList_insertIdx, List.map_insertIdx]
  · rfl

theorem Nat.compare_add_add_right (left right offset : Nat) :
    compare (left + offset) (right + offset) = compare left right := by
  simp only [Nat.compare_eq_ite_lt, Nat.add_lt_add_iff_right]

namespace LeanerIR

open SemanticOperations

theorem RuntimeValue.kindRank_shift (offset : Nat) (value : RuntimeValue) :
    (value.shift offset).kindRank = value.kindRank := by
  cases value <;> simp [RuntimeValue.kindRank]

mutual
theorem RuntimeValue.order_shift (rank : ValueRanks) (offset : Nat) (left right : RuntimeValue) :
    RuntimeValue.order rank (left.shift offset) (right.shift offset) =
      RuntimeValue.order rank left right := by
  rw [RuntimeValue.order, RuntimeValue.order, RuntimeValue.kindRank_shift,
    RuntimeValue.kindRank_shift, RuntimeValue.orderPayload_shift rank offset left right]
termination_by sizeOf left + sizeOf right + 1

theorem RuntimeValue.orderPayload_shift (rank : ValueRanks) (offset : Nat)
    (left right : RuntimeValue) :
    RuntimeValue.orderPayload rank (left.shift offset) (right.shift offset) =
      RuntimeValue.orderPayload rank left right := by
  cases left <;> cases right <;>
    simp only [RuntimeValue.shift_vector, RuntimeValue.shift_tuple, RuntimeValue.shift_nominal,
      RuntimeValue.shift_closure, RuntimeValue.shift_borrow, RuntimeValue.shift_loanHole,
      RuntimeValue.shift_unit, RuntimeValue.shift_bool, RuntimeValue.shift_character,
      RuntimeValue.shift_integer, RuntimeValue.shift_address, RuntimeValue.shift_signer,
      RuntimeValue.shift_string, RuntimeValue.shift_bytes, RuntimeValue.orderPayload,
      Array.toList_map]
  all_goals first
    | exact RuntimeValue.orderList_shift rank offset _ _
    | rw [RuntimeValue.orderList_shift rank offset]
    | rw [RuntimeValue.order_shift rank offset, Nat.compare_add_add_right]
    | exact Nat.compare_add_add_right _ _ _
termination_by sizeOf left + sizeOf right
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); omega)

theorem RuntimeValue.orderList_shift (rank : ValueRanks) (offset : Nat)
    (lefts rights : List RuntimeValue) :
    RuntimeValue.orderList rank (lefts.map (·.shift offset)) (rights.map (·.shift offset)) =
      RuntimeValue.orderList rank lefts rights := by
  match lefts, rights with
  | [], [] => simp [RuntimeValue.orderList]
  | [], _ :: _ => simp [RuntimeValue.orderList]
  | _ :: _, [] => simp [RuntimeValue.orderList]
  | left :: lefts, right :: rights =>
      simp only [List.map_cons, RuntimeValue.orderList]
      rw [RuntimeValue.order_shift rank offset left right,
        RuntimeValue.orderList_shift rank offset lefts rights]
termination_by sizeOf lefts + sizeOf rights
decreasing_by all_goals (simp_wf; omega)
end

/-- A primitive result with the loans of its values raised. -/
def shiftResult (offset : Nat) :
    Except (ThrowKind × Array RuntimeValue) RuntimeValue →
      Except (ThrowKind × Array RuntimeValue) RuntimeValue
  | .ok value => .ok (value.shift offset)
  | .error (kind, values) => .error (kind, values.map (·.shift offset))

@[simp] theorem shiftResult_ok (offset : Nat) (value : RuntimeValue) :
    shiftResult offset (.ok value) = .ok (value.shift offset) := rfl
@[simp] theorem shiftResult_error (offset : Nat) (kind : ThrowKind) (values : Array RuntimeValue) :
    shiftResult offset (.error (kind, values)) = .error (kind, values.map (·.shift offset)) := rfl

@[simp] theorem modularInteger_shift (offset : Nat) (resultType : Ty) (value : Int) :
    Option.map (shiftResult offset ∘ Except.ok) (modularInteger resultType value) =
      Option.map Except.ok (modularInteger resultType value) := by
  unfold modularInteger
  split <;> simp
  split <;> simp

theorem modularInteger_shift_of_eq {offset : Nat} {resultType : Ty} {value : Int}
    {result : RuntimeValue} (result_eq : modularInteger resultType value = some result) :
    result.shift offset = result := by
  unfold modularInteger at result_eq
  split at result_eq
  · split at result_eq
    · cases result_eq
    · simp only [Option.some.injEq] at result_eq
      subst result_eq
      exact RuntimeValue.shift_integer (offset := offset) _
  · cases result_eq

@[simp] theorem modularInteger_bind_shift (offset : Nat) (resultType : Ty) (value : Int) :
    ((modularInteger resultType value).bind fun result =>
        some (Except.ok (result.shift offset)) :
          Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue)) =
      (modularInteger resultType value).bind fun result => some (Except.ok result) :=
  Option.bind_congr fun _ result_eq => by rw [modularInteger_shift_of_eq result_eq]

@[simp] theorem checkedInteger_shift (offset : Nat) (failure : ThrowKind) (resultType : Ty)
    (value : Int) :
    shiftResult offset (checkedInteger failure resultType value) =
      checkedInteger failure resultType value := by
  unfold checkedInteger
  split
  · split <;> simp
  · simp

theorem checkedUnaryInteger_shift (offset : Nat) (failure : ThrowKind) (resultType : Ty)
    (arguments : Array RuntimeValue) (operation : Int → Int) :
    checkedUnaryInteger failure resultType (arguments.map (·.shift offset)) operation =
      (checkedUnaryInteger failure resultType arguments operation).map (shiftResult offset) := by
  unfold checkedUnaryInteger
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, rest⟩⟩
  · simp
  · cases a <;> simp
  · simp

theorem checkedBinaryInteger_shift (offset : Nat) (failure : ThrowKind) (resultType : Ty)
    (arguments : Array RuntimeValue) (operation : Int → Int → Int) :
    checkedBinaryInteger failure resultType (arguments.map (·.shift offset)) operation =
      (checkedBinaryInteger failure resultType arguments operation).map (shiftResult offset) := by
  unfold checkedBinaryInteger
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  · simp
  · simp
  · cases a <;> (try (simp; done)) <;> cases b <;> simp
  · simp

theorem modularUnaryInteger_shift (offset : Nat) (resultType : Ty)
    (arguments : Array RuntimeValue) (operation : Int → Int) :
    modularUnaryInteger resultType (arguments.map (·.shift offset)) operation =
      (modularUnaryInteger resultType arguments operation).map (shiftResult offset) := by
  unfold modularUnaryInteger
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, rest⟩⟩
  · simp
  · cases a <;> simp
  · simp

theorem modularBinaryInteger_shift (offset : Nat) (resultType : Ty)
    (arguments : Array RuntimeValue) (operation : Int → Int → Int) :
    modularBinaryInteger resultType (arguments.map (·.shift offset)) operation =
      (modularBinaryInteger resultType arguments operation).map (shiftResult offset) := by
  unfold modularBinaryInteger
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  · simp
  · simp
  · cases a <;> (try (simp; done)) <;> cases b <;> simp
  · simp

theorem overflowingBinaryInteger_shift (offset : Nat) (ns : Validation.ValidatedNamespace)
    (width : Option Nat) (resultType : Ty) (arguments : Array RuntimeValue)
    (operation : Int → Int → Int) :
    overflowingBinaryInteger ns width resultType (arguments.map (·.shift offset)) operation =
      (overflowingBinaryInteger ns width resultType arguments operation).map
        (shiftResult offset) := by
  unfold overflowingBinaryInteger
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp <;> split <;> (try split) <;> simp; done)) <;> cases b)
  all_goals (simp <;> split <;> (try split) <;> try simp)
  simp only [Function.comp_def, Option.map_bind]
  refine Option.bind_congr fun valueType _ => Option.bind_congr fun overflowType _ => ?_
  split
  · simp only [Function.comp_def, Option.map_bind, Option.map_some]
    refine Option.bind_congr fun bounds _ => Option.bind_congr fun value value_eq => ?_
    simp [modularInteger_shift_of_eq value_eq]
  · rfl

theorem bitwiseBinaryInteger_shift (offset : Nat) (resultType : Ty)
    (arguments : Array RuntimeValue) (operation : Nat → Nat → Nat) :
    bitwiseBinaryInteger resultType (arguments.map (·.shift offset)) operation =
      (bitwiseBinaryInteger resultType arguments operation).map (shiftResult offset) := by
  unfold bitwiseBinaryInteger
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b)
  all_goals try (simp; done)
  simp only [List.map_cons, List.map_nil, RuntimeValue.shift_integer, Option.bind_eq_bind,
    Option.map_bind, Function.comp_def]
  refine Option.bind_congr fun _ _ => Option.bind_congr fun _ _ =>
    Option.bind_congr fun value value_eq => ?_
  simp [modularInteger_shift_of_eq value_eq]

theorem bitwiseBinary_shift (offset : Nat) (resultType : Ty) (arguments : Array RuntimeValue)
    (integerOperation : Nat → Nat → Nat) (booleanOperation : Bool → Bool → Bool) :
    bitwiseBinary resultType (arguments.map (·.shift offset)) integerOperation booleanOperation =
      (bitwiseBinary resultType arguments integerOperation booleanOperation).map
        (shiftResult offset) := by
  have integer := bitwiseBinaryInteger_shift offset resultType arguments integerOperation
  unfold bitwiseBinary
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp [integer]; done)) <;> cases b)
  all_goals simp [integer]

theorem bitwiseNotInteger_shift (offset : Nat) (resultType : Ty)
    (arguments : Array RuntimeValue) :
    bitwiseNotInteger resultType (arguments.map (·.shift offset)) =
      (bitwiseNotInteger resultType arguments).map (shiftResult offset) := by
  unfold bitwiseNotInteger
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, rest⟩⟩
  all_goals try cases a
  all_goals (simp <;> split <;> (try split) <;> simp [Function.comp_def])

theorem checkedShiftInteger_shift (offset : Nat) (failure : ThrowKind) (left : Bool)
    (resultType : Ty) (arguments : Array RuntimeValue) :
    checkedShiftInteger failure left resultType (arguments.map (·.shift offset)) =
      (checkedShiftInteger failure left resultType arguments).map (shiftResult offset) := by
  unfold checkedShiftInteger
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp <;> split <;> (try split) <;> simp; done)) <;> cases b)
  all_goals (simp <;> split <;> (try split) <;> try simp)
  all_goals (split <;> simp [Function.comp_def])

theorem compareOrdered_shift (offset : Nat) (arguments : Array RuntimeValue)
    (integerRelation : Int → Int → Bool) (booleanRelation : Bool → Bool → Bool)
    (characterRelation : Nat → Nat → Bool) :
    compareOrdered (arguments.map (·.shift offset)) integerRelation booleanRelation
        characterRelation =
      (compareOrdered arguments integerRelation booleanRelation characterRelation).map
        (shiftResult offset) := by
  unfold compareOrdered
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b)
  all_goals simp

theorem equalValues?_shift (offset : Nat) (arguments : Array RuntimeValue) :
    equalValues? (arguments.map (·.shift offset)) =
      (equalValues? arguments).map (shiftResult offset) := by
  unfold equalValues?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩ <;>
    simp [RuntimeValue.shift_beq]

theorem notEqualValues?_shift (offset : Nat) (arguments : Array RuntimeValue) :
    notEqualValues? (arguments.map (·.shift offset)) =
      (notEqualValues? arguments).map (shiftResult offset) := by
  unfold notEqualValues?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩ <;>
    simp [bne, RuntimeValue.shift_beq]

theorem divideIntegers?_shift (offset : Nat) (resultType : Ty) (arguments : Array RuntimeValue) :
    divideIntegers? resultType (arguments.map (·.shift offset)) =
      (divideIntegers? resultType arguments).map (shiftResult offset) := by
  unfold divideIntegers?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b)
  all_goals simp [Function.comp_def]

theorem checkedDivideIntegers?_shift (offset : Nat) (failure : ThrowKind) (resultType : Ty)
    (arguments : Array RuntimeValue) :
    checkedDivideIntegers? failure resultType (arguments.map (·.shift offset)) =
      (checkedDivideIntegers? failure resultType arguments).map (shiftResult offset) := by
  unfold checkedDivideIntegers?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b)
  all_goals try (simp; done)
  rename_i right
  rcases right with ⟨_ | _⟩ | _
  all_goals simp only [List.map_cons, List.map_nil, RuntimeValue.shift_integer]
  all_goals simp [Function.comp_def]

theorem moduloIntegers?_shift (offset : Nat) (resultType : Ty) (arguments : Array RuntimeValue) :
    moduloIntegers? resultType (arguments.map (·.shift offset)) =
      (moduloIntegers? resultType arguments).map (shiftResult offset) := by
  unfold moduloIntegers?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b)
  all_goals simp [Function.comp_def]

theorem checkedModuloIntegers?_shift (offset : Nat) (failure : ThrowKind) (resultType : Ty)
    (arguments : Array RuntimeValue) :
    checkedModuloIntegers? failure resultType (arguments.map (·.shift offset)) =
      (checkedModuloIntegers? failure resultType arguments).map (shiftResult offset) := by
  unfold checkedModuloIntegers?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b)
  all_goals try (simp; done)
  rename_i right
  rcases right with ⟨_ | _⟩ | _
  all_goals simp only [List.map_cons, List.map_nil, RuntimeValue.shift_integer]
  · simp
  all_goals
    simp only [Option.bind_eq_bind, Option.pure_def, Option.map_bind, Function.comp_def]
    refine Option.bind_congr fun _ _ => ?_
    split
    · next error error_eq =>
        rw [Option.map_some, ← error_eq, checkedInteger_shift]
    · simp

theorem insertVector?_shift (offset : Nat) (arguments : Array RuntimeValue) :
    insertVector? (arguments.map (·.shift offset)) =
      (insertVector? arguments).map (shiftResult offset) := by
  unfold insertVector?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, _ | ⟨d, rest⟩⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b)
  all_goals simp
  all_goals split <;> simp

theorem removeVector?_shift (offset : Nat) (arguments : Array RuntimeValue) :
    removeVector? (arguments.map (·.shift offset)) =
      (removeVector? arguments).map (shiftResult offset) := by
  unfold removeVector?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b)
  all_goals try (simp; done)
  rename_i elements index
  simp only [List.map_cons, List.map_nil, RuntimeValue.shift_vector, RuntimeValue.shift_integer]
  split
  · simp
  · rw [Array.getElem?_map]
    cases elements[index.toNat]? <;> simp

theorem swapVector?_shift (offset : Nat) (arguments : Array RuntimeValue) :
    swapVector? (arguments.map (·.shift offset)) =
      (swapVector? arguments).map (shiftResult offset) := by
  unfold swapVector?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, _ | ⟨d, rest⟩⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b <;> (try (simp; done)) <;> cases c)
  all_goals try (simp; done)
  rename_i elements left right
  simp only [List.map_cons, List.map_nil, RuntimeValue.shift_vector, RuntimeValue.shift_integer,
    Array.size_map]
  split
  · simp
  · rw [Array.getElem?_map, Array.getElem?_map]
    cases elements[left.toNat]? <;> cases elements[right.toNat]? <;> simp

theorem reverseSliceVector?_shift (offset : Nat) (arguments : Array RuntimeValue) :
    reverseSliceVector? (arguments.map (·.shift offset)) =
      (reverseSliceVector? arguments).map (shiftResult offset) := by
  unfold reverseSliceVector?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, _ | ⟨d, rest⟩⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b <;> (try (simp; done)) <;> cases c)
  all_goals simp [reverseVectorRange_map]
  all_goals split <;> simp

theorem checkVectorIndex?_shift (offset : Nat) (failure : ThrowKind)
    (arguments : Array RuntimeValue) :
    checkVectorIndex? failure (arguments.map (·.shift offset)) =
      (checkVectorIndex? failure arguments).map (shiftResult offset) := by
  unfold checkVectorIndex?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b)
  all_goals simp
  all_goals split <;> simp

theorem containsVector?_shift (offset : Nat) (arguments : Array RuntimeValue) :
    containsVector? (arguments.map (·.shift offset)) =
      (containsVector? arguments).map (shiftResult offset) := by
  unfold containsVector?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a)
  all_goals simp [findVectorIndex_map _ (RuntimeValue.shift_beq offset)]

theorem indexOfVector?_shift (offset : Nat) (indexType : Ty) (arguments : Array RuntimeValue) :
    indexOfVector? indexType (arguments.map (·.shift offset)) =
      (indexOfVector? indexType arguments).map (shiftResult offset) := by
  unfold indexOfVector?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a)
  all_goals simp [findVectorIndex_map _ (RuntimeValue.shift_beq offset)]
  split
  · simp
  · simp only [Option.map_bind, Function.comp_def, Option.map_some, shiftResult_ok]
    refine Option.bind_congr fun index index_eq => ?_
    simp [modularInteger_shift_of_eq index_eq]

theorem destroyEmptyVector?_shift (offset : Nat) (arguments : Array RuntimeValue) :
    destroyEmptyVector? (arguments.map (·.shift offset)) =
      (destroyEmptyVector? arguments).map (shiftResult offset) := by
  unfold destroyEmptyVector?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, rest⟩⟩
  all_goals try (cases a)
  all_goals simp
  all_goals split <;> simp

theorem concatVector?_shift (offset : Nat) (arguments : Array RuntimeValue) :
    concatVector? (arguments.map (·.shift offset)) =
      (concatVector? arguments).map (shiftResult offset) := by
  unfold concatVector?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b)
  all_goals simp

theorem sliceVector?_shift (offset : Nat) (arguments : Array RuntimeValue) :
    sliceVector? (arguments.map (·.shift offset)) =
      (sliceVector? arguments).map (shiftResult offset) := by
  unfold sliceVector?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, _ | ⟨d, rest⟩⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b <;> (try (simp; done)) <;> cases c)
  all_goals simp
  all_goals split <;> simp

theorem shiftInteger_shift (offset : Nat) (left : Bool) (resultType : Ty)
    (arguments : Array RuntimeValue) :
    shiftInteger left resultType (arguments.map (·.shift offset)) =
      (shiftInteger left resultType arguments).map (shiftResult offset) := by
  unfold shiftInteger
  rw [checkedShiftInteger_shift]
  rcases checkedShiftInteger .panic left resultType arguments with _ | ⟨_, _⟩ | _ <;> simp

theorem evaluatePrimitiveOperation?_shift (offset : Nat) (ns : Validation.ValidatedNamespace)
    (resultType : TypeId) (operation : PrimitiveOperation) (arguments : Array RuntimeValue)
    (width : Option Nat) :
    evaluatePrimitiveOperation? ns resultType operation (arguments.map (·.shift offset)) width =
      (evaluatePrimitiveOperation? ns resultType operation arguments width).map
        (shiftResult offset) := by
  unfold evaluatePrimitiveOperation?
  simp only [Option.bind_eq_bind, Option.map_bind]
  refine Option.bind_congr fun resolved _ => ?_
  cases operation
  all_goals try simp only [Function.comp_apply, checkedUnaryInteger_shift,
    checkedBinaryInteger_shift, modularUnaryInteger_shift, modularBinaryInteger_shift,
    overflowingBinaryInteger_shift, bitwiseBinary_shift, bitwiseNotInteger_shift,
    checkedShiftInteger_shift, compareOrdered_shift, equalValues?_shift, notEqualValues?_shift,
    divideIntegers?_shift, checkedDivideIntegers?_shift, moduloIntegers?_shift,
    checkedModuloIntegers?_shift, insertVector?_shift, removeVector?_shift, swapVector?_shift,
    reverseSliceVector?_shift, checkVectorIndex?_shift, containsVector?_shift,
    destroyEmptyVector?_shift, concatVector?_shift, sliceVector?_shift, shiftInteger_shift]
  case tuple | vector | range | implies | equivalent | identical => simp
  case indexOfVector =>
    split
    · split
      · simp only [Option.map_bind]
        exact Option.bind_congr fun indexType _ => indexOfVector?_shift offset indexType arguments
      · rfl
    · rfl
  all_goals rw [Array.toList_map]
  case compare =>
    rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩ <;>
      simp [RuntimeValue.order_shift]
  case copyValue | moveValue =>
    rcases arguments.toList with _ | ⟨a, _ | ⟨b, rest⟩⟩ <;> simp
  case repeatVector =>
    rcases arguments.toList with _ | ⟨a, _ | ⟨b, rest⟩⟩
    all_goals simp only [List.map_cons, List.map_nil]
    all_goals first | rfl | (split <;> split <;> simp_all)
  case length =>
    rcases arguments.toList with _ | ⟨a, _ | ⟨b, rest⟩⟩
    all_goals try cases a
    all_goals simp
    all_goals split <;> simp
  case index =>
    rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
    all_goals try (cases a <;> (try (simp; done)) <;> cases b)
    all_goals try (simp; done)
    rename_i elements index
    simp only [List.map_cons, List.map_nil, RuntimeValue.shift_vector, RuntimeValue.shift_integer]
    split
    · simp
    · rw [Array.getElem?_map]
      cases elements[index.toNat]? <;> simp
  case cast | checkedCast | signerAddress | logicalNot =>
    rcases arguments.toList with _ | ⟨a, _ | ⟨b, rest⟩⟩
    all_goals try cases a
    all_goals simp
    all_goals split <;> (try split) <;> simp
  all_goals rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b)
  all_goals try (simp; done)

theorem evaluatePrimitiveOperation?_above {frontier : Nat} {ns : Validation.ValidatedNamespace}
    {resultType : TypeId} {operation : PrimitiveOperation} {arguments : Array RuntimeValue}
    {width : Option Nat} {result : Except (ThrowKind × Array RuntimeValue) RuntimeValue}
    (above : ∀ argument ∈ arguments, argument.Above frontier)
    (evaluate : evaluatePrimitiveOperation? ns resultType operation arguments width = some result) :
    (∀ value, result = .ok value → value.Above frontier) ∧
      (∀ kind values, result = .error (kind, values) → ∀ value ∈ values, value.Above frontier) := by
  rw [← Array.shift_unshift_all above, evaluatePrimitiveOperation?_shift] at evaluate
  obtain ⟨base, -, rfl⟩ := Option.map_eq_some_iff.mp evaluate
  refine ⟨fun value same => ?_, fun kind values same => ?_⟩
  · cases base with
    | ok base =>
        simp only [shiftResult_ok, Except.ok.injEq] at same
        exact same ▸ RuntimeValue.above_shift_self frontier base
    | error base => cases same
  · cases base with
    | ok base => cases same
    | error base =>
        obtain ⟨baseKind, baseValues⟩ := base
        simp only [shiftResult_error, Except.error.injEq, Prod.mk.injEq] at same
        obtain ⟨-, rfl⟩ := same
        intro value member
        obtain ⟨original, -, rfl⟩ := Array.mem_map.mp member
        exact RuntimeValue.above_shift_self frontier original

end LeanerIR
