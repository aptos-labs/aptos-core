-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean.Data.Json

/-!
# MonoVM link payload codecs

The Lean mirror of the adapter crate's deterministic JSON payload v1
(`mono-move-lean-link/src/payload.rs`). Both sides are generated from
structurally identical definitions, and the payload `version` gates the
pair: the codecs here refuse any other version.
-/

namespace LeanerE2ETests.MonoVM

open Lean

/-- Version of the JSON payload schema. -/
def payloadVersion : Nat := 1

/-- Native ABI version this package links against. -/
def expectedAbiVersion : Nat := 1

/-- One Move source file of the request's compilation unit. -/
structure SourceFile where
  name : String
  text : String
  deriving Inhabited, Repr, BEq

/-- The compilation unit: sources, named addresses, language version. -/
structure CompileSpec where
  sources : Array SourceFile
  addresses : Array (String × String)
  language : Nat
  deriving Inhabited, Repr, BEq

/-- Resource limits for every call of the request. -/
structure Limits where
  gas : Nat
  heap : Option Nat
  deriving Inhabited, Repr, BEq

/-- A typed value in the stable recursive schema. -/
inductive Value where
  | unit
  | bool (value : Bool)
  | integer (width : Nat) (signed : Bool) (value : String)
  | address (value : String)
  | vector (elements : Array Value)
  deriving Inhabited, Repr, BEq

/-- One function call of the request. -/
structure Call where
  function : String
  signers : Array String
  args : Array Value
  deriving Inhabited, Repr, BEq

/-- The adapter's build identity, echoed in every response. -/
structure Identity where
  abi : Nat
  rustc : String
  profile : String
  deriving Inhabited, Repr, BEq

/-- The resource that ran out before a call completed. -/
inductive ExhaustedResource where
  | gas
  | heap
  deriving Repr, BEq

/-- The stage that produced an `error` outcome. -/
inductive Stage where
  | compile
  | load
  | run
  | abi
  | internal
  deriving Repr, BEq

/-- The normalized outcome of one request call. `failed` is an outcome of the
*program* — MonoVM reports arithmetic overflow, an out-of-bounds index, or a
structural limit as a typed execution error rather than as an abort — and
carries the `ExecutionErrorKind` a caller branches on. `error` is the other
thing: the harness obtained no outcome at all. -/
inductive Outcome where
  | returned (values : Array Value) (gasUsed : Nat) (gcCount : Nat)
  | aborted (code : Nat) (location : Option String) (message : Option String)
  | exhausted (resource : ExhaustedResource)
  | failed (failure : String) (message : String)
  | error (stage : Stage) (message : String)
  deriving Inhabited, Repr, BEq

/-- One linked execution request: compile once, then run every call. -/
structure Request where
  version : Nat
  compile : CompileSpec
  limits : Limits
  calls : Array Call
  deriving Inhabited, Repr, BEq

/-- The response to a request: one outcome per call, in request order. -/
structure Response where
  version : Nat
  identity : Identity
  outcomes : Array Outcome
  deriving Inhabited, Repr, BEq

private def str? (json : Json) (field : String) : Except String String := do
  (← json.getObjVal? field).getStr?

private def nat? (json : Json) (field : String) : Except String Nat := do
  (← json.getObjVal? field).getNat?

private def bool? (json : Json) (field : String) : Except String Bool := do
  (← json.getObjVal? field).getBool?

private def arr? (json : Json) (field : String) : Except String (Array Json) := do
  (← json.getObjVal? field).getArr?

private def strField? (json : Json) (field : String) : Except String (Option String) :=
  return (str? json field).toOption

private partial def encodeValue : Value → Json
  | .unit => Json.mkObj [("kind", "unit")]
  | .bool value => Json.mkObj [("kind", "bool"), ("value", .bool value)]
  | .integer width signed value =>
    Json.mkObj
      [ ("kind", "integer"), ("width", .num width), ("signed", .bool signed),
        ("value", .str value) ]
  | .address value => Json.mkObj [("kind", "address"), ("value", .str value)]
  | .vector elements =>
    Json.mkObj [("kind", "vector"), ("elements", .arr (elements.map encodeValue))]

