-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Typed

/-!
# Native computations and their representation boundary

A computation is an ordinary `Spec` with native arguments, locals and results.
It does not execute an encoded function and then decode its answer. Encoding
appears only in `Represents`, the exact agreement obligation with execution.

In particular, agreement is not inferred from a contract: contracts can be
nondeterministic, and a codec need only have a left inverse. Both would make
recovering an encoded execution from a verified postcondition unsound.
-/

namespace LeanerIR.Proofs

namespace Spec

theorem bind_assoc (first : Spec State Error A) (second : A → Spec State Error B)
    (third : B → Spec State Error C) :
    (first.bind second).bind third = first.bind (fun a => (second a).bind third) := by
  apply extensionality
  · funext initial result final
    apply propext
    constructor
    · rintro ⟨b, middle₂, ⟨a, middle₁, one, two⟩, three⟩
      exact ⟨a, middle₁, one, b, middle₂, two, three⟩
    · rintro ⟨a, middle₁, one, b, middle₂, two, three⟩
      exact ⟨b, middle₂, ⟨a, middle₁, one, two⟩, three⟩
  · funext initial error
    apply propext
    constructor
    · rintro ((one | ⟨a, middle₁, one, two⟩) | ⟨b, middle₂, ⟨a, middle₁, one, two⟩, three⟩)
      · exact Or.inl one
      · exact Or.inr ⟨a, middle₁, one, Or.inl two⟩
      · exact Or.inr ⟨a, middle₁, one, Or.inr ⟨b, middle₂, two, three⟩⟩
    · rintro (one | ⟨a, middle₁, one, two | ⟨b, middle₂, two, three⟩⟩)
      · exact Or.inl (Or.inl one)
      · exact Or.inl (Or.inr ⟨a, middle₁, one, two⟩)
      · exact Or.inr ⟨b, middle₂, ⟨a, middle₁, one, two⟩, three⟩
  · funext initial
    apply propext
    constructor
    · rintro ((one | ⟨a, middle₁, one, two⟩) | ⟨b, middle₂, ⟨a, middle₁, one, two⟩, three⟩)
      · exact Or.inl one
      · exact Or.inr ⟨a, middle₁, one, Or.inl two⟩
      · exact Or.inr ⟨a, middle₁, one, Or.inr ⟨b, middle₂, two, three⟩⟩
    · rintro (one | ⟨a, middle₁, one, two | ⟨b, middle₂, two, three⟩⟩)
      · exact Or.inl (Or.inl one)
      · exact Or.inl (Or.inr ⟨a, middle₁, one, two⟩)
      · exact Or.inr ⟨b, middle₂, ⟨a, middle₁, one, two⟩, three⟩

end Spec

/-- An unconditional semantic effect certificate, distinct from a contract's
frame guarantee under its precondition. Used only at representation boundaries. -/
def StatePreserving (computation : Spec State Error Result) : Prop :=
  ∀ initial result final, computation.ok initial result final → final = initial

theorem StatePreserving.pure (value : Result) :
    StatePreserving (Spec.pure value : Spec State Error Result) :=
  fun _ _ _ executed => executed.2

theorem StatePreserving.branch (test : Bool) {left right : Spec State Error Result}
    (yes : StatePreserving left) (no : StatePreserving right) :
    StatePreserving (if test then left else right) := by
  cases test <;> assumption

/-- Select the outer computational branch without splitting matches inside
its typed condition. -/
theorem wp_branch (test : Prop) [Decidable test] (left right : Spec State Error Result)
    (ensures : Result → State → Prop) (aborts : Error → Prop) (initial : State) :
    wp (if test then left else right) ensures aborts initial ↔
      (test → wp left ensures aborts initial) ∧
      (¬test → wp right ensures aborts initial) := by
  by_cases h : test <;> simp [h]

theorem StatePreserving.bind {first : Spec State Error Local}
    {next : Local → Spec State Error Result}
    (firstPreserves : StatePreserving first)
    (nextPreserves : ∀ value, StatePreserving (next value)) :
    StatePreserving (Spec.bind first next) := by
  rintro initial result final ⟨value, middle, firstStep, nextStep⟩
  exact (nextPreserves value middle result final nextStep).trans
    (firstPreserves initial value middle firstStep)

