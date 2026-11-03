-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.StructApiFacts
import Proofs.RefSoundness

/-!
# A struct-API mutable borrow in progress

UNTRUSTED (checked by the kernel). The analysis takes a call of a function
handle with a `BorrowFieldMutable(off)` attribute for a mutable borrow of
field `off` of the reference argument (`callOrSpecialize`), while the
semantics runs the callee: the struct-API wrapper `MoveLoc(0); <borrow>;
Ret` (`WrapperCode`). The reference invariant `RefInvariant` does not hold
while the wrapper runs — the caller's analysis has already borrowed the
field — so executions keep `InvAll`: the type and availability invariant,
and either the reference invariant or a wrapper in progress (`ApiRun`),
whose caller called it from a state `sc` that keeps the reference
invariant. The wrapper changes nothing but its own frame and records only
the removal of its local 0, so when it returns, the caller resumes as from a
field borrow in `sc`.
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-- **A struct-API mutable borrow in progress**: state `s` runs the wrapper
`w` over the caller `fr`, which called it, with the reference `x` on top of
its operand stack, from state `sc`. The state differs from `sc` only by the
call (the wrapper frame, the argument popped, the next frame identity, the
activation and lock) and by the removal of the wrapper's local 0. The
wrapper is at its `MoveLoc(0)` with `x` in local 0, at its borrow with `x`
on its operand stack, or at its `Ret` with the field borrow of `x`. -/
structure ApiRun (P : Program) (s sc : State) : Prop where
  inv : InvRefs P sc
  call : ∃ (fr : Frame) (rest : List Frame) (info : FunctionInfo) (code : CodeUnit) (h : Nat)
      (fh : FunctionHandle) (off : Nat) (x : Reference) (st : List Value) (w : Frame)
      (callee : FunctionRef) (cinfo : FunctionInfo),
    sc.frames = fr :: rest ∧ ActiveAt P (modelTypes P) (modelAvail P) fr info code ∧
    code.code[fr.pc]? = some (.call h) ∧ info.module.functionHandles[h]? = some fh ∧
    ReferenceSafety.borrowFieldMutable fh.attributes = some off ∧ fr.stack = .ref x :: st ∧
    P.resolve fr.function.module h = some callee ∧ P.function? callee = some cinfo ∧
    WrapperCode P callee cinfo off ∧
    s.frames = w :: { fr with stack := st } :: rest ∧ w.function = callee ∧
    w.id = sc.nextFrame ∧ w.countedActive = (cinfo.moduleId != info.moduleId) ∧
    s.nextFrame = sc.nextFrame + 1 ∧ s.storage = sc.storage ∧ s.held = sc.held ∧
    s.active = (if w.countedActive then sc.incActive cinfo.moduleId else sc).active ∧
    s.lockDepth = (if w.lockEntered then sc.lockDepth + 1 else sc.lockDepth) ∧
    (∃ es : List Invalidation, s.invalidations = es ++ sc.invalidations ∧
      ∀ e ∈ es, e = .removed (.local_ w.id 0)) ∧
    (∀ j v, w.locals[j]? = some (some v) → j = 0) ∧
    ((w.pc = 0 ∧ w.locals[0]? = some (some (.ref x)) ∧ w.stack = []) ∨
     (w.pc = 1 ∧ w.locals[0]? = some none ∧ w.stack = [.ref x]) ∨
     (w.pc = 2 ∧ w.locals[0]? = some none ∧ ∃ (y : Reference) (step : PathStep),
        w.stack = [.ref y] ∧ y.mutable = true ∧ y.location = x.location.child step ∧
        absPath [step] = [.field off] ∧ RefOk P s { fr with stack := st } y))

/-- **The invariant of executions with struct-API borrows**: types and
availability, and the reference invariant or a wrapper in progress. -/
def InvAll (P : Program) (s : State) : Prop :=
  Inv P (modelTypes P) (modelAvail P) s ∧ (RefInvariant P s ∨ ∃ sc, ApiRun P s sc)

theorem InvRefs.invAll {P : Program} {s : State} (h : InvRefs P s) : InvAll P s :=
  ⟨h.1, Or.inl h.2⟩

end MoveBytecode.Proofs
