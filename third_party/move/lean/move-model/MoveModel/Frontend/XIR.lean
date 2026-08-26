-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Value
import MoveModel.IR.State
import MoveModel.IR.Spec
import MoveModel.IR.Contract
import MoveModel.IR.Syntax
import MoveModel.IR.Module

/-!
# Exchange IR (XIR)

The `masm%` and `move%` elaborators invoke `aptos move exchange`.  Masm input
is assembled; Move input is compiled.  Both paths use the real toolchain and
`StacklessBytecodeGenerator`, then export the basic-block CFG as JSON.

This module defines `MProgram`, the first-order mirror of `Program` decoded
from that JSON.  Its components are list-based data that the elaborator can
quote directly.  `MProgram.toProgram` then builds the function-valued fields
of the semantic representation.  Proofs can unfold that conversion and its
list lookups with `simp`.
-/

namespace MoveModel.Frontend.XIR

open MoveModel.IR

/-- Loop metadata of one natural loop, as data.  `invariants` are the
`invariant` spec clauses attached to the loop header. -/
structure MLoop where
  header : BlockId
  members : List BlockId
  valTargets : List LocalIndex
  memTargets : List ResourceId
  invariants : List SpecExp

/-- A function contract, as clause lists (`requires`/`ensures` conjoined,
`aborts_if` disjoined, per the Move specification language). -/
structure MContract where
  requires : List SpecExp
  abortsIf : List SpecExp
  ensures : List SpecExp
  modifies : List (ResourceId × SpecExp)

/-- A function: parameter count, the declared types of all locals
(parameters first) and of the results, basic blocks in code layout order
(block 0 is the entry), loop metadata, and the contract. -/
structure MFun where
  name : String
  typeParams : List TypeParamDecl := []
  params : Nat
  locals : List Ty
  returns : List Ty
  blocks : List Block
  entry : BlockId := 0
  loops : List MLoop
  spec : MContract
  native : Bool := false

/-- A struct declaration: name and fields with their types.  The names are
documentation; resource ids and field offsets are positional. -/
structure MStruct where
  name : String
  typeParams : List TypeParamDecl := []
  fields : List (String × Ty)
  variants : Option (List (String × List (String × Ty))) := none

/-- A dumped module: the struct and function declarations (resource and
fun ids are positional). -/
structure MProgram where
  structs : List MStruct
  funs : List MFun

/-- A finite exchange module.  Extending `MProgram` keeps the common
`.structs` and `.funs` projections available to existing clients. -/
structure MModule extends MProgram where
  address : Address
  name : String
  dialect : Dialect
  structMeta : List StructMeta
  funMeta : List FunMeta
  externalFuns : List ExternalFunRef := []
  friends : List ExternalModuleRef := []

/-- Conjunction of a clause list (empty = `true`). -/
def andAll : List SpecExp → SpecExp
  | [] => .value (.bool true)
  | [e] => e
  | e :: es => .binop .and e (andAll es)

/-- Disjunction of a clause list (empty = `false`). -/
def orAll : List SpecExp → SpecExp
  | [] => .value (.bool false)
  | [e] => e
  | e :: es => .binop .or e (orAll es)

def MContract.toContract (c : MContract) : Contract where
  requires := andAll c.requires
  aborts := if c.abortsIf.isEmpty then none else some (orAll c.abortsIf)
  ensures := andAll c.ensures
  modifies := c.modifies

def MLoop.toLoopSpec (l : MLoop) : LoopSpec where
  inv := andAll l.invariants
  valTargets := fun t => t ∈ l.valTargets
  memTargets := fun loc => loc.rsrc.resource ∈ l.memTargets
  members := fun b => b ∈ l.members

def MFun.toFunDecl (f : MFun) : FunDecl where
  typeParams := f.typeParams
  numParams := f.params
  numLocals := f.locals.length
  locals := fun t => f.locals[t]?
  returns := f.returns
  body := { blocks := fun b => f.blocks[b]?, entry := f.entry, size := f.blocks.length }
  loopSpecs := fun b => (f.loops.find? (fun l => l.header = b)).map MLoop.toLoopSpec
  contract := f.spec.toContract
  native := f.native

def MStruct.toStructDecl (s : MStruct) : StructDecl where
  typeParams := s.typeParams
  fields := s.fields.map (·.2)
  variants := s.variants.map fun variants =>
    variants.map fun variant => variant.2.map (·.2)

/-- The id of the function named `name`.  Ids are positional and their
order is a producer detail; the names in the dump are the intended way to
resolve them (e.g. in tests).  Returns the number of functions if absent,
which is undeclared in the resulting `Program`. -/
def MProgram.funId (p : MProgram) (name : String) : FunId :=
  p.funs.findIdx (·.name == name)

/-- The resource id of the struct named `name` (see `funId`). -/
def MProgram.resourceId (p : MProgram) (name : String) : ResourceId :=
  p.structs.findIdx (·.name == name)

/-- The semantic program of a dumped module. -/
def MProgram.toProgram (p : MProgram) : Program where
  funs := fun f => (p.funs[f]?).map MFun.toFunDecl
  structs := fun r => (p.structs[r]?).map MStruct.toStructDecl

/-- Project a deployable XIR module to the existing semantic program. -/
def MModule.toProgram (m : MModule) : Program := m.toMProgram.toProgram

/-- Recover the finite semantic module represented by XIR.  XIR producers
maintain positional declaration/metadata alignment; consumers which accept
untrusted JSON should run their normal structural validation first. -/
def MModule.toModule (m : MModule) : Module where
  address := m.address
  name := m.name
  program := m.toProgram
  numStructs := m.structs.length
  numFuns := m.funs.length
  structMeta := fun r => m.structMeta[r]?
  funMeta := fun f => m.funMeta[f]?
  externalFuns := m.externalFuns
  friends := m.friends
  dialect := m.dialect

/-- Resolve a function name in a deployable XIR module. -/
def MModule.funId (m : MModule) (name : String) : FunId :=
  m.toMProgram.funId name

/-- Resolve a resource name in a deployable XIR module. -/
def MModule.resourceId (m : MModule) (name : String) : ResourceId :=
  m.toMProgram.resourceId name

/-- Existing consumers of `MProgram` can accept a deployable module without
additional projection syntax. -/
instance : Coe MModule MProgram := ⟨MModule.toMProgram⟩

end MoveModel.Frontend.XIR
