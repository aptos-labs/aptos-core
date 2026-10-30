-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! Frames can ignore the values written at other keys and resource types,
including writes linked from a shared entry. A write at the observed slot
must still require its actual postcondition. -/

open Lean Meta Elab Tactic
open VerexIR VerexIR.Proofs.Denote

variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit]

example (memory : Memory unit) (resource : ResourceType) (key : StorageKey)
    (first second : Option (resource.carrier unit)) :
    ∀ other, other ≠ key →
      ((memory.set resource key first).set resource key second) resource other =
        memory resource other := by
  verex_frame_writes

example (memory : Memory unit) (resource : ResourceType) (key : StorageKey)
    (first second : Option (resource.carrier unit)) :
    ∀ other, other ≠ resource →
      ((memory.set resource key first).set resource key second) other = memory other := by
  verex_frame_writes

example (memory : Memory unit) (resource : ResourceType) (key other : StorageKey)
    (value : Option (resource.carrier unit)) (distinct : other ≠ key) :
    (memory.set resource key value) resource other = memory resource other := by
  verex_frame_writes

example (memory : Memory unit) (resource : ResourceType) (key : StorageKey)
    (value : Option (resource.carrier unit)) :
    (∀ other, other ≠ key →
      (memory.set resource key value) resource other = memory resource other) ∧
    (∀ other, other ≠ resource →
      (memory.set resource key value) other = memory other) := by
  verex_frame_writes

example (memory : Memory unit) (resource : ResourceType) (key : StorageKey)
    (value : Option (resource.carrier unit)) :
    (memory.set resource key value) resource key = value := by
  fail_if_success
    have : (memory.set resource key value) resource key = memory resource key := by
      verex_frame_writes
  simp

private def priorEntry (choice : Bool) : Prop := choice = true ∨ choice = false
attribute [irreducible] priorEntry

private def frameEntry (memory before : Memory unit) (resource : ResourceType)
    (key : StorageKey) (choice : Bool) : Prop :=
  ∃ (_ : priorEntry choice) (first second : Option (resource.carrier unit)),
    memory = (before.set resource key first).set resource key second

attribute [irreducible] frameEntry

-- The first linked row suffices. The remaining named assumption need not
-- be opened, even when doing so would produce more predecessor alternatives.
elab "check_linked_frame" : tactic => do
  let closedEarly ← IO.mkRef 0
  let goals ← SharedContinuations.exposeEntries (← getMainGoal) #[``frameEntry, ``priorEntry]
    fun goal => do
      let retainsPrior ← goal.withContext do
        return (← getLCtx).any (·.type.isAppOf ``priorEntry)
      let closed ← closeFrameWrites goal
      if closed && retainsPrior then closedEarly.modify (· + 1)
      return closed
  unless goals.isEmpty && (← closedEarly.get) == 1 do
    throwError "the frame did not close before opening its earlier entry"
  setGoals goals

example (memory before : Memory unit) (resource : ResourceType) (key : StorageKey)
    (choice : Bool) (entry : frameEntry memory before resource key choice) :
    ∀ other, other ≠ key → memory resource other = before resource other := by
  check_linked_frame
