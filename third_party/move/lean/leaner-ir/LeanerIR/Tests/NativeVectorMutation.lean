-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeMutationAgreement
import LeanerIR.Proofs.NativeLoop
import Lean.Util.CollectAxioms

-- Symbolic mutation and one-step invariant proofs must not grow with the
-- concrete vector length or loop trip count. Every declaration has a 1M cap.
set_option maxHeartbeats 1000
set_option Elab.async false

namespace LeanerIR.Tests.NativeVectorMutation

open LeanerIR.Proofs

example (values : SpecVector α) (index : Int) (replacement : α)
    (valid : 0 ≤ index ∧ index < (values.values.size : Int)) :
    wp (NativeVector.set values index replacement (7 : Nat))
      (fun result (_ : Unit) => result.values.size = values.values.size)
      (fun _ => False) () := by
  rw [NativeVector.wp_set]
  exact ⟨fun _ => NativeVector.replaceAt_size .., fun invalid => invalid valid⟩

example (values : SpecVector α) (index other : Nat) (replacement : α)
    (valid : index < values.values.size) (otherValid : other < values.values.size)
    (distinct : index ≠ other) :
    (NativeVector.replaceAt values index replacement valid).values[other]'(by simpa using otherValid) =
      values.values[other] :=
  NativeVector.replaceAt_get_other values index other replacement valid otherValid distinct

def sample : SpecVector Bool := ⟨#[true, true, true], by decide⟩
def changed : SpecVector Bool := ⟨#[true, false, true], by decide⟩

example : (NativeVector.set sample 1 false (7 : Nat)).ok () changed () := by
  exact ⟨SpecVector.ext (by decide), rfl⟩

example : ¬(NativeVector.set sample 1 false (7 : Nat)).ok () sample () := by
  intro executed
  have same := congrArg (fun values : SpecVector Bool => values.values[1]?) executed.1
  contradiction

