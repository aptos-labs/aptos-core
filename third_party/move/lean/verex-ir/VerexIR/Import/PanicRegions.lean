-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Import.Raw

/-!
# Collapsing diverging regions

A panic in MIR is a region of blocks: the message is formatted, then a function
that never returns is called. The source has one operation, and under
`panic=abort` with a stateless failure nothing the region computes is observable,
so a raw graph is normalized before check folding: every region collapses to
`abort`, which holds no statement and no call.

A block diverges when every path from it ends in a call without a destination,
`abort`, or `unreachable`, computed bottom-up, so a block on a cycle does not. No
callee is named: a diverging callee that does not panic only adds a spurious panic
path, which is sound and incomplete.

Only a block with such a call on some path collapses. `unreachable` alone says
that a path cannot happen, as a switch's impossible default does, and must not
become a panic path. It stays, and leaves the graph only where it collapses
together with a call, whose block then fails on every path that can happen.

A terminator whose unwind edge is anything but `unreachable` is not one under
`panic=abort` and stays for the structurizer to reject.
-/

namespace VerexIR.Import.PanicRegions

/-- How a block's paths end, as far as known. -/
private inductive Ending where
  | unknown
  /-- Every path fails or cannot happen, and none is known to hold a call. -/
  | diverges
  /-- Every path fails or cannot happen, and some path holds a diverging call. -/
  | holdsCall
  deriving BEq, Inhabited

/-- How a terminator ends a path on its own, when it does. -/
private def ownEnding : RawTerminator → Ending
  | .call _ none .unreachable => .holdsCall
  | .abort | .unreachable => .diverges
  | _ => .unknown

/-- The blocks after a terminator a region may extend over. -/
private def continuations? (terminator : RawTerminator) : Option (Array BlockId) :=
  match terminator with
  | .goto _ | .branch .. | .switch .. | .call _ (some _) .unreachable
  | .drop _ _ .unreachable | .assert _ _ _ _ .unreachable => some terminator.successors
  | _ => none

/-- One bottom-up step for a block, from its successors' endings. -/
private def step (endings : Array Ending) (block : RawBasicBlock) (current : Ending) : Ending :=
  match continuations? block.terminator with
  | none => current
  | some targets =>
      let target (id : BlockId) := endings[id.index]?.getD .unknown
      if targets.isEmpty || targets.any (target · == .unknown) then current
      else if targets.any (target · == .holdsCall) then .holdsCall
      else .diverges

/-- The least fixed point: a block's ending follows only from its successors',
so a block on a cycle stays unknown. -/
private def endings (graph : RawCfg) : Array Ending := Id.run do
  let mut result := graph.blocks.map (ownEnding ·.terminator)
  for _ in [0:graph.blocks.size + 1] do
    let next := (graph.blocks.zip result).map fun (block, current) => step result block current
    if next == result then break
    result := next
  return result

/-- The blocks every path from which fails or cannot happen. -/
def diverging (graph : RawCfg) : Array Bool :=
  (endings graph).map (· != .unknown)

/-- Collapse every diverging region holding a call to `abort`. A block of a
region keeps its location, so that the `abort` of a region's entry is the panic's
site, and interior blocks that nothing reaches any more are pruned with the rest
of the unreachable ones. -/
def collapse (graph : RawCfg) : RawCfg :=
  let ends := endings graph
  { graph with blocks := (graph.blocks.zip ends).map fun (block, ending) =>
      if ending == .holdsCall then { block with statements := #[], terminator := .abort }
      else block }

/-- A unit with its regions collapsed, for what reads its calls before validation. -/
def collapseUnit (unit : RawUnit) : RawUnit :=
  { unit with namespaces := unit.namespaces.map fun ns =>
    { ns with functions := ns.functions.map fun function =>
      match function.body with
      | .cfg graph => { function with body := .cfg (collapse graph) }
      | _ => function } }

end VerexIR.Import.PanicRegions