/-- The complete call rule does not assume the callee cannot abort. Normal
executions retain the conditional postcondition and refute must-abort clauses. -/
theorem wp_of_satisfies_complete
    {function : Args → Spec State Error Result} {contract : Contract State Error Args Result}
    (verified : Satisfies function contract) (args : Args) (initial : State)
    (permitted : contract.requires args initial) :
    wp (function args)
      (fun result final =>
        (¬contract.mayAbort args initial → contract.ensures args initial result final) ∧
        contract.frame args initial final ∧ ¬contract.mustAbort args initial)
      (contract.aborts args initial) initial :=
  verified args initial permitted

/-- Encode only a computation's result, preserving its state, failures and
undefined behavior exactly. This is a semantic boundary, not a VC operation. -/
def encodeSpec (codec : Codec Native Runtime)
    (computation : Spec State Error Native) : Spec State Error Runtime where
  ok := fun initial result final =>
    ∃ native, computation.ok initial native final ∧ result = codec.encode native
  aborts := computation.aborts
  undefined := computation.undefined

/-- Exact agreement on represented arguments. Runtime results must be the
canonical encoding, not merely some value that happens to decode. -/
def Represents (arguments : Codec NativeArgs RuntimeArgs)
    (results : Codec NativeResult RuntimeResult)
    (computation : NativeArgs → Spec State Error NativeResult)
    (execution : RuntimeArgs → Spec State Error RuntimeResult) : Prop :=
  ∀ args, Spec.Equiv (execution (arguments.encode args))
    (encodeSpec results (computation args))

/-- Retain the exact state effect of a pure native body in its modular
summary. Authored frame clauses can be weaker (in particular about execution
metadata); callers must not have to reopen the body to recover this fact. -/
def Contract.withStateFrame (contract : Contract State Error Args Result) :
    Contract State Error Args Result :=
  { contract with frame := fun args initial final =>
      final = initial ∧ contract.frame args initial final }

theorem satisfies_of_stateFrame
    {computation : Args → Spec State Error Result}
    {contract : Contract State Error Args Result}
    (verified : Satisfies computation contract.withStateFrame) :
    Satisfies computation contract := by
  intro args initial permitted
  have established := verified args initial permitted
  refine ⟨?_, established.2⟩
  intro result final execution
  have normal := established.1 result final execution
  exact ⟨normal.1, normal.2.1.2, normal.2.2⟩

/-- Modular call elimination for state-preserving summaries whose normal
return excludes the may-abort guard. The callee body remains opaque; neither
its execution agreement nor an unconditional effect certificate is needed. -/
theorem wp_of_stateFrame_satisfies
    {function : Args → Spec State Error Result} {contract : Contract State Error Args Result}
    (verified : Satisfies function contract.withStateFrame) (args : Args) (initial : State)
    (permitted : contract.requires args initial)
    (precise : contract.mayAbort args initial → contract.mustAbort args initial)
    {post : Result → State → Prop} {aborts : Error → Prop}
    (normal : ∀ result, contract.ensures args initial result initial →
      ¬contract.mustAbort args initial → post result initial)
    (aborted : ∀ error, contract.aborts args initial error → aborts error) :
    wp (function args) post aborts initial := by
  apply wp_mono (wp_of_satisfies_complete verified args initial permitted)
  · intro result final established
    obtain ⟨output, ⟨sameState, _⟩, noAbort⟩ := established
    subst final
    exact normal result (output (fun possible => noAbort (precise possible))) noAbort
  · exact aborted

/-- Preserve a native result equation alongside the authored postcondition.
Callers use this equation directly, without decoding an encoded postcondition
or reopening the callee body. -/
def Contract.withResult (contract : Contract State Error Args Result)
    (expected : Args → Result) : Contract State Error Args Result :=
  { contract with ensures := fun args initial result final =>
      result = expected args ∧ contract.ensures args initial result final }

theorem satisfies_of_result
    {computation : Args → Spec State Error Result}
    {contract : Contract State Error Args Result} {expected : Args → Result}
    (verified : Satisfies computation (contract.withResult expected)) :
    Satisfies computation contract := by
  intro args initial permitted
  have established := verified args initial permitted
  refine ⟨?_, established.2⟩
  intro result final execution
  have normal := established.1 result final execution
  exact ⟨fun noAbort => (normal.1 noAbort).2, normal.2⟩

