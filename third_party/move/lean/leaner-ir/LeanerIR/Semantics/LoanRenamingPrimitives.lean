-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.LoanRenamingOps

/-!
# Primitive operations under loan renaming

The primitive vocabulary compares values only as they are observed
(`RuntimeValue.observe`), which no renaming of loans changes: a primitive
applied to renamed arguments yields the renamed result
(`evaluatePrimitiveOperation?_rename`).
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

namespace LeanerIR

variable {inside : Nat → Prop}

open SemanticOperations

/-- A primitive result with the loans of its values renamed. -/
def renameResult (ρ : Nat → Nat) :
    Except (ThrowKind × Array RuntimeValue) RuntimeValue →
      Except (ThrowKind × Array RuntimeValue) RuntimeValue
  | .ok value => .ok (value.rename ρ)
  | .error (kind, values) => .error (kind, values.map (·.rename ρ))

@[simp] theorem renameResult_ok (ρ : Nat → Nat) (value : RuntimeValue) :
    renameResult ρ (.ok value) = .ok (value.rename ρ) := rfl
@[simp] theorem renameResult_error (ρ : Nat → Nat) (kind : ThrowKind) (values : Array RuntimeValue) :
    renameResult ρ (.error (kind, values)) = .error (kind, values.map (·.rename ρ)) := rfl

@[simp] theorem modularInteger_rename (ρ : Nat → Nat) (resultType : Ty) (value : Int) :
    Option.map (renameResult ρ ∘ Except.ok) (modularInteger resultType value) =
      Option.map Except.ok (modularInteger resultType value) := by
  unfold modularInteger
  split <;> simp
  split <;> simp

theorem modularInteger_rename_of_eq {ρ : Nat → Nat} {resultType : Ty} {value : Int}
    {result : RuntimeValue} (result_eq : modularInteger resultType value = some result) :
    result.rename ρ = result := by
  unfold modularInteger at result_eq
  split at result_eq
  · split at result_eq
    · cases result_eq
    · simp only [Option.some.injEq] at result_eq
      subst result_eq
      exact RuntimeValue.rename_integer (ρ := ρ) _
  · cases result_eq

@[simp] theorem modularInteger_bind_rename (ρ : Nat → Nat) (resultType : Ty) (value : Int) :
    ((modularInteger resultType value).bind fun result =>
        some (Except.ok (result.rename ρ)) :
          Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue)) =
      (modularInteger resultType value).bind fun result => some (Except.ok result) :=
  Option.bind_congr fun _ result_eq => by rw [modularInteger_rename_of_eq result_eq]

@[simp] theorem checkedInteger_rename (ρ : Nat → Nat) (failure : ThrowKind) (resultType : Ty)
    (value : Int) :
    renameResult ρ (checkedInteger failure resultType value) =
      checkedInteger failure resultType value := by
  unfold checkedInteger
  split
  · split <;> simp
  · simp

theorem checkedUnaryInteger_rename (ρ : Nat → Nat) (failure : ThrowKind) (resultType : Ty)
    (arguments : Array RuntimeValue) (operation : Int → Int) :
    checkedUnaryInteger failure resultType (arguments.map (·.rename ρ)) operation =
      (checkedUnaryInteger failure resultType arguments operation).map (renameResult ρ) := by
  unfold checkedUnaryInteger
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, rest⟩⟩
  · simp
  · cases a <;> simp
  · simp

theorem checkedBinaryInteger_rename (ρ : Nat → Nat) (failure : ThrowKind) (resultType : Ty)
    (arguments : Array RuntimeValue) (operation : Int → Int → Int) :
    checkedBinaryInteger failure resultType (arguments.map (·.rename ρ)) operation =
      (checkedBinaryInteger failure resultType arguments operation).map (renameResult ρ) := by
  unfold checkedBinaryInteger
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  · simp
  · simp
  · cases a <;> (try (simp; done)) <;> cases b <;> simp
  · simp

