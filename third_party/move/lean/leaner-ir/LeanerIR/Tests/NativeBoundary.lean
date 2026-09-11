-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeBoundary
import Lean.Util.CollectAxioms

set_option Elab.async false
set_option maxHeartbeats 1000

namespace LeanerIR.Tests.NativeBoundary

open LeanerIR.Proofs NativeBoundary

-- Output and state have distinct roles: finish uses the actual middle state,
-- once, after the computation and its last continuation.
def advance : Spec Nat Nat Nat := Spec.bind (Spec.modify (· + 2)) (fun _ => Spec.pure 7)
def commit (owner state : Nat) : Nat := state + owner

example (initial : Nat) : (finish id commit advance).ok initial 7 (initial + 2 + 7) :=
  ⟨7, initial + 2, ⟨(), initial + 2, ⟨rfl, rfl⟩, rfl, rfl⟩, rfl, rfl⟩

example (initial : Nat) : ¬(finish id commit advance).ok initial 7 (initial + 7) := by
  rintro ⟨output, middle, ⟨_, _, ⟨rfl, rfl⟩, equal, same⟩, _, final⟩
  subst output middle
  simp only [commit] at final
  omega

-- The successful finalizer must not run on an abort or erase undefinedness.
example (initial error : Nat) :
    (finish id commit (Spec.abort 9)).aborts initial error ↔ error = 9 := Iff.rfl

example (initial result final : Nat) :
    ¬(finish id commit (Spec.abort 9)).ok initial result final := by
  rintro ⟨_, _, impossible, _, _⟩
  exact impossible

def undefined : Spec Nat Nat Nat where
  ok := fun _ _ _ => False
  aborts := fun _ _ => False
  undefined := fun _ => True

example (initial : Nat) : (finish id commit undefined).undefined initial := trivial

example (initial : Nat) :
    ¬wp (finish id commit undefined) (fun _ _ => True) (fun _ => True) initial := by
  rintro ⟨_, _, defined⟩
  exact defined trivial

def native : Contract Nat Nat Unit Nat where
  requires := fun _ _ => True
  ensures := fun _ _ output _ => output = 7
  mayAbort := fun _ _ => False
  aborts := fun _ _ _ => False

def target : Contract Nat Nat Unit Nat where
  requires := fun _ _ => True
  ensures := fun _ initial output final => output = 7 ∧ final = initial + 7
  frame := fun _ initial final => final = initial + 7
  mayAbort := fun _ _ => False
  aborts := fun _ _ _ => False

theorem verified : Satisfies (fun _ => Spec.pure 7) native := by
  apply satisfies_of_wp
  intro args initial _
  simp [wp_pure, native]

theorem bridge : Transports native target id commit := by
  constructor
  · exact fun _ _ _ => trivial
  · intro args initial output middle _ post frame _
    have same : output = 7 := post (fun impossible => impossible)
    have state : middle = initial := frame
    subst output middle
    exact ⟨fun _ => ⟨rfl, rfl⟩, rfl, fun impossible => impossible⟩
  · exact fun _ _ _ _ impossible => impossible.elim

example : Satisfies (fun _ => finish id commit (Spec.pure 7)) target :=
  bridge.satisfies verified

-- A native proof is insufficient to justify an arbitrary boundary adapter.
example : ¬Transports native target id (fun _ state => state) := by
  intro wrong
  have normal := wrong.normal () 0 7 0 trivial (fun _ => rfl) rfl (fun h => h)
  have bad : (0 : Nat) = 7 := normal.2.1
  contradiction

example : ¬Transports native
    { target with ensures := fun _ _ output _ => output = 8 } id commit := by
  intro wrong
  have bad : (7 : Nat) = 8 :=
    (wrong.normal () 0 7 0 trivial (fun _ => rfl) rfl (fun h => h)).1 (fun h => h)
  contradiction

-- The boundary cannot drop frame or must-abort obligations even if the
-- target's may-abort condition excuses its postcondition.
example : ¬Transports native
    { target with mayAbort := fun _ _ => True, frame := fun _ _ _ => False } id commit := by
  intro wrong
  exact (wrong.normal () 0 7 0 trivial (fun _ => rfl) rfl (fun h => h)).2.1

example : ¬Transports native
    { target with mayAbort := fun _ _ => True, mustAbort := fun _ _ => True } id commit := by
  intro wrong
  exact (wrong.normal () 0 7 0 trivial (fun _ => rfl) rfl (fun h => h)).2.2 trivial

open Lean Elab Command in
run_cmd do
  for name in [``NativeBoundary.wp_finish, ``NativeBoundary.finish_bind,
      ``NativeBoundary.Transports.satisfies, ``NativeBoundary.Represents.typed,
      ``NativeBoundary.Represents.satisfies, ``bridge, ``verified] do
    if (← Lean.collectAxioms name).contains ``sorryAx then
      throwError "native boundary admission: {name}"
    let some proof := (← getEnv).find? name |>.bind (·.value? (allowOpaque := true))
      | throwError "missing native boundary proof {name}"
    if (← proof.numObjs) > 10000 then throwError "native boundary proof exceeds 10k objects: {name}"

end LeanerIR.Tests.NativeBoundary