theorem encodeSpec_congr (codec : Codec Native Runtime)
    {left right : Spec State Error Native} (agreement : Spec.Equiv left right) :
    Spec.Equiv (encodeSpec codec left) (encodeSpec codec right) := by
  constructor
  · intro initial result final
    simp only [encodeSpec, agreement.ok]
  · exact agreement.aborts
  · exact agreement.undefined

theorem encodeSpec_pure (codec : Codec Native Runtime) (value : Native) :
    Spec.Equiv (encodeSpec codec (Spec.pure value : Spec State Error Native))
      (Spec.pure (codec.encode value)) := by
  constructor
  · intro initial result final
    constructor
    · rintro ⟨native, ⟨rfl, rfl⟩, rfl⟩
      exact ⟨rfl, rfl⟩
    · rintro ⟨rfl, rfl⟩
      exact ⟨value, ⟨rfl, rfl⟩, rfl⟩
  · intro initial error; rfl
  · intro initial; rfl

theorem encodeSpec_abort (codec : Codec Native Runtime) (error : Error) :
    Spec.Equiv (encodeSpec codec (Spec.abort error : Spec State Error Native))
      (Spec.abort error) := by
  constructor
  · intro initial result final; simp [encodeSpec, Spec.abort]
  · intro initial actual; rfl
  · intro initial; rfl

/-- Roundtrip for an entire computation, including undefined behavior. No
right-inverse property of the decoder is needed. -/
theorem decodeSpec_encodeSpec (codec : Codec Native Runtime)
    (computation : Spec State Error Native) :
    Spec.Equiv (decodeSpec codec (encodeSpec codec computation)) computation := by
  constructor
  · intro initial result final
    constructor
    · rintro ⟨runtime, ⟨native, execution, rfl⟩, decoded⟩
      rw [codec.decode_encode] at decoded
      cases decoded
      exact execution
    · intro execution
      exact ⟨codec.encode result, ⟨result, execution, rfl⟩,
        codec.decode_encode result⟩
  · intro initial error; rfl
  · intro initial
    constructor
    · rintro (undefined | ⟨runtime, final, ⟨native, _, rfl⟩, decoded⟩)
      · exact undefined
      · rw [codec.decode_encode] at decoded
        cases decoded
    · exact Or.inl

theorem decodeSpec_congr (codec : Codec Native Runtime)
    {left right : Spec State Error Runtime} (agreement : Spec.Equiv left right) :
    Spec.Equiv (decodeSpec codec left) (decodeSpec codec right) := by
  constructor
  · intro initial result final
    simp only [decodeSpec, agreement.ok]
  · exact agreement.aborts
  · intro initial
    simp only [decodeSpec, agreement.ok, agreement.undefined]

namespace Represents

/-- Cross the representation boundary once, after proving the native body.
The native proof need not know the executable, codecs, frames or type map. -/
theorem typed {arguments : Codec NativeArgs RuntimeArgs}
    {results : Codec NativeResult RuntimeResult}
    {computation : NativeArgs → Spec State Error NativeResult}
    {execution : RuntimeArgs → Spec State Error RuntimeResult}
    (agreement : Represents arguments results computation execution) (args : NativeArgs) :
    Spec.Equiv (typedFunction arguments results execution args) (computation args) :=
  (decodeSpec_congr results (agreement args)).trans
    (decodeSpec_encodeSpec results (computation args))

/-- Transport a verified native computation to the authoritative runtime
function. This is separate from, and never unfolded during, native VC work. -/
theorem satisfies {arguments : Codec NativeArgs RuntimeArgs}
    {results : Codec NativeResult RuntimeResult}
    {computation : NativeArgs → Spec State Error NativeResult}
    {execution : RuntimeArgs → Spec State Error RuntimeResult}
    (agreement : Represents arguments results computation execution)
    {contract : Contract State Error NativeArgs NativeResult}
    (verified : Satisfies computation contract) :
    Satisfies execution (contract.runtime arguments results) :=
  satisfies_runtime arguments results execution contract
    ((satisfies_congr agreement.typed contract).mpr verified)

end Represents
end LeanerIR.Proofs