theorem modularUnaryInteger_rename (ρ : Nat → Nat) (resultType : Ty)
    (arguments : Array RuntimeValue) (operation : Int → Int) :
    modularUnaryInteger resultType (arguments.map (·.rename ρ)) operation =
      (modularUnaryInteger resultType arguments operation).map (renameResult ρ) := by
  unfold modularUnaryInteger
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, rest⟩⟩
  · simp
  · cases a <;> simp
  · simp

theorem modularBinaryInteger_rename (ρ : Nat → Nat) (resultType : Ty)
    (arguments : Array RuntimeValue) (operation : Int → Int → Int) :
    modularBinaryInteger resultType (arguments.map (·.rename ρ)) operation =
      (modularBinaryInteger resultType arguments operation).map (renameResult ρ) := by
  unfold modularBinaryInteger
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  · simp
  · simp
  · cases a <;> (try (simp; done)) <;> cases b <;> simp
  · simp

theorem overflowingBinaryInteger_rename (ρ : Nat → Nat) (ns : Validation.ValidatedNamespace)
    (width : Option Nat) (resultType : Ty) (arguments : Array RuntimeValue)
    (operation : Int → Int → Int) :
    overflowingBinaryInteger ns width resultType (arguments.map (·.rename ρ)) operation =
      (overflowingBinaryInteger ns width resultType arguments operation).map
        (renameResult ρ) := by
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
    simp [modularInteger_rename_of_eq value_eq]
  · rfl

theorem bitwiseBinaryInteger_rename (ρ : Nat → Nat) (resultType : Ty)
    (arguments : Array RuntimeValue) (operation : Nat → Nat → Nat) :
    bitwiseBinaryInteger resultType (arguments.map (·.rename ρ)) operation =
      (bitwiseBinaryInteger resultType arguments operation).map (renameResult ρ) := by
  unfold bitwiseBinaryInteger
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b)
  all_goals try (simp; done)
  simp only [List.map_cons, List.map_nil, RuntimeValue.rename_integer, Option.bind_eq_bind,
    Option.map_bind, Function.comp_def]
  refine Option.bind_congr fun _ _ => Option.bind_congr fun _ _ =>
    Option.bind_congr fun value value_eq => ?_
  simp [modularInteger_rename_of_eq value_eq]

theorem bitwiseBinary_rename (ρ : Nat → Nat) (resultType : Ty) (arguments : Array RuntimeValue)
    (integerOperation : Nat → Nat → Nat) (booleanOperation : Bool → Bool → Bool) :
    bitwiseBinary resultType (arguments.map (·.rename ρ)) integerOperation booleanOperation =
      (bitwiseBinary resultType arguments integerOperation booleanOperation).map
        (renameResult ρ) := by
  have integer := bitwiseBinaryInteger_rename ρ resultType arguments integerOperation
  unfold bitwiseBinary
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp [integer]; done)) <;> cases b)
  all_goals simp [integer]

theorem bitwiseNotInteger_rename (ρ : Nat → Nat) (resultType : Ty)
    (arguments : Array RuntimeValue) :
    bitwiseNotInteger resultType (arguments.map (·.rename ρ)) =
      (bitwiseNotInteger resultType arguments).map (renameResult ρ) := by
  unfold bitwiseNotInteger
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, rest⟩⟩
  all_goals try cases a
  all_goals (simp <;> split <;> (try split) <;> simp [Function.comp_def])

theorem checkedShiftInteger_rename (ρ : Nat → Nat) (failure : ThrowKind) (left : Bool)
    (resultType : Ty) (arguments : Array RuntimeValue) :
    checkedShiftInteger failure left resultType (arguments.map (·.rename ρ)) =
      (checkedShiftInteger failure left resultType arguments).map (renameResult ρ) := by
  unfold checkedShiftInteger
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp <;> split <;> (try split) <;> simp; done)) <;> cases b)
  all_goals (simp <;> split <;> (try split) <;> try simp)
  all_goals (split <;> simp [Function.comp_def])

