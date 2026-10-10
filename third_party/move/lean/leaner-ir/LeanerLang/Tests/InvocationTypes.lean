-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

open Lean Elab Command LeanerIR in
run_cmd do
  let env ← getEnv
  let parse (source : String) : CommandElabM LeanerLang.CompilationUnit := do
    let .ok stx := Parser.runParserCategory env `command source
      | throwError "invocation fixture did not parse"
    let .ok unit := LeanerLang.compilationUnitOfSyntax stx "invocation-types"
      | throwError "invocation fixture did not elaborate"
    return unit
  let source := "leaner module 0x42::invocation_types where\n" ++
    "  struct Vault {T has Store} has Key where\n    value : T\n" ++
    "  fun leaf {T has Store}(address : Address) -> Bool := exists<Vault<T> >(address)\n" ++
    "  fun middle {T has Store}(address : Address) -> Bool := leaf::<Vector<T> >(address)\n" ++
    "  fun entry(address : Address) -> Bool := middle::<u8>(address)\n"
  let .ok raw := LeanerLang.lower (← parse source)
    | throwError "transitive invocation fixture did not lower"
  let types := raw.tables.types
  let some byte := types.findIdx? (· == .integer (.bits 8) false)
    | throwError "missing u8"
  let some vector := types.findIdx? (· == .vector ⟨byte⟩)
    | throwError "missing transitive Vector<u8> invocation argument"
  let vault := raw.namespaces[0]!.structs[0]!.name
  unless types.any fun
      | .nominal name #[.typeArg argument] => name == vault && argument.typeId.index == vector
      | _ => false do
    throwError "missing callee-body-only Vault<Vector<u8>> resource key"
  let .ok printed := LeanerLang.Print.formatSource env source
    | throwError "invocation fixture did not format"
  let .ok reprinted := LeanerLang.Print.formatSource env printed
    | throwError "invocation fixture did not re-import"
  unless reprinted == printed do
    throwError "type completion changed the canonical source fixed point"
  -- Requirements also propagate when the callee is in a later namespace.
  let caller ← parse ("leaner module 0x42::caller where\n" ++
    "  fun entry(address : Address) -> Bool := 0x42::callee::leaf::<u8>(address)\n")
  let callee ← parse ("leaner module 0x42::callee where\n" ++
    "  struct Vault {T has Store} has Key where\n    value : T\n" ++
    "  public fun leaf {T has Store}(address : Address) -> Bool := exists<Vault<T> >(address)\n")
  let .ok cross := LeanerLang.lower { caller with namespaces := caller.namespaces ++ callee.namespaces }
    | throwError "cross-namespace invocation fixture did not lower"
  let some byte := cross.tables.types.findIdx? (· == .integer (.bits 8) false)
    | throwError "missing cross-namespace u8"
  let vault := cross.namespaces[1]!.structs[0]!.name
  unless cross.tables.types.any fun
      | .nominal name #[.typeArg argument] => name == vault && argument.typeId.index == byte
      | _ => false do
    throwError "missing cross-namespace Vault<u8> resource key"
  unless cross.namespaces.map (·.functions.size) == #[1, 1] do
    throwError "type completion duplicated a generic function body"
  -- Direct, transitive, and parameter-swapping positive cycles follow the
  -- VM rule; ordinary identity, permutation, and constant cycles stay legal.
  let sourcePrefix := "leaner module 0x42::invocation_cycle where\n"
  for body in [
      "  fun cycle {T has Store}(address : Address) -> Bool := cycle::<Vector<T> >(address)\n",
      "  fun first {T has Store}(address : Address) -> Bool := second::<T>(address)\n" ++
      "  fun second {T has Store}(address : Address) -> Bool := first::<Vector<T> >(address)\n",
      "  fun cycle {T has Store} {U has Store}(address : Address) -> Bool := cycle::<U, Vector<T> >(address)\n"] do
    match LeanerLang.lower (← parse (sourcePrefix ++ body)) with
    | .ok _ => throwError "growing instantiation cycle was accepted"
    | Except.error errors => unless errors.any (·.code == "LEANER-INSTANTIATION-LOOP") do
        throwError "wrong growing-cycle diagnostic: {repr errors}"
  for arguments in ["T, U", "U, T", "u64, U"] do
    let source := sourcePrefix ++
      "  struct Vault {T has Store} has Key where\n    value : T\n" ++
      "  fun cycle {T has Store} {U has Store}(address : Address) -> Bool :=\n" ++
      "    if exists<Vault<T> >(address) then true else cycle::<" ++ arguments ++ ">(address)\n"
    let .ok _ := LeanerLang.lower (← parse source)
      | throwError "finite instantiation cycle was rejected: {arguments}"

open Lean Elab Command in
run_cmd do
  let env ← getEnv
  -- Logical scalar inference must refine from the representation-bearing
  -- argument regardless of order, including in derived function meanings.
  let source := "leaner module 0x42::generic_inference where\n" ++
    "  struct Box {T has Copy, Drop, Store} has Copy, Drop, Store where\n    value : T\n" ++
    "  fun first {T has Copy, Drop, Store}(value : T, box : Box<T>) -> T := value\n" ++
    "  fun second {T has Copy, Drop, Store}(box : Box<T>, value : T) -> T := value\n" ++
    "  fun caller(value : u64, box : Box<u64>) -> u64 := first(value, box)\n" ++
    "  fun reverse_caller(value : u64, box : Box<u64>) -> u64 := second(box, value)\n" ++
    "  spec fun select {T has Copy, Drop, Store}(value : T, box : Box<T>) : T := value\n" ++
    "  spec caller where\n    ensures result == select(value, box)\n"
  let printed ← match LeanerLang.Print.formatSource env source with
    | .ok printed => pure printed
    | .error error => throwError "mixed logical/physical inference failed: {error}"
  let .ok reprinted := LeanerLang.Print.formatSource env printed
    | throwError "mixed inference did not re-import"
  unless printed == reprinted do throwError "mixed inference did not stabilize"
  let mismatch := "leaner module 0x42::generic_mismatch where\n" ++
    "  struct Box {T has Copy, Drop, Store} has Copy, Drop, Store where\n    value : T\n" ++
    "  spec fun both {T has Copy, Drop, Store}(left : Box<T>, right : Box<T>) : Bool := true\n" ++
    "  spec fun bad(left : Box<u8>, right : Box<u64>) : Bool := both(left, right)\n"
  match LeanerLang.Print.formatSource env mismatch with
  | .ok _ => throwError "distinct physical nominal arguments were unified"
  | .error error => unless (toString error).contains "LEANER-TYPE-MISMATCH" do
      throwError "wrong physical mismatch diagnostic: {error}"
