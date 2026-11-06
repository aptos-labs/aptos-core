-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Syntax
import Model.Result

/-!
# Control-flow graph

UNTRUSTED. Transcribes `move_binary_format::control_flow_graph::VMControlFlowGraph`
(`third_party/move/move-binary-format/src/control_flow_graph.rs`) and the
`Bytecode` helpers it uses (`file_format.rs`): block boundaries, block
successors, the depth-first traversal order used by abstract interpretation,
and the back edges found during that traversal.

Rust maps (`IndexMap`, `BTreeSet`) become association lists; only their
membership and iteration order are relied on. Loops whose termination Rust
takes for granted get a fuel bound that is sufficient (see each comment).
-/

namespace MoveBytecode.Model

/-! ## `Bytecode` helpers (`file_format.rs`) -/

/-- `Bytecode::offset`. -/
def offset : Bytecode → Option Nat
  | .brFalse o | .brTrue o | .branch o => some o
  | _ => none

/-- `Bytecode::is_unconditional_branch`. -/
def isUnconditionalBranch : Bytecode → Bool
  | .ret | .abort | .abortMsg | .branch _ => true
  | _ => false

/-- `Bytecode::is_conditional_branch`. -/
def isConditionalBranch : Bytecode → Bool
  | .brFalse _ | .brTrue _ => true
  | _ => false

/-- `Bytecode::is_branch`. -/
def isBranch (b : Bytecode) : Bool := isConditionalBranch b || isUnconditionalBranch b

/-- `Bytecode::get_successors`: the jump target, then the next instruction
unless the instruction is an unconditional branch or the next instruction is
past the end; in ascending order. -/
def getSuccessors (pc : Nat) (code : Array Bytecode) : List Nat :=
  match code[pc]? with
  | none => []
  | some b =>
    let v := match offset b with
      | some o => [o]
      | none => []
    if pc + 1 ≥ code.size then v
    else
      let v := if !isUnconditionalBranch b && !v.contains (pc + 1) then v ++ [pc + 1] else v
      match v with
      | [a, c] => if a > c then [c, a] else v
      | _ => v

/-! ## Blocks -/

/-- `record_block_ids`: the jump target, and the instruction after a branch. -/
def recordBlockIds (code : Array Bytecode) (ids : List Nat) (pc : Nat) : List Nat :=
  match code[pc]? with
  | none => ids
  | some b =>
    let ids := match offset b with
      | some o => o :: ids
      | none => ids
    if isBranch b && pc + 1 < code.size then (pc + 1) :: ids else ids

/-- The block ids: the entry and every recorded id. -/
def blockIds (code : Array Bytecode) : List Nat :=
  (List.range code.size).foldl (recordBlockIds code) [0]

/-- `is_end_of_block`. -/
def isEndOfBlock (code : Array Bytecode) (ids : List Nat) (pc : Nat) : Bool :=
  pc + 1 == code.size || ids.contains (pc + 1)

structure BasicBlock where
  /-- The block id: its first instruction. -/
  start : Nat
  /-- Its last instruction. -/
  exit : Nat
  successors : List Nat
  deriving DecidableEq, Repr, Inhabited

/-- The blocks, in increasing order of start (`IndexMap` insertion order). -/
def buildBlocks (code : Array Bytecode) : List BasicBlock :=
  let ids := blockIds code
  let step (acc : Nat × List BasicBlock) (pc : Nat) : Nat × List BasicBlock :=
    if isEndOfBlock code ids pc then (pc + 1, acc.2 ++ [⟨acc.1, pc, getSuccessors pc code⟩])
    else acc
  ((List.range code.size).foldl step (0, [])).2

/-! ## Traversal order and back edges -/

inductive Exploration where
  | inProgress
  | done
  deriving DecidableEq, Repr

structure Traversal where
  stack : List Nat
  exploration : List (Nat × Exploration)
  /-- Loop heads with their back-edge sources. -/
  loopHeads : List (Nat × List Nat)
  postOrder : List Nat
  deriving Repr, Inhabited

def blockSuccessors (blocks : List BasicBlock) (b : Nat) : List Nat :=
  match blocks.find? (·.start == b) with
  | some bb => bb.successors
  | none => []

