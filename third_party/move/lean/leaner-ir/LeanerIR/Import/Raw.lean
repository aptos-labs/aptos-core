-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Syntax

/-!
# Raw LIR frontend boundary

This module contains the stage-specific forms which frontends construct around
the shared syntax. Function bodies may already be structured or may carry a
reducible CFG for the shared structurizer. Every raw unit must pass through
`LeanerIR.Validation.validate` before a backend consumes it.
-/

namespace LeanerIR.Import

/-- Version of the raw LIR schema, independent of any semantic-profile
version. A decoder must validate this value before constructing `RawUnit`. -/
structure Version where
  major : Nat := 1
  minor : Nat := 1
  deriving Repr, BEq, Inhabited

/-- MIR unwind behavior detached from compiler-local block identities. -/
inductive RawUnwindAction where
  | continue_
  | unreachable
  | terminate (reason : String)
  | cleanup (target : BlockId)
  deriving Repr, BEq, Inhabited

/-- Destination written by a returning MIR call and its normal successor. -/
structure RawCallDestination where
  place : PlaceId
  target : BlockId
  deriving Repr, BEq, Inhabited

/-- Semantic class of a MIR assertion. -/
inductive RawAssertKind where
  | boundsCheck
  | overflow
  | divisionByZero
  | remainderByZero
  | misalignedPointerDereference
  | profile (value : ProfileValue)
  deriving Repr, BEq, Inhabited

/-- MIR statement forms which must survive the detached exchange even when the
initial structurizer rejects their semantics. `execute` embeds an ordinary LIR
expression statement; the other cases retain MIR administrative state. -/
inductive RawStatement where
  | execute (expression : ExprId)
  | storageLive (localId : LocalId)
  | storageDead (localId : LocalId)
  | deinit (place : PlaceId)
  | setDiscriminant (place : PlaceId) (variant : NameId)
  | retag (place : PlaceId)
  | placeMention (place : PlaceId)
  | ascribeUserType (place : PlaceId) (type : TypeUse)
  | profile (value : ProfileValue)
  deriving Repr, BEq, Inhabited

/-- Control transfer at the end of a raw basic block. These nodes are accepted
only at the frontend boundary and must be structurized or explicitly rejected
before validation. Call, drop, and assert preserve their cleanup edges. -/
inductive RawTerminator where
  | goto (target : BlockId)
  | branch (condition : ExprId) (thenTarget elseTarget : BlockId)
  | switch (scrutinee : ExprId) (cases : Array (ConstValue × BlockId)) (defaultTarget : BlockId)
  | call (call : ExprId) (destination : Option RawCallDestination)
      (unwind : RawUnwindAction := .continue_)
  | drop (place : PlaceId) (target : BlockId) (unwind : RawUnwindAction := .continue_)
  | assert (condition : ExprId) (expected : Bool) (kind : RawAssertKind)
      (target : BlockId) (unwind : RawUnwindAction := .continue_)
  | return_ (values : Array ExprId)
  | throw_ (kind : ThrowKind) (arguments : Array ExprId)
  | unreachable
  | resume
  | abort
  deriving Repr, BEq, Inhabited

/-- One raw CFG block: a sequence of detached MIR/LIR statements followed by
exactly one explicit terminator. -/
structure RawBasicBlock where
  loc : LocId
  statements : Array RawStatement := #[]
  terminator : RawTerminator
  deriving Repr, BEq, Inhabited

/-- Frontend import graph with block-local positional `BlockId`s. The shared
structurizer accepts supported reducible graphs and never exposes this form to
backends. -/
structure RawCfg where
  entry : BlockId
  blocks : Array RawBasicBlock
  deriving Repr, BEq, Inhabited

/-- Function body admitted from a frontend: absent for native/opaque code,
already structured, or a CFG awaiting checked structurization. -/
inductive RawBody where
  | absent
  | structured (root : ExprId)
  | cfg (graph : RawCfg)
  deriving Repr, BEq, Inhabited

/-- Frontend-produced namespace whose functions may contain raw CFG bodies. -/
abbrev RawNamespace := Namespace RawBody

/-- Dependency summary admitted at the raw boundary. Exported `NameId`s refer
to the dependency interface's agreed name identity. -/
structure RawNamespaceInterface where
  namespaceId : NamespaceId
  profile : Option Profile := none
  exportedNames : Array NameId := #[]
  /-- Nominal declarations this namespace exports. Naming one of its types
  needs only the name, but constructing, selecting, and matching a value of it
  need the shape: generics, abilities, fields, and variants. Bodies, contracts,
  and locals stay with the declaring unit. -/
  structs : Array StructDecl := #[]
  /-- Function signatures this namespace exports. Their bodies are absent: a
  call is checked against the signature, and the implementation belongs to the
  declaring unit. -/
  functions : Array (FunctionDecl RawBody) := #[]
  /-- Specification-function signatures this namespace exports, likewise
  without bodies. -/
  specFunctions : Array SpecFunctionDecl := #[]
  deriving Repr, BEq, Inhabited

/-- Unit-wide statement about an upstream producer, check, or assumption.
`trusted = false` makes unsupported or unverified source semantics explicit;
it must never be promoted to an end-to-end source theorem silently. -/
structure ImportEvidence where
  producer : String
  description : String
  trusted : Bool := false
  deriving Repr, BEq, Inhabited

/-- The only public frontend output: one compilation-unit table snapshot, a
versioned set of profile configurations, raw namespaces, dependency
interfaces, and explicit import evidence. The tables include symbols imported
from dependencies. Every instance must pass `validate` before use by a backend. -/
structure RawUnit where
  version : Version := {}
  tables : Tables
  profiles : Array ProfileConfig
  namespaces : Array RawNamespace
  dependencies : Array RawNamespaceInterface := #[]
  evidence : Array ImportEvidence := #[]
  deriving Repr, BEq, Inhabited

end LeanerIR.Import
