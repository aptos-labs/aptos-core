-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Typed

/-!
# Values whose runtime representation includes storage

An owned logical value can contain information absent from its physical value.
For example, Table snapshots retain contents while physical Tables retain only
their allocation handle. Such an erasure is not an injective `Codec`.

This boundary recovers a logical value from a physical value and a state. It
does not make state part of the logical value or require operations on it to
read storage. Recovery failure is an undefined outcome, never a discarded run.
Agreement of an abstract implementation with the lifted runtime computation
remains a separate proof obligation.
-/

namespace LeanerIR.Proofs

structure StateView (State Native Runtime : Type) where
  erase : Native → Runtime
  recover? : State → Runtime → Option Native
  erase_recover : ∀ state raw value, recover? state raw = some value → erase value = raw

namespace StateView

/-- Select the state component used by one value's representation. -/
def atState (view : StateView State Native Runtime) (select : OtherState → State) :
    StateView OtherState Native Runtime where
  erase := view.erase
  recover? := fun state => view.recover? (select state)
  erase_recover := fun state => view.erase_recover (select state)

/-- Product/argument-row composition preserves each child's logical value. -/
def prod (left : StateView State NativeLeft RuntimeLeft)
    (right : StateView State NativeRight RuntimeRight) :
    StateView State (NativeLeft × NativeRight) (RuntimeLeft × RuntimeRight) where
  erase := fun values => (left.erase values.1, right.erase values.2)
  recover? := fun state raw => do
    let first ← left.recover? state raw.1
    let second ← right.recover? state raw.2
    pure (first, second)
  erase_recover := by
    intro state raw value recovered
    simp only [Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
      Option.some.injEq] at recovered
    obtain ⟨first, firstRecovered, second, secondRecovered, rfl⟩ := recovered
    exact Prod.ext (left.erase_recover state raw.1 first firstRecovered)
      (right.erase_recover state raw.2 second secondRecovered)

/-- A prophecy pair observes its two values in their respective states. In
particular, equal Table handles do not force equal current/final contents.
The caller must supply states resolving the appropriate loans; this operation
does not itself establish a borrowing or ownership certificate. -/
def prophecy (view : StateView State Native Runtime) :
    StateView (State × State) (Native × Native) (Runtime × Runtime) :=
  (view.atState Prod.fst).prod (view.atState Prod.snd)

/-- Ordinary tightly represented values are a state-independent special case.
The tightness premise rejects noncanonical runtime aliases. -/
def ofCodec (codec : Codec Native Runtime)
    (tight : ∀ raw value, codec.decode? raw = some value → codec.encode value = raw) :
    StateView State Native Runtime where
  erase := codec.encode
  recover? := fun _ => codec.decode?
  erase_recover := fun _ => tight

/-- A live logical value agrees with its physical representation in this state.
Historical and functional snapshots need not agree with the current state. -/
def Realizes (view : StateView State Native Runtime) (state : State) (value : Native) : Prop :=
  view.recover? state (view.erase value) = some value

theorem realizes_prod (left : StateView State NativeLeft RuntimeLeft)
    (right : StateView State NativeRight RuntimeRight) (state : State)
    (values : NativeLeft × NativeRight) :
    (left.prod right).Realizes state values ↔
      left.Realizes state values.1 ∧ right.Realizes state values.2 := by
  cases values
  simp [Realizes, prod, Option.bind_eq_some_iff]

theorem realizes_prophecy (view : StateView State Native Runtime)
    (states : State × State) (values : Native × Native) :
    view.prophecy.Realizes states values ↔
      view.Realizes states.1 values.1 ∧ view.Realizes states.2 values.2 :=
  realizes_prod _ _ _ _

theorem realizes_of_recover (view : StateView State Native Runtime)
    (state : State) (raw : Runtime) (value : Native)
    (recovered : view.recover? state raw = some value) : view.Realizes state value := by
  unfold Realizes
  rw [view.erase_recover state raw value recovered]
  exact recovered

/-- Erasure is injective only among values realized in the same state. -/
theorem erase_injective_at (view : StateView State Native Runtime) (state : State)
    {left right : Native} (leftLive : view.Realizes state left)
    (rightLive : view.Realizes state right) (same : view.erase left = view.erase right) :
    left = right := by
  unfold Realizes at leftLive rightLive
  rw [same, rightLive] at leftLive
  exact (Option.some.inj leftLive).symm

