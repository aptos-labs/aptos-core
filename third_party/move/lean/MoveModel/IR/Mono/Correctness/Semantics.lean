-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Mono.Correctness.Rewrite
import MoveModel.IR.Semantics

/-!
# Type-tag congruence of IR semantics

Move erases ordinary type arguments at runtime. In the supported IR, explicit
type arguments affect execution only through generic global resource keys.
This module proves that tag-equivalent substitutions therefore give identical
primitive semantics.
-/

namespace MoveModel.IR

private theorem Oper.sem_unpackInst_irrel (lhs rhs : List Ty)
    (current : FrameId) (deref : RefTarget → Option Value)
    (values : List Value) (memory : Memory) :
    (Oper.unpackInst lhs).sem current deref values memory =
      (Oper.unpackInst rhs).sem current deref values memory := by
  cases values with
  | nil => rfl
  | cons value rest =>
      cases rest with
      | nil => cases value <;> rfl
      | cons _ _ => simp [Oper.sem]

private theorem Oper.sem_unpackVariantInst_irrel (tag : Nat)
    (lhs rhs : List Ty) (current : FrameId)
    (deref : RefTarget → Option Value) (values : List Value)
    (memory : Memory) :
    (Oper.unpackVariantInst tag lhs).sem current deref values memory =
      (Oper.unpackVariantInst tag rhs).sem current deref values memory := by
  cases values with
  | nil => rfl
  | cons value rest =>
      cases rest with
      | nil => cases value <;> rfl
      | cons _ _ => simp [Oper.sem]

private theorem Oper.sem_testVariantInst_irrel (tag : Nat)
    (lhs rhs : List Ty) (current : FrameId)
    (deref : RefTarget → Option Value) (values : List Value)
    (memory : Memory) :
    (Oper.testVariantInst tag lhs).sem current deref values memory =
      (Oper.testVariantInst tag rhs).sem current deref values memory := by
  cases values with
  | nil => rfl
  | cons value rest =>
      cases rest with
      | nil => cases value <;> rfl
      | cons _ _ => simp [Oper.sem]

private theorem Oper.sem_getFieldInst_irrel (field : Nat)
    (lhs rhs : List Ty) (current : FrameId)
    (deref : RefTarget → Option Value) (values : List Value)
    (memory : Memory) :
    (Oper.getFieldInst field lhs).sem current deref values memory =
      (Oper.getFieldInst field rhs).sem current deref values memory := by
  cases values with
  | nil => rfl
  | cons value rest =>
      cases rest with
      | nil => cases value <;> rfl
      | cons _ _ => simp [Oper.sem]

private theorem Oper.sem_getGlobalInst_of_key_eq (resource : ResourceId)
    {lhs rhs : List Ty} (hkey : resourceKey resource lhs = resourceKey resource rhs)
    (current : FrameId) (deref : RefTarget → Option Value)
    (values : List Value) (memory : Memory) :
    (Oper.getGlobalInst resource lhs).sem current deref values memory =
      (Oper.getGlobalInst resource rhs).sem current deref values memory := by
  cases values with
  | nil => rfl
  | cons value rest =>
      cases rest with
      | nil =>
          cases value <;> simp only [Oper.sem]
          rw [hkey]
      | cons _ _ => simp [Oper.sem]

private theorem Oper.sem_moveToInst_of_key_eq (resource : ResourceId)
    {lhs rhs : List Ty} (hkey : resourceKey resource lhs = resourceKey resource rhs)
    (current : FrameId) (deref : RefTarget → Option Value)
    (values : List Value) (memory : Memory) :
    (Oper.moveToInst resource lhs).sem current deref values memory =
      (Oper.moveToInst resource rhs).sem current deref values memory := by
  cases values with
  | nil => rfl
  | cons value rest =>
      cases rest with
      | nil => simp [Oper.sem]
      | cons payload tail =>
          cases tail with
          | nil =>
              cases value <;> simp only [Oper.sem]
              rw [hkey]
          | cons _ _ => simp [Oper.sem]

private theorem Oper.sem_moveFromInst_of_key_eq (resource : ResourceId)
    {lhs rhs : List Ty} (hkey : resourceKey resource lhs = resourceKey resource rhs)
    (current : FrameId) (deref : RefTarget → Option Value)
    (values : List Value) (memory : Memory) :
    (Oper.moveFromInst resource lhs).sem current deref values memory =
      (Oper.moveFromInst resource rhs).sem current deref values memory := by
  cases values with
  | nil => rfl
  | cons value rest =>
      cases rest with
      | nil =>
          cases value <;> simp only [Oper.sem]
          rw [hkey]
      | cons _ _ => simp [Oper.sem]

private theorem Oper.sem_existsInst_of_key_eq (resource : ResourceId)
    {lhs rhs : List Ty} (hkey : resourceKey resource lhs = resourceKey resource rhs)
    (current : FrameId) (deref : RefTarget → Option Value)
    (values : List Value) (memory : Memory) :
    (Oper.existsInst resource lhs).sem current deref values memory =
      (Oper.existsInst resource rhs).sem current deref values memory := by
  cases values with
  | nil => rfl
  | cons value rest =>
      cases rest with
      | nil =>
          cases value <;> simp only [Oper.sem]
          rw [hkey]
      | cons _ _ => simp [Oper.sem]

/-- Instantiating an operation with tag-equivalent substitutions does not
change its primitive semantics. Function calls and reference operations are
handled by `RunFrom`; `Oper.sem` returns `none` for those operations on both
sides. -/
theorem Oper.sem_instantiate_eq {lhs rhs : List Ty}
    (hargs : TypeArgsTagEq lhs rhs) (op : Oper)
    (current : FrameId) (deref : RefTarget → Option Value)
    (values : List Value) (memory : Memory) :
    (op.instantiate lhs).sem current deref values memory =
      (op.instantiate rhs).sem current deref values memory := by
  cases op <;> try rfl
  · exact Oper.sem_unpackInst_irrel _ _ current deref values memory
  · exact Oper.sem_unpackVariantInst_irrel _ _ _ current deref values memory
  · exact Oper.sem_testVariantInst_irrel _ _ _ current deref values memory
  · exact Oper.sem_getFieldInst_irrel _ _ _ current deref values memory
  · apply Oper.sem_getGlobalInst_of_key_eq
    exact resourceKey_instantiateTypes_eq hargs _ _
  · apply Oper.sem_moveToInst_of_key_eq
    exact resourceKey_instantiateTypes_eq hargs _ _
  · apply Oper.sem_moveFromInst_of_key_eq
    exact resourceKey_instantiateTypes_eq hargs _ _
  · apply Oper.sem_existsInst_of_key_eq
    exact resourceKey_instantiateTypes_eq hargs _ _

/-- The same result for an instruction's operation payload. -/
theorem Instr.instantiated_oper_sem_eq {lhs rhs : List Ty}
    (hargs : TypeArgsTagEq lhs rhs) {dsts srcs : List LocalIndex}
    (op : Oper) (current : FrameId) (deref : RefTarget → Option Value)
    (values : List Value) (memory : Memory) :
    match (Instr.call dsts op srcs).instantiate lhs,
        (Instr.call dsts op srcs).instantiate rhs with
    | Instr.call _ lhsOp _, Instr.call _ rhsOp _ =>
        lhsOp.sem current deref values memory =
          rhsOp.sem current deref values memory
    | _, _ => False := by
  exact op.sem_instantiate_eq hargs current deref values memory

end MoveModel.IR
