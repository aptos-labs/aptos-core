-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Frontend.Repr
import Move.IR.RefElim

/-!
# Reference Elimination over Dumped Modules

`MProgram.elim` runs the whole-program reference elimination
(`refElimProg`'s pipeline: imm-ref pre-pass, borrow summaries by Kleene
iteration, mutation-algebra rewriting) over a dumped module, staying in
the first-order representation — declaration names are kept, and the
result can be spliced back into terms by the `masmElim%`/`moveElim%`
elaborators (`Elab.lean`).
-/

namespace Move.Frontend

open Move.IR

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

/-- Whole-module reference elimination, keeping declaration names. -/
def MProgram.elim (p : MProgram) : Except String MProgram := do
  let Δ : StructDecls := fun r => (p.structs[r]?).map MStruct.toStructDecl
  let post ← p.funs.mapM fun f => do
    match elimImmRefs f.toFunDecl with
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

end Move.Frontend