private partial def decodeValue (json : Json) : Except String Value := do
  match ← str? json "kind" with
  | "unit" => return .unit
  | "bool" => return .bool (← bool? json "value")
  | "integer" =>
    return .integer (← nat? json "width") (← bool? json "signed") (← str? json "value")
  | "address" => return .address (← str? json "value")
  | "vector" => return .vector (← (← arr? json "elements").mapM decodeValue)
  | other => throw s!"unknown value kind {other}"

private def encodeExhausted : ExhaustedResource → Json
  | .gas => .str "gas"
  | .heap => .str "heap"

private def encodeStage : Stage → Json
  | .compile => .str "compile"
  | .load => .str "load"
  | .run => .str "run"
  | .abi => .str "abi"
  | .internal => .str "internal"

private partial def encodeOutcome : Outcome → Json
  | .returned values gasUsed gcCount =>
    Json.mkObj
      [ ("kind", "returned"), ("values", .arr (values.map encodeValue)),
        ("gas_used", .num gasUsed), ("gc_count", .num gcCount) ]
  | .aborted code location message =>
    Json.mkObj
      [ ("kind", "aborted"), ("code", .num code),
        ("location", location.map .str |>.getD .null),
        ("message", message.map .str |>.getD .null) ]
  | .exhausted resource =>
    Json.mkObj [("kind", "exhausted"), ("resource", encodeExhausted resource)]
  | .failed failure message =>
    Json.mkObj [("kind", "failed"), ("failure", .str failure), ("message", .str message)]
  | .error stage message =>
    Json.mkObj [("kind", "error"), ("stage", encodeStage stage), ("message", .str message)]

private partial def decodeOutcome (json : Json) : Except String Outcome := do
  match ← str? json "kind" with
  | "returned" =>
    return .returned (← (← arr? json "values").mapM decodeValue) (← nat? json "gas_used")
      (← nat? json "gc_count")
  | "aborted" =>
    return .aborted (← nat? json "code") (← strField? json "location")
      (← strField? json "message")
  | "exhausted" =>
      match ← str? json "resource" with
      | "gas" => return .exhausted .gas
      | "heap" => return .exhausted .heap
      | other => throw s!"unknown exhausted resource {other}"
  | "failed" =>
      return .failed (← str? json "failure") (← str? json "message")
  | "error" =>
      let message ← str? json "message"
      match ← str? json "stage" with
      | "compile" => return .error .compile message
      | "load" => return .error .load message
      | "run" => return .error .run message
      | "abi" => return .error .abi message
      | "internal" => return .error .internal message
      | other => throw s!"unknown stage {other}"
  | other => throw s!"unknown outcome kind {other}"

/-- Encodes a request as the payload the adapter expects. -/
partial def encodeRequest : Request → Json
  | ⟨version, ⟨sources, addresses, language⟩, ⟨gas, heap⟩, calls⟩ =>
  Json.mkObj
    [ ("version", .num version),
      ("compile",
        Json.mkObj
          [ ("sources",
            .arr
              (sources.map fun { name, text } =>
                Json.mkObj [("name", .str name), ("text", .str text)])),
            ("addresses",
              Json.mkObj
                (((addresses.qsort fun a b => a.1 < b.1).map
                  fun (name, address) => (name, Json.str address)).toList)),
            ("language", .num language) ]),
      ("limits", Json.mkObj [("gas", .num gas), ("heap", heap.map (Json.num ·) |>.getD .null)]),
      ("calls",
        .arr
          (calls.map fun { function, signers, args } =>
            Json.mkObj
              [ ("function", .str function), ("signers", .arr (signers.map .str)),
                ("args", .arr (args.map encodeValue)) ])) ]

/-- Decodes a response, rejecting any other payload version. -/
partial def decodeResponse (json : Json) : Except String Response := do
  let version ← nat? json "version"
  unless version == payloadVersion do
    throw s!"payload version {version} is not supported; expected {payloadVersion}"
  let identityJson ← json.getObjVal? "identity"
  let identity : Identity := {
    abi := (← nat? identityJson "abi"),
    rustc := (← str? identityJson "rustc"),
    profile := (← str? identityJson "profile") }
  return { version, identity, outcomes := ← (← arr? json "outcomes").mapM decodeOutcome }

end LeanerE2ETests.MonoVM
