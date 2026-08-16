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

/-- Non-semantic information for one positional struct declaration. -/
structure StructMeta where
  name : String
  fieldNames : List String
  variantNames : Option (List (String × List String)) := none
  abilities : AbilitySet
  deriving BEq, Repr

/-- Non-semantic information for one positional function declaration. -/
structure FunMeta where
  name : String
  visibility : Visibility
  isEntry : Bool
  acquires : List ResourceId
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
  dialect : Dialect := .stackless

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
