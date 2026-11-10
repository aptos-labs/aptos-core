-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Config
import Model.Cfg

/-!
# Control-flow verification

UNTRUSTED. Transcribes `move-bytecode-verifier/src/control_flow.rs` for
bytecode version 6 and later, with `loop_summary.rs`: the code is non-empty
and ends in an unconditional branch, and the control-flow graph is reducible
(Tarjan's algorithm), with loop nesting depth at most `max_loop_depth`.

Versions up to 5 use `control_flow_v5.rs`, which is not modeled yet (plan
§4.2, deliverable A5); the model rejects them with a `panic`-free status so
that nothing is accepted on an unmodeled path.
-/

namespace MoveBytecode.Model

/-- `verify_fallthrough`. -/
def verifyFallthrough (code : CodeUnit) : VResult Unit :=
  match code.code.back? with
  | none => reject "EMPTY_CODE_UNIT"
  | some last => if !isUnconditionalBranch last then reject "INVALID_FALL_THROUGH" else pure ()

/-! ## `LoopSummary` -/

/-! A node of the depth-first spanning tree is its pre-order number, a `Nat`. -/

structure LoopSummary where
  /-- The block of each node. -/
  blocks : Array Nat
  /-- The number of transitive descendants of each node in the spanning tree. -/
  descs : Array Nat
  /-- Back-edge sources of each node, in discovery order. -/
  backs : Array (List Nat)
  /-- The other incoming edges of each node, in discovery order. -/
  preds : Array (List Nat)
  deriving Repr, Inhabited

inductive SummaryExploration where
  | inProgress (node : Nat)
  | done (node : Nat)
  deriving DecidableEq, Repr

inductive Frontier where
  | visit (fromNode : Nat) (toBlock : Nat)
  | finish (block : Nat) (node : Nat) (parent : Nat)
  deriving DecidableEq, Repr

structure SummaryState where
  summary : LoopSummary
  exploration : List (Nat × SummaryExploration)
  nextNode : Nat
  /-- The head is the top. -/
  stack : List Frontier
  deriving Repr, Inhabited

def modifyAt {α : Type} (a : Array α) (i : Nat) (f : α → α) : Array α :=
  match a[i]? with
  | some x => a.set! i (f x)
  | none => a

def SummaryState.step (cfg : Cfg) (st : SummaryState) : Frontier → SummaryState
  | .finish block node parent =>
    let d := st.summary.descs[node]?.getD 0
    { st with
      summary := { st.summary with descs := modifyAt st.summary.descs parent (· + 1 + d) }
      exploration := st.exploration.map
        (fun e => if e.1 == block then (block, .done node) else e) }
  | .visit fromNode toBlock =>
    match st.exploration.lookup toBlock with
    | some (.inProgress toNode) =>
      { st with summary := { st.summary with
          backs := modifyAt st.summary.backs toNode (· ++ [fromNode]) } }
    | some (.done toNode) =>
      { st with summary := { st.summary with
          preds := modifyAt st.summary.preds toNode (· ++ [fromNode]) } }
    | none =>
      let toNode := st.nextNode
      let s := st.summary
      let stack := Frontier.finish toBlock toNode fromNode :: st.stack
      let stack := (cfg.successors toBlock).foldl (fun stk succ => .visit toNode succ :: stk) stack
      { summary := { s with blocks := s.blocks.set! toNode toBlock,
                            preds := modifyAt s.preds toNode (· ++ [fromNode]) }
        exploration := (toBlock, .inProgress toNode) :: st.exploration
        nextNode := toNode + 1
        stack }

def SummaryState.run (cfg : Cfg) : Nat → SummaryState → VResult SummaryState
  | 0, _ => outOfFuel "loop summary"
  | fuel + 1, st =>
    match st.stack with
    | [] => pure st
    | action :: rest => SummaryState.run cfg fuel (SummaryState.step cfg { st with stack := rest } action)

/-- `LoopSummary::new`. Each block is visited once per incoming edge and
finished once, so twice the edges plus blocks bound the iterations. -/
def LoopSummary.build (cfg : Cfg) : VResult LoopSummary := do
  let n := cfg.blocks.length
  let edges := cfg.blocks.foldl (fun k b => k + b.successors.length) 0
  let root := 0
  let summary : LoopSummary :=
    { blocks := (Array.replicate n 0).set! root 0, descs := Array.replicate n 0,
      backs := Array.replicate n [], preds := Array.replicate n [] }
  -- Rust collects the root's successors into a vector and pops from its end.
  let stack := ((cfg.successors 0).map (fun succ => Frontier.visit root succ)).reverse
  let st ← SummaryState.run cfg (2 * (edges + n) + 2)
    { summary, exploration := [(0, .inProgress root)], nextNode := 1, stack }
  pure st.summary

namespace LoopSummary

def isDescendant (s : LoopSummary) (ancestor descendant : Nat) : Bool :=
  ancestor ≤ descendant && descendant ≤ ancestor + s.descs[ancestor]!

def backEdges (s : LoopSummary) (l : Nat) : List Nat := s.backs[l]!

def predEdges (s : LoopSummary) (l : Nat) : List Nat := s.preds[l]!

end LoopSummary

/-! ## `LoopPartition` -/

structure LoopPartition where
  parents : Array Nat
  depths : Array Nat
  deriving Repr, Inhabited

namespace LoopPartition

def new (s : LoopSummary) : LoopPartition :=
  { parents := (List.range s.blocks.size).toArray, depths := Array.replicate s.blocks.size 0 }

def parent (p : LoopPartition) (l : Nat) : Nat := p.parents[l]!

/-- `containing_loop`, with path compression. A parent chain has at most one
link per node, so `fuel` greater than the number of nodes suffices. -/
def containingLoop (p : LoopPartition) (id : Nat) (fuel : Nat) : VResult (Nat × LoopPartition) :=
  let child := id
  let parent := p.parent child
  let grandparent := p.parent parent
  if child == parent || parent == grandparent then pure (parent, p)
  else
    let rec walk : Nat → Nat → Nat → Nat → List Nat → VResult (Nat × List Nat)
      | 0, _, _, _, _ => outOfFuel "containing_loop"
      | fuel + 1, child, parent, grandparent, ds =>
        let ds := ds ++ [child]
        let (child, parent, grandparent) := (parent, grandparent, p.parent grandparent)
        if parent == grandparent then pure (parent, ds)
        else walk fuel child parent grandparent ds
    do
      let (root, ds) ← walk fuel child parent grandparent []
      pure (root, { p with parents := ds.foldl (fun ps d => ps.set! d root) p.parents })

/-- `collapse_loop`: point every constituent at the head and return the
head's new nesting depth. -/
def collapseLoop (p : LoopPartition) (head : Nat) (body : List Nat) : Nat × LoopPartition :=
  let (depth, parents) := body.foldl (fun (depth, parents) c =>
    (Nat.max (p.depths[c]!) depth, parents.set! c head)) (p.depths[head]!, p.parents)
  let depth := depth + 1
  (depth, { parents, depths := p.depths.set! head depth })

end LoopPartition

/-- Insert into a sorted list without duplicates (a `BTreeSet`); `true` when
the element was new. -/
def insertSorted (xs : List Nat) (x : Nat) : List Nat × Bool :=
  if xs.contains x then (xs, false) else ((xs ++ [x]).mergeSort, true)

/-- One predecessor of a frontier node: its loop must lie inside the head's
subtree, and joins the body and the frontier when new. -/
def visitPred (s : LoopSummary) (head nodeFuel : Nat)
    (acc : List Nat × List Nat × LoopPartition) (pred : Nat) :
    VResult (List Nat × List Nat × LoopPartition) := do
  let (body, frontier, p) := acc
  let (pred', p) ← p.containingLoop pred nodeFuel
  if !s.isDescendant head pred' then reject "INVALID_LOOP_SPLIT"
  if pred' != head then
    let (body', inserted) := insertSorted body pred'
    pure (body', if inserted then pred' :: frontier else frontier, p)
  else pure (body, frontier, p)

/-- The inner loop of `verify_reducibility` for one head: grow the body by
following predecessor edges until the head is found again. Each node joins
the body at most once, and each pop processes one frontier node, so `fuel`
greater than nodes times edges suffices. -/
def growBody (s : LoopSummary) (head : Nat) (nodeFuel : Nat) :
    Nat → List Nat → List Nat → LoopPartition → VResult (List Nat × LoopPartition)
  | 0, _, _, _ => outOfFuel "verify_reducibility"
  | fuel + 1, body, frontier, p =>
    match frontier with
    | [] => pure (body, p)
    | node :: frontier => do
      let (body, frontier, p) ←
        (s.predEdges node).foldlM (visitPred s head nodeFuel) (body, frontier, p)
      growBody s head nodeFuel fuel body frontier p

/-- One back-edge source: its loop joins the body unless it is the head. -/
def backStep (nodeFuel head : Nat) (acc : List Nat × LoopPartition) (node : Nat) :
    VResult (List Nat × LoopPartition) := do
  let (body, p) := acc
  let (node', p) ← p.containingLoop node nodeFuel
  pure (if node' != head then (insertSorted body node').1 else body, p)

/-- The loops of the back-edge sources of a head, other than the head. -/
def backBody (p : LoopPartition) (nodeFuel head : Nat) (back : List Nat) :
    VResult (List Nat × LoopPartition) :=
  back.foldlM (backStep nodeFuel head) ([], p)

/-- The body of the outer loop of `verify_reducibility`, for one head. -/
def processHead (config : VerifierConfig) (s : LoopSummary) (nodeFuel growFuel : Nat)
    (p : LoopPartition) (head : Nat) : VResult LoopPartition := do
  let back := s.backEdges head
  if back.isEmpty then return p
  let (body, p) ← backBody p nodeFuel head back
  -- Rust pops the frontier from the end of the sorted body.
  let (body, p) ← growBody s head nodeFuel growFuel body body.reverse p
  let (depth, p) := p.collapseLoop head body
  match config.maxLoopDepth with
  | some maxDepth => if depth > maxDepth then reject "LOOP_MAX_DEPTH_REACHED" else pure p
  | none => pure p

/-- `verify_reducibility`, visiting the nodes in reverse pre-order so that
inner loops are collapsed first. -/
def verifyReducibility (config : VerifierConfig) (cfg : Cfg) : VResult Unit := do
  let s ← LoopSummary.build cfg
  let n := s.blocks.size
  let edges := cfg.blocks.foldl (fun k b => k + b.successors.length) 0
  let _ ← (List.range n).reverse.foldlM
    (processHead config s (n + 1) ((n + 1) * (edges + n + 1))) (LoopPartition.new s)
  pure ()

/-- `control_flow::verify_function`, returning the control-flow graph. -/
def verifyControlFlow (config : VerifierConfig) (version : Nat) (code : CodeUnit) : VResult Cfg := do
  if version ≤ 5 then panicAt "control_flow_v5 is not modeled"
  verifyFallthrough code
  let cfg ← Cfg.build code.code
  verifyReducibility config cfg
  pure cfg

end MoveBytecode.Model
