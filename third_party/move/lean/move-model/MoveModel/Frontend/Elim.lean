-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Frontend.XIR
import MoveModel.IR.RefElim.Transform

/-!
# Reference Elimination over Dumped Modules

`MProgram.elim` runs whole-program reference elimination on a dumped module.
The pipeline first removes immutable references, then computes borrow
summaries by Kleene iteration, and finally rewrites mutable references into
the mutation algebra.

The transformation stays in the first-order representation and preserves
declaration names.  `masmElim%` and `moveElim%` splice its result into Lean.

This is the executable interprocedural pipeline.  The current correctness
theorem has a narrower scope: `refElim_correct` covers the isolated,
summary-free `refElimFun` pipeline.  It does not yet prove preservation for
the computed cross-call summaries used here.
-/

namespace MoveModel.Frontend.XIR

open MoveModel.IR

/-- Rebuild the list-based loop metadata from an elimination result:
split-off blocks join the loops of their source block, and the targets
gain everything the inserted code writes inside (the list-level image of
`ElimOut.toFunDecl`'s propositional extension). -/
private def elimLoop (out : ElimOut) (l : MLoop) : MLoop :=
  let members := l.members ++
    (out.blockSrc.filter (fun p => l.members.contains p.2)).map (·.1)
  let extra := (members.flatMap out.defsIn).filter
    (fun x => !l.valTargets.contains x)
  { l with
    members := members
    valTargets := l.valTargets ++ extra.eraseDups }

/-- Runs executable interprocedural reference elimination while retaining
declaration names.  See the module documentation for its proof boundary. -/
def MProgram.elim (p : MProgram) : Except String MProgram := do
  let Δ : StructDecls := fun r => (p.structs[r]?).map MStruct.toStructDecl
  let sigs : FunId → Option FunDecl := fun f =>
    (p.funs[f]?).map MFun.toFunDecl
  let post ← p.funs.mapM fun f => do
    match elimImmRefs sigs f.toFunDecl with
    | .ok d => pure d
    | .error e => throw s!"in `{f.name}`: {e}"
  let table ← computeSummaries post
  let sums : Summaries := fun f => table[f]?
  let funs' ← (p.funs.zip post).mapM fun (mf : MFun × FunDecl) => do
    let (mf, d) := mf
    match elimCoreOut sums Δ d with
    | .error e => throw s!"in `{mf.name}`: {e}"
    | .ok out =>
        pure { mf with
          params := out.numParams
          locals := out.localTys
          returns := out.returns
          blocks := out.blocks
          loops := mf.loops.map (elimLoop out) }
  pure { p with funs := funs' }

end MoveModel.Frontend.XIR
