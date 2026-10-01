-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang
import LeanerIR.Validation.Link

/-!
# Relocatable namespaces

A namespace extracted from its unit holds only the entries it uses, and
linking it into another unit relocates it losslessly. Linking checks the
boundary: a linked namespace must declare what the unit was checked against.
-/

namespace LeanerLang.Tests.Link

open Lean Elab Command LeanerIR.Validation

leaner module 0x42::linked where
  use std::vector

  struct Counter has Key, Drop where
    value : u64

  enum Choice has Copy, Drop where
    | Left (value : u64)
    | Right

  fun bump(counter : &mut Counter) -> Unit := do
    counter.value := counter.value + 1
  spec bump where
    ensures counter.value == old(counter.value) + 1

  fun pick(choice : Choice) -> u64 :=
    match choice with
      | Choice::Left { value := value } => value
      | Choice::Right {} => 0

  fun has_one(values : &Vector<u64>) -> Bool := values.contains(&1)

/-- A namespace compiled from source text, without registering it. -/
private def compileSource (source : String) : CommandElabM ValidatedUnit := do
  let env ← getEnv
  let stx ← match Parser.runParserCategory env `command source "<test>" with
    | .ok stx => pure stx
    | .error message => throwError message
  let (stx, aliases) ← match canonicalMoveCommand env stx with
    | .ok canonical => pure canonical
    | .error (_, message) => throwError message
  let unit ← match compilationUnitOfSyntax stx "<test>" with
    | .ok unit => pure { unit with namespaces := unit.namespaces.map ({ · with aliases }) }
    | .error (_, message) => throwError message
  match compile unit with
  | .ok unit => pure unit
  | .error _ => throwError "the source did not compile"

-- Extracted, the namespace keeps only the entries it uses; assembled into a
-- unit of its own, it renders as it did and extracts to itself.
run_cmd do
  let env ← getEnv
  let some unit := registeredUnit? env `«0x42».linked | throwError "missing module"
  let .ok object := extract unit ⟨0⟩ | throwError "extraction failed"
  unless object.path == #["0x42", "linked"] do throwError s!"extracted at {object.path}"
  unless object.tables.locations.size < unit.tables.locations.size do
    throwError "extraction kept locations the namespace does not use"
  let relinked ← match assemble unit.profiles #[object] with
    | .ok relinked => pure relinked
    | .error message => throwError message
  let .ok original := Print.render env unit | throwError "the module did not render"
  let .ok printed := Print.render env relinked | throwError "the linked module did not render"
  unless original == printed do throwError s!"relocation changed the rendering:\n{printed}"
  let .ok again := extract relinked ⟨0⟩ | throwError "extraction of the linked unit failed"
  unless again == object do throwError "relocation is not lossless"

-- A namespace declaring something else at the same path is rejected at the
-- boundary, whether a signature differs or a declaration is missing.
run_cmd do
  let env ← getEnv
  let some unit := registeredUnit? env `«0x42».linked | throwError "missing module"
  let changed ← compileSource "leaner module 0x42::linked where
  struct Counter has Key, Drop where
    value : u64
  enum Choice has Copy, Drop where
    | Left (value : u64)
    | Right
  fun bump(counter : &mut Counter, step : u64) -> Unit := ()
  fun pick(choice : Choice) -> u64 := 0
  fun has_one(values : &Vector<u64>) -> Bool := true"
  let .ok object := extract changed ⟨0⟩ | throwError "extraction failed"
  match link unit #[object] with
  | .error message =>
      unless message == "`0x42::linked::bump`: the linked function differs from the one it was \
          checked against" do
        throwError s!"unexpected boundary message: {message}"
  | .ok _ => throwError "a changed signature linked"
  let missing ← compileSource "leaner module 0x42::linked where
  struct Counter has Key, Drop where
    value : u64
  enum Choice has Copy, Drop where
    | Left (value : u64)
    | Right
  fun bump(counter : &mut Counter) -> Unit := ()
  fun has_one(values : &Vector<u64>) -> Bool := true"
  let .ok object := extract missing ⟨0⟩ | throwError "extraction failed"
  match link unit #[object] with
  | .error message =>
      unless message == "`0x42::linked` does not declare the function `pick`" do
        throwError s!"unexpected boundary message: {message}"
  | .ok _ => throwError "a missing function linked"

end LeanerLang.Tests.Link
