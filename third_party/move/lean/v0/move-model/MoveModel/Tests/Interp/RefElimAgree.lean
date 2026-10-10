-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.RefElim.Transform
import MoveModel.IR.Interp.Exec
import MoveModel.Tests.Interp.References
import MoveModel.Tests.Interp.Globals

/-!
# Interpreter Tests: Reference Elimination Agreement

Executable evidence for `refElim_correct` on real compiler output and on
hand-written control-flow shapes: eliminate with `refElimFun` and check
that the eliminated program produces the `AgreeOutcome`-related
interpreter outcome — the source return is a prefix of the target return
before internal `&mut` finals, and abort codes are equal (abort *memory* may
differ: the elimination defers write-backs to borrow death, see
`RefElim.lean`).  Covers cross-block borrows, a
diamond join with the dynamic `is_parent` write-back dispatch, and a
death on only one branch edge (edge splitting).
-/

namespace Tests.Interp.RefElimAgree

open MoveModel.IR
open MoveModel.Frontend.XIR

/-- Eliminate references from every function of a dumped module, through
the whole-program pass (borrow summaries for cross-call references). -/
def elimProgram (m : MProgram) : Option Program :=
  (MProgram.elim m).toOption.map MProgram.toProgram

/-- The original and the eliminated program agree on one run: source normal
values are a prefix of the target values, and abort codes coincide
(`AgreeOutcome`, executably). -/
def agree (p q : Program) (f : FunId) (mem : IMem) (args : List Value) :
    Bool :=
  match interpFun p 1000 f mem args, interpFun q 1000 f mem args with
  | .ok (.ret world vals), .ok (.ret world' vals') =>
      world.memory == world'.memory && vals.isPrefixOf vals'
  | .ok (.abort _ c), .ok (.abort _ c') => c == c'
  | .error _, .error _ => true
  | _, _ => false

/-! ## The reference test module eliminates, and agrees -/

open Tests.Interp.References (refsM refs box) in
section
#guard (elimProgram refsM).isSome

private def refs' : Program := (elimProgram refsM).getD refs

#guard agree refs refs' (refsM.funId "bump") [] [.u64 41]
#guard agree refs refs' (refsM.funId "bump") [] [.u64 18446744073709551615]
#guard agree refs refs' (refsM.funId "set_field") [] [box 1 2, .u64 9]
#guard agree refs refs' (refsM.funId "set_nested") [] [box 1 2, .u64 9]
#guard agree refs refs' (refsM.funId "read_frozen") [] [.u64 0]
#guard agree refs refs' (refsM.funId "swap") [] [.u64 1, .u64 2]
end

/-! ## The globals test module eliminates, and agrees -/

open Tests.Interp.Globals (globalsM globals) in
section
#guard (elimProgram globalsM).isSome

private def globals' : Program := (elimProgram globalsM).getD globals

#guard agree globals globals' (globalsM.funId "publish") [] [.address 5, .u64 7]
#guard agree globals globals' (globalsM.funId "read")
  [(0, 5, .struct [.u64 7])] [.address 5]
#guard agree globals globals' (globalsM.funId "read") [] [.address 5]
#guard agree globals globals' (globalsM.funId "bump")
  [(0, 5, .struct [.u64 7])] [.address 5]
#guard agree globals globals' (globalsM.funId "bump") [] [.address 5]
#guard agree globals globals' (globalsM.funId "take")
  [(0, 5, .struct [.u64 7])] [.address 5]
end

/-! ## Hand-written control-flow shapes -/

private def trivialContract : Contract :=
  ⟨.value (.bool true), none, .value (.bool true), []⟩

private def mkFun (numParams : Nat) (locals : List Ty) (returns : List Ty)
    (blocks : List Block) : FunDecl where
  numParams := numParams
  numLocals := locals.length
  locals := fun t => locals[t]?
  returns := returns
  body :=
    { blocks := fun b => blocks[b]?
      entry := 0
      size := blocks.length }
  loopSpecs := fun _ => none
  contract := trivialContract

private def prog1 (d : FunDecl) : Program :=
  ⟨fun f => if f = 0 then some d else none, fun _ => none⟩

private def emptyProg : Program := ⟨fun _ => none, fun _ => none⟩

/-- A reference-producing assignment is a strong borrow-graph update.
Reusing local 3 must discard its obsolete parent edge rather than retaining
two write-back candidates in straight-line code. -/
private def reusedRefDecl : FunDecl := mkFun 0
  [.u64, .mutRef .u64, .mutRef .u64, .mutRef .u64] []
  [⟨[.assign 3 1, .assign 3 2], .ret []⟩]

private def reusedRefGraph : BGraph :=
  graphThroughBlock noSummaries reusedRefDecl
    ⟨[.assign 3 1, .assign 3 2], .ret []⟩ []

#guard inEdges reusedRefGraph 3 = [⟨.refNode 2, 3, []⟩]

/-- Eliminate a single hand-written function into a runnable program. -/
private def elim1 (d : FunDecl) : Option Program :=
  (refElimFun (fun _ => none) (fun _ => none) d).toOption.map fun d' =>
    ⟨fun f => if f = 0 then some d' else none, fun _ => none⟩

/-- Regression for the external ABI relation: the eliminated declaration
returns the internal final of every `&mut` parameter after the source return
values, even when that parameter is otherwise unused. -/
private def ignoredMutParam : FunDecl := mkFun 1
  [.mutRef .u64, .u64] [.u64]
  [⟨[.load 1 (.u64 7)], .ret [1]⟩]

#guard (elim1 ignoredMutParam).isSome
#guard agree (prog1 ignoredMutParam)
  ((elim1 ignoredMutParam).getD emptyProg) 0 [] [.u64 3]

/-- A borrow crossing a block boundary: borrowed in block 0, written and
released in block 1. -/
private def crossBlockBorrow : FunDecl := mkFun 2
  [.u64, .u64, .mutRef .u64]
  [.u64]
  [⟨[.call [2] .borrowLoc [0]], .jump 1⟩,
   ⟨[.call [] .writeRef [2, 1]], .ret [0]⟩]

#guard (elim1 crossBlockBorrow).isSome
#guard agree (prog1 crossBlockBorrow) ((elim1 crossBlockBorrow).getD emptyProg)
  0 [] [.u64 1, .u64 9]

/-- The TACAS'22 diamond: a reference joining from two different borrows —
the write-back needs the dynamic `is_parent`/`is_mut_loc` dispatch. -/
private def diamondBorrow : FunDecl := mkFun 3
  [.bool, .u64, .u64, .mutRef .u64, .u64, .u64]
  [.u64, .u64]
  [⟨[], .branch 0 1 2⟩,
   ⟨[.call [3] .borrowLoc [1]], .jump 3⟩,
   ⟨[.call [3] .borrowLoc [2]], .jump 3⟩,
   ⟨[.call [4] .readRef [3],
     .load 5 (.u64 100),
     .call [4] (.add .u64) [4, 5],
     .call [] .writeRef [3, 4]], .ret [1, 2]⟩]

#guard (elim1 diamondBorrow).isSome
#guard agree (prog1 diamondBorrow) ((elim1 diamondBorrow).getD emptyProg)
  0 [] [.bool true, .u64 1, .u64 2]
#guard agree (prog1 diamondBorrow) ((elim1 diamondBorrow).getD emptyProg)
  0 [] [.bool false, .u64 1, .u64 2]

/-- A borrow dying on one branch edge only (the other edge keeps using
it): the write-back lands in an edge-split block. -/
private def unevenDeath : FunDecl := mkFun 2
  [.bool, .u64, .mutRef .u64, .u64]
  [.u64]
  [⟨[.call [2] .borrowLoc [1],
     .load 3 (.u64 7),
     .call [] .writeRef [2, 3]], .branch 0 1 2⟩,
   ⟨[.load 3 (.u64 1), .call [] .writeRef [2, 3]], .jump 2⟩,
   ⟨[], .ret [1]⟩]

#guard (elim1 unevenDeath).isSome
#guard agree (prog1 unevenDeath) ((elim1 unevenDeath).getD emptyProg)
  0 [] [.bool true, .u64 0]
#guard agree (prog1 unevenDeath) ((elim1 unevenDeath).getD emptyProg)
  0 [] [.bool false, .u64 0]

/-- A mutable borrow of a *local* returned by the function — its frame
ends with the function, so the reference escapes: rejected (the borrow
checker forbids it too). -/
private def escapingBorrow : FunDecl := mkFun 1
  [.u64, .mutRef .u64]
  [.mutRef .u64]
  [⟨[.call [1] .borrowLoc [0]], .ret [1]⟩]

#guard refElimFun (fun _ => none) (fun _ => none) escapingBorrow matches .error _

/-! ## Exclusivity: verifier-illegal shapes the pass must reject

Each of these would *miscompile* without the exclusivity checks — the
eliminated code copies at borrow (or checks out a payload), the source
reads through the reference at use, and a mutation in the window makes
the two diverge.  They are executable counterexamples to
`refElim_correct` for the unhardened pass. -/

/-- Writing a local while an immutable borrow of it is live: the source
read-through sees `5`, the eliminated copy would still hold `1`. -/
private def immWriteUnderBorrow : FunDecl := mkFun 1
  [.u64, .ref .u64, .u64]
  [.u64]
  [⟨[.call [1] .borrowLoc [0],
     .load 0 (.u64 5),
     .call [2] .readRef [1]], .ret [2]⟩]

#guard refElimFun (fun _ => none) (fun _ => none) immWriteUnderBorrow matches .error _

/-- The mirror image: an immutable copy taken while a *pre-existing*
mutable borrow of the same root is live, then a write through the older
mutable reference.  The reference is not on the copy's derivation chain,
but the chains share the root — the imm layer alone must reject it
(`elimImm_correct` quantifies over `elimImmRefs` output before the
mutation pass runs). -/
private def immCopyUnderMutBorrow : FunDecl := mkFun 1
  [.u64, .mutRef .u64, .ref .u64, .u64, .u64]
  [.u64]
  [⟨[.call [1] .borrowLoc [0],
     .call [2] .borrowLoc [0],
     .load 3 (.u64 5),
     .call [] .writeRef [1, 3],
     .call [4] .readRef [2]], .ret [4]⟩]

/-- No callees: the imm-layer tests without calls run with an empty
signature table. -/
private def noSigs : FunId → Option FunDecl := fun _ => none

/-- One-callee tables for the boundary-checked call tests. -/
private def sig1 (d : FunDecl) : FunId → Option FunDecl :=
  fun f => if f = 1 then some d else none

#guard elimImmRefs noSigs immCopyUnderMutBorrow matches .error _

/-- A call-returned reference is rooted at *some* global (`anyRoot`):
writing through it while a global copy is live must be rejected even
though the caller's graph cannot name the callee's resource. -/
private def callRetRefWrite : FunDecl := mkFun 1
  [.address, .mutRef .u64, .ref .u64, .u64, .u64]
  [.u64]
  [⟨[.call [1] (.function 1) [0],
     .call [2] (.borrowGlobal 0) [0],
     .load 3 (.u64 5),
     .call [] .writeRef [1, 3],
     .call [4] .readRef [2]], .ret [4]⟩]

private def retMutCallee : FunDecl := mkFun 1
  [.address, .mutRef .u64]
  [.mutRef .u64]
  [⟨[], .ret [1]⟩]

#guard elimImmRefs (sig1 retMutCallee) callRetRefWrite matches .error _

/-- `move_to` re-creating a resource `move_from` removed while an
immutable borrow of it is live: the source read-through comes back with
the fresh value, the eliminated copy still holds the old one. -/
private def moveToUnderBorrow : FunDecl := mkFun 1
  [.address, .ref .u64, .u64, .u64, .u64]
  [.u64]
  [⟨[.call [1] (.borrowGlobal 0) [0],
     .call [2] (.moveFrom 0) [0],
     .load 3 (.u64 5),
     .call [] (.moveTo 0) [0, 3],
     .call [4] .readRef [1]], .ret [4]⟩]

#guard elimImmRefs noSigs moveToUnderBorrow matches .error _

/-- Control: the same shape with the borrow dead at the `move_to` is
accepted. -/
private def moveToAfterDeath : FunDecl := mkFun 1
  [.address, .ref .u64, .u64, .u64]
  [.u64]
  [⟨[.call [1] (.borrowGlobal 0) [0],
     .call [2] .readRef [1],
     .call [] (.moveTo 0) [0, 2]], .ret [2]⟩]

#guard elimImmRefs noSigs moveToAfterDeath matches .ok _

/-- Reference kinds must agree at call boundaries: an immutable-slot
argument landing in a *value* parameter would pass the copy where the
callee's source reads through the smuggled reference. -/
private def kindMismatchCallee : FunDecl := mkFun 1 [.u64] [.u64]
  [⟨[], .ret [0]⟩]

private def kindMismatchCaller : FunDecl := mkFun 1
  [.u64, .ref .u64, .u64]
  [.u64]
  [⟨[.call [1] .borrowLoc [0],
     .call [2] (.function 1) [1]], .ret [2]⟩]

#guard refElimProg (fun _ => none)
  [kindMismatchCaller, kindMismatchCallee] matches .error _

/-- Control: the same call against an `&`-typed parameter is fine. -/
private def kindMatchCallee : FunDecl := mkFun 1
  [.ref .u64, .u64] [.u64]
  [⟨[.call [1] .readRef [0]], .ret [1]⟩]

#guard refElimProg (fun _ => none)
  [kindMismatchCaller, kindMatchCallee] matches .ok _

/-- A `&mut`-typed source returned at a value-declared position: the
caller's boundary check keys on the declared return kinds, so the
declaration must not lie. -/
private def mutRetAtValuePos : FunDecl := mkFun 0
  [.u64, .mutRef .u64]
  [.u64]
  [⟨[.load 0 (.u64 1), .call [1] .borrowLoc [0]], .ret [1]⟩]

#guard elimImmRefs noSigs mutRetAtValuePos matches .error _

/-- Control: declared `&mut`, the imm layer accepts (the mutation layer
owns it from here). -/
private def mutRetDeclared : FunDecl := mkFun 0
  [.u64, .mutRef .u64]
  [.mutRef .u64]
  [⟨[.load 0 (.u64 1), .call [1] .borrowLoc [0]], .ret [1]⟩]

#guard elimImmRefs noSigs mutRetDeclared matches .ok _

/-- A `&mut`-slot argument into a *value* parameter: the callee's frame
could not cover the reference's root — kinds must agree exactly. -/
private def mutArgToValueParam : FunDecl := mkFun 1
  [.u64, .mutRef .u64, .u64]
  [.u64]
  [⟨[.call [1] .borrowLoc [0],
     .call [2] (.function 1) [1]], .ret [2]⟩]

#guard refElimProg (fun _ => none)
  [mutArgToValueParam, kindMismatchCallee] matches .error _

/-- The post-imm origin condition on `CoreProgram` is essential.  If the
core pass is applied independently to an ill-typed call boundary, both
functions eliminate but the callee appends its internal `&mut` final while
the caller still supplies only one destination.  The full pass rejects this
shape in its immutable boundary check. -/
private def coreOnlyFinalCallee : FunDecl := mkFun 1
  [.mutRef .u64] [.mutRef .u64]
  [⟨[], .ret [0]⟩]

private def coreOnlyFinalCaller : FunDecl := mkFun 1
  [.u64, .u64] [.u64]
  [⟨[.call [1] (.function 1) [0]], .ret [1]⟩]

private def coreOnlyProgram (caller callee : FunDecl) : Program :=
  ⟨fun f => if f = 0 then some caller
    else if f = 1 then some callee else none,
   fun _ => none⟩

private def coreOnlyFinalTarget : Option Program := do
  let caller ←
    (elimCore noSummaries (fun _ => none) coreOnlyFinalCaller).toOption
  let callee ←
    (elimCore noSummaries (fun _ => none) coreOnlyFinalCallee).toOption
  pure (coreOnlyProgram caller callee)

#guard (elimCore noSummaries (fun _ => none) coreOnlyFinalCaller).isOk
#guard (elimCore noSummaries (fun _ => none) coreOnlyFinalCallee).isOk
#guard interpFun
  (coreOnlyProgram coreOnlyFinalCaller coreOnlyFinalCallee)
  20 0 [] [.u64 7] matches .ok (.ret _ [.u64 7])
#guard interpFun (coreOnlyFinalTarget.getD emptyProg)
  20 0 [] [.u64 7] matches .error (.stuck "arity mismatch in call results")
#guard refElimProg (fun _ => none)
  [coreOnlyFinalCaller, coreOnlyFinalCallee] matches .error _

/-- A reference-typed parameter may be rooted at *some* global
(`paramSeeds`): a global write while a copy derived from the parameter
is live must be rejected — the copy's target may sit in the written
resource. -/
private def globalWriteUnderParamCopy : FunDecl := mkFun 1
  [.ref .u64, .ref .u64, .address, .u64, .u64]
  [.u64]
  [⟨[.assign 1 0,
     .load 2 (.address 7),
     .load 3 (.u64 5),
     .call [] (.writeGlobal 0) [2, 3],
     .call [4] .readRef [1]], .ret [4]⟩]

#guard elimImmRefs noSigs globalWriteUnderParamCopy matches .error _

/-- Control: the write after the copy is dead is accepted. -/
private def globalWriteAfterParamCopyDead : FunDecl := mkFun 1
  [.ref .u64, .ref .u64, .address, .u64, .u64]
  [.u64]
  [⟨[.assign 1 0,
     .call [4] .readRef [1],
     .load 2 (.address 7),
     .load 3 (.u64 5),
     .call [] (.writeGlobal 0) [2, 3]], .ret [4]⟩]

#guard elimImmRefs noSigs globalWriteAfterParamCopyDead matches .ok _

/-- A mutable reference and an immutable copy of the same root passed to
the *same* call: the callee's shadow write-backs sequence against the
copy's checkout at return — rejected. -/
private def mutAndImmArg : FunDecl := mkFun 1
  [.u64, .mutRef .u64, .ref .u64, .u64]
  [.u64]
  [⟨[.call [1] .borrowLoc [0],
     .call [2] .borrowLoc [0],
     .call [3] (.function 1) [1, 2]], .ret [3]⟩]

private def mutImmCallee : FunDecl := mkFun 2
  [.mutRef .u64, .ref .u64, .u64]
  [.u64]
  [⟨[.call [2] .readRef [1]], .ret [2]⟩]

#guard elimImmRefs (sig1 mutImmCallee) mutAndImmArg matches .error _

/-- Control: an immutable argument alone is fine. -/
private def immArgOnly : FunDecl := mkFun 1
  [.u64, .ref .u64, .u64]
  [.u64]
  [⟨[.call [1] .borrowLoc [0],
     .call [2] (.function 1) [1]], .ret [2]⟩]

private def immParamCallee : FunDecl := mkFun 1
  [.ref .u64, .u64]
  [.u64]
  [⟨[.call [1] .readRef [0]], .ret [1]⟩]

#guard elimImmRefs (sig1 immParamCallee) immArgOnly matches .ok _

/-- An immutable borrow of a frame local escaping through `ret`: the
returned copy has no target for the caller to re-root — rejected. -/
private def immEscape : FunDecl := mkFun 0
  [.u64, .ref .u64]
  [.ref .u64]
  [⟨[.load 0 (.u64 1), .call [1] .borrowLoc [0]], .ret [1]⟩]

#guard elimImmRefs noSigs immEscape matches .error _

/-- Returning `&`-references is outside the fragment altogether: the
pass would return the copy where the raw semantics returns the
reference, and a per-function pass cannot rewrite the callers' view
(`&mut` returns go through the mutation pass's finals machinery). -/
private def immRetFromParam : FunDecl := mkFun 1
  [.ref .u64, .ref .u64]
  [.ref .u64]
  [⟨[.assign 1 0], .ret [1]⟩]

#guard elimImmRefs noSigs immRetFromParam matches .error _

/-- Control: an `&`-typed ret source holding a provably plain value (the
result of a read, never a borrow) is fine. -/
private def immRetValue : FunDecl := mkFun 1
  [.u64, .ref .u64, .ref .u64]
  [.u64]
  [⟨[.call [1] .borrowLoc [0],
     .call [2] .readRef [1]], .ret [2]⟩]

#guard elimImmRefs noSigs immRetValue matches .ok _

/-- An `&`-typed *parameter* returned directly: under a checkout call it
holds the shadow reference where the target holds the copy — the one
imm-reference escape with no derivation ancestors, also rejected. -/
private def immRetParam : FunDecl := mkFun 1
  [.ref .u64]
  [.ref .u64]
  [⟨[], .ret [0]⟩]

#guard elimImmRefs noSigs immRetParam matches .error _

-- Mutation-algebra operations and reference literals are the
-- elimination's *output* language, not its input.
private def mutOpInput : FunDecl := mkFun 1
  [.u64, .mutRef .u64]
  []
  [⟨[.call [1] (.mkMutLoc 0) [0]], .ret []⟩]

#guard elimImmRefs noSigs mutOpInput matches .error _

private def refLiteral : FunDecl := mkFun 0
  [.u64, .ref .u64, .u64]
  [.u64]
  [⟨[.load 1 (.ref ⟨.loc 0 0, []⟩), .call [2] .readRef [1]], .ret [2]⟩]

#guard elimImmRefs noSigs refLiteral matches .error _

/-- A reference rooted at a callee local returned through a non-checkout
call is dangling: stuck in the semantics (`callOk` requires
reference-free returns), an error in the interpreter. -/
private def danglingRetCallee : FunDecl := mkFun 0
  [.u64, .mutRef .u64]
  [.mutRef .u64]
  [⟨[.load 0 (.u64 1), .call [1] .borrowLoc [0]], .ret [1]⟩]

private def danglingRetCaller : FunDecl := mkFun 0
  [.mutRef .u64, .u64]
  [.u64]
  [⟨[.call [0] (.function 1) [],
     .call [1] .readRef [0]], .ret [1]⟩]

private def danglingProg : Program :=
  ⟨fun f => if f = 0 then some danglingRetCaller
    else if f = 1 then some danglingRetCallee else none,
   fun _ => none⟩

#guard interpFun danglingProg 100 0 [] [] matches .error _

/-- Instructions referencing undeclared locals (beyond `numLocals`) are
rejected: in a checkout callee they could smash the shadow slots, which
live at different indices in source and target. -/
private def outOfRangeLocal : FunDecl := mkFun 1
  [.u64]
  [.u64]
  [⟨[.load 5 (.u64 1), .assign 0 5], .ret [0]⟩]

#guard elimImmRefs noSigs outOfRangeLocal matches .error _

/-- Borrowing an `&`-typed slot that may hold a copied reference:
reads through the borrow see the reference in the source but the copy in
the target — rejected. -/
private def borrowImmSlot : FunDecl := mkFun 1
  [.u64, .ref .u64, .ref .u64, .mutRef (.ref .u64), .u64]
  [.u64]
  [⟨[.call [1] .borrowLoc [0],
     .assign 2 1,
     .call [3] .borrowLoc [2],
     .call [4] .readRef [3],
     .call [4] .readRef [1]], .ret [4]⟩]

#guard elimImmRefs noSigs borrowImmSlot matches .error _

private def oneFieldΔ : StructDecls :=
  fun r => if r = 0 then some { fields := [.u64] } else none

/-- Reading a parent reference while a *written* derived reference is
live: the parent's checked-out payload is stale below the child. -/
private def staleParentRead : FunDecl := mkFun 0
  [.struct 0, .mutRef (.struct 0), .mutRef .u64, .u64, .struct 0]
  [.u64]
  [⟨[.call [0] .pack [],
     .call [1] .borrowLoc [0],
     .call [2] (.borrowField 0) [1],
     .load 3 (.u64 9),
     .call [] .writeRef [2, 3],
     .call [4] .readRef [1],  -- the stale read; [2] is still live below
     .call [] .writeRef [2, 3]], .ret [3]⟩]

#guard refElimFun (fun _ => none) oneFieldΔ staleParentRead matches .error _

/-- Control: the same shape with the parent read *after* the child's
death (write-back done) is fine. -/
private def parentReadAfterDeath : FunDecl := mkFun 0
  [.struct 0, .mutRef (.struct 0), .mutRef .u64, .u64, .struct 0]
  [.u64]
  [⟨[.call [0] .pack [],
     .call [1] .borrowLoc [0],
     .call [2] (.borrowField 0) [1],
     .load 3 (.u64 9),
     .call [] .writeRef [2, 3],
     .call [4] .readRef [1]], .ret [3]⟩]

#guard refElimFun (fun _ => none) oneFieldΔ parentReadAfterDeath matches .ok _

private def twoFieldΔ : StructDecls :=
  fun r => if r = 0 then some { fields := [.u64, .u64] } else none

/-- Two live borrows of the *same* field (aliasing): rejected. -/
private def sameFieldTwice : FunDecl := mkFun 1
  [.mutRef (.struct 0), .mutRef .u64, .mutRef .u64, .u64]
  []
  [⟨[.call [1] (.borrowField 0) [0],
     .call [2] (.borrowField 0) [0],
     .load 3 (.u64 1),
     .call [] .writeRef [1, 3],
     .call [] .writeRef [2, 3]], .ret []⟩]

#guard refElimFun (fun _ => none) twoFieldΔ sameFieldTwice matches .error _

/-- Sibling borrows of *disjoint* fields are legal Move and accepted:
end-to-end agreement, exercising the two-child write-back cascade. -/
private def siblingBorrows : FunDecl := mkFun 2
  [.u64, .u64, .struct 0, .mutRef (.struct 0), .mutRef .u64,
   .mutRef .u64, .u64, .u64, .struct 0]
  [.u64, .u64]
  [⟨[.call [2] .pack [0, 1],
     .call [3] .borrowLoc [2],
     .call [4] (.borrowField 0) [3],
     .call [5] (.borrowField 1) [3],
     .call [6] .readRef [5],
     .call [] .writeRef [4, 6],
     .load 6 (.u64 42),
     .call [] .writeRef [5, 6],
     .call [8] .readRef [3],
     .call [6, 7] .unpack [8]], .ret [6, 7]⟩]

private def elim1' (Δ : StructDecls) (d : FunDecl) : Option Program :=
  (refElimFun (fun _ => none) Δ d).toOption.map fun d' =>
    ⟨fun f => if f = 0 then some d' else none, fun _ => none⟩

/-- A derived borrow crossing a block also keeps its syntactically dead
parent mutation pending, so the child's update reaches the original root. -/
private def crossBlockChild : FunDecl := mkFun 1
  [.u64, .struct 0, .mutRef (.struct 0), .mutRef .u64, .u64]
  [.struct 0]
  [⟨[.call [1] .pack [0],
     .call [2] .borrowLoc [1],
     .call [3] (.borrowField 0) [2]], .jump 1⟩,
   ⟨[.load 4 (.u64 9),
     .call [] .writeRef [3, 4]], .ret [1]⟩]

#guard (elim1' oneFieldΔ crossBlockChild).isSome
#guard agree (prog1 crossBlockChild)
  ((elim1' oneFieldΔ crossBlockChild).getD emptyProg) 0 [] [.u64 3]

#guard (elim1' twoFieldΔ siblingBorrows).isSome
#guard agree (prog1 siblingBorrows)
  ((elim1' twoFieldΔ siblingBorrows).getD emptyProg) 0 [] [.u64 3, .u64 7]

/-- Two returned references derived from the same field on *crossed*
branches: the summary derivations overlap per return pair, so the
caller-side write-back dispatch could race — conservatively rejected
(dynamically the branches are disjoint; the borrow checker rejects the
shape anyway). -/
private def overlapRet : FunDecl := mkFun 2
  [.mutRef (.struct 0), .bool, .mutRef .u64, .mutRef .u64]
  [.mutRef .u64, .mutRef .u64]
  [⟨[], .branch 1 1 2⟩,
   ⟨[.call [2] (.borrowField 0) [0],
     .call [3] (.borrowField 1) [0]], .jump 3⟩,
   ⟨[.call [2] (.borrowField 1) [0],
     .call [3] (.borrowField 0) [0]], .jump 3⟩,
   ⟨[], .ret [2, 3]⟩]

#guard refElimFun (fun _ => none) twoFieldΔ overlapRet matches .error _

/-- Control: straight sibling returns (disjoint derivations) summarize
fine. -/
private def siblingRet : FunDecl := mkFun 1
  [.mutRef (.struct 0), .mutRef .u64, .mutRef .u64]
  [.mutRef .u64, .mutRef .u64]
  [⟨[.call [1] (.borrowField 0) [0],
     .call [2] (.borrowField 1) [0]], .ret [1, 2]⟩]

#guard refElimFun (fun _ => none) twoFieldΔ siblingRet matches .ok _

/-! ## Path-insensitive interprocedural shapes rejected by the partial pass -/

/-- Reusing a dead mutable-parameter slot for an unrelated local borrow would
make the appended final return the unrelated mutation rather than the input
parameter. -/
private def overwriteMutParam : FunDecl := mkFun 1
  [.mutRef .u64, .u64]
  []
  [⟨[.load 1 (.u64 9), .call [0] .borrowLoc [1]], .ret []⟩]

#guard refElimFun (fun _ => none) (fun _ => none) overwriteMutParam matches .error _

/-- At this join the live child came through parent 2 or parent 3.  A
path-insensitive guarded write-back cannot inspect both parent slots because
one is uninitialized on each execution path. -/
private def joinedIntermediateParents : FunDecl := mkFun 2
  [.mutRef (.struct 0), .bool, .mutRef (.struct 0),
   .mutRef (.struct 0), .mutRef .u64, .u64]
  []
  [⟨[], .branch 1 1 2⟩,
   ⟨[.assign 2 0, .call [4] (.borrowField 0) [2]], .jump 3⟩,
   ⟨[.assign 3 0, .call [4] (.borrowField 0) [3]], .jump 3⟩,
   ⟨[.load 5 (.u64 7), .call [] .writeRef [4, 5]], .ret []⟩]

#guard refElimFun (fun _ => none) oneFieldΔ joinedIntermediateParents matches .error _

/-- The same mutation local cannot satisfy two exclusive `&mut` parameters
of one call. -/
private def duplicateMutArgCaller : FunDecl := mkFun 0
  [.u64, .mutRef .u64]
  []
  [⟨[.load 0 (.u64 0), .call [1] .borrowLoc [0],
     .call [] (.function 1) [1, 1]], .ret []⟩]

private def duplicateMutArgCallee : FunDecl := mkFun 2
  [.mutRef .u64, .mutRef .u64]
  []
  [⟨[], .ret []⟩]

#guard refElimProg (fun _ => none)
  [duplicateMutArgCaller, duplicateMutArgCallee] matches .error _

/-! ## Cross-call references eliminate through borrow summaries

The TACAS'22 Fig.-`MutElim` shapes: `&mut` arguments become
value-in/finals-out (`call r := f(r)`), returned references re-enter the
caller's borrow graph along the callee's summarized derivations —
including a *dynamic* choice between two derivations, dispatched by
`is_parent` at the write-back. -/

private def crossM : MProgram := moveM% "
module 0x42::summaries {
    struct S has copy, drop { f: u64, g: u64 }

    fun increment(x: &mut u64) { *x = *x + 1 }
    fun bump_local(v: u64): u64 { increment(&mut v); v }

    fun set_f(s: &mut S, x: u64) { s.f = x }
    fun bump_struct(): u64 {
        let s = S { f: 1, g: 0 }; set_f(&mut s, 9); s.f
    }

    fun get_f(s: &mut S): &mut u64 { &mut s.f }
    fun via_ret_ref(): u64 {
        let s = S { f: 7, g: 0 };
        let r = get_f(&mut s);
        *r = *r + 3;
        s.f
    }

    fun choose(p: bool, s: &mut S): &mut u64 {
        if (p) &mut s.f else &mut s.g
    }
    fun via_choice(p: bool): u64 {
        let s = S { f: 10, g: 20 };
        let r = choose(p, &mut s);
        *r = *r + 1;
        s.f * 100 + s.g
    }

    fun inc_f(s: &mut S) { increment(get_f(s)) }
    fun nested(): u64 { let s = S { f: 100, g: 0 }; inc_f(&mut s); s.f }

    fun write_then_abort(x: &mut u64) { *x = 99; abort 7 }
    fun caller_aborts(): u64 { let v = 1; write_then_abort(&mut v); v }
}
"

#guard (elimProgram crossM).isSome

private def cross : Program := crossM.toProgram
private def cross' : Program := (elimProgram crossM).getD emptyProg

#guard agree cross cross' (crossM.funId "bump_local") [] [.u64 5]
#guard agree cross cross' (crossM.funId "bump_struct") [] []
#guard agree cross cross' (crossM.funId "via_ret_ref") [] []
#guard agree cross cross' (crossM.funId "via_choice") [] [.bool true]
#guard agree cross cross' (crossM.funId "via_choice") [] [.bool false]
#guard agree cross cross' (crossM.funId "nested") [] []
#guard agree cross cross' (crossM.funId "caller_aborts") [] []

end Tests.Interp.RefElimAgree
