-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move
import Tests.Common

/-!
# Generic in-place quicksort

A self-contained proof-of-concept: generic Move source, an explicit contract,
its source-level proof model, and compiled execution tests.
-/

namespace Tests.MovePrograms

open Move
open MoveModel.Frontend.XIR
open scoped Move Move.Compiler Move.Spec

move_module Quicksort where

  @[move_struct]
  structure PartitionResult (T : Type) where
    values : Move.Vector T
    pivot : U64
    deriving Copy, Drop, Store

  /-- Lomuto partition of the half-open range ending at `pivotIndex`. -/
  partial fun partitionLoop {T : Type}
      (values : Move.Vector T) (pivotIndex scan store : U64) :
      Action (PartitionResult T) := do
    let pivotRef ← &values[pivotIndex]
    let pivot ← *pivotRef
    if scan < pivotIndex then
      let candidateRef ← &values[scan]
      let candidate ← *candidateRef
      if candidate < pivot then
        if store < scan then
          let storedRef ← &values[store]
          let stored ← *storedRef
          let destination ← &mut values[store]
          destination := candidate
          let source ← &mut values[scan]
          source := stored
        partitionLoop values pivotIndex (scan + 1) (store + 1)
      else
        partitionLoop values pivotIndex (scan + 1) store
    else
      if store < pivotIndex then
        let storedRef ← &values[store]
        let stored ← *storedRef
        let destination ← &mut values[store]
        destination := pivot
        let source ← &mut values[pivotIndex]
        source := stored
      pure { values, pivot := store }

  partial fun quickSortRange {T : Type}
      (values : Move.Vector T) (low high : U64) :
      Action (Move.Vector T) := do
    if low < high then
      let span := high - low
      if 1 < span then
        let partitioned ← partitionLoop values (high - 1) low low
        let left ← quickSortRange partitioned.values low partitioned.pivot
        quickSortRange left (partitioned.pivot + 1) high
      else
        pure values
    else
      pure values

  /-- Generic in-place sort using Move's built-in lexicographic comparison. -/
  fun quickSort {T : Type}
      (values : Move.Vector T) : Action (Move.Vector T) :=
    quickSortRange values 0 values.length

