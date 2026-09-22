-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean.Data.Json
import LeanerIR.Import.Raw

/-!
# Raw LIR JSON exchange

This module owns the deterministic JSON representation used between frontend
processes and LeanerIR. The instances are intentionally centralized here so
ordinary semantic users do not acquire a JSON dependency merely by importing
the syntax. Constructor and field names form part of RawUnit JSON v1.
-/

namespace LeanerIR

open Lean

private instance : ToJson UInt8 where
  toJson value := toJson value.toNat

private instance : FromJson UInt8 where
  fromJson? json := do
    let value ← json.getNat?
    if value < 256 then
      pure (UInt8.ofNat value)
    else
      throw s!"expected an 8-bit unsigned integer, got {value}"

deriving instance ToJson, FromJson for
  LoanId, FileId, LocId, OriginId, AlignmentId, ProfileId, TypeId, NamespaceId, NameId,
  TypeDeclId, TraitDeclId, ImplDeclId, AssociatedItemId, FunctionId,
  SpecFunctionId, SpecVarId, LocalId, PlaceId, LifetimeId, EvidenceId, ExprId,
  PatternId, BlockId, IntrinsicId

deriving instance ToJson, FromJson for
  SourceFile, SourceRange, Location, OriginKind, Origin, Trust, Alignment,
  NamespaceRef, QualifiedName, QualifiedRef, Profile, ProfileValue,
  ProfileConfig, IntWidth, ReferenceKind, LifetimeKind, Lifetime, ReferenceType,
  ConstValue, TypeUse, GenericArgument, Ability, TraitRef, GenericPredicate, Ty,
  AttributeValue, Attribute, Comment, Tables, BorrowKind, ThrowKind, CallKind,
  SurfaceSyntax, GlobalKind, PrimitiveOperation, ReferenceOperation,
  DataOperation, MemoryRange, TraceKind, BehaviorKind, SpecOperation, Operation, Place,
  QuantifierKind, MatchArm, QuantifierBinder, ConditionKind, Condition, Frame,
  SpecBlock, ExprKind, Expr, PatternKind, Pattern, BinderKind, GenericBinder,
  Parameter, LocalDecl, Signature, FunctionContract, ConstantDecl, FieldDecl,
  VariantDecl, StructDecl, FunctionDecl, AssociatedItemKind, AssociatedItemDecl,
  TraitDecl, AssociatedItemValue, AssociatedItemBinding, ImplDecl,
  SpecFunctionDecl, SpecVarDecl, NamespaceInvariant, IntrinsicBinding,
  IntrinsicDecl, Namespace

namespace Import

deriving instance ToJson, FromJson for
  Version, RawUnwindAction, RawCallDestination, RawAssertKind, RawStatement,
  RawTerminator, RawBasicBlock, RawCfg, RawBody,
  RawNamespaceInterface, ImportEvidence, RawUnit

/-- The only schema version this decoder constructs. -/
def jsonVersion : Version := { major := 1, minor := 1 }

/-- Encode a raw unit using the canonical compact JSON representation. Object
keys are emitted deterministically and source arrays retain their input order. -/
def encodeJson (unit : RawUnit) : String :=
  (toJson unit).compress

namespace StrictJsonParser

open Std.Internal.Parsec
open Std.Internal.Parsec.String

mutual

  partial def arrayCore (acc : Array Json) : Parser (Array Json) := do
    let value ← valueCore
    let acc := acc.push value
    let separator ← any
    if separator == ']' then
      ws
      pure acc
    else if separator == ',' then
      ws
      arrayCore acc
    else
      fail "unexpected character in array"

  partial def objectCore (fields : Std.TreeMap.Raw String Json) :
      Parser (Std.TreeMap.Raw String Json) := do
    Lean.Json.Parser.lookahead (· == '"') "\""
    skip
    let name ← Lean.Json.Parser.str
    ws
    Lean.Json.Parser.lookahead (· == ':') ":"
    skip
    ws
    if fields.contains name then
      fail s!"duplicate JSON object field `{name}`"
    let value ← valueCore
    let fields := fields.insert name value
    let separator ← any
    if separator == '}' then
      ws
      pure fields
    else if separator == ',' then
      ws
      objectCore fields
    else
      fail "unexpected character in object"

  partial def valueCore : Parser Json := do
    let next ← peek!
    if next == '[' then
      skip
      ws
      if (← peek!) == ']' then
        skip
        ws
        pure (.arr #[])
      else
        pure (.arr (← arrayCore #[]))
    else if next == '{' then
      skip
      ws
      if (← peek!) == '}' then
        skip
        ws
        pure (.obj ∅)
      else
        pure (.obj (← objectCore ∅))
    else if next == '"' then
      skip
      let value ← Lean.Json.Parser.str
      ws
      pure (.str value)
    else if next == 'f' then
      skipString "false"
      ws
      pure (.bool false)
    else if next == 't' then
      skipString "true"
      ws
      pure (.bool true)
    else if next == 'n' then
      skipString "null"
      ws
      pure .null
    else if next == '-' || ('0' <= next && next <= '9') then
      let value ← Lean.Json.Parser.num
      ws
      pure (.num value)
    else
      fail "unexpected input"

end

def document : Parser Json := do
  ws
  let value ← valueCore
  eof
  pure value

end StrictJsonParser

private def parseJson (text : String) : Except String Json :=
  Std.Internal.Parsec.String.Parser.run StrictJsonParser.document text

private def decodeVersion (json : Json) : Except String Version := do
  let versionJson ← json.getObjVal? "version"
  let major ← versionJson.getObjValAs? Nat "major"
  let minor ← versionJson.getObjValAs? Nat "minor"
  let version := { major, minor }
  unless version == jsonVersion do
    throw s!"unsupported raw LIR JSON version {major}.{minor}; expected 1.1"
  pure version

/-- Reject object fields which the typed RawUnit encoder does not know. The
derived decoder intentionally remains the constructor mapping, while its
canonical re-encoding supplies the closed object shape for the constructor
which was actually decoded. This checks nested records and tagged-constructor
payloads as well as the top-level unit. -/
private partial def rejectUnknownFields (path : String) (received expected : Json) :
    Except String Unit := do
  match received, expected with
  | .obj receivedFields, .obj expectedFields =>
      for (name, value) in receivedFields.toList do
        let some expectedValue := expectedFields.get? name
          | throw s!"unknown raw LIR JSON field `{path}.{name}`"
        rejectUnknownFields s!"{path}.{name}" value expectedValue
  | .arr receivedValues, .arr expectedValues =>
      for index in [:receivedValues.size] do
        let some expectedValue := expectedValues[index]?
          | throw s!"unexpected raw LIR JSON array element `{path}[{index}]`"
        rejectUnknownFields s!"{path}[{index}]" receivedValues[index]! expectedValue
  | _, _ => pure ()

/-- Check that every object and array position in `received` belongs to the
canonical closed shape in `expected`. `decodeJson` applies this after typed
decoding; exposing the shape check separately lets schema conformance tests
exercise every nested constructor without repeatedly parsing and decoding the
same complete RawUnit document. -/
def ensureClosedJsonShape (received expected : Json) : Except String Unit :=
  rejectUnknownFields "$" received expected

/-- Parse RawUnit JSON, rejecting an unsupported schema version before
constructing the raw unit. Structural and semantic validation remains the
responsibility of `LeanerIR.Validation.validate`. -/
def decodeJson (text : String) : Except String RawUnit := do
  let json ← parseJson text
  let _ ← decodeVersion json
  let unit ← fromJson? json
  ensureClosedJsonShape json (toJson unit)
  pure unit

end Import
end LeanerIR