/-- Recover results at the final state, preserving aborts and undefined behavior.
A runtime success which cannot be represented is itself an undefined outcome. -/
def lift (view : StateView State Native Runtime) (action : Spec State Error Runtime) :
    Spec State Error Native where
  ok := fun initial result final =>
    ∃ raw, action.ok initial raw final ∧ view.recover? final raw = some result
  aborts := action.aborts
  undefined := fun initial =>
    action.undefined initial ∨
      ∃ raw final, action.ok initial raw final ∧ view.recover? final raw = none

/-- Ordinary codecs retain precisely their existing computation semantics. -/
theorem lift_ofCodec (codec : Codec Native Runtime)
    (tight : ∀ raw value, codec.decode? raw = some value → codec.encode value = raw)
    (action : Spec State Error Runtime) :
    (ofCodec codec tight).lift action = decodeSpec codec action := rfl

/-- Refinement includes failures to recover an otherwise successful result. -/
theorem lift_refines (view : StateView State Native Runtime)
    {left right : Spec State Error Runtime} (refines : Spec.Refines left right) :
    Spec.Refines (view.lift left) (view.lift right) where
  ok := by
    rintro initial value final ⟨raw, execution, recovered⟩
    exact ⟨raw, refines.ok initial raw final execution, recovered⟩
  aborts := refines.aborts
  undefined := by
    rintro initial (undefined | ⟨raw, final, execution, missing⟩)
    · exact .inl (refines.undefined initial undefined)
    · exact .inr ⟨raw, final, refines.ok initial raw final execution, missing⟩

theorem wp_lift (view : StateView State Native Runtime) (action : Spec State Error Runtime)
    (ensures : Native → State → Prop) (aborts : Error → Prop) (initial : State) :
    wp (view.lift action) ensures aborts initial ↔
      wp action (fun raw final =>
        ∃ value, view.recover? final raw = some value ∧ ensures value final) aborts initial := by
  constructor
  · rintro ⟨normal, failing, defined⟩
    refine ⟨?_, failing, fun undefined => defined (.inl undefined)⟩
    intro raw final execution
    cases recovered : view.recover? final raw with
    | none => exact False.elim (defined (.inr ⟨raw, final, execution, recovered⟩))
    | some value => exact ⟨value, rfl, normal value final ⟨raw, execution, recovered⟩⟩
  · rintro ⟨normal, failing, defined⟩
    refine ⟨?_, failing, ?_⟩
    · rintro value final ⟨raw, execution, recovered⟩
      obtain ⟨other, otherRecovered, established⟩ := normal raw final execution
      rw [recovered] at otherRecovered
      cases otherRecovered
      exact established
    · rintro (undefined | ⟨raw, final, execution, missing⟩)
      · exact defined undefined
      · obtain ⟨value, recovered, _⟩ := normal raw final execution
        rw [missing] at recovered
        cases recovered

/-- A function boundary also checks that its logical arguments belong to the
entry state. An inconsistent input is undefined, not a vacuous precondition. -/
def function (arguments : StateView State NativeArgs RuntimeArgs)
    (results : StateView State NativeResult RuntimeResult)
    (runtime : RuntimeArgs → Spec State Error RuntimeResult)
    (args : NativeArgs) : Spec State Error NativeResult where
  ok := fun initial result final => arguments.Realizes initial args ∧
    (results.lift (runtime (arguments.erase args))).ok initial result final
  aborts := fun initial error => arguments.Realizes initial args ∧
    (runtime (arguments.erase args)).aborts initial error
  undefined := fun initial => ¬arguments.Realizes initial args ∨
    (results.lift (runtime (arguments.erase args))).undefined initial

theorem wp_function (arguments : StateView State NativeArgs RuntimeArgs)
    (results : StateView State NativeResult RuntimeResult)
    (runtime : RuntimeArgs → Spec State Error RuntimeResult) (args : NativeArgs)
    (ensures : NativeResult → State → Prop) (aborts : Error → Prop) (initial : State) :
    wp (arguments.function results runtime args) ensures aborts initial ↔
      arguments.Realizes initial args ∧
        wp (results.lift (runtime (arguments.erase args))) ensures aborts initial := by
  classical
  constructor
  · rintro ⟨normal, failing, defined⟩
    have valid : arguments.Realizes initial args :=
      Classical.byContradiction fun invalid => defined (.inl invalid)
    exact ⟨valid, fun value final execution => normal value final ⟨valid, execution⟩,
      fun error execution => failing error ⟨valid, execution⟩,
      fun undefined => defined (.inr undefined)⟩
  · rintro ⟨valid, normal, failing, defined⟩
    exact ⟨fun value final execution => normal value final execution.2,
      fun error execution => failing error execution.2,
      fun | .inl invalid => invalid valid | .inr undefined => defined undefined⟩

end StateView
end LeanerIR.Proofs