def addBackEdge (heads : List (Nat × List Nat)) (head src : Nat) : List (Nat × List Nat) :=
  match heads.find? (·.1 == head) with
  | some (_, srcs) =>
    heads.map (fun e => if e.1 == head then (head, if srcs.contains src then srcs else srcs ++ [src])
      else e)
  | none => heads ++ [(head, [src])]

/-- One iteration of the depth-first exploration in `VMControlFlowGraph::new`. -/
def traversalStep (blocks : List BasicBlock) (t : Traversal) (block : Nat) (rest : List Nat) :
    Traversal :=
  match t.exploration.lookup block with
  | none =>
    -- Start exploring `block`: keep it on the stack to finish it later, and
    -- push the successors not yet seen; a successor in progress closes a loop.
    let t := { t with stack := block :: rest,
                      exploration := (block, .inProgress) :: t.exploration }
    (blockSuccessors blocks block).foldl (fun t succ =>
      match t.exploration.lookup succ with
      | none => { t with stack := succ :: t.stack }
      | some .inProgress => { t with loopHeads := addBackEdge t.loopHeads succ block }
      | some .done => t) t
  | some .done => { t with stack := rest }
  | some .inProgress =>
    { t with stack := rest, postOrder := t.postOrder ++ [block],
             exploration := t.exploration.map (fun e => if e.1 == block then (block, .done) else e) }

/-- The depth-first exploration. Every block is pushed at most once per
incoming edge plus once more to finish it, so `fuel` iterations suffice when
`fuel` exceeds twice the number of edges plus blocks. -/
def traverse (blocks : List BasicBlock) : Nat → Traversal → VResult Traversal
  | 0, _ => outOfFuel "control-flow graph traversal"
  | fuel + 1, t =>
    match t.stack with
    | [] => pure t
    | block :: rest => traverse blocks fuel (traversalStep blocks t block rest)

structure Cfg where
  blocks : List BasicBlock
  /-- Reverse post-order of the blocks reachable from the entry. -/
  traversalOrder : List Nat
  loopHeads : List (Nat × List Nat)
  deriving Repr, Inhabited

/-- `VMControlFlowGraph::new`. -/
def Cfg.build (code : Array Bytecode) : VResult Cfg := do
  let blocks := buildBlocks code
  let edges := blocks.foldl (fun n b => n + b.successors.length) 0
  let t ← traverse blocks (2 * (edges + blocks.length) + 2)
    { stack := [0], exploration := [], loopHeads := [], postOrder := [] }
  pure { blocks, traversalOrder := t.postOrder.reverse, loopHeads := t.loopHeads }

namespace Cfg

def successors (cfg : Cfg) (b : Nat) : List Nat := blockSuccessors cfg.blocks b

def blockEnd (cfg : Cfg) (b : Nat) : Nat :=
  match cfg.blocks.find? (·.start == b) with
  | some bb => bb.exit
  | none => b

/-- The blocks, in increasing order of start. -/
def blockIdList (cfg : Cfg) : List Nat := cfg.blocks.map (·.start)

/-- `next_block`: the next block in traversal order. -/
def nextBlock (cfg : Cfg) (b : Nat) : Option Nat :=
  let rec go : List Nat → Option Nat
    | a :: c :: rest => if a == b then some c else go (c :: rest)
    | _ => none
  go cfg.traversalOrder

/-- `traversal_index`: the position in traversal order. A block missing from
`traversal_successors` (the last block, or one that is unreachable) gets that
map's size, the order's length minus one. -/
def traversalIndex (cfg : Cfg) (b : Nat) : Nat :=
  match cfg.traversalOrder.dropLast.idxOf? b with
  | some i => i
  | none => cfg.traversalOrder.length - 1

def isBackEdge (cfg : Cfg) (cur next : Nat) : Bool :=
  match cfg.loopHeads.lookup next with
  | some srcs => srcs.contains cur
  | none => false

def numBackEdges (cfg : Cfg) : Nat :=
  cfg.loopHeads.foldl (fun n e => n + e.2.length) 0

end Cfg

end MoveBytecode.Model