example : (NativeVector.set sample (-1) false (7 : Nat)).aborts () 7 := rfl
example : (NativeVector.set sample 3 false (7 : Nat)).aborts () 7 := rfl
example : (NativeVector.set sample 18446744073709551615 false (7 : Nat)).aborts () 7 := rfl
example : (NativeVector.set (⟨#[], by decide⟩ : SpecVector Bool) 0 false (7 : Nat)).aborts () 7 := rfl

example : ¬(NativeVector.set sample (-1) false (7 : Nat)).ok () changed () := by
  exact id

example : ¬(NativeVector.set sample 3 false (7 : Nat)).aborts () 8 := by
  change (8 : Nat) ≠ 7
  decide

-- A native mutable reference carries its loan identity through the update.
example (loan : Nat) :
    (NativeMutation.setElement ⟨loan, sample⟩ 1 false (7 : Nat)).ok () ⟨loan, changed⟩ () := by
  rw [NativeMutation.setElement_ok]
  change (NativeVector.set sample 1 false (7 : Nat)).ok () changed () ∧ loan = loan
  exact ⟨⟨SpecVector.ext (by decide), rfl⟩, rfl⟩

example (loan : Nat) :
    ¬(NativeMutation.setElement ⟨loan, sample⟩ 1 false (7 : Nat)).ok () ⟨loan + 1, changed⟩ () := by
  rw [NativeMutation.setElement_ok]
  rintro ⟨_, impossible⟩
  change loan + 1 = loan at impossible
  omega

example (loan : Nat) :
    (NativeMutation.setElement ⟨loan, sample⟩ (-1) false (7 : Nat)).aborts () 7 := by
  rw [NativeMutation.setElement_aborts]
  exact ⟨fun invalid => by have negative := invalid.1; omega, rfl⟩

example (reference : MutableArgument (SpecVector α)) (index : Int) (replacement : α)
    (valid : 0 ≤ index ∧ index < (reference.value.values.size : Int)) :
    wp (NativeMutation.setElement reference index replacement (7 : Nat))
      (fun updated (_ : Unit) => updated.loan = reference.loan ∧
        updated.value.values.size = reference.value.values.size)
      (fun _ => False) () := by
  rw [NativeMutation.wp_setElement]
  exact ⟨fun _ => ⟨rfl, NativeVector.replaceAt_size ..⟩, fun invalid => invalid valid⟩

def resting (loan : Nat) : RuntimeFrame := {
  locals := #[some (.integer 91), none, some (.borrow loan (.bool true))]
  loanLocations := #[(loan, ⟨.local ⟨2⟩, #[], true⟩)] }

-- An actual reference-mutation operation at a nonzero slot, preserving the
-- unrelated local, availability, registries, and complete runtime store.
example (loan : Nat) (state : RuntimeState) :
    Denotation.ReferenceLocationOperation.mutate.evaluate?
      #[.borrow loan (.bool true), .bool false] (resting loan) state =
      some (.value { (resting loan) with
        locals := #[some (.integer 91), none, some (.borrow loan (.bool false))] } state .unit) := by
  simpa [resting, Codec.mutable, Codec.bool, NativeMutation.write] using
    NativeMutation.write_registered Codec.bool ⟨loan, true⟩ false ⟨2⟩ (resting loan) state
      (by simp [SemanticOperations.localLoanPlace?, resting]) rfl

/-- The owned aggregate carried by `clear`: a vector plus an unrelated field.
This is native kernel coverage, not yet a claim about the source borrow emitter. -/
structure Bits where
  length : Nat
  bitField : SpecVector Bool

def iteration (locals : Nat × Bits) :
    Spec Unit Nat (NativeLoop.Step (Nat × Bits) Bits) :=
  if locals.1 < locals.2.bitField.values.size then
    Spec.bind (NativeVector.set locals.2.bitField locals.1 false 7) fun updated =>
      Spec.pure (.next (locals.1 + 1, { locals.2 with bitField := updated }))
  else Spec.pure (.done locals.2)

def invariant (initial : Bits) (locals : Nat × Bits) (_ : Unit) : Prop :=
  locals.1 ≤ locals.2.bitField.values.size ∧
  locals.2.length = initial.length ∧
  locals.2.bitField.values.size = initial.bitField.values.size ∧
  ∀ index (valid : index < locals.2.bitField.values.size), index < locals.1 →
    locals.2.bitField.values[index]'valid = false

def cleared (initial result : Bits) (_ : Unit) : Prop :=
  result.length = initial.length ∧
  result.bitField.values.size = initial.bitField.values.size ∧
  ∀ index, (valid : index < result.bitField.values.size) →
    result.bitField.values[index]'valid = false

/-- One invariant step proves clearing for every bounded vector length; no
fixed execution bound or loop unrolling appears in the verification proof. -/
theorem clear_verified (initial : Bits) :
    wp (NativeLoop.run iteration (0, initial) (invariant initial))
      (cleared initial) (fun _ => False) () := by
  apply NativeLoop.wp_run
  · exact ⟨Nat.zero_le _, rfl, rfl, fun _ _ impossible => by omega⟩
  · intro locals state established
    obtain ⟨bounded, field, size, clearedBefore⟩ := established
    unfold iteration
    split
    next more =>
      rw [wp_bind, NativeVector.wp_set]
      constructor
      · intro valid
        rw [wp_pure]
        change invariant initial _ state
        refine ⟨?_, field, ?_, ?_⟩
        · simp only [NativeVector.replaceAt_size]; omega
        · simpa using size
        · intro index elementValid before
          by_cases same : locals.1 = index
          · subst index
            simp
          · have earlier : index < locals.1 := by omega
            simpa [NativeVector.replaceAt, Array.getElem_set, same] using
              clearedBefore index (by simpa using elementValid) earlier
      · intro invalid
        apply invalid
        omega
    next finished =>
      rw [wp_pure]
      refine ⟨field, size, ?_⟩
      intro index valid
      exact clearedBefore index valid (by omega)

-- Exact finite executions check that the invariant does not replace the body.
example :
    (NativeLoop.run iteration (0, ⟨17, ⟨#[], by decide⟩⟩) (fun _ _ => False)).ok
      () ⟨17, ⟨#[], by decide⟩⟩ () := by
  apply (NativeLoop.run_ok ..).mpr
  exact .done ⟨rfl, rfl⟩

example :
    (NativeLoop.run iteration (0, ⟨17, sample⟩) (invariant ⟨17, sample⟩)).ok
      () ⟨17, ⟨#[false, false, false], by decide⟩⟩ () := by
  apply (NativeLoop.run_ok ..).mpr
  apply NativeLoop.Runs.next (middle := ())
  · exact ⟨_, (), ⟨rfl, rfl⟩, rfl, rfl⟩
  apply NativeLoop.Runs.next (middle := ())
  · exact ⟨_, (), ⟨rfl, rfl⟩, rfl, rfl⟩
  apply NativeLoop.Runs.next (middle := ())
  · exact ⟨_, (), ⟨rfl, rfl⟩, rfl, rfl⟩
  apply NativeLoop.Runs.done
  exact ⟨by congr 2 <;> apply SpecVector.ext <;> decide, rfl⟩

open Lean Elab Command in
run_cmd do
  for theoremName in [``NativeVector.replaceAt_write, ``NativeVector.set_ok,
      ``NativeVector.set_aborts, ``NativeVector.wp_set, ``NativeVector.set_defined,
      ``NativeMutation.setElement_write, ``NativeMutation.setElement_aborts,
      ``NativeMutation.setElement_defined, ``NativeMutation.wp_setElement, ``clear_verified] do
    if (← collectAxioms theoremName).contains ``sorryAx then
      throwError "native vector mutation theorem contains an admission: {theoremName}"
    let some proof := (← getEnv).find? theoremName |>.bind (·.value? (allowOpaque := true))
      | throwError "missing native mutation proof: {theoremName}"
    let size ← (proof.numObjs : IO Nat)
    unless size ≤ 10000 do
      throwError "native mutation proof exceeds 10k objects: {theoremName}: {size}"

open Lean Elab Command in
run_cmd do
  for definitionName in [``NativeVector.replaceAt, ``NativeVector.set,
      ``NativeMutation.write, ``NativeMutation.setElement, ``iteration, ``invariant] do
    let some body := (← getEnv).find? definitionName |>.bind (·.value?)
      | throwError "missing native vector mutation definition: {definitionName}"
    for forbidden in [``RuntimeValue, ``RuntimeFrame, ``Codec.encode, ``Codec.decode?] do
      if body.getUsedConstants.contains forbidden then
        throwError "native vector mutation uses an encoded value: {definitionName}: {forbidden}"

open Lean Elab Command in
run_cmd do
  if (← collectAxioms ``NativeMutation.write_registered).contains ``sorryAx then
    throwError "registered native write agreement contains an admission"
  let some proof := (← getEnv).find? ``NativeMutation.write_registered
      |>.bind (·.value? (allowOpaque := true))
    | throwError "missing registered native write agreement"
  unless (← (proof.numObjs : IO Nat)) ≤ 10000 do
    throwError "registered native write agreement exceeds 10k objects"

end LeanerIR.Tests.NativeVectorMutation
