-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Computation
import LeanerIR.Proofs.Meaning

/-! Native checked arithmetic. Values and range certificates are typed;
the state and failure vocabulary are parameters, not execution frames. -/

namespace LeanerIR.Proofs.NativeArithmetic

def checkedInteger (width : IntWidth) (signed : Bool) (failure : Int → Error)
    (value : Int) : Spec State Error (SpecInt width signed) :=
  if fits : IntegerValueFits width signed value then Spec.pure ⟨value, fits⟩
  else Spec.abort (failure value)

theorem wp_checkedInteger (width : IntWidth) (signed : Bool) (failure : Int → Error)
    (value : Int) (ensures : SpecInt width signed → State → Prop)
    (aborts : Error → Prop) (initial : State) :
    wp (checkedInteger width signed failure value) ensures aborts initial ↔
      (∀ fits : IntegerValueFits width signed value, ensures ⟨value, fits⟩ initial) ∧
      (¬IntegerValueFits width signed value → aborts (failure value)) := by
  by_cases fits : IntegerValueFits width signed value <;>
    simp [checkedInteger, fits]

/-- A checked result may be named as a native local before continuing. Its
value equation is explicit, so later modular calls need not unfold local
definitions to recover arithmetic facts. -/
theorem wp_checkedInteger_value (width : IntWidth) (signed : Bool) (failure : Int → Error)
    (value : Int) (ensures : SpecInt width signed → State → Prop)
    (aborts : Error → Prop) (initial : State) :
    wp (checkedInteger width signed failure value) ensures aborts initial ↔
      (∀ result, result.val = value → ensures result initial) ∧
      (¬IntegerValueFits width signed value → aborts (failure value)) := by
  rw [wp_checkedInteger]
  constructor
  · rintro ⟨normal, failing⟩
    refine ⟨?_, failing⟩
    rintro ⟨actual, fits⟩ rfl
    exact normal fits
  · rintro ⟨normal, failing⟩
    exact ⟨fun fits => normal ⟨value, fits⟩ rfl, failing⟩

/-- Compatibility with the current contract failure vocabulary. Only the
failure payload crosses this boundary; arithmetic operands/results do not. -/
def runtimeFailure (kind : ThrowKind) (value : Int) : Failure :=
  (kind, #[.integer value])

theorem checkedInteger_preserves (width : IntWidth) (signed : Bool)
    (failure : Int → Error) (value : Int) :
    StatePreserving (checkedInteger (State := State) width signed failure value) := by
  intro initial result final executed
  unfold checkedInteger at executed
  split at executed
  · exact executed.2
  · exact False.elim executed

end LeanerIR.Proofs.NativeArithmetic
