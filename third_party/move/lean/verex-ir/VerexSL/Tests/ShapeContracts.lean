-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
A contract at a shape prints as the item it was written as and keeps the
printed unit a fixed point, and its JSON form leaves a function without shapes
as it was. Verification through shapes is in the e2e fixtures
`Check/Generics/ShapeContract*`.
-/

namespace VerexSL.Tests.ShapeContracts

open Lean Elab Command VerexIR

private def source : String :=
  "verex namespace shape_print where\n" ++
  "  enum Opt {T} has Copy where\n" ++
  "    | None\n" ++
  "    | Some (value : T)\n\n" ++
  "  opaque spec fun at_position {T} {A}(store : Opt<T>, position : A) : T\n\n" ++
  "  native fun index {T} {I} {A} {Output}(_1 : &Opt<T>, _2 : I) -> &Output\n\n" ++
  "  spec index at {T} {usize} {A} {T} where\n" ++
  "    requires _2 < 10\n" ++
  "    ensures result == at_position(_1, _2)\n\n" ++
  "  spec index at {u32} {u32} {Bool} {Opt<u32>} where\n" ++
  "    pragma opaque\n"

run_cmd do
  let env ← getEnv
  let .ok printed := VerexSL.Print.formatSource env source
    | throwError "the fixture did not format"
  for expected in ["spec index at {T} {usize} {A} {T} where",
      "spec index at {u32} {u32} {Bool} {Opt<u32>} where"] do
    unless printed.contains expected do
      throwError "the shape is not printed as its item:\n{printed}"
  let .ok again := VerexSL.Print.formatSource env printed
    | throwError "the rendering did not re-import:\n{printed}"
  unless again == printed do
    throwError "the rendering is not a fixed point:\n{printed}\n{again}"

private def plain : VerexIR.FunctionDecl Import.RawBody :=
  { loc := ⟨0⟩, name := ⟨0⟩, profile := .rust, signature := {}, body := .absent, origin := ⟨0⟩
    alignment := ⟨0⟩ }

run_cmd do
  if (toJson plain).compress.contains "shapes" then
    throwError "a function without shapes changed its JSON form"
  let shaped : VerexIR.FunctionDecl Import.RawBody := { plain with shapes? := some #[
    { loc := ⟨0⟩, arguments := #[{ typeId := ⟨0⟩, loc := ⟨0⟩ }], signature := {} }] }
  let .ok decoded := (fromJson? (toJson shaped) : Except String (VerexIR.FunctionDecl Import.RawBody))
    | throwError "a function with shapes did not decode"
  unless decoded == shaped do throwError "a function's shapes did not round-trip"

-- Promoted associated types are independent binders until trait equalities are
-- modelled. A contract cannot silently quantify over arbitrary outputs.
run_cmd do
  let env ← getEnv
  let declaration := "verex namespace promoted_shapes where\n" ++
    "  @[shape_only]\n" ++
    "  native fun project {T} {Output}(value : &T) -> &Output\n\n"
  let unshaped := declaration ++ "  spec project where\n    ensures true\n"
  match VerexSL.Print.formatSource env unshaped with
  | .ok _ => throwError "an unshaped contract on promoted associated types was accepted"
  | .error message => unless (toString message).contains "requires a contract at a shape" do
      throwError "unexpected rejection: {message}"
  let shaped := declaration ++ "  spec project at {u64} {u64} where\n    ensures result == value\n"
  let printed ← match VerexSL.Print.formatSource env shaped with
    | .ok printed => pure printed
    | .error error => throwError "a shaped contract on promoted associated types was rejected: {error}"
  unless printed.contains "shape_only" do
    throwError "printing lost the shape requirement"
  let .ok again := VerexSL.Print.formatSource env printed
    | throwError "the shape-only rendering did not re-import"
  unless again == printed do throwError "the shape-only rendering was not a fixed point"

end VerexSL.Tests.ShapeContracts
