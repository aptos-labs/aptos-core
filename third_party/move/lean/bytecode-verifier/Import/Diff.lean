-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Import.Json
import Spec.Semantics.Run

/-!
# Decoding recorded VM outcomes

Decodes the output of `vm-oracle/` (format `move-bytecode-lean-diff/1`): the
modules, the Rust verifier's decision on each, and every call with its
normalized outcome on the legacy Move VM and on MonoVM. Values use the schema
of `mono-move-lean-link`.
-/

namespace MoveBytecode.Import

open Lean (Json)

def diffFormatTag : String := "move-bytecode-lean-diff/1"

/-- A value as a VM reports it. A vector carries no element type, so the
semantics' values are compared with these structurally. -/
inductive Payload where
  | bool (b : Bool)
  | int (t : IntType) (v : Int)
  | address (a : Address)
  | vector (elems : List Payload)
  deriving Repr, Inhabited

/-- A normalized VM outcome. `failed` carries the legacy VM's status code or
MonoVM's error kind. -/
inductive VmOutcome where
  | returned (values : List Payload)
  | aborted (code : Nat)
  | failed (status : String)
  | exhausted (resource : String)
  | error (status : String)
  deriving Repr, Inhabited

structure DiffCall where
  function : String
  args : List Value
  legacy : VmOutcome
  monovm : VmOutcome
  deriving Repr

structure RustVerifierDecision where
  module : String
  local_ : String
  linking : String
  deriving Repr

structure DiffDocument where
  modules : Array CompiledModule
  rustVerifier : Array RustVerifierDecision
  calls : Array DiffCall

def intType (width : Nat) (signed : Bool) : Dec IntType :=
  match width, signed with
  | 8, false => pure .u8
  | 16, false => pure .u16
  | 32, false => pure .u32
  | 64, false => pure .u64
  | 128, false => pure .u128
  | 256, false => pure .u256
  | 8, true => pure .i8
  | 16, true => pure .i16
  | 32, true => pure .i32
  | 64, true => pure .i64
  | 128, true => pure .i128
  | 256, true => pure .i256
  | w, s => throw s!"unsupported integer width {w} (signed {s})"

/-- An address hex literal of up to 64 digits, such as `0x1`. -/
def hexLiteral (s : String) : Dec Address := do
  let digits := s.toList.drop 2
  if !s.startsWith "0x" || digits.isEmpty || digits.length > 64 then
    throw s!"invalid address literal {s}"
  let hexDigit (c : Char) : Dec Nat :=
    if '0' ≤ c ∧ c ≤ '9' then pure (c.toNat - '0'.toNat)
    else if 'a' ≤ c ∧ c ≤ 'f' then pure (c.toNat - 'a'.toNat + 10)
    else if 'A' ≤ c ∧ c ≤ 'F' then pure (c.toNat - 'A'.toNat + 10)
    else throw s!"invalid address literal {s}"
  let n ← digits.foldlM (fun acc c => do pure (acc * 16 + (← hexDigit c))) 0
  pure (BitVec.ofNat 256 n)

/-- A value of the core subset in the `mono-move-lean-link` schema. -/
def payloadValue (j : Json) : Dec Value := do
  match ← context "kind" (str (← get j "kind")) with
  | "bool" => pure (.bool (← boolField j "value"))
  | "integer" => do
    let t ← intType (← natField j "width") (← boolField j "signed")
    let v ← intOfString (← context "value" (str (← get j "value")))
    if t.inRange v then pure (.int t v) else throw s!"integer {v} out of range"
  | "address" => pure (.address (← hexLiteral (← context "value" (str (← get j "value")))))
  | k => throw s!"unsupported value kind {k}"

/-- A returned value in the `mono-move-lean-link` schema, vectors included. -/
partial def payload (j : Json) : Dec Payload := do
  match ← context "kind" (str (← get j "kind")) with
  | "vector" => pure (.vector (← context "elements" (list payload (← get j "elements"))))
  | _ =>
    match ← payloadValue j with
    | .bool b => pure (.bool b)
    | .int t v => pure (.int t v)
    | .address a => pure (.address a)
    | _ => throw "unsupported returned value"

def vmOutcome (j : Json) : Dec VmOutcome := do
  match ← context "kind" (str (← get j "kind")) with
  | "returned" => pure (.returned (← context "values" (list payload (← get j "values"))))
  | "aborted" => pure (.aborted (← context "code" (natString (← get j "code"))))
  | "failed" => pure (.failed (← context "status" (str (← get j "status"))))
  | "exhausted" => pure (.exhausted (← context "resource" (str (← get j "resource"))))
  | "error" => pure (.error (← context "status" (str (← get j "status"))))
  | k => throw s!"unknown outcome kind {k}"

def diffCall (j : Json) : Dec DiffCall := do
  pure {
    function := ← context "function" (str (← get j "function"))
    args := ← context "args" (list payloadValue (← get j "args"))
    legacy := ← context "legacy" (vmOutcome (← get j "legacy"))
    monovm := ← context "monovm" (vmOutcome (← get j "monovm")) }

def diffDocument (text : String) : Dec DiffDocument := do
  let j ← Json.parse text
  let f ← context "format" (str (← get j "format"))
  if f != diffFormatTag then throw s!"unsupported format {f}, expected {diffFormatTag}"
  pure {
    modules := ← tableField j "modules" compiledModule
    rustVerifier := ← tableField j "rust_verifier" (fun d => do
      pure ⟨← context "module" (str (← get d "module")), ← context "local" (str (← get d "local")),
            ← context "linking" (str (← get d "linking"))⟩)
    calls := ← tableField j "calls" diffCall }

/-- The modules and the Rust verifier's decisions of a recorded document,
without its calls: all that the model-agreement check needs. -/
def verifierDocument (text : String) :
    Dec (Array CompiledModule × Array RustVerifierDecision) := do
  let j ← Json.parse text
  let f ← context "format" (str (← get j "format"))
  if f != diffFormatTag then throw s!"unsupported format {f}, expected {diffFormatTag}"
  pure (← tableField j "modules" compiledModule,
    ← tableField j "rust_verifier" (fun d => do
      pure ⟨← context "module" (str (← get d "module")), ← context "local" (str (← get d "local")),
            ← context "linking" (str (← get d "linking"))⟩))

/-- The function named `0x…::module::function` in a program. -/
def findFunction (P : Program) (name : String) : Dec FunctionRef := do
  match name.splitOn "::" with
  | [addr, module, function] =>
    let id : ModuleId := ⟨← hexLiteral addr, module⟩
    let some mi := P.findModule id | throw s!"no module {addr}::{module}"
    let some m := P.modules[mi]? | throw s!"no module {addr}::{module}"
    let some di := m.functionDefs.findIdx? (fun d => m.functionDefName d == some function)
      | throw s!"no function {name}"
    pure { module := mi, index := di }
  | _ => throw s!"malformed function reference {name}"

end MoveBytecode.Import
