-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.Focus
import LeanerIR.Proofs.SimpAttrs

/-!
# Loan-free values, decided by simp

`Plain` is an inductive predicate; a proof of it for a literal value, or for
the erasure of a typed twin, is a matter of computation.  The constructor
forms below let `simp` decide it structurally, and every generated twin
contributes its `plain_erase` to the same set, so `leaner_plain` closes the
loan-freedom of any value a bracket writes back or reborrows through.
-/

namespace LeanerIR.SemanticOperations

@[leaner_plain] theorem Plain.unit_iff : Plain .unit ↔ True :=
  ⟨fun _ => trivial, fun _ => .unit⟩
@[leaner_plain] theorem Plain.bool_iff (value : Bool) : Plain (.bool value) ↔ True :=
  ⟨fun _ => trivial, fun _ => .bool value⟩
@[leaner_plain] theorem Plain.character_iff (value : Nat) :
    Plain (.character value) ↔ True :=
  ⟨fun _ => trivial, fun _ => .character value⟩
@[leaner_plain] theorem Plain.integer_iff (value : Int) : Plain (.integer value) ↔ True :=
  ⟨fun _ => trivial, fun _ => .integer value⟩
@[leaner_plain] theorem Plain.address_iff (value : String) :
    Plain (.address value) ↔ True :=
  ⟨fun _ => trivial, fun _ => .address value⟩
@[leaner_plain] theorem Plain.signer_iff (value : String) : Plain (.signer value) ↔ True :=
  ⟨fun _ => trivial, fun _ => .signer value⟩
@[leaner_plain] theorem Plain.string_iff (value : String) : Plain (.string value) ↔ True :=
  ⟨fun _ => trivial, fun _ => .string value⟩
@[leaner_plain] theorem Plain.bytes_iff (value : Array UInt8) :
    Plain (.bytes value) ↔ True :=
  ⟨fun _ => trivial, fun _ => .bytes value⟩
@[leaner_plain] theorem Plain.vector_iff (elements : Array RuntimeValue) :
    Plain (.vector elements) ↔ ∀ element ∈ elements, Plain element :=
  ⟨fun plain => by cases plain with | vector _ plain => exact plain, .vector elements⟩
@[leaner_plain] theorem Plain.tuple_iff (elements : Array RuntimeValue) :
    Plain (.tuple elements) ↔ ∀ element ∈ elements, Plain element :=
  ⟨fun plain => by cases plain with | tuple _ plain => exact plain, .tuple elements⟩
@[leaner_plain] theorem Plain.nominal_iff (source : StructHandle) (variant : Option String)
    (fields : Array RuntimeValue) :
    Plain (.nominal source variant fields) ↔ ∀ field ∈ fields, Plain field :=
  ⟨fun plain => by cases plain with | nominal _ _ _ plain => exact plain,
    .nominal source variant fields⟩
@[leaner_plain] theorem Plain.closure_iff (function : FunctionHandle)
    (captures : Array RuntimeValue) :
    Plain (.closure function captures) ↔ ∀ capture ∈ captures, Plain capture :=
  ⟨fun plain => by cases plain with | closure _ _ plain => exact plain,
    .closure function captures⟩
@[leaner_plain] theorem Plain.borrow_iff (loan : Nat) (current : RuntimeValue) :
    Plain (.borrow loan current) ↔ False :=
  ⟨fun plain => (nomatch plain), False.elim⟩
@[leaner_plain] theorem Plain.loanHole_iff (loan : Nat) : Plain (.loanHole loan) ↔ False :=
  ⟨fun plain => (nomatch plain), False.elim⟩

/-- Decide the loan-freedom of a value, or of every sibling along a focused
path, by computation. -/
syntax "leaner_plain" : tactic

macro_rules
  | `(tactic| leaner_plain) =>
      `(tactic| simp [LeanerIR.SemanticOperations.PlainSteps,
          LeanerIR.SemanticOperations.FocusStep.Plain, leaner_plain])

end LeanerIR.SemanticOperations
