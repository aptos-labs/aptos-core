-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Config
import Spec.Properties.Structure

/-!
# Syntactic bounds (L11)

TRUSTED. Property L11 of one module's text (plan §7, group L): every bound
the recorded configuration sets on what a module writes down, each with its
measured object (plan §4.1, "acceptance limits": "soundness does not depend on
their values, except the bounds consumers rely on").

- struct definitions per module (`maxStructDefinitions`), function
  definitions per module (`maxFunctionDefinitions`);
- variants per enum (`maxStructVariants`), fields per struct and per variant
  of an enum (`maxFieldsInStruct`);
- parameters and return values per function definition
  (`maxFunctionParameters`, `maxFunctionReturnValues`);
- basic blocks per function (`maxBasicBlocks`) and loop depth
  (`maxLoopDepth`);
- the size and depth of every type token in the signature pool and of every
  field's type (`maxTypeNodes`, `maxTypeDepth`).

The operand-stack height is L4's (`CodeStructure`, at `maxValueStackSize`).

**Syntactic only.** Every measured object is a token or a table as written.
No bound holds after substitution: a type parameter stands for a type of any
size, and under cyclic dependencies (plan §4.3) instantiation can build types
without bound. That is R7 (plan §6), a run-time obligation, not this
property.

**Type size.** The plan measures "the weighted size ... of syntactic type
tokens (structs and type parameters weigh more because they may expand)".
The weights are not part of the language definition, so L11 states the bound
on the number of nodes of a token (`SignatureToken.nodeCount`), which a
weighted size in which every node weighs at least one bounds from above.

A limit that the configuration sets to `none` is no bound.
-/

namespace MoveBytecode

/-- `n` is within an optional limit; `none` is no limit. -/
def WithinLimit (limit : Option Nat) (n : Nat) : Prop :=
  ∀ k, limit = some k → n ≤ k

mutual
/-- The number of nodes of a type token: one for the token, plus those of its
components (a vector's element, a struct instantiation's type arguments, a
reference's target, a function type's parameters and results). -/
def SignatureToken.nodeCount : SignatureToken → Nat
  | .bool | .u8 | .u16 | .u32 | .u64 | .u128 | .u256 => 1
  | .i8 | .i16 | .i32 | .i64 | .i128 | .i256 => 1
  | .address | .signer | .struct _ | .typeParameter _ => 1
  | .vector elem => elem.nodeCount + 1
  | .function ps rs _ => SignatureToken.nodeCountList ps + SignatureToken.nodeCountList rs + 1
  | .structInstantiation _ args => SignatureToken.nodeCountList args + 1
  | .reference t | .mutableReference t => t.nodeCount + 1

def SignatureToken.nodeCountList : List SignatureToken → Nat
  | [] => 0
  | t :: ts => t.nodeCount + SignatureToken.nodeCountList ts
end

mutual
/-- The depth of a type token: one for a token without components, one more
than its deepest component otherwise. -/
def SignatureToken.depth : SignatureToken → Nat
  | .bool | .u8 | .u16 | .u32 | .u64 | .u128 | .u256 => 1
  | .i8 | .i16 | .i32 | .i64 | .i128 | .i256 => 1
  | .address | .signer | .struct _ | .typeParameter _ => 1
  | .vector elem => elem.depth + 1
  | .function ps rs _ => max (SignatureToken.depthList ps) (SignatureToken.depthList rs) + 1
  | .structInstantiation _ args => SignatureToken.depthList args + 1
  | .reference t | .mutableReference t => t.depth + 1

def SignatureToken.depthList : List SignatureToken → Nat
  | [] => 0
  | t :: ts => max t.depth (SignatureToken.depthList ts)
end

/-! ## Loops and basic blocks -/

/-- Instruction `pc` is reachable from the entry. -/
def EntryReaches (code : Array Bytecode) (pc : Nat) : Prop :=
  ∃ p, Walk (successors code) 0 pc p

/-- A back edge: an edge from a reachable instruction `a` to an instruction
`h` that dominates it, `h` being the head of a loop. -/
def BackEdge (code : Array Bytecode) (a h : Nat) : Prop :=
  h ∈ successors code a ∧ EntryReaches code a ∧ Dominates (successors code) h a

/-- Instruction `pc` is in the loop headed by `h`: `pc` is `h`, or reaches the
source of a back edge into `h` without passing through `h`. -/
def InLoop (code : Array Bytecode) (h pc : Nat) : Prop :=
  ∃ a, BackEdge code a h ∧ (pc = h ∨ ∃ p, Walk (successors code) pc a p ∧ h ∉ p)

/-- Every reachable instruction is in at most `k` loops: its loop depth. -/
def LoopDepthAtMost (code : Array Bytecode) (k : Nat) : Prop :=
  ∀ pc, EntryReaches code pc → ∀ heads : List Nat, heads.Nodup →
    (∀ h ∈ heads, InLoop code h pc) → heads.length ≤ k

/-- The code has at most `k` basic blocks: at most `k` instructions begin one
(`IsLeader`). -/
def BasicBlocksAtMost (code : Array Bytecode) (k : Nat) : Prop :=
  ∀ leaders : List Nat, leaders.Nodup → (∀ l ∈ leaders, l < code.size ∧ IsLeader code l) →
    leaders.length ≤ k

namespace CompiledModule

/-- The size and depth of a type token are within the configured bounds. -/
def TokenWithinLimits (cfg : VerifierConfig) (t : SignatureToken) : Prop :=
  WithinLimit cfg.maxTypeNodes t.nodeCount ∧ WithinLimit cfg.maxTypeDepth t.depth

/-- **L11.** The module is within every syntactic bound of configuration
`cfg` (module docstring). -/
def WithinLimits (m : CompiledModule) (cfg : VerifierConfig) : Prop :=
  WithinLimit cfg.maxStructDefinitions m.structDefs.size ∧
  WithinLimit cfg.maxFunctionDefinitions m.functionDefs.size ∧
  (∀ d ∈ m.structDefs.toList, match d.fieldInformation with
    | .native => True
    | .declared fields => WithinLimit cfg.maxFieldsInStruct fields.length
    | .declaredVariants variants =>
      WithinLimit cfg.maxStructVariants variants.length ∧
      ∀ v ∈ variants, WithinLimit cfg.maxFieldsInStruct v.fields.length) ∧
  (∀ d ∈ m.functionDefs.toList, ∃ fh params returns, m.functionHandles[d.function]? = some fh ∧
    m.signatures[fh.parameters]? = some params ∧ m.signatures[fh.returns]? = some returns ∧
    WithinLimit cfg.maxFunctionParameters params.length ∧
    WithinLimit cfg.maxFunctionReturnValues returns.length) ∧
  (∀ d ∈ m.functionDefs.toList, ∀ code, d.code = some code →
    (∀ k, cfg.maxBasicBlocks = some k → BasicBlocksAtMost code.code k) ∧
    (∀ k, cfg.maxLoopDepth = some k → LoopDepthAtMost code.code k)) ∧
  (∀ sig ∈ m.signatures.toList, ∀ t ∈ sig, TokenWithinLimits cfg t) ∧
  (∀ d ∈ m.structDefs.toList, ∀ f ∈ StructFieldInformation.allFields d.fieldInformation,
    TokenWithinLimits cfg f.signature)

end CompiledModule

end MoveBytecode
