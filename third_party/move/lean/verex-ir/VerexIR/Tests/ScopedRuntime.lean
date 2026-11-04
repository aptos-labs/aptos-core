-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Validation.Check
import VerexIR.Proofs.Interpreter
import VerexIR.Semantics.ScopedMethods
import VerexIR.Proofs.Denote.Compile

namespace VerexIR.Tests.ScopedRuntime
open VerexIR VerexIR.Import VerexIR.Validation

private def use (id : Nat) : TypeUse := { typeId := ⟨id⟩, loc := ⟨0⟩ }
private def requirements : Array Attribute := #[
  .assign "method_evidence" (.constant (.integer 1)),
  .assign "scoped_method_evidence" (.constant (.integer 1))]
private def declaration (name root result : Nat) (parameters : Array Nat)
    (needsScope := false) : FunctionDecl RawBody := {
  loc := ⟨0⟩, name := ⟨name⟩, profile := .rust
  attributes := if needsScope then requirements else #[]
  signature := {
    parameters := parameters.zipIdx.map fun (type, index) =>
      { name := s!"p{index}", typeUse := use type, mutable := true }
    results := #[use result] }
  locals := parameters.zipIdx.map fun (type, index) =>
    { id := ⟨index⟩, name := s!"p{index}", type := use type, mutable := true, loc := ⟨0⟩ }
  body := .structured ⟨root⟩, origin := ⟨0⟩, alignment := ⟨0⟩ }

private def fixture : RawUnit := {
  tables := {
    files := #[{ name := "scoped_runtime.rs" }]
    locations := #[{ primary := some { file := ⟨0⟩, startByte := 0, endByte := 1 } }]
    origins := #[{ kind := .rustMir, location := ⟨0⟩ }]
    alignments := #[{ source := ⟨0⟩, trust := .checked, description := "scoped runtime" }]
    lifetimes := #[{ kind := .inference, loc := ⟨0⟩ }]
    types := #[.integer (.bits 64) false,
      .reference { profile := .rust, kind := .mutable, referent := ⟨0⟩, lifetime := ⟨0⟩ },
      .function #[⟨1⟩] ⟨1⟩ #[.copy, .drop], .function #[⟨1⟩] ⟨0⟩ #[.copy, .drop]]
    namespaces := #[{ segments := #["scoped_runtime"] }]
    names := #["identity", "scalar", "runner", "relay", "safe", "captured_scalar"].map
      fun name => { namespaceId := ⟨0⟩, name } }
  profiles := #[{ profile := .rust, name := "rust-test" }]
  namespaces := #[{
    loc := ⟨0⟩, identity := ⟨0⟩, profile := some .rust
    expressions := #[
      { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .localVar ⟨0⟩ },
      { loc := ⟨0⟩, typeId := ⟨0⟩, kind := .value (.integer 7) },
      { loc := ⟨0⟩, typeId := ⟨2⟩, kind := .localVar ⟨1⟩ },
      { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .operation (.call .invoke) #[] #[⟨2⟩, ⟨0⟩] },
      { loc := ⟨0⟩, typeId := ⟨3⟩, kind := .localVar ⟨1⟩ },
      { loc := ⟨0⟩, typeId := ⟨0⟩, kind := .operation (.call .invoke) #[] #[⟨4⟩, ⟨0⟩] },
      { loc := ⟨0⟩, typeId := ⟨0⟩, kind := .localVar ⟨1⟩ }]
    functions := #[declaration 0 0 1 #[1], declaration 1 1 0 #[1],
      declaration 2 3 1 #[1, 2] true, declaration 3 3 1 #[1, 2],
      declaration 4 5 0 #[1, 3] true, declaration 5 6 0 #[1, 0]] }] }

private def executable? : Option ((unit : ValidatedUnit) × ExecutableUnit unit) := do
  let checked ← (validate #[{ profile := .rust, name := "rust-test" }] fixture).toOption
  let semantics : SemanticProfile := { profile := .rust, name := "rust-test", classify := fun _ _ => none }
  (prepareExecution #[semantics] checked).toOption.map (⟨checked, ·⟩)

#guard executable?.isSome

private def bad : RuntimeValue := .closure ⟨⟨0⟩, ⟨0⟩⟩ 0 #[] #[]
private def good : RuntimeValue := .closure ⟨⟨0⟩, ⟨1⟩⟩ 0 #[] #[]
private def nestedBad : RuntimeValue := .closure ⟨⟨0⟩, ⟨2⟩⟩ 2 #[] #[bad]

-- A real borrowed receiver with no escape receipt cannot enter a scoped body.
#guard match executable? with
  | some ⟨_, executable⟩ =>
      match Interpreter.run executable 50 ⟨⟨0⟩, ⟨2⟩⟩ #[.borrow 0 (.integer 9), bad] with
      | .error error => error.value.code == "VIR-EXEC-UNSCOPED-METHOD-EVIDENCE"
      | .ok _ => false
  | none => false

-- A nominally valid closure target cannot hide an unsafe captured dictionary.
#guard match executable? with
  | some ⟨unit, executable⟩ =>
      receiverScopedTarget unit ⟨⟨0⟩, ⟨2⟩⟩ 2 && !receiverScopedValue unit nestedBad &&
      match Interpreter.run executable 50 ⟨⟨0⟩, ⟨3⟩⟩ #[.borrow 0 (.integer 9), nestedBad] with
      | .error error => error.value.code == "VIR-EXEC-UNSCOPED-METHOD-EVIDENCE" && !error.callers.isEmpty
      | .ok _ => false
  | none => false

-- A checked receiver receipt permits the dictionary and leaves its result intact.
#guard match executable? with
  | some ⟨unit, executable⟩ =>
      receiverScopedValue unit good &&
      match Interpreter.run executable 50 ⟨⟨0⟩, ⟨4⟩⟩ #[.borrow 0 (.integer 9), good] with
      | .ok (_, outcome) => outcome.value == .returned #[.integer 7]
      | .error _ => false
  | none => false

private def nestedGood : RuntimeValue := .closure ⟨⟨0⟩, ⟨4⟩⟩ 2 #[] #[good]

#guard match executable? with
  | some ⟨unit, executable⟩ =>
      receiverScopedValue unit nestedGood &&
      match Interpreter.run executable 50 ⟨⟨0⟩, ⟨4⟩⟩ #[.borrow 0 (.integer 9), nestedGood] with
      | .ok (_, outcome) => outcome.value == .returned #[.integer 7]
      | .error _ => false
  | none => false

-- A missing captured dictionary cannot discharge the recursive requirement.
#guard match executable? with
  | some ⟨unit, _⟩ =>
      !receiverScopedValue unit (.closure ⟨⟨0⟩, ⟨4⟩⟩ 2 #[] #[]) &&
      !receiverScopedValue unit (.closure ⟨⟨0⟩, ⟨0⟩⟩ 0 #[(⟨0⟩, ⟨0⟩)] #[])
  | none => false

-- A receipt-bearing scalar target cannot be forged into a reference target,
-- given an out-of-range mask, or supplied captures its signature does not bind.
#guard match executable? with
  | some ⟨unit, _⟩ =>
      receiverScopedTarget unit ⟨⟨0⟩, ⟨1⟩⟩ 0 &&
      !receiverScopedValue unit (.closure ⟨⟨0⟩, ⟨1⟩⟩ 0 #[(⟨0⟩, ⟨1⟩)] #[]) &&
      !receiverScopedValue unit (.closure ⟨⟨0⟩, ⟨1⟩⟩ 2 #[] #[]) &&
      !receiverScopedValue unit (.closure ⟨⟨0⟩, ⟨1⟩⟩ 0 #[] #[.integer 9])
  | none => false

-- A raw capture cannot smuggle an aliased loan into a declared scalar slot.
#guard match executable? with
  | some ⟨unit, executable⟩ =>
      let forged := RuntimeValue.closure ⟨⟨0⟩, ⟨5⟩⟩ 2 #[] #[.borrow 0 (.integer 9)]
      receiverScopedTarget unit ⟨⟨0⟩, ⟨5⟩⟩ 2 && !receiverScopedValue unit forged &&
      match Interpreter.run executable 50 ⟨⟨0⟩, ⟨4⟩⟩ #[.borrow 0 (.integer 9), forged] with
      | .error error => error.value.code == "VIR-EXEC-UNSCOPED-METHOD-EVIDENCE"
      | .ok _ => false
  | none => false

-- Compilation preserves exactly the runtime entry requirements.
#guard match executable? with
  | some ⟨unit, _⟩ => [2, 3, 4].all fun index =>
      match Proofs.Denote.compileFunction unit ⟨⟨0⟩, ⟨index⟩⟩ with
      | .ok compiled => compiled.scopedMethods == (if index == 3 then [] else [1])
      | .error _ => false
  | none => false

open Proofs Proofs.Denote in
example {unit : ValidatedUnit} [FunctionCarriers unit] [Skolems unit]
    (executable : ExecutableUnit unit) (function : Function unit) (arguments : HList function.params)
    (rejected : scopedValues unit function.scopedMethods (HList.encode arguments).toArray = false) :
    function.denote executable #[] arguments = Spec.stuck := by
  cases scopes : function.scopedMethods <;>
    simp_all only [Function.denote, scopedValues_nil, Bool.false_eq_true,
      Bool.true_eq_false, ↓reduceIte]

open Proofs Proofs.Denote in
example {unit : ValidatedUnit} [FunctionCarriers unit] [Skolems unit]
    {executable : ExecutableUnit unit} (meanings : Meanings executable)
    (function : Function unit) (arguments : HList function.params)
    (rejected : scopedValues unit function.scopedMethods (HList.encode arguments).toArray = false) :
    function.denoteWith meanings arguments = Spec.stuck := by
  cases scopes : function.scopedMethods <;>
    simp_all only [Function.denoteWith, scopedValues_nil, Bool.false_eq_true,
      Bool.true_eq_false, ↓reduceIte]

end VerexIR.Tests.ScopedRuntime
