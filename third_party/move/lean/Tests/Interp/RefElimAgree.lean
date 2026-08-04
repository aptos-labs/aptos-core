-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.IR.RefElim
import Move.IR.Interp
import Tests.Interp.References
import Tests.Interp.Globals

/-!
# Interpreter Tests: Reference Elimination Agreement

Executable evidence for `refElim_correct` on real compiler output and on
hand-written control-flow shapes: eliminate with `refElimFun` and check
that the eliminated program produces the `AgreeOutcome`-related
interpreter outcome — equal on normal returns, equal abort codes on
aborts (abort *memory* may differ: the elimination defers write-backs to
borrow death, see `RefElim.lean`).  Covers cross-block borrows, a
diamond join with the dynamic `is_parent` write-back dispatch, and a
death on only one branch edge (edge splitting).
-/

namespace Tests.Interp.RefElimAgree

open Move.IR
open Move.Frontend

/-- Eliminate references from every function of a dumped module, through
the whole-program pass (borrow summaries for cross-call references). -/
def elimProgram (m : MProgram) : Option Program :=
  (MProgram.elim m).toOption.map MProgram.toProgram

/-- The original and the eliminated program agree on one run — exactly on
normal outcomes, on the code for aborts (`AgreeOutcome`, executably). -/
def agree (p q : Program) (f : FunId) (mem : IMem) (args : List Value) :
    Bool :=
  match interpFun p 1000 f mem args, interpFun q 1000 f mem args with
  | .ok (.ret m vals), .ok (.ret m' vals') => m == m' && vals == vals'
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

/-- Eliminate a single hand-written function into a runnable program. -/
private def elim1 (d : FunDecl) : Option Program :=
  (refElimFun (fun _ => none) d).toOption.map fun d' =>
    ⟨fun f => if f = 0 then some d' else none, fun _ => none⟩

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
     .call [4] .add [4, 5],
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

#guard refElimFun (fun _ => none) escapingBorrow matches .error _

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
