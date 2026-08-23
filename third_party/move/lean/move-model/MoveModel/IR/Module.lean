-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Syntax

/-!
# Finite Move modules

`Program` is the semantic representation: declarations are partial functions,
and names and deployment metadata are intentionally absent.  `Module` adds the
finite bounds and metadata needed by compilers and exchange formats without
changing the proof-facing semantic core.
-/

namespace MoveModel.IR

/-- A function declared by another Move module. Calls in `program` use ids
`numFuns + i` to refer to `externalFuns[i]`; local function ids retain their
existing positional meaning. Keeping this link metadata on the finite module
leaves the proof-facing single-module `Program` representation unchanged. -/
structure ExternalFunRef where
  address : Address
  moduleName : String
  functionName : String
  deriving BEq, Repr

/-- An explicitly trusted module from this module's friend list. -/
structure ExternalModuleRef where
  address : Address
  moduleName : String
  deriving BEq, Repr

/-- File-format visibility.  Entry status is represented independently. -/
inductive Visibility where
  | private_
  | public_
  | friend
  deriving BEq, Repr

/-- Distinguishes executable stackless IR from proof-only reference-elimination
output containing mutation-algebra operations which are not Move bytecodes. -/
inductive Dialect where
  | stackless
  | referenceEliminated
  deriving BEq, Repr

/-- One positional argument of a user-provided attribute: a name path
(optionally instantiated, so it can denote a constant, function, or type) or
a source constant. -/
inductive AttributeArg where
  | name (path : String) (args : List AttributeArg)
  | num (value : Nat)
  | bool (value : Bool)
  deriving BEq, Repr

/-- A user-provided source attribute: a head name applied to positional
arguments. Well-known internal declaration markers are not represented here;
this carries only open-ended source metadata. -/
structure Attribute where
  name : String
  args : List AttributeArg
  deriving BEq, Repr

/-- A half-open UTF-8 byte range in the source file supplied alongside XIR. -/
structure SourceSpan where
  start : Nat
  «end» : Nat
  deriving BEq, Repr

/-- Source ranges aligned with one semantic basic block. Missing ranges denote
compiler-generated code and let consumers fall back to the enclosing function. -/
structure BlockSourceMap where
  instrs : List (Option SourceSpan)
  term : Option SourceSpan
  deriving BEq, Repr

/-- Non-semantic source locations for a function and its CFG. -/
structure FunSourceMap where
  span : Option SourceSpan
  blocks : List BlockSourceMap
  deriving BEq, Repr

/-- Non-semantic information for one positional struct declaration. -/
structure StructMeta where
  name : String
  fieldNames : List String
  variantNames : Option (List (String × List String)) := none
  abilities : AbilitySet
  attributes : List Attribute := []
  deriving BEq, Repr

/-- Non-semantic information for one positional function declaration. -/
structure FunMeta where
  name : String
  visibility : Visibility
  isEntry : Bool
  acquires : List ResourceId
  attributes : List Attribute := []
  /-- User-facing local names aligned with the function's local indices. -/
  localNames : List (Option String) := []
  sourceMap : Option FunSourceMap := none
  deriving BEq, Repr

/-- A finite, deployable view of a semantic program. -/
structure Module where
  address : Address
  name : String
  program : Program
  numStructs : Nat
  numFuns : Nat
  structMeta : ResourceId → Option StructMeta
  funMeta : FunId → Option FunMeta
  externalFuns : List ExternalFunRef := []
  friends : List ExternalModuleRef := []
  dialect : Dialect := .stackless

/-- Construct finite semantic IR directly from compiler-produced lists.
This is intentionally an IR constructor, rather than an XIR conversion: the
lists only close over the partial maps required by the semantic model. -/
def Module.ofLists (address : Address) (name : String)
    (structs : List StructDecl) (funs : List FunDecl)
    (structMeta : List StructMeta) (funMeta : List FunMeta)
    (externalFuns : List ExternalFunRef := [])
    (dialect : Dialect := .stackless)
    (friends : List ExternalModuleRef := []) : Module where
  address := address
  name := name
  program := {
    structs := fun r => structs[r]?
    funs := fun f => funs[f]?
  }
  numStructs := structs.length
  numFuns := funs.length
  structMeta := fun r => structMeta[r]?
  funMeta := fun f => funMeta[f]?
  externalFuns := externalFuns
  friends := friends
  dialect := dialect

/-- The positional identifier of a named function, or `numFuns` when absent.
The sentinel is outside the declared bounds and therefore has no declaration. -/
def Module.funId (m : Module) (name : String) : FunId :=
  (List.range m.numFuns).findIdx fun i => (m.funMeta i).any (·.name == name)

/-- The positional identifier of a named struct, or `numStructs` when absent. -/
def Module.resourceId (m : Module) (name : String) : ResourceId :=
  (List.range m.numStructs).findIdx fun i => (m.structMeta i).any (·.name == name)

/-- Look up a function declaration through its source-level name. -/
def Module.funDecl? (m : Module) (name : String) : Option FunDecl :=
  m.program.funs (m.funId name)

/-- Look up function metadata through its source-level name. -/
def Module.funMeta? (m : Module) (name : String) : Option FunMeta :=
  m.funMeta (m.funId name)

/-- Look up a struct declaration through its source-level name. -/
def Module.structDecl? (m : Module) (name : String) : Option StructDecl :=
  m.program.structs (m.resourceId name)

/-- Look up struct metadata through its source-level name. -/
def Module.structMeta? (m : Module) (name : String) : Option StructMeta :=
  m.structMeta (m.resourceId name)

/-- Apply a semantic program transformation while retaining module metadata. -/
def Module.mapProgram (m : Module) (f : Program → Program) : Module :=
  { m with program := f m.program }

/-- Change the IR dialect while retaining declarations and metadata. -/
def Module.withDialect (m : Module) (dialect : Dialect) : Module :=
  { m with dialect := dialect }

/-- The partial maps of a finite module are dense within their declared
bounds and empty beyond them. -/
def Module.FiniteWellFormed (m : Module) : Prop :=
  (∀ r, r < m.numStructs → ∃ d info,
      m.program.structs r = some d ∧ m.structMeta r = some info) ∧
  (∀ f, f < m.numFuns → ∃ d info,
      m.program.funs f = some d ∧ m.funMeta f = some info) ∧
  (∀ r, m.numStructs ≤ r →
      m.program.structs r = none ∧ m.structMeta r = none) ∧
  (∀ f, m.numFuns ≤ f →
      m.program.funs f = none ∧ m.funMeta f = none)

end MoveModel.IR
