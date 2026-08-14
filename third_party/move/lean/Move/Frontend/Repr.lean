-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.IR.Value
import Move.IR.State
import Move.IR.Spec
import Move.IR.Contract
import Move.IR.Syntax

/-!
# First-Order Program Representation for the Exchange Format

The `masm%` and `move%` elaborators invoke `aptos move exchange`.  Masm input
is assembled; Move input is compiled.  Both paths use the real toolchain and
`StacklessBytecodeGenerator`, then export the basic-block CFG as JSON.

This module defines `MProgram`, the first-order mirror of `Program` decoded
from that JSON.  Its components are list-based data that the elaborator can
quote directly.  `MProgram.toProgram` then builds the function-valued fields
of the semantic representation.  Proofs can unfold that conversion and its
list lookups with `simp`.
-/

namespace Move.Frontend

open Move.IR

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
  params : Nat
  locals : List Ty
  returns : List Ty
  blocks : List Block
  loops : List MLoop
  spec : MContract

/-- A struct declaration: name and fields with their types.  The names are
documentation; resource ids and field offsets are positional. -/
structure MStruct where
  name : String
  fields : List (String × Ty)

/-- A dumped module: the struct and function declarations (resource and
fun ids are positional). -/
structure MProgram where
  structs : List MStruct
  funs : List MFun

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
  memTargets := fun loc => loc.rsrc ∈ l.memTargets
  members := fun b => b ∈ l.members

def MFun.toFunDecl (f : MFun) : FunDecl where
  numParams := f.params
  numLocals := f.locals.length
  locals := fun t => f.locals[t]?
  returns := f.returns
  body := { blocks := fun b => f.blocks[b]?, entry := 0, size := f.blocks.length }
  loopSpecs := fun b => (f.loops.find? (fun l => l.header = b)).map MLoop.toLoopSpec
  contract := f.spec.toContract

def MStruct.toStructDecl (s : MStruct) : StructDecl where
  fields := s.fields.map (·.2)

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

end Move.Frontend
