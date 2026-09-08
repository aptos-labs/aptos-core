-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

set_option leaner.route "native"

/-!
# Storage-parametric V1 denotations

A type parameter which does not determine a global resource key is represented
by one abstract inhabited carrier.  It may still occur in values, signatures,
aggregates, and calls.  The generic bodies below are therefore denoted and
proved once; concrete callers reach the same denotation at distinct type
arguments.
-/

namespace LeanerLang.Tests.VerificationGenerics

open Lean Elab Command

syntax "#guard_parametric_generic_denotations" : command

elab_rules : command
  | `(command| #guard_parametric_generic_denotations) => do
      let root : Array String := #["0x42", "verification_generics"]
      let handle : LeanerIR.FunctionHandle :=
        { namespaceId := ⟨0⟩, functionId := ⟨0⟩ }
      let canonicalDeclaration :=
        LeanerLang.Denotation.relationAgreementName
          (LeanerLang.Denotation.functionSegments root handle) "carry"
      let duplicateDeclaration :=
        LeanerLang.Denotation.relationAgreementName root "carry"
      let currentNamespace ← getCurrNamespace
      let environment ← getEnv
      unless environment.contains (currentNamespace ++ canonicalDeclaration) do
        throwError "the shared parametric carry agreement theorem was not emitted"
      if environment.contains (currentNamespace ++ duplicateDeclaration) then
        throwError "the parametric carry body received a duplicate agreement theorem"
      let some unit := LeanerLang.registeredUnit? environment
          `«0x42».verification_generics
        | throwError "the generic denotation fixture was not registered"
      match LeanerLang.Denotation.checkFunction unit
          { namespaceId := ⟨0⟩, functionId := ⟨3⟩ } with
      | .ok 1 => pure ()
      | .ok count =>
          throwError "the parametric forwarding function reported {count} type parameters"
      | .error reason =>
          throwError "a generic call without a storage-key use was rejected: {reason}"
      match LeanerLang.Denotation.checkFunction unit
          { namespaceId := ⟨0⟩, functionId := ⟨4⟩ } with
      | .ok _ =>
          throwError "a storage-key-dependent type parameter was accepted as parametric"
      | .error reason =>
          unless reason.startsWith
              "generic type parameter `T` determines a global resource key:" do
            throwError "unexpected storage-key diagnostic: {reason}"
      match LeanerLang.Denotation.checkFunction unit
          { namespaceId := ⟨0⟩, functionId := ⟨5⟩ } with
      | .ok _ =>
          throwError "transitive storage-key dependence was accepted as parametric"
      | .error reason =>
          unless reason.contains "passes it to global operation" do
            throwError "storage-key dependence did not propagate through the call: {reason}"

leaner module 0x42::verification_generics where
  public fun carry {T : type}(value : T) -> T := value

  spec carry where
    pragma aborts_if_is_strict
    ensures result == value

  public fun carry_u64(value : u64) -> u64 :=
    core.call carry::<u64>(value)

  spec carry_u64 where
    pragma aborts_if_is_strict
    ensures result == value

  public fun carry_bool(value : Bool) -> Bool :=
    core.call carry::<Bool>(value)

  spec carry_bool where
    pragma aborts_if_is_strict
    ensures result == value

  -- Passing `T` to another generic function does not make it non-parametric.
  public fun forward {T : type}(value : T) -> T :=
    core.call carry::<T>(value)

  spec forward where
    pragma aborts_if_is_strict
    ensures result == value

  -- In contrast, `T` itself is the global resource family.  This function
  -- must be specialized for the reachable concrete instantiations.
  public fun has_resource {T : type has Key}(addr : Address) -> Bool :=
    exists<T>(addr)

  -- Storage-key dependence propagates through direct generic calls.
  public fun calls_has_resource {T : type has Key}(addr : Address) -> Bool :=
    core.call has_resource::<T>(addr)

  verify carry
  verify carry_u64
  verify carry_bool
  verify forward

#leaner_require_native 0x42::verification_generics::carry
#leaner_require_native 0x42::verification_generics::carry_u64
#leaner_require_native 0x42::verification_generics::carry_bool
#leaner_require_native 0x42::verification_generics::forward

/-! Direct verification and both concrete callers must share the same
handle-keyed agreement theorem; no entry-point-keyed copy may exist.  Generic
forwarding stays parametric, while a resource-key use is classified for
specialization. -/
#guard_parametric_generic_denotations

/-! V3 retains the generic body as one theorem over an abstract carrier.
The proof-facing boundary contains no runtime value row. -/
example {Carrier : Nat → Type}
    (arguments : «0x42».verification_generics.carry.Arguments Carrier) :
    LeanerIR.Proofs.Spec LeanerIR.RuntimeState LeanerIR.Proofs.Failure (Carrier 0) :=
  «0x42».verification_generics.carry.computation arguments

/-! The generated local frame preserves the carrier type as well.  Initial
parameter placement is a definitional projection, with `Option` tracking the
Move local's availability independently of its value representation. -/
example (Carrier : Nat → Type) : Type := «0x42».verification_generics.carry.Locals Carrier

example {Carrier : Nat → Type}
    (arguments : «0x42».verification_generics.carry.Arguments Carrier) :
    («0x42».verification_generics.carry.initialLocals arguments).local0 =
      some arguments.value := rfl

end LeanerLang.Tests.VerificationGenerics