theorem compareOrdered_rename (ρ : Nat → Nat) (arguments : Array RuntimeValue)
    (integerRelation : Int → Int → Bool) (booleanRelation : Bool → Bool → Bool)
    (characterRelation : Nat → Nat → Bool) :
    compareOrdered (arguments.map (·.rename ρ)) integerRelation booleanRelation
        characterRelation =
      (compareOrdered arguments integerRelation booleanRelation characterRelation).map
        (renameResult ρ) := by
  unfold compareOrdered
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b)
  all_goals simp

theorem equalValues?_rename (ρ : Nat → Nat) (arguments : Array RuntimeValue) :
    equalValues? (arguments.map (·.rename ρ)) =
      (equalValues? arguments).map (renameResult ρ) := by
  unfold equalValues?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩ <;>
    simp [RuntimeValue.observe_rename]

theorem notEqualValues?_rename (ρ : Nat → Nat) (arguments : Array RuntimeValue) :
    notEqualValues? (arguments.map (·.rename ρ)) =
      (notEqualValues? arguments).map (renameResult ρ) := by
  unfold notEqualValues?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩ <;>
    simp [bne, RuntimeValue.observe_rename]

theorem divideIntegers?_rename (ρ : Nat → Nat) (resultType : Ty) (arguments : Array RuntimeValue) :
    divideIntegers? resultType (arguments.map (·.rename ρ)) =
      (divideIntegers? resultType arguments).map (renameResult ρ) := by
  unfold divideIntegers?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b)
  all_goals simp [Function.comp_def]

theorem checkedDivideIntegers?_rename (ρ : Nat → Nat) (failure : ThrowKind) (resultType : Ty)
    (arguments : Array RuntimeValue) :
    checkedDivideIntegers? failure resultType (arguments.map (·.rename ρ)) =
      (checkedDivideIntegers? failure resultType arguments).map (renameResult ρ) := by
  unfold checkedDivideIntegers?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b)
  all_goals try (simp; done)
  rename_i right
  rcases right with ⟨_ | _⟩ | _
  all_goals simp only [List.map_cons, List.map_nil, RuntimeValue.rename_integer]
  all_goals simp [Function.comp_def]

theorem moduloIntegers?_rename (ρ : Nat → Nat) (resultType : Ty) (arguments : Array RuntimeValue) :
    moduloIntegers? resultType (arguments.map (·.rename ρ)) =
      (moduloIntegers? resultType arguments).map (renameResult ρ) := by
  unfold moduloIntegers?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b)
  all_goals simp [Function.comp_def]

theorem checkedModuloIntegers?_rename (ρ : Nat → Nat) (failure : ThrowKind) (resultType : Ty)
    (arguments : Array RuntimeValue) :
    checkedModuloIntegers? failure resultType (arguments.map (·.rename ρ)) =
      (checkedModuloIntegers? failure resultType arguments).map (renameResult ρ) := by
  unfold checkedModuloIntegers?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b)
  all_goals try (simp; done)
  rename_i right
  rcases right with ⟨_ | _⟩ | _
  all_goals simp only [List.map_cons, List.map_nil, RuntimeValue.rename_integer]
  · simp
  all_goals
    simp only [Option.bind_eq_bind, Option.pure_def, Option.map_bind, Function.comp_def]
    refine Option.bind_congr fun _ _ => ?_
    split
    · next error error_eq =>
        rw [Option.map_some, ← error_eq, checkedInteger_rename]
    · simp

theorem insertVector?_rename (ρ : Nat → Nat) (arguments : Array RuntimeValue) :
    insertVector? (arguments.map (·.rename ρ)) =
      (insertVector? arguments).map (renameResult ρ) := by
  unfold insertVector?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, _ | ⟨d, rest⟩⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b)
  all_goals simp
  all_goals split <;> simp

theorem removeVector?_rename (ρ : Nat → Nat) (arguments : Array RuntimeValue) :
    removeVector? (arguments.map (·.rename ρ)) =
      (removeVector? arguments).map (renameResult ρ) := by
  unfold removeVector?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b)
  all_goals try (simp; done)
  rename_i elements index
  simp only [List.map_cons, List.map_nil, RuntimeValue.rename_vector, RuntimeValue.rename_integer]
  split
  · simp
  · rw [Array.getElem?_map]
    cases elements[index.toNat]? <;> simp

