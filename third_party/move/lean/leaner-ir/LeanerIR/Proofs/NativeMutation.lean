-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeVector

/-! Native ownership-passing updates for live mutable references. The loan
identity is preserved and the current value stays fully typed, as in v0's
ownership-passing representation. Loan creation, suspension, and final export
are separate operations: these updates do not pretend to implement them. -/

namespace LeanerIR.Proofs.NativeMutation

set_option maxHeartbeats 1000

def read (reference : MutableArgument α) : α := reference.value

def write (reference : MutableArgument α) (replacement : α) : MutableArgument α :=
  { reference with value := replacement }

@[simp] theorem read_write (reference : MutableArgument α) (replacement : α) :
    read (write reference replacement) = replacement := rfl

@[simp] theorem write_loan (reference : MutableArgument α) (replacement : α) :
    (write reference replacement).loan = reference.loan := rfl

@[simp] theorem write_read (reference : MutableArgument α) :
    write reference (read reference) = reference := rfl

@[simp] theorem write_write (reference : MutableArgument α) (first second : α) :
    write (write reference first) second = write reference second := rfl

/-- Reconcile an indexed update into its typed vector owner. Bounds failure
does not produce a changed reference or allocate a loan. -/
def setElement (reference : MutableArgument (SpecVector α)) (index : Int)
    (replacement : α) (failure : Error) : Spec State Error (MutableArgument (SpecVector α)) :=
  Spec.bind (NativeVector.set reference.value index replacement failure) fun updated =>
    Spec.pure (write reference updated)

theorem setElement_state (reference : MutableArgument (SpecVector α)) (index : Int)
    (replacement : α) (failure : Error) :
    StatePreserving (setElement reference index replacement failure :
      Spec State Error (MutableArgument (SpecVector α))) :=
  StatePreserving.bind (NativeVector.set_state reference.value index replacement failure)
    (fun _ => StatePreserving.pure _)

theorem wp_setElement (reference : MutableArgument (SpecVector α)) (index : Int)
    (replacement : α) (failure : Error)
    (ensures : MutableArgument (SpecVector α) → State → Prop)
    (aborts : Error → Prop) (initial : State) :
    wp (setElement reference index replacement failure) ensures aborts initial ↔
      (∀ valid : 0 ≤ index ∧ index < (reference.value.values.size : Int),
        ensures (write reference
          (NativeVector.replaceAt reference.value index.toNat replacement (by omega))) initial) ∧
      (¬(0 ≤ index ∧ index < (reference.value.values.size : Int)) → aborts failure) := by
  simp only [setElement, wp_bind, NativeVector.wp_set, wp_pure]

end LeanerIR.Proofs.NativeMutation