namespace Quicksort

  namespace Model

  /- The recursive source translator does not yet derive semantics for this
  example automatically. These relations are its small, explicit proof model. -/

  def Sorted [Move.Compare.Lawful α] (values : Move.Vector α) : Prop :=
    values.toList.Pairwise fun left right => ¬Move.Compare.Less right left

  def Permutation (before after : Move.Vector α) : Prop :=
    after.toList.Perm before.toList

  inductive Partition (pivot : α) : List α → List α → List α → Prop where
    | nil : Partition pivot [] [] []
    | lower (value : α) (rest lower upper : List α)
        (branch : Move.Compare.Less value pivot)
        (next : Partition pivot rest lower upper) :
        Partition pivot (value :: rest) (value :: lower) upper
    | upper (value : α) (rest lower upper : List α)
        (branch : ¬Move.Compare.Less value pivot)
        (next : Partition pivot rest lower upper) :
        Partition pivot (value :: rest) lower (value :: upper)

  namespace Partition

  theorem permutation (partition : Partition pivot input lows highs) :
      (lows ++ highs).Perm input := by
    induction partition with
    | nil => exact .refl []
    | lower value rest lower upper _ _ ih => exact ih.cons value
    | upper value rest lower upper _ _ ih =>
        exact List.perm_middle.trans (ih.cons value)

  theorem lowerValues (partition : Partition pivot input lows highs) :
      ∀ value ∈ lows, Move.Compare.Less value pivot := by
    induction partition with
    | nil => simp
    | lower value rest lower upper branch next ih =>
        simpa only [List.mem_cons, forall_eq_or_imp] using And.intro branch ih
    | upper value rest lower upper branch next ih => exact ih

  theorem upperValues (partition : Partition pivot input lows highs) :
      ∀ value ∈ highs, ¬Move.Compare.Less value pivot := by
    induction partition with
    | nil => simp
    | lower value rest lower upper branch next ih => exact ih
    | upper value rest lower upper branch next ih =>
        simpa only [List.mem_cons, forall_eq_or_imp] using And.intro branch ih

  end Partition

  inductive QuickSort : List α → List α → Prop where
    | nil : QuickSort [] []
    | partition (pivot : α) (rest lower upper sortedLower sortedUpper : List α)
        (scan : Partition pivot rest lower upper)
        (sortLower : QuickSort lower sortedLower)
        (sortUpper : QuickSort upper sortedUpper) :
        QuickSort (pivot :: rest) (sortedLower ++ pivot :: sortedUpper)

  namespace QuickSort

  theorem permutation {input output : List α} (sorted : QuickSort input output) :
      output.Perm input := by
    induction sorted with
    | nil => exact .refl []
    | partition pivot rest lower upper sortedLower sortedUpper scan _ _ ihLower ihUpper =>
        exact (ihLower.append_cons pivot ihUpper).trans
          (List.perm_middle.trans (scan.permutation.cons pivot))

  theorem sorted [Move.Compare.Lawful α] {input output : List α}
      (result : QuickSort input output) :
      output.Pairwise fun left right => ¬Move.Compare.Less right left := by
    induction result with
    | nil => simp
    | partition pivot rest lower upper sortedLower sortedUpper scan
        sortLower sortUpper ihLower ihUpper =>
        have lowerMem : ∀ value ∈ sortedLower, value ∈ lower := by
          intro value member
          exact (sortLower.permutation.mem_iff).mp member
        have upperMem : ∀ value ∈ sortedUpper, value ∈ upper := by
          intro value member
          exact (sortUpper.permutation.mem_iff).mp member
        rw [List.pairwise_append]
        refine ⟨ihLower, ?_, ?_⟩
        · simp only [List.pairwise_cons]
          exact ⟨fun value member => scan.upperValues value (upperMem value member), ihUpper⟩
        · intro left leftMember right rightMember
          have leftLt := scan.lowerValues left (lowerMem left leftMember)
          simp only [List.mem_cons] at rightMember
          rcases rightMember with rfl | rightMember
          · exact Move.Compare.Lawful.asymm leftLt
          · intro rightLt
            exact scan.upperValues right (upperMem right rightMember)
              (Move.Compare.Lawful.trans rightLt leftLt)

  end QuickSort

  def sourceSpec (values : Move.Vector α) :
      Move.Semantics.Spec σ (Move.Vector α) :=
    @Move.Semantics.Spec.mk σ (Move.Vector α)
      (fun initial result final =>
        QuickSort values.toList result.toList ∧ final = initial)
      (fun _ _ => False)

  theorem sourceSpec_satisfies [Move.Compare.Lawful α] :
      Move.Verify.Satisfies (sourceSpec (σ := σ) (α := α))
        (@Move.Verify.Contract.mk σ (Move.Vector α) (Move.Vector α)
          (fun _ _ => True)
          (fun values _ result _ => Sorted result ∧ Permutation values result)
          (fun _ _ _ => False)) := by
    intro values initial _
    constructor
    · intro result final execution
      exact ⟨execution.1.sorted, execution.1.permutation⟩
    · intro code execution
      exact execution.elim

  end Model

  /- The contract is the user-facing part of the proof. -/
  def quickSort.sourceSpec {T : Type} [Inhabited T] {State : Type}
      (values : Move.Vector T) : Move.Semantics.Spec State (Move.Vector T) :=
    Model.sourceSpec values

  spec quickSort {T : Type} [Move.Compare.Lawful T]
      (values : Move.Vector T) where
    requires True;
    ensures Model.Sorted result ∧ Model.Permutation values result;
    aborts_if False

  verify quickSort by
    intro T _ _ state
    exact Model.sourceSpec_satisfies

  def compiled : MModule := move_module% "QuicksortTest"

  private def run := Tests.run compiled

  #test run "quickSort" [] [.vector [.u64 5, .u64 1, .u64 4, .u64 2, .u64 3]] =
    Tests.okRet [] [.vector [.u64 1, .u64 2, .u64 3, .u64 4, .u64 5]]
  #test run "quickSort" [] [.vector []] = Tests.okRet [] [.vector []]
  #test run "quickSort" [] [.vector [.bool true, .bool false, .bool true]] =
    Tests.okRet [] [.vector [.bool false, .bool true, .bool true]]
  #test run "quickSort" [] [.vector
      [.vector [.u64 1, .u64 3], .vector [.u64 1, .u64 2],
       .vector [.u64 0, .u64 9]]] =
    Tests.okRet [] [.vector
      [.vector [.u64 0, .u64 9], .vector [.u64 1, .u64 2],
       .vector [.u64 1, .u64 3]]]
  #test run "quickSort" [] [.vector [.u64 2, .u64 1, .u64 2]] =
    Tests.okRet [] [.vector [.u64 1, .u64 2, .u64 2]]

end Quicksort

end Tests.MovePrograms