theorem swapVector?_rename (ρ : Nat → Nat) (arguments : Array RuntimeValue) :
    swapVector? (arguments.map (·.rename ρ)) =
      (swapVector? arguments).map (renameResult ρ) := by
  unfold swapVector?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, _ | ⟨d, rest⟩⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b <;> (try (simp; done)) <;> cases c)
  all_goals try (simp; done)
  rename_i elements left right
  simp only [List.map_cons, List.map_nil, RuntimeValue.rename_vector, RuntimeValue.rename_integer,
    Array.size_map]
  split
  · simp
  · rw [Array.getElem?_map, Array.getElem?_map]
    cases elements[left.toNat]? <;> cases elements[right.toNat]? <;> simp

theorem reverseSliceVector?_rename (ρ : Nat → Nat) (arguments : Array RuntimeValue) :
    reverseSliceVector? (arguments.map (·.rename ρ)) =
      (reverseSliceVector? arguments).map (renameResult ρ) := by
  unfold reverseSliceVector?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, _ | ⟨d, rest⟩⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b <;> (try (simp; done)) <;> cases c)
  all_goals simp [reverseVectorRange_map]
  all_goals split <;> simp

theorem checkVectorIndex?_rename (ρ : Nat → Nat) (failure : ThrowKind)
    (arguments : Array RuntimeValue) :
    checkVectorIndex? failure (arguments.map (·.rename ρ)) =
      (checkVectorIndex? failure arguments).map (renameResult ρ) := by
  unfold checkVectorIndex?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b)
  all_goals simp
  all_goals split <;> simp

theorem containsVector?_rename (ρ : Nat → Nat) (arguments : Array RuntimeValue) :
    containsVector? (arguments.map (·.rename ρ)) =
      (containsVector? arguments).map (renameResult ρ) := by
  unfold containsVector?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a)
  all_goals simp [findVectorIndex_map (·.rename ρ)
    (fun left right => by rw [RuntimeValue.observe_rename, RuntimeValue.observe_rename])]

theorem indexOfVector?_rename (ρ : Nat → Nat) (indexType : Ty) (arguments : Array RuntimeValue) :
    indexOfVector? indexType (arguments.map (·.rename ρ)) =
      (indexOfVector? indexType arguments).map (renameResult ρ) := by
  unfold indexOfVector?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a)
  all_goals simp [findVectorIndex_map (·.rename ρ)
    (fun left right => by rw [RuntimeValue.observe_rename, RuntimeValue.observe_rename])]
  split
  · simp
  · simp only [Option.map_bind, Function.comp_def, Option.map_some, renameResult_ok]
    refine Option.bind_congr fun index index_eq => ?_
    simp [modularInteger_rename_of_eq index_eq]

theorem destroyEmptyVector?_rename (ρ : Nat → Nat) (arguments : Array RuntimeValue) :
    destroyEmptyVector? (arguments.map (·.rename ρ)) =
      (destroyEmptyVector? arguments).map (renameResult ρ) := by
  unfold destroyEmptyVector?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, rest⟩⟩
  all_goals try (cases a)
  all_goals simp
  all_goals split <;> simp

theorem concatVector?_rename (ρ : Nat → Nat) (arguments : Array RuntimeValue) :
    concatVector? (arguments.map (·.rename ρ)) =
      (concatVector? arguments).map (renameResult ρ) := by
  unfold concatVector?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b)
  all_goals simp

theorem sliceVector?_rename (ρ : Nat → Nat) (arguments : Array RuntimeValue) :
    sliceVector? (arguments.map (·.rename ρ)) =
      (sliceVector? arguments).map (renameResult ρ) := by
  unfold sliceVector?
  rw [Array.toList_map]
  rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, _ | ⟨d, rest⟩⟩⟩⟩
  all_goals try (cases a <;> (try (simp; done)) <;> cases b <;> (try (simp; done)) <;> cases c)
  all_goals simp
  all_goals split <;> simp

