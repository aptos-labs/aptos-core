-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Model.Cfg

/-!
# Abstract interpretation

UNTRUSTED. Transcribes `AbstractInterpreter::analyze_function`
(`move-bytecode-verifier/src/absint.rs`): starting from the entry block, run
each block from its recorded pre-state, join the post-state into each
successor's pre-state (recording it on first visit), and move on to the next
block in traversal order, or back to a loop head whose pre-state a back-edge
join changed, whichever comes first in traversal order. Blocks that are never
reached are never analyzed.

The result is the invariant map, from each analyzed block to its pre-state,
which the proofs use.
-/

namespace MoveBytecode.Model

/-- The functions an analysis supplies: `join old new` returns the joined
state and whether it differs from `old` (`JoinResult::Changed`), and
`execute state instr offset blockEnd` is the transfer function. -/
structure Analysis (S : Type) where
  join : S → S → VResult (S × Bool)
  execute : S → Bytecode → Nat → Nat → VResult S

def setPre {S : Type} (inv : List (Nat × S)) (b : Nat) (s : S) : List (Nat × S) :=
  inv.map (fun e => if e.1 == b then (b, s) else e)

/-- `execute_block`. -/
def executeBlock {S : Type} (a : Analysis S) (cfg : Cfg) (code : Array Bytecode) (b : Nat)
    (pre : S) : VResult S := do
  let blockEnd := cfg.blockEnd b
  (List.range' b (blockEnd + 1 - b)).foldlM (fun st offset => do
    a.execute st (← lookup code offset "code[offset]") offset blockEnd) pre

/-- Join a block's post-state into one successor: record it on first visit,
otherwise join, and schedule a back-edge target whose pre-state changed. -/
def propagate {S : Type} (a : Analysis S) (cfg : Cfg) (b : Nat) (post : S)
    (acc : List (Nat × S) × List Nat) (succ : Nat) : VResult (List (Nat × S) × List Nat) := do
  let (inv, candidates) := acc
  match inv.lookup succ with
  | none => pure (inv ++ [(succ, post)], candidates)
  | some old =>
    let (joined, changed) ← a.join old post
    if changed then
      pure (setPre inv succ joined,
        if cfg.isBackEdge b succ then candidates ++ [succ] else candidates)
    else pure (inv, candidates)

/-- The candidate first in traversal order (`min_by_key`, which keeps the
first of equal keys). -/
def firstInTraversal (cfg : Cfg) : List Nat → Option Nat
  | [] => none
  | c :: cs =>
    match firstInTraversal cfg cs with
    | none => some c
    | some d => if cfg.traversalIndex d < cfg.traversalIndex c then some d else some c

/-- The main loop of `analyze_function`. -/
def analyzeLoop {S : Type} (a : Analysis S) (cfg : Cfg) (code : Array Bytecode) :
    Nat → List (Nat × S) → Option Nat → VResult (List (Nat × S))
  | _, inv, none => pure inv
  | 0, _, some _ => outOfFuel "analyze_function"
  | fuel + 1, inv, some b =>
    match inv.lookup b with
    | none => analyzeLoop a cfg code fuel inv (cfg.nextBlock b)
    | some pre => do
      let post ← executeBlock a cfg code b pre
      let candidates := match cfg.nextBlock b with
        | some n => [n]
        | none => []
      let (inv, candidates) ← (cfg.successors b).foldlM (propagate a cfg b post) (inv, candidates)
      analyzeLoop a cfg code fuel inv (firstInTraversal cfg candidates)

/-- `analyze_function`, with a fuel bound on block executions. A block is
re-run only after a strictly increasing join at a loop head, so `fuel`
greater than the blocks times (`height` + 1) times the blocks suffices, for a
domain whose ascending chains have at most `height` steps. -/
def analyzeFunction {S : Type} (a : Analysis S) (cfg : Cfg) (code : Array Bytecode)
    (initial : S) (height : Nat) : VResult (List (Nat × S)) :=
  let n := cfg.blocks.length + 1
  analyzeLoop a cfg code (n * n * (height + 1) + n) [(0, initial)] (some 0)

end MoveBytecode.Model