theorem shiftInteger_rename (ρ : Nat → Nat) (left : Bool) (resultType : Ty)
    (arguments : Array RuntimeValue) :
    shiftInteger left resultType (arguments.map (·.rename ρ)) =
      (shiftInteger left resultType arguments).map (renameResult ρ) := by
  unfold shiftInteger
  rw [checkedShiftInteger_rename]
  rcases checkedShiftInteger .panic left resultType arguments with _ | ⟨_, _⟩ | _ <;> simp

theorem evaluatePrimitiveOperation?_rename (ρ : Nat → Nat) (ns : Validation.ValidatedNamespace)
    (resultType : TypeId) (operation : PrimitiveOperation) (arguments : Array RuntimeValue)
    (width : Option Nat) :
    evaluatePrimitiveOperation? ns resultType operation (arguments.map (·.rename ρ)) width =
      (evaluatePrimitiveOperation? ns resultType operation arguments width).map
        (renameResult ρ) := by
  unfold evaluatePrimitiveOperation?
  simp only [Option.bind_eq_bind, Option.map_bind]
  refine Option.bind_congr fun resolved _ => ?_
  cases operation
  all_goals try simp only [Function.comp_apply, checkedUnaryInteger_rename,
    checkedBinaryInteger_rename, modularUnaryInteger_rename, modularBinaryInteger_rename,
    overflowingBinaryInteger_rename, bitwiseBinary_rename, bitwiseNotInteger_rename,
    checkedShiftInteger_rename, compareOrdered_rename, equalValues?_rename, notEqualValues?_rename,
    divideIntegers?_rename, checkedDivideIntegers?_rename, moduloIntegers?_rename,
    checkedModuloIntegers?_rename, insertVector?_rename, removeVector?_rename, swapVector?_rename,
    reverseSliceVector?_rename, checkVectorIndex?_rename, containsVector?_rename,
    destroyEmptyVector?_rename, concatVector?_rename, sliceVector?_rename, shiftInteger_rename]
  case tuple | vector | range | implies | equivalent | identical => simp
  case indexOfVector =>
    split
    · split
      · simp only [Option.map_bind]
        exact Option.bind_congr fun indexType _ => indexOfVector?_rename ρ indexType arguments
      · rfl
    · rfl
  all_goals rw [Array.toList_map]
  case compare =>
    rcases arguments.toList with _ | ⟨a, _ | ⟨b, _ | ⟨c, rest⟩⟩⟩ <;>
      simp [RuntimeValue.observe_rename]
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
    simp only [List.map_cons, List.map_nil, RuntimeValue.rename_vector, RuntimeValue.rename_integer]
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

theorem evaluatePrimitiveOperation?_within {β γ : Nat → Nat} (base : LoanBase β γ inside)
    {ns : Validation.ValidatedNamespace}
    {resultType : TypeId} {operation : PrimitiveOperation} {arguments : Array RuntimeValue}
    {width : Option Nat} {result : Except (ThrowKind × Array RuntimeValue) RuntimeValue}
    (above : ∀ argument ∈ arguments, argument.Within inside)
    (evaluate : evaluatePrimitiveOperation? ns resultType operation arguments width = some result) :
    (∀ value, result = .ok value → value.Within inside) ∧
      (∀ kind values, result = .error (kind, values) → ∀ value ∈ values, value.Within inside) := by
  rw [← Array.rename_inverse_all base above, evaluatePrimitiveOperation?_rename] at evaluate
  obtain ⟨original, -, rfl⟩ := Option.map_eq_some_iff.mp evaluate
  refine ⟨fun value same => ?_, fun kind values same => ?_⟩
  · cases original with
    | ok original =>
        simp only [renameResult_ok, Except.ok.injEq] at same
        exact same ▸ base.within original
    | error original => cases same
  · cases original with
    | ok original => cases same
    | error original =>
        obtain ⟨baseKind, baseValues⟩ := original
        simp only [renameResult_error, Except.error.injEq, Prod.mk.injEq] at same
        obtain ⟨-, rfl⟩ := same
        intro value member
        obtain ⟨renamed, -, rfl⟩ := Array.mem_map.mp member
        exact base.within renamed

end LeanerIR
